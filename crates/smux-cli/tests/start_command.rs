//! CLI smoke tests for argument parsing and wiring.
//!
//! These tests verify that the CLI accepts the expected arguments and rejects
//! invalid invocations. They do NOT test actual daemon connection.
//! Tests that run `smux start` use a short timeout to avoid hanging when
//! the daemon is not running.

use assert_cmd::Command;
use std::time::Duration;

/// Helper: run a CLI command with a timeout to prevent hanging.
/// Returns the output. Panics if the timeout is reached.
fn run_with_timeout(args: &[&str]) -> std::process::Output {
    Command::cargo_bin("smux")
        .unwrap()
        .args(args)
        .timeout(Duration::from_secs(5))
        .output()
        .expect("command timed out or failed to execute")
}

#[test]
fn start_accepts_omitted_planner() {
    // --planner is optional (defaults from config).
    // The command may fail because no daemon is running, but it should NOT
    // fail with "required argument" error.
    let output = run_with_timeout(&["start", "--verifier", "codex", "--task", "test"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("required"),
        "planner should be optional, got: {stderr}"
    );
}

#[test]
fn start_accepts_omitted_verifier() {
    let output = run_with_timeout(&["start", "--planner", "claude", "--task", "test"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("required"),
        "verifier should be optional, got: {stderr}"
    );
}

#[test]
fn start_requires_task_arg() {
    Command::cargo_bin("smux")
        .unwrap()
        .args(["start", "--planner", "claude", "--verifier", "codex"])
        .timeout(Duration::from_secs(5))
        .assert()
        .failure();
}

#[test]
fn list_runs_successfully() {
    Command::cargo_bin("smux")
        .unwrap()
        .arg("list")
        .timeout(Duration::from_secs(5))
        .assert()
        .success();
}

#[test]
fn daemon_status_runs() {
    Command::cargo_bin("smux")
        .unwrap()
        .args(["daemon", "status"])
        .timeout(Duration::from_secs(5))
        .assert()
        .success();
}

#[test]
fn rewind_requires_session_id_and_round() {
    Command::cargo_bin("smux")
        .unwrap()
        .arg("rewind")
        .timeout(Duration::from_secs(5))
        .assert()
        .failure();
}

#[test]
fn attach_requires_session_id() {
    Command::cargo_bin("smux")
        .unwrap()
        .arg("attach")
        .timeout(Duration::from_secs(5))
        .assert()
        .failure();
}

#[test]
fn detach_subcommand_is_accepted() {
    let output = run_with_timeout(&["detach"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unrecognized subcommand"),
        "detach should be a recognized subcommand, got: {stderr}"
    );
}
