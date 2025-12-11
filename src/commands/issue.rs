use std::io::Read;
use std::str::FromStr;

use ski::db::{self, SkisDb};
use ski::error::Result;
use ski::models::{
    Issue, IssueCreate, IssueFilter, IssueState, IssueType, IssueUpdate, IssueView, SortField,
    SortOrder, StateReason,
};

use crate::{
    IssueCloseArgs, IssueCommentArgs, IssueCreateArgs, IssueDeleteArgs, IssueEditArgs,
    IssueListArgs, IssueLinkArgs, IssueReopenArgs, IssueRestoreArgs, IssueUnlinkArgs,
    IssueViewArgs,
};

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

/// Resolve body from --body or --body-file options
fn resolve_body(body: Option<String>, body_file: Option<String>) -> Result<Option<String>> {
    match (body, body_file) {
        (Some(b), _) => Ok(Some(b)),
        (None, Some(path)) => Ok(Some(read_body_from_file(&path)?)),
        (None, None) => Ok(None),
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
    let body = resolve_body(args.body, args.body_file)?;

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
        // Simple table output for now
        println!("{:<6} {:<8} {:<8} {:<20} {}", "ID", "TYPE", "STATE", "LABELS", "TITLE");
        println!("{}", "-".repeat(80));
        for issue in &issues {
            let labels = db::get_issue_labels(db.conn(), issue.id)?;
            let label_str = if labels.is_empty() {
                "-".to_string()
            } else {
                labels.iter().map(|l| l.name.as_str()).collect::<Vec<_>>().join(",")
            };
            println!(
                "{:<6} {:<8} {:<8} {:<20} {}",
                format!("#{}", issue.id),
                issue.issue_type,
                issue.state,
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
            labels,
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
    println!("#{} {}", issue.id, issue.title);
    println!("Type: {}  State: {}", issue.issue_type, issue.state);
    if let Some(reason) = &issue.state_reason {
        println!("Closed: {}", reason);
    }
    println!("Created: {}", issue.created_at);
    println!("Updated: {}", issue.updated_at);

    // Show labels
    let labels = db::get_issue_labels(conn, issue.id)?;
    if !labels.is_empty() {
        let label_names: Vec<&str> = labels.iter().map(|l| l.name.as_str()).collect();
        println!("Labels: {}", label_names.join(", "));
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
                println!("[{}]", comment.created_at);
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

    let body = resolve_body(args.body, args.body_file)?;

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
    let issue = db::close_issue(db.conn(), args.number, reason)?;

    // Add comment if provided
    if let Some(comment_body) = &args.comment {
        db::add_comment(db.conn(), args.number, comment_body)?;
    }

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
    let body = resolve_body(args.body, args.body_file)?;
    let body = match body {
        Some(b) => b,
        None => {
            eprintln!("error: --body or --body-file is required");
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
