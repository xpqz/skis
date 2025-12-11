use std::io::Read;
use std::str::FromStr;

use colored::Colorize;
use ski::db::{self, SkisDb};
use ski::error::Result;
use ski::models::{
    Issue, IssueCreate, IssueFilter, IssueState, IssueType, IssueUpdate, IssueView, SortField,
    SortOrder, StateReason,
};
use ski::output::format_timestamp;

use crate::{
    IssueCloseArgs, IssueCommentArgs, IssueCreateArgs, IssueDeleteArgs, IssueEditArgs,
    IssueListArgs, IssueLinkArgs, IssueReopenArgs, IssueRestoreArgs, IssueUnlinkArgs,
    IssueViewArgs,
};

/// Format issue type with color
fn format_type_colored(issue_type: IssueType) -> colored::ColoredString {
    match issue_type {
        IssueType::Epic => "epic".magenta().bold(),
        IssueType::Task => "task".blue(),
        IssueType::Bug => "bug".red(),
        IssueType::Request => "request".cyan(),
    }
}

/// Format issue state with color
fn format_state_colored(state: IssueState) -> colored::ColoredString {
    match state {
        IssueState::Open => "open".green(),
        IssueState::Closed => "closed".red(),
    }
}

/// Format a label with its color (if available)
fn format_label_colored(name: &str, color: Option<&str>) -> String {
    match color {
        Some(hex) if hex.len() == 6 => {
            // Parse hex color and apply
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                return name.truecolor(r, g, b).to_string();
            }
            name.to_string()
        }
        _ => name.yellow().to_string(),
    }
}

/// Read body content from file or stdin (if path is "-")
fn read_body_from_file(path: &str) -> Result<String> {
    if path == "-" {
        let mut content = String::new();
        std::io::stdin().read_to_string(&mut content)?;
        Ok(content)
    } else {
        Ok(std::fs::read_to_string(path)?)
    }
}

/// Open $EDITOR to get content from user
fn read_body_from_editor() -> Result<Option<String>> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    // Create a temp file
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("skis-{}.md", std::process::id()));

    // Spawn editor
    let status = std::process::Command::new(&editor)
        .arg(&temp_path)
        .status()?;

    if !status.success() {
        eprintln!("Editor exited with non-zero status");
        return Ok(None);
    }

    // Read the file if it exists
    if temp_path.exists() {
        let content = std::fs::read_to_string(&temp_path)?;
        let _ = std::fs::remove_file(&temp_path); // Clean up

        let content = content.trim().to_string();
        if content.is_empty() {
            return Ok(None);
        }
        return Ok(Some(content));
    }

    Ok(None)
}

/// Resolve body from --body, --body-file, or --editor options
fn resolve_body(
    body: Option<String>,
    body_file: Option<String>,
    editor: bool,
) -> Result<Option<String>> {
    match (body, body_file, editor) {
        (Some(b), _, _) => Ok(Some(b)),
        (None, Some(path), _) => Ok(Some(read_body_from_file(&path)?)),
        (None, None, true) => read_body_from_editor(),
        (None, None, false) => Ok(None),
    }
}

pub fn create(args: IssueCreateArgs) -> Result<()> {
    let title = match args.title {
        Some(t) => t,
        None => {
            eprintln!("error: --title is required");
            std::process::exit(1);
        }
    };

    let issue_type = IssueType::from_str(&args.issue_type)?;
    let body = resolve_body(args.body, args.body_file, args.editor)?;

    let db = SkisDb::open()?;
    let create = IssueCreate {
        title,
        body,
        issue_type,
        labels: args.labels,
    };

    let issue = db::create_issue(db.conn(), &create)?;
    println!("Created issue #{}", issue.id);
    Ok(())
}

pub fn list(args: IssueListArgs) -> Result<()> {
    let db = SkisDb::open()?;

    let state = match args.state.to_lowercase().as_str() {
        "open" => Some(IssueState::Open),
        "closed" => Some(IssueState::Closed),
        "all" => None,
        _ => {
            eprintln!("error: invalid state '{}', must be open, closed, or all", args.state);
            std::process::exit(1);
        }
    };

    let issue_type = args
        .issue_type
        .map(|t| IssueType::from_str(&t))
        .transpose()?;

    let sort_by = match args.sort.to_lowercase().as_str() {
        "updated" => SortField::Updated,
        "created" => SortField::Created,
        "id" => SortField::Id,
        _ => {
            eprintln!(
                "error: invalid sort field '{}', must be updated, created, or id",
                args.sort
            );
            std::process::exit(1);
        }
    };

    let sort_order = match args.order.to_lowercase().as_str() {
        "asc" => SortOrder::Asc,
        "desc" => SortOrder::Desc,
        _ => {
            eprintln!(
                "error: invalid sort order '{}', must be asc or desc",
                args.order
            );
            std::process::exit(1);
        }
    };

    let filter = IssueFilter {
        state,
        issue_type,
        labels: args.labels,
        include_deleted: args.deleted,
        sort_by,
        sort_order,
        limit: args.limit,
        offset: args.offset,
    };

    let issues = if let Some(query) = &args.search {
        db::search_issues(db.conn(), query, &filter)?
    } else {
        db::list_issues(db.conn(), &filter)?
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&issues)?);
    } else if issues.is_empty() {
        println!("No issues found");
    } else {
        // Simple table output with colors
        println!(
            "{:<6} {:<8} {:<8} {:<20} {}",
            "ID".bold(),
            "TYPE".bold(),
            "STATE".bold(),
            "LABELS".bold(),
            "TITLE".bold()
        );
        println!("{}", "-".repeat(80));
        for issue in &issues {
            let labels = db::get_issue_labels(db.conn(), issue.id)?;
            let label_str = if labels.is_empty() {
                "-".dimmed().to_string()
            } else {
                labels
                    .iter()
                    .map(|l| format_label_colored(&l.name, l.color.as_deref()))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            println!(
                "{:<6} {:<8} {:<8} {:<20} {}",
                format!("#{}", issue.id),
                format_type_colored(issue.issue_type),
                format_state_colored(issue.state),
                label_str,
                issue.title
            );
        }
    }

    Ok(())
}

pub fn view(args: IssueViewArgs) -> Result<()> {
    let db = SkisDb::open()?;
    let issue = db::get_issue(db.conn(), args.number)?
        .ok_or_else(|| ski::error::Error::IssueNotFound(args.number))?;

    if args.json {
        // Build enriched view with labels and linked issues
        let labels = db::get_issue_labels(db.conn(), issue.id)?;
        let linked_issues = db::get_linked_issues_with_titles(db.conn(), issue.id)?;

        let view = IssueView {
            id: issue.id,
            title: issue.title.clone(),
            body: issue.body.clone(),
            issue_type: issue.issue_type,
            state: issue.state,
            state_reason: issue.state_reason,
            labels: labels.into_iter().map(Into::into).collect(),
            linked_issues,
            created_at: issue.created_at,
            updated_at: issue.updated_at,
            closed_at: issue.closed_at,
            deleted_at: issue.deleted_at,
        };
        println!("{}", serde_json::to_string_pretty(&view)?);
    } else {
        print_issue_view(db.conn(), &issue, args.comments)?;
    }

    Ok(())
}

fn print_issue_view(
    conn: &rusqlite::Connection,
    issue: &Issue,
    show_comments: bool,
) -> Result<()> {
    println!(
        "{} {}",
        format!("#{}", issue.id).bold(),
        issue.title.bold()
    );
    println!(
        "Type: {}  State: {}",
        format_type_colored(issue.issue_type),
        format_state_colored(issue.state)
    );
    if let Some(reason) = &issue.state_reason {
        println!("Closed: {}", reason);
    }
    println!("Created: {}", format_timestamp(issue.created_at).dimmed());
    println!("Updated: {}", format_timestamp(issue.updated_at).dimmed());

    // Show labels
    let labels = db::get_issue_labels(conn, issue.id)?;
    if !labels.is_empty() {
        let label_strs: Vec<String> = labels
            .iter()
            .map(|l| format_label_colored(&l.name, l.color.as_deref()))
            .collect();
        println!("Labels: {}", label_strs.join(", "));
    }

    // Show linked issues
    let linked = db::get_linked_issues(conn, issue.id)?;
    if !linked.is_empty() {
        let linked_str: Vec<String> = linked.iter().map(|id| format!("#{}", id)).collect();
        println!("Linked: {}", linked_str.join(", "));
    }

    if let Some(body) = &issue.body {
        println!("\n{}", body);
    }

    // Show comments if requested
    if show_comments {
        let comments = db::get_comments(conn, issue.id)?;
        if !comments.is_empty() {
            println!("\nComments:");
            println!("{}", "-".repeat(40));
            for comment in comments {
                println!("[{}]", format_timestamp(comment.created_at));
                println!("{}", comment.body);
                println!();
            }
        }
    }

    Ok(())
}

pub fn edit(args: IssueEditArgs) -> Result<()> {
    let db = SkisDb::open()?;

    let issue_type = args
        .issue_type
        .map(|t| IssueType::from_str(&t))
        .transpose()?;

    let body = resolve_body(args.body, args.body_file, args.editor)?;

    let update = IssueUpdate {
        title: args.title,
        body,
        issue_type,
    };

    let issue = db::update_issue(db.conn(), args.number, &update)?;

    // Handle label additions
    for label in &args.add_labels {
        db::add_label_to_issue(db.conn(), args.number, label)?;
    }

    // Handle label removals
    for label in &args.remove_labels {
        db::remove_label_from_issue(db.conn(), args.number, label)?;
    }

    println!("Updated issue #{}", issue.id);
    Ok(())
}

pub fn close(args: IssueCloseArgs) -> Result<()> {
    let db = SkisDb::open()?;
    let reason = StateReason::from_str(&args.reason)?;
    let issue = db::close_issue_with_comment(
        db.conn(),
        args.number,
        reason,
        args.comment.as_deref(),
    )?;

    println!("Closed issue #{} as {}", issue.id, args.reason);
    Ok(())
}

pub fn reopen(args: IssueReopenArgs) -> Result<()> {
    let db = SkisDb::open()?;
    let issue = db::reopen_issue(db.conn(), args.number)?;
    println!("Reopened issue #{}", issue.id);
    Ok(())
}

pub fn delete(args: IssueDeleteArgs) -> Result<()> {
    if !args.yes {
        eprint!("Delete issue #{}? [y/N] ", args.number);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    let db = SkisDb::open()?;
    db::delete_issue(db.conn(), args.number)?;
    println!("Deleted issue #{}", args.number);
    Ok(())
}

pub fn restore(args: IssueRestoreArgs) -> Result<()> {
    let db = SkisDb::open()?;
    let issue = db::restore_issue(db.conn(), args.number)?;
    println!("Restored issue #{}", issue.id);
    Ok(())
}

pub fn comment(args: IssueCommentArgs) -> Result<()> {
    let body = resolve_body(args.body, args.body_file, args.editor)?;
    let body = match body {
        Some(b) => b,
        None => {
            eprintln!("error: --body, --body-file, or --editor is required");
            std::process::exit(1);
        }
    };

    let db = SkisDb::open()?;
    let comment = db::add_comment(db.conn(), args.number, &body)?;
    println!("Added comment #{} to issue #{}", comment.id, args.number);
    Ok(())
}

pub fn link(args: IssueLinkArgs) -> Result<()> {
    let db = SkisDb::open()?;
    db::add_link(db.conn(), args.issue_a, args.issue_b)?;
    println!("Linked issue #{} and #{}", args.issue_a, args.issue_b);
    Ok(())
}

pub fn unlink(args: IssueUnlinkArgs) -> Result<()> {
    let db = SkisDb::open()?;
    db::remove_link(db.conn(), args.issue_a, args.issue_b)?;
    println!("Unlinked issue #{} and #{}", args.issue_a, args.issue_b);
    Ok(())
}
