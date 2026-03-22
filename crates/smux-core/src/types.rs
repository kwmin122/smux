//! Core types for the smux session model.

use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Round & session types (commit-per-round rewind)
// ---------------------------------------------------------------------------

/// Snapshot of a single round's outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundSnapshot {
    /// Round number (1-based).
    pub round: u32,
    /// Git commit SHA captured after the round's changes were committed.
    pub commit_sha: String,
    /// Path to the planner context file used for this round.
    pub planner_context_path: PathBuf,
    /// Path to the verifier context file used for this round.
    pub verifier_context_path: PathBuf,
    /// The verifier's verdict for this round.
    pub verdict: VerifyResult,
    /// List of files changed in this round.
    pub files_changed: Vec<String>,
    /// ISO 8601 timestamp of when this round was committed.
    pub timestamp: String,
    /// Cross-verification result (present when multiple verifiers are used).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cross_verify: Option<ConsensusResult>,
}

/// Metadata for an entire smux session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    /// Unique session identifier.
    pub id: String,
    /// Task description / goal.
    pub task: String,
    /// Planner adapter identifier (e.g. "claude", "codex").
    pub planner: String,
    /// Verifier adapter identifier (backward compat: primary verifier).
    pub verifier: String,
    /// All verifier adapter identifiers (v0.3+). If empty, falls back to `verifier`.
    #[serde(default)]
    pub verifiers: Vec<String>,
    /// Consensus strategy for multi-verifier mode.
    #[serde(default)]
    pub consensus_strategy: ConsensusStrategy,
    /// Current round number.
    pub current_round: u32,
    /// Session status.
    pub status: SessionStatus,
    /// Path to the git worktree for this session.
    pub worktree_path: PathBuf,
    /// ISO 8601 timestamp of when the session was created.
    pub created_at: String,
}

/// Status of a smux session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    /// The session is actively running rounds.
    InProgress,
    /// The session completed successfully (verifier approved).
    Completed,
    /// The session was rewound to a previous round.
    Rewound,
}

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
///
/// Shape matches the verifier verdict contract in the design spec:
/// `{"verdict": "APPROVED"|"REJECTED", "category": "...", "reason": "...", "confidence": 0.0-1.0}`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VerifyResult {
    /// The output passes verification.
    Approved { reason: String, confidence: f64 },
    /// The output is rejected for the given reason and category.
    Rejected {
        reason: String,
        category: RejectCategory,
        confidence: f64,
    },
    /// More information is needed before a verdict can be made.
    NeedsInfo { question: String },
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

// ---------------------------------------------------------------------------
// Cross-verify / consensus types (v0.3)
// ---------------------------------------------------------------------------

/// Strategy for combining multiple verifier verdicts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ConsensusStrategy {
    /// More than 50% must approve.
    #[default]
    Majority,
    /// Confidence-weighted average > 0.5 → approved.
    Weighted,
    /// All verifiers must approve.
    Unanimous,
    /// Leader model decides after seeing other verdicts.
    LeaderDelegate,
}

/// Individual verifier's verdict in a cross-verify round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifierVerdict {
    /// Which adapter produced this verdict (e.g. "claude", "codex").
    pub adapter_name: String,
    /// The verdict itself.
    pub result: VerifyResult,
    /// Wall-clock time this verifier took, in milliseconds.
    pub duration_ms: u64,
}

/// Result of applying a consensus strategy to multiple verifier verdicts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    /// Individual verdicts from each verifier.
    pub individual: Vec<VerifierVerdict>,
    /// The final combined verdict.
    pub final_verdict: VerifyResult,
    /// Which strategy was used.
    pub strategy: ConsensusStrategy,
    /// Agreement ratio (0.0–1.0): fraction of verifiers that agreed with the final verdict.
    pub agreement_ratio: f64,
}
