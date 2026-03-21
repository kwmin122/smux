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
fn list_runs_without_daemon() {
    // When daemon is not running, `list` should still succeed (graceful fallback).
    Command::cargo_bin("smux")
        .unwrap()
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("no active sessions"));
}

#[test]
fn daemon_status_runs() {
    Command::cargo_bin("smux")
        .unwrap()
        .args(["daemon", "status"])
        .assert()
        .success()
        .stdout(predicates::str::contains("daemon is not running"));
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
