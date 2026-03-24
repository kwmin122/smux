//! Tests for the multi-agent session pipeline types.

use smux_core::pipeline::{
    AgentRole, ApprovalMode, OwnershipLane, PipelineValidationError, SessionPipeline, SessionStage,
    StageParticipants,
};

#[test]
fn minimal_pipeline_is_valid() {
    let pipeline = SessionPipeline::new(vec![SessionStage {
        name: "execute".to_string(),
        participants: StageParticipants {
            planners: vec!["claude".to_string()],
            verifiers: vec!["codex".to_string()],
            workers: vec![],
        },
        approval_mode: ApprovalMode::Gated,
        consensus: "majority".to_string(),
    }]);
    assert!(pipeline.validate().is_ok());
}

#[test]
fn pipeline_requires_at_least_one_planner() {
    let pipeline = SessionPipeline::new(vec![SessionStage {
        name: "execute".to_string(),
        participants: StageParticipants {
            planners: vec![],
            verifiers: vec!["codex".to_string()],
            workers: vec![],
        },
        approval_mode: ApprovalMode::Gated,
        consensus: "majority".to_string(),
    }]);
    assert!(matches!(
        pipeline.validate(),
        Err(PipelineValidationError::NoPlannerInStage(_))
    ));
}

#[test]
fn gated_stage_requires_verifier() {
    let pipeline = SessionPipeline::new(vec![SessionStage {
        name: "execute".to_string(),
        participants: StageParticipants {
            planners: vec!["claude".to_string()],
            verifiers: vec![],
            workers: vec![],
        },
        approval_mode: ApprovalMode::Gated,
        consensus: "majority".to_string(),
    }]);
    assert!(matches!(
        pipeline.validate(),
        Err(PipelineValidationError::NoVerifierInGatedStage(_))
    ));
}

#[test]
fn auto_stage_without_verifier_is_ok() {
    let pipeline = SessionPipeline::new(vec![SessionStage {
        name: "ideate".to_string(),
        participants: StageParticipants {
            planners: vec!["claude".to_string()],
            verifiers: vec![],
            workers: vec![],
        },
        approval_mode: ApprovalMode::FullAuto,
        consensus: "none".to_string(),
    }]);
    assert!(pipeline.validate().is_ok());
}

#[test]
fn duplicate_agent_ids_rejected() {
    let pipeline = SessionPipeline::new(vec![SessionStage {
        name: "execute".to_string(),
        participants: StageParticipants {
            planners: vec!["claude".to_string()],
            verifiers: vec!["claude".to_string()],
            workers: vec![],
        },
        approval_mode: ApprovalMode::Gated,
        consensus: "majority".to_string(),
    }]);
    assert!(matches!(
        pipeline.validate(),
        Err(PipelineValidationError::DuplicateAgent(_))
    ));
}

#[test]
fn multi_verifier_pipeline() {
    let pipeline = SessionPipeline::new(vec![SessionStage {
        name: "verify".to_string(),
        participants: StageParticipants {
            planners: vec!["claude".to_string()],
            verifiers: vec!["codex".to_string(), "gemini".to_string()],
            workers: vec![],
        },
        approval_mode: ApprovalMode::Gated,
        consensus: "majority".to_string(),
    }]);
    assert!(pipeline.validate().is_ok());
    assert_eq!(pipeline.stages[0].participants.verifiers.len(), 2);
}

#[test]
fn worker_lanes_pipeline() {
    let pipeline = SessionPipeline::new(vec![SessionStage {
        name: "execute".to_string(),
        participants: StageParticipants {
            planners: vec!["claude".to_string()],
            verifiers: vec!["codex".to_string()],
            workers: vec!["frontend-worker".to_string(), "backend-worker".to_string()],
        },
        approval_mode: ApprovalMode::Gated,
        consensus: "majority".to_string(),
    }]);
    assert!(pipeline.validate().is_ok());
    assert_eq!(pipeline.stages[0].participants.workers.len(), 2);
}

#[test]
fn agent_roles_display() {
    assert_eq!(format!("{}", AgentRole::Planner), "planner");
    assert_eq!(format!("{}", AgentRole::Verifier), "verifier");
    assert_eq!(format!("{}", AgentRole::Worker), "worker");
    assert_eq!(format!("{}", AgentRole::Integrator), "integrator");
    assert_eq!(format!("{}", AgentRole::Auditor), "auditor");
}

#[test]
fn ownership_lane_file_glob_match() {
    let lane = OwnershipLane {
        name: "frontend".to_string(),
        file_globs: vec!["src/components/**".to_string(), "src/hooks/**".to_string()],
    };
    assert_eq!(lane.name, "frontend");
    assert_eq!(lane.file_globs.len(), 2);
}

#[test]
fn default_pipeline_preset() {
    let pipeline = SessionPipeline::default_dual();
    assert!(pipeline.validate().is_ok());
    assert!(!pipeline.stages.is_empty());
}
