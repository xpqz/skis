use ski::error::{Error, Result};

use crate::{LabelCreateArgs, LabelDeleteArgs, LabelListArgs};

pub fn list(_args: LabelListArgs) -> Result<()> {
    Err(Error::NotImplemented("label list".to_string()))
}

pub fn create(_args: LabelCreateArgs) -> Result<()> {
    Err(Error::NotImplemented("label create".to_string()))
}

pub fn delete(_args: LabelDeleteArgs) -> Result<()> {
    Err(Error::NotImplemented("label delete".to_string()))
}
