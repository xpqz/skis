---
name: skis
description: Manage local issues using SKIS (Stefan's Keep-It-Simple Issue System). Use this skill when the user asks to create, view, list, edit, close, or manage issues, bugs, tasks, or epics. Also use for labels, comments, and issue linking.
---

# SKIS Issue Tracker

SKIS is a local, git-friendly issue tracker stored in `.skis/issues.db`. Use this skill to manage issues, labels, and comments.

## Initialization

Before using SKIS in a new project, initialize it:

```bash
skis init
```

This creates a `.skis/` directory with the SQLite database.

## Issue Management

### Create Issues

```bash
skis issue create -t "Issue title" -b "Description body" -T <type> -l <label>
```

Options:
- `-t/--title` (required): Issue title
- `-b/--body`: Issue description
- `-F/--body-file <file>`: Read body from file (use `-` for stdin)
- `-e/--editor`: Open $EDITOR to write body
- `-T/--type`: One of `epic`, `task` (default), `bug`, `request`
- `-l/--label`: Add label (repeatable)

### List Issues

```bash
skis issue list [options]
```

Options:
- `-s/--state`: `open` (default), `closed`, or `all`
- `-T/--type`: Filter by type
- `-l/--label`: Filter by label (repeatable, AND logic)
- `--search <query>`: Full-text search
- `--sort`: `updated` (default), `created`, `id`
- `--order`: `desc` (default), `asc`
- `-L/--limit`: Max results (default 30)
- `--offset`: Skip N issues
- `--deleted`: Include soft-deleted issues
- `--json`: Output as JSON

Alias: `skis issue ls`

### View Issue

```bash
skis issue view <number> [--comments] [--json]
```

### Edit Issue

```bash
skis issue edit <number> [options]
```

Options:
- `-t/--title`: New title
- `-b/--body`: New body
- `-F/--body-file`: Read body from file
- `-e/--editor`: Open $EDITOR
- `-T/--type`: Change type
- `--add-label <name>`: Add label (repeatable)
- `--remove-label <name>`: Remove label (repeatable)

### Close/Reopen Issues

```bash
skis issue close <number> [-r <reason>] [-c <comment>]
skis issue reopen <number>
```

Reasons: `completed` (default), `not_planned`

### Delete/Restore Issues

```bash
skis issue delete <number> [--yes]
skis issue restore <number>
```

Delete is soft-delete (can be restored).

### Comments

```bash
skis issue comment <number> -b "Comment text"
skis issue comment <number> -F comment.md
skis issue comment <number> --editor
```

### Link Issues

```bash
skis issue link <issue_a> <issue_b>
skis issue unlink <issue_a> <issue_b>
```

Links are bidirectional.

## Label Management

### Create Labels

```bash
skis label create <name> [-d <description>] [-c <color>]
```

Color is 6-character hex (no #). If omitted, a color is auto-generated from the name.

### List Labels

```bash
skis label list [--json]
```

### Delete Labels

```bash
skis label delete <name> [--yes]
```

## Issue Types

- `epic`: Large feature or initiative
- `task`: General work item (default)
- `bug`: Something broken
- `request`: Feature request

## Common Workflows

### Create a bug with label

```bash
skis label create bug --description "Something is broken"
skis issue create -t "Login fails on Safari" -T bug -l bug -b "Users report..."
```

### Find and close an issue

```bash
skis issue list --search "login"
skis issue close 42 -c "Fixed in commit abc123"
```

### Review all open bugs

```bash
skis issue list -T bug -s open
```

### Link related issues

```bash
skis issue link 1 5
skis issue view 1  # Shows "Linked: #5"
```

## JSON Output

Use `--json` flag for machine-readable output:

```bash
skis issue list --json
skis issue view 1 --json
skis label list --json
```

JSON output includes full details: labels with colors, linked issues with titles, timestamps.
