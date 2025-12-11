# TESTING.md

Comprehensive testing strategy for SKIS to enable fearless refactoring and prevent regressions.

---

## Testing Philosophy

1. **Test behavior, not implementation**: Tests should verify what the system does, not how it does it internally
2. **Fast feedback loop**: Unit tests run in milliseconds; integration tests use in-memory SQLite
3. **Design decisions as tests**: Each design decision in PLAN.md should have corresponding test cases
4. **Regression tests for bugs**: Every bug fix gets a test that would have caught it

---

## Test Organization

```
tests/
├── unit/                    # Fast, isolated tests (mocked dependencies)
│   ├── models/
│   │   ├── issue_test.rs
│   │   ├── label_test.rs
│   │   └── comment_test.rs
│   └── validation_test.rs
├── integration/             # Database tests with in-memory SQLite
│   ├── db/
│   │   ├── schema_test.rs
│   │   ├── issue_crud_test.rs
│   │   ├── label_crud_test.rs
│   │   ├── comment_test.rs
│   │   ├── link_test.rs
│   │   ├── fts_test.rs
│   │   └── migration_test.rs
│   └── commands/
│       ├── issue_test.rs
│       └── label_test.rs
└── cli/                     # End-to-end CLI tests
    ├── init_test.rs
    ├── issue_workflow_test.rs
    └── label_workflow_test.rs
```

---

## Running Tests

```bash
cargo test                          # Run all tests
cargo test unit                     # Run only unit tests
cargo test integration              # Run only integration tests
cargo test cli                      # Run only CLI tests
cargo test issue                    # Run tests matching "issue"
cargo test -- --nocapture           # Show println! output
cargo test -- --test-threads=1      # Run sequentially (for debugging)
```

---

## Test Utilities

### Database Test Fixture

Create a test helper module for consistent database setup:

```rust
// tests/common/mod.rs

use ski::db::SkisDb;
use tempfile::TempDir;

/// Creates a database in a temp directory for fast, isolated tests.
/// Returns both the db and TempDir (keep TempDir alive to prevent cleanup).
pub fn test_db() -> (SkisDb, TempDir) {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let db = SkisDb::init(dir.path()).expect("Failed to init db");
    (db, dir)
}

/// Seeds database with common test data
pub fn seed_issues(db: &SkisDb) {
    db.create_label("bug", Some("Something broken"), Some("d73a4a")).unwrap();
    db.create_label("enhancement", None, Some("a2eeef")).unwrap();

    db.create_issue(&IssueCreate {
        title: "First issue".into(),
        body: Some("Body text".into()),
        issue_type: IssueType::Task,
        labels: vec![],
    }).unwrap();
    // ... more seed data
}
```

### Time Control

For tests involving timestamp ordering, avoid `thread::sleep()` which is flaky under CI load.
Instead, use direct SQL to set timestamps deterministically:

```rust
/// Sets created_at/updated_at for an issue to enable deterministic ordering tests
pub fn set_issue_timestamps(db: &SkisDb, issue_id: i64, created: &str, updated: &str) {
    db.conn().execute(
        "UPDATE issues SET created_at = ?, updated_at = ? WHERE id = ?",
        [created, updated, &issue_id.to_string()],
    ).unwrap();
}

// Usage in tests:
let (db, _dir) = test_db();
let issue1 = db.create_issue(&IssueCreate { title: "First".into(), ..Default::default() }).unwrap();
let issue2 = db.create_issue(&IssueCreate { title: "Second".into(), ..Default::default() }).unwrap();

// Force deterministic ordering
set_issue_timestamps(&db, issue1.id, "2024-01-01 10:00:00", "2024-01-01 10:00:00");
set_issue_timestamps(&db, issue2.id, "2024-01-01 11:00:00", "2024-01-01 11:00:00");
```

Note: This requires exposing `conn()` as a test-only accessor or using a `#[cfg(test)]` helper.

---

## Unit Tests

### Model Validation

```rust
// tests/unit/models/issue_test.rs

#[test]
fn issue_type_from_str_valid() {
    assert_eq!(IssueType::from_str("task"), Ok(IssueType::Task));
    assert_eq!(IssueType::from_str("bug"), Ok(IssueType::Bug));
    assert_eq!(IssueType::from_str("epic"), Ok(IssueType::Epic));
    assert_eq!(IssueType::from_str("request"), Ok(IssueType::Request));
}

#[test]
fn issue_type_from_str_invalid() {
    assert!(IssueType::from_str("invalid").is_err());
    assert!(IssueType::from_str("").is_err());
}

#[test]
fn issue_type_case_insensitive() {
    assert_eq!(IssueType::from_str("BUG"), Ok(IssueType::Bug));
    assert_eq!(IssueType::from_str("Bug"), Ok(IssueType::Bug));
}
```

### Label Color Validation

```rust
// tests/unit/validation_test.rs

#[test]
fn valid_hex_colors() {
    assert!(validate_color("ff0000").is_ok());
    assert!(validate_color("000000").is_ok());
    assert!(validate_color("ffffff").is_ok());
    assert!(validate_color("a2eeef").is_ok());
}

#[test]
fn invalid_hex_colors() {
    assert!(validate_color("#ff0000").is_err());  // No # prefix
    assert!(validate_color("ff000").is_err());    // Too short
    assert!(validate_color("ff00000").is_err());  // Too long
    assert!(validate_color("gggggg").is_err());   // Invalid chars
    assert!(validate_color("").is_err());         // Empty
}
```

---

## Integration Tests

### Schema and Constraints

Test that database constraints enforce design decisions:

```rust
// tests/integration/db/schema_test.rs

#[test]
fn state_reason_requires_closed_state() {
    let db = test_db().0;

    // Cannot set state_reason on open issue
    let result = db.conn.execute(
        "INSERT INTO issues (title, state, state_reason) VALUES (?, 'open', 'completed')",
        ["Test"]
    );
    assert!(result.is_err());
}

#[test]
fn closed_at_requires_closed_state() {
    let db = test_db().0;

    // Cannot set closed_at on open issue
    let result = db.conn.execute(
        "INSERT INTO issues (title, state, closed_at) VALUES (?, 'open', datetime('now'))",
        ["Test"]
    );
    assert!(result.is_err());
}

#[test]
fn issue_type_constraint() {
    let db = test_db().0;

    let result = db.conn.execute(
        "INSERT INTO issues (title, type) VALUES (?, 'invalid')",
        ["Test"]
    );
    assert!(result.is_err());
}

#[test]
fn label_name_case_insensitive_uniqueness() {
    let db = test_db().0;

    db.create_label("Bug", None, None).unwrap();
    let result = db.create_label("bug", None, None);
    assert!(result.is_err());  // Duplicate due to case-insensitivity
}

#[test]
fn updated_at_trigger_fires() {
    let (db, _dir) = test_db();
    let issue = db.create_issue(&IssueCreate {
        title: "Test".into(),
        ..Default::default()
    }).unwrap();

    // Set a known past timestamp
    set_issue_timestamps(&db, issue.id, "2024-01-01 10:00:00", "2024-01-01 10:00:00");
    let before_update = db.get_issue(issue.id).unwrap().unwrap();

    db.update_issue(issue.id, &IssueUpdate {
        title: Some("Updated".into()),
        ..Default::default()
    }).unwrap();

    let after_update = db.get_issue(issue.id).unwrap().unwrap();
    assert!(after_update.updated_at > before_update.updated_at);
}
```

### Issue CRUD

```rust
// tests/integration/db/issue_crud_test.rs

#[test]
fn create_issue_with_defaults() {
    let db = test_db().0;
    let issue = db.create_issue(&IssueCreate {
        title: "Test issue".into(),
        body: None,
        issue_type: IssueType::Task,
        labels: vec![],
    }).unwrap();

    assert_eq!(issue.title, "Test issue");
    assert_eq!(issue.issue_type, IssueType::Task);
    assert_eq!(issue.state, IssueState::Open);
    assert!(issue.state_reason.is_none());
    assert!(issue.closed_at.is_none());
    assert!(issue.deleted_at.is_none());
}

#[test]
fn create_issue_with_nonexistent_label_fails() {
    let db = test_db().0;
    let result = db.create_issue(&IssueCreate {
        title: "Test".into(),
        body: None,
        issue_type: IssueType::Task,
        labels: vec!["nonexistent".into()],
    });

    assert!(result.is_err());
    // Verify helpful error message
    let err = result.unwrap_err().to_string();
    assert!(err.contains("nonexistent"));
    assert!(err.contains("skis label create"));
}

#[test]
fn close_issue_sets_fields() {
    let db = test_db().0;
    let issue = db.create_issue(&IssueCreate {
        title: "Test".into(),
        ..Default::default()
    }).unwrap();

    let closed = db.close_issue(issue.id, StateReason::Completed).unwrap();

    assert_eq!(closed.state, IssueState::Closed);
    assert_eq!(closed.state_reason, Some(StateReason::Completed));
    assert!(closed.closed_at.is_some());
}

#[test]
fn reopen_issue_clears_closed_fields() {
    let db = test_db().0;
    let issue = db.create_issue(&IssueCreate {
        title: "Test".into(),
        ..Default::default()
    }).unwrap();

    db.close_issue(issue.id, StateReason::Completed).unwrap();
    let reopened = db.reopen_issue(issue.id).unwrap();

    assert_eq!(reopened.state, IssueState::Open);
    assert!(reopened.state_reason.is_none());
    assert!(reopened.closed_at.is_none());
}
```

### Soft Delete

```rust
// tests/integration/db/issue_crud_test.rs

#[test]
fn soft_delete_sets_deleted_at() {
    let db = test_db().0;
    let issue = db.create_issue(&IssueCreate {
        title: "Test".into(),
        ..Default::default()
    }).unwrap();

    db.delete_issue(issue.id).unwrap();

    let deleted = db.get_issue(issue.id).unwrap().unwrap();
    assert!(deleted.deleted_at.is_some());
}

#[test]
fn deleted_issues_excluded_from_list_by_default() {
    let db = test_db().0;
    db.create_issue(&IssueCreate { title: "Keep".into(), ..Default::default() }).unwrap();
    let to_delete = db.create_issue(&IssueCreate { title: "Delete".into(), ..Default::default() }).unwrap();

    db.delete_issue(to_delete.id).unwrap();

    let issues = db.list_issues(&IssueFilter::default()).unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].title, "Keep");
}

#[test]
fn deleted_issues_included_with_flag() {
    let db = test_db().0;
    db.create_issue(&IssueCreate { title: "Keep".into(), ..Default::default() }).unwrap();
    let to_delete = db.create_issue(&IssueCreate { title: "Delete".into(), ..Default::default() }).unwrap();

    db.delete_issue(to_delete.id).unwrap();

    let issues = db.list_issues(&IssueFilter {
        include_deleted: true,
        ..Default::default()
    }).unwrap();
    assert_eq!(issues.len(), 2);
}

#[test]
fn restore_clears_deleted_at() {
    let db = test_db().0;
    let issue = db.create_issue(&IssueCreate { title: "Test".into(), ..Default::default() }).unwrap();

    db.delete_issue(issue.id).unwrap();
    let restored = db.restore_issue(issue.id).unwrap();

    assert!(restored.deleted_at.is_none());
}
```

### Issue Links

```rust
// tests/integration/db/link_test.rs

#[test]
fn link_is_bidirectional() {
    let db = test_db().0;
    let a = db.create_issue(&IssueCreate { title: "A".into(), ..Default::default() }).unwrap();
    let b = db.create_issue(&IssueCreate { title: "B".into(), ..Default::default() }).unwrap();

    db.add_link(a.id, b.id).unwrap();

    // Both issues should see the link
    assert!(db.get_linked_issues(a.id).unwrap().contains(&b.id));
    assert!(db.get_linked_issues(b.id).unwrap().contains(&a.id));
}

#[test]
fn link_order_does_not_matter() {
    let db = test_db().0;
    let a = db.create_issue(&IssueCreate { title: "A".into(), ..Default::default() }).unwrap();
    let b = db.create_issue(&IssueCreate { title: "B".into(), ..Default::default() }).unwrap();

    // Link with reversed order should work the same
    db.add_link(b.id, a.id).unwrap();  // Note: b, a instead of a, b

    assert!(db.get_linked_issues(a.id).unwrap().contains(&b.id));
    assert!(db.get_linked_issues(b.id).unwrap().contains(&a.id));
}

#[test]
fn duplicate_link_fails() {
    let db = test_db().0;
    let a = db.create_issue(&IssueCreate { title: "A".into(), ..Default::default() }).unwrap();
    let b = db.create_issue(&IssueCreate { title: "B".into(), ..Default::default() }).unwrap();

    db.add_link(a.id, b.id).unwrap();
    let result = db.add_link(a.id, b.id);

    assert!(result.is_err());
}

#[test]
fn duplicate_link_reversed_order_fails() {
    let db = test_db().0;
    let a = db.create_issue(&IssueCreate { title: "A".into(), ..Default::default() }).unwrap();
    let b = db.create_issue(&IssueCreate { title: "B".into(), ..Default::default() }).unwrap();

    db.add_link(a.id, b.id).unwrap();
    let result = db.add_link(b.id, a.id);  // Reversed order

    assert!(result.is_err());  // Should still detect as duplicate
}

#[test]
fn unlink_order_does_not_matter() {
    let db = test_db().0;
    let a = db.create_issue(&IssueCreate { title: "A".into(), ..Default::default() }).unwrap();
    let b = db.create_issue(&IssueCreate { title: "B".into(), ..Default::default() }).unwrap();

    db.add_link(a.id, b.id).unwrap();
    db.remove_link(b.id, a.id).unwrap();  // Reversed order

    assert!(db.get_linked_issues(a.id).unwrap().is_empty());
    assert!(db.get_linked_issues(b.id).unwrap().is_empty());
}

#[test]
fn self_link_fails() {
    let db = test_db().0;
    let a = db.create_issue(&IssueCreate { title: "A".into(), ..Default::default() }).unwrap();

    let result = db.add_link(a.id, a.id);
    assert!(result.is_err());
}

#[test]
fn link_to_deleted_issue_allowed() {
    // Design decision: links persist even if one issue is soft-deleted
    let db = test_db().0;
    let a = db.create_issue(&IssueCreate { title: "A".into(), ..Default::default() }).unwrap();
    let b = db.create_issue(&IssueCreate { title: "B".into(), ..Default::default() }).unwrap();

    db.add_link(a.id, b.id).unwrap();
    db.delete_issue(b.id).unwrap();

    // Link should still be visible from the active issue
    assert!(db.get_linked_issues(a.id).unwrap().contains(&b.id));
}
```

### Label Filtering

```rust
// tests/integration/db/issue_crud_test.rs

#[test]
fn filter_by_single_label() {
    let db = test_db().0;
    db.create_label("bug", None, None).unwrap();
    db.create_label("feature", None, None).unwrap();

    db.create_issue(&IssueCreate { title: "Bug 1".into(), labels: vec!["bug".into()], ..Default::default() }).unwrap();
    db.create_issue(&IssueCreate { title: "Feature 1".into(), labels: vec!["feature".into()], ..Default::default() }).unwrap();

    let bugs = db.list_issues(&IssueFilter {
        labels: vec!["bug".into()],
        ..Default::default()
    }).unwrap();

    assert_eq!(bugs.len(), 1);
    assert_eq!(bugs[0].title, "Bug 1");
}

#[test]
fn filter_by_multiple_labels_uses_and_logic() {
    let db = test_db().0;
    db.create_label("bug", None, None).unwrap();
    db.create_label("priority", None, None).unwrap();

    db.create_issue(&IssueCreate { title: "Just bug".into(), labels: vec!["bug".into()], ..Default::default() }).unwrap();
    db.create_issue(&IssueCreate { title: "Both".into(), labels: vec!["bug".into(), "priority".into()], ..Default::default() }).unwrap();

    let both = db.list_issues(&IssueFilter {
        labels: vec!["bug".into(), "priority".into()],
        ..Default::default()
    }).unwrap();

    assert_eq!(both.len(), 1);
    assert_eq!(both[0].title, "Both");
}

#[test]
fn filter_label_case_insensitive() {
    let db = test_db().0;
    db.create_label("Bug", None, None).unwrap();

    db.create_issue(&IssueCreate { title: "Test".into(), labels: vec!["Bug".into()], ..Default::default() }).unwrap();

    // Filter with different case should work
    let issues = db.list_issues(&IssueFilter {
        labels: vec!["bug".into()],  // lowercase
        ..Default::default()
    }).unwrap();

    assert_eq!(issues.len(), 1);
}
```

### Full-Text Search

```rust
// tests/integration/db/fts_test.rs

#[test]
fn search_finds_title_match() {
    let db = test_db().0;
    db.create_issue(&IssueCreate { title: "Login bug".into(), ..Default::default() }).unwrap();
    db.create_issue(&IssueCreate { title: "Logout feature".into(), ..Default::default() }).unwrap();

    let results = db.search_issues("login", &IssueFilter::default()).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Login bug");
}

#[test]
fn search_finds_body_match() {
    let db = test_db().0;
    db.create_issue(&IssueCreate {
        title: "Issue".into(),
        body: Some("Authentication failure".into()),
        ..Default::default()
    }).unwrap();

    let results = db.search_issues("authentication", &IssueFilter::default()).unwrap();

    assert_eq!(results.len(), 1);
}

#[test]
fn search_respects_filters() {
    let db = test_db().0;
    let open = db.create_issue(&IssueCreate { title: "Login open".into(), ..Default::default() }).unwrap();
    let closed = db.create_issue(&IssueCreate { title: "Login closed".into(), ..Default::default() }).unwrap();
    db.close_issue(closed.id, StateReason::Completed).unwrap();

    let results = db.search_issues("login", &IssueFilter {
        state: Some(IssueState::Open),
        ..Default::default()
    }).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, open.id);
}

#[test]
fn fts_stays_in_sync_after_update() {
    let db = test_db().0;
    let issue = db.create_issue(&IssueCreate { title: "Old title".into(), ..Default::default() }).unwrap();

    db.update_issue(issue.id, &IssueUpdate {
        title: Some("New searchable title".into()),
        ..Default::default()
    }).unwrap();

    let old_results = db.search_issues("old", &IssueFilter::default()).unwrap();
    let new_results = db.search_issues("searchable", &IssueFilter::default()).unwrap();

    assert!(old_results.is_empty());
    assert_eq!(new_results.len(), 1);
}
```

### List Sorting and Pagination

```rust
// tests/integration/db/issue_crud_test.rs

#[test]
fn default_sort_is_updated_desc() {
    let (db, _dir) = test_db();
    let first = db.create_issue(&IssueCreate { title: "First".into(), ..Default::default() }).unwrap();
    let second = db.create_issue(&IssueCreate { title: "Second".into(), ..Default::default() }).unwrap();

    // Set deterministic timestamps (second is more recent)
    set_issue_timestamps(&db, first.id, "2024-01-01 10:00:00", "2024-01-01 10:00:00");
    set_issue_timestamps(&db, second.id, "2024-01-01 11:00:00", "2024-01-01 11:00:00");

    let issues = db.list_issues(&IssueFilter::default()).unwrap();

    assert_eq!(issues[0].id, second.id);  // Most recent first
    assert_eq!(issues[1].id, first.id);
}

#[test]
fn sort_by_created_asc() {
    let (db, _dir) = test_db();
    let first = db.create_issue(&IssueCreate { title: "First".into(), ..Default::default() }).unwrap();
    let second = db.create_issue(&IssueCreate { title: "Second".into(), ..Default::default() }).unwrap();

    // Set deterministic timestamps
    set_issue_timestamps(&db, first.id, "2024-01-01 10:00:00", "2024-01-01 10:00:00");
    set_issue_timestamps(&db, second.id, "2024-01-01 11:00:00", "2024-01-01 11:00:00");

    let issues = db.list_issues(&IssueFilter {
        sort_by: SortField::Created,
        sort_order: SortOrder::Asc,
        ..Default::default()
    }).unwrap();

    assert_eq!(issues[0].id, first.id);
    assert_eq!(issues[1].id, second.id);
}

#[test]
fn pagination_limit() {
    let db = test_db().0;
    for i in 0..10 {
        db.create_issue(&IssueCreate { title: format!("Issue {}", i), ..Default::default() }).unwrap();
    }

    let issues = db.list_issues(&IssueFilter {
        limit: 5,
        ..Default::default()
    }).unwrap();

    assert_eq!(issues.len(), 5);
}

#[test]
fn pagination_offset() {
    let db = test_db().0;
    for i in 0..10 {
        db.create_issue(&IssueCreate { title: format!("Issue {}", i), ..Default::default() }).unwrap();
    }

    let page1 = db.list_issues(&IssueFilter { limit: 3, offset: 0, ..Default::default() }).unwrap();
    let page2 = db.list_issues(&IssueFilter { limit: 3, offset: 3, ..Default::default() }).unwrap();

    // No overlap between pages
    let page1_ids: Vec<_> = page1.iter().map(|i| i.id).collect();
    let page2_ids: Vec<_> = page2.iter().map(|i| i.id).collect();
    assert!(page1_ids.iter().all(|id| !page2_ids.contains(id)));
}
```

---

## CLI Tests

End-to-end tests that invoke the actual binary:

```rust
// tests/cli/init_test.rs

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn skis() -> Command {
    Command::cargo_bin("skis").unwrap()
}

#[test]
fn init_creates_skis_directory() {
    let dir = TempDir::new().unwrap();

    skis()
        .current_dir(&dir)
        .arg("init")
        .assert()
        .success();

    assert!(dir.path().join(".skis").exists());
    assert!(dir.path().join(".skis/issues.db").exists());
}

#[test]
fn init_fails_if_already_initialized() {
    let dir = TempDir::new().unwrap();

    skis().current_dir(&dir).arg("init").assert().success();
    skis()
        .current_dir(&dir)
        .arg("init")
        .assert()
        .failure()
        .stderr(predicate::str::contains("already initialized"));
}

#[test]
fn commands_fail_without_init() {
    let dir = TempDir::new().unwrap();

    skis()
        .current_dir(&dir)
        .args(["issue", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not a skis repository"));
}
```

### Repository Discovery

```rust
// tests/cli/init_test.rs

#[test]
fn discovers_skis_in_parent_directory() {
    let dir = TempDir::new().unwrap();
    let subdir = dir.path().join("sub/dir");
    std::fs::create_dir_all(&subdir).unwrap();

    // Init at root
    skis().current_dir(&dir).arg("init").assert().success();

    // Create issue from subdirectory
    skis()
        .current_dir(&subdir)
        .args(["issue", "create", "-t", "Test"])
        .assert()
        .success();

    // Verify issue exists
    skis()
        .current_dir(&dir)
        .args(["issue", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Test"));
}
```

### Issue Workflow

```rust
// tests/cli/issue_workflow_test.rs

#[test]
fn full_issue_lifecycle() {
    let dir = TempDir::new().unwrap();
    skis().current_dir(&dir).arg("init").assert().success();

    // Create
    skis()
        .current_dir(&dir)
        .args(["issue", "create", "-t", "Bug report", "-b", "Details", "-T", "bug"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#1"));

    // View
    skis()
        .current_dir(&dir)
        .args(["issue", "view", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Bug report"))
        .stdout(predicate::str::contains("bug"))
        .stdout(predicate::str::contains("open"));

    // Close
    skis()
        .current_dir(&dir)
        .args(["issue", "close", "1", "-r", "completed"])
        .assert()
        .success();

    // Verify closed
    skis()
        .current_dir(&dir)
        .args(["issue", "view", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("closed"));

    // Reopen
    skis()
        .current_dir(&dir)
        .args(["issue", "reopen", "1"])
        .assert()
        .success();

    // Delete
    skis()
        .current_dir(&dir)
        .args(["issue", "delete", "1", "--yes"])
        .assert()
        .success();

    // Not in default list
    skis()
        .current_dir(&dir)
        .args(["issue", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Bug report").not());

    // Visible with --deleted
    skis()
        .current_dir(&dir)
        .args(["issue", "list", "--deleted"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Bug report"));

    // Restore
    skis()
        .current_dir(&dir)
        .args(["issue", "restore", "1"])
        .assert()
        .success();
}

#[test]
fn json_output_is_valid() {
    let dir = TempDir::new().unwrap();
    skis().current_dir(&dir).arg("init").assert().success();
    skis()
        .current_dir(&dir)
        .args(["issue", "create", "-t", "Test"])
        .assert()
        .success();

    let output = skis()
        .current_dir(&dir)
        .args(["issue", "view", "1", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["title"], "Test");
    assert_eq!(json["state"], "open");
}
```

---

## Error Message Tests

Verify helpful error messages for common mistakes:

```rust
#[test]
fn nonexistent_label_suggests_create() {
    let db = test_db().0;
    let result = db.create_issue(&IssueCreate {
        title: "Test".into(),
        labels: vec!["nonexistent".into()],
        ..Default::default()
    });

    let err = result.unwrap_err().to_string();
    assert!(err.contains("Label 'nonexistent' not found"));
    assert!(err.contains("skis label create nonexistent"));
}

#[test]
fn view_nonexistent_issue_error() {
    let dir = TempDir::new().unwrap();
    skis().current_dir(&dir).arg("init").assert().success();

    skis()
        .current_dir(&dir)
        .args(["issue", "view", "999"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Issue #999 not found"));
}

#[test]
fn close_already_closed_issue() {
    let db = test_db().0;
    let issue = db.create_issue(&IssueCreate { title: "Test".into(), ..Default::default() }).unwrap();
    db.close_issue(issue.id, StateReason::Completed).unwrap();

    let result = db.close_issue(issue.id, StateReason::Completed);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already closed"));
}

#[test]
fn reopen_already_open_issue() {
    let db = test_db().0;
    let issue = db.create_issue(&IssueCreate { title: "Test".into(), ..Default::default() }).unwrap();

    let result = db.reopen_issue(issue.id);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already open"));
}
```

---

## Migration Tests

```rust
// tests/integration/db/migration_test.rs

#[test]
fn fresh_db_has_latest_schema_version() {
    let db = test_db().0;
    let version: i32 = db.conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();

    assert_eq!(version, LATEST_SCHEMA_VERSION);
}

#[test]
fn migration_from_v0_to_v1() {
    // Create db with old schema, run migration, verify
    // ... specific to actual migration changes
}
```

---

## Property-Based Tests

For complex invariants, use proptest:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn issue_link_canonical_ordering(a in 1i64..1000, b in 1i64..1000) {
        prop_assume!(a != b);
        let db = test_db().0;

        // Create two issues
        for id in [a.min(b), a.max(b)] {
            db.conn.execute(
                "INSERT INTO issues (id, title) VALUES (?, 'test')",
                [id]
            ).ok();
        }

        // Link should work regardless of order
        let result = db.add_link(a, b);
        if result.is_ok() {
            // Verify stored with canonical ordering
            let stored: (i64, i64) = db.conn.query_row(
                "SELECT issue_a_id, issue_b_id FROM issue_links",
                [],
                |r| Ok((r.get(0)?, r.get(1)?))
            ).unwrap();

            prop_assert!(stored.0 < stored.1);
        }
    }
}
```

---

## Test Coverage Goals

| Layer | Target Coverage |
|-------|-----------------|
| Models / Validation | 90%+ |
| Database operations | 85%+ |
| CLI commands | 80%+ |
| Output formatting | 70%+ |

Run coverage with:

```bash
cargo tarpaulin --out Html
```

---

## CI Integration

```yaml
# .github/workflows/test.yml
name: Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --all-features
      - run: cargo clippy -- -D warnings
      - run: cargo fmt -- --check
```

---

## Testing Checklist for New Features

When implementing a new feature:

1. [ ] Unit tests for any new validation logic
2. [ ] Integration tests for database operations
3. [ ] CLI tests for new commands/flags
4. [ ] Error message tests for failure cases
5. [ ] Update existing tests if behavior changes
6. [ ] Add regression test if fixing a bug
