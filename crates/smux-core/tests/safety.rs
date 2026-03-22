//! Integration tests for safety layers 2 & 3.

use std::process::Command;

use smux_core::config::SafetyConfig;
use smux_core::safety::{self, AlertSeverity};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a temporary git repo and return its path.
fn init_temp_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("failed to create tempdir");
    let path = dir.path();

    git(&["init"], path);
    git(&["config", "user.email", "test@smux.dev"], path);
    git(&["config", "user.name", "smux-test"], path);

    // Initial commit so we have a base SHA.
    std::fs::write(path.join("README.md"), "# test\n").unwrap();
    git(&["add", "-A"], path);
    git(&["commit", "-m", "initial"], path);

    dir
}

/// Run a git command in the given directory.
fn git(args: &[&str], cwd: &std::path::Path) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to run git");
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Get the HEAD sha.
fn head_sha(cwd: &std::path::Path) -> String {
    git(&["rev-parse", "HEAD"], cwd)
}

// ---------------------------------------------------------------------------
// Layer 3 — audit tests
// ---------------------------------------------------------------------------

#[test]
fn audit_detects_large_deletion() {
    let dir = init_temp_repo();
    let path = dir.path();
    let base = head_sha(path);

    // Create 8 files, commit them, then delete 6 in the next commit.
    for i in 0..8 {
        std::fs::write(path.join(format!("file_{i}.txt")), format!("content {i}\n")).unwrap();
    }
    git(&["add", "-A"], path);
    git(&["commit", "-m", "add files"], path);
    let after_add = head_sha(path);

    // Delete 6 files.
    for i in 0..6 {
        std::fs::remove_file(path.join(format!("file_{i}.txt"))).unwrap();
    }
    git(&["add", "-A"], path);
    git(&["commit", "-m", "delete files"], path);

    let config = SafetyConfig {
        max_files_deleted_per_round: 5,
        max_lines_changed_per_round: 2000,
        ..Default::default()
    };

    let result = safety::audit_round_changes(path, &after_add, &config).unwrap();
    assert_eq!(result.files_deleted, 6);
    assert!(
        result
            .alerts
            .iter()
            .any(|a| a.severity == AlertSeverity::Critical),
        "expected Critical alert for large deletion, got: {:?}",
        result.alerts
    );

    // Also verify against original base to exercise multi-commit range.
    let result2 = safety::audit_round_changes(path, &base, &config).unwrap();
    assert_eq!(result2.files_added, 2); // 6 deleted out of 8 added = 2 net added
}

#[test]
fn audit_detects_large_change() {
    let dir = init_temp_repo();
    let path = dir.path();
    let base = head_sha(path);

    // Create a file with >2000 lines.
    let big_content: String = (0..2500).map(|i| format!("line {i}\n")).collect();
    std::fs::write(path.join("big.txt"), &big_content).unwrap();
    git(&["add", "-A"], path);
    git(&["commit", "-m", "add big file"], path);

    let config = SafetyConfig {
        max_files_deleted_per_round: 5,
        max_lines_changed_per_round: 2000,
        ..Default::default()
    };

    let result = safety::audit_round_changes(path, &base, &config).unwrap();
    assert!(result.lines_added >= 2500);
    assert!(
        result
            .alerts
            .iter()
            .any(|a| a.severity == AlertSeverity::Warning),
        "expected Warning alert for large change, got: {:?}",
        result.alerts
    );
}

#[test]
fn audit_clean_round_no_alerts() {
    let dir = init_temp_repo();
    let path = dir.path();
    let base = head_sha(path);

    // Small change — well within thresholds.
    std::fs::write(path.join("small.txt"), "hello world\n").unwrap();
    git(&["add", "-A"], path);
    git(&["commit", "-m", "small change"], path);

    let config = SafetyConfig::default();

    let result = safety::audit_round_changes(path, &base, &config).unwrap();
    assert!(
        result.alerts.is_empty(),
        "expected no alerts, got: {:?}",
        result.alerts
    );
    assert_eq!(result.files_added, 1);
    assert_eq!(result.files_deleted, 0);
}

#[test]
fn audit_with_no_changes() {
    let dir = init_temp_repo();
    let path = dir.path();
    let base = head_sha(path);

    // No changes since base — empty diff.
    let config = SafetyConfig::default();

    let result = safety::audit_round_changes(path, &base, &config).unwrap();
    assert_eq!(result.files_added, 0);
    assert_eq!(result.files_deleted, 0);
    assert_eq!(result.files_modified, 0);
    assert_eq!(result.lines_added, 0);
    assert_eq!(result.lines_removed, 0);
    assert!(result.alerts.is_empty());
}

// ---------------------------------------------------------------------------
// Layer 2 — permission flag tests
// ---------------------------------------------------------------------------

#[test]
fn claude_permission_args_from_config() {
    let config = SafetyConfig {
        claude_permission_mode: "manual".to_string(),
        claude_allowed_tools: vec!["Read".to_string(), "Edit".to_string()],
        ..Default::default()
    };

    let args = safety::claude_permission_args(&config);
    assert!(args.contains(&"--permission-mode".to_string()));
    assert!(args.contains(&"manual".to_string()));
    assert!(args.contains(&"--allowedTools".to_string()));
    assert!(args.contains(&"Read".to_string()));
    assert!(args.contains(&"Edit".to_string()));
}

#[test]
fn claude_permission_args_default() {
    let config = SafetyConfig::default();
    let args = safety::claude_permission_args(&config);
    // Default mode is "auto" — should have --permission-mode auto.
    assert!(args.contains(&"--permission-mode".to_string()));
    assert!(args.contains(&"auto".to_string()));
    // No allowed tools specified.
    assert!(!args.contains(&"--allowedTools".to_string()));
}

#[test]
fn codex_permission_args_from_config() {
    let config = SafetyConfig {
        codex_approval_policy: "auto-approve".to_string(),
        codex_sandbox_mode: "full-write".to_string(),
        ..Default::default()
    };

    let args = safety::codex_permission_args(&config);
    assert_eq!(args, vec!["--full-auto", "-s", "full-write"]);
}

#[test]
fn codex_permission_args_default() {
    let config = SafetyConfig::default();
    let args = safety::codex_permission_args(&config);
    assert_eq!(args, vec!["--full-auto", "-s", "workspace-write"]);
}
