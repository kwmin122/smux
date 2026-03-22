//! smux CLI — talks to the smux daemon via Unix socket IPC.

use std::process::Stdio;

use clap::{Parser, Subcommand};
use tokio::net::UnixStream;

use smux_core::config::{SmuxConfig, default_config_path};
use smux_core::ipc::{
    ClientMessage, DaemonMessage, IpcError, default_socket_path, recv_message, send_message,
};

#[derive(Parser)]
#[command(name = "smux", version, about = "AI-multiplexed terminal sessions")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a new smux session
    Start {
        /// Planner provider (e.g. claude, codex)
        #[arg(long)]
        planner: Option<String>,

        /// Verifier provider (e.g. claude, codex)
        #[arg(long)]
        verifier: Option<String>,

        /// Multiple verifiers for cross-verify (comma-separated, e.g. claude,codex,gemini)
        #[arg(long, value_delimiter = ',')]
        verifiers: Option<Vec<String>>,

        /// Consensus strategy (majority, weighted, unanimous, leader)
        #[arg(long)]
        consensus: Option<String>,

        /// Task description
        #[arg(long)]
        task: String,

        /// Maximum planner-verifier rounds
        #[arg(long)]
        max_rounds: Option<u32>,
    },

    /// Initialize config file at ~/.smux/config.toml
    Init,

    /// List active sessions
    List,

    /// Attach to a running session
    Attach {
        /// Session ID to attach to
        session_id: String,
    },

    /// Detach from the currently attached session
    Detach,

    /// Rewind a session to a specific round
    Rewind {
        /// Session ID to rewind
        session_id: String,

        /// Round number to rewind to
        round: u32,
    },

    /// List and recover orphaned/failed sessions
    Recover {
        /// Clean up sessions older than this many days
        #[arg(long)]
        cleanup_days: Option<u64>,
    },

    /// Manage the daemon process
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
}

#[derive(Subcommand)]
enum DaemonAction {
    /// Start the daemon in the background
    Start,
    /// Stop the running daemon
    Stop,
    /// Check daemon status
    Status,
}

#[tokio::main]
async fn main() {
    // Initialize tracing. Use SMUX_LOG env var for filter (e.g. SMUX_LOG=debug).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("SMUX_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .compact()
        .with_target(false)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            let config_path = default_config_path();
            if config_path.exists() {
                eprintln!("smux: config already exists at {}", config_path.display());
                eprintln!("smux: delete it first if you want to regenerate");
                std::process::exit(1);
            }

            match SmuxConfig::save_default(&config_path) {
                Ok(()) => {
                    println!("smux: created config at {}", config_path.display());
                }
                Err(e) => {
                    eprintln!("smux: failed to create config: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Start {
            planner,
            verifier,
            verifiers,
            consensus,
            task,
            max_rounds,
        } => {
            // Load config; CLI flags override config values.
            let config = match SmuxConfig::load() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("smux: warning: failed to load config, using defaults: {e}");
                    SmuxConfig::default()
                }
            };

            let planner = planner.unwrap_or(config.agents.planner.default.clone());
            let verifier = verifier.unwrap_or(config.agents.verifier.default.clone());
            let max_rounds = max_rounds.unwrap_or(config.defaults.max_rounds);
            let verifiers_list = verifiers.unwrap_or_default();
            let consensus_str = consensus.unwrap_or_else(|| "majority".into());

            // Ensure daemon is running.
            if let Err(e) = ensure_daemon_running().await {
                eprintln!("error: could not start daemon: {e}");
                std::process::exit(1);
            }

            let mut stream = match connect_to_daemon().await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not connect to daemon: {e}");
                    std::process::exit(1);
                }
            };

            let msg = ClientMessage::StartSession {
                planner: planner.clone(),
                verifier: verifier.clone(),
                task: task.clone(),
                max_rounds,
                verifiers: verifiers_list,
                consensus: consensus_str,
            };

            if let Err(e) = send_message(&mut stream, &msg).await {
                eprintln!("error: failed to send start message: {e}");
                std::process::exit(1);
            }

            // Receive SessionCreated, then stream events.
            match recv_message::<DaemonMessage>(&mut stream).await {
                Ok(DaemonMessage::SessionCreated { session_id }) => {
                    println!("smux: session created — {session_id}");
                    println!("smux: planner={planner}, verifier={verifier}");
                    println!("smux: task = {task}");
                    println!();
                }
                Ok(DaemonMessage::Error { message }) => {
                    eprintln!("smux: daemon error — {message}");
                    std::process::exit(1);
                }
                Ok(other) => {
                    eprintln!("smux: unexpected response — {other:?}");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("smux: receive error — {e}");
                    std::process::exit(1);
                }
            }

            // Auto-attach: stream events until session completes.
            stream_until_complete(&mut stream).await;
        }

        Commands::List => {
            if !daemon_is_running().await {
                println!("no active sessions (daemon not running)");
                return;
            }

            let mut stream = match connect_to_daemon().await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not connect to daemon: {e}");
                    std::process::exit(1);
                }
            };

            if let Err(e) = send_message(&mut stream, &ClientMessage::ListSessions).await {
                eprintln!("error: failed to send list message: {e}");
                std::process::exit(1);
            }

            match recv_message::<DaemonMessage>(&mut stream).await {
                Ok(DaemonMessage::SessionList { sessions }) => {
                    if sessions.is_empty() {
                        println!("no active sessions");
                    } else {
                        println!(
                            "{:<10} {:<12} {:<10} {:<10} {:<8} TASK",
                            "ID", "STATUS", "PLANNER", "VERIFIER", "ROUND"
                        );
                        for s in &sessions {
                            println!(
                                "{:<10} {:<12} {:<10} {:<10} {:<8} {}",
                                s.id, s.status, s.planner, s.verifier, s.current_round, s.task
                            );
                        }
                    }
                }
                Ok(DaemonMessage::Error { message }) => {
                    eprintln!("smux: error — {message}");
                    std::process::exit(1);
                }
                Ok(other) => {
                    eprintln!("smux: unexpected response — {other:?}");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("smux: receive error — {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Attach { session_id } => {
            let mut stream = match connect_to_daemon().await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not connect to daemon: {e}");
                    std::process::exit(1);
                }
            };

            let msg = ClientMessage::AttachSession {
                session_id: session_id.clone(),
            };

            if let Err(e) = send_message(&mut stream, &msg).await {
                eprintln!("error: failed to send attach message: {e}");
                std::process::exit(1);
            }

            match recv_message::<DaemonMessage>(&mut stream).await {
                Ok(DaemonMessage::Ok) => {
                    println!("smux: attached to session {session_id}");
                    stream_until_complete(&mut stream).await;
                }
                Ok(DaemonMessage::Error { message }) => {
                    eprintln!("smux: error — {message}");
                    std::process::exit(1);
                }
                Ok(other) => {
                    eprintln!("smux: unexpected response — {other:?}");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("smux: receive error — {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Detach => {
            let mut stream = match connect_to_daemon().await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not connect to daemon: {e}");
                    std::process::exit(1);
                }
            };

            if let Err(e) = send_message(&mut stream, &ClientMessage::DetachSession).await {
                eprintln!("error: failed to send detach message: {e}");
                std::process::exit(1);
            }

            match recv_message::<DaemonMessage>(&mut stream).await {
                Ok(DaemonMessage::Ok) => {
                    println!("smux: detached from session");
                }
                Ok(DaemonMessage::Error { message }) => {
                    eprintln!("smux: error — {message}");
                    std::process::exit(1);
                }
                Ok(other) => {
                    eprintln!("smux: unexpected response — {other:?}");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("smux: receive error — {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Rewind { session_id, round } => {
            let mut stream = match connect_to_daemon().await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not connect to daemon: {e}");
                    std::process::exit(1);
                }
            };

            let msg = ClientMessage::RewindSession {
                session_id: session_id.clone(),
                round,
            };

            if let Err(e) = send_message(&mut stream, &msg).await {
                eprintln!("error: failed to send rewind message: {e}");
                std::process::exit(1);
            }

            match recv_message::<DaemonMessage>(&mut stream).await {
                Ok(DaemonMessage::Ok) => {
                    println!("smux: rewound session {session_id} to round {round}");
                }
                Ok(DaemonMessage::Error { message }) => {
                    eprintln!("smux: error — {message}");
                    std::process::exit(1);
                }
                Ok(other) => {
                    eprintln!("smux: unexpected response — {other:?}");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("smux: receive error — {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Recover { cleanup_days } => {
            if let Some(days) = cleanup_days {
                match smux_core::session_store::cleanup_old_sessions(days) {
                    Ok(0) => println!("smux: no sessions older than {days} days to clean up"),
                    Ok(n) => println!("smux: removed {n} session(s) older than {days} days"),
                    Err(e) => {
                        eprintln!("smux: cleanup failed: {e}");
                        std::process::exit(1);
                    }
                }
            }

            match smux_core::session_store::list_all_sessions() {
                Ok(sessions) if sessions.is_empty() => {
                    println!("smux: no sessions found in ~/.smux/sessions/");
                }
                Ok(sessions) => {
                    println!(
                        "{:<14} {:<12} {:<8} {:<8} {:<6} TASK",
                        "ID", "STATUS", "PLANNER", "VERIFIER", "ROUND"
                    );
                    for s in &sessions {
                        let status = format!("{:?}", s.status);
                        println!(
                            "{:<14} {:<12} {:<8} {:<8} {:<6} {}",
                            s.id, status, s.planner, s.verifier, s.current_round, s.task
                        );
                    }
                    println!();
                    println!(
                        "smux: {} session(s) found. Use `smux recover --cleanup-days N` to remove old ones.",
                        sessions.len()
                    );
                }
                Err(e) => {
                    eprintln!("smux: failed to list sessions: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::Daemon { action } => match action {
            DaemonAction::Start => {
                if daemon_is_running().await {
                    println!("smux: daemon is already running");
                    return;
                }
                match start_daemon_background().await {
                    Ok(()) => println!("smux: daemon started"),
                    Err(e) => {
                        eprintln!("smux: failed to start daemon: {e}");
                        std::process::exit(1);
                    }
                }
            }
            DaemonAction::Stop => {
                if !daemon_is_running().await {
                    println!("smux: daemon is not running");
                    return;
                }
                match connect_to_daemon().await {
                    Ok(mut stream) => {
                        let _ = send_message(&mut stream, &ClientMessage::Shutdown).await;
                        println!("smux: daemon stopped");
                    }
                    Err(e) => {
                        eprintln!("smux: failed to stop daemon: {e}");
                        std::process::exit(1);
                    }
                }
            }
            DaemonAction::Status => {
                if daemon_is_running().await {
                    println!("smux: daemon is running");
                } else {
                    println!("smux: daemon is not running");
                }
            }
        },
    }
}

// ---------------------------------------------------------------------------
// Daemon lifecycle helpers
// ---------------------------------------------------------------------------

/// Check if the daemon is reachable.
async fn daemon_is_running() -> bool {
    connect_to_daemon().await.is_ok()
}

/// Connect to the daemon's Unix socket.
async fn connect_to_daemon() -> Result<UnixStream, IpcError> {
    let socket_path = default_socket_path();
    UnixStream::connect(&socket_path)
        .await
        .map_err(IpcError::Io)
}

/// Ensure the daemon is running; start it in the background if not.
async fn ensure_daemon_running() -> Result<(), String> {
    if daemon_is_running().await {
        return Ok(());
    }
    start_daemon_background().await?;

    // Wait for the daemon to become reachable (up to 3 seconds).
    for _ in 0..30 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if daemon_is_running().await {
            return Ok(());
        }
    }
    Err("daemon did not start within 3 seconds".into())
}

/// Start the daemon binary as a detached background process.
async fn start_daemon_background() -> Result<(), String> {
    // Find the smux-daemon binary next to our own binary.
    let self_path = std::env::current_exe().map_err(|e| format!("cannot find self: {e}"))?;
    let daemon_path = self_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("smux-daemon");

    if !daemon_path.exists() {
        return Err(format!(
            "daemon binary not found at {}",
            daemon_path.display()
        ));
    }

    std::process::Command::new(&daemon_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to spawn daemon: {e}"))?;

    Ok(())
}

/// Stream events from the daemon to stdout until SessionComplete.
async fn stream_until_complete(stream: &mut UnixStream) {
    loop {
        match recv_message::<DaemonMessage>(stream).await {
            Ok(DaemonMessage::AgentOutput { role, content }) => {
                println!("[{role}] {content}");
            }
            Ok(DaemonMessage::RoundComplete {
                round,
                verdict_summary,
            }) => {
                println!("--- round {round} complete: {verdict_summary} ---");
            }
            Ok(DaemonMessage::SessionComplete { summary }) => {
                println!();
                println!("smux: session complete — {summary}");
                return;
            }
            Ok(DaemonMessage::CrossVerifyResult {
                round,
                individual,
                final_verdict,
                strategy,
                agreement_ratio,
            }) => {
                println!();
                println!("=== Cross-Verify (round {round}) ===");
                for v in &individual {
                    println!(
                        "  {}: {} (confidence: {:.0}%) — {}",
                        v.verifier,
                        v.verdict,
                        v.confidence * 100.0,
                        v.reason
                    );
                }
                println!(
                    "  Final: {final_verdict} ({strategy}, {:.0}% agreement)",
                    agreement_ratio * 100.0
                );
                println!();
            }
            Ok(DaemonMessage::Error { message }) => {
                eprintln!("smux: error — {message}");
                return;
            }
            Ok(other) => {
                eprintln!("smux: unexpected message — {other:?}");
            }
            Err(IpcError::ConnectionClosed) => {
                eprintln!("smux: connection to daemon lost");
                return;
            }
            Err(e) => {
                eprintln!("smux: receive error — {e}");
                return;
            }
        }
    }
}
