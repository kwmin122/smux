//! Integration tests for `smux_core::session_store` — round snapshot persistence.

use std::path::PathBuf;

use tempfile::TempDir;

use smux_core::session_store::SessionStore;
use smux_core::types::{RoundSnapshot, SessionMeta, SessionStatus, VerifyResult};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a `SessionStore` backed by a temp directory (not ~/.smux).
fn store_in_tmp() -> (SessionStore, TempDir) {
    let tmp = TempDir::new().expect("create temp dir");
    let store = SessionStore::with_base_dir(tmp.path().to_path_buf());
    (store, tmp)
}

fn sample_snapshot(round: u32) -> RoundSnapshot {
    RoundSnapshot {
        round,
        commit_sha: format!("abc{round:04}"),
        planner_context_path: PathBuf::from(format!("/tmp/planner-{round}.json")),
        verifier_context_path: PathBuf::from(format!("/tmp/verifier-{round}.json")),
        verdict: VerifyResult::Approved {
            reason: format!("looks good (round {round})"),
            confidence: 0.95,
        },
        files_changed: vec![format!("file-{round}.rs")],
        timestamp: format!("2026-03-21T12:00:{round:02}Z"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn save_and_load_round() {
    let (store, _tmp) = store_in_tmp();

    let snap = sample_snapshot(1);
    store.save_round(&snap).expect("save_round");

    let loaded = store.load_round(1).expect("load_round");

    assert_eq!(loaded.round, 1);
    assert_eq!(loaded.commit_sha, "abc0001");
    assert_eq!(loaded.files_changed, vec!["file-1.rs"]);
    assert_eq!(loaded.timestamp, "2026-03-21T12:00:01Z");

    // Check verdict round-trips correctly.
    match &loaded.verdict {
        VerifyResult::Approved { reason, confidence } => {
            assert_eq!(reason, "looks good (round 1)");
            assert!((confidence - 0.95).abs() < f64::EPSILON);
        }
        other => panic!("expected Approved, got {other:?}"),
    }
}

#[test]
fn list_rounds_returns_sorted() {
    let (store, _tmp) = store_in_tmp();

    // Save rounds out of order.
    store.save_round(&sample_snapshot(3)).unwrap();
    store.save_round(&sample_snapshot(1)).unwrap();
    store.save_round(&sample_snapshot(2)).unwrap();

    let rounds = store.list_rounds().expect("list_rounds");
    assert_eq!(rounds, vec![1, 2, 3]);
}

#[test]
fn save_and_load_session_meta() {
    let (store, _tmp) = store_in_tmp();

    let meta = SessionMeta {
        id: "sess-42".into(),
        task: "fix the bug".into(),
        planner: "claude".into(),
        verifier: "codex".into(),
        current_round: 3,
        status: SessionStatus::InProgress,
        worktree_path: PathBuf::from("/tmp/wt-42"),
        created_at: "2026-03-21T10:00:00Z".into(),
    };

    store.save_session_meta(&meta).expect("save_session_meta");
    let loaded = store.load_session_meta().expect("load_session_meta");

    assert_eq!(loaded.id, "sess-42");
    assert_eq!(loaded.task, "fix the bug");
    assert_eq!(loaded.planner, "claude");
    assert_eq!(loaded.verifier, "codex");
    assert_eq!(loaded.current_round, 3);
    assert_eq!(loaded.status, SessionStatus::InProgress);
    assert_eq!(loaded.worktree_path, PathBuf::from("/tmp/wt-42"));
    assert_eq!(loaded.created_at, "2026-03-21T10:00:00Z");
}

#[test]
fn load_nonexistent_round_fails() {
    let (store, _tmp) = store_in_tmp();

    let result = store.load_round(99);
    assert!(result.is_err(), "loading a non-existent round should fail");
}
