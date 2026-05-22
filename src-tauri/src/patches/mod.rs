//! Shared patch schema used by external clients (the MCP server)
//! and the in-app Inbox apply engine. See
//! `docs/specs/2026-05-22-blikplan-claude-connector-design.md`.

pub mod schema;
pub mod validate;

pub use schema::{Patch, PatchOp, PATCH_VERSION};
pub use validate::{validate_patch, ValidationError};
