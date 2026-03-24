//! smux-daemon — background process that owns sessions and serves IPC.
//!
//! Listens on a Unix socket at `~/.smux/smux.sock`.
//! Spawns an async task per session (orchestrator run).
//! Clients connect, send [`ClientMessage`]s, receive [`DaemonMessage`]s.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{Mutex, broadcast};

use tracing_subscriber::fmt::writer::MakeWriterExt;

use smux_core::config::SmuxConfig;
use smux_core::ipc::{
    ClientMessage, DaemonMessage, IpcError, SessionInfo, default_pid_path, default_socket_path,
    recv_message, send_message,
};
use smux_core::orchestrator::{Orchestrator, OrchestratorConfig, OrchestratorOutcome};

// ---------------------------------------------------------------------------
// Session handle — tracks a running or completed session
// ---------------------------------------------------------------------------

/// Status of a daemon-managed session.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SessionStatus {
    Running,
    Completed,
    Failed,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

/// Daemon-side handle for a session.
struct SessionHandle {
    id: String,
    task: String,
    planner: String,
    verifier: String,
    /// All verifier names (v0.3+). Non-empty when using cross-verify.
    verifiers: Vec<String>,
    current_round: Arc<Mutex<u32>>,
    status: Arc<Mutex<SessionStatus>>,
    /// Broadcast channel for events (AgentOutput, RoundComplete, SessionComplete).
    event_tx: broadcast::Sender<DaemonMessage>,
}

// ---------------------------------------------------------------------------
// Daemon state
// ---------------------------------------------------------------------------

/// Shared daemon state behind `Arc<Mutex<_>>`.
pub struct DaemonState {
    sessions: HashMap<String, Arc<SessionHandle>>,
    shutdown: bool,
    #[allow(dead_code)]
    config: SmuxConfig,
}

impl DaemonState {
    fn new(config: SmuxConfig) -> Self {
        Self {
            sessions: HashMap::new(),
            shutdown: false,
            config,
        }
    }
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // Set up file-based logging to ~/.smux/logs/daemon.log alongside stderr.
    let log_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".smux/logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let file_appender = tracing_appender::rolling::never(&log_dir, "daemon.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = tracing_subscriber::EnvFilter::try_from_env("SMUX_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr.and(non_blocking))
        .init();

    let socket_path = std::env::var("SMUX_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_socket_path());

    if let Err(e) = run_daemon(&socket_path).await {
        tracing::error!(?e, "daemon exited with error");
        std::process::exit(1);
    }
}

/// Run the daemon: bind socket, write PID file, accept connections.
pub async fn run_daemon(socket_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure parent directory exists.
    if let Some(parent) = socket_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Remove stale socket file if present.
    if socket_path.exists() {
        tokio::fs::remove_file(&socket_path).await?;
    }

    let listener = UnixListener::bind(socket_path)?;

    // Restrict socket to owner-only access (0o600) to prevent unauthorized
    // local processes from connecting and controlling sessions.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(socket_path, std::fs::Permissions::from_mode(0o600))?;
    }

    tracing::info!(path = %socket_path.display(), "daemon listening (socket permissions: 0600)");

    // Write PID file.
    let pid_path = std::env::var("SMUX_PID_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_pid_path());
    tokio::fs::write(&pid_path, std::process::id().to_string()).await?;
    tracing::info!(pid_path = %pid_path.display(), pid = std::process::id(), "PID file written");

    // Load config at startup.
    let config = SmuxConfig::load().unwrap_or_else(|e| {
        tracing::warn!(?e, "failed to load config, using defaults");
        SmuxConfig::default()
    });
    tracing::info!("config loaded (max_rounds={})", config.defaults.max_rounds);

    let state = Arc::new(Mutex::new(DaemonState::new(config)));

    // Spawn Ctrl-C handler.
    let state_signal = state.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("received SIGINT, shutting down");
        state_signal.lock().await.shutdown = true;
    });

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, _addr)) => {
                        // Validate peer credentials — reject connections from other UIDs.
                        #[cfg(unix)]
                        {
                            use std::os::unix::io::AsRawFd;
                            let fd = stream.as_raw_fd();
                            let mut peer_uid: libc::uid_t = 0;
                            let mut peer_gid: libc::gid_t = 0;
                            let ret = unsafe {
                                libc::getpeereid(fd, &mut peer_uid, &mut peer_gid)
                            };
                            if ret == 0 {
                                let my_uid = unsafe { libc::getuid() };
                                if peer_uid != my_uid {
                                    tracing::warn!(
                                        peer_uid,
                                        my_uid,
                                        "rejected connection from different UID"
                                    );
                                    continue; // drop the connection
                                }
                            } else {
                                tracing::warn!(
                                    errno = std::io::Error::last_os_error().to_string(),
                                    "failed to get peer credentials, rejecting"
                                );
                                continue;
                            }
                        }
                        let state = state.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_client(stream, state).await {
                                tracing::warn!(?e, "client handler error");
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!(?e, "accept error");
                    }
                }
            }
            _ = check_shutdown(state.clone()) => {
                tracing::info!("shutdown flag set, exiting accept loop");
                break;
            }
        }
    }

    // Cleanup: remove socket and PID file.
    let _ = tokio::fs::remove_file(socket_path).await;
    let _ = tokio::fs::remove_file(&pid_path).await;
    tracing::info!("daemon shut down");
    Ok(())
}

/// Poll until the shutdown flag is set.
async fn check_shutdown(state: Arc<Mutex<DaemonState>>) {
    loop {
        if state.lock().await.shutdown {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

// ---------------------------------------------------------------------------
// Per-client connection handler
// ---------------------------------------------------------------------------

async fn handle_client(
    mut stream: UnixStream,
    state: Arc<Mutex<DaemonState>>,
) -> Result<(), IpcError> {
    loop {
        let msg: ClientMessage = match recv_message(&mut stream).await {
            Ok(m) => m,
            Err(IpcError::ConnectionClosed) => {
                tracing::debug!("client disconnected");
                return Ok(());
            }
            Err(e) => return Err(e),
        };

        tracing::debug!(?msg, "received client message");

        match msg {
            ClientMessage::StartSession {
                planner,
                verifier,
                task,
                max_rounds,
                verifiers,
                consensus,
            } => {
                // Build the effective verifier list. The primary `verifier` field
                // is always included. If `verifiers` (extras) were provided, merge
                // them: primary first, then extras (deduplicating the primary if the
                // user also listed it in extras).
                let effective_verifiers = if verifiers.is_empty() {
                    vec![verifier.clone()]
                } else {
                    let mut merged = vec![verifier.clone()];
                    for v in verifiers {
                        if v != verifier {
                            merged.push(v);
                        }
                    }
                    merged
                };

                let session_id = uuid::Uuid::new_v4().to_string();
                let (event_tx, _) = broadcast::channel(256);

                let handle = Arc::new(SessionHandle {
                    id: session_id.clone(),
                    task: task.clone(),
                    planner: planner.clone(),
                    verifier: effective_verifiers.first().cloned().unwrap_or_default(),
                    verifiers: effective_verifiers.clone(),
                    current_round: Arc::new(Mutex::new(0)),
                    status: Arc::new(Mutex::new(SessionStatus::Running)),
                    event_tx: event_tx.clone(),
                });

                state
                    .lock()
                    .await
                    .sessions
                    .insert(session_id.clone(), handle.clone());

                // Spawn the orchestrator task with all verifiers.
                let safety_config = state.lock().await.config.safety.clone();
                let config_max_rounds = state.lock().await.config.defaults.max_rounds;
                // Server-side cap: client cannot exceed configured max_rounds.
                let capped_max_rounds = max_rounds.min(config_max_rounds.max(1));
                spawn_session(
                    handle.clone(),
                    planner,
                    effective_verifiers,
                    consensus,
                    task,
                    capped_max_rounds,
                    safety_config,
                );

                send_message(
                    &mut stream,
                    &DaemonMessage::SessionCreated {
                        session_id: session_id.clone(),
                    },
                )
                .await?;

                // Auto-attach: stream events to this client until session completes.
                stream_events_to_client(&mut stream, &handle).await?;
            }

            ClientMessage::AttachSession { session_id } => {
                let handle = {
                    let s = state.lock().await;
                    s.sessions.get(&session_id).cloned()
                };

                match handle {
                    Some(h) => {
                        send_message(&mut stream, &DaemonMessage::Ok).await?;
                        stream_events_to_client(&mut stream, &h).await?;
                    }
                    None => {
                        send_message(
                            &mut stream,
                            &DaemonMessage::Error {
                                message: format!("session not found: {session_id}"),
                            },
                        )
                        .await?;
                    }
                }
            }

            ClientMessage::DetachSession => {
                send_message(&mut stream, &DaemonMessage::Ok).await?;
                // Just break this event-streaming loop; client stays connected.
            }

            ClientMessage::ListSessions => {
                let s = state.lock().await;
                let mut sessions = Vec::new();
                for handle in s.sessions.values() {
                    let round = *handle.current_round.lock().await;
                    let status = handle.status.lock().await.to_string();
                    sessions.push(SessionInfo {
                        id: handle.id.clone(),
                        task: handle.task.clone(),
                        planner: handle.planner.clone(),
                        verifier: handle.verifier.clone(),
                        verifiers: handle.verifiers.clone(),
                        current_round: round,
                        status,
                    });
                }
                send_message(&mut stream, &DaemonMessage::SessionList { sessions }).await?;
            }

            ClientMessage::Intervene {
                session_id,
                target,
                message,
            } => {
                let handle = {
                    let s = state.lock().await;
                    s.sessions.get(&session_id).cloned()
                };

                match handle {
                    Some(h) => {
                        // For v0.2, forward the intervention as an AgentOutput event on
                        // the session broadcast so attached clients can see it, and log
                        // which agent was targeted.
                        let _ = h.event_tx.send(DaemonMessage::AgentOutput {
                            role: format!("intervene:{target}"),
                            content: message,
                        });
                        send_message(&mut stream, &DaemonMessage::Ok).await?;
                    }
                    None => {
                        send_message(
                            &mut stream,
                            &DaemonMessage::Error {
                                message: format!("session not found: {session_id}"),
                            },
                        )
                        .await?;
                    }
                }
            }

            ClientMessage::RewindSession { session_id, round } => {
                let handle = {
                    let s = state.lock().await;
                    s.sessions.get(&session_id).cloned()
                };

                match handle {
                    Some(h) => {
                        // For v0.2, rewind is a placeholder — just update the round counter.
                        *h.current_round.lock().await = round;
                        send_message(&mut stream, &DaemonMessage::Ok).await?;
                    }
                    None => {
                        send_message(
                            &mut stream,
                            &DaemonMessage::Error {
                                message: format!("session not found: {session_id}"),
                            },
                        )
                        .await?;
                    }
                }
            }

            ClientMessage::Shutdown => {
                tracing::info!("shutdown requested by client");
                state.lock().await.shutdown = true;
                send_message(&mut stream, &DaemonMessage::Ok).await?;
                return Ok(());
            }

            ClientMessage::StartSessionWithPipeline { task, agents, .. } => {
                tracing::info!(task = %task, agents = ?agents, "pipeline session requested (v0.6+)");
                // For now, extract first planner/verifier and delegate to existing session logic
                let planner = agents
                    .iter()
                    .find(|(_, r)| r == "planner")
                    .map(|(id, _)| id.clone())
                    .unwrap_or_else(|| "claude".to_string());
                let verifier = agents
                    .iter()
                    .find(|(_, r)| r == "verifier")
                    .map(|(id, _)| id.clone())
                    .unwrap_or_else(|| "codex".to_string());
                send_message(&mut stream, &DaemonMessage::Error {
                    message: format!("pipeline sessions not yet fully implemented — use planner={planner} verifier={verifier} for now"),
                }).await?;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Event streaming to attached client
// ---------------------------------------------------------------------------

async fn stream_events_to_client(
    stream: &mut UnixStream,
    handle: &SessionHandle,
) -> Result<(), IpcError> {
    let mut rx = handle.event_tx.subscribe();

    loop {
        match rx.recv().await {
            Ok(msg) => {
                send_message(stream, &msg).await?;
                // If session is complete, stop streaming.
                if matches!(msg, DaemonMessage::SessionComplete { .. }) {
                    return Ok(());
                }
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!(n, "client lagged behind, skipped {n} events");
            }
            Err(broadcast::error::RecvError::Closed) => {
                // Session task dropped the sender — session is done.
                return Ok(());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Spawn orchestrator session task
// ---------------------------------------------------------------------------

fn spawn_session(
    handle: Arc<SessionHandle>,
    planner: String,
    verifiers: Vec<String>,
    consensus: String,
    task: String,
    max_rounds: u32,
    safety_config: smux_core::config::SafetyConfig,
) {
    tokio::spawn(async move {
        let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/tmp"));

        let planner_adapter = match smux_core::adapter::create_adapter_with_safety(
            &planner,
            working_dir.clone(),
            safety_config.clone(),
        ) {
            Ok(a) => a,
            Err(e) => {
                let _ = handle.event_tx.send(DaemonMessage::SessionComplete {
                    summary: format!("failed to create planner adapter: {e}"),
                });
                *handle.status.lock().await = SessionStatus::Failed;
                return;
            }
        };

        // Create adapter for each verifier with safety config applied.
        // Skip unavailable verifiers (e.g. gemini CLI not installed) with warning.
        let mut verifier_adapters: Vec<Box<dyn smux_core::adapter::AgentAdapter>> = Vec::new();
        for v_name in &verifiers {
            // Check availability for gemini adapter before creating.
            if v_name == "gemini"
                && !smux_core::adapter::gemini::GeminiHeadlessAdapter::is_available().await
            {
                tracing::warn!(
                    verifier = v_name,
                    "gemini CLI not available, skipping verifier"
                );
                let _ = handle.event_tx.send(DaemonMessage::AgentOutput {
                    role: "system".into(),
                    content: format!(
                        "Warning: gemini CLI not available, skipping verifier '{v_name}'"
                    ),
                });
                continue;
            }
            match smux_core::adapter::create_adapter_with_safety(
                v_name,
                working_dir.clone(),
                safety_config.clone(),
            ) {
                Ok(a) => verifier_adapters.push(a),
                Err(e) => {
                    let _ = handle.event_tx.send(DaemonMessage::SessionComplete {
                        summary: format!("failed to create verifier adapter '{v_name}': {e}"),
                    });
                    *handle.status.lock().await = SessionStatus::Failed;
                    return;
                }
            }
        }

        if verifier_adapters.is_empty() {
            let _ = handle.event_tx.send(DaemonMessage::SessionComplete {
                summary: "no verifiers available — all requested verifiers were unavailable"
                    .to_string(),
            });
            *handle.status.lock().await = SessionStatus::Failed;
            return;
        }

        // Parse consensus strategy from string.
        let consensus_strategy = match consensus.to_lowercase().as_str() {
            "weighted" => smux_core::types::ConsensusStrategy::Weighted,
            "unanimous" => smux_core::types::ConsensusStrategy::Unanimous,
            "leader" | "leaderdelegate" => smux_core::types::ConsensusStrategy::LeaderDelegate,
            other => {
                tracing::warn!(
                    strategy = other,
                    "unknown consensus strategy, defaulting to majority"
                );
                smux_core::types::ConsensusStrategy::Majority
            }
        };

        let config = OrchestratorConfig {
            task,
            max_rounds,
            max_tokens: 0,
            health_config: None,
            consensus_strategy,
            verifier_names: verifiers.clone(),
        };

        // Wire event streaming from orchestrator to daemon broadcast.
        let (event_tx, mut event_rx) =
            tokio::sync::mpsc::channel::<smux_core::orchestrator::OrchestratorEvent>(256);
        let broadcast_tx = handle.event_tx.clone();
        let round_counter = handle.current_round.clone();

        // Spawn a task that forwards orchestrator events to the broadcast channel.
        tokio::spawn(async move {
            use smux_core::orchestrator::OrchestratorEvent;
            while let Some(event) = event_rx.recv().await {
                match &event {
                    OrchestratorEvent::RoundStarted { round } => {
                        *round_counter.lock().await = *round;
                    }
                    OrchestratorEvent::PlannerOutput { round: _, content } => {
                        let _ = broadcast_tx.send(DaemonMessage::AgentOutput {
                            role: "planner".into(),
                            content: content.clone(),
                        });
                    }
                    OrchestratorEvent::VerifierOutput { round: _, content } => {
                        let _ = broadcast_tx.send(DaemonMessage::AgentOutput {
                            role: "verifier".into(),
                            content: content.clone(),
                        });
                    }
                    OrchestratorEvent::RoundComplete { round, verdict } => {
                        let summary = format!("{verdict:?}");
                        let _ = broadcast_tx.send(DaemonMessage::RoundComplete {
                            round: *round,
                            verdict_summary: summary,
                        });
                    }
                    OrchestratorEvent::CrossVerifyResult { round, result } => {
                        let individual = result
                            .individual
                            .iter()
                            .map(|v| {
                                let (verdict_str, confidence, reason) = match &v.result {
                                    smux_core::types::VerifyResult::Approved {
                                        reason,
                                        confidence,
                                    } => ("APPROVED".to_string(), *confidence, reason.clone()),
                                    smux_core::types::VerifyResult::Rejected {
                                        reason,
                                        confidence,
                                        ..
                                    } => ("REJECTED".to_string(), *confidence, reason.clone()),
                                    smux_core::types::VerifyResult::NeedsInfo { question } => {
                                        ("NEEDS_INFO".to_string(), 0.0, question.clone())
                                    }
                                };
                                smux_core::ipc::VerifierVerdictInfo {
                                    verifier: v.adapter_name.clone(),
                                    verdict: verdict_str,
                                    confidence,
                                    reason,
                                }
                            })
                            .collect();

                        let final_str = match &result.final_verdict {
                            smux_core::types::VerifyResult::Approved { .. } => "APPROVED",
                            smux_core::types::VerifyResult::Rejected { .. } => "REJECTED",
                            smux_core::types::VerifyResult::NeedsInfo { .. } => "NEEDS_INFO",
                        };

                        let _ = broadcast_tx.send(DaemonMessage::CrossVerifyResult {
                            round: *round,
                            individual,
                            final_verdict: final_str.to_string(),
                            strategy: format!("{:?}", result.strategy),
                            agreement_ratio: result.agreement_ratio,
                        });
                    }
                    OrchestratorEvent::HealthStateChanged { agent, state } => {
                        let _ = broadcast_tx.send(DaemonMessage::AgentOutput {
                            role: format!("health:{agent}"),
                            content: state.clone(),
                        });
                    }
                    OrchestratorEvent::SafetyAlert {
                        round,
                        severity,
                        message,
                    } => {
                        let _ = broadcast_tx.send(DaemonMessage::AgentOutput {
                            role: format!("safety:{severity}"),
                            content: format!("[round {round}] {message}"),
                        });
                    }
                }
            }
        });

        let mut orchestrator = Orchestrator::new_multi(planner_adapter, verifier_adapters, config)
            .with_event_sink(event_tx);
        let outcome = orchestrator.run().await;

        let summary = match &outcome {
            OrchestratorOutcome::Approved { round, reason } => {
                format!("APPROVED at round {round}: {reason}")
            }
            OrchestratorOutcome::MaxRoundsReached { rounds_completed } => {
                format!("max rounds reached ({rounds_completed})")
            }
            OrchestratorOutcome::Error { message } => {
                format!("error: {message}")
            }
        };

        let _ = handle
            .event_tx
            .send(DaemonMessage::SessionComplete { summary });

        let new_status = match outcome {
            OrchestratorOutcome::Approved { .. } => SessionStatus::Completed,
            _ => SessionStatus::Failed,
        };
        *handle.status.lock().await = new_status;
    });
}
