# TODO.md

Actionable implementation tasks for SKIS, organized by phase. Each task is testable in isolation.

---

## Phase 0: Project Setup ✅

### 0.1 Configure Cargo.toml with dependencies ✅

Add required dependencies to `Cargo.toml`:

```toml
[package]
name = "skis"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "skis"
path = "src/main.rs"

[lib]
name = "ski"
path = "src/lib.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1"
colored = "2"
tabled = "0.15"

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

**Acceptance**: `cargo check` succeeds. ✅

---

### 0.2 Create module structure ✅

Create the directory and file structure (matching PLAN.md architecture):

```
src/
├── main.rs           # CLI entry point, argument parsing
├── lib.rs            # Library root, re-exports
├── db/
│   ├── mod.rs
│   ├── connection.rs # Database connection management, repository discovery
│   ├── migrations.rs # Schema migrations, version tracking
│   └── queries.rs    # Prepared statements / query helpers
├── models/
│   ├── mod.rs
│   ├── issue.rs      # Issue, IssueType, IssueState, StateReason, IssueLink
│   ├── label.rs      # Label struct
│   └── comment.rs    # Comment struct
├── commands/
│   ├── mod.rs
│   ├── init.rs
│   ├── issue.rs
│   └── label.rs
├── output/
│   ├── mod.rs
│   └── format.rs     # Table formatting, JSON output, view formatting
└── error.rs
```

**Acceptance**: `cargo build` succeeds with empty module stubs. ✅

---

### 0.3 Define error types ✅

Create `src/error.rs` with a custom error enum using `thiserror`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Not a skis repository (or any parent up to /). Run 'skis init' to create one.")]
    NotARepository,

    #[error("Already initialized")]
    AlreadyInitialized,

    #[error("Issue #{0} not found")]
    IssueNotFound(i64),

    #[error("Label '{0}' not found. Create it with: skis label create {0}")]
    LabelNotFound(String),

    #[error("Issue #{0} is already {1}")]
    InvalidStateTransition(i64, String),

    #[error("Invalid color '{0}': must be 6 hex characters (e.g., ff0000)")]
    InvalidColor(String),

    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
```

**Acceptance**: Unit test that error messages match expected strings. ✅

---

### 0.4 Set up test infrastructure ✅

Create `tests/common/mod.rs` with test helpers:

```rust
pub fn test_db() -> (SkisDb, TempDir) { ... }
pub fn set_issue_timestamps(...) { ... }
```

**Acceptance**: A trivial test using `test_db()` passes. ✅

---

## Phase 1: Core Foundation ✅

### 1.1 Implement repository discovery ✅

In `src/db/connection.rs`:

- `find_skis_dir()` - Walk up from cwd looking for `.skis/` directory
- Return `Error::NotARepository` if not found

**Acceptance**:
```rust
#[test] fn finds_skis_in_current_dir()
#[test] fn finds_skis_in_parent_dir()
#[test] fn finds_skis_in_grandparent_dir()
#[test] fn errors_when_no_skis_dir()
```

---

### 1.2 Implement schema and migrations infrastructure ✅

In `src/db/migrations.rs`:

- Define `LATEST_SCHEMA_VERSION` constant (start at 1)
- Implement `run_migrations(conn)` that checks `PRAGMA user_version` and applies missing migrations
- Migration v0→v1: Create full schema (issues, labels, issue_labels, comments, issue_links, issues_fts, all triggers and indexes)

**Acceptance**:
```rust
#[test] fn fresh_db_has_latest_schema_version()
#[test] fn migration_is_idempotent()  // running twice doesn't error
```

---

### 1.3 Implement `SkisDb::init()` ✅

Create `.skis/` directory and initialize database:

- Create `.skis/` directory
- Create `issues.db` file
- Run migrations to create full schema
- Verify `PRAGMA user_version` is set correctly

Schema includes:
- `issues` table with all constraints (type CHECK, state CHECK, state integrity CHECK)
- `updated_at` trigger
- `labels` table with `COLLATE NOCASE`
- `issue_labels` junction table
- `comments` table
- `issue_links` table with canonical ordering constraint (`issue_a_id < issue_b_id`)
- `issues_fts` virtual table and sync triggers (insert, update, delete)
- All indexes from PLAN.md

**Acceptance**:
```rust
#[test] fn init_creates_skis_directory()
#[test] fn init_creates_database_file()
#[test] fn init_fails_if_already_initialized()
#[test] fn schema_has_correct_tables()
#[test] fn state_reason_requires_closed_state()  // CHECK constraint
#[test] fn closed_at_requires_closed_state()     // CHECK constraint
#[test] fn issue_type_constraint()               // CHECK constraint
#[test] fn label_name_case_insensitive_uniqueness()
#[test] fn issue_link_canonical_ordering()       // CHECK constraint
#[test] fn fresh_db_has_user_version_1()
```

---

### 1.4 Implement `SkisDb::open()` and `SkisDb::open_at()` ✅

- `open()` - Find `.skis/` via discovery, open database
- `open_at(path)` - Open database at specific path (for testing)

**Acceptance**:
```rust
#[test] fn open_succeeds_after_init()
#[test] fn open_fails_without_init()
#[test] fn open_at_specific_path()
```

---

### 1.5 Define model structs ✅

In `src/models/`:

```rust
// issue.rs
pub struct Issue { id, title, body, issue_type, state, state_reason, created_at, updated_at, closed_at, deleted_at }
pub enum IssueType { Epic, Task, Bug, Request }
pub enum IssueState { Open, Closed }
pub enum StateReason { Completed, NotPlanned }
pub struct IssueCreate { title, body, issue_type, labels }
pub struct IssueFilter { state, issue_type, labels, include_deleted, sort_by, sort_order, limit, offset }
pub struct IssueUpdate { title, body, issue_type }
pub enum SortField { Updated, Created, Id }
pub enum SortOrder { Asc, Desc }

// For bidirectional links (used in JSON output and link operations)
pub struct IssueLink { issue_a_id, issue_b_id, created_at }

// label.rs
pub struct Label { id, name, description, color }

// comment.rs
pub struct Comment { id, issue_id, body, created_at, updated_at }
```

Implement `FromStr` for enums, `Default` for filter/create structs, `Serialize` for all structs.

**Acceptance**:
```rust
#[test] fn issue_type_from_str_valid()
#[test] fn issue_type_from_str_invalid()
#[test] fn issue_type_case_insensitive()
#[test] fn issue_filter_default_values()
#[test] fn issue_serializes_to_json()
#[test] fn issue_link_serializes_to_json()
```

---

### 1.6 Implement `create_issue()` ✅

- Insert into `issues` table
- If labels provided, verify all exist (error with suggestion if not)
- Insert into `issue_labels` for each label
- Use transaction for atomicity
- Return created `Issue`

**Acceptance**:
```rust
#[test] fn create_issue_with_defaults()
#[test] fn create_issue_with_all_fields()
#[test] fn create_issue_with_labels()
#[test] fn create_issue_with_nonexistent_label_fails()
#[test] fn create_issue_error_suggests_label_create()
```

---

### 1.7 Implement `get_issue()` ✅

- Query by ID
- Return `Option<Issue>` (None if not found or deleted)

**Acceptance**:
```rust
#[test] fn get_existing_issue()
#[test] fn get_nonexistent_issue_returns_none()
#[test] fn get_deleted_issue_returns_issue()  // get_issue ignores deleted_at
```

---

### 1.8 Implement `list_issues()` ✅

- Build query based on `IssueFilter`
- Filter by state, type, labels (AND logic)
- Exclude deleted by default (`include_deleted: false`)
- Sort by `sort_by` field in `sort_order` direction
- Apply limit and offset

**Acceptance**:
```rust
#[test] fn list_all_open_issues_by_default()
#[test] fn list_filter_by_state_closed()
#[test] fn list_filter_by_state_all()
#[test] fn list_filter_by_type()
#[test] fn list_filter_by_single_label()
#[test] fn list_filter_by_multiple_labels_and_logic()
#[test] fn list_excludes_deleted_by_default()
#[test] fn list_includes_deleted_with_flag()
#[test] fn list_default_sort_updated_desc()
#[test] fn list_sort_by_created_asc()
#[test] fn list_pagination_limit()
#[test] fn list_pagination_offset()
```

---

### 1.9 Implement `close_issue()` and `reopen_issue()` ✅

- `close_issue(id, reason)` - Set state=closed, state_reason, closed_at
- `reopen_issue(id)` - Set state=open, clear state_reason and closed_at
- Error if already in target state

**Acceptance**:
```rust
#[test] fn close_issue_sets_fields()
#[test] fn close_issue_already_closed_errors()
#[test] fn reopen_issue_clears_fields()
#[test] fn reopen_issue_already_open_errors()
#[test] fn updated_at_changes_on_close()
#[test] fn updated_at_changes_on_reopen()
```

---

### 1.10 Implement `delete_issue()` and `restore_issue()` ✅

- `delete_issue(id)` - Set `deleted_at` to now
- `restore_issue(id)` - Clear `deleted_at`

**Acceptance**:
```rust
#[test] fn soft_delete_sets_deleted_at()
#[test] fn restore_clears_deleted_at()
#[test] fn delete_nonexistent_issue_errors()
```

---

### 1.11 Implement CLI skeleton with clap ✅

In `src/main.rs`, define CLI structure:

```
skis init
skis issue create|list|view|close|reopen|delete|restore
skis label create|list|delete
```

Parse arguments but don't implement handlers yet (just print "not implemented").

**Acceptance**: `skis --help` shows all commands; `skis issue --help` shows subcommands.

---

### 1.12 Implement `skis init` command ✅

- Call `SkisDb::init()` in current directory
- Print success message

**Acceptance**:
```rust
#[test] fn cli_init_creates_skis_directory()
#[test] fn cli_init_fails_if_already_initialized()
#[test] fn cli_commands_fail_without_init()  // All non-init commands error
#[test] fn cli_discovers_skis_in_parent_directory()  // Repository discovery
```

---

### 1.13 Implement `skis issue create` command ✅

Options: `-t/--title`, `-b/--body`, `-T/--type`, `-l/--label` (repeatable)

- Open database
- Call `create_issue()`
- Print "Created issue #N"

**Acceptance**:
```rust
#[test] fn cli_issue_create_with_title()
#[test] fn cli_issue_create_with_all_options()
#[test] fn cli_issue_create_with_labels()
#[test] fn cli_issue_create_nonexistent_label_shows_suggestion()
```

---

### 1.14 Implement `skis issue list` command ✅

Options: `-s/--state`, `-T/--type`, `-l/--label`, `--sort`, `--order`, `-L/--limit`, `--offset`, `--deleted`

- Open database
- Call `list_issues()` with filter
- Print table output

**Acceptance**:
```rust
#[test] fn cli_issue_list_default()
#[test] fn cli_issue_list_with_filters()
#[test] fn cli_issue_list_empty_shows_no_issues()
```

---

### 1.15 Implement `skis issue view` command ✅

Argument: `<NUMBER>`

- Open database
- Call `get_issue()`
- Print formatted issue details

**Acceptance**:
```rust
#[test] fn cli_issue_view_existing()
#[test] fn cli_issue_view_nonexistent_shows_error()
```

---

### 1.16 Implement `skis issue close` command ✅

Argument: `<NUMBER>`, Options: `-r/--reason`

**Acceptance**:
```rust
#[test] fn cli_issue_close_default_reason()
#[test] fn cli_issue_close_with_reason()
#[test] fn cli_issue_close_already_closed_shows_error()
```

---

### 1.17 Implement `skis issue reopen` command ✅

Argument: `<NUMBER>`

**Acceptance**:
```rust
#[test] fn cli_issue_reopen()
#[test] fn cli_issue_reopen_already_open_shows_error()
```

---

### 1.18 Implement `skis issue delete` command ✅

Argument: `<NUMBER>`, Options: `--yes`

- Prompt for confirmation unless `--yes`
- Call `delete_issue()`

**Acceptance**:
```rust
#[test] fn cli_issue_delete_with_yes()
#[test] fn cli_issue_delete_removes_from_list()
```

---

### 1.19 Implement `skis issue restore` command ✅

Argument: `<NUMBER>`

**Acceptance**:
```rust
#[test] fn cli_issue_restore()
#[test] fn cli_issue_restore_appears_in_list()
```

---

## Phase 2: Full Issue Support

### 2.1 Implement `update_issue()`

- Update title, body, and/or issue_type
- Only update fields that are `Some` in `IssueUpdate`

**Acceptance**:
```rust
#[test] fn update_issue_title_only()
#[test] fn update_issue_body_only()
#[test] fn update_issue_type_only()
#[test] fn update_issue_multiple_fields()
#[test] fn update_issue_triggers_updated_at()
```

---

### 2.2 Implement `skis issue edit` command

Argument: `<NUMBER>`, Options: `-t/--title`, `-b/--body`, `-T/--type`

**Acceptance**:
```rust
#[test] fn cli_issue_edit_title()
#[test] fn cli_issue_edit_type()
#[test] fn cli_issue_edit_nonexistent_shows_error()
```

---

### 2.3 Implement `add_comment()` and `get_comments()`

- `add_comment(issue_id, body)` - Insert comment, return `Comment`
- `get_comments(issue_id)` - Return all comments for issue, ordered by created_at

**Acceptance**:
```rust
#[test] fn add_comment_to_issue()
#[test] fn get_comments_returns_in_order()
#[test] fn add_comment_to_nonexistent_issue_errors()
```

---

### 2.4 Implement `skis issue comment` command

Argument: `<NUMBER>`, Options: `-b/--body`

**Acceptance**:
```rust
#[test] fn cli_issue_comment_with_body()
```

---

### 2.5 Implement `skis issue view --comments`

- Include comments in view output when `-c/--comments` flag is set

**Acceptance**:
```rust
#[test] fn cli_issue_view_with_comments()
#[test] fn cli_issue_view_without_comments_flag_hides_comments()
```

---

### 2.6 Implement `search_issues()`

- Use FTS5 `issues_fts` table
- Combine with `IssueFilter` for state/type/label filtering

**Acceptance**:
```rust
#[test] fn search_finds_title_match()
#[test] fn search_finds_body_match()
#[test] fn search_respects_state_filter()
#[test] fn search_respects_label_filter()
#[test] fn fts_stays_in_sync_after_insert()
#[test] fn fts_stays_in_sync_after_update()
#[test] fn fts_stays_in_sync_after_delete()  // Hard delete removes from FTS
```

---

### 2.7 Implement `skis issue list --search`

Option: `-S/--search <QUERY>`

**Acceptance**:
```rust
#[test] fn cli_issue_list_search()
#[test] fn cli_issue_list_search_with_filters()
```

---

### 2.8 Implement issue links: `add_link()`, `remove_link()`, `get_linked_issues()`

- `add_link(a, b)` - Store with canonical ordering (min, max)
- `remove_link(a, b)` - Remove regardless of order provided
- `get_linked_issues(id)` - Return all linked issue IDs

**Acceptance**:
```rust
#[test] fn link_is_bidirectional()
#[test] fn link_order_does_not_matter()
#[test] fn duplicate_link_fails()
#[test] fn duplicate_link_reversed_order_fails()
#[test] fn unlink_order_does_not_matter()
#[test] fn self_link_fails()
#[test] fn link_to_deleted_issue_allowed()
```

---

### 2.9 Implement `skis issue link` and `skis issue unlink` commands

Arguments: `<ISSUE_A> <ISSUE_B>`

**Acceptance**:
```rust
#[test] fn cli_issue_link()
#[test] fn cli_issue_unlink()
#[test] fn cli_issue_link_shows_in_view()
```

---

### 2.10 Show linked issues in `skis issue view`

Display linked issues in view output.

**Acceptance**:
```rust
#[test] fn cli_issue_view_shows_links()
```

---

## Phase 3: Labels

### 3.1 Implement `create_label()`

- Validate color format (6 hex chars, no #)
- Insert with case-insensitive name

**Acceptance**:
```rust
#[test] fn create_label_with_all_fields()
#[test] fn create_label_name_only()
#[test] fn create_label_invalid_color_errors()
#[test] fn create_label_duplicate_name_errors()
#[test] fn create_label_duplicate_name_different_case_errors()
```

---

### 3.2 Implement `list_labels()` and `delete_label()`

- `list_labels()` - Return all labels
- `delete_label(name)` - Delete by name (case-insensitive)

**Acceptance**:
```rust
#[test] fn list_labels_returns_all()
#[test] fn delete_label_by_name()
#[test] fn delete_label_case_insensitive()
#[test] fn delete_label_nonexistent_errors()
```

---

### 3.3 Implement `skis label create` command

Argument: `<NAME>`, Options: `-d/--description`, `-c/--color`

**Acceptance**:
```rust
#[test] fn cli_label_create()
#[test] fn cli_label_create_with_color()
#[test] fn cli_label_create_invalid_color_shows_error()
```

---

### 3.4 Implement `skis label list` command

**Acceptance**:
```rust
#[test] fn cli_label_list()
#[test] fn cli_label_list_empty()
```

---

### 3.5 Implement `skis label delete` command

Argument: `<NAME>`, Options: `--yes`

**Acceptance**:
```rust
#[test] fn cli_label_delete_with_yes()
```

---

### 3.6 Implement `add_label_to_issue()` and `remove_label_from_issue()`

- Used by `skis issue edit --add-label` and `--remove-label`

**Acceptance**:
```rust
#[test] fn add_label_to_issue()
#[test] fn add_nonexistent_label_errors()
#[test] fn add_duplicate_label_is_idempotent()
#[test] fn remove_label_from_issue()
#[test] fn remove_nonexistent_label_is_idempotent()
```

---

### 3.7 Implement `skis issue edit --add-label` and `--remove-label`

Options: `--add-label <LABEL>` (repeatable), `--remove-label <LABEL>` (repeatable)

**Acceptance**:
```rust
#[test] fn cli_issue_edit_add_label()
#[test] fn cli_issue_edit_remove_label()
#[test] fn cli_issue_edit_add_and_remove_labels()
```

---

### 3.8 Implement `get_issue_labels()`

Return labels for an issue (used in view output).

**Acceptance**:
```rust
#[test] fn get_issue_labels_returns_all()
#[test] fn get_issue_labels_empty()
```

---

### 3.9 Show labels in `skis issue view` and `skis issue list`

**Acceptance**:
```rust
#[test] fn cli_issue_view_shows_labels()
#[test] fn cli_issue_list_shows_labels()
```

---

## Phase 4: Polish

### 4.1 Implement `--json` output for `skis issue view`

Serialize issue to JSON matching schema in PLAN.md.

**Acceptance**:
```rust
#[test] fn cli_issue_view_json_valid()
#[test] fn cli_issue_view_json_includes_labels()
#[test] fn cli_issue_view_json_includes_linked_issues()
```

---

### 4.2 Implement `--json` output for `skis issue list`

Serialize issue list to JSON array.

**Acceptance**:
```rust
#[test] fn cli_issue_list_json_valid()
```

---

### 4.3 Implement `--json` output for `skis label list`

**Acceptance**:
```rust
#[test] fn cli_label_list_json_valid()
```

---

### 4.4 Implement `skis issue close --comment`

Option: `-c/--comment <BODY>` - Add comment when closing

**Acceptance**:
```rust
#[test] fn cli_issue_close_with_comment()
```

---

### 4.5 Implement `--body-file` for issue create/edit/comment

Option: `-F/--body-file <FILE>` - Read body from file (use `-` for stdin)

**Acceptance**:
```rust
#[test] fn cli_issue_create_body_from_file()
#[test] fn cli_issue_create_body_from_stdin()
#[test] fn cli_issue_edit_body_from_file()
#[test] fn cli_issue_comment_body_from_file()
```

---

### 4.6 Implement `--editor` for issue create/edit/comment

Option: `-e/--editor` - Open `$EDITOR` to write content

**Acceptance**: Manual testing (involves spawning editor).

---

### 4.7 Add colored terminal output

Use `colored` crate for:
- Issue type badges
- State indicators (green=open, red=closed)
- Label colors

**Acceptance**: Manual visual verification (colors don't affect test assertions).

---

### 4.8 Implement human-readable timestamps

Display "2 hours ago", "3 days ago" instead of raw timestamps.

**Acceptance**:
```rust
#[test] fn format_relative_time_seconds()
#[test] fn format_relative_time_minutes()
#[test] fn format_relative_time_hours()
#[test] fn format_relative_time_days()
```

---

### 4.9 Add `ls` alias for `list`

`skis issue ls` should work as alias for `skis issue list`.

**Acceptance**:
```rust
#[test] fn cli_issue_ls_alias()
```

---

### 4.10 Final integration test: full workflow

End-to-end test covering complete user journey:

```rust
#[test]
fn full_issue_lifecycle() {
    // init -> create label -> create issue with label ->
    // view -> edit -> comment -> link -> close -> reopen ->
    // delete -> restore -> list with filters -> search
}
```

---

## Dependency Graph

```
Phase 0 (Setup)
    ↓
Phase 1.1-1.4 (DB foundation: discovery, migrations, init, open)
    ↓
Phase 1.5-1.10 (Issue model & CRUD)
    ↓
Phase 1.11-1.19 (CLI commands)      Phase 3.1-3.5 (Labels - can start after 1.5)
    ↓                                     ↓
Phase 2.1-2.10 (Edit, Comments,      Phase 3.6-3.9 (Label-Issue integration)
               Search, Links)              ↓
    ↓                                     ↓
    └─────────────────┬───────────────────┘
                      ↓
              Phase 4 (Polish)
```

**Key independence points**:
- Phase 3 (Labels) can begin after model structs are defined (1.5)
- Search (2.6-2.7) is independent of comments (2.3-2.5)
- Links (2.8-2.10) are independent of search and comments
- All Phase 4 items are independent of each other
