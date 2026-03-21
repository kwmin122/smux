use clap::{Parser, Subcommand};

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
        /// Planner provider (e.g. claude, gpt-4o)
        #[arg(long)]
        planner: Option<String>,

        /// Verifier provider
        #[arg(long)]
        verifier: Option<String>,

        /// Task description
        #[arg(long)]
        task: Option<String>,
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

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start {
            planner,
            verifier,
            task,
        } => {
            println!(
                "smux start: not yet implemented (planner={:?}, verifier={:?}, task={:?})",
                planner, verifier, task
            );
        }
        Commands::List => {
            println!("smux list: not yet implemented");
        }
        Commands::Rewind { session_id, round } => {
            println!(
                "smux rewind: not yet implemented (session={}, round={})",
                session_id, round
            );
        }
    }
}
