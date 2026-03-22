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
use crate::config::SafetyConfig;
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
    /// Shared handle to the running child process (if any), for termination.
    child_handle: Arc<tokio::sync::Mutex<Option<tokio::process::Child>>>,
    /// Safety configuration for permission flag generation (Layer 2).
    safety_config: Option<SafetyConfig>,
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
            child_handle: Arc::new(tokio::sync::Mutex::new(None)),
            safety_config: None,
        }
    }

    /// Create a new adapter with safety config for permission flag generation.
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

    /// Build the full prompt including system prompt + prior transcript context + new prompt.
    /// Truncates transcript to stay under 200KB (well below macOS ARG_MAX of ~262KB).
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
        tracing::info!(
            provider = "codex",
            prompt_len = full_prompt.len(),
            turn = self.turn_index,
            "sending turn"
        );

        // Build CLI args: base command + safety permission flags (Layer 2).
        let mut cli_args = vec!["@openai/codex".to_string(), "exec".to_string()];

        // Always skip git repo check since daemon runs in its own cwd
        cli_args.push("--skip-git-repo-check".to_string());

        if let Some(ref safety) = self.safety_config {
            let perm_args = crate::safety::codex_permission_args(safety);
            if perm_args.is_empty() {
                // No explicit permissions — use full-auto (sandboxed workspace-write).
                cli_args.push("--full-auto".to_string());
            } else {
                cli_args.extend(perm_args);
            }
        } else {
            // No safety config — use full-auto for non-interactive execution.
            cli_args.push("--full-auto".to_string());
        }

        cli_args.push(full_prompt.clone());

        let arg_refs: Vec<&str> = cli_args.iter().map(|s| s.as_str()).collect();
        let mut child = Command::new("npx")
            .args(&arg_refs)
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

        let stderr = child.stderr.take();

        let (tx, rx) = mpsc::channel::<AgentEvent>(64);

        // Store child in shared handle so terminate() can kill it mid-turn.
        *self.child_handle.lock().await = Some(child);

        // Spawn stderr reader for debugging
        if let Some(stderr) = stderr {
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::debug!(provider = "codex", stderr = %line, "stderr");
                }
            });
        }

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
                tracing::debug!(provider = "codex", line_len = line.len(), "stream chunk");
                let _ = tx.send(AgentEvent::Chunk(line)).await;
            }

            // stdout closed → process likely done. Take child to wait for exit.
            let exit_code = if let Some(mut child) = child_handle.lock().await.take() {
                match child.wait().await {
                    Ok(status) => {
                        if let Some(code) = status.code()
                            && code != 0
                        {
                            tracing::error!(provider = "codex", code, "process exited with error");
                        }
                        status.code()
                    }
                    Err(e) => {
                        tracing::error!(provider = "codex", error = %e, "failed to wait on process");
                        None
                    }
                }
            } else {
                None // terminate() already killed it
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
        if let Some(mut child) = self.child_handle.lock().await.take() {
            let _ = child.kill().await;
        }
        self.session_started = false;
        *self.current_rx.lock().unwrap() = None;
        Ok(())
    }
}
