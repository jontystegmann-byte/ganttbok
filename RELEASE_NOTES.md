Blik Plan v1.9.0 — Delete a dependency

NEW
- Click a dependency arrow to select it (turns BP red, thickens to 2px). Press Delete or Backspace to remove it. Undo (Cmd+Z) brings it back. A wide invisible hit-path sits along the visible arrow so the 1px line is comfortable to click without precision aim.

—

Blik Plan v1.8.2 — Fix dependency arrow on zero-gap predecessors

FIX
- When a successor task started in the same column its predecessor ended in (no calendar gap), the dependency arrow's elbow couldn't fit between the bars and wrapped back into the successor — so the arrowhead appeared to come from inside the bar pointing left. The path now detours via a lane just outside the predecessor's row when the gap is tight, so the arrowhead always enters from outside the successor's left edge.

—

Blik Plan v1.8.1 — Fix new-job modal not appearing

FIX
- Clicking "+ New job" in the job-switcher dropdown now actually opens the modal. Since the v1.7 layout redesign the modal was being rendered inside the gantt canvas's isolated stacking context, so it was buried behind the rest of the app. Lifted the modal mount to App.svelte so it renders at the top level.

—

Blik Plan v1.8.0 — Drag physics rewrite

FIX
- Dragging a long bar no longer "snaps further than where you released." The bar now follows the pointer 1:1 in raw pixels, and a 1px red ghost marker shows the workday it will commit to. Release lands on the ghost — no end-of-drag jump.
- Dragging across a visible weekend no longer silently costs an extra workday or two. Pixel-to-workday now respects which calendar dates actually have columns in the timeline.
- Tasks can now be dropped on Saturday or Sunday when the "Show weekends" toggle is on — useful for genuinely weekend-only work (site meetings, weekend shoots). Previously the backend always collapsed weekend drops to the nearest weekday; now the workday arithmetic honours the global include-weekends setting end-to-end.
- Dragging onto a public holiday now snaps to the nearest workable day (forward in ties), respecting the per-job "Holidays block work" flag and the active region.
- Cell width is now read from the canvas instead of a hard-coded 24px constant, so any future zoom won't desynchronise drag from rendering.

UNDER THE HOOD
- New `src/lib/canvas/timeline.ts` pure module owns pixel↔date conversion driven by the rendered `ViewportDay[]`.
- New `src/lib/canvas/drag-physics.ts` module owns the ghost-date computation (`computeGhostDate`).
- New `calendar.snapToNearestWorkable(iso, noWorkSet, includeWeekends)` helper. Symmetric ±90-day search; ties go forward.
- Removed `src/lib/snap.ts` (magneticSnap) and its dead `absFrac > 0.5` branch.
- Rust workday arithmetic (`is_workday` / `add_workdays` / `count_workdays` / `add_workdays_excluding`) now takes an `include_weekends` flag read from the meta KV. Threaded through `apply_ripple`, `compute_ripple`, `drag_task`, `patches::apply`, `chaser::nudge`, and the inbox-review commands.
- 29 new unit tests across calendar, timeline, drag-physics, and the Rust workday module.

—

Blik Plan v1.7.3 — Fix Claude patch proposals

FIX
- The `propose_patch` MCP tool rejected every patch coming from Claude with a "parse_error" because Claude sends the patch document as a JSON string while the server expected an object. The server now accepts both encodings, and the tool advertises the full patch schema so Claude builds valid patches directly. Patches sent from Claude land in your Inbox as expected.
- Error details from `propose_patch` are now properly JSON-escaped (previously a quote inside an error message could produce malformed JSON).

—

Blik Plan v1.7.2 — Compact header

- Header collapsed back to a single row: logo + Inbox · Notes · Contacts · Settings · Print + saved indicator + version.
- The job-switcher dropdown moved into the chart's corner cell (top-left, aligned with the week-days header). Picking jobs is right where the task column starts.
- Empty state (no job selected) centres a job-switcher in the canvas so picking a first job is obvious.

—

Blik Plan v1.7.1 — Fix Claude MCP path

FIX
- The "Connect to Claude" button was writing the wrong sidecar path into Claude's MCP config (Contents/Resources/ instead of Contents/MacOS/), so Claude couldn't actually launch the blikplan MCP server. v1.7.1 resolves the path relative to the running executable so it's always correct on macOS/Linux/Windows. If you previously connected, click Disconnect then Connect again to rewrite the path.

—

Blik Plan v1.7.0 — Live schedule, inbox review, redesigned layout

NEW
- **Interactive task status** — every task carries On Track (green), Late (red), or Done (grey) right on the bar. Pick status from the Status dropdown in TaskDetails. Phases roll up: if any child is Late the phase bar paints red.
- **Inbox review workflow** — when a task's end date passes without being marked Done, an entry appears in the Inbox with the phase, task name, and due date. Two actions:
  - **Mark Done…** opens a date picker (defaults to the planned end). Confirming adjusts the bar's duration so its end aligns with the picked date and ripples dependent tasks both directions: pulls them in if you finished early, pushes them out if you finished late.
  - **Running late** flags it red, catch-up extends the bar so it reaches today, and pushes dependent tasks out by the same workday delta.
- **Daily Late auto-extension** — on app launch and again at midnight rollover, every Late task gets catch-up extended by the workdays that have passed; dependents shift accordingly. Idempotent — same-day re-runs do nothing.
- **Per-job dependency ripple** respects the new `auto_shift_dependents` flag on each job (default ON). Switch it off for exploratory jobs where you don't want dependents auto-moving.
- **Bidirectional ripple** — the dependency engine now supports both downstream pushes (Late) and upstream pulls (early Done). Previously drag-only and downstream-only.
- **Undo (Cmd+Z) covers status changes** — flip a task to Done, regret it, hit Cmd+Z. Bar geometry and dependent shifts both revert.
- **Phase divider lines** — a 1px slate-grey line between every phase so it's obvious which tasks belong to which phase when two phases are expanded next to each other.

REDESIGN
- **Top header bar** with the BLIK Plan logo on the left, an inline **job switcher dropdown** centre-left, and saved indicator + version on the right.
- **Labelled action bar** under the header — Inbox · Notes · Contacts · Settings · Print. Every tool visible, nothing hidden in popovers.
- **Unified right-hand-side panels** — Inbox, Notes, Contacts, and Settings all slide in from the right at the same 420px width. Picking a different tool while one is open swaps them; picking the same tool again closes it.
- **Scroll-locked split view** — the task/phase column and the bar timeline now scroll together in a single shared scroll container with sticky positioning. No more JS sync — completely smooth.
- **Left sidebar removed** — jobs live in the dropdown, so the chart fills the full width.
- **Tooltip on bar hover** — 200ms delay, shows status, start/end, duration, completion date.

—

Blik Plan v1.6.0-beta.1 — Claude connector

FIX (v1.6.0-beta.1)
- The "Connect to Claude (beta)" section is now actually mounted in the Settings popover. In v1.6.0-beta it was wired into an orphan component that the app never rendered, so the button never appeared.

—

v1.6.0-beta — Claude connector

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
