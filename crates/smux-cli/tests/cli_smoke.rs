use assert_cmd::Command;

#[test]
fn shows_help() {
    Command::cargo_bin("smux")
        .unwrap()
        .arg("--help")
        .assert()
        .success();
}
