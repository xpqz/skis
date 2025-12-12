// Query helpers for SKIS database operations

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{Error, Result};
use crate::models::{
    generate_color, validate_color, Comment, Issue, IssueCreate, IssueFilter, IssueState,
    IssueType, IssueUpdate, Label, SortField, SortOrder, StateReason,
};

/// Create a new issue with optional labels
pub fn create_issue(conn: &Connection, create: &IssueCreate) -> Result<Issue> {
    let tx = conn.unchecked_transaction()?;

    // Verify all labels exist first
    for label_name in &create.labels {
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM labels WHERE name = ?1 COLLATE NOCASE)",
            [label_name],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(Error::LabelNotFound(label_name.clone()));
        }
    }

    // Insert the issue
    tx.execute(
        "INSERT INTO issues (title, body, type) VALUES (?1, ?2, ?3)",
        params![create.title, create.body, create.issue_type.to_string()],
    )?;

    let issue_id = tx.last_insert_rowid();

    // Add labels (use INSERT OR IGNORE to handle duplicates from user input)
    for label_name in &create.labels {
        tx.execute(
            "INSERT OR IGNORE INTO issue_labels (issue_id, label_id)
             SELECT ?1, id FROM labels WHERE name = ?2 COLLATE NOCASE",
            params![issue_id, label_name],
        )?;
    }

    tx.commit()?;

    // Fetch and return the created issue
    get_issue(conn, issue_id)?.ok_or(Error::IssueNotFound(issue_id))
}

/// Get a single issue by ID (returns None if not found, but DOES return deleted issues)
pub fn get_issue(conn: &Connection, id: i64) -> Result<Option<Issue>> {
    let issue = conn
        .query_row(
            "SELECT id, title, body, type, state, state_reason, created_at, updated_at, closed_at, deleted_at
             FROM issues WHERE id = ?1",
            [id],
            |row| {
                Ok(Issue {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    body: row.get(2)?,
                    issue_type: parse_issue_type(row.get::<_, String>(3)?),
                    state: parse_issue_state(row.get::<_, String>(4)?),
                    state_reason: row.get::<_, Option<String>>(5)?.map(parse_state_reason),
                    created_at: parse_datetime(row.get::<_, String>(6)?),
                    updated_at: parse_datetime(row.get::<_, String>(7)?),
                    closed_at: row.get::<_, Option<String>>(8)?.map(parse_datetime),
                    deleted_at: row.get::<_, Option<String>>(9)?.map(parse_datetime),
                })
            },
        )
        .optional()?;

    Ok(issue)
}

/// List issues with filtering, sorting, and pagination
pub fn list_issues(conn: &Connection, filter: &IssueFilter) -> Result<Vec<Issue>> {
    let mut sql = String::from(
        "SELECT DISTINCT i.id, i.title, i.body, i.type, i.state, i.state_reason,
                i.created_at, i.updated_at, i.closed_at, i.deleted_at
         FROM issues i",
    );

    let mut conditions = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    // Join with issue_labels if filtering by labels
    if !filter.labels.is_empty() {
        sql.push_str(
            " INNER JOIN issue_labels il ON i.id = il.issue_id
              INNER JOIN labels l ON il.label_id = l.id",
        );
    }

    // Filter by state
    if let Some(state) = &filter.state {
        conditions.push(format!("i.state = ?{}", params.len() + 1));
        params.push(Box::new(state.to_string()));
    }

    // Filter by type
    if let Some(issue_type) = &filter.issue_type {
        conditions.push(format!("i.type = ?{}", params.len() + 1));
        params.push(Box::new(issue_type.to_string()));
    }

    // Filter by labels (AND logic - must have all specified labels)
    for label in &filter.labels {
        conditions.push(format!("l.name = ?{} COLLATE NOCASE", params.len() + 1));
        params.push(Box::new(label.clone()));
    }

    // Exclude deleted by default
    if !filter.include_deleted {
        conditions.push("i.deleted_at IS NULL".to_string());
    }

    // Build WHERE clause
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }

    // For multiple label filtering with AND logic, we need to ensure the issue has ALL labels
    // Dedup labels case-insensitively to avoid count mismatches
    if filter.labels.len() > 1 {
        let mut seen = std::collections::HashSet::new();
        let deduped_labels: Vec<&String> = filter
            .labels
            .iter()
            .filter(|l| seen.insert(l.to_lowercase()))
            .collect();

        sql = format!(
            "SELECT id, title, body, type, state, state_reason, created_at, updated_at, closed_at, deleted_at
             FROM issues i
             WHERE {}
             AND (SELECT COUNT(DISTINCT l.name COLLATE NOCASE) FROM issue_labels il
                  INNER JOIN labels l ON il.label_id = l.id
                  WHERE il.issue_id = i.id AND l.name IN ({}) COLLATE NOCASE) = ?{}",
            if filter.include_deleted {
                "1=1"
            } else {
                "i.deleted_at IS NULL"
            },
            deduped_labels
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", i + 1))
                .collect::<Vec<_>>()
                .join(", "),
            deduped_labels.len() + 1
        );
        params.clear();
        for label in &deduped_labels {
            params.push(Box::new((*label).clone()));
        }
        params.push(Box::new(deduped_labels.len() as i64));

        // Re-add state filter
        if let Some(state) = &filter.state {
            sql.push_str(&format!(" AND i.state = ?{}", params.len() + 1));
            params.push(Box::new(state.to_string()));
        }

        // Re-add type filter
        if let Some(issue_type) = &filter.issue_type {
            sql.push_str(&format!(" AND i.type = ?{}", params.len() + 1));
            params.push(Box::new(issue_type.to_string()));
        }
    }

    // Sort
    let sort_column = match filter.sort_by {
        SortField::Updated => "i.updated_at",
        SortField::Created => "i.created_at",
        SortField::Id => "i.id",
    };
    let sort_direction = match filter.sort_order {
        SortOrder::Asc => "ASC",
        SortOrder::Desc => "DESC",
    };
    sql.push_str(&format!(" ORDER BY {} {}", sort_column, sort_direction));

    // Pagination
    sql.push_str(&format!(" LIMIT {} OFFSET {}", filter.limit, filter.offset));

    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&sql)?;
    let issues = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(Issue {
                id: row.get(0)?,
                title: row.get(1)?,
                body: row.get(2)?,
                issue_type: parse_issue_type(row.get::<_, String>(3)?),
                state: parse_issue_state(row.get::<_, String>(4)?),
                state_reason: row.get::<_, Option<String>>(5)?.map(parse_state_reason),
                created_at: parse_datetime(row.get::<_, String>(6)?),
                updated_at: parse_datetime(row.get::<_, String>(7)?),
                closed_at: row.get::<_, Option<String>>(8)?.map(parse_datetime),
                deleted_at: row.get::<_, Option<String>>(9)?.map(parse_datetime),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(issues)
}

/// Close an issue with a reason
pub fn close_issue(conn: &Connection, id: i64, reason: StateReason) -> Result<Issue> {
    close_issue_with_comment(conn, id, reason, None)
}

/// Close an issue with an optional comment (atomic operation)
pub fn close_issue_with_comment(
    conn: &Connection,
    id: i64,
    reason: StateReason,
    comment: Option<&str>,
) -> Result<Issue> {
    let issue = get_issue(conn, id)?.ok_or(Error::IssueNotFound(id))?;

    if issue.state == IssueState::Closed {
        return Err(Error::InvalidStateTransition(id, "closed".to_string()));
    }

    let tx = conn.unchecked_transaction()?;

    tx.execute(
        "UPDATE issues SET state = 'closed', state_reason = ?1, closed_at = datetime('now')
         WHERE id = ?2",
        params![reason.to_string(), id],
    )?;

    if let Some(body) = comment {
        tx.execute(
            "INSERT INTO comments (issue_id, body) VALUES (?1, ?2)",
            params![id, body],
        )?;
    }

    tx.commit()?;

    get_issue(conn, id)?.ok_or(Error::IssueNotFound(id))
}

/// Reopen a closed issue
pub fn reopen_issue(conn: &Connection, id: i64) -> Result<Issue> {
    let issue = get_issue(conn, id)?.ok_or(Error::IssueNotFound(id))?;

    if issue.state == IssueState::Open {
        return Err(Error::InvalidStateTransition(id, "open".to_string()));
    }

    conn.execute(
        "UPDATE issues SET state = 'open', state_reason = NULL, closed_at = NULL
         WHERE id = ?1",
        [id],
    )?;

    get_issue(conn, id)?.ok_or(Error::IssueNotFound(id))
}

/// Soft delete an issue
pub fn delete_issue(conn: &Connection, id: i64) -> Result<()> {
    let _issue = get_issue(conn, id)?.ok_or(Error::IssueNotFound(id))?;

    conn.execute(
        "UPDATE issues SET deleted_at = datetime('now') WHERE id = ?1",
        [id],
    )?;

    Ok(())
}

/// Restore a soft-deleted issue
pub fn restore_issue(conn: &Connection, id: i64) -> Result<Issue> {
    let _issue = get_issue(conn, id)?.ok_or(Error::IssueNotFound(id))?;

    conn.execute("UPDATE issues SET deleted_at = NULL WHERE id = ?1", [id])?;

    get_issue(conn, id)?.ok_or(Error::IssueNotFound(id))
}

/// Update an existing issue
pub fn update_issue(conn: &Connection, id: i64, update: &IssueUpdate) -> Result<Issue> {
    let _issue = get_issue(conn, id)?.ok_or(Error::IssueNotFound(id))?;

    let mut updates = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(title) = &update.title {
        params.push(Box::new(title.clone()));
        updates.push(format!("title = ?{}", params.len()));
    }

    if let Some(body) = &update.body {
        params.push(Box::new(body.clone()));
        updates.push(format!("body = ?{}", params.len()));
    }

    if let Some(issue_type) = &update.issue_type {
        params.push(Box::new(issue_type.to_string()));
        updates.push(format!("type = ?{}", params.len()));
    }

    if updates.is_empty() {
        return get_issue(conn, id)?.ok_or(Error::IssueNotFound(id));
    }

    params.push(Box::new(id));
    let sql = format!(
        "UPDATE issues SET {} WHERE id = ?{}",
        updates.join(", "),
        params.len()
    );

    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    conn.execute(&sql, params_refs.as_slice())?;

    get_issue(conn, id)?.ok_or(Error::IssueNotFound(id))
}

// Phase 2: Comment operations

/// Add a comment to an issue
pub fn add_comment(conn: &Connection, issue_id: i64, body: &str) -> Result<Comment> {
    // Verify issue exists
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
        [issue_id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(Error::IssueNotFound(issue_id));
    }

    conn.execute(
        "INSERT INTO comments (issue_id, body) VALUES (?1, ?2)",
        params![issue_id, body],
    )?;

    let comment_id = conn.last_insert_rowid();

    conn.query_row(
        "SELECT id, issue_id, body, created_at, updated_at FROM comments WHERE id = ?1",
        [comment_id],
        |row| {
            Ok(Comment {
                id: row.get(0)?,
                issue_id: row.get(1)?,
                body: row.get(2)?,
                created_at: parse_datetime(row.get(3)?),
                updated_at: parse_datetime(row.get(4)?),
            })
        },
    )
    .map_err(Error::from)
}

/// Get all comments for an issue, ordered by creation time
pub fn get_comments(conn: &Connection, issue_id: i64) -> Result<Vec<Comment>> {
    let mut stmt = conn.prepare(
        "SELECT id, issue_id, body, created_at, updated_at
         FROM comments
         WHERE issue_id = ?1
         ORDER BY created_at ASC",
    )?;

    let comments = stmt
        .query_map([issue_id], |row| {
            Ok(Comment {
                id: row.get(0)?,
                issue_id: row.get(1)?,
                body: row.get(2)?,
                created_at: parse_datetime(row.get(3)?),
                updated_at: parse_datetime(row.get(4)?),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(comments)
}

/// Update a comment's body
pub fn update_comment(conn: &Connection, comment_id: i64, body: &str) -> Result<Comment> {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let rows = conn.execute(
        "UPDATE comments SET body = ?1, updated_at = ?2 WHERE id = ?3",
        params![body, now, comment_id],
    )?;

    if rows == 0 {
        return Err(Error::CommentNotFound(comment_id));
    }

    let comment = conn.query_row(
        "SELECT id, issue_id, body, created_at, updated_at FROM comments WHERE id = ?1",
        [comment_id],
        |row| {
            Ok(Comment {
                id: row.get(0)?,
                issue_id: row.get(1)?,
                body: row.get(2)?,
                created_at: parse_datetime(row.get(3)?),
                updated_at: parse_datetime(row.get(4)?),
            })
        },
    )?;

    Ok(comment)
}

/// Delete a comment
pub fn delete_comment(conn: &Connection, comment_id: i64) -> Result<()> {
    let rows = conn.execute("DELETE FROM comments WHERE id = ?1", [comment_id])?;

    if rows == 0 {
        return Err(Error::CommentNotFound(comment_id));
    }

    Ok(())
}

// Phase 2: Search operations

/// Search issues using FTS5 full-text search
pub fn search_issues(conn: &Connection, query: &str, filter: &IssueFilter) -> Result<Vec<Issue>> {
    // Build the query dynamically based on filter
    let mut sql = String::from(
        "SELECT i.id, i.title, i.body, i.type, i.state, i.state_reason,
                i.created_at, i.updated_at, i.closed_at, i.deleted_at
         FROM issues i
         JOIN issues_fts fts ON i.id = fts.rowid
         WHERE issues_fts MATCH ?1",
    );

    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(query.to_string())];
    let mut param_idx = 2;

    // Add state filter
    if let Some(state) = &filter.state {
        sql.push_str(&format!(" AND i.state = ?{}", param_idx));
        params_vec.push(Box::new(state.to_string()));
        param_idx += 1;
    }

    // Add type filter
    if let Some(issue_type) = &filter.issue_type {
        sql.push_str(&format!(" AND i.type = ?{}", param_idx));
        params_vec.push(Box::new(issue_type.to_string()));
        param_idx += 1;
    }

    // Exclude deleted unless requested
    if !filter.include_deleted {
        sql.push_str(" AND i.deleted_at IS NULL");
    }

    // Add label filters (AND logic)
    for label in &filter.labels {
        sql.push_str(&format!(
            " AND EXISTS (SELECT 1 FROM issue_labels il
                          JOIN labels l ON il.label_id = l.id
                          WHERE il.issue_id = i.id AND l.name = ?{} COLLATE NOCASE)",
            param_idx
        ));
        params_vec.push(Box::new(label.clone()));
        param_idx += 1;
    }

    // Add sorting
    let sort_col = match filter.sort_by {
        SortField::Updated => "i.updated_at",
        SortField::Created => "i.created_at",
        SortField::Id => "i.id",
    };
    let sort_dir = match filter.sort_order {
        SortOrder::Asc => "ASC",
        SortOrder::Desc => "DESC",
    };
    sql.push_str(&format!(" ORDER BY {} {}", sort_col, sort_dir));

    // Add pagination
    sql.push_str(&format!(" LIMIT {} OFFSET {}", filter.limit, filter.offset));

    let mut stmt = conn.prepare(&sql)?;

    // Convert params to references
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let issues = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(Issue {
                id: row.get(0)?,
                title: row.get(1)?,
                body: row.get(2)?,
                issue_type: parse_issue_type(row.get(3)?),
                state: parse_issue_state(row.get(4)?),
                state_reason: row.get::<_, Option<String>>(5)?.map(parse_state_reason),
                created_at: parse_datetime(row.get(6)?),
                updated_at: parse_datetime(row.get(7)?),
                closed_at: row.get::<_, Option<String>>(8)?.map(parse_datetime),
                deleted_at: row.get::<_, Option<String>>(9)?.map(parse_datetime),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(issues)
}

// Phase 2: Link operations

/// Link two issues together (bidirectional)
pub fn add_link(conn: &Connection, issue_a: i64, issue_b: i64) -> Result<()> {
    // Check for self-link
    if issue_a == issue_b {
        return Err(Error::SelfLink);
    }

    // Check that both issues exist
    let issue_a_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
        [issue_a],
        |row| row.get(0),
    )?;
    if !issue_a_exists {
        return Err(Error::IssueNotFound(issue_a));
    }

    let issue_b_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
        [issue_b],
        |row| row.get(0),
    )?;
    if !issue_b_exists {
        return Err(Error::IssueNotFound(issue_b));
    }

    // Store with canonical ordering (smaller ID first)
    let (min_id, max_id) = if issue_a < issue_b {
        (issue_a, issue_b)
    } else {
        (issue_b, issue_a)
    };

    // Check if link already exists
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM issue_links WHERE issue_a_id = ?1 AND issue_b_id = ?2)",
        params![min_id, max_id],
        |row| row.get(0),
    )?;
    if exists {
        return Err(Error::DuplicateLink(min_id, max_id));
    }

    conn.execute(
        "INSERT INTO issue_links (issue_a_id, issue_b_id) VALUES (?1, ?2)",
        params![min_id, max_id],
    )?;

    Ok(())
}

/// Remove a link between two issues
pub fn remove_link(conn: &Connection, issue_a: i64, issue_b: i64) -> Result<()> {
    // Use canonical ordering
    let (min_id, max_id) = if issue_a < issue_b {
        (issue_a, issue_b)
    } else {
        (issue_b, issue_a)
    };

    conn.execute(
        "DELETE FROM issue_links WHERE issue_a_id = ?1 AND issue_b_id = ?2",
        params![min_id, max_id],
    )?;

    Ok(())
}

/// Get all issue IDs linked to a given issue
pub fn get_linked_issues(conn: &Connection, issue_id: i64) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare(
        "SELECT CASE WHEN issue_a_id = ?1 THEN issue_b_id ELSE issue_a_id END as linked_id
         FROM issue_links
         WHERE issue_a_id = ?1 OR issue_b_id = ?1",
    )?;

    let ids = stmt
        .query_map([issue_id], |row| row.get(0))?
        .collect::<std::result::Result<Vec<i64>, _>>()?;

    Ok(ids)
}

/// Get linked issues with their titles (for JSON output)
pub fn get_linked_issues_with_titles(
    conn: &Connection,
    issue_id: i64,
) -> Result<Vec<crate::models::LinkedIssueRef>> {
    let mut stmt = conn.prepare(
        "SELECT i.id, i.title
         FROM issues i
         INNER JOIN issue_links l ON (
             (l.issue_a_id = ?1 AND l.issue_b_id = i.id) OR
             (l.issue_b_id = ?1 AND l.issue_a_id = i.id)
         )
         WHERE i.id != ?1",
    )?;

    let refs = stmt
        .query_map([issue_id], |row| {
            Ok(crate::models::LinkedIssueRef {
                id: row.get(0)?,
                title: row.get(1)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(refs)
}

// Phase 3: Label operations

/// Create a new label
pub fn create_label(
    conn: &Connection,
    name: &str,
    description: Option<&str>,
    color: Option<&str>,
) -> Result<Label> {
    // Validate color if provided, otherwise auto-generate
    let final_color = match color {
        Some(c) => {
            validate_color(c)?;
            c.to_string()
        }
        None => generate_color(name),
    };

    conn.execute(
        "INSERT INTO labels (name, description, color) VALUES (?1, ?2, ?3)",
        params![name, description, final_color],
    )?;

    let label_id = conn.last_insert_rowid();

    conn.query_row(
        "SELECT id, name, description, color FROM labels WHERE id = ?1",
        [label_id],
        |row| {
            Ok(Label {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                color: row.get(3)?,
            })
        },
    )
    .map_err(Error::from)
}

/// List all labels
pub fn list_labels(conn: &Connection) -> Result<Vec<Label>> {
    let mut stmt = conn.prepare("SELECT id, name, description, color FROM labels ORDER BY name")?;

    let labels = stmt
        .query_map([], |row| {
            Ok(Label {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                color: row.get(3)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(labels)
}

/// Delete a label by name (case-insensitive)
pub fn delete_label(conn: &Connection, name: &str) -> Result<()> {
    let rows = conn.execute(
        "DELETE FROM labels WHERE name = ?1 COLLATE NOCASE",
        [name],
    )?;

    if rows == 0 {
        return Err(Error::LabelNotFound(name.to_string()));
    }

    Ok(())
}

/// Add a label to an issue (idempotent)
pub fn add_label_to_issue(conn: &Connection, issue_id: i64, label_name: &str) -> Result<()> {
    // Check if label exists
    let label_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM labels WHERE name = ?1 COLLATE NOCASE",
            [label_name],
            |row| row.get(0),
        )
        .optional()?;

    let label_id = label_id.ok_or_else(|| Error::LabelNotFound(label_name.to_string()))?;

    // Insert if not already present (idempotent)
    conn.execute(
        "INSERT OR IGNORE INTO issue_labels (issue_id, label_id) VALUES (?1, ?2)",
        params![issue_id, label_id],
    )?;

    Ok(())
}

/// Remove a label from an issue (idempotent)
pub fn remove_label_from_issue(conn: &Connection, issue_id: i64, label_name: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM issue_labels
         WHERE issue_id = ?1 AND label_id = (
             SELECT id FROM labels WHERE name = ?2 COLLATE NOCASE
         )",
        params![issue_id, label_name],
    )?;

    Ok(())
}

/// Get all labels for an issue
pub fn get_issue_labels(conn: &Connection, issue_id: i64) -> Result<Vec<Label>> {
    let mut stmt = conn.prepare(
        "SELECT l.id, l.name, l.description, l.color
         FROM labels l
         JOIN issue_labels il ON l.id = il.label_id
         WHERE il.issue_id = ?1
         ORDER BY l.name",
    )?;

    let labels = stmt
        .query_map([issue_id], |row| {
            Ok(Label {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                color: row.get(3)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(labels)
}

// Helper functions for parsing database values

fn parse_issue_type(s: String) -> IssueType {
    match s.as_str() {
        "epic" => IssueType::Epic,
        "task" => IssueType::Task,
        "bug" => IssueType::Bug,
        "request" => IssueType::Request,
        _ => IssueType::Task, // Default fallback
    }
}

fn parse_issue_state(s: String) -> IssueState {
    match s.as_str() {
        "open" => IssueState::Open,
        "closed" => IssueState::Closed,
        _ => IssueState::Open, // Default fallback
    }
}

fn parse_state_reason(s: String) -> StateReason {
    match s.as_str() {
        "completed" => StateReason::Completed,
        "not_planned" => StateReason::NotPlanned,
        _ => StateReason::Completed, // Default fallback
    }
}

fn parse_datetime(s: String) -> DateTime<Utc> {
    // SQLite stores as "YYYY-MM-DD HH:MM:SS"
    chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
        .map(|dt| dt.and_utc())
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::SkisDb;
    use tempfile::TempDir;

    fn test_db() -> (SkisDb, TempDir) {
        let dir = TempDir::new().unwrap();
        let db = SkisDb::init(dir.path()).unwrap();
        (db, dir)
    }

    // Task 1.6: create_issue tests

    #[test]
    fn create_issue_with_defaults() {
        let (db, _dir) = test_db();
        let create = IssueCreate {
            title: "Test issue".to_string(),
            ..Default::default()
        };

        let issue = create_issue(db.conn(), &create).unwrap();

        assert_eq!(issue.title, "Test issue");
        assert_eq!(issue.body, None);
        assert_eq!(issue.issue_type, IssueType::Task);
        assert_eq!(issue.state, IssueState::Open);
        assert!(issue.state_reason.is_none());
        assert!(issue.closed_at.is_none());
        assert!(issue.deleted_at.is_none());
    }

    #[test]
    fn create_issue_with_all_fields() {
        let (db, _dir) = test_db();
        let create = IssueCreate {
            title: "Bug report".to_string(),
            body: Some("This is the body".to_string()),
            issue_type: IssueType::Bug,
            labels: vec![],
        };

        let issue = create_issue(db.conn(), &create).unwrap();

        assert_eq!(issue.title, "Bug report");
        assert_eq!(issue.body, Some("This is the body".to_string()));
        assert_eq!(issue.issue_type, IssueType::Bug);
    }

    #[test]
    fn create_issue_with_labels() {
        let (db, _dir) = test_db();

        // Create a label first
        db.conn()
            .execute(
                "INSERT INTO labels (name, description) VALUES ('bug', 'Bug label')",
                [],
            )
            .unwrap();

        let create = IssueCreate {
            title: "Issue with label".to_string(),
            labels: vec!["bug".to_string()],
            ..Default::default()
        };

        let issue = create_issue(db.conn(), &create).unwrap();
        assert_eq!(issue.title, "Issue with label");

        // Verify label was attached
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM issue_labels WHERE issue_id = ?1",
                [issue.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn create_issue_with_nonexistent_label_fails() {
        let (db, _dir) = test_db();
        let create = IssueCreate {
            title: "Issue with bad label".to_string(),
            labels: vec!["nonexistent".to_string()],
            ..Default::default()
        };

        let result = create_issue(db.conn(), &create);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::LabelNotFound(_)));
    }

    #[test]
    fn create_issue_error_suggests_label_create() {
        let (db, _dir) = test_db();
        let create = IssueCreate {
            title: "Issue".to_string(),
            labels: vec!["missing".to_string()],
            ..Default::default()
        };

        let result = create_issue(db.conn(), &create);
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Label 'missing' not found"));
        assert!(msg.contains("skis label create missing"));
    }

    // Task 1.7: get_issue tests

    #[test]
    fn get_existing_issue() {
        let (db, _dir) = test_db();
        let create = IssueCreate {
            title: "Test".to_string(),
            ..Default::default()
        };
        let created = create_issue(db.conn(), &create).unwrap();

        let fetched = get_issue(db.conn(), created.id).unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().title, "Test");
    }

    #[test]
    fn get_nonexistent_issue_returns_none() {
        let (db, _dir) = test_db();
        let result = get_issue(db.conn(), 9999).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn get_deleted_issue_returns_issue() {
        let (db, _dir) = test_db();
        let create = IssueCreate {
            title: "To delete".to_string(),
            ..Default::default()
        };
        let created = create_issue(db.conn(), &create).unwrap();
        delete_issue(db.conn(), created.id).unwrap();

        // get_issue should still return it
        let fetched = get_issue(db.conn(), created.id).unwrap();
        assert!(fetched.is_some());
        assert!(fetched.unwrap().deleted_at.is_some());
    }

    // Task 1.8: list_issues tests

    #[test]
    fn list_with_default_filter_returns_all_states() {
        let (db, _dir) = test_db();

        // Create open and closed issues
        create_issue(
            db.conn(),
            &IssueCreate {
                title: "Open 1".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        create_issue(
            db.conn(),
            &IssueCreate {
                title: "Open 2".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let closed = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Closed".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        close_issue(db.conn(), closed.id, StateReason::Completed).unwrap();

        // IssueFilter::default() has state=None, which means "all states"
        // CLI will explicitly set state=Some(Open) to match PLAN.md default
        let filter = IssueFilter::default();
        let issues = list_issues(db.conn(), &filter).unwrap();
        assert_eq!(issues.len(), 3); // All 3 issues (2 open + 1 closed)
    }

    #[test]
    fn list_filter_by_state_open() {
        let (db, _dir) = test_db();

        create_issue(
            db.conn(),
            &IssueCreate {
                title: "Open".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let to_close = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Closed".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        close_issue(db.conn(), to_close.id, StateReason::Completed).unwrap();

        // This is what CLI will use by default (state=open per PLAN.md)
        let filter = IssueFilter {
            state: Some(IssueState::Open),
            ..Default::default()
        };
        let issues = list_issues(db.conn(), &filter).unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].title, "Open");
    }

    #[test]
    fn list_filter_by_state_closed() {
        let (db, _dir) = test_db();

        create_issue(
            db.conn(),
            &IssueCreate {
                title: "Open".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let to_close = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Closed".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        close_issue(db.conn(), to_close.id, StateReason::Completed).unwrap();

        let filter = IssueFilter {
            state: Some(IssueState::Closed),
            ..Default::default()
        };
        let issues = list_issues(db.conn(), &filter).unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].title, "Closed");
    }

    #[test]
    fn list_filter_by_type() {
        let (db, _dir) = test_db();

        create_issue(
            db.conn(),
            &IssueCreate {
                title: "Task".to_string(),
                issue_type: IssueType::Task,
                ..Default::default()
            },
        )
        .unwrap();
        create_issue(
            db.conn(),
            &IssueCreate {
                title: "Bug".to_string(),
                issue_type: IssueType::Bug,
                ..Default::default()
            },
        )
        .unwrap();

        let filter = IssueFilter {
            issue_type: Some(IssueType::Bug),
            ..Default::default()
        };
        let issues = list_issues(db.conn(), &filter).unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].title, "Bug");
    }

    #[test]
    fn list_filter_by_single_label() {
        let (db, _dir) = test_db();

        db.conn()
            .execute("INSERT INTO labels (name) VALUES ('urgent')", [])
            .unwrap();

        let labeled = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Labeled".to_string(),
                labels: vec!["urgent".to_string()],
                ..Default::default()
            },
        )
        .unwrap();
        create_issue(
            db.conn(),
            &IssueCreate {
                title: "Unlabeled".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        let filter = IssueFilter {
            labels: vec!["urgent".to_string()],
            ..Default::default()
        };
        let issues = list_issues(db.conn(), &filter).unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, labeled.id);
    }

    #[test]
    fn list_filter_by_multiple_labels_and_logic() {
        let (db, _dir) = test_db();

        db.conn()
            .execute("INSERT INTO labels (name) VALUES ('urgent')", [])
            .unwrap();
        db.conn()
            .execute("INSERT INTO labels (name) VALUES ('bug')", [])
            .unwrap();

        // Issue with both labels
        create_issue(
            db.conn(),
            &IssueCreate {
                title: "Both labels".to_string(),
                labels: vec!["urgent".to_string(), "bug".to_string()],
                ..Default::default()
            },
        )
        .unwrap();
        // Issue with only one label
        create_issue(
            db.conn(),
            &IssueCreate {
                title: "One label".to_string(),
                labels: vec!["urgent".to_string()],
                ..Default::default()
            },
        )
        .unwrap();

        // Filter requiring both labels (AND logic)
        let filter = IssueFilter {
            labels: vec!["urgent".to_string(), "bug".to_string()],
            ..Default::default()
        };
        let issues = list_issues(db.conn(), &filter).unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].title, "Both labels");
    }

    #[test]
    fn create_issue_with_duplicate_labels_is_idempotent() {
        let (db, _dir) = test_db();

        db.conn()
            .execute("INSERT INTO labels (name) VALUES ('bug')", [])
            .unwrap();

        // Create issue with same label specified twice
        let create = IssueCreate {
            title: "Duplicate labels".to_string(),
            labels: vec!["bug".to_string(), "bug".to_string()],
            ..Default::default()
        };

        let issue = create_issue(db.conn(), &create).unwrap();

        // Should only have one label attachment
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM issue_labels WHERE issue_id = ?1",
                [issue.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn create_issue_with_duplicate_labels_different_case() {
        let (db, _dir) = test_db();

        db.conn()
            .execute("INSERT INTO labels (name) VALUES ('Bug')", [])
            .unwrap();

        // Create issue with same label in different cases
        let create = IssueCreate {
            title: "Case duplicate".to_string(),
            labels: vec!["bug".to_string(), "BUG".to_string(), "Bug".to_string()],
            ..Default::default()
        };

        let issue = create_issue(db.conn(), &create).unwrap();

        // Should only have one label attachment
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM issue_labels WHERE issue_id = ?1",
                [issue.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn list_filter_with_duplicate_labels_case_insensitive() {
        let (db, _dir) = test_db();

        db.conn()
            .execute("INSERT INTO labels (name) VALUES ('bug')", [])
            .unwrap();
        db.conn()
            .execute("INSERT INTO labels (name) VALUES ('feature')", [])
            .unwrap();

        create_issue(
            db.conn(),
            &IssueCreate {
                title: "Has both".to_string(),
                labels: vec!["bug".to_string(), "feature".to_string()],
                ..Default::default()
            },
        )
        .unwrap();

        // Filter with duplicate labels in different cases should still find the issue
        let filter = IssueFilter {
            labels: vec!["bug".to_string(), "BUG".to_string(), "feature".to_string()],
            ..Default::default()
        };
        let issues = list_issues(db.conn(), &filter).unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].title, "Has both");
    }

    #[test]
    fn list_excludes_deleted_by_default() {
        let (db, _dir) = test_db();

        create_issue(
            db.conn(),
            &IssueCreate {
                title: "Active".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let to_delete = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Deleted".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        delete_issue(db.conn(), to_delete.id).unwrap();

        let filter = IssueFilter::default();
        let issues = list_issues(db.conn(), &filter).unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].title, "Active");
    }

    #[test]
    fn list_includes_deleted_with_flag() {
        let (db, _dir) = test_db();

        create_issue(
            db.conn(),
            &IssueCreate {
                title: "Active".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let to_delete = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Deleted".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        delete_issue(db.conn(), to_delete.id).unwrap();

        let filter = IssueFilter {
            include_deleted: true,
            ..Default::default()
        };
        let issues = list_issues(db.conn(), &filter).unwrap();
        assert_eq!(issues.len(), 2);
    }

    #[test]
    fn list_default_sort_updated_desc() {
        let (db, _dir) = test_db();

        let first = create_issue(
            db.conn(),
            &IssueCreate {
                title: "First".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let second = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Second".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        // Update first to make it most recently updated
        update_issue(
            db.conn(),
            first.id,
            &IssueUpdate {
                title: Some("First Updated".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        let filter = IssueFilter::default(); // Default: sort by updated DESC
        let issues = list_issues(db.conn(), &filter).unwrap();
        assert_eq!(issues[0].title, "First Updated");
        assert_eq!(issues[1].id, second.id);
    }

    #[test]
    fn list_sort_by_created_asc() {
        let (db, _dir) = test_db();

        create_issue(
            db.conn(),
            &IssueCreate {
                title: "First".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        create_issue(
            db.conn(),
            &IssueCreate {
                title: "Second".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        let filter = IssueFilter {
            sort_by: SortField::Created,
            sort_order: SortOrder::Asc,
            ..Default::default()
        };
        let issues = list_issues(db.conn(), &filter).unwrap();
        assert_eq!(issues[0].title, "First");
        assert_eq!(issues[1].title, "Second");
    }

    #[test]
    fn list_pagination_limit() {
        let (db, _dir) = test_db();

        for i in 1..=5 {
            create_issue(
                db.conn(),
                &IssueCreate {
                    title: format!("Issue {}", i),
                    ..Default::default()
                },
            )
            .unwrap();
        }

        let filter = IssueFilter {
            limit: 2,
            ..Default::default()
        };
        let issues = list_issues(db.conn(), &filter).unwrap();
        assert_eq!(issues.len(), 2);
    }

    #[test]
    fn list_pagination_offset() {
        let (db, _dir) = test_db();

        for i in 1..=5 {
            create_issue(
                db.conn(),
                &IssueCreate {
                    title: format!("Issue {}", i),
                    ..Default::default()
                },
            )
            .unwrap();
        }

        let filter = IssueFilter {
            sort_by: SortField::Id,
            sort_order: SortOrder::Asc,
            limit: 2,
            offset: 2,
            ..Default::default()
        };
        let issues = list_issues(db.conn(), &filter).unwrap();
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].title, "Issue 3");
        assert_eq!(issues[1].title, "Issue 4");
    }

    // Task 1.9: close_issue and reopen_issue tests

    #[test]
    fn close_issue_sets_fields() {
        let (db, _dir) = test_db();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "To close".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        let closed = close_issue(db.conn(), issue.id, StateReason::Completed).unwrap();

        assert_eq!(closed.state, IssueState::Closed);
        assert_eq!(closed.state_reason, Some(StateReason::Completed));
        assert!(closed.closed_at.is_some());
    }

    #[test]
    fn close_issue_already_closed_errors() {
        let (db, _dir) = test_db();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "To close".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        close_issue(db.conn(), issue.id, StateReason::Completed).unwrap();

        let result = close_issue(db.conn(), issue.id, StateReason::NotPlanned);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            Error::InvalidStateTransition(_, _)
        ));
    }

    #[test]
    fn reopen_issue_clears_fields() {
        let (db, _dir) = test_db();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "To reopen".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        close_issue(db.conn(), issue.id, StateReason::Completed).unwrap();

        let reopened = reopen_issue(db.conn(), issue.id).unwrap();

        assert_eq!(reopened.state, IssueState::Open);
        assert!(reopened.state_reason.is_none());
        assert!(reopened.closed_at.is_none());
    }

    #[test]
    fn reopen_issue_already_open_errors() {
        let (db, _dir) = test_db();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Already open".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        let result = reopen_issue(db.conn(), issue.id);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            Error::InvalidStateTransition(_, _)
        ));
    }

    #[test]
    fn updated_at_changes_on_close() {
        let (db, _dir) = test_db();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Test".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let original_updated = issue.updated_at;

        // Small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        let closed = close_issue(db.conn(), issue.id, StateReason::Completed).unwrap();
        assert!(closed.updated_at >= original_updated);
    }

    #[test]
    fn updated_at_changes_on_reopen() {
        let (db, _dir) = test_db();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Test".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let closed = close_issue(db.conn(), issue.id, StateReason::Completed).unwrap();
        let closed_updated = closed.updated_at;

        // Small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        let reopened = reopen_issue(db.conn(), issue.id).unwrap();
        assert!(reopened.updated_at >= closed_updated);
    }

    // Task 1.10: delete_issue and restore_issue tests

    #[test]
    fn soft_delete_sets_deleted_at() {
        let (db, _dir) = test_db();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "To delete".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        delete_issue(db.conn(), issue.id).unwrap();

        let deleted = get_issue(db.conn(), issue.id).unwrap().unwrap();
        assert!(deleted.deleted_at.is_some());
    }

    #[test]
    fn restore_clears_deleted_at() {
        let (db, _dir) = test_db();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "To restore".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        delete_issue(db.conn(), issue.id).unwrap();

        let restored = restore_issue(db.conn(), issue.id).unwrap();

        assert!(restored.deleted_at.is_none());
    }

    #[test]
    fn delete_nonexistent_issue_errors() {
        let (db, _dir) = test_db();
        let result = delete_issue(db.conn(), 9999);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::IssueNotFound(9999)));
    }

    // Task 2.1: update_issue tests

    #[test]
    fn update_issue_title_only() {
        let (db, _dir) = test_db();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Original".to_string(),
                body: Some("Body".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        let updated = update_issue(
            db.conn(),
            issue.id,
            &IssueUpdate {
                title: Some("New Title".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(updated.title, "New Title");
        assert_eq!(updated.body, Some("Body".to_string())); // Unchanged
    }

    #[test]
    fn update_issue_body_only() {
        let (db, _dir) = test_db();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Title".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        let updated = update_issue(
            db.conn(),
            issue.id,
            &IssueUpdate {
                body: Some("New body".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(updated.title, "Title"); // Unchanged
        assert_eq!(updated.body, Some("New body".to_string()));
    }

    #[test]
    fn update_issue_type_only() {
        let (db, _dir) = test_db();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Title".to_string(),
                issue_type: IssueType::Task,
                ..Default::default()
            },
        )
        .unwrap();

        let updated = update_issue(
            db.conn(),
            issue.id,
            &IssueUpdate {
                issue_type: Some(IssueType::Bug),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(updated.issue_type, IssueType::Bug);
    }

    #[test]
    fn update_issue_multiple_fields() {
        let (db, _dir) = test_db();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Old".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        let updated = update_issue(
            db.conn(),
            issue.id,
            &IssueUpdate {
                title: Some("New".to_string()),
                body: Some("Body".to_string()),
                issue_type: Some(IssueType::Epic),
            },
        )
        .unwrap();

        assert_eq!(updated.title, "New");
        assert_eq!(updated.body, Some("Body".to_string()));
        assert_eq!(updated.issue_type, IssueType::Epic);
    }

    #[test]
    fn update_issue_triggers_updated_at() {
        let (db, _dir) = test_db();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Original".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let original_updated = issue.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));

        let updated = update_issue(
            db.conn(),
            issue.id,
            &IssueUpdate {
                title: Some("Changed".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert!(updated.updated_at >= original_updated);
    }

    // Task 2.3: Comment tests

    #[test]
    fn add_comment_to_issue() {
        let (db, _dir) = test_db();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Test".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        let comment = add_comment(db.conn(), issue.id, "This is a comment").unwrap();

        assert_eq!(comment.issue_id, issue.id);
        assert_eq!(comment.body, "This is a comment");
        assert!(comment.id > 0);
    }

    #[test]
    fn get_comments_returns_in_order() {
        let (db, _dir) = test_db();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Test".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        add_comment(db.conn(), issue.id, "First").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        add_comment(db.conn(), issue.id, "Second").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        add_comment(db.conn(), issue.id, "Third").unwrap();

        let comments = get_comments(db.conn(), issue.id).unwrap();

        assert_eq!(comments.len(), 3);
        assert_eq!(comments[0].body, "First");
        assert_eq!(comments[1].body, "Second");
        assert_eq!(comments[2].body, "Third");
    }

    #[test]
    fn add_comment_to_nonexistent_issue_errors() {
        let (db, _dir) = test_db();

        let result = add_comment(db.conn(), 9999, "Comment");
        assert!(result.is_err());
    }

    // Task 2.6: Search tests

    #[test]
    fn search_finds_title_match() {
        let (db, _dir) = test_db();
        create_issue(
            db.conn(),
            &IssueCreate {
                title: "Login button broken".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        create_issue(
            db.conn(),
            &IssueCreate {
                title: "Update documentation".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        let results = search_issues(db.conn(), "login", &IssueFilter::default()).unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("Login"));
    }

    #[test]
    fn search_finds_body_match() {
        let (db, _dir) = test_db();
        create_issue(
            db.conn(),
            &IssueCreate {
                title: "Bug report".to_string(),
                body: Some("The authentication system fails".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        let results = search_issues(db.conn(), "authentication", &IssueFilter::default()).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Bug report");
    }

    #[test]
    fn search_respects_state_filter() {
        let (db, _dir) = test_db();
        let issue1 = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Open searchable issue".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let issue2 = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Closed searchable issue".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        close_issue(db.conn(), issue2.id, StateReason::Completed).unwrap();

        // Search only open issues
        let open_filter = IssueFilter {
            state: Some(IssueState::Open),
            ..Default::default()
        };
        let results = search_issues(db.conn(), "searchable", &open_filter).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, issue1.id);
    }

    #[test]
    fn search_respects_label_filter() {
        let (db, _dir) = test_db();

        // Create label
        db.conn()
            .execute("INSERT INTO labels (name) VALUES ('urgent')", [])
            .unwrap();

        let issue1 = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Important task".to_string(),
                labels: vec!["urgent".to_string()],
                ..Default::default()
            },
        )
        .unwrap();
        create_issue(
            db.conn(),
            &IssueCreate {
                title: "Important but not urgent".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        let filter = IssueFilter {
            labels: vec!["urgent".to_string()],
            ..Default::default()
        };
        let results = search_issues(db.conn(), "important", &filter).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, issue1.id);
    }

    // Task 2.8: Link tests

    #[test]
    fn link_is_bidirectional() {
        let (db, _dir) = test_db();
        let issue1 = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Issue 1".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let issue2 = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Issue 2".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        add_link(db.conn(), issue1.id, issue2.id).unwrap();

        // Both issues should see the link
        let links_from_1 = get_linked_issues(db.conn(), issue1.id).unwrap();
        let links_from_2 = get_linked_issues(db.conn(), issue2.id).unwrap();

        assert_eq!(links_from_1.len(), 1);
        assert_eq!(links_from_1[0], issue2.id);
        assert_eq!(links_from_2.len(), 1);
        assert_eq!(links_from_2[0], issue1.id);
    }

    #[test]
    fn link_order_does_not_matter() {
        let (db, _dir) = test_db();
        let issue1 = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Issue 1".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let issue2 = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Issue 2".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        // Link with larger ID first
        add_link(db.conn(), issue2.id, issue1.id).unwrap();

        let links = get_linked_issues(db.conn(), issue1.id).unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0], issue2.id);
    }

    #[test]
    fn duplicate_link_fails() {
        let (db, _dir) = test_db();
        let issue1 = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Issue 1".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let issue2 = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Issue 2".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        add_link(db.conn(), issue1.id, issue2.id).unwrap();
        let result = add_link(db.conn(), issue1.id, issue2.id);

        assert!(result.is_err());
    }

    #[test]
    fn duplicate_link_reversed_order_fails() {
        let (db, _dir) = test_db();
        let issue1 = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Issue 1".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let issue2 = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Issue 2".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        add_link(db.conn(), issue1.id, issue2.id).unwrap();
        // Try to link in reverse order - should fail as duplicate
        let result = add_link(db.conn(), issue2.id, issue1.id);

        assert!(result.is_err());
    }

    #[test]
    fn unlink_order_does_not_matter() {
        let (db, _dir) = test_db();
        let issue1 = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Issue 1".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let issue2 = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Issue 2".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        add_link(db.conn(), issue1.id, issue2.id).unwrap();
        // Remove with reversed order
        remove_link(db.conn(), issue2.id, issue1.id).unwrap();

        let links = get_linked_issues(db.conn(), issue1.id).unwrap();
        assert!(links.is_empty());
    }

    #[test]
    fn self_link_fails() {
        let (db, _dir) = test_db();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Issue".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        let result = add_link(db.conn(), issue.id, issue.id);
        assert!(result.is_err());
    }

    #[test]
    fn link_to_deleted_issue_allowed() {
        let (db, _dir) = test_db();
        let issue1 = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Issue 1".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        let issue2 = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Issue 2".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        delete_issue(db.conn(), issue2.id).unwrap();

        // Should still be able to link to deleted issue
        let result = add_link(db.conn(), issue1.id, issue2.id);
        assert!(result.is_ok());
    }

    // Phase 3: Label tests

    #[test]
    fn create_label_with_all_fields() {
        let (db, _dir) = test_db();

        let label = create_label(db.conn(), "bug", Some("Bug reports"), Some("d73a4a")).unwrap();

        assert_eq!(label.name, "bug");
        assert_eq!(label.description, Some("Bug reports".to_string()));
        assert_eq!(label.color, Some("d73a4a".to_string()));
        assert!(label.id > 0);
    }

    #[test]
    fn create_label_name_only() {
        let (db, _dir) = test_db();

        let label = create_label(db.conn(), "enhancement", None, None).unwrap();

        assert_eq!(label.name, "enhancement");
        assert_eq!(label.description, None);
        // Color is auto-generated when not provided
        assert!(label.color.is_some());
        assert_eq!(label.color.as_ref().unwrap().len(), 6);
    }

    #[test]
    fn create_label_invalid_color_errors() {
        let (db, _dir) = test_db();

        let result = create_label(db.conn(), "test", None, Some("invalid"));
        assert!(result.is_err());

        let result = create_label(db.conn(), "test", None, Some("#ff0000"));
        assert!(result.is_err());
    }

    #[test]
    fn create_label_duplicate_name_errors() {
        let (db, _dir) = test_db();

        create_label(db.conn(), "bug", None, None).unwrap();
        let result = create_label(db.conn(), "bug", None, None);

        assert!(result.is_err());
    }

    #[test]
    fn create_label_duplicate_name_different_case_errors() {
        let (db, _dir) = test_db();

        create_label(db.conn(), "bug", None, None).unwrap();
        let result = create_label(db.conn(), "BUG", None, None);

        assert!(result.is_err());
    }

    #[test]
    fn list_labels_returns_all() {
        let (db, _dir) = test_db();

        create_label(db.conn(), "bug", None, None).unwrap();
        create_label(db.conn(), "enhancement", None, None).unwrap();
        create_label(db.conn(), "docs", None, None).unwrap();

        let labels = list_labels(db.conn()).unwrap();

        assert_eq!(labels.len(), 3);
        let names: Vec<&str> = labels.iter().map(|l| l.name.as_str()).collect();
        assert!(names.contains(&"bug"));
        assert!(names.contains(&"enhancement"));
        assert!(names.contains(&"docs"));
    }

    #[test]
    fn delete_label_by_name() {
        let (db, _dir) = test_db();

        create_label(db.conn(), "bug", None, None).unwrap();
        delete_label(db.conn(), "bug").unwrap();

        let labels = list_labels(db.conn()).unwrap();
        assert!(labels.is_empty());
    }

    #[test]
    fn delete_label_case_insensitive() {
        let (db, _dir) = test_db();

        create_label(db.conn(), "bug", None, None).unwrap();
        delete_label(db.conn(), "BUG").unwrap();

        let labels = list_labels(db.conn()).unwrap();
        assert!(labels.is_empty());
    }

    #[test]
    fn delete_label_nonexistent_errors() {
        let (db, _dir) = test_db();

        let result = delete_label(db.conn(), "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn add_label_to_issue_test() {
        let (db, _dir) = test_db();

        create_label(db.conn(), "bug", None, None).unwrap();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Test".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        add_label_to_issue(db.conn(), issue.id, "bug").unwrap();

        let labels = get_issue_labels(db.conn(), issue.id).unwrap();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].name, "bug");
    }

    #[test]
    fn add_nonexistent_label_errors() {
        let (db, _dir) = test_db();

        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Test".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        let result = add_label_to_issue(db.conn(), issue.id, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn add_duplicate_label_is_idempotent() {
        let (db, _dir) = test_db();

        create_label(db.conn(), "bug", None, None).unwrap();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Test".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        add_label_to_issue(db.conn(), issue.id, "bug").unwrap();
        // Adding again should succeed (idempotent)
        add_label_to_issue(db.conn(), issue.id, "bug").unwrap();

        let labels = get_issue_labels(db.conn(), issue.id).unwrap();
        assert_eq!(labels.len(), 1);
    }

    #[test]
    fn remove_label_from_issue_test() {
        let (db, _dir) = test_db();

        create_label(db.conn(), "bug", None, None).unwrap();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Test".to_string(),
                labels: vec!["bug".to_string()],
                ..Default::default()
            },
        )
        .unwrap();

        remove_label_from_issue(db.conn(), issue.id, "bug").unwrap();

        let labels = get_issue_labels(db.conn(), issue.id).unwrap();
        assert!(labels.is_empty());
    }

    #[test]
    fn remove_nonexistent_label_is_idempotent() {
        let (db, _dir) = test_db();

        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Test".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        // Removing a label that's not on the issue should succeed (idempotent)
        let result = remove_label_from_issue(db.conn(), issue.id, "nonexistent");
        assert!(result.is_ok());
    }

    #[test]
    fn get_issue_labels_returns_all() {
        let (db, _dir) = test_db();

        create_label(db.conn(), "bug", None, None).unwrap();
        create_label(db.conn(), "urgent", None, None).unwrap();
        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Test".to_string(),
                labels: vec!["bug".to_string(), "urgent".to_string()],
                ..Default::default()
            },
        )
        .unwrap();

        let labels = get_issue_labels(db.conn(), issue.id).unwrap();

        assert_eq!(labels.len(), 2);
        let names: Vec<&str> = labels.iter().map(|l| l.name.as_str()).collect();
        assert!(names.contains(&"bug"));
        assert!(names.contains(&"urgent"));
    }

    #[test]
    fn get_issue_labels_empty() {
        let (db, _dir) = test_db();

        let issue = create_issue(
            db.conn(),
            &IssueCreate {
                title: "Test".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        let labels = get_issue_labels(db.conn(), issue.id).unwrap();
        assert!(labels.is_empty());
    }
}
