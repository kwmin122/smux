//! Claude Code headless adapter.
//!
//! Spawns `npx @anthropic-ai/claude-code -p --output-format text --dangerously-skip-permissions`
//! per turn.  The full prompt is built from the system prompt, prior transcript context, and the
//! new user prompt.  Stdout is streamed line-by-line as [`AgentEvent::Chunk`] events, and the
//! process exit produces a [`AgentEvent::TurnComplete`].
//!
//! For v0.1 we use `--output-format text` (plain text).  Parsing `stream-json` is deferred.

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

/// Claude Code headless adapter.
///
/// Each [`send_turn`](AgentAdapter::send_turn) call spawns a new `npx` child process.  The adapter
/// is *not* persistent — prior context is replayed via the prompt preamble.
pub struct ClaudeHeadlessAdapter {
    session_started: bool,
    working_dir: PathBuf,
    system_prompt: String,
    transcript: Vec<Turn>,
    turn_index: u64,
    /// Receiver for the most recent turn's event stream.
    current_rx: Arc<Mutex<Option<mpsc::Receiver<AgentEvent>>>>,
    /// Shared handle to the running child process (if any), for termination.
    /// Shared with the streaming task so terminate() can kill it mid-turn.
    child_handle: Arc<tokio::sync::Mutex<Option<tokio::process::Child>>>,
}

impl ClaudeHeadlessAdapter {
    /// Create a new adapter that will run commands in `working_dir`.
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            session_started: false,
            working_dir,
            system_prompt: String::new(),
            transcript: Vec::new(),
            turn_index: 0,
            child_handle: Arc::new(tokio::sync::Mutex::new(None)),
            current_rx: Arc::new(Mutex::new(None)),
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
impl AgentAdapter for ClaudeHeadlessAdapter {
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
        tracing::info!(
            provider = "claude",
            prompt_len = full_prompt.len(),
            turn = self.turn_index,
            "sending turn"
        );

        let mut child = Command::new("npx")
            .args([
                "@anthropic-ai/claude-code",
                "-p",
                "--output-format",
                "text",
                "--dangerously-skip-permissions",
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

        // Store child in shared handle so terminate() can kill it mid-turn.
        // stdout is already taken, so the child only has stderr + process handle.
        *self.child_handle.lock().await = Some(child);

        let child_handle = Arc::clone(&self.child_handle);
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut full_output = String::new();

            while let Ok(Some(line)) = lines.next_line().await {
                if !full_output.is_empty() {
                    full_output.push('\n');
                }
                full_output.push_str(&line);
                tracing::debug!(provider = "claude", line_len = line.len(), "stream chunk");
                let _ = tx.send(AgentEvent::Chunk(line)).await;
            }

            // stdout closed → process likely done. Take child to wait for exit.
            let exit_code = if let Some(mut child) = child_handle.lock().await.take() {
                match child.wait().await {
                    Ok(status) => {
                        if let Some(code) = status.code()
                            && code != 0
                        {
                            tracing::error!(provider = "claude", code, "process exited with error");
                        }
                        status.code()
                    }
                    Err(e) => {
                        tracing::error!(provider = "claude", error = %e, "failed to wait on process");
                        None
                    }
                }
            } else {
                // terminate() already killed it
                None
            };

            let _ = tx.send(AgentEvent::TurnComplete(full_output)).await;
            let _ = tx.send(AgentEvent::ProcessExited(exit_code)).await;
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
            adapter_type: "claude".into(),
            state,
        })
    }

    async fn restore_state(&mut self, snapshot: SessionSnapshot) -> Result<(), AdapterError> {
        if snapshot.adapter_type != "claude" {
            return Err(AdapterError::RestoreFailed(format!(
                "expected adapter_type 'claude', got '{}'",
                snapshot.adapter_type
            )));
        }
        self.transcript = serde_json::from_slice(&snapshot.state)
            .map_err(|e| AdapterError::RestoreFailed(format!("failed to deserialize: {e}")))?;
        self.session_started = true;
        Ok(())
    }

    async fn terminate(&mut self) -> Result<(), AdapterError> {
        if let Some(mut child) = self.child_handle.lock().await.take() {
            let _ = child.kill().await;
        }
        self.session_started = false;
        *self.current_rx.lock().unwrap() = None;
        Ok(())
    }
}
