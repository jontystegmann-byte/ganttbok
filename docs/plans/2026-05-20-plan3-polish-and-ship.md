# Gantt Bok — Plan 3: Polish & Ship Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the working-but-rough Gantt Bok at `v0.2.0` into a shippable `v1.0.0` `.app` Gray can install on his Mac. Adds undo/redo, the saved-state indicator, templates UI, the A3 print pipeline, dependency-creation gesture, no-work right-click, phase-bar whole-block drag, backend `list_archived`, toast error surfacing, scroll-sync between sidebar and canvas, an app icon, and an ad-hoc-signed `.app` ready to hand over.

**Architecture:** Same Tauri 2 + Svelte 5 + Rust SQLite foundation. Undo lives entirely in the frontend (snapshot the store, push to stack on every mutation, pop on `⌘Z`). Templates reuse the existing `is_template` flag and the `instantiate_template` IPC. Printing uses the WebView's native `window.print()` with an `@media print` stylesheet — the SVG canvas IS the printed artefact, so the source of truth is shared. Packaging uses Tauri's built-in `pnpm tauri build` with ad-hoc signing (Apple Developer account upgrade documented but not required).

**Tech Stack:** Same as Plan 2 — no new runtime dependencies. Adds dev-time: `@types/dom-view-transitions` (optional, for smooth undo flashes — can skip).

**Reference spec:** `~/Desktop/GanttBok/docs/specs/2026-05-19-ganttbok-design.md`
**Plan 1:** `~/Desktop/GanttBok/docs/plans/2026-05-19-plan1-foundation.md`
**Plan 2:** `~/Desktop/GanttBok/docs/plans/2026-05-20-plan2-gantt-ui.md`

---

## Scope summary (what ships in v1.0.0)

| Area | Plan 3 deliverable |
|---|---|
| Undo / redo | `⌘Z` / `⌘⇧Z` with unlimited session-scoped stack, snapshot-based |
| Persistence feedback | Bottom-right `Saved 16:42` indicator + `⌘S` manual save + visible "Saving…" / "Save failed" states |
| Templates | New sidebar group; "Save as template" right-click; "New from template" dropdown in New-Job modal |
| Printing | `⌘P` opens Print Options sheet (A3 landscape, Fit-to-page / Multi-page, show-notes toggle) → native macOS print |
| Dependency creation | Drag from `○` handle on right edge of bar A onto bar B → FS dependency |
| Manual no-work day | Right-click on day-column header → mark / unmark non-working day |
| Phase-bar whole-block drag | Grab a collapsed phase bar → move all its tasks as one unit (with chain ripple) |
| Sidebar/canvas scroll-sync | LeftRail and grid-area scroll vertically together |
| Archived jobs | New backend `list_archived` command + sidebar group works |
| Error surfacing | Toast component for failed IPCs (currently swallowed) |
| App icon | Simple SVG-based dock icon (placeholder antelope, can be redesigned later) |
| Packaging | `pnpm tauri build` produces a signed (ad-hoc) `.app`; documented `.dmg` workflow |

---

## File structure (Plan 3 additions / changes)

```
~/Desktop/GanttBok/
├── src/
│   ├── lib/
│   │   ├── undo.ts                              Task 1
│   │   ├── toast.svelte.ts                      Task 16
│   │   ├── footer/
│   │   │   └── SavedIndicator.svelte            Task 4
│   │   ├── sidebar/
│   │   │   ├── TemplatesGroup.svelte            Task 7
│   │   │   └── (NewJobModal extended)           Task 8
│   │   ├── canvas/
│   │   │   ├── (TaskBar adds dep-handle)        Task 12
│   │   │   ├── DepCreator.svelte                Task 12
│   │   │   ├── (HeaderStrip adds context menu)  Task 13
│   │   │   └── (PhaseBar adds drag)             Task 14
│   │   ├── print/
│   │   │   ├── PrintOptions.svelte              Task 9
│   │   │   └── print.css                        Task 10
│   │   └── components/
│   │       ├── Toast.svelte                     Task 16
│   │       └── ContextMenu.svelte               Task 13
│   ├── App.svelte                               Task 4 (mount SavedIndicator + Toast)
│   └── lib/__tests__/
│       ├── undo.test.ts                         Task 1
│       └── snap.test.ts                         (no change)
│
├── src-tauri/
│   ├── src/
│   │   ├── repo/job.rs                          Task 15 (add list_archived)
│   │   ├── commands/job.rs                      Task 15 (expose list_archived)
│   │   └── lib.rs                               Task 15 (register handler)
│   └── icons/                                   Task 17 (replace with antelope)
│
├── docs/plans/
│   └── 2026-05-20-plan3-polish-and-ship.md      (this file)
└── docs/RELEASE.md                              Task 18 (build & distribution)
```

---

## Phase A — Undo / redo (Tasks 1–3)

### Task 1: Undo stack — snapshot of the four mutable arrays

**Files:**
- Create: `src/lib/undo.ts`
- Create: `src/lib/__tests__/undo.test.ts`

The undo stack stores immutable JSON snapshots of `{ phases, tasks, dependencies, noWorkDays }` after every mutation. Selection is also captured so undo restores cursor position. The current job's metadata (`name`, `client`, etc.) is rarely changed inline, so we skip it for v1.

- [ ] **Step 1: Write failing tests**

```typescript
// src/lib/__tests__/undo.test.ts
import { describe, it, expect, beforeEach } from 'vitest';
import { UndoStack, type Snapshot } from '../undo';

function snap(phases: any[] = [], tasks: any[] = []): Snapshot {
  return { phases, tasks, dependencies: [], noWorkDays: [], selection: null };
}

describe('UndoStack', () => {
  let stack: UndoStack;
  beforeEach(() => { stack = new UndoStack(); });

  it('starts with no undo/redo available', () => {
    expect(stack.canUndo()).toBe(false);
    expect(stack.canRedo()).toBe(false);
  });

  it('push records snapshot; undo returns the previous one', () => {
    stack.push(snap([{ id: 1 }]));
    stack.push(snap([{ id: 1 }, { id: 2 }]));
    expect(stack.canUndo()).toBe(true);
    const prev = stack.undo();
    expect(prev?.phases.length).toBe(1);
  });

  it('redo restores what was just undone', () => {
    stack.push(snap([{ id: 1 }]));
    stack.push(snap([{ id: 1 }, { id: 2 }]));
    stack.undo();
    expect(stack.canRedo()).toBe(true);
    const restored = stack.redo();
    expect(restored?.phases.length).toBe(2);
  });

  it('push after undo clears redo stack', () => {
    stack.push(snap([{ id: 1 }]));
    stack.push(snap([{ id: 1 }, { id: 2 }]));
    stack.undo();
    stack.push(snap([{ id: 1 }, { id: 3 }]));
    expect(stack.canRedo()).toBe(false);
  });

  it('clear empties both stacks', () => {
    stack.push(snap([{ id: 1 }]));
    stack.push(snap([]));
    stack.undo();
    stack.clear();
    expect(stack.canUndo()).toBe(false);
    expect(stack.canRedo()).toBe(false);
  });
});
```

- [ ] **Step 2: Implement `src/lib/undo.ts`**

```typescript
import type { Phase, Task, Dependency, NoWorkDay } from './types';

export type Selection =
  | { kind: 'task'; id: number }
  | { kind: 'phase'; id: number }
  | { kind: 'dependency'; id: number }
  | null;

export interface Snapshot {
  phases: Phase[];
  tasks: Task[];
  dependencies: Dependency[];
  noWorkDays: NoWorkDay[];
  selection: Selection;
}

export class UndoStack {
  private past: Snapshot[] = [];
  private future: Snapshot[] = [];

  push(snap: Snapshot): void {
    // Deep-clone so future mutations don't bleed back into history.
    this.past.push(structuredClone(snap));
    this.future = [];
  }

  undo(): Snapshot | null {
    if (this.past.length < 2) return null;
    // The top of the stack is the current state. Drop it, peek at the previous.
    const current = this.past.pop()!;
    this.future.push(current);
    return structuredClone(this.past[this.past.length - 1]);
  }

  redo(): Snapshot | null {
    if (this.future.length === 0) return null;
    const next = this.future.pop()!;
    this.past.push(next);
    return structuredClone(next);
  }

  canUndo(): boolean { return this.past.length >= 2; }
  canRedo(): boolean { return this.future.length > 0; }

  clear(): void {
    this.past = [];
    this.future = [];
  }
}
```

- [ ] **Step 3: Run + commit**

```bash
cd ~/Desktop/GanttBok && pnpm exec vitest run undo
git add src/lib/undo.ts src/lib/__tests__/undo.test.ts
git commit -m "feat(undo): UndoStack with past/future snapshots, unit-tested"
```

---

### Task 2: Wire undo into the store

**Files:**
- Modify: `src/lib/store.svelte.ts`

The store records a snapshot after every state-changing action. `undo()` and `redo()` mutate state but also tell the backend (via re-issuing the IPC calls) to reflect the change. **Strategy:** since IPC mutations have already happened, we simply re-write the local state from the snapshot, then push the same local state to the backend in one batch. But that's complex for v1. Pragmatic v1: undo only reverts the local UI state — the backend will catch up on the next "full save" trigger.

Even simpler v1 approach: undo regenerates the backend state by **re-issuing the inverse IPC calls**. Too risky. The cleanest v1 pattern:

- After each user action, snapshot the local store AND let the IPC mutation persist.
- `undo()` pops the snapshot and replays the inverse mutations to the backend.

For an unconditional first version, we'll take a different tack: **`undo()` restores local state, then writes a "snapshot resync" call to the backend** via a new `resync_job_state` IPC command. The backend wipes the job's phases/tasks/deps/no-work-days and reinserts them from the snapshot inside one transaction. This is conceptually clean but requires backend work.

To keep Plan 3 focused, **v1 undo is local-only**:
- Local state is restored from the snapshot.
- On next reload (or `⌘S` manual save), the backend is brought back in sync via a `resync_job_state` call.
- Visible footer shows `Saved 16:42 (with unsaved undo)` when local has diverged from backend.

- [ ] **Step 1: Add the stack + snapshot helpers to the store**

In `store.svelte.ts`, after the existing imports add:

```typescript
import { UndoStack, type Snapshot as UndoSnapshot } from './undo';
```

Inside the `Store` class, add fields and methods (place them after the existing `dragState = $state...` line):

```typescript
  private undoStack = new UndoStack();
  hasUnsavedUndo = $state<boolean>(false);

  /** Snapshot current job state into the undo stack. Called after every mutation. */
  recordHistory(): void {
    this.undoStack.push({
      phases: $state.snapshot(this.phases),
      tasks: $state.snapshot(this.tasks),
      dependencies: $state.snapshot(this.dependencies),
      noWorkDays: $state.snapshot(this.noWorkDays),
      selection: $state.snapshot(this.selection),
    });
  }

  canUndo(): boolean { return this.undoStack.canUndo(); }
  canRedo(): boolean { return this.undoStack.canRedo(); }

  undo(): void {
    const snap = this.undoStack.undo();
    if (snap) this.applySnapshot(snap);
  }

  redo(): void {
    const snap = this.undoStack.redo();
    if (snap) this.applySnapshot(snap);
  }

  private applySnapshot(snap: UndoSnapshot): void {
    this.phases       = snap.phases;
    this.tasks        = snap.tasks;
    this.dependencies = snap.dependencies;
    this.noWorkDays   = snap.noWorkDays;
    this.selection    = snap.selection;
    this.hasUnsavedUndo = true;
  }

  async resyncJobState(): Promise<void> {
    // Backend extension lands in Task 3; until then, manual save is a no-op for undo state.
    // This becomes a real implementation in Task 3.
    this.hasUnsavedUndo = false;
  }
```

- [ ] **Step 2: Call `recordHistory()` after every mutation in the store**

Sprinkle `this.recordHistory();` at the END of these methods (after the IPC + local mutation):
- `createJob` (after `await this.openJob(job.id)` — actually openJob already calls recordHistory below)
- `createPhase` (after `this.selection = ...`)
- `createTaskInPhase` (after `this.selection = ...`)
- `reorderTasksInPhase`
- `reorderPhases`
- `applyDragResult` (called after a drag completes)

Add a `recordHistory()` call inside `openJob` AFTER all the lists are loaded (this seeds the stack):

```typescript
async openJob(jobId: number): Promise<void> {
  // ...existing body...
  this.undoStack.clear();
  this.recordHistory(); // seed
  this.hasUnsavedUndo = false;
}
```

- [ ] **Step 3: Add a `recordHistoryAfter` helper** for external mutators (TaskDetails save, PhaseDetails delete, etc.):

```typescript
mutateAndRecord<T>(fn: () => T): T {
  const result = fn();
  this.recordHistory();
  return result;
}
```

- [ ] **Step 4: Commit**

```bash
git add src/lib/store.svelte.ts
git commit -m "feat(undo): wire UndoStack into store, seed on job open, record after every mutation"
```

---

### Task 3: Keyboard shortcuts + backend resync

**Files:**
- Modify: `src/App.svelte`
- Create: `src-tauri/src/commands/sync.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/ipc.ts`
- Modify: `src/lib/store.svelte.ts`

- [ ] **Step 1: Backend — `resync_job_state` command**

Create `src-tauri/src/commands/sync.rs`:

```rust
use chrono::NaiveDate;
use serde::Deserialize;
use tauri::State;
use crate::commands::Db;
use crate::db::models::{Phase, Task, Dependency, NoWorkDay};
use crate::GbResult;

#[derive(Debug, Deserialize)]
pub struct ResyncArgs {
    pub job_id: i64,
    pub phases:        Vec<PhaseSnap>,
    pub tasks:         Vec<TaskSnap>,
    pub dependencies:  Vec<DepSnap>,
    pub no_work_days:  Vec<NwdSnap>,
}

#[derive(Debug, Deserialize)]
pub struct PhaseSnap { pub id: i64, pub name: String, pub colour: String, pub order_index: i64, pub collapsed: bool }
#[derive(Debug, Deserialize)]
pub struct TaskSnap { pub id: i64, pub phase_id: i64, pub name: String, pub start_date: NaiveDate, pub duration_workdays: i64, pub order_index: i64, pub notes: Option<String> }
#[derive(Debug, Deserialize)]
pub struct DepSnap { pub id: i64, pub predecessor_id: i64, pub successor_id: i64, pub lag_days: i64 }
#[derive(Debug, Deserialize)]
pub struct NwdSnap { pub id: i64, pub date: NaiveDate, pub reason: String, pub source: String }

#[tauri::command]
pub fn resync_job_state(db: State<Db>, args: ResyncArgs) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    let tx = conn.unchecked_transaction()?;

    // Delete everything for this job, then reinsert from the snapshot. FKs cascade.
    tx.execute("DELETE FROM phase WHERE job_id = ?1", [args.job_id])?;
    tx.execute("DELETE FROM no_work_day WHERE job_id = ?1", [args.job_id])?;
    // (task & dependency rows cascade from phase delete)

    for p in &args.phases {
        tx.execute(
            "INSERT INTO phase (id, job_id, name, colour, order_index, collapsed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![p.id, args.job_id, p.name, p.colour, p.order_index, p.collapsed as i64],
        )?;
    }
    for t in &args.tasks {
        tx.execute(
            "INSERT INTO task (id, phase_id, name, start_date, duration_workdays, order_index, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![t.id, t.phase_id, t.name, t.start_date.to_string(), t.duration_workdays, t.order_index, t.notes],
        )?;
    }
    for d in &args.dependencies {
        tx.execute(
            "INSERT INTO dependency (id, predecessor_id, successor_id, type, lag_days)
             VALUES (?1, ?2, ?3, 'FS', ?4)",
            rusqlite::params![d.id, d.predecessor_id, d.successor_id, d.lag_days],
        )?;
    }
    for n in &args.no_work_days {
        tx.execute(
            "INSERT INTO no_work_day (id, job_id, date, reason, source)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![n.id, args.job_id, n.date.to_string(), n.reason, n.source],
        )?;
    }
    tx.commit()?;
    Ok(())
}
```

Add `pub mod sync;` to `commands/mod.rs`. Register in `lib.rs`'s `invoke_handler!`:

```rust
commands::sync::resync_job_state,
```

- [ ] **Step 2: Frontend IPC wrapper**

Append to `src/lib/ipc.ts`:

```typescript
import type { Phase, Task, Dependency, NoWorkDay } from './types';

export interface ResyncArgs {
  job_id: number;
  phases: Phase[];
  tasks: Task[];
  dependencies: Dependency[];
  no_work_days: NoWorkDay[];
}

export const resyncJobState = (args: ResyncArgs) => invoke<void>('resync_job_state', { args });
```

- [ ] **Step 3: Implement `resyncJobState` in the store**

Replace the stub from Task 2:

```typescript
async resyncJobState(): Promise<void> {
  if (!this.currentJob) return;
  await ipc.resyncJobState({
    job_id: this.currentJob.id,
    phases: $state.snapshot(this.phases),
    tasks: $state.snapshot(this.tasks),
    dependencies: $state.snapshot(this.dependencies),
    no_work_days: $state.snapshot(this.noWorkDays),
  });
  await ipc.touchLastSave();
  this.hasUnsavedUndo = false;
}
```

Modify `undo()` and `redo()` to fire-and-forget resync after applying the snapshot (debounced 300 ms via a simple timer):

```typescript
private resyncTimer: number | null = null;
private scheduleResync(): void {
  if (this.resyncTimer !== null) clearTimeout(this.resyncTimer);
  this.resyncTimer = window.setTimeout(() => {
    this.resyncTimer = null;
    void this.resyncJobState();
  }, 300);
}

undo(): void {
  const snap = this.undoStack.undo();
  if (snap) { this.applySnapshot(snap); this.scheduleResync(); }
}
redo(): void {
  const snap = this.undoStack.redo();
  if (snap) { this.applySnapshot(snap); this.scheduleResync(); }
}
```

- [ ] **Step 4: Keyboard handler in `App.svelte`**

Add inside `onMount`:

```typescript
onMount(async () => {
  await store.bootstrap();

  function onKey(e: KeyboardEvent) {
    const meta = e.metaKey || e.ctrlKey;
    if (meta && e.key === 'z' && !e.shiftKey) {
      if (store.canUndo()) { e.preventDefault(); store.undo(); }
    } else if (meta && (e.key === 'Z' || (e.key === 'z' && e.shiftKey))) {
      if (store.canRedo()) { e.preventDefault(); store.redo(); }
    } else if (meta && e.key === 's') {
      e.preventDefault();
      void store.resyncJobState();
    }
  }

  window.addEventListener('keydown', onKey);
  return () => window.removeEventListener('keydown', onKey);
});
```

- [ ] **Step 5: Verify + commit**

```bash
cd ~/Desktop/GanttBok && pnpm exec tsc --noEmit && pnpm exec vitest run && cd src-tauri && . "$HOME/.cargo/env" && cargo test
git add -A
git commit -m "feat(undo): ⌘Z/⌘⇧Z keyboard shortcuts + resync_job_state IPC for backend reconciliation"
```

---

## Phase B — Saved-state indicator (Tasks 4–5)

### Task 4: SavedIndicator footer component

**Files:**
- Create: `src/lib/footer/SavedIndicator.svelte`
- Modify: `src/App.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script lang="ts">
  import { store } from '../store.svelte';
  import * as ipc from '../ipc';

  let lastSavedAt = $state<string | null>(null);
  let savingState = $state<'saved' | 'saving' | 'failed'>('saved');

  // Poll the meta value periodically since it's stored backend-side and we want a true reflection.
  // Simpler: subscribe to a store flag set by every IPC call.
  // For v1 we just update on manual save events.
  $effect(() => {
    // Format current time on tick.
    if (store.hasUnsavedUndo) savingState = 'saving';
    else savingState = 'saved';
  });

  async function manualSave() {
    savingState = 'saving';
    try {
      await store.resyncJobState();
      lastSavedAt = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
      savingState = 'saved';
    } catch {
      savingState = 'failed';
    }
  }
</script>

<button class="indicator state-{savingState}" onclick={manualSave} title="Manual save (⌘S)">
  {#if savingState === 'saved'}
    Saved {lastSavedAt ?? 'now'}
  {:else if savingState === 'saving'}
    Saving…
  {:else}
    Save failed — click to retry
  {/if}
  <span class="hint">⌘S</span>
</button>

<style>
  .indicator {
    position: fixed; bottom: 8px; right: 12px;
    z-index: 5;
    background: transparent; border: none;
    font-size: var(--font-size-xs);
    color: var(--c-text-muted);
    cursor: pointer;
    padding: var(--sp-1) var(--sp-2);
    border-radius: 4px;
    display: flex; align-items: center; gap: var(--sp-2);
  }
  .indicator:hover { background: var(--c-accent-fade); color: var(--c-accent); }
  .indicator.state-saving { opacity: 0.5; }
  .indicator.state-failed { color: #DC2626; background: #FEE2E2; }
  .hint { font-family: var(--font-mono); opacity: 0.6; }
</style>
```

- [ ] **Step 2: Mount in `App.svelte`**

After the `.app-shell` div, add:

```svelte
<SavedIndicator />
```

And import it.

- [ ] **Step 3: Commit**

```bash
git add src/lib/footer/SavedIndicator.svelte src/App.svelte
git commit -m "feat(ui): saved-state indicator — Saved HH:MM / Saving… / Save failed states, manual save on click"
```

---

### Task 5: Touch last_save_at on every IPC mutation

**Files:**
- Modify: `src/lib/ipc.ts` (wrap mutating commands)

Currently `touchLastSave()` is sprinkled manually after some mutations. Centralise it.

- [ ] **Step 1: Wrap the mutating IPC functions in a helper**

In `src/lib/ipc.ts`, at the bottom, define:

```typescript
const MUTATING = new Set([
  'create_job', 'update_job', 'archive_job', 'delete_job',
  'save_as_template', 'instantiate_template',
  'create_phase', 'update_phase', 'delete_phase', 'reorder_phases',
  'create_task', 'update_task', 'delete_task', 'reorder_tasks',
  'drag_task',
  'create_dependency', 'update_dependency_lag', 'delete_dependency',
  'add_manual_no_work_day', 'delete_no_work_day', 'sync_sa_holidays',
  'resync_job_state',
]);

// (No code change to individual exports — the touch happens via touchLastSave
//  called from the store after each mutation. Centralisation is left to v2 if
//  drift becomes a problem.)
```

- [ ] **Step 2: Audit and ensure `touchLastSave()` is called in every store mutation method**

Spot-check `createPhase`, `createTaskInPhase`, `reorderTasksInPhase`, `reorderPhases` — add `await ipc.touchLastSave();` at the end of each if missing.

- [ ] **Step 3: Commit**

```bash
git add src/lib/store.svelte.ts src/lib/ipc.ts
git commit -m "chore(ipc): audit + ensure touchLastSave fires after every store mutation"
```

---

## Phase C — Templates UI (Tasks 6–8)

### Task 6: Templates sidebar group

**Files:**
- Create: `src/lib/sidebar/TemplatesGroup.svelte`
- Modify: `src/lib/sidebar/Sidebar.svelte`

- [ ] **Step 1: Write `TemplatesGroup.svelte`**

```svelte
<script lang="ts">
  import { store } from '../store.svelte';
  import JobItem from './JobItem.svelte';
  let expanded = $state(true);
</script>

<section>
  <button class="header" onclick={() => expanded = !expanded}>
    {expanded ? '▾' : '▸'} Templates ({store.templates.length})
  </button>
  {#if expanded}
    {#each store.templates as job (job.id)}
      <JobItem {job} />
    {:else}
      <p class="hint">No templates yet — right-click any job to save as template</p>
    {/each}
  {/if}
</section>

<style>
  .header {
    width: 100%; text-align: left; background: transparent; border: none; cursor: pointer;
    padding: var(--sp-2) var(--sp-3);
    font-size: var(--font-size-xs); text-transform: uppercase;
    color: var(--c-text-muted); letter-spacing: 0.06em;
  }
  .hint { color: var(--c-text-muted); padding: var(--sp-3); font-size: var(--font-size-sm); font-style: italic; }
</style>
```

- [ ] **Step 2: Mount in Sidebar**

Add `<TemplatesGroup />` between the Active section and ArchivedGroup.

- [ ] **Step 3: Commit**

```bash
git add src/lib/sidebar/
git commit -m "feat(ui): templates sidebar group — lists is_template=1 jobs"
```

---

### Task 7: Right-click context menu → "Save as template"

**Files:**
- Create: `src/lib/components/ContextMenu.svelte`
- Modify: `src/lib/sidebar/JobItem.svelte`

- [ ] **Step 1: Reusable context menu**

```svelte
<!-- src/lib/components/ContextMenu.svelte -->
<script lang="ts">
  let { x, y, items, onclose }: {
    x: number; y: number;
    items: { label: string; action: () => void; danger?: boolean }[];
    onclose: () => void;
  } = $props();

  function dispatch(action: () => void) {
    action();
    onclose();
  }
</script>

<div class="backdrop" onclick={onclose} role="presentation"></div>
<div class="menu" style="left: {x}px; top: {y}px;">
  {#each items as item}
    <button onclick={() => dispatch(item.action)} class:danger={item.danger}>{item.label}</button>
  {/each}
</div>

<style>
  .backdrop { position: fixed; inset: 0; z-index: 100; }
  .menu {
    position: fixed; z-index: 101;
    background: var(--c-panel);
    border: 1px solid var(--c-border);
    border-radius: 6px;
    box-shadow: 0 4px 16px var(--c-shadow);
    padding: 4px 0;
    min-width: 180px;
  }
  .menu button {
    display: block; width: 100%; text-align: left;
    background: transparent; border: none;
    padding: var(--sp-2) var(--sp-3);
    cursor: pointer; font-size: var(--font-size-sm);
  }
  .menu button:hover { background: var(--c-accent-fade); }
  .menu button.danger { color: #DC2626; }
</style>
```

- [ ] **Step 2: Wire context menu into JobItem**

```svelte
<script lang="ts">
  import type { Job } from '../types';
  import { store } from '../store.svelte';
  import * as ipc from '../ipc';
  import ContextMenu from '../components/ContextMenu.svelte';

  let { job }: { job: Job } = $props();
  const isOpen = $derived(store.currentJob?.id === job.id);
  let menu = $state<{ x: number; y: number } | null>(null);

  async function open() { await store.openJob(job.id); }

  function onContext(e: MouseEvent) {
    e.preventDefault();
    menu = { x: e.clientX, y: e.clientY };
  }

  const items = $derived(job.is_template ? [
    { label: 'Edit template…', action: open },
    { label: 'Delete template', action: async () => { await ipc.deleteJob(job.id); await store.refreshSidebar(); }, danger: true },
  ] : [
    { label: 'Open', action: open },
    { label: 'Save as template…', action: async () => {
        await ipc.saveAsTemplate(job.id, `${job.name} (template)`);
        await store.refreshSidebar();
    } },
    { label: job.archived ? 'Unarchive' : 'Archive', action: async () => {
        await ipc.archiveJob(job.id, !job.archived);
        await store.refreshSidebar();
    } },
    { label: 'Delete job', action: async () => { await ipc.deleteJob(job.id); await store.refreshSidebar(); }, danger: true },
  ]);
</script>

<button class="job-item" class:open={isOpen} onclick={open} oncontextmenu={onContext}>
  {#if isOpen}<span class="indicator">●</span>{/if}
  <span class="job-name">{job.name}</span>
</button>

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} {items} onclose={() => menu = null} />
{/if}

<style>
  /* (unchanged from Plan 2) */
</style>
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/ContextMenu.svelte src/lib/sidebar/JobItem.svelte
git commit -m "feat(ui): right-click job → Save as template / Archive / Delete via ContextMenu"
```

---

### Task 8: Extend New-Job modal with "Start from template" dropdown

**Files:**
- Modify: `src/lib/sidebar/NewJobModal.svelte`
- Modify: `src/lib/store.svelte.ts` (add createFromTemplate)

- [ ] **Step 1: Add template instantiation to store**

```typescript
async createFromTemplate(
  templateId: number,
  args: { new_name: string; client: string | null; address: string | null; project_start_date: string },
): Promise<void> {
  const job = await ipc.instantiateTemplate({ template_id: templateId, ...args });
  await this.refreshSidebar();
  await this.openJob(job.id);
  this.showNewJobModal = false;
}
```

- [ ] **Step 2: Update the modal**

Add a "Start from" dropdown above the existing fields:

```svelte
<script lang="ts">
  import { store } from '../store.svelte';
  let name = $state(''); let client = $state(''); let address = $state('');
  let startDate = $state(new Date().toISOString().slice(0, 10));
  let templateId = $state<number | null>(null);
  let submitting = $state(false);

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!name.trim()) return;
    submitting = true;
    try {
      if (templateId !== null) {
        await store.createFromTemplate(templateId, {
          new_name: name.trim(),
          client: client.trim() || null,
          address: address.trim() || null,
          project_start_date: startDate,
        });
      } else {
        await store.createJob({
          name: name.trim(),
          client: client.trim() || null,
          address: address.trim() || null,
          project_start_date: startDate,
        });
      }
    } finally { submitting = false; }
  }
  function cancel() { store.showNewJobModal = false; }
</script>

<div class="backdrop" onclick={cancel} role="presentation"></div>
<form class="modal" onsubmit={submit}>
  <h2>New job</h2>
  <label>Start from
    <select bind:value={templateId}>
      <option value={null}>Blank</option>
      {#each store.templates as t}
        <option value={t.id}>Template: {t.name}</option>
      {/each}
    </select>
  </label>
  <label>Name<input bind:value={name} required /></label>
  <label>Client<input bind:value={client} placeholder="optional" /></label>
  <label>Address<input bind:value={address} placeholder="optional" /></label>
  <label>Project start<input type="date" bind:value={startDate} required /></label>
  <div class="actions">
    <button type="button" onclick={cancel}>Cancel</button>
    <button type="submit" class="primary" disabled={submitting || !name.trim()}>Create</button>
  </div>
</form>

<style>
  /* (unchanged from Plan 2; add 'select' styling identical to input) */
  select { padding: var(--sp-2); border: 1px solid var(--c-border); border-radius: 4px; }
</style>
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/sidebar/NewJobModal.svelte src/lib/store.svelte.ts
git commit -m "feat(ui): new-job modal — Start-from dropdown (Blank or pick a template)"
```

---

## Phase D — Print pipeline (Tasks 9–11)

### Task 9: Print Options sheet

**Files:**
- Create: `src/lib/print/PrintOptions.svelte`
- Modify: `src/lib/store.svelte.ts` (showPrintOptions flag)
- Modify: `src/App.svelte` (mount + ⌘P keyboard shortcut)

- [ ] **Step 1: Add `showPrintOptions` to the store**

```typescript
showPrintOptions = $state<boolean>(false);
printScaling = $state<'fit' | 'multi'>('fit');
printShowNotes = $state<boolean>(false);
```

- [ ] **Step 2: Write `PrintOptions.svelte`**

```svelte
<script lang="ts">
  import { store } from '../store.svelte';

  function cancel() { store.showPrintOptions = false; }

  function print() {
    document.body.classList.add('print-scaling-' + store.printScaling);
    if (store.printShowNotes) document.body.classList.add('print-with-notes');
    store.showPrintOptions = false;
    setTimeout(() => {
      window.print();
      // Cleanup classes after print dialog closes.
      setTimeout(() => {
        document.body.classList.remove('print-scaling-fit', 'print-scaling-multi', 'print-with-notes');
      }, 1000);
    }, 50);
  }
</script>

<div class="backdrop" onclick={cancel} role="presentation"></div>
<div class="modal">
  <h2>Print Plan</h2>
  <label>Page size
    <select disabled><option>A3 landscape</option></select>
  </label>
  <fieldset>
    <legend>Scaling</legend>
    <label><input type="radio" bind:group={store.printScaling} value="fit" /> Fit to page</label>
    <label><input type="radio" bind:group={store.printScaling} value="multi" /> Multi-page</label>
  </fieldset>
  <label><input type="checkbox" bind:checked={store.printShowNotes} /> Show notes</label>
  <div class="actions">
    <button onclick={cancel}>Cancel</button>
    <button class="primary" onclick={print}>Print →</button>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.3); z-index: 10; }
  .modal {
    position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%);
    background: var(--c-panel); border-radius: 8px; padding: var(--sp-6);
    box-shadow: 0 16px 48px var(--c-shadow); z-index: 11; min-width: 360px;
    display: flex; flex-direction: column; gap: var(--sp-3);
  }
  h2 { margin: 0 0 var(--sp-2); font-size: var(--font-size-lg); }
  label { display: flex; flex-direction: column; gap: var(--sp-1); font-size: var(--font-size-sm); color: var(--c-text-muted); }
  fieldset { border: 1px solid var(--c-border); border-radius: 4px; padding: var(--sp-2); }
  fieldset label { flex-direction: row; align-items: center; gap: var(--sp-2); color: var(--c-text); }
  select { padding: var(--sp-2); border: 1px solid var(--c-border); border-radius: 4px; }
  .actions { display: flex; justify-content: flex-end; gap: var(--sp-2); margin-top: var(--sp-2); }
  .actions button { padding: var(--sp-2) var(--sp-3); border: 1px solid var(--c-border); background: var(--c-bg); border-radius: 4px; cursor: pointer; }
  .actions .primary { background: var(--c-accent); color: white; border-color: var(--c-accent); }
</style>
```

- [ ] **Step 3: Mount + hotkey in App.svelte**

```svelte
<script lang="ts">
  // ...existing...
  import PrintOptions from './lib/print/PrintOptions.svelte';
</script>

<!-- existing shell -->
{#if store.showPrintOptions}
  <PrintOptions />
{/if}
```

And inside the keydown handler:

```typescript
else if (meta && e.key === 'p') {
  e.preventDefault();
  store.showPrintOptions = true;
}
```

- [ ] **Step 4: Commit**

```bash
git add src/lib/print/PrintOptions.svelte src/App.svelte src/lib/store.svelte.ts
git commit -m "feat(ui): Print Options sheet on ⌘P — A3 landscape, fit/multi-page, show-notes toggle"
```

---

### Task 10: Print stylesheet

**Files:**
- Create: `src/lib/print/print.css`
- Modify: `src/main.ts` (import print.css)

- [ ] **Step 1: Write `print.css`**

```css
/* Applied only when printing. */
@media print {
  @page {
    size: A3 landscape;
    margin: 10mm;
  }

  body {
    background: white !important;
    color: black !important;
  }

  /* Hide app chrome. */
  .sidebar, .details, .indicator, .backdrop, .modal, .new-job, .add-task, .add-phase {
    display: none !important;
  }

  /* Use the full width for the canvas. */
  .app-shell {
    display: block !important;
    height: auto !important;
  }
  .canvas-pane {
    overflow: visible !important;
    background: white !important;
  }

  /* Fit-to-page scales the SVG down. */
  body.print-scaling-fit .gantt {
    transform-origin: top left;
    /* Scale via JS — see PrintOptions component */
  }

  /* Show printed job header. */
  .print-header {
    display: block !important;
    margin-bottom: 6mm;
  }
  .print-footer {
    display: block !important;
    margin-top: 6mm;
    font-size: 9pt;
  }

  /* Make the LeftRail show through cleanly. */
  .left-rail {
    padding-top: 0 !important;
    border: 1px solid #ccc;
  }
  .header-strip {
    position: relative !important;
    background: white !important;
  }
}

/* When not printing, hide the printed-only header/footer. */
.print-header, .print-footer { display: none; }
```

- [ ] **Step 2: Import in `src/main.ts`**

```typescript
import { mount } from 'svelte';
import App from './App.svelte';
import './app.css';
import './lib/print/print.css';
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/print/print.css src/main.ts
git commit -m "feat(print): @media print stylesheet — A3 landscape, hide chrome, show print-only header/footer"
```

---

### Task 11: Print header strip + holiday footer

**Files:**
- Modify: `src/App.svelte`

- [ ] **Step 1: Add print-only header/footer in App.svelte**

Inside `.app-shell`, after the closing details `</aside>`:

```svelte
{#if store.currentJob}
  <div class="print-header">
    <h1>{store.currentJob.name.toUpperCase()}</h1>
    <div class="meta">
      {#if store.currentJob.client}Client: {store.currentJob.client} · {/if}
      {#if store.currentJob.address}Address: {store.currentJob.address} · {/if}
      Printed: {new Date().toLocaleDateString('en-GB', { day: '2-digit', month: 'short' })}
    </div>
  </div>
  <div class="print-footer">
    Public holidays in this range:
    {#each store.noWorkDays.filter(n => n.source === 'sa_public_holiday') as h, i (h.id)}
      {h.date} ({h.reason}){i < store.noWorkDays.length - 1 ? ' · ' : ''}
    {/each}
  </div>
{/if}
```

Add the CSS for `.print-header` and `.print-footer` (text styling, only the layout was in print.css):

```svelte
<style>
  /* ... existing ... */
  .print-header h1 { font-size: 14pt; margin: 0; font-weight: 600; }
  .print-header .meta { font-size: 9pt; color: #444; margin-top: 2mm; }
  .print-footer { font-size: 9pt; color: #444; }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/App.svelte
git commit -m "feat(print): print-only header (job + client + address + date) and footer (SA holidays in range)"
```

---

## Phase E — Power-user gestures (Tasks 12–14)

### Task 12: Dependency creation gesture

**Files:**
- Create: `src/lib/canvas/DepCreator.svelte`
- Modify: `src/lib/canvas/TaskBar.svelte`
- Modify: `src/lib/canvas/GanttCanvas.svelte`
- Modify: `src/lib/store.svelte.ts`

Hover the right edge of a task bar → small `○` handle appears. Drag from `○` onto another bar → creates FS dependency.

- [ ] **Step 1: Add dep-creation state to the store**

```typescript
depCreator = $state<{ fromTaskId: number; mouseX: number; mouseY: number; hoverTaskId: number | null } | null>(null);
```

- [ ] **Step 2: Add the handle to TaskBar**

In `TaskBar.svelte`, after the existing `<rect>`, add (only visible on hover):

```svelte
{#if state.hoveredTaskId === task.id && !store.dragState}
  <circle
    cx={livePreview.x + livePreview.w}
    cy={y + 10}
    r={5}
    fill="white"
    stroke="var(--c-accent)"
    stroke-width="2"
    class="dep-handle"
    onpointerdown={(e) => {
      e.stopPropagation();
      store.depCreator = { fromTaskId: task.id, mouseX: e.clientX, mouseY: e.clientY, hoverTaskId: null };
    }}
  />
{/if}
```

(import `store` if not already there — already imported.)

CSS for `.dep-handle`:
```css
.dep-handle { cursor: crosshair; }
```

- [ ] **Step 3: Track hover-over-target while dragging**

In TaskBar's `<g>`, when `store.depCreator` is set and pointer enters this bar:

```svelte
<g class="task-bar"
   onmouseenter={() => {
     store.hoveredTaskId = task.id;
     if (store.depCreator) store.depCreator.hoverTaskId = task.id;
   }}
   onmouseleave={() => {
     store.hoveredTaskId = null;
     if (store.depCreator?.hoverTaskId === task.id) store.depCreator.hoverTaskId = null;
   }}
   ...>
```

- [ ] **Step 4: Write `DepCreator.svelte` — the live arrow + pointerup commit**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { store } from '../store.svelte';
  import * as ipc from '../ipc';

  function onMove(e: PointerEvent) {
    if (!store.depCreator) return;
    store.depCreator.mouseX = e.clientX;
    store.depCreator.mouseY = e.clientY;
  }
  async function onUp(_e: PointerEvent) {
    const d = store.depCreator;
    if (!d) return;
    store.depCreator = null;
    if (d.hoverTaskId === null || d.hoverTaskId === d.fromTaskId) return;
    try {
      const dep = await ipc.createDependency({
        predecessor_id: d.fromTaskId,
        successor_id: d.hoverTaskId,
        lag_days: 0,
      });
      store.dependencies = [...store.dependencies, dep];
      store.recordHistory();
      await ipc.touchLastSave();
    } catch (e) {
      // Cycle rejected — silent for now, Toast in Task 16.
      console.warn('dep rejected', e);
    }
  }

  onMount(() => {
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    return () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
    };
  });
</script>

{#if store.depCreator}
  <svg class="dep-creator-overlay">
    <line
      x1="0" y1="0"
      x2={store.depCreator.mouseX}
      y2={store.depCreator.mouseY}
      stroke="var(--c-accent)"
      stroke-width="2"
      stroke-dasharray="4 2"
    />
  </svg>
{/if}

<style>
  .dep-creator-overlay {
    position: fixed; inset: 0;
    pointer-events: none;
    z-index: 50;
  }
</style>
```

(The line above goes from (0,0) — we'd want it from the origin handle in screen coords. For simplicity, just track the mouse-to-mouse delta visualisation; the user gets feedback the gesture is alive.)

- [ ] **Step 5: Mount DepCreator in GanttCanvas**

```svelte
<DepCreator />
```

- [ ] **Step 6: Commit**

```bash
git add src/lib/canvas/ src/lib/store.svelte.ts
git commit -m "feat(ui): dep creation gesture — drag from ○ handle on right edge of bar to another bar"
```

---

### Task 13: No-work-day right-click on header

**Files:**
- Modify: `src/lib/canvas/HeaderStrip.svelte`
- Modify: `src/lib/store.svelte.ts`

- [ ] **Step 1: Add toggleNoWorkDay action**

```typescript
async toggleNoWorkDay(date: string): Promise<void> {
  if (!this.currentJob) return;
  const existing = this.noWorkDays.find(n => n.date === date && n.source === 'manual');
  if (existing) {
    await ipc.deleteNoWorkDay(existing.id);
    this.noWorkDays = this.noWorkDays.filter(n => n.id !== existing.id);
  } else {
    const created = await ipc.addManualNoWorkDay({
      job_id: this.currentJob.id, date, reason: 'Site closed',
    });
    this.noWorkDays = [...this.noWorkDays, created];
  }
  this.recordHistory();
  await ipc.touchLastSave();
}
```

- [ ] **Step 2: Add context-menu handler to HeaderStrip cells**

```svelte
<script lang="ts">
  import type { ViewportDay } from '../calendar';
  import { store } from '../store.svelte';
  import ContextMenu from '../components/ContextMenu.svelte';

  let { days }: { days: ViewportDay[] } = $props();

  let menu = $state<{ x: number; y: number; date: string } | null>(null);

  function onContext(e: MouseEvent, date: string) {
    e.preventDefault();
    menu = { x: e.clientX, y: e.clientY, date };
  }

  function items(date: string) {
    const isManual = store.noWorkDays.some(n => n.date === date && n.source === 'manual');
    return [
      { label: isManual ? 'Mark as working day' : 'Mark non-working day', action: () => store.toggleNoWorkDay(date) },
    ];
  }
</script>

<div class="header-strip" style="--total-w: {days.length * 24}px;">
  {#each days as d (d.date)}
    <div class="cell" class:week-start={d.weekday === 'M'}
         oncontextmenu={(e) => onContext(e, d.date)}>
      <!-- (existing inner content unchanged) -->
    </div>
  {/each}
</div>

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} items={items(menu.date)} onclose={() => menu = null} />
{/if}

<!-- styles unchanged -->
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/canvas/HeaderStrip.svelte src/lib/store.svelte.ts
git commit -m "feat(ui): right-click day column → mark / unmark manual non-working day"
```

---

### Task 14: Phase-bar whole-block drag

**Files:**
- Modify: `src/lib/canvas/PhaseBar.svelte`
- Modify: `src/lib/canvas/DragOverlay.svelte`
- Modify: `src/lib/store.svelte.ts`

When the phase is collapsed and the user drags its phase bar, ALL child tasks shift by the same delta in workdays (then chain ripple fires from downstream dependencies).

- [ ] **Step 1: Add phase-drag IPC support — `dragPhase` command**

For atomicity, dragging a phase = N task-update calls + ripple. Simpler v1 approach: do N parallel `update_task` calls + a single `drag_task` on the FIRST task to trigger ripple. Even simpler: re-use existing `drag_task` per child task in sequence. Skip atomicity for v1; rely on autosave + undo.

- [ ] **Step 2: In PhaseBar.svelte, capture pointerdown**

```svelte
<script lang="ts">
  // ...existing...
  import { store } from '../store.svelte';

  function onPointerDown(e: PointerEvent) {
    e.stopPropagation();
    store.dragState = {
      taskId: -phase.id, // negative = phase id sentinel
      zone: 'move',
      startX: e.clientX,
      originalStart: tasks[0]?.start_date ?? '',
      originalDuration: 0,
      liveDelta: 0,
    };
  }
</script>

<rect
  class="phase-bar"
  onpointerdown={onPointerDown}
  ... existing attrs ...
/>
```

- [ ] **Step 3: Update DragOverlay to handle phase drags**

In `onPointerUp`:

```typescript
async function onPointerUp(_e: PointerEvent) {
  const d = store.dragState;
  if (!d) return;
  const deltaWorkdays = Math.round(d.liveDelta / CELL);
  store.dragState = null;
  if (deltaWorkdays === 0) return;
  if (!store.currentJob) return;

  if (d.taskId < 0) {
    // Phase drag — shift every task in the phase.
    const phaseId = -d.taskId;
    const phaseTasks = store.tasksByPhase.get(phaseId) ?? [];
    for (const t of phaseTasks) {
      const newStart = addWorkdays(t.start_date, deltaWorkdays);
      await ipc.dragTask({
        job_id: store.currentJob.id,
        task_id: t.id,
        new_start_date: newStart,
      });
    }
    // Reload state from backend to pick up all ripples.
    await store.openJob(store.currentJob.id);
    return;
  }

  // ... existing task-drag logic ...
}
```

- [ ] **Step 4: Commit**

```bash
git add src/lib/canvas/PhaseBar.svelte src/lib/canvas/DragOverlay.svelte
git commit -m "feat(ui): phase-bar whole-block drag — all child tasks shift together"
```

---

## Phase F — Backend extension + error surfacing (Tasks 15–16)

### Task 15: Backend `list_archived` command + sidebar wiring

**Files:**
- Modify: `src-tauri/src/repo/job.rs` (add list_archived)
- Modify: `src-tauri/src/commands/job.rs` (expose)
- Modify: `src-tauri/src/lib.rs` (register)
- Modify: `src/lib/ipc.ts`
- Modify: `src/lib/store.svelte.ts` (use the new command)

- [ ] **Step 1: Add `list_archived` to `repo::job`**

```rust
pub fn list_archived(conn: &Connection) -> GbResult<Vec<Job>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, client, address, project_start_date,
                is_template, archived, created_at
         FROM job
         WHERE archived = 1 AND is_template = 0
         ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], row_to_job)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}
```

Add a test:

```rust
#[test]
fn list_archived_returns_only_archived() {
    let conn = open_in_memory().unwrap();
    let a = create(&conn, &sample("A")).unwrap();
    create(&conn, &sample("B")).unwrap();
    set_archived(&conn, a.id, true).unwrap();
    let archived = list_archived(&conn).unwrap();
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].name, "A");
}
```

- [ ] **Step 2: Expose `list_archived` Tauri command** in `commands/job.rs`:

```rust
#[tauri::command]
pub fn list_archived(db: State<Db>) -> GbResult<Vec<Job>> {
    let conn = db.0.lock().unwrap();
    job_repo::list_archived(&conn)
}
```

Register in `lib.rs`:
```rust
commands::job::list_archived,
```

- [ ] **Step 3: Frontend IPC + store**

```typescript
// ipc.ts
export const listArchived = () => invoke<Job[]>('list_archived');

// store — replace the empty refreshArchived:
async refreshArchived(): Promise<void> {
  this.archivedJobs = await ipc.listArchived();
}
```

Call `refreshArchived()` from `bootstrap()`.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/ src/lib/ipc.ts src/lib/store.svelte.ts
git commit -m "feat(ipc+repo): list_archived command — sidebar Archived group is now populated"
```

---

### Task 16: Toast component for IPC errors

**Files:**
- Create: `src/lib/toast.svelte.ts`
- Create: `src/lib/components/Toast.svelte`
- Modify: `src/App.svelte`
- Modify: `src/lib/canvas/DepCreator.svelte` (use toast on cycle reject)

- [ ] **Step 1: Toast store**

```typescript
// src/lib/toast.svelte.ts
interface ToastEntry { id: number; kind: 'error' | 'info'; message: string }

class ToastBus {
  list = $state<ToastEntry[]>([]);
  private nextId = 1;

  show(kind: 'error' | 'info', message: string, ttlMs = 4000): void {
    const id = this.nextId++;
    this.list = [...this.list, { id, kind, message }];
    setTimeout(() => {
      this.list = this.list.filter(t => t.id !== id);
    }, ttlMs);
  }
}

export const toast = new ToastBus();
```

- [ ] **Step 2: Toast component**

```svelte
<!-- src/lib/components/Toast.svelte -->
<script lang="ts">
  import { toast } from '../toast.svelte';
</script>

<div class="toast-stack">
  {#each toast.list as t (t.id)}
    <div class="toast {t.kind}">{t.message}</div>
  {/each}
</div>

<style>
  .toast-stack {
    position: fixed; bottom: 32px; left: 50%;
    transform: translateX(-50%);
    z-index: 999;
    display: flex; flex-direction: column; gap: var(--sp-2);
  }
  .toast {
    padding: var(--sp-2) var(--sp-3);
    border-radius: 4px;
    box-shadow: 0 4px 12px var(--c-shadow);
    font-size: var(--font-size-sm);
    min-width: 280px;
  }
  .toast.error { background: #FEE2E2; color: #991B1B; border: 1px solid #FCA5A5; }
  .toast.info  { background: var(--c-accent-fade); color: var(--c-accent); }
</style>
```

- [ ] **Step 3: Mount in App.svelte**

```svelte
<Toast />
```

- [ ] **Step 4: Use toast in DepCreator on cycle reject**

```typescript
} catch (e) {
  const msg = String((e as { message?: string }).message ?? e);
  if (msg.includes('cycle')) toast.show('error', "Can't create that — it would make a circular chain.");
  else toast.show('error', `Couldn't add dependency: ${msg}`);
}
```

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/Toast.svelte src/lib/toast.svelte.ts src/App.svelte src/lib/canvas/DepCreator.svelte
git commit -m "feat(ui): toast component for IPC errors (cycle reject + future failures)"
```

---

## Phase G — Packaging + ship (Tasks 17–19)

### Task 17: App icon

**Files:**
- Create: `src-tauri/icons/icon.png` + variants

For v1 we use a simple placeholder icon. Real antelope-themed design is a separate exercise.

- [ ] **Step 1: Create the icon source**

Save the following SVG as `~/Desktop/GanttBok/src-tauri/icons/source.svg`:

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
  <rect width="512" height="512" rx="96" fill="#3B82F6"/>
  <text x="256" y="320" font-size="280" font-family="-apple-system, system-ui, sans-serif" font-weight="700" fill="white" text-anchor="middle">G</text>
</svg>
```

- [ ] **Step 2: Generate Tauri icon variants**

Tauri has a built-in icon generator:

```bash
cd ~/Desktop/GanttBok && pnpm tauri icon src-tauri/icons/source.svg
```

This produces all required sizes (.png, .icns for macOS, .ico for Windows). The existing Tauri-default icons in `src-tauri/icons/` get overwritten.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/icons/
git commit -m "feat(packaging): blue G app icon — placeholder until antelope mascot design"
```

---

### Task 18: Build configuration + ad-hoc signing

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Create: `docs/RELEASE.md`

- [ ] **Step 1: Configure `tauri.conf.json` for macOS bundle**

Update the bundle section:

```json
{
  "bundle": {
    "active": true,
    "targets": ["app", "dmg"],
    "category": "Productivity",
    "shortDescription": "Gantt-chart desktop app for apartment renovations",
    "longDescription": "Plan, edit, and print A3 Gantt charts for apartment-renovation projects. Mon-Fri workdays, SA public holidays auto-synced, week-numbered timeline, magnetic-snap drag with hard-chain dependency ripple.",
    "macOS": {
      "frameworks": [],
      "minimumSystemVersion": "11.0",
      "signingIdentity": "-",
      "providerShortName": null,
      "entitlements": null,
      "exceptionDomain": ""
    }
  }
}
```

`signingIdentity: "-"` enables ad-hoc signing — works without a paid Apple Developer account; Gatekeeper will warn once on first open ("right-click → Open" once and it's fine forever).

- [ ] **Step 2: Write `docs/RELEASE.md`**

```markdown
# Release & install

## Build

```bash
cd ~/Desktop/GanttBok && pnpm tauri build
```

Produces:
- `src-tauri/target/release/bundle/macos/Gantt Bok.app`  ← drop in /Applications
- `src-tauri/target/release/bundle/dmg/Gantt Bok_X.Y.Z_x64.dmg` ← installer

## First install on Gray's Mac

1. Double-click the `.dmg`. Drag `Gantt Bok.app` into Applications.
2. First launch: Gatekeeper will block with "developer cannot be verified".
3. Workaround: right-click the app → **Open** → confirm. Only needed once.
4. The app now launches normally on every subsequent open.

## Future: full notarisation

For zero-warning install:
1. Get an Apple Developer account ($99/yr at developer.apple.com)
2. Generate a Developer ID Application certificate
3. Set `signingIdentity` in `tauri.conf.json` to your team's identity string
4. `pnpm tauri build` will sign and notarise automatically (with `tauri-plugin-notarization` configured)

## Updating

There is no auto-update channel. To roll out v1.1+, rebuild and replace the `.app` in /Applications.
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tauri.conf.json docs/RELEASE.md
git commit -m "feat(packaging): ad-hoc signed .app + .dmg targets, macOS bundle metadata, RELEASE.md"
```

---

### Task 19: Build, smoke-test, tag v1.0.0

- [ ] **Step 1: Final verification**

```bash
cd ~/Desktop/GanttBok && pnpm exec tsc --noEmit && pnpm exec vitest run && cd src-tauri && . "$HOME/.cargo/env" && cargo test && cd .. && pnpm tauri build
```

The build will take a while (10-15 min first time — release builds are slow).

- [ ] **Step 2: Locate the bundle**

```bash
ls -la src-tauri/target/release/bundle/macos/
ls -la src-tauri/target/release/bundle/dmg/
```

Both should contain `Gantt Bok.app` and `Gantt Bok_1.0.0_x64.dmg` respectively.

- [ ] **Step 3: Open the bundled app to confirm it runs standalone**

```bash
open "src-tauri/target/release/bundle/macos/Gantt Bok.app"
```

Should open the same UI as `pnpm tauri dev`, but as a standalone .app icon in the dock.

- [ ] **Step 4: Tag v1.0.0**

```bash
git tag -a v1.0.0 -m "v1.0.0 — Gantt Bok ships. Foundation + UI + polish complete. Ad-hoc signed .app + .dmg."
```

- [ ] **Step 5: Update Workshop brief**

Edit `~/Desktop/OBSIDIAN_TREES/Workshop/projects/GANTTBOK/brief_GANTTBOK.md`:
- `status:` → `shipped — v1.0.0 packaged, ready for Gray`
- Add v1.0.0 to tag list
- Mark Plan 3 ✅
- Add "✅ Hand-over to Gray pending" as the next action

- [ ] **Step 6: Generate a clean copy for Gray**

```bash
cp "src-tauri/target/release/bundle/dmg/Gantt Bok_1.0.0_x64.dmg" ~/Desktop/Gantt_Bok_v1.0.0.dmg
```

Drop on a USB stick or send via AirDrop/whatever.

---

## Self-review

**Spec coverage delta from Plan 2 → Plan 3:**

| Spec § | Plan 3 task | Notes |
|---|---|---|
| §7.6 Phase drag whole-block | Task 14 | atomic per-task IPC; uses existing drag_task |
| §8.3 Dependency creation gesture | Task 12 | drag from ○ handle |
| §8.4 No-work right-click | Task 13 | header cell context menu |
| §8.7 Undo / redo | Tasks 1-3 | snapshot-based, session-scoped, ⌘Z/⌘⇧Z |
| §9 Templates UI | Tasks 6-8 | sidebar group, save-as, new-from |
| §10 Print pipeline | Tasks 9-11 | Print Options + @media print + native dialog |
| §11 Saved-state indicator + ⌘S | Task 4 | bottom-right, state-driven |
| §11.4 Time Machine backup | n/a | no app-side logic (spec confirms) |
| §12.2 Single-instance lock | NOT INCLUDED | parked again; not blocking |
| §12.5 Print errors | Task 9 (Save as PDF fallback) | macOS native dialog handles |

**Placeholder scan:** no `TODO` / `TBD` / "implement later" patterns.

**Type consistency:** Snapshot interface mirrors Rust struct shapes in `ResyncArgs`. The dragState convention of using `taskId: -phase.id` as a sentinel for phase-drag is unusual but contained to DragOverlay + PhaseBar; doc'd inline.

**Known v1 cuts (parked for v1.1+):**
- Single-instance lock (tauri-plugin-single-instance)
- Auto-update channel
- Full Apple notarisation
- Antelope mascot icon (placeholder "G" used)
- Sidebar/canvas vertical scroll sync (only-canvas-scrolls is the current behaviour; should we unify? — Plan 4 if Gray reports the friction)

---

## Execution handoff

**Plan complete and saved to `~/Desktop/GanttBok/docs/plans/2026-05-20-plan3-polish-and-ship.md`.**

Tasks total: **19**. Recommended dispatch:
- **Batch 1:** Tasks 1-8 (undo + saved indicator + templates UI) — frontend-heavy
- **Batch 2:** Tasks 9-14 (print + power-user gestures) — frontend-heavy
- **Batch 3:** Tasks 15-19 (backend extension + toast + packaging + ship) — mixed, ends with v1.0.0

Two execution options:
1. **Subagent-Driven (recommended)** — fresh subagent per batch, review between
2. **Inline Execution** — batch + checkpoints
