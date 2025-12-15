# SKIS - Stefan's Keep-It-Simple Issue System

A local, git-friendly issue tracker that stores everything in a SQLite database. No server, no cloud, no complexity.

## Features

- **Local-first**: All data stored in `.skis/issues.db`
- **Git-friendly**: Check in the database with your repo
- **Fast**: SQLite with full-text search
- **Simple CLI**: Intuitive commands inspired by GitHub CLI
- **Colored output**: Issue types, states, and labels are color-coded
- **Human-readable timestamps**: "2 hours ago" instead of raw dates
- **JSON output**: Machine-readable format for scripting

## Installation

### Pre-built binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/xpqz/skis/releases).

**macOS (Apple Silicon)**
```bash
curl -LO https://github.com/xpqz/skis/releases/latest/download/skis-macos-aarch64.tar.gz
tar xzf skis-macos-aarch64.tar.gz
sudo mv skis /usr/local/bin/
```

**macOS (Intel)**
```bash
curl -LO https://github.com/xpqz/skis/releases/latest/download/skis-macos-x86_64.tar.gz
tar xzf skis-macos-x86_64.tar.gz
sudo mv skis /usr/local/bin/
```

**Linux (x86_64)**
```bash
curl -LO https://github.com/xpqz/skis/releases/latest/download/skis-linux-x86_64.tar.gz
tar xzf skis-linux-x86_64.tar.gz
sudo mv skis /usr/local/bin/
```

**Windows (x86_64)**

Download `skis-windows-x86_64.zip` from the releases page, extract, and add to your PATH.

### Build from source

```bash
cargo build --release
cp target/release/skis /usr/local/bin/
# or
cargo install --path .
```

## Quick Start

```bash
# Initialize in your project
cd my-project
skis init

# Create some labels
skis label create bug
skis label create feature
skis label create urgent --color ff0000

# Create an issue
skis issue create -t "Fix login timeout" -T bug -l bug

# List open issues
skis issue list

# View an issue
skis issue view 1

# Close it
skis issue close 1 -c "Fixed in commit abc123"
```

## Commands

### Repository

```bash
skis init              # Initialize SKIS in current directory
```

Creates a `.skis/` directory with the SQLite database. Run this once per project.

### Issues

#### Create

```bash
skis issue create -t "Title" [options]
```

| Option | Description |
|--------|-------------|
| `-t, --title` | Issue title (required) |
| `-b, --body` | Issue description |
| `-F, --body-file` | Read body from file (`-` for stdin) |
| `-e, --editor` | Open $EDITOR to write body |
| `-T, --type` | `epic`, `task` (default), `bug`, `request` |
| `-l, --label` | Add label (repeatable) |

Examples:
```bash
skis issue create -t "Add dark mode" -T feature -l feature
skis issue create -t "Fix crash" -T bug -b "App crashes on startup"
skis issue create -t "Q1 Roadmap" -T epic --editor
cat spec.md | skis issue create -t "Implement spec" -F -
```

#### List

```bash
skis issue list [options]
skis issue ls [options]    # alias
```

| Option | Description |
|--------|-------------|
| `-s, --state` | `open` (default), `closed`, `all` |
| `-T, --type` | Filter by type |
| `-l, --label` | Filter by label (repeatable, AND logic) |
| `--search` | Full-text search in title and body |
| `--sort` | `updated` (default), `created`, `id` |
| `--order` | `desc` (default), `asc` |
| `-L, --limit` | Max results (default 30) |
| `--offset` | Skip N issues (pagination) |
| `--deleted` | Include soft-deleted issues |
| `--json` | Output as JSON |

Examples:
```bash
skis issue list                      # Open issues
skis issue list -s all               # All issues
skis issue list -T bug               # Only bugs
skis issue list -l urgent -l bug     # Has both labels
skis issue list --search "login"     # Search
skis issue list --json | jq '.[].title'
```

#### View

```bash
skis issue view <number> [options]
```

| Option | Description |
|--------|-------------|
| `--comments` | Include comments |
| `--json` | Output as JSON (includes labels, links) |

#### Edit

```bash
skis issue edit <number> [options]
```

| Option | Description |
|--------|-------------|
| `-t, --title` | New title |
| `-b, --body` | New body |
| `-F, --body-file` | Read body from file |
| `-e, --editor` | Open $EDITOR |
| `-T, --type` | Change type |
| `--add-label` | Add label (repeatable) |
| `--remove-label` | Remove label (repeatable) |

Examples:
```bash
skis issue edit 1 -t "New title"
skis issue edit 1 --add-label urgent --remove-label low-priority
skis issue edit 1 --editor
```

#### Close / Reopen

```bash
skis issue close <number> [-r <reason>] [-c <comment>]
skis issue reopen <number>
```

Reasons: `completed` (default), `not_planned`

Examples:
```bash
skis issue close 1                           # Completed
skis issue close 2 -r not_planned            # Won't fix
skis issue close 3 -c "Fixed in v1.2.0"      # With comment
skis issue reopen 1
```

#### Delete / Restore

```bash
skis issue delete <number> [--yes]
skis issue restore <number>
```

Delete is a soft-delete (sets `deleted_at`). Use `--deleted` flag in list to see deleted issues.

#### Comments

```bash
skis issue comment <number> -b "Comment text"
skis issue comment <number> -F notes.md
skis issue comment <number> --editor
```

View comments with `skis issue view <number> --comments`.

#### Link / Unlink

```bash
skis issue link <issue_a> <issue_b>
skis issue unlink <issue_a> <issue_b>
```

Links are bidirectional. Linked issues appear in `skis issue view`.

### Labels

#### Create

```bash
skis label create <name> [-d <description>] [-c <color>]
```

Color is a 6-character hex code (no `#`). If omitted, a color is automatically generated from the label name.

Examples:
```bash
skis label create bug                              # Auto-generated color
skis label create critical --color ff0000          # Red
skis label create "help wanted" -d "Good for new contributors"
```

#### List

```bash
skis label list [--json]
```

#### Delete

```bash
skis label delete <name> [--yes]
```

## Issue Types

| Type | Description | Color |
|------|-------------|-------|
| `epic` | Large feature or initiative | Magenta |
| `task` | General work item (default) | Blue |
| `bug` | Something broken | Red |
| `request` | Feature request | Cyan |

## Data Storage

All data is stored in `.skis/issues.db`, a SQLite database. You can:

- **Check it into git**: The database is portable
- **Query directly**: `sqlite3 .skis/issues.db "SELECT * FROM issues"`
- **Back it up**: Just copy the file

## JSON Output

Use `--json` for machine-readable output:

```bash
# List issues as JSON
skis issue list --json

# View single issue with full details
skis issue view 1 --json

# Labels
skis label list --json
```

JSON output includes:
- Full issue details with `type` field
- Labels with name, color, and description
- Linked issues with id and title
- All timestamps in ISO 8601 format

## Environment Variables

| Variable | Description |
|----------|-------------|
| `EDITOR` | Editor for `--editor` flag (default: `vi`) |
| `NO_COLOR` | Disable colored output |

## Claude Code Integration

SKIS includes a Claude Code skill for AI-assisted issue management. After installing SKIS in a project, Claude Code can:

- Create and manage issues
- Search and filter issues
- Add comments and labels
- Close and link issues

Just ask naturally: "Create a bug for the login issue" or "Show me all open tasks".

## License

MIT
