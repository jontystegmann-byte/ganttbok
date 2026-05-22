# Blik Plan ↔ Claude Connector — Design

**Date:** 2026-05-22
**Status:** Approved, pending implementation plan
**Owner:** JT
**Target release:** v1.6 (beta-labelled)

## Summary

Ship a Model Context Protocol (MCP) server (`blikplan-mcp`) plus an in-app "Connect to Claude (beta)" button so any Blik Plan user can hook their schedule into Claude Code and Claude Desktop with one click. Headline use case: paste a meeting transcript into Claude → Claude proposes a batched patch to the relevant job → user reviews and accepts the patch inside Blik Plan.

Audio transcription is out of scope; that pipeline is being built separately by Jon Holmes. The connector accepts text only.

## Goals

- Any Blik Plan user can connect Claude in one click, no terminal.
- Claude can read the full schedule (jobs, phases, tasks, deps, notes, chasers, contacts) directly.
- Claude can never silently mutate the schedule. All writes go through a preview-gated Inbox inside Blik Plan.
- The MCP server is also installable via `npm` for power users / Linux / headless setups.
- Shipping does not require a redesign of any existing Blik Plan command; the connector wraps the current surface.

## Non-goals (v1)

- No audio ingestion or local transcription.
- No direct write tools exposed to Claude — only `propose_patch`.
- No cloud sync, multi-user collaboration, or remote MCP / OAuth connector.
- No per-job or per-tool permission system; the trust boundary is "Claude is connected at all".
- No encryption-at-rest beyond what `ganttbok.db` already has.

## Architecture

```
Claude Code / Desktop ──MCP stdio──▶ blikplan-mcp (Rust binary)
                                            │
                                     SQLite │ (reads + writes pending_patches)
                                            ▼
                                     ganttbok.db
                                            ▲
                                            │
                          ┌─────────────────┴──────────────────┐
                          │      Blik Plan (Tauri app)         │
                          │  - Inbox panel (watches table)     │
                          │  - "Connect to Claude (beta)" btn  │
                          │  - Existing Gantt UI               │
                          └────────────────────────────────────┘
```

Three deliverables:

1. **`blikplan-mcp`** — Rust binary. Two distribution channels: Tauri sidecar (bundled in the .app) and an npm wrapper (`@blikplan/mcp`) that downloads the prebuilt platform binary on `postinstall`.
2. **Blik Plan additions** — Inbox panel, "Connect to Claude (beta)" Settings panel, `pending_patches` table migration.
3. **Shared schema** — versioned JSON patch format consumed by both sides.

Rust for the server because it reuses Blik Plan's existing query/ripple code, ships as a single static binary per platform, and keeps the toolchain consistent.

## MCP Tool Surface

### Read tools (direct, no preview)

| Tool | Purpose |
|---|---|
| `list_jobs()` | All jobs: id, name, status, date range. |
| `get_job(job_id)` | Full job: phases, tasks, dependencies, notes, chasers. |
| `list_tasks(job_id?, filter?)` | Tasks with optional filters: due window, status, contact, phase. |
| `get_task(task_id)` | Single task with full context. |
| `list_contacts()` | All contacts (for chaser context). |
| `search(query)` | Free-text search across job names, task names, notes. |
| `today(job_id?)` | What's due / overdue / in-progress right now. |

### Write tool (single, preview-gated)

| Tool | Purpose |
|---|---|
| `propose_patch(job_id, patch, summary)` | Inserts a row in `pending_patches`. Returns `patch_id` + human-readable preview. User accepts/rejects inside Blik Plan. |

### Patch document

```jsonc
{
  "patch_version": 1,
  "summary": "From 2026-05-22 site meeting: 4 new tasks for basement phase, push windows back 1 week",
  "ops": [
    { "op": "add_task", "phase_id": "noord.basement", "name": "Order vent ducting from Doug", "due": "2026-06-03", "contact_id": "doug" },
    { "op": "shift_task", "task_id": "noord.windows.order", "by_days": 7 },
    { "op": "add_dependency", "from": "noord.windows.order", "to": "noord.windows.measure" },
    { "op": "add_chaser", "task_id": "noord.solar.plans", "contact_id": "renaissance", "template": "weekly" },
    { "op": "append_note", "job_id": "noord", "text": "Graham wants fewer cavity walls — reopen Henry Fagan" }
  ]
}
```

One fat write tool, not fifteen, because every meeting produces a batch of related changes that should be accepted or rejected as a single decision. One preview, one Accept, one undo unit.

### Response shape for `propose_patch`

```json
{
  "patch_id": "p_01H...",
  "status": "proposed",
  "preview": "Will add 4 tasks to Noordhoek/Basement, shift 1 task by +7 days, add 1 chaser. Open Blik Plan to review.",
  "inbox_count": 3
}
```

Claude is never told whether the user accepted the patch. This is deliberate — it prevents nagging or auto-retry behaviour.

## Install Flow

Lives in Blik Plan: **Settings → Integrations → "Connect to Claude (beta)"**.

Steps when the user clicks Connect:

1. **Detect installed Claude surfaces** by checking known config paths:
   - Claude Code: `~/.claude.json` (macOS/Linux), `%USERPROFILE%\.claude.json` (Windows)
   - Claude Desktop: `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS), `%APPDATA%\Claude\claude_desktop_config.json` (Windows)
2. **Show detection checklist** to the user.
3. **On confirm**, merge an entry into each config's `mcpServers` block:
   ```json
   "mcpServers": {
     "blikplan": {
       "command": "/Applications/Blik Plan.app/Contents/Resources/blikplan-mcp",
       "env": { "BLIKPLAN_DB": "/Users/<user>/Library/Application Support/blikplan/ganttbok.db" }
     }
   }
   ```
   - Existing entries preserved; we merge, never overwrite.
   - Back up the file to `<config>.bak-<timestamp>` before writing.
   - Atomic write: write to `.tmp`, validate JSON parse, rename.
4. **Show next steps card:** "Restart Claude Desktop and start a new Claude Code session. Try: *What's on my Blik Plan schedule this week?*"
5. **Status indicator** stays live: "Connected to Claude Code ✓ · Connected to Claude Desktop ✓". Refresh-on-click only (no aggressive polling).

**Disconnect** does the inverse: removes only the `blikplan` entry, leaves other MCP servers alone.

### npm channel (power users)

```bash
npm install -g @blikplan/mcp
claude mcp add blikplan -- blikplan-mcp
```

Thin wrapper; `postinstall` downloads the right prebuilt platform binary (same pattern as `esbuild`, `swc`).

### Binary bundling

The Rust binary is bundled inside the Blik Plan .app/.exe at build time (adds ~30 MB; offline-safe; no first-connect download).

### `ganttbok.db` discovery

- **Sidecar path:** Blik Plan writes the absolute DB path into the MCP config's `env` block at connect time. Zero ambiguity.
- **npm path:** server checks `$BLIKPLAN_DB`, then OS-default location, then errors with a clear hint.

## Safety Model

### `pending_patches` table

```sql
CREATE TABLE pending_patches (
  id            TEXT PRIMARY KEY,             -- uuid
  job_id        TEXT NOT NULL,
  patch_json    TEXT NOT NULL,                -- full patch document
  summary       TEXT NOT NULL,                -- Claude's one-line description
  source        TEXT NOT NULL DEFAULT 'mcp',  -- future-proof for webhook/etc.
  status        TEXT NOT NULL DEFAULT 'proposed',
  created_at    INTEGER NOT NULL,
  resolved_at   INTEGER,
  error         TEXT                          -- populated if apply_failed
);
```

### Patch lifecycle

```
proposed ──▶ accepted ──▶ applied
    │            │
    │            └──▶ apply_failed (rollback, row kept for diagnosis)
    ▼
rejected
    │
    ▼
expired   (auto-clean after 30 days unactioned)
```

### Apply path (inside Blik Plan, not MCP)

- Wrapped in a single SQLite transaction.
- Reuses the existing Tauri command surface (`create_task`, `drag_task`, etc.) — ripple and dependency logic stay identical to manual edits.
- Any op failure → full rollback, row marked `apply_failed`, Inbox panel highlights the offending op.

### Schema validation

- MCP server validates patch JSON against a versioned JSON Schema before inserting the row.
- `patch_version: 1` field is mandatory. Blik Plan refuses unknown versions rather than guessing.

### Failure modes

| Scenario | Behaviour |
|---|---|
| MCP server can't find `ganttbok.db` | Returns `{ "error": "db_not_found", "hint": "set BLIKPLAN_DB env var" }`. |
| Blik Plan schema newer than MCP server | MCP allows reads, refuses writes, prompts user to update the MCP package. |
| Two Blik Plan installs on one machine | `BLIKPLAN_DB` env var disambiguates; without it, MCP picks the most-recently-modified DB and warns. |
| Concurrent UI edit invalidates a pending patch | Apply checks referenced `task_id`s still exist; missing refs → `apply_failed` with diff of what changed. |
| Inbox grows huge | "Clear resolved" button (sweeps `rejected`, `expired`, `apply_failed`, and `applied` rows older than 7 days) + 30-day auto-expiry on unactioned `proposed` rows. |

## Testing

### MCP server (Rust)

- Unit tests on the patch JSON Schema validator (golden patches pass; mutated patches fail with specific error codes).
- Integration tests against a fixture `ganttbok.db`: every read tool returns expected shape; `propose_patch` inserts the row correctly.
- One end-to-end test using the official MCP test client over stdio: handshake → tool list → call → response.

### Blik Plan additions

- Migration test: fresh DB + migration → `pending_patches` exists with right columns.
- Inbox panel: fixture row renders correct diff; Accept fires the right Tauri commands; Reject marks the row.
- Apply transaction: a patch with one bad op rolls back all ops, row goes `apply_failed`.
- Config-writer: fixture `claude_desktop_config.json` gets `blikplan` merged in without clobbering siblings; backup file created.

### Manual smoke test (release checklist)

Before every release:

1. Fresh Blik Plan install, click Connect to Claude → both surfaces detected and written.
2. Open Claude Code, ask "what's on my Noordhoek schedule" → reads return real data.
3. Paste a meeting transcript, ask Claude to update the schedule → patch appears in Inbox.
4. Accept it → Gantt updates correctly, ripple fires.
5. Disconnect → MCP entry removed, other servers untouched.

### Out of scope (v1)

- Load testing (single-user local app).
- Property-based testing of the patch op space.
- Cross-Claude-version compatibility matrix (target current stable Claude Code + Desktop; older versions best-effort).

## Open Questions

- Monorepo (`ganttbok/packages/mcp`) or separate repo (`blikplan-mcp`)? Defer until writing-plans phase; no design impact.
- Patch schema versioning policy beyond v1 (additive only? breaking changes how?). Decide when v2 is on the table.

## References

- Existing GanttBok command modules — basis for the read-tool implementations.
- `coros-mcp` (JT's prior MCP server) — distribution and structure precedent.
- Blik Plan v1.5 (Blik Plan rebrand + holidays + today-line; shipped 2026-05-22).
- Blik Plan v1.4 plan (chaser feature; in progress) — `propose_patch` ops must support `add_chaser` once v1.4 lands.
