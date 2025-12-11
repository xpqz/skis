use std::str::FromStr;

use ski::db::{self, SkisDb};
use ski::error::Result;
use ski::models::{IssueCreate, IssueFilter, IssueState, IssueType, IssueUpdate, StateReason};

use crate::{
    IssueCloseArgs, IssueCommentArgs, IssueCreateArgs, IssueDeleteArgs, IssueEditArgs,
    IssueListArgs, IssueLinkArgs, IssueReopenArgs, IssueRestoreArgs, IssueUnlinkArgs,
    IssueViewArgs,
};

pub fn create(args: IssueCreateArgs) -> Result<()> {
    let title = match args.title {
        Some(t) => t,
        None => {
            eprintln!("error: --title is required");
            std::process::exit(1);
        }
    };

    let issue_type = IssueType::from_str(&args.issue_type)?;

    let db = SkisDb::open()?;
    let create = IssueCreate {
        title,
        body: args.body,
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

    let filter = IssueFilter {
        state,
        issue_type,
        labels: args.labels,
        include_deleted: args.deleted,
        limit: args.limit,
        offset: args.offset,
        ..Default::default()
    };

    let issues = db::list_issues(db.conn(), &filter)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&issues)?);
    } else if issues.is_empty() {
        println!("No issues found");
    } else {
        // Simple table output for now
        println!("{:<6} {:<8} {:<8} {}", "ID", "TYPE", "STATE", "TITLE");
        println!("{}", "-".repeat(60));
        for issue in issues {
            println!(
                "{:<6} {:<8} {:<8} {}",
                format!("#{}", issue.id),
                issue.issue_type,
                issue.state,
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
        println!("{}", serde_json::to_string_pretty(&issue)?);
    } else {
        println!("#{} {}", issue.id, issue.title);
        println!("Type: {}  State: {}", issue.issue_type, issue.state);
        if let Some(reason) = &issue.state_reason {
            println!("Closed: {}", reason);
        }
        println!("Created: {}", issue.created_at);
        println!("Updated: {}", issue.updated_at);
        if let Some(body) = &issue.body {
            println!("\n{}", body);
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

    let update = IssueUpdate {
        title: args.title,
        body: args.body,
        issue_type,
    };

    let issue = db::update_issue(db.conn(), args.number, &update)?;
    println!("Updated issue #{}", issue.id);
    Ok(())
}

pub fn close(args: IssueCloseArgs) -> Result<()> {
    let db = SkisDb::open()?;
    let reason = StateReason::from_str(&args.reason)?;
    let issue = db::close_issue(db.conn(), args.number, reason)?;
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

pub fn comment(_args: IssueCommentArgs) -> Result<()> {
    eprintln!("issue comment: not yet implemented");
    Ok(())
}

pub fn link(_args: IssueLinkArgs) -> Result<()> {
    eprintln!("issue link: not yet implemented");
    Ok(())
}

pub fn unlink(_args: IssueUnlinkArgs) -> Result<()> {
    eprintln!("issue unlink: not yet implemented");
    Ok(())
}
