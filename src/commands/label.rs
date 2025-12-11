use ski::db::{self, SkisDb};
use ski::error::Result;

use crate::{LabelCreateArgs, LabelDeleteArgs, LabelListArgs};

pub fn list(args: LabelListArgs) -> Result<()> {
    let db = SkisDb::open()?;
    let labels = db::list_labels(db.conn())?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&labels)?);
    } else if labels.is_empty() {
        println!("No labels found");
    } else {
        println!("{:<20} {:<10} {}", "NAME", "COLOR", "DESCRIPTION");
        println!("{}", "-".repeat(60));
        for label in labels {
            println!(
                "{:<20} {:<10} {}",
                label.name,
                label.color.as_deref().unwrap_or("-"),
                label.description.as_deref().unwrap_or("")
            );
        }
    }

    Ok(())
}

pub fn create(args: LabelCreateArgs) -> Result<()> {
    let db = SkisDb::open()?;
    let label = db::create_label(
        db.conn(),
        &args.name,
        args.description.as_deref(),
        args.color.as_deref(),
    )?;
    println!("Created label '{}'", label.name);
    Ok(())
}

pub fn delete(args: LabelDeleteArgs) -> Result<()> {
    if !args.yes {
        eprint!("Delete label '{}'? [y/N] ", args.name);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    let db = SkisDb::open()?;
    db::delete_label(db.conn(), &args.name)?;
    println!("Deleted label '{}'", args.name);
    Ok(())
}
