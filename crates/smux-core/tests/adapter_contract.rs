//! Contract tests for [`AgentAdapter`] using [`FakeAdapter`].

use std::path::PathBuf;

use futures::StreamExt;
use smux_core::adapter::AgentAdapter;
use smux_core::adapter::fake::FakeAdapter;
use smux_core::types::{AgentEvent, SessionConfig};

fn session_config() -> SessionConfig {
    SessionConfig {
        system_prompt: "You are a test assistant.".into(),
        working_directory: PathBuf::from("/tmp"),
        prior_transcript: vec![],
    }
}

// ── Capabilities ──────────────────────────────────────────────────────

#[tokio::test]
async fn capabilities_returns_expected_values() {
    let adapter = FakeAdapter::new(vec![]);
    let caps = adapter.capabilities();
    assert!(
        !caps.persistent_session,
        "FakeAdapter has no persistent session"
    );
    assert!(caps.streaming, "FakeAdapter supports streaming");
    assert!(!caps.native_snapshot, "FakeAdapter has no native snapshot");
}

// ── Start session ─────────────────────────────────────────────────────

#[tokio::test]
async fn start_session_succeeds() {
    let mut adapter = FakeAdapter::new(vec!["hello".into()]);
    let result = adapter.start_session(session_config()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn start_session_twice_fails() {
    let mut adapter = FakeAdapter::new(vec![]);
    adapter.start_session(session_config()).await.unwrap();
    let result = adapter.start_session(session_config()).await;
    assert!(result.is_err(), "starting a session twice should fail");
}

// ── Send turn / stream events ─────────────────────────────────────────

#[tokio::test]
async fn send_turn_without_session_fails() {
    let mut adapter = FakeAdapter::new(vec!["hello".into()]);
    let result = adapter.send_turn("hi").await;
    assert!(
        result.is_err(),
        "send_turn before start_session should fail"
    );
}

#[tokio::test]
async fn send_one_turn_and_receive_turn_complete() {
    let mut adapter = FakeAdapter::new(vec!["response-0".into()]);
    adapter.start_session(session_config()).await.unwrap();

    let handle = adapter.send_turn("prompt").await.unwrap();
    assert_eq!(handle.turn_index, 0);

    let events: Vec<AgentEvent> = adapter.stream_events().unwrap().collect().await;
    assert_eq!(events.len(), 2, "should get Chunk + TurnComplete");

    match &events[0] {
        AgentEvent::Chunk(text) => assert_eq!(text, "response-0"),
        other => panic!("expected Chunk, got {other:?}"),
    }
    match &events[1] {
        AgentEvent::TurnComplete(text) => assert_eq!(text, "response-0"),
        other => panic!("expected TurnComplete, got {other:?}"),
    }
}

#[tokio::test]
async fn send_multiple_turns_matches_canned_data() {
    let responses: Vec<String> = (0..3).map(|i| format!("answer-{i}")).collect();
    let mut adapter = FakeAdapter::new(responses.clone());
    adapter.start_session(session_config()).await.unwrap();

    for (i, expected) in responses.iter().enumerate() {
        let handle = adapter.send_turn(&format!("q{i}")).await.unwrap();
        assert_eq!(handle.turn_index, i as u64);

        let events: Vec<AgentEvent> = adapter.stream_events().unwrap().collect().await;
        match &events[1] {
            AgentEvent::TurnComplete(text) => assert_eq!(text, expected),
            other => panic!("turn {i}: expected TurnComplete, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn send_turn_past_canned_responses_fails() {
    let mut adapter = FakeAdapter::new(vec!["only-one".into()]);
    adapter.start_session(session_config()).await.unwrap();

    adapter.send_turn("first").await.unwrap();
    let _ = adapter.stream_events().unwrap().collect::<Vec<_>>().await;

    let result = adapter.send_turn("second").await;
    assert!(
        result.is_err(),
        "should fail when canned responses exhausted"
    );
}

// ── stream_events lifecycle errors ────────────────────────────────────

#[tokio::test]
async fn stream_events_without_send_turn_fails() {
    let mut adapter = FakeAdapter::new(vec!["a".into()]);
    adapter.start_session(session_config()).await.unwrap();

    let result = adapter.stream_events();
    assert!(
        result.is_err(),
        "stream_events before send_turn should fail"
    );
}

#[tokio::test]
async fn stream_events_consumed_twice_fails() {
    let mut adapter = FakeAdapter::new(vec!["a".into()]);
    adapter.start_session(session_config()).await.unwrap();
    adapter.send_turn("q").await.unwrap();

    // First consume succeeds
    let _ = adapter.stream_events().unwrap().collect::<Vec<_>>().await;

    // Second consume should fail (stream already taken)
    let result = adapter.stream_events();
    assert!(
        result.is_err(),
        "stream_events called twice should fail on second call"
    );
}

// ── Snapshot / Restore ────────────────────────────────────────────────

#[tokio::test]
async fn snapshot_returns_serializable_data() {
    let mut adapter = FakeAdapter::new(vec!["a".into(), "b".into()]);
    adapter.start_session(session_config()).await.unwrap();

    adapter.send_turn("q").await.unwrap();
    let _ = adapter.stream_events().unwrap().collect::<Vec<_>>().await;

    let snapshot = adapter.snapshot_state().await.unwrap();
    assert_eq!(snapshot.adapter_type, "fake");
    assert_eq!(snapshot.state.len(), 8, "state should be 8 bytes (u64 le)");

    // Index should be 1 (one turn consumed).
    let index = u64::from_le_bytes(snapshot.state.try_into().unwrap());
    assert_eq!(index, 1);
}

#[tokio::test]
async fn restore_state_resets_to_prior_position() {
    let mut adapter = FakeAdapter::new(vec!["a".into(), "b".into(), "c".into()]);
    adapter.start_session(session_config()).await.unwrap();

    // Consume two turns.
    adapter.send_turn("q1").await.unwrap();
    let _ = adapter.stream_events().unwrap().collect::<Vec<_>>().await;
    adapter.send_turn("q2").await.unwrap();
    let _ = adapter.stream_events().unwrap().collect::<Vec<_>>().await;

    // Snapshot at index 2.
    let snap_at_2 = adapter.snapshot_state().await.unwrap();

    // Restore to index 1 (manually crafted).
    let snap_at_1 = smux_core::types::SessionSnapshot {
        adapter_type: "fake".into(),
        state: 1u64.to_le_bytes().to_vec(),
    };
    adapter.restore_state(snap_at_1).await.unwrap();

    // Next turn should be "b" (index 1).
    let handle = adapter.send_turn("q again").await.unwrap();
    assert_eq!(handle.turn_index, 1);

    let events: Vec<AgentEvent> = adapter.stream_events().unwrap().collect().await;
    match &events[1] {
        AgentEvent::TurnComplete(text) => assert_eq!(text, "b"),
        other => panic!("expected TurnComplete('b'), got {other:?}"),
    }

    // Restore to snap_at_2 — next turn should be "c".
    adapter.restore_state(snap_at_2).await.unwrap();
    let handle = adapter.send_turn("q3").await.unwrap();
    assert_eq!(handle.turn_index, 2);

    let events: Vec<AgentEvent> = adapter.stream_events().unwrap().collect().await;
    match &events[1] {
        AgentEvent::TurnComplete(text) => assert_eq!(text, "c"),
        other => panic!("expected TurnComplete('c'), got {other:?}"),
    }
}

#[tokio::test]
async fn restore_wrong_adapter_type_fails() {
    let mut adapter = FakeAdapter::new(vec![]);
    adapter.start_session(session_config()).await.unwrap();

    let bad_snapshot = smux_core::types::SessionSnapshot {
        adapter_type: "not-fake".into(),
        state: vec![0; 8],
    };
    let result = adapter.restore_state(bad_snapshot).await;
    assert!(result.is_err(), "restoring a foreign snapshot should fail");
}

// ── Terminate ─────────────────────────────────────────────────────────

#[tokio::test]
async fn terminate_then_send_turn_fails() {
    let mut adapter = FakeAdapter::new(vec!["a".into()]);
    adapter.start_session(session_config()).await.unwrap();
    adapter.terminate().await.unwrap();

    let result = adapter.send_turn("after terminate").await;
    assert!(result.is_err(), "send_turn after terminate should fail");
}
