//! Gemini CLI headless adapter.
//!
//! Spawns `npx @google/gemini-cli` per turn, consistent with the Claude/Codex npx pattern.
//! Stdout is streamed line-by-line as [`AgentEvent::Chunk`] events.  Process exit produces
//! [`AgentEvent::TurnComplete`].

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
use crate::config::SafetyConfig;
use crate::types::{
    AdapterCapabilities, AgentEvent, SessionConfig, SessionSnapshot, Turn, TurnHandle,
};

/// Gemini CLI headless adapter.
///
/// Each [`send_turn`](AgentAdapter::send_turn) call spawns a new `npx` child process.
/// Prior context is replayed via the prompt preamble.
pub struct GeminiHeadlessAdapter {
    session_started: bool,
    working_dir: PathBuf,
    system_prompt: String,
    transcript: Vec<Turn>,
    turn_index: u64,
    current_rx: Arc<Mutex<Option<mpsc::Receiver<AgentEvent>>>>,
    child_handle: Arc<tokio::sync::Mutex<Option<tokio::process::Child>>>,
    #[allow(dead_code)]
    safety_config: Option<SafetyConfig>,
}

impl GeminiHeadlessAdapter {
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            session_started: false,
            working_dir,
            system_prompt: String::new(),
            transcript: Vec::new(),
            turn_index: 0,
            current_rx: Arc::new(Mutex::new(None)),
            child_handle: Arc::new(tokio::sync::Mutex::new(None)),
            safety_config: None,
        }
    }

    pub fn with_safety(working_dir: PathBuf, safety_config: SafetyConfig) -> Self {
        Self {
            session_started: false,
            working_dir,
            system_prompt: String::new(),
            transcript: Vec::new(),
            turn_index: 0,
            current_rx: Arc::new(Mutex::new(None)),
            child_handle: Arc::new(tokio::sync::Mutex::new(None)),
            safety_config: Some(safety_config),
        }
    }

    /// Check if the Gemini CLI is available. Tries `npx @google/gemini-cli --version`
    /// first, then falls back to `gemini --version`.
    /// Check if the Gemini CLI is available. Tries `npx @google/gemini-cli --version`
    /// first, then falls back to `gemini --version`.
    pub async fn is_available() -> bool {
        // Try npx first.
        if let Ok(output) = tokio::process::Command::new("npx")
            .args(["@google/gemini-cli", "--version"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .stdin(std::process::Stdio::null())
            .output()
            .await
            && output.status.success()
        {
            return true;
        }
        // Fallback: globally installed binary.
        matches!(
            tokio::process::Command::new("gemini")
                .arg("--version")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .stdin(std::process::Stdio::null())
                .output()
                .await,
            Ok(output) if output.status.success()
        )
    }

    fn build_full_prompt(&self, prompt: &str) -> String {
        const MAX_PROMPT_BYTES: usize = 200_000;

        let mut parts: Vec<String> = Vec::new();
        if !self.system_prompt.is_empty() {
            parts.push(format!("[System]\n{}\n", self.system_prompt));
        }

        let user_part = format!("[user]\n{}", prompt);
        let mut budget = MAX_PROMPT_BYTES
            .saturating_sub(parts.iter().map(|p| p.len()).sum::<usize>())
            .saturating_sub(user_part.len())
            .saturating_sub(100);

        let mut transcript_parts: Vec<String> = Vec::new();
        for turn in self.transcript.iter().rev() {
            let part = format!("[{}]\n{}\n", turn.role, turn.content);
            if part.len() > budget {
                transcript_parts.push("[...transcript truncated...]\n".into());
                break;
            }
            budget -= part.len();
            transcript_parts.push(part);
        }
        transcript_parts.reverse();
        parts.extend(transcript_parts);
        parts.push(user_part);

        parts.join("\n")
    }
}

#[async_trait]
impl AgentAdapter for GeminiHeadlessAdapter {
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
            provider = "gemini",
            prompt_len = full_prompt.len(),
            turn = self.turn_index,
            "sending turn"
        );

        // Gemini CLI: npx @google/gemini-cli -p "<prompt>"
        // The -p flag runs in non-interactive (prompt) mode.
        let mut child = Command::new("npx")
            .args(["@google/gemini-cli", "-p", &full_prompt])
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
                tracing::debug!(provider = "gemini", line_len = line.len(), "stream chunk");
                let _ = tx.send(AgentEvent::Chunk(line)).await;
            }

            let exit_code = if let Some(mut child) = child_handle.lock().await.take() {
                match child.wait().await {
                    Ok(status) => {
                        if let Some(code) = status.code()
                            && code != 0
                        {
                            tracing::error!(provider = "gemini", code, "process exited with error");
                        }
                        status.code()
                    }
                    Err(e) => {
                        tracing::error!(provider = "gemini", error = %e, "failed to wait on process");
                        None
                    }
                }
            } else {
                None
            };

            let _ = tx.send(AgentEvent::TurnComplete(full_output)).await;
            let _ = tx.send(AgentEvent::ProcessExited(exit_code)).await;
        });

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
            adapter_type: "gemini".into(),
            state,
        })
    }

    async fn restore_state(&mut self, snapshot: SessionSnapshot) -> Result<(), AdapterError> {
        if snapshot.adapter_type != "gemini" {
            return Err(AdapterError::RestoreFailed(format!(
                "expected adapter_type 'gemini', got '{}'",
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
