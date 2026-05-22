Blik Plan v1.6.0-beta — Claude connector

NEW
- **Connect to Claude (beta)**. Settings → bottom of the popover. One-click button merges a `blikplan` MCP server entry into Claude Code (`~/.claude.json`) and Claude Desktop (`~/Library/Application Support/Claude/claude_desktop_config.json`). Backups of the previous config are written to `<path>.bak-<timestamp>` before any change. Disconnect button reverses the merge cleanly, leaving sibling MCP servers untouched.
- **Inbox panel**. Envelope icon in the bottom toolbar with a live badge showing the number of pending proposals. Claude reads your schedule and proposes patches (add task, shift task, add dependency, add chaser, append note); each proposal renders a per-op diff that you accept or reject. Polls every 5 seconds while the window is open.
- **Patch apply engine** with full SQLite transaction rollback. If any op in a multi-op patch fails, the entire patch rolls back and the row is marked `apply_failed` with the error attached — your schedule never ends up in a half-applied state.
- **Auto-expiry of stale proposals**. Patches sitting in `proposed` state for more than 30 days are marked `expired` on app launch, so the Inbox stays clean.

USE IT
- Install Claude Code (`npm i -g @anthropic-ai/claude-code` or the official installer) or Claude Desktop.
- Open Blik Plan → Settings cog → scroll to "Connect to Claude (beta)" → Connect.
- Restart Claude Desktop and start a new Claude Code session.
- Try: *"Use the blikplan MCP server to list my jobs."*  Then paste a meeting transcript and ask Claude to propose a patch. Watch the envelope badge tick over within 5 seconds.

UNDER THE HOOD
- New Rust workspace crate `gb-patches` (shared `Patch` / `PatchOp` / `TaskRef` schema + structural validator with 10 typed error variants).
- New Rust binary `blikplan-mcp` (rmcp 0.3.2 SDK) bundled as a Tauri sidecar via `externalBin`. Eight MCP tools exposed: `list_jobs`, `get_job`, `list_tasks`, `get_task`, `list_contacts`, `search`, `today`, `propose_patch`.
- DB schema bumped to v7: new `pending_patches` table holds the proposal queue and tracks status transitions (`proposed → accepted → applied | apply_failed | rejected | expired`).
- Six new Tauri IPC commands: `list_pending_patches`, `get_pending_patch`, `accept_patch`, `reject_patch`, `clear_resolved_patches`, `expire_stale_patches`. Plus three for the connector itself: `detect_claude_surfaces`, `connect_to_claude`, `disconnect_from_claude`.
- 135+ unit / integration tests across the workspace (was 104 in v1.4.0).
- Apply engine reuses the existing `apply_ripple` from drag-task logic (extracted into a transaction-safe `pub` helper), so a shifted task ripples the same way Claude's proposed shift does — no second source of truth.

BETA NOTES
- This is the first build that lets an external agent write to your project DB through a structured, user-confirmed apply path. Backups are written before every connector config change, and the apply engine is transactional, but please report anything unexpected.
- The "Connect to Claude" button writes the path to the bundled MCP binary into Claude's config. If you move the app, click Disconnect → Connect to refresh the path.
