//! Shared patch schema used by `ganttbok_lib` (the Tauri app) and
//! `blikplan-mcp` (the MCP server).
//! Source of truth for the v1 patch document format.

pub mod schema;
pub mod validate;

pub use schema::{Patch, PatchOp, TaskRef, PATCH_VERSION};
pub use validate::{validate_patch, ValidationError};
