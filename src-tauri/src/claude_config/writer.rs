//! Stub — Task 2 of Plan 4 implements this module.

use std::path::Path;

#[derive(Debug)]
pub enum WriteError {}

pub fn merge_blikplan_entry(_path: &Path, _bin: &Path, _db: &Path) -> Result<(), WriteError> {
    Ok(())
}

pub fn remove_blikplan_entry(_path: &Path) -> Result<(), WriteError> {
    Ok(())
}
