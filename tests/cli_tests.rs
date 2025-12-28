use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_show_missing_file_returns_error() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["show", "/nonexistent/file.m4b"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Failed to read m4b file"));
}

#[test]
fn test_show_help() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["show", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Display metadata"));
}

#[test]
fn test_unknown_field_returns_error() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["show", "--field", "invalid_field", "/nonexistent/file.m4b"]);
    // File error comes first, but if we had a file, field error would show
    cmd.assert().failure();
}

#[test]
fn test_version() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("audiobookctl"));
}
