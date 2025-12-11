use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

mod commands;

/// SKIS - Stefan's Keep-It-Simple Issue System
#[derive(Parser)]
#[command(name = "skis")]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new SKIS repository
    Init,
    /// Manage issues
    #[command(subcommand)]
    Issue(IssueCommands),
    /// Manage labels
    #[command(subcommand)]
    Label(LabelCommands),
}

#[derive(Subcommand)]
enum IssueCommands {
    /// Create a new issue
    Create(IssueCreateArgs),
    /// List issues
    #[command(alias = "ls")]
    List(IssueListArgs),
    /// View an issue
    View(IssueViewArgs),
    /// Edit an issue
    Edit(IssueEditArgs),
    /// Close an issue
    Close(IssueCloseArgs),
    /// Reopen a closed issue
    Reopen(IssueReopenArgs),
    /// Soft-delete an issue
    Delete(IssueDeleteArgs),
    /// Restore a soft-deleted issue
    Restore(IssueRestoreArgs),
    /// Add a comment to an issue
    Comment(IssueCommentArgs),
    /// Link two issues
    Link(IssueLinkArgs),
    /// Unlink two issues
    Unlink(IssueUnlinkArgs),
}

#[derive(Args)]
struct IssueCreateArgs {
    /// Issue title
    #[arg(short, long)]
    title: Option<String>,

    /// Issue body
    #[arg(short, long)]
    body: Option<String>,

    /// Read body from file (- for stdin)
    #[arg(short = 'F', long = "body-file")]
    body_file: Option<String>,

    /// Issue type: epic, task, bug, request
    #[arg(short = 'T', long = "type", default_value = "task")]
    issue_type: String,

    /// Add label(s), can be repeated
    #[arg(short, long = "label", action = clap::ArgAction::Append)]
    labels: Vec<String>,

    /// Open $EDITOR to write title and body
    #[arg(short, long)]
    editor: bool,
}

#[derive(Args)]
struct IssueListArgs {
    /// Filter by state: open, closed, all
    #[arg(short, long, default_value = "open")]
    state: String,

    /// Filter by type: epic, task, bug, request
    #[arg(short = 'T', long = "type")]
    issue_type: Option<String>,

    /// Filter by label, can be repeated (AND logic)
    #[arg(short, long = "label", action = clap::ArgAction::Append)]
    labels: Vec<String>,

    /// Full-text search in title and body
    #[arg(short = 'S', long)]
    search: Option<String>,

    /// Sort by: updated, created, id
    #[arg(long, default_value = "updated")]
    sort: String,

    /// Sort direction: asc, desc
    #[arg(long, default_value = "desc")]
    order: String,

    /// Maximum issues to show
    #[arg(short = 'L', long, default_value = "30")]
    limit: usize,

    /// Skip first N issues (for pagination)
    #[arg(long, default_value = "0")]
    offset: usize,

    /// Include soft-deleted issues
    #[arg(long)]
    deleted: bool,

    /// Output as JSON
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct IssueViewArgs {
    /// Issue number
    number: i64,

    /// Include comments
    #[arg(short, long)]
    comments: bool,

    /// Output as JSON
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct IssueEditArgs {
    /// Issue number
    number: i64,

    /// Set new title
    #[arg(short, long)]
    title: Option<String>,

    /// Set new body
    #[arg(short, long)]
    body: Option<String>,

    /// Read body from file
    #[arg(short = 'F', long = "body-file")]
    body_file: Option<String>,

    /// Change issue type
    #[arg(short = 'T', long = "type")]
    issue_type: Option<String>,

    /// Add label(s)
    #[arg(long = "add-label", action = clap::ArgAction::Append)]
    add_labels: Vec<String>,

    /// Remove label(s)
    #[arg(long = "remove-label", action = clap::ArgAction::Append)]
    remove_labels: Vec<String>,

    /// Open in $EDITOR
    #[arg(short, long)]
    editor: bool,
}

#[derive(Args)]
struct IssueCloseArgs {
    /// Issue number
    number: i64,

    /// Reason: completed, not_planned
    #[arg(short, long, default_value = "completed")]
    reason: String,

    /// Add a closing comment
    #[arg(short, long)]
    comment: Option<String>,
}

#[derive(Args)]
struct IssueReopenArgs {
    /// Issue number
    number: i64,
}

#[derive(Args)]
struct IssueDeleteArgs {
    /// Issue number
    number: i64,

    /// Skip confirmation prompt
    #[arg(long)]
    yes: bool,
}

#[derive(Args)]
struct IssueRestoreArgs {
    /// Issue number
    number: i64,
}

#[derive(Args)]
struct IssueCommentArgs {
    /// Issue number
    number: i64,

    /// Comment body
    #[arg(short, long)]
    body: Option<String>,

    /// Read body from file
    #[arg(short = 'F', long = "body-file")]
    body_file: Option<String>,

    /// Open in $EDITOR
    #[arg(short, long)]
    editor: bool,
}

#[derive(Args)]
struct IssueLinkArgs {
    /// First issue number
    issue_a: i64,

    /// Second issue number
    issue_b: i64,
}

#[derive(Args)]
struct IssueUnlinkArgs {
    /// First issue number
    issue_a: i64,

    /// Second issue number
    issue_b: i64,
}

#[derive(Subcommand)]
enum LabelCommands {
    /// List all labels
    #[command(alias = "ls")]
    List(LabelListArgs),
    /// Create a new label
    Create(LabelCreateArgs),
    /// Delete a label
    Delete(LabelDeleteArgs),
}

#[derive(Args)]
struct LabelListArgs {
    /// Output as JSON
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct LabelCreateArgs {
    /// Label name
    name: String,

    /// Label description
    #[arg(short, long)]
    description: Option<String>,

    /// Color in hex (e.g., ff0000)
    #[arg(short, long)]
    color: Option<String>,
}

#[derive(Args)]
struct LabelDeleteArgs {
    /// Label name
    name: String,

    /// Skip confirmation
    #[arg(long)]
    yes: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init => commands::init::run(),
        Commands::Issue(cmd) => match cmd {
            IssueCommands::Create(args) => commands::issue::create(args),
            IssueCommands::List(args) => commands::issue::list(args),
            IssueCommands::View(args) => commands::issue::view(args),
            IssueCommands::Edit(args) => commands::issue::edit(args),
            IssueCommands::Close(args) => commands::issue::close(args),
            IssueCommands::Reopen(args) => commands::issue::reopen(args),
            IssueCommands::Delete(args) => commands::issue::delete(args),
            IssueCommands::Restore(args) => commands::issue::restore(args),
            IssueCommands::Comment(args) => commands::issue::comment(args),
            IssueCommands::Link(args) => commands::issue::link(args),
            IssueCommands::Unlink(args) => commands::issue::unlink(args),
        },
        Commands::Label(cmd) => match cmd {
            LabelCommands::List(args) => commands::label::list(args),
            LabelCommands::Create(args) => commands::label::create(args),
            LabelCommands::Delete(args) => commands::label::delete(args),
        },
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::FAILURE
        }
    }
}
