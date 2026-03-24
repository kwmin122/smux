//! Tests for pipeline-based relay routing logic.
//!
//! Validates that the orchestrator can route messages based on
//! session pipeline stage definitions instead of hardcoded planner/verifier.

use smux_core::pipeline::{
    ApprovalMode, OwnershipLane, SessionPipeline, SessionStage, StageParticipants,
};

#[test]
fn planner_plus_two_verifier_routing() {
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

    // Verify routing: planner output should go to BOTH verifiers
    let stage = &pipeline.stages[0];
    assert_eq!(stage.participants.verifiers.len(), 2);
    assert!(stage.participants.verifiers.contains(&"codex".to_string()));
    assert!(stage.participants.verifiers.contains(&"gemini".to_string()));
}

#[test]
fn planner_to_worker_dispatch() {
    let pipeline = SessionPipeline::new(vec![SessionStage {
        name: "execute".to_string(),
        participants: StageParticipants {
            planners: vec!["claude".to_string()],
            verifiers: vec!["codex".to_string()],
            workers: vec!["frontend".to_string(), "backend".to_string()],
        },
        approval_mode: ApprovalMode::Gated,
        consensus: "majority".to_string(),
    }]);

    assert!(pipeline.validate().is_ok());
    let stage = &pipeline.stages[0];
    assert_eq!(stage.participants.workers.len(), 2);
}

#[test]
fn multi_stage_advancement() {
    let pipeline = SessionPipeline::default_dual();
    assert!(pipeline.validate().is_ok());

    // Should have 4 stages: ideate, plan, execute, harden
    assert_eq!(pipeline.stages.len(), 4);
    assert_eq!(pipeline.stages[0].name, "ideate");
    assert_eq!(pipeline.stages[1].name, "plan");
    assert_eq!(pipeline.stages[2].name, "execute");
    assert_eq!(pipeline.stages[3].name, "harden");

    // Ideate is full-auto (no verifier needed)
    assert_eq!(pipeline.stages[0].approval_mode, ApprovalMode::FullAuto);
    // Plan/Execute/Harden are gated
    assert_eq!(pipeline.stages[1].approval_mode, ApprovalMode::Gated);
}

#[test]
fn stage_recipients_for_planner_output() {
    let pipeline = SessionPipeline::new(vec![SessionStage {
        name: "plan".to_string(),
        participants: StageParticipants {
            planners: vec!["claude".to_string()],
            verifiers: vec!["codex".to_string(), "gemini".to_string()],
            workers: vec![],
        },
        approval_mode: ApprovalMode::Gated,
        consensus: "unanimous".to_string(),
    }]);

    // When planner produces output in "plan" stage,
    // recipients should be all verifiers
    let recipients = pipeline.stage_recipients_for_planner(&pipeline.stages[0]);
    assert_eq!(recipients.len(), 2);
}

#[test]
fn stage_recipients_for_worker_output() {
    let pipeline = SessionPipeline::new(vec![SessionStage {
        name: "execute".to_string(),
        participants: StageParticipants {
            planners: vec!["claude".to_string()],
            verifiers: vec!["codex".to_string()],
            workers: vec!["frontend".to_string()],
        },
        approval_mode: ApprovalMode::Gated,
        consensus: "majority".to_string(),
    }]);

    // When a worker completes, recipients should be verifiers
    let recipients = pipeline.stage_recipients_for_worker(&pipeline.stages[0]);
    assert_eq!(recipients.len(), 1);
    assert_eq!(recipients[0], "codex");
}

#[test]
fn backward_compat_single_verifier() {
    // The default_dual preset should work exactly like the old
    // planner + single verifier model
    let pipeline = SessionPipeline::default_dual();
    let execute_stage = &pipeline.stages[2]; // "execute"
    assert_eq!(execute_stage.participants.planners.len(), 1);
    assert_eq!(execute_stage.participants.verifiers.len(), 1);
    assert_eq!(execute_stage.participants.planners[0], "claude");
    assert_eq!(execute_stage.participants.verifiers[0], "codex");
}

#[test]
fn ownership_lane_collision_detection() {
    let lanes = vec![
        OwnershipLane {
            name: "frontend".to_string(),
            file_globs: vec!["src/components/**".to_string()],
        },
        OwnershipLane {
            name: "backend".to_string(),
            file_globs: vec!["src/components/**".to_string()], // overlaps!
        },
    ];

    // Detect that two lanes claim the same glob
    let collisions = detect_lane_collisions(&lanes);
    assert!(!collisions.is_empty());
}

/// Simple collision detection helper for tests.
fn detect_lane_collisions(lanes: &[OwnershipLane]) -> Vec<(String, String, String)> {
    let mut collisions = Vec::new();
    for i in 0..lanes.len() {
        for j in (i + 1)..lanes.len() {
            for glob_a in &lanes[i].file_globs {
                for glob_b in &lanes[j].file_globs {
                    if glob_a == glob_b {
                        collisions.push((
                            lanes[i].name.clone(),
                            lanes[j].name.clone(),
                            glob_a.clone(),
                        ));
                    }
                }
            }
        }
    }
    collisions
}
