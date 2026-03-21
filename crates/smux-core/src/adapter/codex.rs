//! Codex CLI headless adapter.
//!
//! Spawns `npx @openai/codex exec -a never -s workspace-write "<prompt>"` per turn.
//! Stdout is streamed line-by-line as [`AgentEvent::Chunk`] events.  Process exit produces
//! [`AgentEvent::TurnComplete`].
//!
//! For v0.1 we use plain text output (not `--json` JSONL).

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use super::{AdapterError, AgentAdapter, AgentEventStream};
use crate::types::{
    AdapterCapabilities, AgentEvent, SessionConfig, SessionSnapshot, Turn, TurnHandle,
};

/// Codex CLI headless adapter.
///
/// Each [`send_turn`](AgentAdapter::send_turn) call spawns a new `npx` child process.  The adapter
/// is *not* persistent — prior context is replayed via the prompt preamble.
pub struct CodexHeadlessAdapter {
    session_started: bool,
    working_dir: PathBuf,
    system_prompt: String,
    transcript: Vec<Turn>,
    turn_index: u64,
    /// Receiver for the most recent turn's event stream.
    current_rx: Arc<Mutex<Option<mpsc::Receiver<AgentEvent>>>>,
    /// Handle to the running child process (if any), for termination.
    child_handle: Option<tokio::process::Child>,
}

impl CodexHeadlessAdapter {
    /// Create a new adapter that will run commands in `working_dir`.
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            session_started: false,
            working_dir,
            system_prompt: String::new(),
            transcript: Vec::new(),
            turn_index: 0,
            current_rx: Arc::new(Mutex::new(None)),
            child_handle: None,
        }
    }

    /// Build the full prompt including system prompt + prior transcript context + new prompt.
    fn build_full_prompt(&self, prompt: &str) -> String {
        let mut parts: Vec<String> = Vec::new();

        if !self.system_prompt.is_empty() {
            parts.push(format!("[System]\n{}\n", self.system_prompt));
        }

        for turn in &self.transcript {
            parts.push(format!("[{}]\n{}\n", turn.role, turn.content));
        }

        parts.push(format!("[user]\n{}", prompt));

        parts.join("\n")
    }
}

#[async_trait]
impl AgentAdapter for CodexHeadlessAdapter {
    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            persistent_session: false,
            streaming: true,
            native_snapshot: false,
        }
    }

    async fn start_session(&mut self, config: SessionConfig) -> Result<(), AdapterError> {
        if self.session_started {
            return Err(AdapterError::AlreadyStarted);
        }
        self.working_dir = config.working_directory;
        self.system_prompt = config.system_prompt;
        self.transcript = config.prior_transcript;
        self.session_started = true;
        Ok(())
    }

    async fn send_turn(&mut self, prompt: &str) -> Result<TurnHandle, AdapterError> {
        if !self.session_started {
            return Err(AdapterError::NotStarted);
        }

        let full_prompt = self.build_full_prompt(prompt);

        let mut child = Command::new("npx")
            .args([
                "@openai/codex",
                "exec",
                "-a",
                "never",
                "-s",
                "workspace-write",
                &full_prompt,
            ])
            .current_dir(&self.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .spawn()?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AdapterError::Other("failed to capture stdout".into()))?;

        let (tx, rx) = mpsc::channel::<AgentEvent>(64);

        // Spawn a task to read stdout line-by-line and forward as events.
        let mut child_for_wait = child;
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut full_output = String::new();

            while let Ok(Some(line)) = lines.next_line().await {
                if !full_output.is_empty() {
                    full_output.push('\n');
                }
                full_output.push_str(&line);
                let _ = tx.send(AgentEvent::Chunk(line)).await;
            }

            // Wait for the process to exit.
            let exit_code = match child_for_wait.wait().await {
                Ok(status) => status.code(),
                Err(_) => None,
            };

            let _ = tx.send(AgentEvent::TurnComplete(full_output)).await;
            let _ = tx.send(AgentEvent::ProcessExited(exit_code)).await;
            // tx drops here, closing the stream.
        });

        // Record in transcript.
        self.transcript.push(Turn {
            role: "user".into(),
            content: prompt.to_string(),
            timestamp: SystemTime::now(),
        });

        let handle = TurnHandle {
            turn_index: self.turn_index,
        };
        self.turn_index += 1;

        *self.current_rx.lock().unwrap() = Some(rx);

        Ok(handle)
    }

    fn stream_events(&self) -> Result<AgentEventStream<'_>, AdapterError> {
        let rx = self
            .current_rx
            .lock()
            .unwrap()
            .take()
            .ok_or(AdapterError::NoTurns)?;
        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    async fn snapshot_state(&self) -> Result<SessionSnapshot, AdapterError> {
        if !self.session_started {
            return Err(AdapterError::NotStarted);
        }
        let state = serde_json::to_vec(&self.transcript)
            .map_err(|e| AdapterError::Other(format!("failed to serialize transcript: {e}")))?;
        Ok(SessionSnapshot {
            adapter_type: "codex".into(),
            state,
        })
    }

    async fn restore_state(&mut self, snapshot: SessionSnapshot) -> Result<(), AdapterError> {
        if snapshot.adapter_type != "codex" {
            return Err(AdapterError::RestoreFailed(format!(
                "expected adapter_type 'codex', got '{}'",
                snapshot.adapter_type
            )));
        }
        self.transcript = serde_json::from_slice(&snapshot.state)
            .map_err(|e| AdapterError::RestoreFailed(format!("failed to deserialize: {e}")))?;
        self.session_started = true;
        Ok(())
    }

    async fn terminate(&mut self) -> Result<(), AdapterError> {
        if let Some(ref mut child) = self.child_handle {
            let _ = child.kill().await;
        }
        self.child_handle = None;
        self.session_started = false;
        *self.current_rx.lock().unwrap() = None;
        Ok(())
    }
}
