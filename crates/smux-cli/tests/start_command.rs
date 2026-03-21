//! CLI smoke tests for argument parsing and wiring.
//!
//! These tests verify that the CLI accepts the expected arguments and rejects
//! invalid invocations.  They do NOT test actual Claude/Codex invocation.

use assert_cmd::Command;

#[test]
fn start_requires_planner_arg() {
    Command::cargo_bin("smux")
        .unwrap()
        .args(["start", "--verifier", "codex", "--task", "test"])
        .assert()
        .failure();
}

#[test]
fn start_requires_verifier_arg() {
    Command::cargo_bin("smux")
        .unwrap()
        .args(["start", "--planner", "claude", "--task", "test"])
        .assert()
        .failure();
}

#[test]
fn start_requires_task_arg() {
    Command::cargo_bin("smux")
        .unwrap()
        .args(["start", "--planner", "claude", "--verifier", "codex"])
        .assert()
        .failure();
}

#[test]
fn start_rejects_unknown_provider() {
    // The CLI should create the adapter and fail with "unknown provider".
    // This tests the wiring from CLI → create_adapter → error path.
    let assert = Command::cargo_bin("smux")
        .unwrap()
        .args([
            "start",
            "--planner",
            "unknown-provider",
            "--verifier",
            "codex",
            "--task",
            "test",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("unknown provider"),
        "expected 'unknown provider' in stderr, got: {stderr}"
    );
}

#[test]
fn list_succeeds() {
    Command::cargo_bin("smux")
        .unwrap()
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("no active sessions"));
}

#[test]
fn rewind_succeeds() {
    Command::cargo_bin("smux")
        .unwrap()
        .args(["rewind", "test-session-id", "1"])
        .assert()
        .success()
        .stdout(predicates::str::contains("rewind requires daemon"));
}
