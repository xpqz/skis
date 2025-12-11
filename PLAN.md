# SKIS - Stefan's Issue System

A lightweight, local-first issue tracking system backed by SQLite3, written in Rust.

## Overview

SKIS is a command-line issue tracker designed for local-first software development. It stores all data in a local SQLite database, requiring no server or network connectivity. The CLI (`skis`) is modelled after GitHub's `gh issue` commands for familiarity.

## Design Principles

1. **Local-first**: All data stored locally in SQLite; works offline
2. **Simple**: Minimal dependencies, fast startup, intuitive commands
3. **Familiar**: CLI mirrors `gh issue` where sensible
4. **Portable**: Single binary, single database file

---

## Design Decisions

This section documents explicit choices made for ambiguous scenarios:

**Repository discovery**: When `.skis/` is not found walking up from cwd, all commands except `init` error out. No fallback to home directory or auto-creation.

**Issue state integrity**: Database constraints enforce that `state_reason` and `closed_at` can only be set when `state='closed'`. Reopening an issue clears both fields. A trigger auto-updates `updated_at` on any change.

**Soft deletes**: Issues are never hard-deleted. The `delete` command sets `deleted_at` timestamp. Deleted issues are excluded from listings by default but can be included with `--deleted` flag. This preserves history and allows recovery.

**Issue links**: Simple bidirectional links between issues with no semantic types (parent, blocks, etc.) at the database level. Stored once with canonical ordering (`issue_a_id < issue_b_id`) to prevent duplicates. When viewing either issue, the link is shown. Graph constraints can be enforced in the application layer later if needed.

**Label names**: Case-insensitive (`Bug` and `bug` are the same). Labels must be created before use; referencing a non-existent label errors with a helpful suggestion.

**Label colors**: Stored as 6-character hex without `#` prefix (e.g., `ff0000`). Validated in application code.

**List ordering**: Default sort is `updated_at DESC` (most recently active first). Explicit `--sort` and `--order` flags available.

**Label filtering**: Multiple `--label` flags use AND logic (issue must have all specified labels).

---

## Database Schema

### Location

The database file will be stored at `.skis/issues.db` in the project root (detected by walking up from cwd looking for `.skis/` directory, similar to how `.git/` works).

**Behavior when `.skis/` not found**: Commands other than `init` will error with "Not a skis repository (or any parent up to /). Run 'skis init' to create one." This prevents accidental writes to unexpected locations.

### Tables

```sql
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
    deleted_at TEXT,  -- soft delete: NULL = active, non-NULL = deleted
    -- Enforce: state_reason and closed_at only set when closed
    CHECK ((state = 'open' AND state_reason IS NULL AND closed_at IS NULL) OR state = 'closed')
);

-- Trigger to auto-update updated_at on any change
CREATE TRIGGER issues_update_timestamp AFTER UPDATE ON issues BEGIN
    UPDATE issues SET updated_at = datetime('now') WHERE id = new.id;
END;

-- Labels (many-to-many with issues)
-- Name uniqueness is case-insensitive via COLLATE NOCASE
CREATE TABLE labels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    description TEXT,
    color TEXT  -- hex color code without '#', e.g., 'ff0000' (validated in app)
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

-- Issue links (simple bidirectional references between issues)
-- Stored once with canonical ordering (issue_a_id < issue_b_id) to prevent duplicates
-- No semantic types at DB level; any graph constraints handled in app layer
CREATE TABLE issue_links (
    issue_a_id INTEGER NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    issue_b_id INTEGER NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (issue_a_id, issue_b_id),
    CHECK (issue_a_id < issue_b_id)  -- canonical ordering ensures no duplicates
);

-- Full-text search for issues
CREATE VIRTUAL TABLE issues_fts USING fts5(
    title,
    body,
    content='issues',
    content_rowid='id'
);

-- Triggers to keep FTS in sync
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

-- Indexes for common queries
CREATE INDEX idx_issues_type ON issues(type);
CREATE INDEX idx_issues_state ON issues(state);
CREATE INDEX idx_issues_deleted ON issues(deleted_at);
CREATE INDEX idx_issues_created ON issues(created_at);
CREATE INDEX idx_issues_updated ON issues(updated_at);
CREATE INDEX idx_comments_issue ON comments(issue_id);
CREATE INDEX idx_issue_links_a ON issue_links(issue_a_id);
CREATE INDEX idx_issue_links_b ON issue_links(issue_b_id);
```

---

## Architecture

### Crate Structure

```
ski/
├── Cargo.toml
├── src/
│   ├── main.rs           # CLI entry point, argument parsing
│   ├── lib.rs            # Library root, re-exports
│   ├── db/
│   │   ├── mod.rs
│   │   ├── connection.rs # Database connection management
│   │   ├── migrations.rs # Schema migrations
│   │   └── queries.rs    # Prepared statements / query helpers
│   ├── models/
│   │   ├── mod.rs
│   │   ├── issue.rs      # Issue struct and methods
│   │   ├── label.rs      # Label struct and methods
│   │   └── comment.rs    # Comment struct and methods
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── issue.rs      # issue subcommands (create, list, view, etc.)
│   │   └── label.rs      # label subcommands
│   └── output/
│       ├── mod.rs
│       └── format.rs     # Output formatting (table, JSON, etc.)
└── .skis/
    └── issues.db         # SQLite database (per-project)
```

### Key Dependencies

- `clap` - Command-line argument parsing with derive macros
- `rusqlite` - SQLite bindings for Rust
- `serde` / `serde_json` - Serialization for JSON output
- `chrono` - Date/time handling
- `colored` - Terminal colors for output
- `tabled` or manual formatting - Table output

### Core Components

#### 1. Database Layer (`db/`)

- **Connection management**: Find `.skis/` directory, open/create database
- **Migrations**: Version-controlled schema migrations
- **Query helpers**: Type-safe query builders

#### 2. Models (`models/`)

Each model struct mirrors a database table and provides:
- CRUD operations
- Validation
- Serialization (for JSON output)

#### 3. Commands (`commands/`)

Each command module handles CLI logic for a command group:
- Parse subcommand-specific arguments
- Call model methods
- Format and print output

#### 4. Output (`output/`)

- Table formatting for `list` commands
- Detailed view formatting for `view` commands
- JSON output when `--json` flag is used

---

## CLI Interface

### Initialization

```
skis init
```

Creates `.skis/` directory and initializes the database in the current directory.

---

### Issue Commands

#### `skis issue create`

Create a new issue.

```
skis issue create [OPTIONS]

Options:
  -t, --title <TITLE>       Issue title (required, or prompted)
  -b, --body <BODY>         Issue body
  -F, --body-file <FILE>    Read body from file (- for stdin)
  -T, --type <TYPE>         Issue type: epic, task, bug, request (default: task)
  -l, --label <LABEL>       Add label(s), can be repeated
  -e, --editor              Open $EDITOR to write title and body

Notes:
  - Labels must exist before use. If a label doesn't exist, the command errors
    with a suggestion: "Label 'foo' not found. Create it with: skis label create foo"

Examples:
  skis issue create --title "Fix login bug" --body "Users cannot log in" --type bug
  skis issue create --title "Add feature" --label enhancement --type request
  skis issue create -e
```

#### `skis issue list`

List issues.

```
skis issue list [OPTIONS]

Aliases: skis issue ls

Options:
  -s, --state <STATE>       Filter by state: open, closed, all (default: open)
  -T, --type <TYPE>         Filter by type: epic, task, bug, request
  -l, --label <LABEL>       Filter by label, can be repeated (AND logic)
  -S, --search <QUERY>      Full-text search in title and body
      --sort <FIELD>        Sort by: updated, created, id (default: updated)
      --order <DIR>         Sort direction: asc, desc (default: desc)
  -L, --limit <N>           Maximum issues to show (default: 30)
      --offset <N>          Skip first N issues (for pagination)
      --deleted             Include soft-deleted issues in results
      --json                Output as JSON

Notes:
  - Multiple --label flags use AND logic (issue must have all specified labels)
  - Default sort is by updated_at descending (most recently updated first)
  - Deleted issues are excluded by default; use --deleted to include them

Examples:
  skis issue list
  skis issue list --state all
  skis issue list --type bug
  skis issue list --label bug --label priority:high
  skis issue list --search "login error"
  skis issue list --sort created --order asc
  skis issue list --limit 10 --offset 10
  skis issue list --deleted
```

#### `skis issue view`

View an issue's details.

```
skis issue view <NUMBER> [OPTIONS]

Options:
  -c, --comments            Include comments
      --json                Output as JSON

Examples:
  skis issue view 42
  skis issue view 42 --comments
  skis issue view 42 --json
```

#### `skis issue edit`

Edit an issue.

```
skis issue edit <NUMBER> [OPTIONS]

Options:
  -t, --title <TITLE>           Set new title
  -b, --body <BODY>             Set new body
  -F, --body-file <FILE>        Read body from file
  -T, --type <TYPE>             Change issue type
      --add-label <LABEL>       Add label(s)
      --remove-label <LABEL>    Remove label(s)
  -e, --editor                  Open in $EDITOR

Examples:
  skis issue edit 42 --title "Updated title"
  skis issue edit 42 --type bug
  skis issue edit 42 --add-label bug --remove-label question
  skis issue edit 42 -e
```

#### `skis issue close`

Close an issue.

```
skis issue close <NUMBER> [OPTIONS]

Options:
  -r, --reason <REASON>     Reason: completed, not_planned (default: completed)
  -c, --comment <BODY>      Add a closing comment

Examples:
  skis issue close 42
  skis issue close 42 --reason not_planned --comment "Won't implement"
```

#### `skis issue reopen`

Reopen a closed issue.

```
skis issue reopen <NUMBER>

Examples:
  skis issue reopen 42
```

#### `skis issue delete`

Soft-delete an issue (sets `deleted_at` timestamp).

```
skis issue delete <NUMBER> [OPTIONS]

Options:
      --yes                 Skip confirmation prompt

Examples:
  skis issue delete 42
  skis issue delete 42 --yes
```

#### `skis issue restore`

Restore a soft-deleted issue.

```
skis issue restore <NUMBER>

Examples:
  skis issue restore 42
```

#### `skis issue comment`

Add a comment to an issue.

```
skis issue comment <NUMBER> [OPTIONS]

Options:
  -b, --body <BODY>         Comment body (required, or prompted)
  -F, --body-file <FILE>    Read body from file
  -e, --editor              Open $EDITOR

Examples:
  skis issue comment 42 --body "Looking into this"
  skis issue comment 42 -e
```

#### `skis issue link`

Create a bidirectional link between two issues.

```
skis issue link <ISSUE_A> <ISSUE_B>

Creates a link visible from both issues. Order doesn't matter;
`link 42 10` and `link 10 42` are equivalent.

Examples:
  skis issue link 42 10    # Links issues 42 and 10
  skis issue link 42 43    # Links issues 42 and 43
```

#### `skis issue unlink`

Remove a link between issues.

```
skis issue unlink <ISSUE_A> <ISSUE_B>

Order doesn't matter; `unlink 42 10` and `unlink 10 42` are equivalent.

Examples:
  skis issue unlink 42 10
  skis issue unlink 42 43
```

---

### Label Commands

#### `skis label list`

List all labels.

```
skis label list [OPTIONS]

Options:
      --json                Output as JSON
```

#### `skis label create`

Create a new label.

```
skis label create <NAME> [OPTIONS]

Options:
  -d, --description <DESC>  Label description
  -c, --color <HEX>         Color in hex (e.g., ff0000)

Examples:
  skis label create bug --color d73a4a --description "Something isn't working"
  skis label create enhancement --color a2eeef
```

#### `skis label delete`

Delete a label.

```
skis label delete <NAME> [OPTIONS]

Options:
      --yes                 Skip confirmation
```

---

## Library API

The `ski` crate exposes a library for programmatic use:

### Core Types

```rust
pub struct Issue {
    pub id: i64,
    pub title: String,
    pub body: Option<String>,
    pub issue_type: IssueType,
    pub state: IssueState,
    pub state_reason: Option<StateReason>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

pub enum IssueType {
    Epic,
    Task,
    Bug,
    Request,
}

pub enum IssueState {
    Open,
    Closed,
}

pub enum StateReason {
    Completed,
    NotPlanned,
}

pub struct IssueLink {
    pub issue_a_id: i64,
    pub issue_b_id: i64,
    pub created_at: DateTime<Utc>,
}

pub struct Label {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
}

pub struct Comment {
    pub id: i64,
    pub issue_id: i64,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### Database Handle

```rust
pub struct SkisDb {
    conn: rusqlite::Connection,
}

impl SkisDb {
    /// Open database, searching up from cwd for .skis/ directory
    pub fn open() -> Result<Self>;

    /// Open database at specific path
    pub fn open_at(path: &Path) -> Result<Self>;

    /// Initialize new database
    pub fn init(path: &Path) -> Result<Self>;
}
```

### Issue Operations

```rust
impl SkisDb {
    // Create - transactional, handles labels in one call
    pub fn create_issue(&self, create: &IssueCreate) -> Result<Issue>;

    // Read
    pub fn get_issue(&self, id: i64) -> Result<Option<Issue>>;
    pub fn list_issues(&self, filter: &IssueFilter) -> Result<Vec<Issue>>;
    pub fn search_issues(&self, query: &str, filter: &IssueFilter) -> Result<Vec<Issue>>;

    // Update
    pub fn update_issue(&self, id: i64, update: &IssueUpdate) -> Result<Issue>;
    pub fn close_issue(&self, id: i64, reason: StateReason) -> Result<Issue>;
    pub fn reopen_issue(&self, id: i64) -> Result<Issue>;

    // Soft delete / restore
    pub fn delete_issue(&self, id: i64) -> Result<()>;   // sets deleted_at
    pub fn restore_issue(&self, id: i64) -> Result<Issue>; // clears deleted_at

    // Labels
    pub fn add_label_to_issue(&self, issue_id: i64, label: &str) -> Result<()>;
    pub fn remove_label_from_issue(&self, issue_id: i64, label: &str) -> Result<()>;
    pub fn get_issue_labels(&self, issue_id: i64) -> Result<Vec<Label>>;

    // Comments
    pub fn add_comment(&self, issue_id: i64, body: &str) -> Result<Comment>;
    pub fn get_comments(&self, issue_id: i64) -> Result<Vec<Comment>>;

    // Links (bidirectional)
    pub fn add_link(&self, issue_a: i64, issue_b: i64) -> Result<IssueLink>;  // order doesn't matter
    pub fn remove_link(&self, issue_a: i64, issue_b: i64) -> Result<()>;      // order doesn't matter
    pub fn get_linked_issues(&self, issue_id: i64) -> Result<Vec<i64>>;       // all linked issue IDs
}

pub struct IssueCreate {
    pub title: String,
    pub body: Option<String>,
    pub issue_type: IssueType,        // default: Task
    pub labels: Vec<String>,          // must all exist, or error
}

pub struct IssueFilter {
    pub state: Option<IssueState>,
    pub issue_type: Option<IssueType>,
    pub labels: Vec<String>,          // AND logic
    pub include_deleted: bool,        // default: false
    pub sort_by: SortField,           // default: Updated
    pub sort_order: SortOrder,        // default: Desc
    pub limit: usize,                 // default: 30
    pub offset: usize,                // default: 0
}

pub enum SortField { Updated, Created, Id }
pub enum SortOrder { Asc, Desc }

pub struct IssueUpdate {
    pub title: Option<String>,
    pub body: Option<String>,
    pub issue_type: Option<IssueType>,
}
```

### Label Operations

```rust
impl SkisDb {
    pub fn create_label(&self, name: &str, desc: Option<&str>, color: Option<&str>) -> Result<Label>;
    pub fn list_labels(&self) -> Result<Vec<Label>>;
    pub fn delete_label(&self, name: &str) -> Result<()>;
}
```

---

## Output Formats

### List Output (Default)

```
#   TYPE    TITLE                          STATE   LABELS              UPDATED
42  bug     Fix login bug                  open    priority:high       2 hours ago
41  request Add dark mode                  open    enhancement         1 day ago
40  task    Update documentation           closed  docs                3 days ago
10  epic    User Authentication            open                        1 week ago
```

### View Output (Default)

```
Fix login bug #42
bug • open • opened 2 hours ago

  Users cannot log in when using special characters in their password.

  Steps to reproduce:
  1. Create account with password containing '&'
  2. Log out
  3. Try to log in

Labels:  priority:high
Links:   #10 User Authentication, #43 Deploy to production

──────────────────────────────────────────────────────────────────────────────
COMMENTS (2)

• 1 hour ago
  Looking into this now.

• 30 minutes ago
  Found the issue - password not being URL-encoded. Fix incoming.
```

### JSON Output

When `--json` is specified, output full structured JSON:

```json
{
  "id": 42,
  "title": "Fix login bug",
  "body": "Users cannot log in...",
  "type": "bug",
  "state": "open",
  "state_reason": null,
  "labels": [
    {"name": "priority:high", "color": "ff0000"}
  ],
  "linked_issues": [
    {"id": 10, "title": "User Authentication"},
    {"id": 43, "title": "Deploy to production"}
  ],
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T12:30:00Z",
  "closed_at": null,
  "deleted_at": null
}
```

---

## Future Considerations

These are explicitly out of scope for the initial version but noted for potential future work:

1. **Sync/Export**: Export to/import from JSON for backup or sharing
2. **Git integration**: Auto-reference issues in commits, close issues via commit message
3. **Templates**: Issue templates for common types (bug report, feature request)
4. **Priorities**: Built-in priority field (instead of relying on labels)
5. **Assignees**: If multi-user support is ever needed
6. **Due dates on issues**: Per-issue deadlines
7. **Time tracking**: Log time spent on issues
8. **Web UI**: Optional local web interface for browsing issues
9. **Milestones**: Grouping issues into milestones with due dates

---

## Implementation Phases

### Phase 1: Core Foundation
- Project structure and Cargo.toml setup
- Database connection and schema initialization
- Basic Issue CRUD (create, list, view, close, reopen, delete)

### Phase 2: Full Issue Support
- Issue editing
- Comments
- Full-text search

### Phase 3: Labels
- Label CRUD
- Label filtering in issue list
- Adding/removing labels from issues

### Phase 4: Polish
- Colored terminal output
- Editor integration ($EDITOR)
- JSON output format
- Error handling and user-friendly messages
