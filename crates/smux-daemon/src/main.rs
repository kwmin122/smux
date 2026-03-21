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
    tracing::info!(path = %socket_path.display(), "daemon listening");

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
            } => {
                let session_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
                let (event_tx, _) = broadcast::channel(256);

                let handle = Arc::new(SessionHandle {
                    id: session_id.clone(),
                    task: task.clone(),
                    planner: planner.clone(),
                    verifier: verifier.clone(),
                    current_round: Arc::new(Mutex::new(0)),
                    status: Arc::new(Mutex::new(SessionStatus::Running)),
                    event_tx: event_tx.clone(),
                });

                state
                    .lock()
                    .await
                    .sessions
                    .insert(session_id.clone(), handle.clone());

                // Spawn the orchestrator task.
                spawn_session(handle.clone(), planner, verifier, task, max_rounds);

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
    verifier: String,
    task: String,
    max_rounds: u32,
) {
    tokio::spawn(async move {
        let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/tmp"));

        let planner_adapter =
            match smux_core::adapter::create_adapter(&planner, working_dir.clone()) {
                Ok(a) => a,
                Err(e) => {
                    let _ = handle.event_tx.send(DaemonMessage::SessionComplete {
                        summary: format!("failed to create planner adapter: {e}"),
                    });
                    *handle.status.lock().await = SessionStatus::Failed;
                    return;
                }
            };

        let verifier_adapter = match smux_core::adapter::create_adapter(&verifier, working_dir) {
            Ok(a) => a,
            Err(e) => {
                let _ = handle.event_tx.send(DaemonMessage::SessionComplete {
                    summary: format!("failed to create verifier adapter: {e}"),
                });
                *handle.status.lock().await = SessionStatus::Failed;
                return;
            }
        };

        let config = OrchestratorConfig {
            task,
            max_rounds,
            max_tokens: 0,
        };

        // VG-008: Wire event streaming from orchestrator to daemon broadcast.
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
                }
            }
        });

        let mut orchestrator =
            Orchestrator::new(planner_adapter, verifier_adapter, config).with_event_sink(event_tx);
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
