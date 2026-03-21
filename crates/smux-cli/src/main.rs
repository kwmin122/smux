use std::path::PathBuf;

use clap::{Parser, Subcommand};

use smux_core::adapter::create_adapter;
use smux_core::orchestrator::{Orchestrator, OrchestratorConfig, OrchestratorOutcome};

#[derive(Parser)]
#[command(name = "smux", about = "AI-multiplexed terminal sessions")]
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
        planner: String,

        /// Verifier provider (e.g. claude, codex)
        #[arg(long)]
        verifier: String,

        /// Task description
        #[arg(long)]
        task: String,

        /// Maximum planner-verifier rounds (default: 5)
        #[arg(long, default_value_t = 5)]
        max_rounds: u32,
    },

    /// List active sessions
    List,

    /// Rewind a session to a specific round
    Rewind {
        /// Session ID to rewind
        session_id: String,

        /// Round number to rewind to
        round: u32,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start {
            planner,
            verifier,
            task,
            max_rounds,
        } => {
            let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

            let planner_adapter = match create_adapter(&planner, working_dir.clone()) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("error: failed to create planner adapter: {e}");
                    std::process::exit(1);
                }
            };

            let verifier_adapter = match create_adapter(&verifier, working_dir) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("error: failed to create verifier adapter: {e}");
                    std::process::exit(1);
                }
            };

            let config = OrchestratorConfig {
                task: task.clone(),
                max_rounds,
                max_tokens: 0, // use default
            };

            let mut orchestrator = Orchestrator::new(planner_adapter, verifier_adapter, config);

            println!("smux: starting session (planner={planner}, verifier={verifier})");
            println!("smux: task = {task}");
            println!("smux: max_rounds = {max_rounds}");
            println!();

            let outcome = orchestrator.run().await;

            match outcome {
                OrchestratorOutcome::Approved { round, reason } => {
                    println!("smux: APPROVED at round {round}");
                    println!("smux: reason = {reason}");
                }
                OrchestratorOutcome::MaxRoundsReached { rounds_completed } => {
                    println!(
                        "smux: max rounds reached ({rounds_completed} rounds completed without approval)"
                    );
                    std::process::exit(1);
                }
                OrchestratorOutcome::Error { message } => {
                    eprintln!("smux: error — {message}");
                    std::process::exit(1);
                }
            }
        }
        Commands::List => {
            println!("no active sessions (daemon not implemented in v0.1)");
        }
        Commands::Rewind { session_id, round } => {
            println!(
                "rewind requires daemon (not implemented in v0.1) (session={session_id}, round={round})"
            );
        }
    }
}
