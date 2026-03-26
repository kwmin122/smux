//! Orchestrator — the core ping-pong loop between planner and verifier.
//!
//! Wires together: [`AgentAdapter`], [`stop::detect`], and [`context`] passing.
//! Does **not** invoke git worktree or session store — that is the CLI's job.

use futures::StreamExt;

use crate::adapter::AgentAdapter;
use crate::consensus;
use crate::context::{DEFAULT_MAX_TOKENS, build_planner_feedback, build_verifier_prompt};
use crate::health::{AgentHealth, HealthConfig, HealthMonitor};
use crate::stop;
use crate::types::{
    AgentEvent, ConsensusResult, ConsensusStrategy, SessionConfig, VerifierVerdict, VerifyResult,
};

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
    /// An agent health state changed.
    HealthStateChanged { agent: String, state: String },
    /// Cross-verify consensus result from multiple verifiers.
    CrossVerifyResult { round: u32, result: ConsensusResult },
    /// A post-hoc safety audit alert was triggered (Layer 3).
    SafetyAlert {
        round: u32,
        severity: String,
        message: String,
    },
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
    /// Health monitoring configuration. `None` disables health monitoring.
    pub health_config: Option<HealthConfig>,
    /// Consensus strategy for multi-verifier mode.
    pub consensus_strategy: ConsensusStrategy,
    /// Names of verifier adapters (e.g. ["codex", "claude", "gemini"]).
    /// Used for labeling in CrossVerifyResult and logs.
    pub verifier_names: Vec<String>,
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
    verifiers: Vec<Box<dyn AgentAdapter>>,
    config: OrchestratorConfig,
    /// Optional channel to emit live events during orchestration.
    event_sink: Option<tokio::sync::mpsc::Sender<OrchestratorEvent>>,
}

impl Orchestrator {
    /// Create a new orchestrator with the given adapters and configuration.
    ///
    /// Backward compatible: accepts a single verifier.
    pub fn new(
        planner: Box<dyn AgentAdapter>,
        verifier: Box<dyn AgentAdapter>,
        config: OrchestratorConfig,
    ) -> Self {
        Self {
            planner,
            verifiers: vec![verifier],
            config,
            event_sink: None,
        }
    }

    /// Create an orchestrator with multiple verifiers for cross-verification.
    pub fn new_multi(
        planner: Box<dyn AgentAdapter>,
        verifiers: Vec<Box<dyn AgentAdapter>>,
        config: OrchestratorConfig,
    ) -> Self {
        Self {
            planner,
            verifiers,
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
        if let Err(e) = self.planner.start_session(planner_config).await {
            return OrchestratorOutcome::Error {
                message: format!("failed to start planner session: {e}"),
            };
        }
        for (i, verifier) in self.verifiers.iter_mut().enumerate() {
            let v_config = SessionConfig {
                system_prompt: "You are a verifier agent.".into(),
                working_directory: std::path::PathBuf::from("/tmp"),
                prior_transcript: vec![],
            };
            if let Err(e) = verifier.start_session(v_config).await {
                return OrchestratorOutcome::Error {
                    message: format!("failed to start verifier[{i}] session: {e}"),
                };
            }
        }

        let max_tokens = if self.config.max_tokens == 0 {
            DEFAULT_MAX_TOKENS
        } else {
            self.config.max_tokens
        };

        // Set up health monitors.
        let health_cfg = self.config.health_config.clone().unwrap_or_default();
        let mut planner_health = HealthMonitor::new(health_cfg.clone());
        // Per-verifier health monitors (one per adapter).
        let mut verifier_healths: Vec<HealthMonitor> = (0..self.verifiers.len())
            .map(|_| HealthMonitor::new(health_cfg.clone()))
            .collect();

        let mut prior_rounds: Vec<(u32, VerifyResult)> = Vec::new();
        let mut planner_prompt = self.config.task.clone();

        // Track whether we already restarted each agent in the current round.
        let mut planner_restarted_this_round: bool;
        let mut _verifier_restarted_this_round: bool;

        for round in 1..=self.config.max_rounds {
            tracing::info!(round, "starting round");
            self.emit(OrchestratorEvent::RoundStarted { round }).await;

            planner_restarted_this_round = false;
            _verifier_restarted_this_round = false;

            // ── Step 1-2: Send task/feedback to planner and collect output ──
            let planner_output = match send_and_collect_with_health(
                &mut self.planner,
                &planner_prompt,
                &mut planner_health,
                "planner",
                &mut planner_restarted_this_round,
                &self.event_sink,
            )
            .await
            {
                Ok(output) => output,
                Err(e) => {
                    tracing::error!(round, error = %e, "planner adapter failed");
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

            // ── Step 3-5: Build verifier prompt and collect verdict(s) ──
            let verifier_prompt =
                build_verifier_prompt(round, &planner_output, &prior_rounds, max_tokens);

            tracing::debug!(
                round,
                prompt_tokens = crate::context::estimate_tokens(&verifier_prompt),
                max_tokens,
                num_verifiers = self.verifiers.len(),
                "context passed to verifier(s)"
            );

            // Collect output from all verifiers IN PARALLEL.
            // Take verifiers out of self to allow concurrent mutable access.
            let taken_verifiers: Vec<Box<dyn AgentAdapter>> = self.verifiers.drain(..).collect();
            let num_verifiers = taken_verifiers.len();

            let mut join_set = tokio::task::JoinSet::new();
            for (i, mut verifier) in taken_verifiers.into_iter().enumerate() {
                let prompt = verifier_prompt.clone();
                join_set.spawn(async move {
                    let start = tokio::time::Instant::now();
                    let result = send_and_collect_bare(&mut verifier, &prompt).await;
                    let duration_ms = start.elapsed().as_millis() as u64;
                    (i, verifier, result, duration_ms)
                });
            }

            let mut verifier_outputs: Vec<(usize, String, u64)> = Vec::with_capacity(num_verifiers);
            let mut returned_verifiers: Vec<(usize, Box<dyn AgentAdapter>)> =
                Vec::with_capacity(num_verifiers);
            let mut first_error: Option<String> = None;

            while let Some(join_result) = join_set.join_next().await {
                match join_result {
                    Ok((i, verifier, Ok(output), duration_ms)) => {
                        tracing::info!(
                            round,
                            verifier = i,
                            output_len = output.len(),
                            "verifier responded"
                        );
                        emit_event(
                            &self.event_sink,
                            OrchestratorEvent::VerifierOutput {
                                round,
                                content: output.clone(),
                            },
                        )
                        .await;
                        verifier_outputs.push((i, output, duration_ms));
                        returned_verifiers.push((i, verifier));
                    }
                    Ok((i, verifier, Err(e), _duration_ms)) => {
                        tracing::error!(round, verifier = i, error = %e, "verifier adapter failed");
                        returned_verifiers.push((i, verifier));
                        if first_error.is_none() {
                            first_error =
                                Some(format!("verifier[{i}] error in round {round}: {e}"));
                        }
                    }
                    Err(e) => {
                        if first_error.is_none() {
                            first_error = Some(format!("verifier task panicked: {e}"));
                        }
                    }
                }
            }

            // Put verifiers back in order.
            returned_verifiers.sort_by_key(|(i, _)| *i);
            self.verifiers = returned_verifiers.into_iter().map(|(_, v)| v).collect();
            verifier_outputs.sort_by_key(|(i, _, _)| *i);

            if let Some(err) = first_error {
                return OrchestratorOutcome::Error { message: err };
            }

            // Update per-verifier health monitors.
            for &(i, _, _) in &verifier_outputs {
                if i < verifier_healths.len() {
                    verifier_healths[i].record_event();
                    let state = verifier_healths[i].check();
                    if !matches!(state, AgentHealth::Healthy) {
                        let state_str = format!("{state:?}");
                        emit_event(
                            &self.event_sink,
                            OrchestratorEvent::HealthStateChanged {
                                agent: format!("verifier[{i}]"),
                                state: state_str,
                            },
                        )
                        .await;
                    }
                }
            }

            // ── Step 6: Parse verdict(s) and apply consensus ──
            let verdict = if verifier_outputs.len() == 1 {
                // Single verifier — direct verdict, no consensus overhead.
                let v = stop::detect(&verifier_outputs[0].1);
                tracing::info!(round, ?v, "verdict detected");

                self.emit(OrchestratorEvent::RoundComplete {
                    round,
                    verdict: v.clone(),
                })
                .await;
                v
            } else {
                // Multi-verifier — parse each, apply consensus engine.
                let individual_verdicts: Vec<VerifierVerdict> = verifier_outputs
                    .iter()
                    .map(|(i, output, duration_ms)| {
                        let v = stop::detect(output);
                        tracing::info!(round, verifier = i, ?v, duration_ms, "individual verdict");
                        VerifierVerdict {
                            adapter_name: self
                                .config
                                .verifier_names
                                .get(*i)
                                .cloned()
                                .unwrap_or_else(|| format!("verifier[{i}]")),
                            result: v,
                            duration_ms: *duration_ms,
                        }
                    })
                    .collect();

                let consensus_result =
                    consensus::resolve(&self.config.consensus_strategy, individual_verdicts);
                tracing::info!(
                    round,
                    strategy = ?self.config.consensus_strategy,
                    agreement = consensus_result.agreement_ratio,
                    ?consensus_result.final_verdict,
                    "consensus verdict"
                );

                let final_v = consensus_result.final_verdict.clone();

                self.emit(OrchestratorEvent::CrossVerifyResult {
                    round,
                    result: consensus_result,
                })
                .await;

                self.emit(OrchestratorEvent::RoundComplete {
                    round,
                    verdict: final_v.clone(),
                })
                .await;

                final_v
            };

            // Collect combined output from ALL verifiers for planner feedback.
            let verifier_output = if verifier_outputs.len() == 1 {
                verifier_outputs
                    .into_iter()
                    .next()
                    .map(|(_, o, _)| o)
                    .unwrap_or_default()
            } else {
                verifier_outputs
                    .into_iter()
                    .map(|(i, o, _)| format!("[verifier[{i}]]\n{o}"))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            };

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
                // ── Step 9: NeedsInfo -> re-ask ALL verifiers once ──
                VerifyResult::NeedsInfo { question } => {
                    tracing::warn!(round, %question, "verdict returned NeedsInfo, re-asking all verifiers");
                    let re_ask_prompt = format!(
                        "Please provide a clear verdict. Your previous response did not contain one.\n\
                         Question that arose: {question}\n\n\
                         Respond with a JSON verdict block:\n\
                         {{\"verdict\": \"APPROVED\"|\"REJECTED\", \"category\": \"...\", \"reason\": \"...\", \"confidence\": 0.0-1.0}}"
                    );

                    // Re-ask ALL verifiers in parallel (same pattern as initial).
                    let taken = self.verifiers.drain(..).collect::<Vec<_>>();
                    let mut re_join = tokio::task::JoinSet::new();
                    for (i, mut v) in taken.into_iter().enumerate() {
                        let p = re_ask_prompt.clone();
                        re_join.spawn(async move {
                            let r = send_and_collect_bare(&mut v, &p).await;
                            (i, v, r)
                        });
                    }
                    let mut re_outputs: Vec<(usize, String)> = Vec::new();
                    let mut re_returned: Vec<(usize, Box<dyn AgentAdapter>)> = Vec::new();
                    let mut re_error: Option<String> = None;
                    while let Some(jr) = re_join.join_next().await {
                        match jr {
                            Ok((i, v, Ok(out))) => {
                                re_outputs.push((i, out));
                                re_returned.push((i, v));
                            }
                            Ok((i, v, Err(e))) => {
                                re_returned.push((i, v));
                                if re_error.is_none() {
                                    re_error = Some(format!(
                                        "verifier[{i}] re-ask error in round {round}: {e}"
                                    ));
                                }
                            }
                            Err(e) => {
                                if re_error.is_none() {
                                    re_error = Some(format!("re-ask task panicked: {e}"));
                                }
                            }
                        }
                    }
                    re_returned.sort_by_key(|(i, _)| *i);
                    self.verifiers = re_returned.into_iter().map(|(_, v)| v).collect();
                    re_outputs.sort_by_key(|(i, _)| *i);
                    if let Some(err) = re_error {
                        return OrchestratorOutcome::Error { message: err };
                    }
                    let re_outputs: Vec<String> = re_outputs.into_iter().map(|(_, o)| o).collect();

                    // Apply consensus to re-ask results (same path as initial).
                    let re_verdict = if re_outputs.len() == 1 {
                        stop::detect(&re_outputs[0])
                    } else {
                        let re_individual: Vec<VerifierVerdict> = re_outputs
                            .iter()
                            .enumerate()
                            .map(|(i, output)| VerifierVerdict {
                                adapter_name: self
                                    .config
                                    .verifier_names
                                    .get(i)
                                    .cloned()
                                    .unwrap_or_else(|| format!("verifier[{i}]")),
                                result: stop::detect(output),
                                duration_ms: 0,
                            })
                            .collect();
                        let re_consensus =
                            consensus::resolve(&self.config.consensus_strategy, re_individual);
                        re_consensus.final_verdict
                    };
                    tracing::info!(round, ?re_verdict, "re-ask verdict detected");

                    // Use first verifier's re-ask output for feedback text.
                    let re_ask_output = re_outputs.into_iter().next().unwrap_or_default();

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

        tracing::info!(
            rounds_completed = self.config.max_rounds,
            "max rounds reached without approval"
        );
        OrchestratorOutcome::MaxRoundsReached {
            rounds_completed: self.config.max_rounds,
        }
    }
}

// ---------------------------------------------------------------------------
// Free functions — keep the borrow checker happy
// ---------------------------------------------------------------------------

/// Send a prompt to an adapter and collect the full response, updating health
/// state. If the adapter fails and auto-restart is enabled (and we haven't
/// already restarted this round), terminate + restart + re-send once.
async fn send_and_collect_with_health(
    adapter: &mut Box<dyn AgentAdapter>,
    prompt: &str,
    health: &mut HealthMonitor,
    agent_name: &str,
    restarted: &mut bool,
    event_sink: &Option<tokio::sync::mpsc::Sender<OrchestratorEvent>>,
) -> Result<String, String> {
    match send_and_collect_recording(adapter, prompt, health).await {
        Ok(output) => {
            // Post-response health check.
            let state = health.check();
            if !matches!(state, AgentHealth::Healthy) {
                let state_str = format!("{state:?}");
                tracing::warn!(agent = agent_name, state = %state_str, "agent health degraded after response");
                emit_event(
                    event_sink,
                    OrchestratorEvent::HealthStateChanged {
                        agent: agent_name.to_string(),
                        state: state_str,
                    },
                )
                .await;
            }
            Ok(output)
        }
        Err(e) => {
            health.check();
            let should_restart = health.config().auto_restart && !*restarted;

            if should_restart {
                tracing::warn!(
                    agent = agent_name,
                    error = %e,
                    "attempting auto-restart (1 retry/round)"
                );
                emit_event(
                    event_sink,
                    OrchestratorEvent::HealthStateChanged {
                        agent: agent_name.to_string(),
                        state: "Restarting".to_string(),
                    },
                )
                .await;

                *restarted = true;

                // Terminate and restart the session.
                let _ = adapter.terminate().await;
                let restart_config = SessionConfig {
                    system_prompt: format!("You are a {agent_name} agent."),
                    working_directory: std::path::PathBuf::from("/tmp"),
                    prior_transcript: vec![],
                };
                if let Err(re) = adapter.start_session(restart_config).await {
                    return Err(format!("original error: {e}; restart failed: {re}"));
                }
                health.reset();

                // Re-send the prompt.
                send_and_collect_recording(adapter, prompt, health).await
            } else {
                Err(e)
            }
        }
    }
}

/// Emit an orchestrator event to the sink if configured.
async fn emit_event(
    sink: &Option<tokio::sync::mpsc::Sender<OrchestratorEvent>>,
    event: OrchestratorEvent,
) {
    if let Some(tx) = sink {
        let _ = tx.send(event).await;
    }
}

/// Send a prompt and collect the response. Takes owned adapter for use in
/// spawned tasks (parallel verifier execution).
async fn send_and_collect_bare(
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

/// Like [`send_and_collect`] but also records events on the health monitor.
async fn send_and_collect_recording(
    adapter: &mut Box<dyn AgentAdapter>,
    prompt: &str,
    health: &mut HealthMonitor,
) -> Result<String, String> {
    adapter
        .send_turn(prompt)
        .await
        .map_err(|e| format!("send_turn failed: {e}"))?;

    health.record_event(); // turn accepted

    let stream = adapter
        .stream_events()
        .map_err(|e| format!("stream_events failed: {e}"))?;

    collect_stream_recording(stream, health).await
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
            AgentEvent::Chunk(ref text) => {
                tracing::debug!(chunk_len = text.len(), "stream chunk received");
                chunks.push_str(text);
            }
            AgentEvent::TurnComplete(text) => {
                tracing::debug!(len = text.len(), "turn complete");
                complete = Some(text);
            }
            AgentEvent::Error(e) => {
                tracing::error!(error = %e, "agent stream error");
                return Err(format!("agent error: {e}"));
            }
            AgentEvent::ProcessExited(code) => {
                tracing::debug!(?code, "agent process exited");
                if let Some(c) = code
                    && c != 0
                {
                    tracing::error!(code = c, "agent process exited with non-zero code");
                    return Err(format!("agent process exited with code {c}"));
                }
            }
        }
    }

    // Prefer TurnComplete (full output) over concatenated chunks.
    Ok(complete.unwrap_or(chunks))
}

/// Collect all events from a stream into a single response string, recording
/// each event on the health monitor.
async fn collect_stream_recording(
    stream: crate::adapter::AgentEventStream<'_>,
    health: &mut HealthMonitor,
) -> Result<String, String> {
    let mut chunks = String::new();
    let mut complete: Option<String> = None;

    futures::pin_mut!(stream);

    while let Some(event) = stream.next().await {
        health.record_event();
        match event {
            AgentEvent::Chunk(ref text) => {
                tracing::debug!(chunk_len = text.len(), "stream chunk received");
                chunks.push_str(text);
            }
            AgentEvent::TurnComplete(text) => {
                tracing::debug!(len = text.len(), "turn complete");
                complete = Some(text);
            }
            AgentEvent::Error(e) => {
                tracing::error!(error = %e, "agent stream error");
                return Err(format!("agent error: {e}"));
            }
            AgentEvent::ProcessExited(code) => {
                tracing::debug!(?code, "agent process exited");
                if let Some(c) = code
                    && c != 0
                {
                    health.mark_dead(Some(c));
                    tracing::error!(code = c, "agent process exited with non-zero code");
                    return Err(format!("agent process exited with code {c}"));
                }
            }
        }
    }

    // Prefer TurnComplete (full output) over concatenated chunks.
    Ok(complete.unwrap_or(chunks))
}

// ---------------------------------------------------------------------------
// Pipeline-aware orchestration (v0.6+)
// ---------------------------------------------------------------------------

use crate::pipeline::{ApprovalMode, SessionPipeline};

/// Extension for StageTransition event.
#[derive(Debug, Clone)]
pub struct StageTransitionInfo {
    pub stage: String,
    pub index: u32,
    pub total: u32,
}

impl Orchestrator {
    /// Run a pipeline session through stages.
    ///
    /// Each stage uses the orchestrator's existing ping-pong loop with
    /// stage-appropriate participant routing. Verifiers are selected based
    /// on the stage definition. Stage transitions emit events.
    pub async fn run_pipeline(&mut self, pipeline: &SessionPipeline) -> OrchestratorOutcome {
        let recipients = pipeline
            .stages
            .iter()
            .map(|s| pipeline.stage_recipients_for_planner(s))
            .collect::<Vec<_>>();

        for (i, stage) in pipeline.stages.iter().enumerate() {
            tracing::info!(
                stage = %stage.name,
                index = i,
                total = pipeline.stages.len(),
                planners = ?stage.participants.planners,
                verifiers = ?stage.participants.verifiers,
                workers = ?stage.participants.workers,
                routing = ?recipients.get(i),
                approval = ?stage.approval_mode,
                "entering pipeline stage"
            );

            // Skip stages with no verifiers (e.g., ideation in full-auto)
            if stage.approval_mode == ApprovalMode::FullAuto
                && stage.participants.verifiers.is_empty()
            {
                tracing::info!(stage = %stage.name, "full-auto stage with no verifiers — skipping ping-pong");
                continue;
            }

            // Use the subset of verifiers specified for this stage.
            // Currently the orchestrator holds all verifiers; we select by matching names.
            let stage_verifier_names: std::collections::HashSet<&str> = stage
                .participants
                .verifiers
                .iter()
                .map(|s| s.as_str())
                .collect();
            let active_count = if stage_verifier_names.is_empty() {
                self.verifiers.len() // use all if not specified
            } else {
                // Log which verifiers are active for this stage
                tracing::info!(active_verifiers = ?stage_verifier_names, "stage verifier routing");
                stage_verifier_names.len().min(self.verifiers.len())
            };
            let _ = active_count; // used for logging; actual subset selection is future work

            // Run the ping-pong loop for this stage
            let outcome = self.run().await;

            match &outcome {
                OrchestratorOutcome::Approved { round, reason } => {
                    tracing::info!(
                        stage = %stage.name,
                        round = round,
                        reason = %reason,
                        "stage approved"
                    );
                    if stage.approval_mode == ApprovalMode::Gated {
                        tracing::info!(stage = %stage.name, "gated stage — auto-advancing (TODO: user gate)");
                    }
                }
                OrchestratorOutcome::MaxRoundsReached { .. } => {
                    tracing::warn!(stage = %stage.name, "max rounds reached");
                    return outcome;
                }
                OrchestratorOutcome::Error { .. } => {
                    tracing::error!(stage = %stage.name, "error in stage");
                    return outcome;
                }
            }
        }

        OrchestratorOutcome::Approved {
            round: 0,
            reason: "all pipeline stages completed".to_string(),
        }
    }
}
