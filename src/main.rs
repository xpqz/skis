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
pub struct IssueCreateArgs {
    /// Issue title (required)
    #[arg(short, long)]
    pub title: Option<String>,

    /// Issue body
    #[arg(short, long)]
    pub body: Option<String>,

    /// Issue type: epic, task, bug, request
    #[arg(short = 'T', long = "type", default_value = "task")]
    pub issue_type: String,

    /// Add label(s), can be repeated
    #[arg(short, long = "label", action = clap::ArgAction::Append)]
    pub labels: Vec<String>,
}

#[derive(Args)]
pub struct IssueListArgs {
    /// Filter by state: open, closed, all
    #[arg(short, long, default_value = "open")]
    pub state: String,

    /// Search query (full-text)
    #[arg(long)]
    pub search: Option<String>,

    /// Filter by type: epic, task, bug, request
    #[arg(short = 'T', long = "type")]
    pub issue_type: Option<String>,

    /// Filter by label, can be repeated (AND logic)
    #[arg(short, long = "label", action = clap::ArgAction::Append)]
    pub labels: Vec<String>,

    /// Sort by: updated, created, id
    #[arg(long, default_value = "updated")]
    pub sort: String,

    /// Sort direction: asc, desc
    #[arg(long, default_value = "desc")]
    pub order: String,

    /// Maximum issues to show
    #[arg(short = 'L', long, default_value = "30")]
    pub limit: usize,

    /// Skip first N issues (for pagination)
    #[arg(long, default_value = "0")]
    pub offset: usize,

    /// Include soft-deleted issues
    #[arg(long)]
    pub deleted: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct IssueViewArgs {
    /// Issue number
    pub number: i64,

    /// Include comments in output
    #[arg(long)]
    pub comments: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct IssueEditArgs {
    /// Issue number
    pub number: i64,

    /// Set new title
    #[arg(short, long)]
    pub title: Option<String>,

    /// Set new body
    #[arg(short, long)]
    pub body: Option<String>,

    /// Change issue type
    #[arg(short = 'T', long = "type")]
    pub issue_type: Option<String>,

    /// Add label(s), can be repeated
    #[arg(long = "add-label", action = clap::ArgAction::Append)]
    pub add_labels: Vec<String>,

    /// Remove label(s), can be repeated
    #[arg(long = "remove-label", action = clap::ArgAction::Append)]
    pub remove_labels: Vec<String>,
}

#[derive(Args)]
pub struct IssueCloseArgs {
    /// Issue number
    pub number: i64,

    /// Reason: completed, not_planned
    #[arg(short, long, default_value = "completed")]
    pub reason: String,
}

#[derive(Args)]
pub struct IssueReopenArgs {
    /// Issue number
    pub number: i64,
}

#[derive(Args)]
pub struct IssueDeleteArgs {
    /// Issue number
    pub number: i64,

    /// Skip confirmation prompt
    #[arg(long)]
    pub yes: bool,
}

#[derive(Args)]
pub struct IssueRestoreArgs {
    /// Issue number
    pub number: i64,
}

#[derive(Args)]
pub struct IssueCommentArgs {
    /// Issue number
    pub number: i64,

    /// Comment body (required)
    #[arg(short, long)]
    pub body: String,
}

#[derive(Args)]
pub struct IssueLinkArgs {
    /// First issue number
    pub issue_a: i64,

    /// Second issue number
    pub issue_b: i64,
}

#[derive(Args)]
pub struct IssueUnlinkArgs {
    /// First issue number
    pub issue_a: i64,

    /// Second issue number
    pub issue_b: i64,
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
pub struct LabelListArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct LabelCreateArgs {
    /// Label name
    pub name: String,

    /// Label description
    #[arg(short, long)]
    pub description: Option<String>,

    /// Color in hex (e.g., ff0000)
    #[arg(short, long)]
    pub color: Option<String>,
}

#[derive(Args)]
pub struct LabelDeleteArgs {
    /// Label name
    pub name: String,

    /// Skip confirmation
    #[arg(long)]
    pub yes: bool,
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
