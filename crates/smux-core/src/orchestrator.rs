//! Orchestrator — the core ping-pong loop between planner and verifier.
//!
//! Wires together: [`AgentAdapter`], [`stop::detect`], and [`context`] passing.
//! Does **not** invoke git worktree or session store — that is the CLI's job.

use futures::StreamExt;

use crate::adapter::AgentAdapter;
use crate::context::{DEFAULT_MAX_TOKENS, build_planner_feedback, build_verifier_prompt};
use crate::stop;
use crate::types::{AgentEvent, SessionConfig, VerifyResult};

/// Events emitted by the orchestrator during execution.
///
/// Consumers (e.g. the daemon) can subscribe to these to forward live output
/// to attached clients.
#[derive(Debug, Clone)]
pub enum OrchestratorEvent {
    /// A new round has started.
    RoundStarted { round: u32 },
    /// The planner produced output for the given round.
    PlannerOutput { round: u32, content: String },
    /// The verifier produced output for the given round.
    VerifierOutput { round: u32, content: String },
    /// A round completed with the given verdict.
    RoundComplete { round: u32, verdict: VerifyResult },
}

/// Configuration for an orchestrator run.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// The task description to send to the planner.
    pub task: String,
    /// Maximum number of planner-verifier rounds before giving up.
    pub max_rounds: u32,
    /// Token budget for context passing (truncation threshold).
    pub max_tokens: usize,
}

/// Outcome of an orchestrator run.
#[derive(Debug)]
pub enum OrchestratorOutcome {
    /// The verifier approved the planner's output.
    Approved { round: u32, reason: String },
    /// The maximum number of rounds was reached without approval.
    MaxRoundsReached { rounds_completed: u32 },
    /// An unrecoverable error occurred.
    Error { message: String },
}

/// The core ping-pong loop orchestrator.
///
/// Holds a planner adapter and a verifier adapter, driving turns between them
/// until the verifier approves, the round budget is exhausted, or an error
/// occurs.
pub struct Orchestrator {
    planner: Box<dyn AgentAdapter>,
    verifier: Box<dyn AgentAdapter>,
    config: OrchestratorConfig,
    /// Optional channel to emit live events during orchestration.
    event_sink: Option<tokio::sync::mpsc::Sender<OrchestratorEvent>>,
}

impl Orchestrator {
    /// Create a new orchestrator with the given adapters and configuration.
    pub fn new(
        planner: Box<dyn AgentAdapter>,
        verifier: Box<dyn AgentAdapter>,
        config: OrchestratorConfig,
    ) -> Self {
        Self {
            planner,
            verifier,
            config,
            event_sink: None,
        }
    }

    /// Set an optional event sink for live event streaming.
    pub fn with_event_sink(mut self, sink: tokio::sync::mpsc::Sender<OrchestratorEvent>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    /// Emit an event to the sink (if configured). Errors are silently ignored.
    async fn emit(&self, event: OrchestratorEvent) {
        if let Some(sink) = &self.event_sink {
            let _ = sink.send(event).await;
        }
    }

    /// Run the ping-pong loop.
    ///
    /// 1. Send task to planner
    /// 2. Collect planner output
    /// 3. Build verifier prompt (with context passing)
    /// 4. Send to verifier
    /// 5. Collect verifier output
    /// 6. Parse verdict (stop detection)
    /// 7. If APPROVED -> return `Approved`
    /// 8. If REJECTED -> build planner feedback, go to step 1
    /// 9. If NeedsInfo -> re-ask verifier (once per round)
    /// 10. If max rounds -> return `MaxRoundsReached`
    pub async fn run(&mut self) -> OrchestratorOutcome {
        // Start both adapter sessions.
        let planner_config = SessionConfig {
            system_prompt: "You are a planner agent.".into(),
            working_directory: std::path::PathBuf::from("/tmp"),
            prior_transcript: vec![],
        };
        let verifier_config = SessionConfig {
            system_prompt: "You are a verifier agent.".into(),
            working_directory: std::path::PathBuf::from("/tmp"),
            prior_transcript: vec![],
        };

        if let Err(e) = self.planner.start_session(planner_config).await {
            return OrchestratorOutcome::Error {
                message: format!("failed to start planner session: {e}"),
            };
        }
        if let Err(e) = self.verifier.start_session(verifier_config).await {
            return OrchestratorOutcome::Error {
                message: format!("failed to start verifier session: {e}"),
            };
        }

        let max_tokens = if self.config.max_tokens == 0 {
            DEFAULT_MAX_TOKENS
        } else {
            self.config.max_tokens
        };

        let mut prior_rounds: Vec<(u32, VerifyResult)> = Vec::new();
        let mut planner_prompt = self.config.task.clone();

        for round in 1..=self.config.max_rounds {
            tracing::info!(round, "starting round");
            self.emit(OrchestratorEvent::RoundStarted { round }).await;

            // ── Step 1-2: Send task/feedback to planner and collect output ──
            let planner_output = match send_and_collect(&mut self.planner, &planner_prompt).await {
                Ok(output) => output,
                Err(e) => {
                    return OrchestratorOutcome::Error {
                        message: format!("planner error in round {round}: {e}"),
                    };
                }
            };

            tracing::info!(
                round,
                planner_output_len = planner_output.len(),
                "planner responded"
            );

            self.emit(OrchestratorEvent::PlannerOutput {
                round,
                content: planner_output.clone(),
            })
            .await;

            // ── Step 3-5: Build verifier prompt and collect verdict ──
            let verifier_prompt =
                build_verifier_prompt(round, &planner_output, &prior_rounds, max_tokens);

            let verifier_output = match send_and_collect(&mut self.verifier, &verifier_prompt).await
            {
                Ok(output) => output,
                Err(e) => {
                    return OrchestratorOutcome::Error {
                        message: format!("verifier error in round {round}: {e}"),
                    };
                }
            };

            tracing::info!(
                round,
                verifier_output_len = verifier_output.len(),
                "verifier responded"
            );

            self.emit(OrchestratorEvent::VerifierOutput {
                round,
                content: verifier_output.clone(),
            })
            .await;

            // ── Step 6: Parse verdict ──
            let verdict = stop::detect(&verifier_output);
            tracing::info!(round, ?verdict, "verdict detected");

            self.emit(OrchestratorEvent::RoundComplete {
                round,
                verdict: verdict.clone(),
            })
            .await;

            match &verdict {
                // ── Step 7: Approved -> done ──
                VerifyResult::Approved { reason, .. } => {
                    return OrchestratorOutcome::Approved {
                        round,
                        reason: reason.clone(),
                    };
                }
                // ── Step 8: Rejected -> build feedback and continue loop ──
                VerifyResult::Rejected { .. } => {
                    prior_rounds.push((round, verdict.clone()));
                    planner_prompt =
                        build_planner_feedback(round, &verifier_output, &verdict, max_tokens);
                }
                // ── Step 9: NeedsInfo -> re-ask verifier once ──
                VerifyResult::NeedsInfo { question } => {
                    let re_ask_prompt = format!(
                        "Please provide a clear verdict. Your previous response did not contain one.\n\
                         Question that arose: {question}\n\n\
                         Respond with a JSON verdict block:\n\
                         {{\"verdict\": \"APPROVED\"|\"REJECTED\", \"category\": \"...\", \"reason\": \"...\", \"confidence\": 0.0-1.0}}"
                    );

                    let re_ask_output =
                        match send_and_collect(&mut self.verifier, &re_ask_prompt).await {
                            Ok(output) => output,
                            Err(e) => {
                                return OrchestratorOutcome::Error {
                                    message: format!("verifier re-ask error in round {round}: {e}"),
                                };
                            }
                        };

                    let re_verdict = stop::detect(&re_ask_output);
                    tracing::info!(round, ?re_verdict, "re-ask verdict detected");

                    match &re_verdict {
                        VerifyResult::Approved { reason, .. } => {
                            return OrchestratorOutcome::Approved {
                                round,
                                reason: reason.clone(),
                            };
                        }
                        VerifyResult::Rejected { .. } => {
                            prior_rounds.push((round, re_verdict.clone()));
                            planner_prompt = build_planner_feedback(
                                round,
                                &re_ask_output,
                                &re_verdict,
                                max_tokens,
                            );
                        }
                        VerifyResult::NeedsInfo { .. } => {
                            // Second NeedsInfo — treat as rejection to avoid infinite loop.
                            let fallback_verdict = VerifyResult::Rejected {
                                reason: "Verifier failed to provide a verdict after re-ask".into(),
                                category: crate::types::RejectCategory::IncompleteImpl,
                                confidence: 0.0,
                            };
                            prior_rounds.push((round, fallback_verdict.clone()));
                            planner_prompt = build_planner_feedback(
                                round,
                                &re_ask_output,
                                &fallback_verdict,
                                max_tokens,
                            );
                        }
                    }
                }
            }
        }

        OrchestratorOutcome::MaxRoundsReached {
            rounds_completed: self.config.max_rounds,
        }
    }
}

/// Send a prompt to an adapter and collect the full response string.
///
/// Gathers all `Chunk` events and the `TurnComplete` payload into a single
/// concatenated string. Returns the `TurnComplete` content (which in
/// `FakeAdapter` is the full response) if present, otherwise falls back to
/// the concatenated chunks.
async fn send_and_collect(
    adapter: &mut Box<dyn AgentAdapter>,
    prompt: &str,
) -> Result<String, String> {
    adapter
        .send_turn(prompt)
        .await
        .map_err(|e| format!("send_turn failed: {e}"))?;

    let stream = adapter
        .stream_events()
        .map_err(|e| format!("stream_events failed: {e}"))?;

    collect_stream(stream).await
}

/// Collect all events from a stream into a single response string.
///
/// Uses `TurnComplete` content if available; otherwise concatenates `Chunk`s.
/// Returns an error string if an `Error` event is encountered.
async fn collect_stream(stream: crate::adapter::AgentEventStream<'_>) -> Result<String, String> {
    let mut chunks = String::new();
    let mut complete: Option<String> = None;

    futures::pin_mut!(stream);

    while let Some(event) = stream.next().await {
        match event {
            AgentEvent::Chunk(text) => {
                chunks.push_str(&text);
            }
            AgentEvent::TurnComplete(text) => {
                complete = Some(text);
            }
            AgentEvent::Error(e) => {
                return Err(format!("agent error: {e}"));
            }
            AgentEvent::ProcessExited(code) => {
                if let Some(c) = code
                    && c != 0
                {
                    return Err(format!("agent process exited with code {c}"));
                }
            }
        }
    }

    // Prefer TurnComplete (full output) over concatenated chunks.
    Ok(complete.unwrap_or(chunks))
}
