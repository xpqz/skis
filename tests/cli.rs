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

#[test]
fn cli_issue_create_with_duplicate_labels() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    // Create label first
    skis()
        .args(["label", "create", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Create issue with same label twice (should not error)
    skis()
        .args([
            "issue", "create",
            "--title", "Duplicate label test",
            "--label", "bug",
            "--label", "bug",
        ])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created issue #1"));

    // Verify issue has label (only once)
    skis()
        .args(["issue", "view", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Labels: bug"));
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
        .stdout(predicate::str::contains("Labels: enhancement"))
        .stdout(predicate::str::contains("bug").not());
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

// Phase 4: Polish

// 4.1: JSON output for issue view with labels and linked issues

#[test]
fn cli_issue_view_json_valid() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "JSON test", "--body", "Test body", "--type", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    let output = skis()
        .args(["issue", "view", "1", "--json"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).expect("valid JSON");

    // Required fields per PLAN.md JSON schema
    assert_eq!(json["id"], 1);
    assert_eq!(json["title"], "JSON test");
    assert_eq!(json["body"], "Test body");
    assert_eq!(json["type"], "bug");
    assert_eq!(json["state"], "open");
    assert!(json["state_reason"].is_null());
    assert!(json["created_at"].is_string(), "created_at should be a timestamp string");
    assert!(json["updated_at"].is_string(), "updated_at should be a timestamp string");
    assert!(json["closed_at"].is_null());
    assert!(json["deleted_at"].is_null());
    assert!(json["labels"].is_array(), "labels should be an array");
    assert!(json["linked_issues"].is_array(), "linked_issues should be an array");
}

#[test]
fn cli_issue_view_json_includes_labels() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["label", "create", "bug", "--color", "ff0000", "--description", "Bug reports"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "create", "--title", "Labeled", "--label", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    let output = skis()
        .args(["issue", "view", "1", "--json"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).expect("valid JSON");
    assert!(json["labels"].is_array(), "labels should be an array");
    let labels = json["labels"].as_array().unwrap();
    assert_eq!(labels.len(), 1);

    // Per PLAN.md, labels should include name and color
    let label = &labels[0];
    assert_eq!(label["name"], "bug");
    assert_eq!(label["color"], "ff0000");
    // description is optional but should be present if provided
    assert_eq!(label["description"], "Bug reports");
}

#[test]
fn cli_issue_view_json_includes_linked_issues() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "Issue A"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "create", "--title", "Issue B"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "link", "1", "2"])
        .current_dir(dir.path())
        .assert()
        .success();

    let output = skis()
        .args(["issue", "view", "1", "--json"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).expect("valid JSON");
    assert!(json["linked_issues"].is_array(), "linked_issues should be an array");
    let linked = json["linked_issues"].as_array().unwrap();
    assert_eq!(linked.len(), 1);

    // Per PLAN.md, linked_issues should be objects with id and title
    let linked_issue = &linked[0];
    assert_eq!(linked_issue["id"], 2);
    assert_eq!(linked_issue["title"], "Issue B");
}

// 4.2: JSON output for issue list

#[test]
fn cli_issue_list_json_valid() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "First", "--type", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "create", "--title", "Second", "--body", "With body"])
        .current_dir(dir.path())
        .assert()
        .success();

    let output = skis()
        .args(["issue", "list", "--json"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).expect("valid JSON");
    assert!(json.is_array(), "should be an array");
    let issues = json.as_array().unwrap();
    assert_eq!(issues.len(), 2);

    // Verify each issue has required fields
    for issue in issues {
        assert!(issue["id"].is_i64(), "id should be an integer");
        assert!(issue["title"].is_string(), "title should be a string");
        assert!(issue["type"].is_string(), "type should be a string");
        assert!(issue["state"].is_string(), "state should be a string");
        assert!(issue["created_at"].is_string(), "created_at should be a timestamp");
        assert!(issue["updated_at"].is_string(), "updated_at should be a timestamp");
    }
}

// 4.3: JSON output for label list

#[test]
fn cli_label_list_json_valid() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["label", "create", "bug", "--color", "ff0000"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["label", "create", "feature", "--description", "New feature"])
        .current_dir(dir.path())
        .assert()
        .success();

    let output = skis()
        .args(["label", "list", "--json"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).expect("valid JSON");
    assert!(json.is_array(), "should be an array");
    let labels = json.as_array().unwrap();
    assert_eq!(labels.len(), 2);

    // Verify label objects have required fields
    for label in labels {
        assert!(label["id"].is_i64(), "id should be an integer");
        assert!(label["name"].is_string(), "name should be a string");
        // color and description can be null
    }

    // Find bug label and verify its color
    let bug_label = labels.iter().find(|l| l["name"] == "bug").expect("bug label");
    assert_eq!(bug_label["color"], "ff0000");

    // Find feature label and verify its description
    let feature_label = labels.iter().find(|l| l["name"] == "feature").expect("feature label");
    assert_eq!(feature_label["description"], "New feature");
}

// 4.4: Close with comment

#[test]
fn cli_issue_close_with_comment() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "To close"])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "close", "1", "--comment", "Fixed in commit abc123"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Closed issue #1"));

    // Verify comment was added
    skis()
        .args(["issue", "view", "1", "--comments"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Fixed in commit abc123"));
}

// 4.5: Body from file

#[test]
fn cli_issue_create_body_from_file() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    // Create a file with body content
    let body_file = dir.path().join("body.txt");
    std::fs::write(&body_file, "This is the body from a file.\nWith multiple lines.").unwrap();

    skis()
        .args(["issue", "create", "--title", "From file", "--body-file", body_file.to_str().unwrap()])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "view", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("This is the body from a file"))
        .stdout(predicate::str::contains("With multiple lines"));
}

#[test]
fn cli_issue_create_body_from_stdin() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "From stdin", "--body-file", "-"])
        .current_dir(dir.path())
        .write_stdin("Body from stdin input")
        .assert()
        .success();

    skis()
        .args(["issue", "view", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Body from stdin input"));
}

#[test]
fn cli_issue_edit_body_from_file() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "To edit", "--body", "Original body"])
        .current_dir(dir.path())
        .assert()
        .success();

    let body_file = dir.path().join("new_body.txt");
    std::fs::write(&body_file, "Updated body from file").unwrap();

    skis()
        .args(["issue", "edit", "1", "--body-file", body_file.to_str().unwrap()])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "view", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated body from file"))
        .stdout(predicate::str::contains("Original body").not());
}

#[test]
fn cli_issue_comment_body_from_file() {
    let dir = TempDir::new().unwrap();
    skis().arg("init").current_dir(dir.path()).assert().success();

    skis()
        .args(["issue", "create", "--title", "To comment"])
        .current_dir(dir.path())
        .assert()
        .success();

    let comment_file = dir.path().join("comment.txt");
    std::fs::write(&comment_file, "Comment from file").unwrap();

    skis()
        .args(["issue", "comment", "1", "--body-file", comment_file.to_str().unwrap()])
        .current_dir(dir.path())
        .assert()
        .success();

    skis()
        .args(["issue", "view", "1", "--comments"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Comment from file"));
}

// 4.10: Full integration test

#[test]
fn full_issue_lifecycle() {
    let dir = TempDir::new().unwrap();

    // init
    skis().arg("init").current_dir(dir.path()).assert().success();

    // create label
    skis()
        .args(["label", "create", "bug", "--color", "ff0000", "--description", "Bug reports"])
        .current_dir(dir.path())
        .assert()
        .success();

    // create issue with label
    skis()
        .args(["issue", "create", "--title", "Critical bug", "--body", "Something broke", "--label", "bug"])
        .current_dir(dir.path())
        .assert()
        .success();

    // view
    skis()
        .args(["issue", "view", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Critical bug"))
        .stdout(predicate::str::contains("bug"));

    // edit
    skis()
        .args(["issue", "edit", "1", "--title", "Critical bug (updated)"])
        .current_dir(dir.path())
        .assert()
        .success();

    // comment
    skis()
        .args(["issue", "comment", "1", "--body", "Working on this"])
        .current_dir(dir.path())
        .assert()
        .success();

    // create another issue for linking
    skis()
        .args(["issue", "create", "--title", "Related issue"])
        .current_dir(dir.path())
        .assert()
        .success();

    // link
    skis()
        .args(["issue", "link", "1", "2"])
        .current_dir(dir.path())
        .assert()
        .success();

    // verify link shows in view
    skis()
        .args(["issue", "view", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Linked: #2"));

    // close
    skis()
        .args(["issue", "close", "1", "--reason", "completed"])
        .current_dir(dir.path())
        .assert()
        .success();

    // reopen
    skis()
        .args(["issue", "reopen", "1"])
        .current_dir(dir.path())
        .assert()
        .success();

    // delete
    skis()
        .args(["issue", "delete", "1", "--yes"])
        .current_dir(dir.path())
        .assert()
        .success();

    // restore
    skis()
        .args(["issue", "restore", "1"])
        .current_dir(dir.path())
        .assert()
        .success();

    // list with filters
    skis()
        .args(["issue", "list", "--state", "open", "--label", "bug"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Critical bug"));

    // search
    skis()
        .args(["issue", "list", "--search", "Critical"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Critical bug"));
}
