use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::{Error, Result};

use super::migrations;

const SKIS_DIR: &str = ".skis";
const DB_FILE: &str = "issues.db";

/// Database handle for SKIS operations
#[derive(Debug)]
pub struct SkisDb {
    conn: Connection,
}

impl SkisDb {
    /// Initialize a new SKIS repository at the given path.
    /// Creates `.skis/` directory and initializes the database.
    pub fn init(path: &Path) -> Result<Self> {
        let skis_dir = path.join(SKIS_DIR);

        if skis_dir.exists() {
            return Err(Error::AlreadyInitialized);
        }

        std::fs::create_dir_all(&skis_dir)?;

        let db_path = skis_dir.join(DB_FILE);
        let conn = Connection::open(&db_path)?;
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        migrations::run_migrations(&conn)?;

        Ok(Self { conn })
    }

    /// Open database, searching up from cwd for `.skis/` directory
    pub fn open() -> Result<Self> {
        let skis_dir = find_skis_dir()?;
        Self::open_at(&skis_dir)
    }

    /// Open database at a specific `.skis/` directory path
    pub fn open_at(skis_dir: &Path) -> Result<Self> {
        let db_path = skis_dir.join(DB_FILE);
        if !db_path.exists() {
            return Err(Error::NotARepository);
        }

        let conn = Connection::open(&db_path)?;
        conn.execute("PRAGMA foreign_keys = ON", [])?;
        Ok(Self { conn })
    }

    /// Get a reference to the underlying connection
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

/// Walk up from current directory looking for `.skis/` directory
pub fn find_skis_dir() -> Result<PathBuf> {
    let mut current = std::env::current_dir()?;

    loop {
        let skis_dir = current.join(SKIS_DIR);
        if skis_dir.is_dir() {
            return Ok(skis_dir);
        }

        if !current.pop() {
            return Err(Error::NotARepository);
        }
    }
}

/// Find `.skis/` directory starting from a specific path (for testing)
pub fn find_skis_dir_from(start: &Path) -> Result<PathBuf> {
    let mut current = start.to_path_buf();

    loop {
        let skis_dir = current.join(SKIS_DIR);
        if skis_dir.is_dir() {
            return Ok(skis_dir);
        }

        if !current.pop() {
            return Err(Error::NotARepository);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn finds_skis_in_current_dir() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join(SKIS_DIR)).unwrap();

        let result = find_skis_dir_from(dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), dir.path().join(SKIS_DIR));
    }

    #[test]
    fn finds_skis_in_parent_dir() {
        let dir = TempDir::new().unwrap();
        let subdir = dir.path().join("sub");
        std::fs::create_dir_all(&subdir).unwrap();
        std::fs::create_dir(dir.path().join(SKIS_DIR)).unwrap();

        let result = find_skis_dir_from(&subdir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), dir.path().join(SKIS_DIR));
    }

    #[test]
    fn finds_skis_in_grandparent_dir() {
        let dir = TempDir::new().unwrap();
        let subdir = dir.path().join("sub/subsub");
        std::fs::create_dir_all(&subdir).unwrap();
        std::fs::create_dir(dir.path().join(SKIS_DIR)).unwrap();

        let result = find_skis_dir_from(&subdir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), dir.path().join(SKIS_DIR));
    }

    #[test]
    fn errors_when_no_skis_dir() {
        let dir = TempDir::new().unwrap();

        let result = find_skis_dir_from(dir.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NotARepository));
    }

    #[test]
    fn init_creates_skis_directory() {
        let dir = TempDir::new().unwrap();

        let result = SkisDb::init(dir.path());
        assert!(result.is_ok());
        assert!(dir.path().join(SKIS_DIR).exists());
        assert!(dir.path().join(SKIS_DIR).join(DB_FILE).exists());
    }

    #[test]
    fn init_fails_if_already_initialized() {
        let dir = TempDir::new().unwrap();

        SkisDb::init(dir.path()).unwrap();
        let result = SkisDb::init(dir.path());

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::AlreadyInitialized));
    }

    #[test]
    fn open_succeeds_after_init() {
        let dir = TempDir::new().unwrap();
        SkisDb::init(dir.path()).unwrap();

        let skis_dir = dir.path().join(SKIS_DIR);
        let result = SkisDb::open_at(&skis_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn open_fails_without_init() {
        let dir = TempDir::new().unwrap();
        let skis_dir = dir.path().join(SKIS_DIR);

        let result = SkisDb::open_at(&skis_dir);
        assert!(result.is_err());
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let dir = TempDir::new().unwrap();
        let db = SkisDb::init(dir.path()).unwrap();

        // Try to insert a comment referencing non-existent issue
        let result = db.conn().execute(
            "INSERT INTO comments (issue_id, body) VALUES (999, 'orphan comment')",
            [],
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("FOREIGN KEY constraint failed"));
    }
}
