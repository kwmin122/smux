//! Integration tests for `smux_core::git_worktree`.
//!
//! Every test creates a temporary git repository, exercises worktree operations,
//! and cleans up via `TempDir` drop.

use std::fs;
use std::process::Command;

use tempfile::TempDir;

use smux_core::git_worktree;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a fresh `git init` repo inside a temp dir and make one initial commit
/// so that HEAD exists (worktree creation needs at least one commit).
fn init_tmp_repo() -> TempDir {
    let tmp = TempDir::new().expect("create temp dir");
    let repo = tmp.path();

    run_git(&["init"], repo);
    run_git(&["config", "user.email", "test@smux.dev"], repo);
    run_git(&["config", "user.name", "smux-test"], repo);

    // Need at least one commit for worktree to have a valid HEAD.
    fs::write(repo.join("README.md"), "# test repo\n").unwrap();
    run_git(&["add", "-A"], repo);
    run_git(&["commit", "-m", "initial commit"], repo);

    tmp
}

/// Run a git command in `cwd` and panic on failure.
fn run_git(args: &[&str], cwd: &std::path::Path) {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("git command spawned");
    assert!(
        out.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr),
    );
}

/// Read the HEAD SHA of a repo/worktree via `git rev-parse HEAD`.
fn read_head(cwd: &std::path::Path) -> String {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(cwd)
        .output()
        .expect("git rev-parse");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn create_worktree_succeeds() {
    let tmp = init_tmp_repo();
    let repo = tmp.path();
    let sid = "test-create";

    let wt = git_worktree::create_worktree(repo, sid).expect("create_worktree");

    // Worktree directory must exist.
    assert!(wt.exists(), "worktree dir should exist");

    // Branch smux/<sid> must exist.
    let branches = Command::new("git")
        .args(["branch", "--list", &format!("smux/{sid}")])
        .current_dir(repo)
        .output()
        .unwrap();
    let branch_list = String::from_utf8_lossy(&branches.stdout);
    assert!(
        branch_list.contains(&format!("smux/{sid}")),
        "branch smux/{sid} should exist, got: {branch_list}",
    );
}

#[test]
fn commit_round_captures_changes() {
    let tmp = init_tmp_repo();
    let repo = tmp.path();

    let wt = git_worktree::create_worktree(repo, "commit-test").unwrap();

    // Create a file in the worktree.
    fs::write(wt.join("hello.txt"), "hello").unwrap();

    let sha = git_worktree::commit_round(&wt, 1, "APPROVED").expect("commit_round");
    assert!(!sha.is_empty(), "SHA should be non-empty");

    // The returned SHA should match the worktree HEAD.
    let head = read_head(&wt);
    assert_eq!(sha, head);
}

#[test]
fn commit_round_captures_untracked() {
    let tmp = init_tmp_repo();
    let repo = tmp.path();

    let wt = git_worktree::create_worktree(repo, "untracked-test").unwrap();

    // Create an untracked file (not `git add`-ed manually).
    fs::write(wt.join("untracked.txt"), "should be committed").unwrap();

    let sha = git_worktree::commit_round(&wt, 1, "APPROVED").unwrap();
    assert!(!sha.is_empty());

    // Verify the file is in the commit.
    let show = Command::new("git")
        .args(["show", "--name-only", "--pretty=format:", &sha])
        .current_dir(&wt)
        .output()
        .unwrap();
    let files = String::from_utf8_lossy(&show.stdout);
    assert!(
        files.contains("untracked.txt"),
        "untracked.txt should appear in commit, got: {files}",
    );
}

#[test]
fn rewind_restores_tracked_files() {
    let tmp = init_tmp_repo();
    let repo = tmp.path();

    let wt = git_worktree::create_worktree(repo, "rewind-tracked").unwrap();

    // Round 1: write file.
    fs::write(wt.join("data.txt"), "round-1-content").unwrap();
    let sha1 = git_worktree::commit_round(&wt, 1, "R1").unwrap();

    // Round 2: modify file.
    fs::write(wt.join("data.txt"), "round-2-content").unwrap();
    let _sha2 = git_worktree::commit_round(&wt, 2, "R2").unwrap();

    // Verify round 2 content.
    assert_eq!(
        fs::read_to_string(wt.join("data.txt")).unwrap(),
        "round-2-content"
    );

    // Rewind to round 1.
    git_worktree::rewind_to(&wt, &sha1).unwrap();
    assert_eq!(
        fs::read_to_string(wt.join("data.txt")).unwrap(),
        "round-1-content"
    );
}

#[test]
fn rewind_cleans_untracked() {
    let tmp = init_tmp_repo();
    let repo = tmp.path();

    let wt = git_worktree::create_worktree(repo, "rewind-untracked").unwrap();

    // Round 1: one file.
    fs::write(wt.join("keep.txt"), "keep").unwrap();
    let sha1 = git_worktree::commit_round(&wt, 1, "R1").unwrap();

    // Round 2: add a new file.
    fs::write(wt.join("extra.txt"), "extra").unwrap();
    let _sha2 = git_worktree::commit_round(&wt, 2, "R2").unwrap();

    // Add yet another untracked file (not committed).
    fs::write(wt.join("debris.txt"), "debris").unwrap();

    // Rewind to round 1: both extra.txt and debris.txt should be gone.
    git_worktree::rewind_to(&wt, &sha1).unwrap();

    assert!(wt.join("keep.txt").exists(), "keep.txt should still exist");
    assert!(
        !wt.join("extra.txt").exists(),
        "extra.txt should be removed by reset"
    );
    assert!(
        !wt.join("debris.txt").exists(),
        "debris.txt should be removed by clean"
    );
}

#[test]
fn head_sha_returns_current() {
    let tmp = init_tmp_repo();
    let repo = tmp.path();

    let wt = git_worktree::create_worktree(repo, "head-sha-test").unwrap();

    let our_sha = git_worktree::head_sha(&wt).unwrap();
    let git_sha = read_head(&wt);

    assert_eq!(our_sha, git_sha);
}

#[test]
fn remove_worktree_cleans_up() {
    let tmp = init_tmp_repo();
    let repo = tmp.path();
    let sid = "remove-test";

    let wt = git_worktree::create_worktree(repo, sid).unwrap();
    assert!(wt.exists());

    git_worktree::remove_worktree(repo, &wt, sid).unwrap();

    // Directory gone.
    assert!(!wt.exists(), "worktree dir should be removed");

    // Branch gone.
    let branches = Command::new("git")
        .args(["branch", "--list", &format!("smux/{sid}")])
        .current_dir(repo)
        .output()
        .unwrap();
    let branch_list = String::from_utf8_lossy(&branches.stdout).trim().to_string();
    assert!(
        branch_list.is_empty(),
        "branch smux/{sid} should be deleted, got: {branch_list}",
    );
}

#[test]
fn create_worktree_fails_without_git_repo() {
    let tmp = TempDir::new().expect("create temp dir");
    let non_repo = tmp.path();

    // Do NOT run `git init` here.
    let result = git_worktree::create_worktree(non_repo, "no-repo");
    assert!(result.is_err(), "should fail in a non-git directory");
}
