use ski::error::Result;

use crate::{LabelCreateArgs, LabelDeleteArgs, LabelListArgs};

pub fn list(_args: LabelListArgs) -> Result<()> {
    eprintln!("label list: not yet implemented");
    Ok(())
}

pub fn create(_args: LabelCreateArgs) -> Result<()> {
    eprintln!("label create: not yet implemented");
    Ok(())
}

pub fn delete(_args: LabelDeleteArgs) -> Result<()> {
    eprintln!("label delete: not yet implemented");
    Ok(())
}
