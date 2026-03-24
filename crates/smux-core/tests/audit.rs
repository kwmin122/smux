//! Tests for audit logging primitives.

use smux_core::audit::{AuditEvent, AuditSink};

#[test]
fn record_stage_transition() {
    let mut sink = AuditSink::new();
    sink.record(AuditEvent::StageTransition {
        from: "ideate".into(),
        to: "plan".into(),
    });
    assert_eq!(sink.records().len(), 1);
    assert!(matches!(
        &sink.records()[0].event,
        AuditEvent::StageTransition { .. }
    ));
}

#[test]
fn record_approval() {
    let mut sink = AuditSink::new();
    sink.record(AuditEvent::Approval {
        stage: "plan".into(),
        approved_by: "user".into(),
    });
    assert_eq!(sink.records().len(), 1);
}

#[test]
fn record_verifier_finding() {
    let mut sink = AuditSink::new();
    sink.record(AuditEvent::VerifierFinding {
        verifier: "codex".into(),
        verdict: "rejected".into(),
        reason: "missing tests".into(),
    });
    let rec = &sink.records()[0];
    if let AuditEvent::VerifierFinding { reason, .. } = &rec.event {
        assert_eq!(reason, "missing tests");
    } else {
        panic!("wrong event type");
    }
}

#[test]
fn record_command_blocked() {
    let mut sink = AuditSink::new();
    sink.record(AuditEvent::CommandBlocked {
        command: "rm -rf /".into(),
        reason: "deny-list".into(),
    });
    assert_eq!(sink.records().len(), 1);
}

#[test]
fn records_have_timestamps() {
    let mut sink = AuditSink::new();
    sink.record(AuditEvent::StageTransition {
        from: "a".into(),
        to: "b".into(),
    });
    assert!(sink.records()[0].timestamp > 0);
}

#[test]
fn sink_serializes_to_json() {
    let mut sink = AuditSink::new();
    sink.record(AuditEvent::Approval {
        stage: "execute".into(),
        approved_by: "admin".into(),
    });
    let json = serde_json::to_string(sink.records()).unwrap();
    assert!(json.contains("execute"));
}
