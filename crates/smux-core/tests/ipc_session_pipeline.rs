//! Tests for pipeline-aware IPC message serialization.

use smux_core::ipc::{ClientMessage, DaemonMessage};

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
            "ideate".to_string(),
            "plan".to_string(),
            "execute".to_string(),
        ],
        approval_mode: "gated".to_string(),
        consensus: "majority".to_string(),
        max_rounds: 5,
    };

    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: ClientMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(msg, deserialized);
}

#[test]
fn start_session_with_pipeline_contains_agents() {
    let msg = ClientMessage::StartSessionWithPipeline {
        task: "test".to_string(),
        agents: vec![
            ("claude".to_string(), "planner".to_string()),
            ("codex".to_string(), "verifier".to_string()),
        ],
        stages: vec!["execute".to_string()],
        approval_mode: "gated".to_string(),
        consensus: "majority".to_string(),
        max_rounds: 3,
    };

    if let ClientMessage::StartSessionWithPipeline { agents, .. } = &msg {
        assert_eq!(agents.len(), 2);
        assert_eq!(agents[0].0, "claude");
        assert_eq!(agents[0].1, "planner");
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
    assert!(json.contains("StageTransition") || json.contains("stage_transition"));
    let deserialized: DaemonMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(msg, deserialized);
}

#[test]
fn old_start_session_still_works() {
    // Backward compatibility: old StartSession variant must still parse
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
