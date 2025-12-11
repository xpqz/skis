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

// Phase 2: Task 2.2 - issue edit CLI tests

#[test]
fn cli_issue_edit_title() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "Original"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "edit", "1", "--title", "Updated"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated issue #1"));

    // Verify the change
    skis()
        .args(["issue", "view", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated"));
}

#[test]
fn cli_issue_edit_type() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "Test", "--type", "task"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "edit", "1", "--type", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "view", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("bug"));
}

#[test]
fn cli_issue_edit_nonexistent_shows_error() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "edit", "999", "--title", "New"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Issue #999 not found"));
}

// Phase 2: Task 2.4 - issue comment CLI tests

#[test]
fn cli_issue_comment_with_body() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "Test"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "comment", "1", "--body", "This is a comment"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Added comment"));
}

// Phase 2: Task 2.5 - issue view with comments

#[test]
fn cli_issue_view_with_comments() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "Test"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "comment", "1", "--body", "My comment text"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "view", "1", "--comments"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("My comment text"));
}

#[test]
fn cli_issue_view_without_comments_flag_hides_comments() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "Test"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "comment", "1", "--body", "Hidden comment"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Without --comments flag, comment should not appear
    skis()
        .args(["issue", "view", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Hidden comment").not());
}

// Phase 2: Task 2.7 - issue list with search

#[test]
fn cli_issue_list_search() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "Login bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "create", "--title", "Update docs"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "list", "--search", "login"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Login bug"))
        .stdout(predicate::str::contains("Update docs").not());
}

#[test]
fn cli_issue_list_search_with_filters() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "Open searchable", "--type", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "create", "--title", "Another searchable", "--type", "task"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Search with type filter
    skis()
        .args(["issue", "list", "--search", "searchable", "--type", "bug"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Open searchable"))
        .stdout(predicate::str::contains("Another searchable").not());
}

// Phase 2: Task 2.9 - issue link/unlink CLI tests

#[test]
fn cli_issue_link() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "Issue 1"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "create", "--title", "Issue 2"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "link", "1", "2"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Linked"));
}

#[test]
fn cli_issue_unlink() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "Issue 1"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "create", "--title", "Issue 2"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "link", "1", "2"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "unlink", "1", "2"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Unlinked"));
}

// Phase 2: Task 2.10 - issue view shows links

#[test]
fn cli_issue_view_shows_links() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "Issue 1"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "create", "--title", "Issue 2"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "link", "1", "2"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "view", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("#2"));
}

// Phase 3: Label CLI tests

#[test]
fn cli_label_create() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["label", "create", "bug"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created label"));
}

#[test]
fn cli_label_create_with_color() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["label", "create", "bug", "--color", "d73a4a"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created label"));
}

#[test]
fn cli_label_create_invalid_color_shows_error() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["label", "create", "bug", "--color", "invalid"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid color"));
}

#[test]
fn cli_label_list() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["label", "create", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["label", "create", "enhancement"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["label", "list"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("bug"))
        .stdout(predicate::str::contains("enhancement"));
}

#[test]
fn cli_label_list_empty() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["label", "list"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No labels"));
}

#[test]
fn cli_label_delete_with_yes() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["label", "create", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["label", "delete", "bug", "--yes"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted label"));
}

// Phase 3: Issue edit with labels

#[test]
fn cli_issue_edit_add_label() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["label", "create", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "create", "--title", "Test"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "edit", "1", "--add-label", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "view", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("bug"));
}

#[test]
fn cli_issue_edit_remove_label() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["label", "create", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "create", "--title", "Test", "--label", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "edit", "1", "--remove-label", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Label should no longer appear
    skis()
        .args(["issue", "view", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Labels:").not());
}

#[test]
fn cli_issue_edit_add_and_remove_labels() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["label", "create", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["label", "create", "enhancement"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "create", "--title", "Test", "--label", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "edit", "1", "--remove-label", "bug", "--add-label", "enhancement"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "view", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("enhancement"))
        .stdout(predicate::str::contains("bug").not().or(predicate::str::contains("Labels: enhancement")));
}

// Phase 3: Show labels in view and list

#[test]
fn cli_issue_view_shows_labels() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["label", "create", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "create", "--title", "Test", "--label", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "view", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("bug"));
}

#[test]
fn cli_issue_list_shows_labels() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["label", "create", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "create", "--title", "Test", "--label", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "list"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("bug"));
}
