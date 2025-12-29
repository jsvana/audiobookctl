use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_lookup_all_help() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["lookup-all", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Look up metadata for all audiobooks",
        ))
        .stdout(predicate::str::contains("--auto-accept"))
        .stdout(predicate::str::contains("--no-dry-run"))
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("--no-backup-i-void-my-warranty"));
}

#[test]
fn test_lookup_all_empty_directory() {
    let temp = tempfile::tempdir().unwrap();

    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["lookup-all", temp.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("No .m4b files found"));
}

#[test]
fn test_lookup_all_nonexistent_directory() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["lookup-all", "/nonexistent/directory/path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No .m4b files found"));
}

#[test]
fn test_backups_list_help() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["backups", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("List all backup files"));
}

#[test]
fn test_backups_clean_help() {
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["backups", "clean", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Clean backup files"));
}

#[test]
fn test_backups_list_empty_directory() {
    let temp = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args(["backups", "list", temp.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("No backup files found"));
}

#[test]
fn test_backups_clean_empty_directory() {
    let temp = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("audiobookctl").unwrap();
    cmd.args([
        "backups",
        "clean",
        temp.path().to_str().unwrap(),
        "--all",
        "--yes",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("No backup files to clean"));
}
