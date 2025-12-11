use ski::db::SkisDb;
use ski::error::Result;

pub fn run() -> Result<()> {
    let cwd = std::env::current_dir()?;
    SkisDb::init(&cwd)?;
    println!("Initialized empty SKIS repository in {}/.skis/", cwd.display());
    Ok(())
}
