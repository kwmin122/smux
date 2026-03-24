//! Tests for pipeline-aware IPC message serialization.

use smux_core::ipc::{ClientMessage, DaemonMessage, IpcStageDefinition};

fn make_stage(
    name: &str,
    planners: Vec<&str>,
    verifiers: Vec<&str>,
    workers: Vec<&str>,
) -> IpcStageDefinition {
    IpcStageDefinition {
        name: name.to_string(),
        approval_mode: "gated".to_string(),
        consensus: "majority".to_string(),
        planners: planners.into_iter().map(|s| s.to_string()).collect(),
        verifiers: verifiers.into_iter().map(|s| s.to_string()).collect(),
        workers: workers.into_iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn start_session_with_pipeline_serializes() {
    let msg = ClientMessage::StartSessionWithPipeline {
        task: "build a blog tool".to_string(),
        agents: vec![
            ("claude".to_string(), "planner".to_string()),
            ("codex".to_string(), "verifier".to_string()),
            ("gemini".to_string(), "verifier".to_string()),
        ],
        stages: vec![
            make_stage("ideate", vec!["claude"], vec![], vec![]),
            make_stage("plan", vec!["claude"], vec!["codex", "gemini"], vec![]),
            make_stage(
                "execute",
                vec!["claude"],
                vec!["codex"],
                vec!["frontend", "backend"],
            ),
        ],
        max_rounds: 5,
    };

    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: ClientMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(msg, deserialized);
}

#[test]
fn stage_carries_per_stage_participants() {
    let msg = ClientMessage::StartSessionWithPipeline {
        task: "test".to_string(),
        agents: vec![
            ("claude".to_string(), "planner".to_string()),
            ("codex".to_string(), "verifier".to_string()),
        ],
        stages: vec![make_stage(
            "execute",
            vec!["claude"],
            vec!["codex"],
            vec!["fe-worker"],
        )],
        max_rounds: 3,
    };

    if let ClientMessage::StartSessionWithPipeline { stages, .. } = &msg {
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0].planners, vec!["claude"]);
        assert_eq!(stages[0].verifiers, vec!["codex"]);
        assert_eq!(stages[0].workers, vec!["fe-worker"]);
        assert_eq!(stages[0].approval_mode, "gated");
        assert_eq!(stages[0].consensus, "majority");
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn stage_transition_event_serializes() {
    let msg = DaemonMessage::StageTransition {
        from: "ideate".to_string(),
        to: "plan".to_string(),
        approval: "auto".to_string(),
    };

    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: DaemonMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(msg, deserialized);
}

#[test]
fn old_start_session_still_works() {
    let msg = ClientMessage::StartSession {
        planner: "claude".to_string(),
        verifier: "codex".to_string(),
        task: "fix bug".to_string(),
        max_rounds: 5,
        verifiers: vec![],
        consensus: "majority".to_string(),
    };

    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: ClientMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(msg, deserialized);
}
