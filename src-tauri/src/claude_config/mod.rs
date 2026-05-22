//! Detect installed Claude surfaces (Claude Code + Claude Desktop) and
//! merge the `blikplan` MCP server entry into their config files.
//! See `docs/specs/2026-05-22-blikplan-claude-connector-design.md` § "Install Flow".

pub mod paths;
pub mod writer;

pub use paths::{claude_code_config_path, claude_desktop_config_path, ClaudeSurface};
pub use writer::{merge_blikplan_entry, remove_blikplan_entry, WriteError};
