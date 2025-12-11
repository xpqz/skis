# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SKIS (Stefan's Issue System) is a local-first CLI issue tracker backed by SQLite, written in Rust. The CLI (`skis`) is modeled after GitHub's `gh issue` commands.

## Build & Run Commands

```bash
cargo build              # Build the project
cargo run                # Run the CLI
cargo test               # Run all tests
cargo test <test_name>   # Run a single test
cargo clippy             # Run linter
cargo fmt                # Format code
```

## Architecture

The planned crate structure (from PLAN.md):

```
src/
├── main.rs           # CLI entry point, argument parsing (clap)
├── lib.rs            # Library root, re-exports
├── db/
│   ├── connection.rs # Database connection, .skis/ directory discovery
│   ├── migrations.rs # Schema migrations
│   └── queries.rs    # Prepared statements / query helpers
├── models/
│   ├── issue.rs      # Issue, IssueType, IssueState, StateReason
│   ├── label.rs      # Label struct
│   └── comment.rs    # Comment struct
├── commands/
│   ├── issue.rs      # issue subcommands (create, list, view, close, reopen, etc.)
│   └── label.rs      # label subcommands
└── output/
    └── format.rs     # Output formatting (table, JSON)
```

## Key Design Decisions

- **Repository discovery**: `.skis/` directory is found by walking up from cwd. Commands other than `init` error if not found.
- **Soft deletes**: Issues have `deleted_at` field; never hard-deleted. Use `--deleted` flag to include in listings.
- **Issue links**: Bidirectional, stored once with canonical ordering (`issue_a_id < issue_b_id`).
- **Label names**: Case-insensitive via `COLLATE NOCASE`. Must be created before use.
- **Label colors**: 6-char hex without `#` prefix (e.g., `ff0000`).
- **State integrity**: `state_reason` and `closed_at` only set when `state='closed'`. Reopening clears both.
- **List defaults**: Sort by `updated_at DESC`, limit 30. Multiple `--label` flags use AND logic.

## Planned Dependencies

- `clap` - CLI argument parsing with derive macros
- `rusqlite` - SQLite bindings
- `serde` / `serde_json` - JSON serialization
- `chrono` - Date/time handling
- `colored` - Terminal colors

## Database

SQLite database at `.skis/issues.db` with tables: `issues`, `labels`, `issue_labels`, `comments`, `issue_links`, and `issues_fts` (full-text search).

See PLAN.md for complete schema and CLI interface documentation.
