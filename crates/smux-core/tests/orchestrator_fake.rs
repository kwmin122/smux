//! Integration tests for the orchestrator ping-pong loop using [`FakeAdapter`].

use smux_core::adapter::fake::FakeAdapter;
use smux_core::orchestrator::{Orchestrator, OrchestratorConfig, OrchestratorOutcome};

fn config(task: &str, max_rounds: u32) -> OrchestratorConfig {
    OrchestratorConfig {
        task: task.to_string(),
        max_rounds,
        max_tokens: 4000,
        health_config: None,
        consensus_strategy: Default::default(),
    }
}

// ── Test 1: Approved on first round ──────────────────────────────────────

#[tokio::test]
async fn planner_approved_on_first_round() {
    // Planner responds once with a plan.
    let planner = FakeAdapter::new(vec!["Here is my implementation plan.".into()]);

    // Verifier responds once with an APPROVED verdict.
    let verifier = FakeAdapter::new(vec![
        r#"{"verdict":"APPROVED","reason":"looks good","confidence":0.9}"#.into(),
    ]);

    let mut orch = Orchestrator::new(
        Box::new(planner),
        Box::new(verifier),
        config("Fix the bug", 5),
    );

    let outcome = orch.run().await;

    match outcome {
        OrchestratorOutcome::Approved { round, reason } => {
            assert_eq!(round, 1);
            assert_eq!(reason, "looks good");
        }
        other => panic!("expected Approved, got {other:?}"),
    }
}

// ── Test 2: Rejection then approval ──────────────────────────────────────

#[tokio::test]
async fn rejection_then_approval() {
    // Planner: round 1 → initial plan, round 2 → revised plan.
    let planner = FakeAdapter::new(vec![
        "Initial plan without tests.".into(),
        "Revised plan with comprehensive tests.".into(),
    ]);

    // Verifier: round 1 → reject, round 2 → approve.
    let verifier = FakeAdapter::new(vec![
        r#"{"verdict":"REJECTED","category":"weak_test","reason":"no tests","confidence":0.7}"#
            .into(),
        r#"{"verdict":"APPROVED","reason":"tests added","confidence":0.95}"#.into(),
    ]);

    let mut orch = Orchestrator::new(
        Box::new(planner),
        Box::new(verifier),
        config("Add feature X", 5),
    );

    let outcome = orch.run().await;

    match outcome {
        OrchestratorOutcome::Approved { round, reason } => {
            assert_eq!(round, 2);
            assert_eq!(reason, "tests added");
        }
        other => panic!("expected Approved at round 2, got {other:?}"),
    }
}

// ── Test 3: Max rounds reached ───────────────────────────────────────────

#[tokio::test]
async fn max_rounds_reached() {
    // Planner: responds for each round.
    let planner = FakeAdapter::new(vec!["Plan attempt 1.".into(), "Plan attempt 2.".into()]);

    // Verifier: always rejects.
    let verifier = FakeAdapter::new(vec![
        r#"{"verdict":"REJECTED","category":"incomplete","reason":"still broken","confidence":0.6}"#
            .into(),
        r#"{"verdict":"REJECTED","category":"incomplete","reason":"still broken","confidence":0.6}"#
            .into(),
    ]);

    let mut orch = Orchestrator::new(
        Box::new(planner),
        Box::new(verifier),
        config("Impossible task", 2),
    );

    let outcome = orch.run().await;

    match outcome {
        OrchestratorOutcome::MaxRoundsReached { rounds_completed } => {
            assert_eq!(rounds_completed, 2);
        }
        other => panic!("expected MaxRoundsReached, got {other:?}"),
    }
}

// ── Test 4: NeedsInfo triggers re-ask ────────────────────────────────────

#[tokio::test]
async fn needs_info_triggers_re_ask() {
    // Planner: responds once.
    let planner = FakeAdapter::new(vec!["Here is my plan.".into()]);

    // Verifier: first response has no verdict (NeedsInfo), second (re-ask) approves.
    let verifier = FakeAdapter::new(vec![
        "I have some questions about the implementation.".into(),
        r#"{"verdict":"APPROVED","reason":"clarified and approved","confidence":0.85}"#.into(),
    ]);

    let mut orch = Orchestrator::new(
        Box::new(planner),
        Box::new(verifier),
        config("Implement feature Y", 5),
    );

    let outcome = orch.run().await;

    match outcome {
        OrchestratorOutcome::Approved { round, reason } => {
            assert_eq!(round, 1, "should approve on round 1 after re-ask");
            assert_eq!(reason, "clarified and approved");
        }
        other => panic!("expected Approved after NeedsInfo re-ask, got {other:?}"),
    }
}

// ── Test 5: Context passing includes prior rounds ────────────────────────

#[tokio::test]
async fn context_passing_includes_prior_rounds() {
    // We use a custom FakeAdapter that records the prompts it receives.
    // Since FakeAdapter doesn't record prompts, we verify indirectly:
    // The verifier's second prompt is built by build_verifier_prompt with
    // prior_rounds containing R1 info. We test that the orchestrator
    // correctly passes context by checking the outcome flows correctly
    // through 2 rounds.
    //
    // For a more direct test, we use a CapturingAdapter that records prompts.

    use smux_core::adapter::{AdapterError, AgentAdapter, AgentEventStream};
    use smux_core::types::{AdapterCapabilities, SessionConfig, SessionSnapshot, TurnHandle};
    use std::sync::{Arc, Mutex};

    /// An adapter that records prompts sent to it, and replays canned responses.
    struct CapturingAdapter {
        inner: FakeAdapter,
        prompts: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait::async_trait]
    impl AgentAdapter for CapturingAdapter {
        fn capabilities(&self) -> AdapterCapabilities {
            self.inner.capabilities()
        }

        async fn start_session(&mut self, config: SessionConfig) -> Result<(), AdapterError> {
            self.inner.start_session(config).await
        }

        async fn send_turn(&mut self, prompt: &str) -> Result<TurnHandle, AdapterError> {
            self.prompts.lock().unwrap().push(prompt.to_string());
            self.inner.send_turn(prompt).await
        }

        fn stream_events(&self) -> Result<AgentEventStream<'_>, AdapterError> {
            self.inner.stream_events()
        }

        async fn snapshot_state(&self) -> Result<SessionSnapshot, AdapterError> {
            self.inner.snapshot_state().await
        }

        async fn restore_state(&mut self, snapshot: SessionSnapshot) -> Result<(), AdapterError> {
            self.inner.restore_state(snapshot).await
        }

        async fn terminate(&mut self) -> Result<(), AdapterError> {
            self.inner.terminate().await
        }
    }

    let planner = FakeAdapter::new(vec![
        "Plan v1: no tests.".into(),
        "Plan v2: with tests.".into(),
    ]);

    let verifier_prompts: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let verifier_prompts_clone = verifier_prompts.clone();

    let verifier = CapturingAdapter {
        inner: FakeAdapter::new(vec![
            r#"{"verdict":"REJECTED","category":"weak_test","reason":"no tests","confidence":0.7}"#
                .into(),
            r#"{"verdict":"APPROVED","reason":"tests added","confidence":0.9}"#.into(),
        ]),
        prompts: verifier_prompts_clone,
    };

    let mut orch = Orchestrator::new(
        Box::new(planner),
        Box::new(verifier),
        config("Build feature Z", 5),
    );

    let outcome = orch.run().await;

    match &outcome {
        OrchestratorOutcome::Approved { round, .. } => {
            assert_eq!(*round, 2);
        }
        other => panic!("expected Approved at round 2, got {other:?}"),
    }

    // Verify the second verifier prompt contains prior round context.
    let prompts = verifier_prompts.lock().unwrap();
    assert_eq!(prompts.len(), 2, "verifier should have received 2 prompts");

    // First prompt should NOT contain prior rounds summary (round 1).
    assert!(
        !prompts[0].contains("Previous Rounds Summary"),
        "round 1 verifier prompt should not contain prior rounds"
    );

    // Second prompt SHOULD contain prior rounds summary with R1 info.
    assert!(
        prompts[1].contains("Previous Rounds Summary"),
        "round 2 verifier prompt should contain prior rounds summary"
    );
    assert!(
        prompts[1].contains("R1: REJECTED (weak_test)"),
        "round 2 verifier prompt should reference R1 rejection: got:\n{}",
        &prompts[1]
    );
    assert!(
        prompts[1].contains("no tests"),
        "round 2 verifier prompt should include R1 reason"
    );
}

// VG-008: Test that OrchestratorEvent events are emitted via the event sink
#[tokio::test]
async fn event_sink_receives_events() {
    use smux_core::orchestrator::OrchestratorEvent;

    let planner = FakeAdapter::new(vec!["Here is my plan.".into()]);
    let verifier = FakeAdapter::new(vec![
        r#"{"verdict":"APPROVED","reason":"looks good","confidence":0.9}"#.into(),
    ]);

    let (tx, mut rx) = tokio::sync::mpsc::channel::<OrchestratorEvent>(64);

    let mut orch = Orchestrator::new(
        Box::new(planner),
        Box::new(verifier),
        config("Test events", 5),
    )
    .with_event_sink(tx);

    let outcome = orch.run().await;
    assert!(matches!(outcome, OrchestratorOutcome::Approved { .. }));

    // Collect all events from the channel.
    let mut events = Vec::new();
    while let Ok(e) = rx.try_recv() {
        events.push(e);
    }

    // Expect: RoundStarted(1), PlannerOutput(1), VerifierOutput(1), RoundComplete(1)
    assert!(
        events.len() >= 4,
        "expected at least 4 events, got {}",
        events.len()
    );
    assert!(
        matches!(&events[0], OrchestratorEvent::RoundStarted { round: 1 }),
        "first event should be RoundStarted(1), got {:?}",
        events[0]
    );
    assert!(
        matches!(
            &events[1],
            OrchestratorEvent::PlannerOutput { round: 1, .. }
        ),
        "second event should be PlannerOutput(1), got {:?}",
        events[1]
    );
    assert!(
        matches!(
            &events[2],
            OrchestratorEvent::VerifierOutput { round: 1, .. }
        ),
        "third event should be VerifierOutput(1), got {:?}",
        events[2]
    );
    assert!(
        matches!(
            &events[3],
            OrchestratorEvent::RoundComplete { round: 1, .. }
        ),
        "fourth event should be RoundComplete(1), got {:?}",
        events[3]
    );
}

// VG-008: Test multi-round event emission
#[tokio::test]
async fn event_sink_multi_round() {
    use smux_core::orchestrator::OrchestratorEvent;

    let planner = FakeAdapter::new(vec!["plan v1".into(), "plan v2".into()]);
    let verifier = FakeAdapter::new(vec![
        r#"{"verdict":"REJECTED","category":"weak_test","reason":"no tests","confidence":0.7}"#
            .into(),
        r#"{"verdict":"APPROVED","reason":"tests added","confidence":0.95}"#.into(),
    ]);

    let (tx, mut rx) = tokio::sync::mpsc::channel::<OrchestratorEvent>(64);

    let mut orch = Orchestrator::new(
        Box::new(planner),
        Box::new(verifier),
        config("Multi-round events", 5),
    )
    .with_event_sink(tx);

    let outcome = orch.run().await;
    assert!(matches!(
        outcome,
        OrchestratorOutcome::Approved { round: 2, .. }
    ));

    let mut events = Vec::new();
    while let Ok(e) = rx.try_recv() {
        events.push(e);
    }

    // 2 rounds x 4 events = 8 events total
    assert!(
        events.len() >= 8,
        "expected at least 8 events for 2 rounds, got {}",
        events.len()
    );
    // Verify round 2 events exist
    let round2_starts: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, OrchestratorEvent::RoundStarted { round: 2 }))
        .collect();
    assert_eq!(
        round2_starts.len(),
        1,
        "should have exactly one RoundStarted(2)"
    );
}

// VG-001: Test error path — adapter failure returns OrchestratorOutcome::Error
#[tokio::test]
async fn adapter_error_returns_outcome_error() {
    // Planner has a response but verifier has NONE → verifier will fail on send_turn
    let planner = FakeAdapter::new(vec!["my plan".into()]);
    let verifier = FakeAdapter::new(vec![]); // no canned responses → error

    let config = OrchestratorConfig {
        task: "test error path".into(),
        max_rounds: 5,
        max_tokens: 4000,
        health_config: None,
        consensus_strategy: Default::default(),
    };

    let mut orch = Orchestrator::new(Box::new(planner), Box::new(verifier), config);
    let outcome = orch.run().await;

    match &outcome {
        OrchestratorOutcome::Error { message } => {
            assert!(
                !message.is_empty(),
                "error message should be non-empty: {message}"
            );
        }
        other => panic!("expected Error, got {other:?}"),
    }
}

// VG-002: Test double-NeedsInfo fallback — verifier returns NeedsInfo twice → synthetic Rejected
#[tokio::test]
async fn double_needs_info_falls_back_to_rejected() {
    // Planner: responds twice (for 2 rounds if needed)
    // Verifier: two responses with no verdict → NeedsInfo twice
    let planner = FakeAdapter::new(vec!["plan v1".into(), "plan v2".into()]);
    let verifier = FakeAdapter::new(vec![
        "I'm not sure about this.".into(), // Round 1: no verdict → NeedsInfo
        "Still not sure.".into(),          // Re-ask: still NeedsInfo → synthetic Rejected
        // Round 2: planner gets feedback, but we need a verifier response for round 2
        r#"{"verdict":"APPROVED","reason":"ok now","confidence":0.9}"#.into(),
    ]);

    let config = OrchestratorConfig {
        task: "test double needs-info".into(),
        max_rounds: 5,
        max_tokens: 4000,
        health_config: None,
        consensus_strategy: Default::default(),
    };

    let mut orch = Orchestrator::new(Box::new(planner), Box::new(verifier), config);
    let outcome = orch.run().await;

    // Should eventually reach APPROVED in round 2 after the double-NeedsInfo in round 1
    // was handled as a synthetic rejection
    match &outcome {
        OrchestratorOutcome::Approved { round, .. } => {
            assert_eq!(
                *round, 2,
                "should approve at round 2 after synthetic rejection in R1"
            );
        }
        other => panic!("expected Approved at round 2, got {other:?}"),
    }
}
