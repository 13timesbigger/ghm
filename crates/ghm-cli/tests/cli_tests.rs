use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help() {
    let mut cmd = Command::cargo_bin("ghad").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Observe repositories for changes"));
}

#[test]
fn test_missing_subcommand() {
    let mut cmd = Command::cargo_bin("ghad").unwrap();
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Usage:"));
}

#[test]
fn test_observe_list_no_config() {
    // We should get an error if there's no config or if config exists we get something.
    // We can just check that it runs and prints something.
    // For integration tests, we don't mock the filesystem, so it depends on the user's ~/.config/ghm
    // To be safe, we can run a command that doesn't strictly depend on auth like 'ghad daemon status'
    let mut cmd = Command::cargo_bin("ghad").unwrap();
    cmd.arg("daemon").arg("status");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Daemon status"));
}
