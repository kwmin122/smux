//! Unix socket IPC protocol between CLI and daemon.
//!
//! Wire format: length-prefixed JSON (4-byte big-endian length + JSON bytes).

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

// ---------------------------------------------------------------------------
// Client -> Daemon messages
// ---------------------------------------------------------------------------

/// Messages sent from the CLI to the daemon.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ClientMessage {
    /// Start a new planner-verifier session.
    StartSession {
        planner: String,
        verifier: String,
        task: String,
        max_rounds: u32,
        /// Additional verifiers for cross-verify mode (v0.3+).
        #[serde(default)]
        verifiers: Vec<String>,
        /// Consensus strategy (v0.3+). Defaults to Majority.
        #[serde(default)]
        consensus: String,
    },
    /// Attach to a running session to receive streamed events.
    AttachSession { session_id: String },
    /// Detach from the currently attached session.
    DetachSession,
    /// List all sessions.
    ListSessions,
    /// Rewind a session to a specific round.
    RewindSession { session_id: String, round: u32 },
    /// Send an intervention message to a specific agent in a session.
    Intervene {
        session_id: String,
        target: String,
        message: String,
    },
    /// Gracefully shut down the daemon.
    Shutdown,
    /// Start a session using a pipeline definition (v0.6+).
    /// Each stage carries its own participants, approval mode, and consensus.
    StartSessionWithPipeline {
        task: String,
        /// All agents in the session: (agent_id, role).
        agents: Vec<(String, String)>,
        /// Per-stage definitions with participants and policies.
        stages: Vec<IpcStageDefinition>,
        max_rounds: u32,
    },
}

/// Wire representation of a pipeline stage for IPC.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct IpcStageDefinition {
    pub name: String,
    pub approval_mode: String,
    pub consensus: String,
    pub planners: Vec<String>,
    pub verifiers: Vec<String>,
    pub workers: Vec<String>,
}

// ---------------------------------------------------------------------------
// Daemon -> Client messages
// ---------------------------------------------------------------------------

/// Messages sent from the daemon to the CLI.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum DaemonMessage {
    /// A session was successfully created.
    SessionCreated { session_id: String },
    /// Response to ListSessions.
    SessionList { sessions: Vec<SessionInfo> },
    /// Streamed output from an agent.
    AgentOutput { role: String, content: String },
    /// A round completed with the given verdict summary.
    RoundComplete { round: u32, verdict_summary: String },
    /// The session finished.
    SessionComplete { summary: String },
    /// Cross-verify result from multiple verifiers (v0.3+).
    CrossVerifyResult {
        round: u32,
        individual: Vec<VerifierVerdictInfo>,
        final_verdict: String,
        strategy: String,
        agreement_ratio: f64,
    },
    /// An error occurred.
    Error { message: String },
    /// Generic acknowledgement.
    Ok,
    /// A pipeline stage transition occurred (v0.6+).
    StageTransition {
        from: String,
        to: String,
        approval: String,
    },
}

/// Individual verifier verdict info for IPC (simplified for wire format).
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct VerifierVerdictInfo {
    pub verifier: String,
    pub verdict: String,
    pub confidence: f64,
    pub reason: String,
}

/// Summary info about a session (for list display).
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SessionInfo {
    pub id: String,
    pub task: String,
    pub planner: String,
    pub verifier: String,
    /// All verifiers (v0.3+). Empty means single-verifier mode.
    #[serde(default)]
    pub verifiers: Vec<String>,
    pub current_round: u32,
    pub status: String,
}

// ---------------------------------------------------------------------------
// Wire helpers: length-prefixed JSON
// ---------------------------------------------------------------------------

/// Send a message over a Unix socket using length-prefixed JSON.
///
/// Wire format: `[4-byte big-endian length][JSON bytes]`
pub async fn send_message<T: Serialize>(stream: &mut UnixStream, msg: &T) -> Result<(), IpcError> {
    let json = serde_json::to_vec(msg).map_err(IpcError::Serialize)?;
    let len = json.len() as u32;
    stream
        .write_all(&len.to_be_bytes())
        .await
        .map_err(IpcError::Io)?;
    stream.write_all(&json).await.map_err(IpcError::Io)?;
    stream.flush().await.map_err(IpcError::Io)?;
    Ok(())
}

/// Receive a message from a Unix socket using length-prefixed JSON.
///
/// Returns `IpcError::ConnectionClosed` if the peer hung up.
pub async fn recv_message<T: DeserializeOwned>(stream: &mut UnixStream) -> Result<T, IpcError> {
    let mut len_buf = [0u8; 4];
    match stream.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Err(IpcError::ConnectionClosed);
        }
        Err(e) => return Err(IpcError::Io(e)),
    }
    let len = u32::from_be_bytes(len_buf) as usize;

    // Sanity limit: 1 MiB (reduced from 16 MiB to limit local DoS surface)
    if len > 1024 * 1024 {
        return Err(IpcError::MessageTooLarge(len));
    }

    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await.map_err(IpcError::Io)?;
    serde_json::from_slice(&buf).map_err(IpcError::Deserialize)
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors from IPC operations.
#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialize(serde_json::Error),
    #[error("deserialization error: {0}")]
    Deserialize(serde_json::Error),
    #[error("connection closed by peer")]
    ConnectionClosed,
    #[error("message too large: {0} bytes")]
    MessageTooLarge(usize),
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Returns the default socket path: `~/.smux/smux.sock`
pub fn default_socket_path() -> std::path::PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    home.join(".smux").join("smux.sock")
}

/// Returns the default PID file path: `~/.smux/smux.pid`
pub fn default_pid_path() -> std::path::PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    home.join(".smux").join("smux.pid")
}
