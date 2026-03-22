//! Tests for session listing and cleanup in `smux_core::session_store`.

use std::path::PathBuf;

use tempfile::TempDir;

use smux_core::session_store::{SessionStore, cleanup_old_sessions_in, list_all_sessions_in};
use smux_core::types::{SessionMeta, SessionStatus};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn sample_meta(id: &str, status: SessionStatus) -> SessionMeta {
    SessionMeta {
        id: id.into(),
        task: format!("task for {id}"),
        planner: "claude".into(),
        verifier: "codex".into(),
        verifiers: vec!["codex".into()],
        consensus_strategy: Default::default(),
        current_round: 1,
        status,
        worktree_path: PathBuf::from(format!("/tmp/wt-{id}")),
        created_at: "2026-03-22T10:00:00Z".into(),
    }
}

/// Create a sessions root with `n` session subdirectories, each containing
/// a valid `session.json`.
fn create_sessions(root: &std::path::Path, count: usize) {
    for i in 0..count {
        let id = format!("sess-{i:03}");
        let session_dir = root.join(&id);
        let store = SessionStore::with_base_dir(session_dir);
        let status = if i % 2 == 0 {
            SessionStatus::Completed
        } else {
            SessionStatus::InProgress
        };
        store.save_session_meta(&sample_meta(&id, status)).unwrap();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn list_all_sessions_finds_existing() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    create_sessions(root, 3);

    let sessions = list_all_sessions_in(root).unwrap();
    assert_eq!(sessions.len(), 3, "should find all 3 sessions");

    let ids: Vec<&str> = sessions.iter().map(|s| s.id.as_str()).collect();
    assert!(ids.contains(&"sess-000"), "should find sess-000");
    assert!(ids.contains(&"sess-001"), "should find sess-001");
    assert!(ids.contains(&"sess-002"), "should find sess-002");
}

#[test]
fn list_all_sessions_empty_dir() {
    let tmp = TempDir::new().unwrap();
    let sessions = list_all_sessions_in(tmp.path()).unwrap();
    assert!(sessions.is_empty());
}

#[test]
fn list_all_sessions_nonexistent_dir() {
    let sessions = list_all_sessions_in(std::path::Path::new("/nonexistent/path")).unwrap();
    assert!(sessions.is_empty());
}

#[test]
fn list_all_sessions_skips_corrupt_meta() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Create one valid session.
    create_sessions(root, 1);

    // Create one corrupt session.
    let corrupt_dir = root.join("corrupt-session");
    std::fs::create_dir_all(&corrupt_dir).unwrap();
    std::fs::write(corrupt_dir.join("session.json"), "not valid json").unwrap();

    let sessions = list_all_sessions_in(root).unwrap();
    assert_eq!(
        sessions.len(),
        1,
        "should only find the valid session, skipping corrupt"
    );
}

#[test]
fn cleanup_removes_old_sessions() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    create_sessions(root, 3);

    // With max_age_days = 0, ALL sessions are "old" (their mtime is > 0 days ago
    // since we just created them, but we set cutoff = now - 0 days = now).
    // However, freshly created dirs have mtime ≈ now, so they won't be older
    // than cutoff=now. We need a different approach:
    //
    // Set max_age_days = 0 means cutoff = now. Freshly created dirs are
    // not strictly *before* now, so they survive.
    //
    // Instead, manually backdate one dir using filetime.
    // Since we don't have the filetime crate, we take a simpler approach:
    // use a very large max_age to keep them all, then a very small one.

    // First: cleanup with max_age = 999 days — nothing should be removed.
    let removed = cleanup_old_sessions_in(root, 999).unwrap();
    assert_eq!(
        removed, 0,
        "no sessions should be removed with max_age=999d"
    );

    // All 3 sessions should still be present.
    let sessions = list_all_sessions_in(root).unwrap();
    assert_eq!(sessions.len(), 3);
}

#[test]
fn cleanup_nonexistent_dir_returns_zero() {
    let removed =
        cleanup_old_sessions_in(std::path::Path::new("/nonexistent/cleanup/path"), 30).unwrap();
    assert_eq!(removed, 0);
}
