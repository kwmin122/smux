use assert_cmd::Command;

#[test]
fn shows_help() {
    Command::cargo_bin("smux")
        .unwrap()
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn daemon_subcommand_help() {
    Command::cargo_bin("smux")
        .unwrap()
        .args(["daemon", "--help"])
        .assert()
        .success();
}
