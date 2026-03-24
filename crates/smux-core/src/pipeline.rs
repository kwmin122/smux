//! Multi-agent session pipeline types.
//!
//! Models sessions as a stage pipeline with participant slots rather than
//! a fixed planner/verifier pair. Routing is derived from stage definitions.

use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Agent roles
// ---------------------------------------------------------------------------

/// Role an agent can play in a session pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Planner,
    Verifier,
    Worker,
    Integrator,
    Auditor,
}

impl fmt::Display for AgentRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentRole::Planner => write!(f, "planner"),
            AgentRole::Verifier => write!(f, "verifier"),
            AgentRole::Worker => write!(f, "worker"),
            AgentRole::Integrator => write!(f, "integrator"),
            AgentRole::Auditor => write!(f, "auditor"),
        }
    }
}

// ---------------------------------------------------------------------------
// Approval mode
// ---------------------------------------------------------------------------

/// How a stage transitions to the next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalMode {
    /// Keep advancing until blocked by policy, ambiguity, or failure.
    FullAuto,
    /// Require user approval between stages.
    Gated,
}

// ---------------------------------------------------------------------------
// Stage participants
// ---------------------------------------------------------------------------

/// Who participates in a given stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageParticipants {
    /// Planner agent IDs (at least one required per stage).
    pub planners: Vec<String>,
    /// Verifier agent IDs (required in gated stages).
    pub verifiers: Vec<String>,
    /// Worker agent IDs (optional, for parallel execution).
    pub workers: Vec<String>,
}

// ---------------------------------------------------------------------------
// Session stage
// ---------------------------------------------------------------------------

/// A single stage in the session pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStage {
    /// Stage name (e.g., "ideate", "plan", "execute", "verify", "harden").
    pub name: String,
    /// Who participates.
    pub participants: StageParticipants,
    /// How this stage advances.
    pub approval_mode: ApprovalMode,
    /// Consensus strategy for verifiers (e.g., "majority", "unanimous").
    pub consensus: String,
}

// ---------------------------------------------------------------------------
// Ownership lane
// ---------------------------------------------------------------------------

/// Maps a worker to a set of file globs for parallel execution isolation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnershipLane {
    /// Lane name (e.g., "frontend", "backend", "api").
    pub name: String,
    /// File glob patterns this lane owns.
    pub file_globs: Vec<String>,
}

// ---------------------------------------------------------------------------
// Pipeline validation
// ---------------------------------------------------------------------------

/// Errors when validating a session pipeline.
#[derive(Debug, Clone, thiserror::Error)]
pub enum PipelineValidationError {
    #[error("stage '{0}' has no planner")]
    NoPlannerInStage(String),
    #[error("gated stage '{0}' has no verifier")]
    NoVerifierInGatedStage(String),
    #[error("duplicate agent ID '{0}' in the same stage")]
    DuplicateAgent(String),
    #[error("pipeline has no stages")]
    Empty,
}

// ---------------------------------------------------------------------------
// Session pipeline
// ---------------------------------------------------------------------------

/// A session pipeline: ordered stages with participant slots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPipeline {
    /// Ordered list of stages.
    pub stages: Vec<SessionStage>,
}

impl SessionPipeline {
    /// Create a new pipeline with the given stages.
    pub fn new(stages: Vec<SessionStage>) -> Self {
        Self { stages }
    }

    /// Validate the pipeline structure.
    pub fn validate(&self) -> Result<(), PipelineValidationError> {
        if self.stages.is_empty() {
            return Err(PipelineValidationError::Empty);
        }

        for stage in &self.stages {
            // Every stage needs at least one planner
            if stage.participants.planners.is_empty() {
                return Err(PipelineValidationError::NoPlannerInStage(
                    stage.name.clone(),
                ));
            }

            // Gated stages require at least one verifier
            if stage.approval_mode == ApprovalMode::Gated && stage.participants.verifiers.is_empty()
            {
                return Err(PipelineValidationError::NoVerifierInGatedStage(
                    stage.name.clone(),
                ));
            }

            // No duplicate agent IDs within a stage
            let mut seen = HashSet::new();
            for id in stage
                .participants
                .planners
                .iter()
                .chain(stage.participants.verifiers.iter())
                .chain(stage.participants.workers.iter())
            {
                if !seen.insert(id.as_str()) {
                    return Err(PipelineValidationError::DuplicateAgent(id.clone()));
                }
            }
        }

        Ok(())
    }

    /// Get recipients for planner output in a given stage.
    /// - If stage has workers → planner dispatches to workers
    /// - If no workers → planner output goes to verifiers
    pub fn stage_recipients_for_planner(&self, stage: &SessionStage) -> Vec<String> {
        if !stage.participants.workers.is_empty() {
            stage.participants.workers.clone()
        } else {
            stage.participants.verifiers.clone()
        }
    }

    /// Get recipients for worker output in a given stage.
    /// Worker output goes to verifiers for review.
    pub fn stage_recipients_for_worker(&self, stage: &SessionStage) -> Vec<String> {
        stage.participants.verifiers.clone()
    }

    /// Default preset: planner + verifier (backward compatible with v0.5).
    pub fn default_dual() -> Self {
        Self::new(vec![
            SessionStage {
                name: "ideate".to_string(),
                participants: StageParticipants {
                    planners: vec!["claude".to_string()],
                    verifiers: vec![],
                    workers: vec![],
                },
                approval_mode: ApprovalMode::FullAuto,
                consensus: "none".to_string(),
            },
            SessionStage {
                name: "plan".to_string(),
                participants: StageParticipants {
                    planners: vec!["claude".to_string()],
                    verifiers: vec!["codex".to_string()],
                    workers: vec![],
                },
                approval_mode: ApprovalMode::Gated,
                consensus: "majority".to_string(),
            },
            SessionStage {
                name: "execute".to_string(),
                participants: StageParticipants {
                    planners: vec!["claude".to_string()],
                    verifiers: vec!["codex".to_string()],
                    workers: vec![],
                },
                approval_mode: ApprovalMode::Gated,
                consensus: "majority".to_string(),
            },
            SessionStage {
                name: "harden".to_string(),
                participants: StageParticipants {
                    planners: vec!["claude".to_string()],
                    verifiers: vec!["codex".to_string()],
                    workers: vec![],
                },
                approval_mode: ApprovalMode::Gated,
                consensus: "majority".to_string(),
            },
        ])
    }
}
