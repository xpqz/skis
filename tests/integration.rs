//! Integration tests for SKIS
//!
//! These tests verify cross-module behavior and database operations
//! that span multiple components.

use ski::db::SkisDb;
use ski::error::Error;
use tempfile::TempDir;

/// Helper to create a test database in a temporary directory
fn test_db() -> (SkisDb, TempDir) {
    let dir = TempDir::new().unwrap();
    let db = SkisDb::init(dir.path()).unwrap();
    (db, dir)
}

#[test]
fn init_and_reopen_database() {
    let dir = TempDir::new().unwrap();

    // Initialize
    let _db = SkisDb::init(dir.path()).unwrap();
    drop(_db);

    // Reopen
    let skis_dir = dir.path().join(".skis");
    let result = SkisDb::open_at(&skis_dir);
    assert!(result.is_ok());
}

#[test]
fn double_init_fails() {
    let dir = TempDir::new().unwrap();

    SkisDb::init(dir.path()).unwrap();
    let result = SkisDb::init(dir.path());

    assert!(matches!(result.unwrap_err(), Error::AlreadyInitialized));
}

#[test]
fn foreign_key_cascade_delete() {
    let (db, _dir) = test_db();
    let conn = db.conn();

    // Create an issue
    conn.execute("INSERT INTO issues (title) VALUES ('Test issue')", [])
        .unwrap();

    // Add a label
    conn.execute("INSERT INTO labels (name) VALUES ('bug')", [])
        .unwrap();

    // Associate label with issue
    conn.execute(
        "INSERT INTO issue_labels (issue_id, label_id) VALUES (1, 1)",
        [],
    )
    .unwrap();

    // Add a comment
    conn.execute(
        "INSERT INTO comments (issue_id, body) VALUES (1, 'Test comment')",
        [],
    )
    .unwrap();

    // Delete the issue - should cascade to issue_labels and comments
    conn.execute("DELETE FROM issues WHERE id = 1", []).unwrap();

    // Verify cascade
    let label_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM issue_labels", [], |row| row.get(0))
        .unwrap();
    assert_eq!(label_count, 0);

    let comment_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM comments", [], |row| row.get(0))
        .unwrap();
    assert_eq!(comment_count, 0);

    // Label itself should still exist
    let label_exists: i64 = conn
        .query_row("SELECT COUNT(*) FROM labels", [], |row| row.get(0))
        .unwrap();
    assert_eq!(label_exists, 1);
}

#[test]
fn fts_search_finds_issues() {
    let (db, _dir) = test_db();
    let conn = db.conn();

    // Create issues with searchable content
    conn.execute(
        "INSERT INTO issues (title, body) VALUES ('Authentication bug', 'Login fails for users')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO issues (title, body) VALUES ('UI improvement', 'Make buttons bigger')",
        [],
    )
    .unwrap();

    // Search for 'login'
    let results: Vec<i64> = conn
        .prepare("SELECT rowid FROM issues_fts WHERE issues_fts MATCH 'login'")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0], 1);
}

#[test]
fn fts_updates_on_issue_change() {
    let (db, _dir) = test_db();
    let conn = db.conn();

    // Create issue
    conn.execute(
        "INSERT INTO issues (title, body) VALUES ('Original title', 'Some body text')",
        [],
    )
    .unwrap();

    // Update the issue title
    conn.execute(
        "UPDATE issues SET title = 'Updated authentication' WHERE id = 1",
        [],
    )
    .unwrap();

    // Old title term should not match (only "original" was in title)
    let old_results: Vec<i64> = conn
        .prepare("SELECT rowid FROM issues_fts WHERE issues_fts MATCH 'original'")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(old_results.is_empty());

    // New term should match
    let new_results: Vec<i64> = conn
        .prepare("SELECT rowid FROM issues_fts WHERE issues_fts MATCH 'authentication'")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(new_results.len(), 1);

    // Body text should still be searchable
    let body_results: Vec<i64> = conn
        .prepare("SELECT rowid FROM issues_fts WHERE issues_fts MATCH 'body'")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(body_results.len(), 1);
}

#[test]
fn fts_removes_on_issue_delete() {
    let (db, _dir) = test_db();
    let conn = db.conn();

    // Create issue
    conn.execute(
        "INSERT INTO issues (title, body) VALUES ('Searchable content', 'findme keyword')",
        [],
    )
    .unwrap();

    // Verify FTS has the entry
    let before: Vec<i64> = conn
        .prepare("SELECT rowid FROM issues_fts WHERE issues_fts MATCH 'findme'")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(before.len(), 1);

    // Delete the issue
    conn.execute("DELETE FROM issues WHERE id = 1", []).unwrap();

    // FTS should be empty
    let after: Vec<i64> = conn
        .prepare("SELECT rowid FROM issues_fts WHERE issues_fts MATCH 'findme'")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(after.is_empty());
}

#[test]
fn issue_link_bidirectional_constraint() {
    let (db, _dir) = test_db();
    let conn = db.conn();

    // Create two issues
    conn.execute("INSERT INTO issues (title) VALUES ('Issue A')", [])
        .unwrap();
    conn.execute("INSERT INTO issues (title) VALUES ('Issue B')", [])
        .unwrap();

    // Link with canonical ordering (a < b) works
    conn.execute(
        "INSERT INTO issue_links (issue_a_id, issue_b_id) VALUES (1, 2)",
        [],
    )
    .unwrap();

    // Verify the link exists
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM issue_links", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn issue_state_constraints() {
    let (db, _dir) = test_db();
    let conn = db.conn();

    // Open issue cannot have state_reason
    let result = conn.execute(
        "INSERT INTO issues (title, state, state_reason) VALUES ('Test', 'open', 'completed')",
        [],
    );
    assert!(result.is_err());

    // Closed issue can have state_reason
    let result = conn.execute(
        "INSERT INTO issues (title, state, state_reason, closed_at) VALUES ('Test', 'closed', 'completed', datetime('now'))",
        [],
    );
    assert!(result.is_ok());
}
