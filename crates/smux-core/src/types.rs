//! Core types for the smux session model.

use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// Declares what an adapter implementation supports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterCapabilities {
    /// True if the adapter can keep state across multiple turns without
    /// restarting the underlying process.
    pub persistent_session: bool,
    /// True if the adapter can stream partial output (Chunk events).
    pub streaming: bool,
    /// True if the adapter provides its own snapshot/restore mechanism.
    pub native_snapshot: bool,
}

/// Configuration passed to `AgentAdapter::start_session`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// System prompt injected at session start.
    pub system_prompt: String,
    /// Working directory for the agent process.
    pub working_directory: PathBuf,
    /// Optional prior conversation to restore context (for headless adapters
    /// that start a new process per turn).
    pub prior_transcript: Vec<Turn>,
}

/// Opaque serialized snapshot of adapter-internal state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    /// Identifier for the adapter type that produced this snapshot.
    pub adapter_type: String,
    /// Serialized internal state (format defined by each adapter).
    pub state: Vec<u8>,
}

/// A single conversational turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    /// Who produced this turn (e.g. "user", "assistant").
    pub role: String,
    /// The textual content of the turn.
    pub content: String,
    /// When this turn was recorded.
    pub timestamp: SystemTime,
}

/// Handle returned by `send_turn` — placeholder for v0.1.
///
/// In later versions this may carry a process ID or cancellation token.
#[derive(Debug, Clone)]
pub struct TurnHandle {
    /// Monotonically increasing turn index within the session.
    pub turn_index: u64,
}

/// Events emitted by an adapter while processing a turn.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// A partial chunk of streaming output.
    Chunk(String),
    /// The current turn finished successfully; carries full output.
    TurnComplete(String),
    /// A non-fatal error reported by the agent.
    Error(String),
    /// The underlying agent process exited (with optional exit code).
    ProcessExited(Option<i32>),
}

/// Result of a verification step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerifyResult {
    /// The output passes verification.
    Approved,
    /// The output is rejected for the given reason and category.
    Rejected {
        reason: String,
        category: RejectCategory,
    },
    /// More information is needed before a verdict can be made.
    NeedsInfo(String),
}

/// Why a verification was rejected.
///
/// These categories match the verifier verdict contract in the design spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RejectCategory {
    /// Workaround — not a root-cause fix.
    Mitigation,
    /// Test coverage is insufficient for the change.
    WeakTest,
    /// The change introduces a regression.
    Regression,
    /// The implementation is incomplete.
    IncompleteImpl,
    /// The change has a security vulnerability.
    SecurityIssue,
}
