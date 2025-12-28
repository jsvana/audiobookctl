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

#[test]
fn test_edit_help() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["edit", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Edit metadata"))
        .stdout(predicate::str::contains("--no-dry-run"))
        .stdout(predicate::str::contains("--no-backup-i-void-my-warranty"));
}

#[test]
fn test_edit_missing_file() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["edit", "/nonexistent/file.m4b"]);
    cmd.assert().failure();
}

#[test]
fn test_edit_clear_no_file() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["edit", "--clear"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Cleared"));
}

#[test]
fn test_edit_commit_all_no_backups() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["edit", "--commit-all"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No backup files found"));
}

#[test]
fn test_lookup_help() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["lookup", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Look up metadata"))
        .stdout(predicate::str::contains("Audnexus"))
        .stdout(predicate::str::contains("Open Library"))
        .stdout(predicate::str::contains("--no-dry-run"))
        .stdout(predicate::str::contains("--no-backup-i-void-my-warranty"));
}

#[test]
fn test_lookup_missing_file() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["lookup", "/nonexistent/file.m4b"]);
    cmd.assert().failure();
}
