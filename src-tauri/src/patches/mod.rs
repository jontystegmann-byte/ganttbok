//! Re-exports the shared patch types from the `gb-patches` workspace crate.
//! All callsites inside `ganttbok_lib` that use `crate::patches::*` continue
//! to work without modification.
//!
//! `apply` is the in-app Inbox apply engine — it lives here (not in
//! `gb-patches`) because it depends on the database and repo layer.
pub mod apply;

pub use gb_patches::schema;
pub use gb_patches::validate;
pub use gb_patches::{Patch, PatchOp, TaskRef, PATCH_VERSION};
pub use gb_patches::{validate_patch, ValidationError};

pub use apply::apply_patch;
