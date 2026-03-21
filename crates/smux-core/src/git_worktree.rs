//! Git worktree management for smux sessions.
//!
//! Each session gets its own worktree and branch (`smux/<session-id>`).
//! Each round commits all changes; rewind = `git reset --hard` + `git clean -fd`.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::SmuxError;

/// Run a git command and return its stdout on success, or a descriptive error.
fn git(args: &[&str], cwd: &Path) -> Result<String, SmuxError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| SmuxError::Git(format!("failed to run git {}: {e}", args.join(" "))))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(SmuxError::Git(format!(
            "git {} failed (exit {}): {stderr}",
            args.join(" "),
            output.status.code().unwrap_or(-1),
        )))
    }
}

/// Create a new worktree for a session. Returns the worktree path.
///
/// Runs: `git worktree add <path> -b smux/<session-id>`
pub fn create_worktree(repo_root: &Path, session_id: &str) -> Result<PathBuf, SmuxError> {
    let branch = format!("smux/{session_id}");
    let worktree_path = repo_root.join(format!(".smux-worktrees/{session_id}"));

    // Ensure parent directory exists
    if let Some(parent) = worktree_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            SmuxError::Git(format!(
                "failed to create worktree parent dir {}: {e}",
                parent.display()
            ))
        })?;
    }

    let wt_str = worktree_path
        .to_str()
        .ok_or_else(|| SmuxError::Git("worktree path contains non-UTF-8 characters".into()))?;

    git(&["worktree", "add", wt_str, "-b", &branch], repo_root)?;

    tracing::info!(
        session_id,
        path = %worktree_path.display(),
        branch = %branch,
        "worktree created"
    );

    Ok(worktree_path)
}

/// Remove a worktree and its branch.
///
/// Runs: `git worktree remove <path> --force` then `git branch -D smux/<session-id>`
pub fn remove_worktree(
    repo_root: &Path,
    worktree_path: &Path,
    session_id: &str,
) -> Result<(), SmuxError> {
    let branch = format!("smux/{session_id}");

    let wt_str = worktree_path
        .to_str()
        .ok_or_else(|| SmuxError::Git("worktree path contains non-UTF-8 characters".into()))?;

    git(&["worktree", "remove", wt_str, "--force"], repo_root)?;

    git(&["branch", "-D", &branch], repo_root)?;

    tracing::info!(
        session_id,
        path = %worktree_path.display(),
        "worktree removed"
    );

    Ok(())
}

/// Commit all changes in the worktree for a given round.
///
/// Runs: `git -C <worktree> add -A && git -C <worktree> commit -m "smux: round N -- {verdict}"`
///
/// Returns the commit SHA.
pub fn commit_round(
    worktree_path: &Path,
    round: u32,
    verdict_summary: &str,
) -> Result<String, SmuxError> {
    git(&["add", "-A"], worktree_path)?;

    let message = format!("smux: round {round} — {verdict_summary}");
    git(&["commit", "-m", &message, "--allow-empty"], worktree_path)?;

    let sha = head_sha(worktree_path)?;
    tracing::info!(round, sha = %sha, verdict_summary, "round committed");
    Ok(sha)
}

/// Rewind the worktree to a specific commit.
///
/// Runs: `git -C <worktree> reset --hard <sha> && git -C <worktree> clean -fd`
pub fn rewind_to(worktree_path: &Path, commit_sha: &str) -> Result<(), SmuxError> {
    tracing::info!(
        sha = %commit_sha,
        path = %worktree_path.display(),
        "rewinding worktree"
    );
    git(&["reset", "--hard", commit_sha], worktree_path)?;
    git(&["clean", "-fd"], worktree_path)?;
    Ok(())
}

/// Get the current HEAD commit SHA of the worktree.
pub fn head_sha(worktree_path: &Path) -> Result<String, SmuxError> {
    git(&["rev-parse", "HEAD"], worktree_path)
}
