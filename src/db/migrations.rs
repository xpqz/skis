use rusqlite::Connection;

use crate::error::Result;

#[allow(dead_code)] // Used in tests
pub const LATEST_SCHEMA_VERSION: i32 = 1;

/// Run all pending migrations on the database
pub fn run_migrations(conn: &Connection) -> Result<()> {
    let current_version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if current_version < 1 {
        migrate_v0_to_v1(conn)?;
    }

    Ok(())
}

/// Initial schema creation (v0 -> v1)
fn migrate_v0_to_v1(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- Core issue table
        CREATE TABLE issues (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            body TEXT,
            type TEXT NOT NULL DEFAULT 'task' CHECK (type IN ('epic', 'task', 'bug', 'request')),
            state TEXT NOT NULL DEFAULT 'open' CHECK (state IN ('open', 'closed')),
            state_reason TEXT CHECK (state_reason IN ('completed', 'not_planned', NULL)),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            closed_at TEXT,
            deleted_at TEXT,
            CHECK ((state = 'open' AND state_reason IS NULL AND closed_at IS NULL) OR state = 'closed')
        );

        -- Trigger to auto-update updated_at on any change
        CREATE TRIGGER issues_update_timestamp AFTER UPDATE ON issues BEGIN
            UPDATE issues SET updated_at = datetime('now') WHERE id = new.id;
        END;

        -- Labels (many-to-many with issues)
        CREATE TABLE labels (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE COLLATE NOCASE,
            description TEXT,
            color TEXT
        );

        CREATE TABLE issue_labels (
            issue_id INTEGER NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
            label_id INTEGER NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
            PRIMARY KEY (issue_id, label_id)
        );

        -- Comments on issues
        CREATE TABLE comments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            issue_id INTEGER NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
            body TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Issue links (bidirectional)
        CREATE TABLE issue_links (
            issue_a_id INTEGER NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
            issue_b_id INTEGER NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (issue_a_id, issue_b_id),
            CHECK (issue_a_id < issue_b_id)
        );

        -- Full-text search
        CREATE VIRTUAL TABLE issues_fts USING fts5(
            title,
            body,
            content='issues',
            content_rowid='id'
        );

        -- FTS sync triggers
        CREATE TRIGGER issues_ai AFTER INSERT ON issues BEGIN
            INSERT INTO issues_fts(rowid, title, body) VALUES (new.id, new.title, new.body);
        END;

        CREATE TRIGGER issues_ad AFTER DELETE ON issues BEGIN
            INSERT INTO issues_fts(issues_fts, rowid, title, body) VALUES('delete', old.id, old.title, old.body);
        END;

        CREATE TRIGGER issues_au AFTER UPDATE ON issues BEGIN
            INSERT INTO issues_fts(issues_fts, rowid, title, body) VALUES('delete', old.id, old.title, old.body);
            INSERT INTO issues_fts(rowid, title, body) VALUES (new.id, new.title, new.body);
        END;

        -- Indexes
        CREATE INDEX idx_issues_type ON issues(type);
        CREATE INDEX idx_issues_state ON issues(state);
        CREATE INDEX idx_issues_deleted ON issues(deleted_at);
        CREATE INDEX idx_issues_created ON issues(created_at);
        CREATE INDEX idx_issues_updated ON issues(updated_at);
        CREATE INDEX idx_comments_issue ON comments(issue_id);
        CREATE INDEX idx_issue_links_a ON issue_links(issue_a_id);
        CREATE INDEX idx_issue_links_b ON issue_links(issue_b_id);

        PRAGMA user_version = 1;
        "#,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_db() -> (Connection, TempDir) {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = Connection::open(&db_path).unwrap();
        (conn, dir)
    }

    #[test]
    fn fresh_db_has_latest_schema_version() {
        let (conn, _dir) = test_db();
        run_migrations(&conn).unwrap();

        let version: i32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();

        assert_eq!(version, LATEST_SCHEMA_VERSION);
    }

    #[test]
    fn migration_is_idempotent() {
        let (conn, _dir) = test_db();

        run_migrations(&conn).unwrap();
        let result = run_migrations(&conn);

        assert!(result.is_ok());
    }

    #[test]
    fn schema_has_correct_tables() {
        let (conn, _dir) = test_db();
        run_migrations(&conn).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();

        assert!(tables.contains(&"issues".to_string()));
        assert!(tables.contains(&"labels".to_string()));
        assert!(tables.contains(&"issue_labels".to_string()));
        assert!(tables.contains(&"comments".to_string()));
        assert!(tables.contains(&"issue_links".to_string()));
        assert!(tables.contains(&"issues_fts".to_string()));
    }

    #[test]
    fn state_reason_requires_closed_state() {
        let (conn, _dir) = test_db();
        run_migrations(&conn).unwrap();

        // Cannot set state_reason on open issue
        let result = conn.execute(
            "INSERT INTO issues (title, state, state_reason) VALUES ('Test', 'open', 'completed')",
            [],
        );

        assert!(result.is_err());
    }

    #[test]
    fn closed_at_requires_closed_state() {
        let (conn, _dir) = test_db();
        run_migrations(&conn).unwrap();

        // Cannot set closed_at on open issue
        let result = conn.execute(
            "INSERT INTO issues (title, state, closed_at) VALUES ('Test', 'open', datetime('now'))",
            [],
        );

        assert!(result.is_err());
    }

    #[test]
    fn issue_type_constraint() {
        let (conn, _dir) = test_db();
        run_migrations(&conn).unwrap();

        let result = conn.execute(
            "INSERT INTO issues (title, type) VALUES ('Test', 'invalid')",
            [],
        );

        assert!(result.is_err());
    }

    #[test]
    fn label_name_case_insensitive_uniqueness() {
        let (conn, _dir) = test_db();
        run_migrations(&conn).unwrap();

        conn.execute("INSERT INTO labels (name) VALUES ('Bug')", [])
            .unwrap();

        let result = conn.execute("INSERT INTO labels (name) VALUES ('bug')", []);

        assert!(result.is_err());
    }

    #[test]
    fn issue_link_canonical_ordering() {
        let (conn, _dir) = test_db();
        run_migrations(&conn).unwrap();

        // Create two issues
        conn.execute("INSERT INTO issues (id, title) VALUES (1, 'Issue 1')", [])
            .unwrap();
        conn.execute("INSERT INTO issues (id, title) VALUES (2, 'Issue 2')", [])
            .unwrap();

        // Link with correct ordering should work
        let result = conn.execute(
            "INSERT INTO issue_links (issue_a_id, issue_b_id) VALUES (1, 2)",
            [],
        );
        assert!(result.is_ok());

        // Link with wrong ordering should fail
        conn.execute("DELETE FROM issue_links", []).unwrap();
        let result = conn.execute(
            "INSERT INTO issue_links (issue_a_id, issue_b_id) VALUES (2, 1)",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn updated_at_trigger_fires() {
        let (conn, _dir) = test_db();
        run_migrations(&conn).unwrap();

        // Drop the trigger temporarily so we can set a specific timestamp
        conn.execute("DROP TRIGGER issues_update_timestamp", [])
            .unwrap();

        conn.execute("INSERT INTO issues (title) VALUES ('Test')", [])
            .unwrap();

        // Set a known past timestamp
        conn.execute(
            "UPDATE issues SET updated_at = '2020-01-01 00:00:00' WHERE id = 1",
            [],
        )
        .unwrap();

        let before: String = conn
            .query_row("SELECT updated_at FROM issues WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(before, "2020-01-01 00:00:00");

        // Recreate the trigger
        conn.execute(
            r#"CREATE TRIGGER issues_update_timestamp AFTER UPDATE ON issues BEGIN
                UPDATE issues SET updated_at = datetime('now') WHERE id = new.id;
            END"#,
            [],
        )
        .unwrap();

        // Update title - trigger should fire and change updated_at to current time
        conn.execute("UPDATE issues SET title = 'Updated' WHERE id = 1", [])
            .unwrap();

        let after: String = conn
            .query_row("SELECT updated_at FROM issues WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();

        // After should be different from our manually set timestamp
        assert_ne!(before, after);
        // After should be much more recent than 2020
        assert!(after > "2024-01-01 00:00:00".to_string());
    }
}
