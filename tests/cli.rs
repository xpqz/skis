//! CLI integration tests for SKIS
//!
//! These tests verify command-line behavior using assert_cmd.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn skis() -> Command {
    Command::cargo_bin("skis").unwrap()
}

// Task 1.11: CLI skeleton tests

#[test]
fn cli_shows_help() {
    skis()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("SKIS"))
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("issue"))
        .stdout(predicate::str::contains("label"));
}

#[test]
fn cli_issue_shows_subcommands() {
    skis()
        .arg("issue")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("view"))
        .stdout(predicate::str::contains("close"))
        .stdout(predicate::str::contains("reopen"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("restore"));
}

#[test]
fn cli_label_shows_subcommands() {
    skis()
        .arg("label")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("delete"));
}

// Task 1.12: init command tests

#[test]
fn cli_init_creates_skis_directory() {
    let dir = TempDir::new().unwrap();

    skis()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized"));

    assert!(dir.path().join(".skis").exists());
    assert!(dir.path().join(".skis/issues.db").exists());
}

#[test]
fn cli_init_fails_if_already_initialized() {
    let dir = TempDir::new().unwrap();

    // First init succeeds
    skis()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Second init fails
    skis()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Already initialized"));
}

#[test]
fn cli_commands_fail_without_init() {
    let dir = TempDir::new().unwrap();

    skis()
        .args(["issue", "list"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not a skis repository"));
}

// Task 1.13: issue create tests

#[test]
fn cli_issue_create_with_title() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "Test issue"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created issue #1"));
}

#[test]
fn cli_issue_create_with_all_options() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args([
            "issue", "create",
            "--title", "Bug report",
            "--body", "Something is broken",
            "--type", "bug",
        ])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created issue #1"));
}

// Task 1.14: issue list tests

#[test]
fn cli_issue_list_default() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    // Create an issue first
    skis()
        .args(["issue", "create", "--title", "Test"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "list"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Test"));
}

#[test]
fn cli_issue_list_empty_shows_no_issues() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "list"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No issues found"));
}

#[test]
fn cli_issue_ls_alias() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "ls"])
        .current_dir(dir.path())
        .assert()
        .success();
}

// Task 1.15: issue view tests

#[test]
fn cli_issue_view_existing() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "View me", "--body", "Body text"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "view", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("#1"))
        .stdout(predicate::str::contains("View me"))
        .stdout(predicate::str::contains("Body text"));
}

#[test]
fn cli_issue_view_nonexistent_shows_error() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "view", "999"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Issue #999 not found"));
}

// Task 1.16: issue close tests

#[test]
fn cli_issue_close_default_reason() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "To close"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "close", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Closed issue #1"));
}

#[test]
fn cli_issue_close_with_reason() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "Won't do"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "close", "1", "--reason", "not_planned"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Closed issue #1"));
}

#[test]
fn cli_issue_close_already_closed_shows_error() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "Test"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "close", "1"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "close", "1"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("already closed"));
}

// Task 1.17: issue reopen tests

#[test]
fn cli_issue_reopen() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "Test"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "close", "1"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "reopen", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Reopened issue #1"));
}

#[test]
fn cli_issue_reopen_already_open_shows_error() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "Test"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "reopen", "1"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("already open"));
}

// Task 1.18: issue delete tests

#[test]
fn cli_issue_delete_with_yes() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "To delete"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "delete", "1", "--yes"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted issue #1"));
}

#[test]
fn cli_issue_delete_removes_from_list() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "Delete me"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "delete", "1", "--yes"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Issue should not appear in default list
    skis()
        .args(["issue", "list"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Delete me").not());
}

// Task 1.19: issue restore tests

#[test]
fn cli_issue_restore() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "Restore me"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "delete", "1", "--yes"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "restore", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Restored issue #1"));
}

#[test]
fn cli_issue_restore_appears_in_list() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "Restore me"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "delete", "1", "--yes"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "restore", "1"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Issue should appear in list again
    skis()
        .args(["issue", "list"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Restore me"));
}

// Unimplemented commands should fail

#[test]
fn cli_unimplemented_commands_return_error() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    // label commands are not implemented
    skis()
        .args(["label", "list"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not yet implemented"));

    skis()
        .args(["label", "create", "bug"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not yet implemented"));
}

// Sort and order flags

#[test]
fn cli_issue_list_with_sort_and_order() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "First"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "create", "--title", "Second"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Sort by id ascending should show First before Second
    skis()
        .args(["issue", "list", "--sort", "id", "--order", "asc"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("First"));
}

// Repository discovery test

#[test]
fn cli_discovers_skis_in_parent_directory() {
    let dir = TempDir::new().unwrap();
    let subdir = dir.path().join("sub");
    std::fs::create_dir(&subdir).unwrap();

    // Init in parent
    skis().arg("init").current_dir(dir.path()).assert().success();

    // Create issue from subdir
    skis()
        .args(["issue", "create", "--title", "From subdir"])
        .current_dir(&subdir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Created issue #1"));
}
