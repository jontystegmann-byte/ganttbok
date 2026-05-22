//! Stub — Task 3 of Plan 1 implements this module.

use crate::patches::schema::Patch;

#[derive(Debug)]
pub enum ValidationError {}

pub fn validate_patch(_p: &Patch) -> Result<(), ValidationError> {
    Ok(())
}
