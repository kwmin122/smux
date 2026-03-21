//! CLI smoke tests for argument parsing and wiring.
//!
//! These tests verify that the CLI accepts the expected arguments and rejects
//! invalid invocations.  They do NOT test actual Claude/Codex invocation.

use assert_cmd::Command;

#[test]
fn start_accepts_omitted_planner() {
    // With config support, --planner is optional (defaults from config).
    // The command should be parsed successfully (it may fail at runtime
    // because the daemon isn't available, but argument parsing succeeds).
    let output = Command::cargo_bin("smux")
        .unwrap()
        .args(["start", "--verifier", "codex", "--task", "test"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("required"),
        "planner should be optional, got: {stderr}"
    );
}

#[test]
fn start_accepts_omitted_verifier() {
    // With config support, --verifier is optional (defaults from config).
    let output = Command::cargo_bin("smux")
        .unwrap()
        .args(["start", "--planner", "claude", "--task", "test"])
        .output()
        .unwrap();
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
        .assert()
        .failure();
}

#[test]
fn list_runs_successfully() {
    // `list` should succeed regardless of daemon state — it reads session files.
    Command::cargo_bin("smux")
        .unwrap()
        .arg("list")
        .assert()
        .success();
}

#[test]
fn daemon_status_runs() {
    // `daemon status` should always succeed (shows running or not running).
    Command::cargo_bin("smux")
        .unwrap()
        .args(["daemon", "status"])
        .assert()
        .success();
}

#[test]
fn rewind_requires_session_id_and_round() {
    // Missing both args.
    Command::cargo_bin("smux")
        .unwrap()
        .arg("rewind")
        .assert()
        .failure();
}

#[test]
fn attach_requires_session_id() {
    Command::cargo_bin("smux")
        .unwrap()
        .arg("attach")
        .assert()
        .failure();
}

#[test]
fn detach_subcommand_is_accepted() {
    // `smux detach` should be recognized as a valid subcommand.
    // It will fail at runtime because no daemon is running, but the
    // argument parsing should succeed (exit due to connection error, not
    // unknown subcommand). We verify it doesn't fail with "unrecognized
    // subcommand" by checking stderr doesn't contain "unrecognized".
    let output = Command::cargo_bin("smux")
        .unwrap()
        .arg("detach")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unrecognized subcommand"),
        "detach should be a recognized subcommand, got: {stderr}"
    );
}
