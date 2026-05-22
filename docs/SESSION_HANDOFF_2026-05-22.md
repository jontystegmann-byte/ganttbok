# Session Handoff — Blik Plan ↔ Claude Connector

**Date:** 2026-05-22
**Branch:** `feat/claude-connector-plan1` (name now misleading; carries Plans 1+2 implementation + all 4 plan docs)
**Status:** Plans 1 + 2 implemented and committed. Plans 3 + 4 docs ready, awaiting implementation.

## Where to resume

**Next task:** implement Plan 3 against `docs/plans/2026-05-22-blikplan-claude-connector-plan3-inbox-apply.md` (16 tasks).

After Plan 3 lands, implement Plan 4 against `docs/plans/2026-05-22-blikplan-claude-connector-plan4-install-flow.md` (8 tasks). Plan 4 depends on Plan 3.

Use the `superpowers:subagent-driven-development` skill. Dispatch a fresh `general-purpose` subagent per task with strict file-scope rules — see "Standing rules" below.

## What ships from this session

- **Plan 1 — Foundation** (4 commits, `e014971` → `80e5871`)
  - `pending_patches` SQLite table (migration v7)
  - `Patch` / `PatchOp` / `TaskRef` Rust schema (`gb-patches` crate)
  - Structural validator with 10 typed error variants
  - Matching TypeScript types in `src/lib/types.ts`
  - 17 new tests passing

- **Plan 2 — MCP server** (12 commits, `178973b` → `4cf6785`)
  - Cargo workspace introduced; `gb-patches` extracted to `crates/gb-patches/`
  - `blikplan-mcp` binary at `crates/blikplan-mcp/` — Rust, rmcp 0.3.2 SDK
  - 8 MCP tools wired and tested: `list_jobs`, `get_job`, `list_tasks`, `get_task`, `list_contacts`, `search`, `today`, `propose_patch`
  - DB path discovery: `$BLIKPLAN_DB` → "Blik Plan" → "Gantt Bok" fallback → error
  - Tauri sidecar copy script at `scripts/copy-mcp-sidecar.sh` (uses `cargo metadata` to handle the SD-card target redirect)
  - npm wrapper at `packages/blikplan-mcp-npm/` (esbuild-pattern postinstall)
  - Release binary: `/Volumes/SD EXTREME JT/ganttbok-target/target/release/blikplan-mcp` (4.4 MB)
  - 21 new tests passing in the MCP crate; 118 tests total across the workspace

## Critical context the next session needs

### rmcp 0.3.2 API quirks (Plan 2's plan was written before these were discovered)

When implementing the Tauri-app side (Plan 3), if any code touches the MCP server's Rust types, expect these deviations from the original plan doc:

- `#[tool_router]` on the impl block (not `#[tool_router(server_handler)]`)
- `#[tool_handler(router = self.tool_router)]` on the `ServerHandler` impl
- `BlikPlanServer` carries a `tool_router: ToolRouter<Self>` field initialised via `Self::tool_router()` in `new()`
- `use std::future::Future;` required where `#[tool]` is used
- Test transport uses `tokio::io::duplex(4096)`, not `rmcp::transport::io::duplex`
- Test client init: `ClientInfo::default().serve(transport).await` (not `Client::new(...)`)
- `peer_info()` returns `Option<&ServerInfo>` — unwrap or pattern-match
- Tool name access: `t.name.as_ref()` (Cow), not `t.name.as_str()`
- `RunningService` has a `DropGuard` — keep alive in spawned tasks via `.waiting().await`

These are all in `crates/blikplan-mcp/src/server.rs` already — reference that file for the working shape.

### Workspace cargo target redirect (machine-specific, gitignored)

`.cargo/config.toml` (workspace root, gitignored) redirects `target/` to `/Volumes/SD EXTREME JT/ganttbok-target/target/` because the internal disk was 99% full. If the SD card is unplugged or the path changes, builds will fail with "no such file or directory" until the config is updated or removed.

If working on another machine: delete `.cargo/config.toml` and cargo will use the default `target/` location again.

### Two stale files left behind

- `src-tauri/Cargo.lock` — predates the workspace. Workspace-level `Cargo.lock` at the repo root is authoritative now. Safe to delete; left in place for caution.
- `crates/blikplan-mcp/src/lib.rs` (4 lines) — a stub from before Task 2 made the crate a bin. Effectively dead code. Safe to delete.

Neither is blocking; clean up when convenient.

### Pre-existing tech debt (not from this build)

- `src-tauri/src/chaser/telegram.rs` doctest fails to compile (broken before Plan 1). Reported by every full test run; ignore it unless touching chaser code.
- 7 TypeScript errors in `App.svelte` + test fixtures (`calendar.test.ts`, `hierarchical-numbering.test.ts`, `store.test.ts`) — broken by v3/v6 migrations not updating the test fixtures. Pre-dates Plan 1.

Neither affects the new connector code.

## Standing rules learned during this session

These are now memorialised in `~/.claude/projects/-Users-cncuser/memory/feedback_subagent_file_scope.md`:

1. **Always pass an explicit "STRICT FILE SCOPE" block to implementer subagents** — list the exact files they may touch and forbid anything else, especially `src/lib/types.ts` which one subagent silently deleted 74 lines of (commit `821fcf8`, restored by `195385f`).
2. **Tell subagents to use explicit paths with `git add`** — never `git add .` or `git add -A`. The earlier subagent picked up an unrelated `.gitignore` change that was sitting in the working tree.
3. **After every subagent reports DONE, run `git show --stat HEAD` yourself** to verify the file list before marking the task complete. Subagent self-reports about scope have been wrong at least once.

## How to manually test the MCP server right now (optional, while waiting on Plan 3)

```bash
cd /Users/cncuser/Desktop/GanttBok
BLIKPLAN_DB=~/Library/Application\ Support/Blik\ Plan/ganttbok.db \
  cargo run --release -p blikplan-mcp
```

Then use any MCP test client (`mcp inspect`, Claude Desktop dev mode, etc.) to call the 8 tools. `propose_patch` writes a row into `pending_patches`; you can verify with:

```bash
sqlite3 ~/Library/Application\ Support/Blik\ Plan/ganttbok.db \
  'SELECT id, status, summary FROM pending_patches'
```

(Until Plan 3 lands the Inbox panel, there's no UI to act on those rows — they just queue up.)

## Open questions for Plan 3 / Plan 4

- **DB-path discrepancy** — the codebase still references `"Gantt Bok"` in places (`src-tauri/src/lib.rs`); the rename to `"Blik Plan"` is a user-prompted one-time action. Plan 4's "Connect to Claude" button should set `$BLIKPLAN_DB` explicitly in the MCP config it writes, so it never relies on the auto-fallback.
- **`add_chaser` template validation** — deferred from Plan 1's validator to Plan 3's apply engine. The template name should be checked against the v1.4 chaser templates (see `src-tauri/src/chaser/`).
- **Pre-existing TS errors** — out of scope for this build, but worth a separate clean-up plan at some point.

## Commit graph at handoff

```
4cf6785 (HEAD) feat(npm): add @blikplan/mcp npm wrapper package
b6d7d98 feat(mcp): add sidecar copy script for Tauri externalBin wiring
7457e17 feat(mcp): wire db_path into main for propose_patch RW access
b3c29c1 feat(mcp): add propose_patch write tool with validation + DB insert
5f5d6b1 test(mcp): add search + today integration tests
b4c2cf7 feat(mcp): implement list_tasks, get_task, list_contacts tools
f5e2c3a feat(mcp): add list_jobs + get_job read tools
8a6ba32 test(mcp): db path discovery + connection flag tests
428d3d0 feat(mcp): add blikplan-mcp crate skeleton with MCP handshake
94b6c68 chore(workspace): cargo target-dir + blikplan-mcp stub
836023c refactor(patches): extract gb-patches workspace crate
178973b chore(workspace): introduce Cargo workspace root
427b37a docs: plan 4 (Connect-to-Claude install flow)
195385f fix(types): restore patch types accidentally dropped in 821fcf8
821fcf8 docs: plan 3 (Inbox panel + apply engine)
a7fe98c docs: plan 2 (MCP server)
80e5871 feat(types): add Patch + PendingPatch TS types          ← Plan 1 ends here
2ada9eb feat(patches): add structural validator with typed errors
d01b57d feat(patches): add shared Patch + PatchOp schema
e014971 feat(db): add pending_patches table (v7 migration)
3feb993 docs: plan 1 (foundation) — Blik Plan ↔ Claude connector
fa63f54 docs: design spec — Blik Plan ↔ Claude connector
96a3924 v1.4.0 — Chaser feature (Telegram bot nudges)            ← shared base
```

End of handoff.
