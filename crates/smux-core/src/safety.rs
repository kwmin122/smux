//! Safety layers 2 & 3 for smux.
//!
//! **Layer 2** — Provider permission flags: translate [`SafetyConfig`] into
//! CLI arguments for each supported agent (Claude Code, Codex).
//!
//! **Layer 3** — Post-hoc git-diff audit: after each round's commit, inspect
//! the diff to detect unusually large deletions or changes and emit alerts.

use std::path::Path;
use std::process::Command;

use crate::SmuxError;
use crate::config::SafetyConfig;

// ---------------------------------------------------------------------------
// Audit types
// ---------------------------------------------------------------------------

/// Result of a post-hoc safety audit on one round's changes.
#[derive(Debug, Clone)]
pub struct AuditResult {
    /// Number of files deleted in this round.
    pub files_deleted: usize,
    /// Number of files added in this round.
    pub files_added: usize,
    /// Number of files modified in this round.
    pub files_modified: usize,
    /// Total lines added across all files.
    pub lines_added: usize,
    /// Total lines removed across all files.
    pub lines_removed: usize,
    /// Safety alerts triggered (if any).
    pub alerts: Vec<SafetyAlert>,
}

/// A single safety alert emitted by the audit.
#[derive(Debug, Clone)]
pub struct SafetyAlert {
    pub severity: AlertSeverity,
    pub message: String,
}

/// Severity level for a safety alert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertSeverity {
    Warning,
    Critical,
}

// ---------------------------------------------------------------------------
// Layer 3 — post-hoc audit
// ---------------------------------------------------------------------------

/// Run a post-hoc safety audit on the worktree changes between `base_sha` and
/// HEAD.
///
/// Uses `git diff --stat` and `git diff --numstat` to assess change scope,
/// then checks thresholds from [`SafetyConfig`].
pub fn audit_round_changes(
    worktree_path: &Path,
    base_sha: &str,
    config: &SafetyConfig,
) -> Result<AuditResult, SmuxError> {
    // Get --numstat for precise line counts.
    let numstat_output = git_diff_numstat(worktree_path, base_sha)?;
    // Get --diff-filter to count added/deleted/modified files.
    let (files_added, files_deleted, files_modified) =
        git_diff_file_counts(worktree_path, base_sha)?;

    let (lines_added, lines_removed) = parse_numstat(&numstat_output);

    let mut alerts = Vec::new();

    // Check file deletion threshold.
    if files_deleted > config.max_files_deleted_per_round {
        alerts.push(SafetyAlert {
            severity: AlertSeverity::Critical,
            message: format!(
                "round deleted {files_deleted} files (threshold: {})",
                config.max_files_deleted_per_round
            ),
        });
    }

    // Check total lines changed threshold.
    let total_lines = lines_added + lines_removed;
    if total_lines > config.max_lines_changed_per_round {
        alerts.push(SafetyAlert {
            severity: AlertSeverity::Warning,
            message: format!(
                "round changed {total_lines} lines (threshold: {})",
                config.max_lines_changed_per_round
            ),
        });
    }

    Ok(AuditResult {
        files_deleted,
        files_added,
        files_modified,
        lines_added,
        lines_removed,
        alerts,
    })
}

/// Run `git diff --numstat <base>..HEAD` and return raw output.
fn git_diff_numstat(worktree_path: &Path, base_sha: &str) -> Result<String, SmuxError> {
    let range = format!("{base_sha}..HEAD");
    let output = Command::new("git")
        .args(["diff", "--numstat", &range])
        .current_dir(worktree_path)
        .output()
        .map_err(|e| SmuxError::Git(format!("failed to run git diff --numstat: {e}")))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(SmuxError::Git(format!(
            "git diff --numstat failed: {stderr}"
        )))
    }
}

/// Count files by diff filter: Added, Deleted, Modified.
fn git_diff_file_counts(
    worktree_path: &Path,
    base_sha: &str,
) -> Result<(usize, usize, usize), SmuxError> {
    let range = format!("{base_sha}..HEAD");

    let count_filter = |filter: &str| -> Result<usize, SmuxError> {
        let output = Command::new("git")
            .args(["diff", "--diff-filter", filter, "--name-only", &range])
            .current_dir(worktree_path)
            .output()
            .map_err(|e| {
                SmuxError::Git(format!(
                    "failed to run git diff --diff-filter {filter}: {e}"
                ))
            })?;

        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            Ok(text.lines().filter(|l| !l.is_empty()).count())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(SmuxError::Git(format!(
                "git diff --diff-filter {filter} failed: {stderr}"
            )))
        }
    };

    let added = count_filter("A")?;
    let deleted = count_filter("D")?;
    let modified = count_filter("M")?;

    Ok((added, deleted, modified))
}

/// Parse `git diff --numstat` output into (lines_added, lines_removed).
///
/// Each line is: `<added>\t<removed>\t<file>`. Binary files show `-` which we
/// skip.
fn parse_numstat(output: &str) -> (usize, usize) {
    let mut added = 0usize;
    let mut removed = 0usize;

    for line in output.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        // Binary files produce "-\t-\t<file>" — skip those (parse fails).
        if parts.len() >= 2
            && let (Ok(a), Ok(r)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>())
        {
            added += a;
            removed += r;
        }
    }

    (added, removed)
}

// ---------------------------------------------------------------------------
// Layer 2 — provider permission args
// ---------------------------------------------------------------------------

/// Build Claude Code CLI permission arguments from [`SafetyConfig`].
///
/// Maps config fields to:
/// - `--permission-mode <mode>` (when not using `--dangerously-skip-permissions`)
/// - `--allowedTools <tool1> <tool2> ...` (when tools are specified)
pub fn claude_permission_args(config: &SafetyConfig) -> Vec<String> {
    let mut args = Vec::new();

    // Permission mode controls how Claude asks for tool approval.
    // "auto" = default, "manual" = ask for each, "deny" = reject all tool use.
    // If the mode is not the dangerous skip-all, we pass --permission-mode.
    if !config.claude_permission_mode.is_empty() {
        args.push("--permission-mode".to_string());
        args.push(config.claude_permission_mode.clone());
    }

    // Allowed tools whitelist.
    if !config.claude_allowed_tools.is_empty() {
        args.push("--allowedTools".to_string());
        for tool in &config.claude_allowed_tools {
            args.push(tool.clone());
        }
    }

    args
}

/// Build Codex CLI permission arguments from [`SafetyConfig`].
///
/// Maps config fields to:
/// - `-s <sandbox_mode>` (e.g. "workspace-write", "danger-full-access", "read-only")
/// - `--full-auto` for non-interactive execution
///
/// Note: Codex CLI v0.116+ removed the `-a` flag. Use `--full-auto` instead.
pub fn codex_permission_args(config: &SafetyConfig) -> Vec<String> {
    let mut args = Vec::new();

    // Codex v0.116+ uses --full-auto instead of -a flag
    if !config.codex_approval_policy.is_empty() {
        args.push("--full-auto".to_string());
    }

    if !config.codex_sandbox_mode.is_empty() {
        args.push("-s".to_string());
        args.push(config.codex_sandbox_mode.clone());
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_numstat_basic() {
        let input = "10\t5\tsrc/main.rs\n3\t1\tsrc/lib.rs\n";
        let (added, removed) = parse_numstat(input);
        assert_eq!(added, 13);
        assert_eq!(removed, 6);
    }

    #[test]
    fn parse_numstat_empty() {
        let (added, removed) = parse_numstat("");
        assert_eq!(added, 0);
        assert_eq!(removed, 0);
    }

    #[test]
    fn parse_numstat_binary_skipped() {
        let input = "-\t-\timage.png\n10\t5\tsrc/main.rs\n";
        let (added, removed) = parse_numstat(input);
        assert_eq!(added, 10);
        assert_eq!(removed, 5);
    }
}
