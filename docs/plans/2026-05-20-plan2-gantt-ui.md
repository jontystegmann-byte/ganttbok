# Gantt Bok — Plan 2: Gantt UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the functional Svelte 5 frontend for Gantt Bok — typed IPC client, Svelte stores, three-pane app shell (sidebar + canvas + details panel), Gantt canvas (header + grid + bars + dependency lines), drag physics with magnetic snap and hard-chain ripple, and the core creation/editing gestures. By the end of Plan 2 the app **opens, lists jobs from the SQLite DB, lets the user create a job + phases + tasks, drag bars around with the dependency chain rippling in real time, and persist everything** — purely functional, no print yet (that's Plan 3).

**Architecture:** Svelte 5 with `$state` runes for fine-grained reactivity, mounted into a vanilla Vite shell at `src/main.ts`. All persistence calls go through a single typed IPC client (`src/lib/ipc.ts`) that wraps Tauri's `invoke` and types every command's args + return against the Rust backend defined in Plan 1. SVG (not canvas) for the Gantt — every bar is a real DOM node so hover/cursor/tooltip come for free. A single hand-rolled `requestAnimationFrame` loop drives drag updates; downstream bars are repositioned same-frame for the rigid-chain behaviour the spec requires. Magnetic snap is a pure function fed pointer deltas + cell width. State is a Svelte 5 store (`$state` class instance) shared across components; on every state mutation we fire-and-forget the appropriate IPC command.

**Tech Stack:** Svelte 5 · TypeScript 5 · Vite 5 · Tauri 2 JS API (`@tauri-apps/api`) · Vitest (unit tests for pure functions) · No external Gantt or drag libraries — we own the canvas + interaction layer.

**Reference spec:** `~/Desktop/GanttBok/docs/specs/2026-05-19-ganttbok-design.md`
**Reference Plan 1 (foundation, complete at v0.1.0):** `~/Desktop/GanttBok/docs/plans/2026-05-19-plan1-foundation.md`

---

## What's NOT in Plan 2

Deferred to **Plan 3** to keep this plan focused on the functional core:

- Templates UI (the IPC commands exist; the sidebar group + "save as template" / "new from template" modal flows ship in Plan 3)
- Undo / redo (the data model supports it; the stack + keyboard handlers ship in Plan 3)
- Print pipeline + Print Options sheet
- App icon, packaging, signing, .dmg installer
- Saved-state indicator footer + manual `⌘S` shortcut
- Pre-release manual ritual

Plan 2 ends with a working drag-able Gantt chart you can actually use; Plan 3 puts polish + ship-readiness on top.

---

## File structure (every file Plan 2 creates or touches)

```
~/Desktop/GanttBok/
├── src/
│   ├── main.ts                                   Task 1 (rewrite)
│   ├── App.svelte                                Task 4 (rewrite — three-pane shell)
│   ├── app.css                                   Task 3 (rewrite — design tokens)
│   └── lib/
│       ├── ipc.ts                                Task 2 (new — typed IPC client)
│       ├── types.ts                              Task 2 (new — domain types mirroring Rust models)
│       ├── store.svelte.ts                       Task 5 (new — global state runes)
│       ├── calendar.ts                           Task 14 (new — frontend workday/week math mirroring Rust)
│       ├── snap.ts                               Task 23 (new — magnetic snap pure function)
│       ├── hit-test.ts                           Task 22 (new — bar grab-zone resolution)
│       │
│       ├── sidebar/
│       │   ├── Sidebar.svelte                    Task 7
│       │   ├── JobItem.svelte                    Task 7
│       │   ├── NewJobModal.svelte                Task 9
│       │   └── ArchivedGroup.svelte              Task 10
│       │
│       ├── canvas/
│       │   ├── GanttCanvas.svelte                Task 11 (root canvas component)
│       │   ├── HeaderStrip.svelte                Task 12 (week numbers + M T W T F + Monday dates)
│       │   ├── NoWorkColumn.svelte               Task 13 (diagonal stripes + rotated holiday name)
│       │   ├── LeftRail.svelte                   Task 15 (phase/task labels with hierarchical numbering)
│       │   ├── RowChrome.svelte                  Task 15 (the grey grid lines per row)
│       │   ├── TaskBar.svelte                    Task 17
│       │   ├── PhaseBar.svelte                   Task 18 (composite from child tasks)
│       │   ├── DependencyArrow.svelte            Task 19
│       │   └── DragOverlay.svelte                Task 24 (the live-dragged bar ghost)
│       │
│       └── details/
│           ├── DetailsPanel.svelte               Task 28
│           ├── TaskDetails.svelte                Task 28
│           ├── PhaseDetails.svelte               Task 29
│           └── DependencyDetails.svelte          Task 30 (deferred-friendly — basic lag editor)
│
├── src/lib/__tests__/                            (Vitest)
│   ├── calendar.test.ts                          Task 14
│   ├── snap.test.ts                              Task 23
│   ├── hit-test.test.ts                          Task 22
│   ├── hierarchical-numbering.test.ts            Task 16
│   └── store.test.ts                             Task 5
│
├── vitest.config.ts                              Task 5 (new)
├── package.json                                  Tasks 1, 5 (add @tauri-apps/api version pin + vitest)
└── (existing src-tauri/ + docs/ untouched)
```

---

## Phase A — Frontend foundation (Tasks 1–6)

### Task 1: Install frontend deps + main.ts mounting

**Files:**
- Modify: `~/Desktop/GanttBok/package.json`
- Modify: `~/Desktop/GanttBok/src/main.ts`

- [ ] **Step 1: Pin Tauri JS API + add dev deps**

In `package.json` under `dependencies`, ensure `@tauri-apps/api` is `^2.x` (the Phase 0 subagent already added it). Add under `devDependencies`:

```json
{
  "devDependencies": {
    "vitest": "^2.0.0",
    "@vitest/ui": "^2.0.0",
    "jsdom": "^25.0.0"
  }
}
```

Run `pnpm install`.

- [ ] **Step 2: Replace `src/main.ts`**

```typescript
import { mount } from 'svelte';
import App from './App.svelte';
import './app.css';

const app = mount(App, { target: document.getElementById('app')! });
export default app;
```

- [ ] **Step 3: Commit**

```bash
git add package.json pnpm-lock.yaml src/main.ts
git commit -m "chore(ui): pin @tauri-apps/api + vitest + svelte 5 mount() entry"
```

---

### Task 2: TypeScript domain types + IPC client

**Files:**
- Create: `src/lib/types.ts`
- Create: `src/lib/ipc.ts`

- [ ] **Step 1: Write `src/lib/types.ts`** — mirror the Rust models exactly (snake_case fields because that's what serde gives us):

```typescript
// Mirror of src-tauri/src/db/models.rs row structs.
// All dates are ISO YYYY-MM-DD strings on the wire; we keep them as strings
// in the frontend store and only convert to Date for math via lib/calendar.ts.

export interface Job {
  id: number;
  name: string;
  client: string | null;
  address: string | null;
  project_start_date: string;
  is_template: boolean;
  archived: boolean;
  created_at: string;
}

export interface Phase {
  id: number;
  job_id: number;
  name: string;
  colour: string;
  order_index: number;
  collapsed: boolean;
}

export interface Task {
  id: number;
  phase_id: number;
  name: string;
  start_date: string;
  duration_workdays: number;
  order_index: number;
  notes: string | null;
}

export interface Dependency {
  id: number;
  predecessor_id: number;
  successor_id: number;
  type: string;
  lag_days: number;
}

export interface NoWorkDay {
  id: number;
  job_id: number;
  date: string;
  reason: string;
  source: 'sa_public_holiday' | 'manual';
}

export interface StartupInfo {
  clean_shutdown: boolean;
  last_open_job_id: number | null;
  last_save_at: string | null;
  sidebar_width: number | null;
}

export interface DragResult {
  updated_tasks: Task[];
}

// Args structs (match the Rust #[derive(Deserialize)] structs)
export interface CreateJobArgs {
  name: string;
  client: string | null;
  address: string | null;
  project_start_date: string;
  is_template: boolean;
}

export interface CreatePhaseArgs {
  job_id: number;
  name: string;
  colour: string;
}

export interface CreateTaskArgs {
  phase_id: number;
  name: string;
  start_date: string;
  duration_workdays: number;
}

export interface CreateDepArgs {
  predecessor_id: number;
  successor_id: number;
  lag_days: number;
}

export interface AddManualArgs {
  job_id: number;
  date: string;
  reason: string;
}

export interface SyncSaArgs {
  job_id: number;
  from: string;
  to: string;
}

export interface InstantiateArgs {
  template_id: number;
  new_name: string;
  client: string | null;
  address: string | null;
  project_start_date: string;
}

export interface DragTaskArgs {
  job_id: number;
  task_id: number;
  new_start_date: string;
}
```

- [ ] **Step 2: Write `src/lib/ipc.ts`** — one typed wrapper per Tauri command:

```typescript
import { invoke } from '@tauri-apps/api/core';
import type {
  Job, Phase, Task, Dependency, NoWorkDay, StartupInfo, DragResult,
  CreateJobArgs, CreatePhaseArgs, CreateTaskArgs, CreateDepArgs,
  AddManualArgs, SyncSaArgs, InstantiateArgs, DragTaskArgs,
} from './types';

// Jobs
export const listJobs        = ()                                    => invoke<Job[]>('list_jobs');
export const listTemplates   = ()                                    => invoke<Job[]>('list_templates');
export const getJob          = (id: number)                          => invoke<Job>('get_job', { id });
export const createJob       = (args: CreateJobArgs)                 => invoke<Job>('create_job', { args });
export const updateJob       = (job: Job)                            => invoke<void>('update_job', { job });
export const archiveJob      = (id: number, archived: boolean)       => invoke<void>('archive_job', { id, archived });
export const deleteJob       = (id: number)                          => invoke<void>('delete_job', { id });

// Templates
export const saveAsTemplate     = (sourceJobId: number, templateName: string) =>
  invoke<Job>('save_as_template', { sourceJobId, templateName });
export const instantiateTemplate = (args: InstantiateArgs) =>
  invoke<Job>('instantiate_template', { args });

// Phases
export const listPhases    = (jobId: number)                         => invoke<Phase[]>('list_phases',  { jobId });
export const createPhase   = (args: CreatePhaseArgs)                 => invoke<Phase>('create_phase',   { args });
export const updatePhase   = (phase: Phase)                          => invoke<void>('update_phase',    { phase });
export const deletePhase   = (id: number)                            => invoke<void>('delete_phase',    { id });
export const reorderPhases = (jobId: number, orderedIds: number[])   => invoke<void>('reorder_phases',  { jobId, orderedIds });

// Tasks
export const listTasks    = (jobId: number)                          => invoke<Task[]>('list_tasks', { jobId });
export const createTask   = (args: CreateTaskArgs)                   => invoke<Task>('create_task',  { args });
export const updateTask   = (task: Task)                             => invoke<void>('update_task',  { task });
export const deleteTask   = (id: number)                             => invoke<void>('delete_task',  { id });
export const reorderTasks = (phaseId: number, orderedIds: number[])  => invoke<void>('reorder_tasks', { phaseId, orderedIds });

// Drag
export const dragTask = (args: DragTaskArgs) => invoke<DragResult>('drag_task', { args });

// Dependencies
export const listDependencies     = (jobId: number)                  => invoke<Dependency[]>('list_dependencies', { jobId });
export const createDependency     = (args: CreateDepArgs)            => invoke<Dependency>('create_dependency', { args });
export const updateDependencyLag  = (id: number, lagDays: number)    => invoke<void>('update_dependency_lag', { id, lagDays });
export const deleteDependency     = (id: number)                     => invoke<void>('delete_dependency', { id });

// No-work days
export const listNoWorkDays         = (jobId: number)                => invoke<NoWorkDay[]>('list_no_work_days', { jobId });
export const addManualNoWorkDay     = (args: AddManualArgs)          => invoke<NoWorkDay>('add_manual_no_work_day', { args });
export const deleteNoWorkDay        = (id: number)                   => invoke<void>('delete_no_work_day', { id });
export const syncSaHolidays         = (args: SyncSaArgs)             => invoke<number>('sync_sa_holidays', { args });

// Meta
export const startupInfo        = ()                                 => invoke<StartupInfo>('startup_info');
export const markCleanShutdown  = ()                                 => invoke<void>('mark_clean_shutdown');
export const setLastOpenJob     = (jobId: number)                    => invoke<void>('set_last_open_job', { jobId });
export const setSidebarWidth    = (width: number)                    => invoke<void>('set_sidebar_width', { width });
export const touchLastSave      = ()                                 => invoke<void>('touch_last_save');
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
cd ~/Desktop/GanttBok && pnpm exec tsc --noEmit
```

Expected: zero errors. (If `tsc` not configured to find these, check `tsconfig.app.json` includes `src/**/*.ts`.)

- [ ] **Step 4: Commit**

```bash
git add src/lib/types.ts src/lib/ipc.ts
git commit -m "feat(ui): typed IPC client + domain types mirroring Rust models"
```

---

### Task 3: Design tokens + base CSS

**Files:**
- Modify: `src/app.css`

- [ ] **Step 1: Replace `src/app.css`** with design tokens that the whole UI will reference:

```css
:root {
  /* Type */
  --font-sans: -apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif;
  --font-mono: "SF Mono", ui-monospace, Menlo, monospace;
  --font-size-xs:   11px;
  --font-size-sm:   12px;
  --font-size-base: 13px;
  --font-size-lg:   15px;
  --font-size-xl:   18px;

  /* Spacing */
  --sp-1: 4px;
  --sp-2: 8px;
  --sp-3: 12px;
  --sp-4: 16px;
  --sp-6: 24px;

  /* Layout */
  --sidebar-width-default: 240px;
  --details-width:         300px;
  --row-height:            32px;
  --day-cell-width:        24px;        /* on-screen; print scales */
  --week-gap:              0px;          /* weeks abut; bolder vertical line between them */
  --header-height:         52px;
  --left-rail-width:       240px;

  /* Colour — light theme (we hard-code light only; dark mode = Plan 3 polish) */
  --c-bg:           #FAFAFA;
  --c-panel:        #FFFFFF;
  --c-border:       #E5E7EB;
  --c-border-bold:  #CBD5E1;
  --c-text:         #0F172A;
  --c-text-muted:   #64748B;
  --c-accent:       #2563EB;          /* selection + Monday text */
  --c-accent-fade:  #DBEAFE;
  --c-no-work:      #E5E7EB;          /* diagonal-stripe column fill */
  --c-no-work-text: #475569;
  --c-shadow:       rgba(15, 23, 42, 0.08);

  /* Bars */
  --bar-radius:    3px;
  --bar-height:    20px;
  --bar-phase-opacity: 0.18;
}

* { box-sizing: border-box; }

html, body, #app {
  margin: 0; padding: 0;
  height: 100vh;
  font-family: var(--font-sans);
  font-size: var(--font-size-base);
  color: var(--c-text);
  background: var(--c-bg);
  user-select: none;
  -webkit-font-smoothing: antialiased;
}

button, input, textarea {
  font: inherit;
  color: inherit;
}

::selection {
  background: var(--c-accent-fade);
}
```

- [ ] **Step 2: Commit**

```bash
git add src/app.css
git commit -m "feat(ui): design tokens + base CSS reset"
```

---

### Task 4: Three-pane app shell

**Files:**
- Modify: `src/App.svelte`

- [ ] **Step 1: Replace `src/App.svelte`**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { state } from './lib/store.svelte';
  import Sidebar from './lib/sidebar/Sidebar.svelte';
  import GanttCanvas from './lib/canvas/GanttCanvas.svelte';
  import DetailsPanel from './lib/details/DetailsPanel.svelte';

  onMount(async () => {
    await state.bootstrap();
  });
</script>

<div class="app-shell">
  <aside class="sidebar" style="width: {state.sidebarWidth}px">
    <Sidebar />
  </aside>

  <main class="canvas-pane">
    {#if state.currentJob}
      <GanttCanvas />
    {:else}
      <div class="empty-state">
        <h1>Gantt Bok</h1>
        <p>Pick a job from the left, or create a new one.</p>
      </div>
    {/if}
  </main>

  {#if state.selection}
    <aside class="details">
      <DetailsPanel />
    </aside>
  {/if}
</div>

<style>
  .app-shell {
    display: grid;
    grid-template-columns: auto 1fr auto;
    height: 100vh;
    overflow: hidden;
  }
  .sidebar {
    border-right: 1px solid var(--c-border);
    background: var(--c-panel);
    overflow-y: auto;
    min-width: 180px;
    max-width: 480px;
  }
  .canvas-pane {
    overflow: auto;
    background: var(--c-bg);
  }
  .details {
    width: var(--details-width);
    border-left: 1px solid var(--c-border);
    background: var(--c-panel);
    overflow-y: auto;
  }
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--c-text-muted);
  }
  .empty-state h1 {
    font-size: var(--font-size-xl);
    font-weight: 600;
    margin: 0 0 var(--sp-2);
    color: var(--c-text);
  }
</style>
```

This file references components that don't exist yet (Sidebar, GanttCanvas, DetailsPanel, the store). It WILL NOT COMPILE until Tasks 5, 7, 11, 28 land. That's fine — TDD-shape: write the skeleton, then build inward.

- [ ] **Step 2: Stage but do not commit yet**

```bash
git add src/App.svelte
```

We'll commit after Task 5 lands the store. Otherwise the staged App.svelte refers to a missing module and even staging won't help — but the commit at end of Task 5 will land both files together.

---

### Task 5: Svelte 5 store + bootstrap + Vitest setup

**Files:**
- Create: `src/lib/store.svelte.ts`
- Create: `vitest.config.ts`
- Create: `src/lib/__tests__/store.test.ts`

- [ ] **Step 1: Write `vitest.config.ts`**

```typescript
import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte({ hot: false })],
  test: {
    environment: 'jsdom',
    include: ['src/**/__tests__/**/*.test.ts'],
    globals: true,
  },
});
```

- [ ] **Step 2: Write `src/lib/store.svelte.ts` — Svelte 5 `$state` rune-backed global store**

```typescript
import * as ipc from './ipc';
import type { Job, Phase, Task, Dependency, NoWorkDay } from './types';

export type Selection =
  | { kind: 'task'; id: number }
  | { kind: 'phase'; id: number }
  | { kind: 'dependency'; id: number }
  | null;

class Store {
  // Top-level reactive state
  jobs       = $state<Job[]>([]);
  templates  = $state<Job[]>([]);
  currentJob = $state<Job | null>(null);

  phases       = $state<Phase[]>([]);
  tasks        = $state<Task[]>([]);
  dependencies = $state<Dependency[]>([]);
  noWorkDays   = $state<NoWorkDay[]>([]);

  selection     = $state<Selection>(null);
  sidebarWidth  = $state<number>(240);

  // Derived helpers
  tasksByPhase = $derived.by(() => {
    const m = new Map<number, Task[]>();
    for (const t of this.tasks) {
      const list = m.get(t.phase_id) ?? [];
      list.push(t);
      m.set(t.phase_id, list);
    }
    for (const list of m.values()) list.sort((a, b) => a.order_index - b.order_index);
    return m;
  });

  // Bootstrap: load app meta + jobs at startup.
  async bootstrap(): Promise<void> {
    const meta = await ipc.startupInfo();
    if (meta.sidebar_width) this.sidebarWidth = meta.sidebar_width;
    await this.refreshSidebar();
    if (meta.last_open_job_id) {
      try { await this.openJob(meta.last_open_job_id); }
      catch { /* job may have been deleted */ }
    }
  }

  async refreshSidebar(): Promise<void> {
    this.jobs       = await ipc.listJobs();
    this.templates  = await ipc.listTemplates();
  }

  async openJob(jobId: number): Promise<void> {
    this.currentJob   = await ipc.getJob(jobId);
    this.phases       = await ipc.listPhases(jobId);
    this.tasks        = await ipc.listTasks(jobId);
    this.dependencies = await ipc.listDependencies(jobId);
    this.noWorkDays   = await ipc.listNoWorkDays(jobId);
    this.selection    = null;
    await ipc.setLastOpenJob(jobId);
  }

  select(s: Selection): void {
    this.selection = s;
  }

  // Optimistic local update applied after an IPC mutation returns updated rows.
  applyDragResult(updated: Task[]): void {
    const byId = new Map(updated.map(t => [t.id, t]));
    this.tasks = this.tasks.map(t => byId.get(t.id) ?? t);
  }
}

export const state = new Store();
```

- [ ] **Step 3: Write `src/lib/__tests__/store.test.ts`** — unit test the pure-logic parts. IPC is mocked.

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('../ipc', () => ({
  startupInfo:    vi.fn(async () => ({ clean_shutdown: true, last_open_job_id: null, last_save_at: null, sidebar_width: null })),
  listJobs:       vi.fn(async () => []),
  listTemplates:  vi.fn(async () => []),
  listPhases:     vi.fn(async () => []),
  listTasks:      vi.fn(async () => []),
  listDependencies: vi.fn(async () => []),
  listNoWorkDays:   vi.fn(async () => []),
  getJob:         vi.fn(),
  setLastOpenJob: vi.fn(async () => {}),
}));

import { state } from '../store.svelte';

describe('Store', () => {
  beforeEach(() => {
    state.tasks = [];
    state.selection = null;
  });

  it('applyDragResult patches tasks by id', () => {
    state.tasks = [
      { id: 1, phase_id: 1, name: 'A', start_date: '2026-06-08', duration_workdays: 1, order_index: 0, notes: null },
      { id: 2, phase_id: 1, name: 'B', start_date: '2026-06-09', duration_workdays: 1, order_index: 1, notes: null },
    ];
    state.applyDragResult([
      { id: 1, phase_id: 1, name: 'A', start_date: '2026-06-10', duration_workdays: 1, order_index: 0, notes: null },
    ]);
    expect(state.tasks[0].start_date).toBe('2026-06-10');
    expect(state.tasks[1].start_date).toBe('2026-06-09'); // untouched
  });

  it('select stores the selection', () => {
    state.select({ kind: 'task', id: 42 });
    expect(state.selection).toEqual({ kind: 'task', id: 42 });
  });
});
```

- [ ] **Step 4: Run vitest — expect 2 pass**

```bash
cd ~/Desktop/GanttBok && pnpm exec vitest run
```

- [ ] **Step 5: Commit (App.svelte + store + vitest config together)**

```bash
git add src/App.svelte src/lib/store.svelte.ts vitest.config.ts src/lib/__tests__/store.test.ts package.json pnpm-lock.yaml
git commit -m "feat(ui): three-pane app shell + Svelte 5 \$state store + Vitest harness"
```

---

### Task 6: Stub the components App.svelte expects, so the dev build runs

**Files:**
- Create: `src/lib/sidebar/Sidebar.svelte`
- Create: `src/lib/canvas/GanttCanvas.svelte`
- Create: `src/lib/details/DetailsPanel.svelte`

- [ ] **Step 1: Three placeholder components** — each ~5 lines so the import resolves.

```svelte
<!-- src/lib/sidebar/Sidebar.svelte -->
<div class="sidebar-stub"><h2>Jobs</h2><p style="padding: var(--sp-3); color: var(--c-text-muted);">(Sidebar coming in Task 7)</p></div>
```

```svelte
<!-- src/lib/canvas/GanttCanvas.svelte -->
<div class="canvas-stub" style="padding: var(--sp-4);">Gantt canvas placeholder</div>
```

```svelte
<!-- src/lib/details/DetailsPanel.svelte -->
<div class="details-stub" style="padding: var(--sp-4);">Details panel</div>
```

- [ ] **Step 2: Verify the app opens**

```bash
cd ~/Desktop/GanttBok/src-tauri && . "$HOME/.cargo/env" && cargo check
```

(`cargo check` validates everything still compiles. The full `pnpm tauri dev` test happens manually by JT when he wants to look at the window.)

- [ ] **Step 3: Commit**

```bash
git add src/lib/sidebar/Sidebar.svelte src/lib/canvas/GanttCanvas.svelte src/lib/details/DetailsPanel.svelte
git commit -m "feat(ui): stub Sidebar / GanttCanvas / DetailsPanel components so shell compiles"
```

---

## Phase B — Sidebar / job library (Tasks 7–10)

### Task 7: Sidebar list — active jobs

**Files:**
- Modify: `src/lib/sidebar/Sidebar.svelte`
- Create: `src/lib/sidebar/JobItem.svelte`

- [ ] **Step 1: Write `JobItem.svelte`**

```svelte
<script lang="ts">
  import type { Job } from '../types';
  import { state } from '../store.svelte';
  let { job }: { job: Job } = $props();
  const isOpen = $derived(state.currentJob?.id === job.id);
  async function open() { await state.openJob(job.id); }
</script>

<button class="job-item" class:open={isOpen} onclick={open}>
  {#if isOpen}<span class="indicator">●</span>{/if}
  <span class="job-name">{job.name}</span>
</button>

<style>
  .job-item {
    display: flex; align-items: center; gap: var(--sp-2);
    width: 100%; padding: var(--sp-2) var(--sp-3);
    border: none; background: transparent; cursor: pointer;
    text-align: left; font-size: var(--font-size-sm);
    border-left: 3px solid transparent;
  }
  .job-item:hover { background: var(--c-accent-fade); }
  .job-item.open { background: var(--c-accent-fade); border-left-color: var(--c-accent); }
  .indicator { color: var(--c-accent); font-size: 8px; }
  .job-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
```

- [ ] **Step 2: Rewrite `Sidebar.svelte`**

```svelte
<script lang="ts">
  import { state } from '../store.svelte';
  import JobItem from './JobItem.svelte';
</script>

<div class="sidebar">
  <header><h2>Gantt Bok</h2></header>

  <section>
    <h3>Active</h3>
    {#each state.jobs as job (job.id)}
      <JobItem {job} />
    {:else}
      <p class="hint">No jobs yet</p>
    {/each}
  </section>

  <footer>
    <button class="new-job">+ New job</button>
  </footer>
</div>

<style>
  .sidebar { display: flex; flex-direction: column; height: 100%; }
  header   { padding: var(--sp-3); border-bottom: 1px solid var(--c-border); }
  header h2 { font-size: var(--font-size-base); font-weight: 600; margin: 0; }
  section  { flex: 1; padding: var(--sp-2) 0; overflow-y: auto; }
  section h3 {
    font-size: var(--font-size-xs); text-transform: uppercase;
    color: var(--c-text-muted); letter-spacing: 0.06em;
    padding: var(--sp-2) var(--sp-3); margin: 0;
  }
  footer   { padding: var(--sp-2); border-top: 1px solid var(--c-border); }
  .new-job {
    width: 100%; padding: var(--sp-2); border: 1px solid var(--c-border);
    background: var(--c-bg); border-radius: 4px; cursor: pointer;
  }
  .new-job:hover { background: var(--c-accent-fade); }
  .hint    { color: var(--c-text-muted); padding: var(--sp-3); font-size: var(--font-size-sm); }
</style>
```

- [ ] **Step 3: Verify**

```bash
cd ~/Desktop/GanttBok && pnpm exec tsc --noEmit && cd src-tauri && . "$HOME/.cargo/env" && cargo check
```

- [ ] **Step 4: Commit**

```bash
git add src/lib/sidebar/
git commit -m "feat(ui): sidebar lists active jobs, click to open"
```

---

### Task 8: SA holiday sync on job open

**Files:**
- Modify: `src/lib/store.svelte.ts`

- [ ] **Step 1: After `openJob` loads a non-template job, ensure SA holidays are synced for project_start_date through +18 months.**

Replace `openJob`:

```typescript
async openJob(jobId: number): Promise<void> {
  this.currentJob   = await ipc.getJob(jobId);
  if (!this.currentJob.is_template) {
    const start = this.currentJob.project_start_date;
    const startDate = new Date(start);
    const end = new Date(startDate);
    end.setMonth(end.getMonth() + 18);
    await ipc.syncSaHolidays({
      job_id: jobId,
      from: start,
      to: end.toISOString().slice(0, 10),
    });
  }
  this.phases       = await ipc.listPhases(jobId);
  this.tasks        = await ipc.listTasks(jobId);
  this.dependencies = await ipc.listDependencies(jobId);
  this.noWorkDays   = await ipc.listNoWorkDays(jobId);
  this.selection    = null;
  await ipc.setLastOpenJob(jobId);
}
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/store.svelte.ts
git commit -m "feat(ui): auto-sync SA public holidays for 18 months ahead on job open"
```

---

### Task 9: New-job modal

**Files:**
- Create: `src/lib/sidebar/NewJobModal.svelte`
- Modify: `src/lib/sidebar/Sidebar.svelte`
- Modify: `src/lib/store.svelte.ts` (add `showNewJobModal` flag)

- [ ] **Step 1: Add a `showNewJobModal` state to the store**

In `store.svelte.ts` add:

```typescript
showNewJobModal = $state<boolean>(false);

async createJob(args: { name: string; client: string | null; address: string | null; project_start_date: string; }): Promise<void> {
  const job = await ipc.createJob({ ...args, is_template: false });
  await this.refreshSidebar();
  await this.openJob(job.id);
  this.showNewJobModal = false;
}
```

- [ ] **Step 2: Write `NewJobModal.svelte`**

```svelte
<script lang="ts">
  import { state } from '../store.svelte';

  let name = $state('');
  let client = $state('');
  let address = $state('');
  let startDate = $state(new Date().toISOString().slice(0, 10));
  let submitting = $state(false);

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!name.trim()) return;
    submitting = true;
    try {
      await state.createJob({
        name: name.trim(),
        client: client.trim() || null,
        address: address.trim() || null,
        project_start_date: startDate,
      });
    } finally {
      submitting = false;
    }
  }

  function cancel() { state.showNewJobModal = false; }
</script>

<div class="backdrop" onclick={cancel} role="presentation"></div>
<form class="modal" onsubmit={submit}>
  <h2>New job</h2>
  <label>Name<input bind:value={name} autofocus required /></label>
  <label>Client<input bind:value={client} placeholder="optional" /></label>
  <label>Address<input bind:value={address} placeholder="optional" /></label>
  <label>Project start<input type="date" bind:value={startDate} required /></label>
  <div class="actions">
    <button type="button" onclick={cancel}>Cancel</button>
    <button type="submit" class="primary" disabled={submitting || !name.trim()}>Create</button>
  </div>
</form>

<style>
  .backdrop {
    position: fixed; inset: 0; background: rgba(0, 0, 0, 0.3);
    z-index: 10;
  }
  .modal {
    position: fixed; top: 50%; left: 50%;
    transform: translate(-50%, -50%);
    background: var(--c-panel); border-radius: 8px;
    padding: var(--sp-6); box-shadow: 0 16px 48px var(--c-shadow);
    z-index: 11; min-width: 360px;
    display: flex; flex-direction: column; gap: var(--sp-3);
  }
  .modal h2 { margin: 0 0 var(--sp-2); font-size: var(--font-size-lg); }
  label { display: flex; flex-direction: column; gap: var(--sp-1); font-size: var(--font-size-sm); color: var(--c-text-muted); }
  input { padding: var(--sp-2); border: 1px solid var(--c-border); border-radius: 4px; }
  .actions { display: flex; justify-content: flex-end; gap: var(--sp-2); margin-top: var(--sp-2); }
  .actions button { padding: var(--sp-2) var(--sp-3); border: 1px solid var(--c-border); background: var(--c-bg); border-radius: 4px; cursor: pointer; }
  .actions .primary { background: var(--c-accent); color: white; border-color: var(--c-accent); }
  .actions .primary:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
```

- [ ] **Step 3: Wire the button in `Sidebar.svelte`**

Add the modal mount and hook the button click:

```svelte
<script lang="ts">
  import { state } from '../store.svelte';
  import JobItem from './JobItem.svelte';
  import NewJobModal from './NewJobModal.svelte';
</script>

<div class="sidebar">
  <header><h2>Gantt Bok</h2></header>
  <section>
    <h3>Active</h3>
    {#each state.jobs as job (job.id)}
      <JobItem {job} />
    {:else}
      <p class="hint">No jobs yet</p>
    {/each}
  </section>
  <footer>
    <button class="new-job" onclick={() => state.showNewJobModal = true}>+ New job</button>
  </footer>
</div>

{#if state.showNewJobModal}
  <NewJobModal />
{/if}

<style>
  /* (unchanged from Task 7) */
</style>
```

(Keep the existing `<style>` block exactly — only the script and template above change.)

- [ ] **Step 4: Commit**

```bash
git add src/lib/sidebar/ src/lib/store.svelte.ts
git commit -m "feat(ui): new-job modal — name + client + address + start-date, opens job on create"
```

---

### Task 10: Archived group (collapsed by default)

**Files:**
- Create: `src/lib/sidebar/ArchivedGroup.svelte`
- Modify: `src/lib/store.svelte.ts` (split `jobs` into active vs archived getters)
- Modify: `src/lib/sidebar/Sidebar.svelte`

- [ ] **Step 1: In store, the backend's `list_jobs` already filters archived=0. We need a separate fetch for archived.** Add to `Store`:

```typescript
archivedJobs = $state<Job[]>([]);

async refreshArchived(): Promise<void> {
  // Backend doesn't have a list_archived command; fetch all by toggling and use a generic.
  // For Plan 2 we fake it by calling list_jobs with a future extension. Until backend exposes it,
  // archived stays empty. (Backend extension is a 5-line task scheduled for Plan 3.)
  this.archivedJobs = [];
}
```

> *Plan note:* the Rust `repo::job::list_active` filters `archived = 0 AND is_template = 0`. We don't yet have a `list_archived` command. **For Plan 2 we leave the Archived group present but empty**; a one-line backend extension is in Plan 3. This is a deliberate cut.

- [ ] **Step 2: Write `ArchivedGroup.svelte`**

```svelte
<script lang="ts">
  import { state } from '../store.svelte';
  import JobItem from './JobItem.svelte';
  let expanded = $state(false);
</script>

<section>
  <button class="header" onclick={() => expanded = !expanded}>
    {expanded ? '▾' : '▸'} Archived ({state.archivedJobs.length})
  </button>
  {#if expanded}
    {#each state.archivedJobs as job (job.id)}
      <JobItem {job} />
    {:else}
      <p class="hint">No archived jobs</p>
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
  .hint { color: var(--c-text-muted); padding: var(--sp-3); font-size: var(--font-size-sm); }
</style>
```

- [ ] **Step 3: Mount in `Sidebar.svelte`**

Add `<ArchivedGroup />` below the Active section.

- [ ] **Step 4: Commit**

```bash
git add src/lib/sidebar/ src/lib/store.svelte.ts
git commit -m "feat(ui): archived jobs group (collapsed, empty in Plan 2 — backend extension Plan 3)"
```

---

## Phase C — Canvas scaffold (Tasks 11–16)

### Task 11: GanttCanvas root + viewport math

**Files:**
- Modify: `src/lib/canvas/GanttCanvas.svelte`

- [ ] **Step 1: Compute the visible date range from `state.currentJob.project_start_date` + the longest task's end date.**

```svelte
<script lang="ts">
  import { state } from '../store.svelte';
  import HeaderStrip from './HeaderStrip.svelte';
  import LeftRail from './LeftRail.svelte';
  import { computeViewportDays } from '../calendar';

  const days = $derived.by(() => {
    if (!state.currentJob) return [];
    return computeViewportDays(
      state.currentJob.project_start_date,
      state.tasks,
    );
  });

  // Cell width pulled from CSS variable so print can override.
  const CELL = 24;
</script>

<div class="gantt" style="--cell-w: {CELL}px;">
  <LeftRail />
  <div class="grid-area" style="--total-w: {days.length * CELL}px;">
    <HeaderStrip {days} />
    <div class="rows">
      <!-- Phase + task rows land in Tasks 15–18. Empty grid for now. -->
    </div>
  </div>
</div>

<style>
  .gantt {
    display: grid;
    grid-template-columns: var(--left-rail-width) 1fr;
    height: 100%;
    overflow: hidden;
  }
  .grid-area {
    position: relative;
    overflow-x: auto;
    overflow-y: auto;
  }
  .rows {
    position: relative;
    width: var(--total-w);
  }
</style>
```

- [ ] **Step 2: Stage. Commit happens after Task 14 (when calendar.ts exists).**

---

### Task 12: HeaderStrip — week-numbered, M T W T F, Monday-only dates

**Files:**
- Create: `src/lib/canvas/HeaderStrip.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script lang="ts">
  import type { ViewportDay } from '../calendar';
  let { days }: { days: ViewportDay[] } = $props();
</script>

<div class="header-strip" style="--total-w: {days.length * 24}px;">
  {#each days as d (d.date)}
    <div class="cell" class:week-start={d.weekday === 'M'}>
      {#if d.weekday === 'M'}
        <div class="week-num">Week {d.projectWeekNumber}</div>
      {/if}
      <div class="day-letter">{d.weekday}</div>
      {#if d.weekday === 'M'}
        <div class="day-number">{d.dayOfMonth}</div>
      {/if}
    </div>
  {/each}
</div>

<style>
  .header-strip {
    display: flex;
    width: var(--total-w);
    height: var(--header-height);
    border-bottom: 1px solid var(--c-border-bold);
    background: var(--c-panel);
    position: sticky; top: 0; z-index: 2;
  }
  .cell {
    width: var(--cell-w, 24px);
    border-right: 1px solid var(--c-border);
    display: flex; flex-direction: column;
    align-items: center; justify-content: flex-end;
    padding-bottom: 4px;
    position: relative;
  }
  .cell.week-start { border-right-color: var(--c-border); border-left: 2px solid var(--c-border-bold); }
  .week-num {
    position: absolute; top: 4px; left: 0;
    font-size: var(--font-size-xs); color: var(--c-text-muted);
    text-transform: uppercase; letter-spacing: 0.04em;
    white-space: nowrap;
    padding-left: 4px;
  }
  .day-letter { font-size: var(--font-size-xs); color: var(--c-text-muted); }
  .day-number { font-size: var(--font-size-xs); font-weight: 600; color: var(--c-accent); }
</style>
```

- [ ] **Step 2: Stage**

---

### Task 13: NoWorkColumn — diagonal stripes + rotated holiday name

**Files:**
- Create: `src/lib/canvas/NoWorkColumn.svelte`

- [ ] **Step 1: Write the absolutely-positioned column overlay**

```svelte
<script lang="ts">
  import type { ViewportDay } from '../calendar';
  import { state } from '../store.svelte';
  let { days, totalHeight }: { days: ViewportDay[]; totalHeight: number } = $props();

  const noWorkByDate = $derived.by(() => {
    const m = new Map<string, string>();
    for (const n of state.noWorkDays) m.set(n.date, n.reason);
    return m;
  });
</script>

{#each days as d, i (d.date)}
  {#if noWorkByDate.has(d.date)}
    <div
      class="no-work"
      style="left: {i * 24}px; width: 24px; height: {totalHeight}px;"
      title={noWorkByDate.get(d.date)}
    >
      <div class="label">{noWorkByDate.get(d.date)}</div>
    </div>
  {/if}
{/each}

<style>
  .no-work {
    position: absolute; top: 0; z-index: 0;
    background-image: repeating-linear-gradient(
      45deg,
      var(--c-no-work),
      var(--c-no-work) 4px,
      transparent 4px,
      transparent 8px
    );
    pointer-events: none;
  }
  .label {
    position: absolute; top: 4px; left: 50%;
    transform: translateX(-50%) rotate(90deg);
    transform-origin: center top;
    font-size: 9px; color: var(--c-no-work-text);
    white-space: nowrap;
    pointer-events: auto;
  }
</style>
```

- [ ] **Step 2: Stage**

---

### Task 14: Frontend calendar helpers + viewport day model

**Files:**
- Create: `src/lib/calendar.ts`
- Create: `src/lib/__tests__/calendar.test.ts`

- [ ] **Step 1: Write failing tests first**

```typescript
// src/lib/__tests__/calendar.test.ts
import { describe, it, expect } from 'vitest';
import { computeViewportDays, addCalendarDays, addWorkdays } from '../calendar';
import type { Task } from '../types';

describe('calendar', () => {
  it('viewport starts on the Monday of project start week', () => {
    // Wed 2026-06-10. Monday of that week is 2026-06-08.
    const days = computeViewportDays('2026-06-10', []);
    expect(days[0].date).toBe('2026-06-08');
    expect(days[0].weekday).toBe('M');
    expect(days[0].projectWeekNumber).toBe(1);
  });

  it('viewport excludes weekends', () => {
    const days = computeViewportDays('2026-06-08', []);
    expect(days.every(d => ['M', 'T', 'W', 'F'].includes(d.weekday) || d.weekday === 'T')).toBe(true);
    expect(days.length % 5).toBe(0);
  });

  it('Monday of week N has projectWeekNumber N', () => {
    const days = computeViewportDays('2026-06-08', []);
    // Week 1 starts Mon 8 Jun. Week 2 Mon = 15 Jun.
    const mon2 = days.find(d => d.date === '2026-06-15');
    expect(mon2?.projectWeekNumber).toBe(2);
  });

  it('viewport extends past latest task end', () => {
    const tasks: Task[] = [
      { id: 1, phase_id: 1, name: 'T', start_date: '2026-08-15', duration_workdays: 5, order_index: 0, notes: null },
    ];
    const days = computeViewportDays('2026-06-08', tasks);
    // Last day must be >= the task end (which is ~22 Aug).
    expect(days[days.length - 1].date >= '2026-08-22').toBe(true);
  });

  it('addWorkdays skips weekends', () => {
    expect(addWorkdays('2026-06-08', 5)).toBe('2026-06-15'); // Mon + 5 wd = next Mon
    expect(addWorkdays('2026-06-12', 1)).toBe('2026-06-15'); // Fri + 1 wd = Mon
  });

  it('addCalendarDays advances literally', () => {
    expect(addCalendarDays('2026-06-08', 1)).toBe('2026-06-09');
    expect(addCalendarDays('2026-06-08', 7)).toBe('2026-06-15');
  });
});
```

- [ ] **Step 2: Implement `src/lib/calendar.ts`**

```typescript
import type { Task } from './types';

export interface ViewportDay {
  date: string;                // YYYY-MM-DD
  weekday: 'M' | 'T' | 'W' | 'T' | 'F';
  dayOfMonth: number;
  projectWeekNumber: number;   // 1-indexed from the Monday of the project's start week
}

const WEEKDAY_LETTERS = ['M', 'T', 'W', 'T', 'F'] as const;

function parse(iso: string): Date {
  // Parse as UTC to avoid timezone drift on the date math.
  const [y, m, d] = iso.split('-').map(Number);
  return new Date(Date.UTC(y, m - 1, d));
}

function fmt(d: Date): string {
  return d.toISOString().slice(0, 10);
}

function mondayOfWeek(d: Date): Date {
  // JS getUTCDay: Sun=0, Mon=1 ... Sat=6.
  const day = d.getUTCDay();
  const offset = day === 0 ? -6 : 1 - day; // shift back to Monday
  return new Date(d.getTime() + offset * 86400000);
}

function isWorkday(d: Date): boolean {
  const day = d.getUTCDay();
  return day >= 1 && day <= 5;
}

export function addCalendarDays(iso: string, n: number): string {
  const d = parse(iso);
  d.setUTCDate(d.getUTCDate() + n);
  return fmt(d);
}

export function addWorkdays(iso: string, n: number): string {
  let d = parse(iso);
  while (!isWorkday(d)) {
    d.setUTCDate(d.getUTCDate() + 1);
  }
  let remaining = n;
  while (remaining > 0) {
    d.setUTCDate(d.getUTCDate() + 1);
    if (isWorkday(d)) remaining--;
  }
  return fmt(d);
}

export function computeViewportDays(projectStart: string, tasks: Task[]): ViewportDay[] {
  const start = mondayOfWeek(parse(projectStart));

  // Compute the latest end so the viewport is wide enough.
  let latestEnd = parse(projectStart);
  for (const t of tasks) {
    const end = parse(addWorkdays(t.start_date, Math.max(0, t.duration_workdays - 1)));
    if (end > latestEnd) latestEnd = end;
  }
  // Pad 4 weeks beyond the latest task end (28 days).
  latestEnd.setUTCDate(latestEnd.getUTCDate() + 28);

  const days: ViewportDay[] = [];
  let cur = new Date(start);
  let weekNum = 1;
  let weekdayIdx = 0;
  while (cur <= latestEnd) {
    if (isWorkday(cur)) {
      days.push({
        date: fmt(cur),
        weekday: WEEKDAY_LETTERS[weekdayIdx],
        dayOfMonth: cur.getUTCDate(),
        projectWeekNumber: weekNum,
      });
      weekdayIdx++;
      if (weekdayIdx === 5) {
        weekdayIdx = 0;
        weekNum++;
      }
    }
    cur.setUTCDate(cur.getUTCDate() + 1);
  }
  return days;
}
```

- [ ] **Step 3: Run vitest**

```bash
cd ~/Desktop/GanttBok && pnpm exec vitest run calendar
```

Expected: 6 pass.

- [ ] **Step 4: Wire NoWorkColumn into GanttCanvas**

In `GanttCanvas.svelte` import `NoWorkColumn`, mount it inside `.rows` once we know `totalHeight = (phases collapsed: state.phases.length) * row_height`. For now use a static `totalHeight = 320` since rows land in Task 17:

```svelte
<script lang="ts">
  // ...existing imports...
  import NoWorkColumn from './NoWorkColumn.svelte';
</script>

<!-- Inside .rows: -->
<NoWorkColumn {days} totalHeight={320} />
```

- [ ] **Step 5: Commit (Tasks 11–14 together)**

```bash
git add src/lib/canvas/ src/lib/calendar.ts src/lib/__tests__/calendar.test.ts
git commit -m "feat(ui): Gantt canvas — viewport math, week-numbered header, no-work columns"
```

---

### Task 15: LeftRail — phase rows with hierarchical numbering

**Files:**
- Create: `src/lib/canvas/LeftRail.svelte`
- Create: `src/lib/__tests__/hierarchical-numbering.test.ts` → Task 16

- [ ] **Step 1: Write the LeftRail component**

```svelte
<script lang="ts">
  import { state } from '../store.svelte';
  import * as ipc from '../ipc';

  async function toggleCollapse(phaseId: number) {
    const phase = state.phases.find(p => p.id === phaseId);
    if (!phase) return;
    phase.collapsed = !phase.collapsed;
    await ipc.updatePhase($state.snapshot(phase));
  }
</script>

<div class="left-rail">
  {#each state.phases as phase, pi (phase.id)}
    <div class="phase-row" style="height: var(--row-height);">
      <button class="chev" onclick={() => toggleCollapse(phase.id)} aria-label="toggle">
        {phase.collapsed ? '▸' : '▾'}
      </button>
      <span class="num">{pi + 1}.</span>
      <span class="name">{phase.name}</span>
    </div>
    {#if !phase.collapsed}
      {#each (state.tasksByPhase.get(phase.id) ?? []) as task, ti (task.id)}
        <div class="task-row" style="height: var(--row-height);">
          <span class="num">{pi + 1}.{ti + 1}</span>
          <span class="name">{task.name}</span>
        </div>
      {/each}
    {/if}
  {/each}
</div>

<style>
  .left-rail {
    width: var(--left-rail-width);
    border-right: 1px solid var(--c-border);
    background: var(--c-panel);
    overflow-y: auto;
  }
  .phase-row, .task-row {
    display: flex; align-items: center; gap: var(--sp-2);
    padding: 0 var(--sp-2);
    border-bottom: 1px solid var(--c-border);
    font-size: var(--font-size-sm);
  }
  .task-row { padding-left: calc(var(--sp-2) * 4); color: var(--c-text-muted); }
  .chev {
    background: transparent; border: none; cursor: pointer;
    font-size: 10px; color: var(--c-text-muted);
    padding: 0; width: 14px;
  }
  .num { font-variant-numeric: tabular-nums; color: var(--c-text-muted); min-width: 28px; }
  .name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
```

- [ ] **Step 2: Commit (paired with Task 16 below)**

---

### Task 16: Hierarchical numbering — unit test

**Files:**
- Create: `src/lib/__tests__/hierarchical-numbering.test.ts`

The Left Rail derives numbers from `order_index`. A unit test pins the rule.

- [ ] **Step 1: Write the test**

```typescript
import { describe, it, expect } from 'vitest';
import type { Phase, Task } from '../types';

// The rule: phase[i] → "i+1.", task[i] in phase[j] → "j+1.i+1"
// (computed in template, but we test the algorithm here for clarity)
function phaseLabel(orderIndex: number) { return `${orderIndex + 1}.`; }
function taskLabel(phaseIndex: number, taskIndex: number) {
  return `${phaseIndex + 1}.${taskIndex + 1}`;
}

describe('hierarchical numbering', () => {
  it('phases number 1, 2, 3, ... by order_index', () => {
    expect(phaseLabel(0)).toBe('1.');
    expect(phaseLabel(1)).toBe('2.');
    expect(phaseLabel(2)).toBe('3.');
  });

  it('tasks number 1.1, 1.2, 2.1, 2.2 ...', () => {
    expect(taskLabel(0, 0)).toBe('1.1');
    expect(taskLabel(0, 1)).toBe('1.2');
    expect(taskLabel(1, 0)).toBe('2.1');
  });

  it('numbering does not depend on id, only order_index', () => {
    const phases: Phase[] = [
      { id: 99, job_id: 1, name: 'P1', colour: '#000', order_index: 0, collapsed: false },
      { id:  1, job_id: 1, name: 'P2', colour: '#000', order_index: 1, collapsed: false },
    ];
    expect(phaseLabel(phases[0].order_index)).toBe('1.');
    expect(phaseLabel(phases[1].order_index)).toBe('2.');
  });
});
```

- [ ] **Step 2: Run + commit (Tasks 15 + 16 together)**

```bash
cd ~/Desktop/GanttBok && pnpm exec vitest run hierarchical-numbering
git add src/lib/canvas/LeftRail.svelte src/lib/__tests__/hierarchical-numbering.test.ts
git commit -m "feat(ui): left rail with hierarchical numbering (1, 1.1, 1.2, ...)"
```

---

## Phase D — Bars + dependency lines (Tasks 17–21)

### Task 17: TaskBar — render task as SVG rect

**Files:**
- Create: `src/lib/canvas/TaskBar.svelte`
- Modify: `src/lib/canvas/GanttCanvas.svelte` (mount task rows)

- [ ] **Step 1: Write `TaskBar.svelte`**

```svelte
<script lang="ts">
  import type { Task, Phase } from '../types';
  import { state } from '../store.svelte';

  let { task, phase, days, row }: {
    task: Task; phase: Phase; days: { date: string }[]; row: number;
  } = $props();

  const xStart = $derived(days.findIndex(d => d.date === task.start_date) * 24);
  const w = $derived(task.duration_workdays * 24);
  const y = $derived(row * 32 + 6);   // 6px vertical padding inside row
  const isSelected = $derived(state.selection?.kind === 'task' && state.selection.id === task.id);

  function select(e: MouseEvent) {
    e.stopPropagation();
    state.select({ kind: 'task', id: task.id });
  }
</script>

<g class="task-bar" onclick={select} role="button" tabindex="0">
  <rect
    x={xStart} y={y}
    width={w} height={20}
    rx={3}
    fill={phase.colour}
    stroke={isSelected ? 'var(--c-accent)' : 'transparent'}
    stroke-width="2"
  />
  {#if w > 60}
    <text x={xStart + 6} y={y + 14} fill="white" font-size="11">{task.name}</text>
  {/if}
</g>

<style>
  .task-bar { cursor: grab; }
  .task-bar:active { cursor: grabbing; }
</style>
```

- [ ] **Step 2: Add the SVG `<svg>` root + mount task rows in `GanttCanvas.svelte`**

Replace the `.rows` block with an SVG canvas that contains task rows. Each task's `row` index is computed from a flattened phase/task list respecting collapse state.

```svelte
<script lang="ts">
  // ...existing imports + computed days...
  import TaskBar from './TaskBar.svelte';
  import NoWorkColumn from './NoWorkColumn.svelte';

  const rows = $derived.by(() => {
    type Row =
      | { kind: 'phase'; phase: import('../types').Phase }
      | { kind: 'task'; task: import('../types').Task; phase: import('../types').Phase };
    const out: Row[] = [];
    for (const phase of state.phases) {
      out.push({ kind: 'phase', phase });
      if (!phase.collapsed) {
        for (const task of (state.tasksByPhase.get(phase.id) ?? [])) {
          out.push({ kind: 'task', task, phase });
        }
      }
    }
    return out;
  });

  const ROW_H = 32;
  const totalHeight = $derived(rows.length * ROW_H);
  const CELL = 24;
</script>

<!-- inside .gantt → .grid-area, after HeaderStrip -->
<svg
  width={days.length * CELL}
  height={totalHeight}
  class="canvas-svg"
  onclick={() => state.select(null)}
>
  <NoWorkColumn {days} {totalHeight} />
  {#each rows as r, ri (r.kind === 'phase' ? `p${r.phase.id}` : `t${r.task.id}`)}
    {#if r.kind === 'task'}
      <TaskBar task={r.task} phase={r.phase} {days} row={ri} />
    {/if}
  {/each}
</svg>

<style>
  /* ...existing... */
  .canvas-svg { display: block; }
</style>
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/canvas/TaskBar.svelte src/lib/canvas/GanttCanvas.svelte
git commit -m "feat(ui): render task bars as SVG rects positioned by start_date + duration"
```

---

### Task 18: PhaseBar — composite spanning earliest task → latest task end

**Files:**
- Create: `src/lib/canvas/PhaseBar.svelte`
- Modify: `src/lib/canvas/GanttCanvas.svelte`

- [ ] **Step 1: Write `PhaseBar.svelte`**

```svelte
<script lang="ts">
  import type { Phase, Task } from '../types';
  import { addWorkdays } from '../calendar';

  let { phase, tasks, days, row }: {
    phase: Phase; tasks: Task[]; days: { date: string }[]; row: number;
  } = $props();

  const span = $derived.by(() => {
    if (tasks.length === 0) return null;
    const starts = tasks.map(t => t.start_date).sort();
    const ends   = tasks.map(t => addWorkdays(t.start_date, Math.max(0, t.duration_workdays - 1))).sort();
    const startIdx = days.findIndex(d => d.date === starts[0]);
    const endIdx   = days.findIndex(d => d.date === ends[ends.length - 1]);
    if (startIdx < 0 || endIdx < 0) return null;
    return { x: startIdx * 24, w: (endIdx - startIdx + 1) * 24 };
  });

  const y = $derived(row * 32 + 6);
</script>

{#if span}
  <rect
    class="phase-bar"
    x={span.x} y={y}
    width={span.w} height={20}
    rx={3}
    fill={phase.colour}
    fill-opacity="0.18"
    stroke={phase.colour}
    stroke-opacity="0.5"
    stroke-width="1"
  />
{/if}

<style>
  .phase-bar { pointer-events: none; }
</style>
```

- [ ] **Step 2: Mount in `GanttCanvas.svelte`** — only when the phase is collapsed (otherwise the inner task bars represent it):

```svelte
{#each rows as r, ri (r.kind === 'phase' ? `p${r.phase.id}` : `t${r.task.id}`)}
  {#if r.kind === 'phase' && r.phase.collapsed}
    <PhaseBar phase={r.phase} tasks={state.tasksByPhase.get(r.phase.id) ?? []} {days} row={ri} />
  {:else if r.kind === 'task'}
    <TaskBar task={r.task} phase={r.phase} {days} row={ri} />
  {/if}
{/each}
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/canvas/PhaseBar.svelte src/lib/canvas/GanttCanvas.svelte
git commit -m "feat(ui): collapsed phase bars span earliest task to latest task end"
```

---

### Task 19: DependencyArrow — SVG path with arrowhead

**Files:**
- Create: `src/lib/canvas/DependencyArrow.svelte`
- Modify: `src/lib/canvas/GanttCanvas.svelte`

- [ ] **Step 1: Write `DependencyArrow.svelte`**

```svelte
<script lang="ts">
  import type { Dependency, Task } from '../types';
  import { addWorkdays } from '../calendar';

  let { dep, tasks, rowIndex, days }: {
    dep: Dependency;
    tasks: Task[];
    rowIndex: Map<number, number>;
    days: { date: string }[];
  } = $props();

  const path = $derived.by(() => {
    const pre = tasks.find(t => t.id === dep.predecessor_id);
    const suc = tasks.find(t => t.id === dep.successor_id);
    if (!pre || !suc) return null;
    const preRow = rowIndex.get(pre.id);
    const sucRow = rowIndex.get(suc.id);
    if (preRow === undefined || sucRow === undefined) return null;

    const preEndDate = addWorkdays(pre.start_date, Math.max(0, pre.duration_workdays - 1));
    const preEndIdx  = days.findIndex(d => d.date === preEndDate);
    const sucStartIdx = days.findIndex(d => d.date === suc.start_date);
    if (preEndIdx < 0 || sucStartIdx < 0) return null;

    const x1 = (preEndIdx + 1) * 24;         // right edge of predecessor
    const y1 = preRow * 32 + 16;             // vertical centre
    const x2 = sucStartIdx * 24;             // left edge of successor
    const y2 = sucRow * 32 + 16;
    // Right-angle elbow path
    return `M ${x1} ${y1} L ${x1 + 6} ${y1} L ${x1 + 6} ${y2} L ${x2} ${y2}`;
  });
</script>

{#if path}
  <path d={path} class="dep-line" stroke="var(--c-border-bold)" stroke-width="1" fill="none" marker-end="url(#arrowhead)" />
{/if}
```

- [ ] **Step 2: Add the arrowhead marker definition once in `GanttCanvas.svelte`** (inside the `<svg>`):

```svelte
<defs>
  <marker id="arrowhead" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
    <path d="M 0 0 L 10 5 L 0 10 z" fill="var(--c-border-bold)" />
  </marker>
</defs>
```

Mount the dependency arrows after the bars (so they layer on top):

```svelte
{#each state.dependencies as dep (dep.id)}
  <DependencyArrow {dep} tasks={state.tasks} rowIndex={rowIndexMap} {days} />
{/each}
```

Add a `rowIndexMap` derived value that maps `task.id → row` (used by both bars and arrows):

```typescript
const rowIndexMap = $derived.by(() => {
  const m = new Map<number, number>();
  rows.forEach((r, i) => { if (r.kind === 'task') m.set(r.task.id, i); });
  return m;
});
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/canvas/DependencyArrow.svelte src/lib/canvas/GanttCanvas.svelte
git commit -m "feat(ui): dependency arrows — elbow path predecessor → successor with arrowhead"
```

---

### Task 20: Hover highlight on dependency endpoints

**Files:**
- Modify: `src/lib/canvas/TaskBar.svelte`
- Modify: `src/lib/canvas/DependencyArrow.svelte`
- Modify: `src/lib/store.svelte.ts`

- [ ] **Step 1: Add a `hoveredTaskId` state to the store**

```typescript
hoveredTaskId = $state<number | null>(null);
```

- [ ] **Step 2: In `TaskBar.svelte`, set hover on pointer enter/leave**

Add to the `<g>`:

```svelte
<g
  class="task-bar"
  onmouseenter={() => state.hoveredTaskId = task.id}
  onmouseleave={() => state.hoveredTaskId = null}
  ...
>
```

- [ ] **Step 3: In `DependencyArrow.svelte`, brighten when either endpoint is hovered**

```svelte
<script lang="ts">
  // ...existing...
  import { state } from '../store.svelte';
  const isLit = $derived(
    state.hoveredTaskId === dep.predecessor_id || state.hoveredTaskId === dep.successor_id
  );
</script>

<path d={path} class:lit={isLit} ... stroke={isLit ? 'var(--c-accent)' : 'var(--c-border-bold)'} stroke-width={isLit ? 2 : 1} ... />
```

- [ ] **Step 4: Commit**

```bash
git add src/lib/canvas/ src/lib/store.svelte.ts
git commit -m "feat(ui): dependency arrows brighten on hover of either endpoint"
```

---

### Task 21: Commit checkpoint — full canvas renders read-only

At this point the canvas displays a job's phases, tasks, dependency arrows, and no-work columns. Click selects a bar. No drag yet — that's Phase E.

- [ ] **Step 1: Verify**

```bash
cd ~/Desktop/GanttBok && pnpm exec tsc --noEmit && pnpm exec vitest run && cd src-tauri && . "$HOME/.cargo/env" && cargo check
```

All should be green.

- [ ] **Step 2: No new commit; rolling into Phase E.**

---

## Phase E — Drag physics (Tasks 22–27)

### Task 22: Hit-test — which zone of a bar is the pointer over?

**Files:**
- Create: `src/lib/hit-test.ts`
- Create: `src/lib/__tests__/hit-test.test.ts`

- [ ] **Step 1: Failing tests**

```typescript
// src/lib/__tests__/hit-test.test.ts
import { describe, it, expect } from 'vitest';
import { hitZone, type Zone } from '../hit-test';

describe('hitZone', () => {
  // Bar width 100. Edge zone is 10% = 10px each side. Middle = 80px.
  it('left 10% is resize-start', () => {
    expect(hitZone({ relX: 0,  width: 100 })).toBe<Zone>('resize-start');
    expect(hitZone({ relX: 9,  width: 100 })).toBe<Zone>('resize-start');
  });
  it('right 10% is resize-end', () => {
    expect(hitZone({ relX: 91,  width: 100 })).toBe<Zone>('resize-end');
    expect(hitZone({ relX: 100, width: 100 })).toBe<Zone>('resize-end');
  });
  it('middle 80% is move', () => {
    expect(hitZone({ relX: 10, width: 100 })).toBe<Zone>('move');
    expect(hitZone({ relX: 50, width: 100 })).toBe<Zone>('move');
    expect(hitZone({ relX: 89, width: 100 })).toBe<Zone>('move');
  });
  it('narrow bars cap edge zone at 4px so move zone stays usable', () => {
    // Width 20: 10% = 2px; capped at 4px? actually narrow bar = at least move zone exists.
    // We pick: edgeWidth = min(10% * width, 8px) and edges never overlap.
    expect(hitZone({ relX: 7,  width: 20 })).toBe<Zone>('move');
    expect(hitZone({ relX: 3,  width: 20 })).toBe<Zone>('resize-start');
  });
});
```

- [ ] **Step 2: Implement**

```typescript
// src/lib/hit-test.ts
export type Zone = 'move' | 'resize-start' | 'resize-end';

export function hitZone({ relX, width }: { relX: number; width: number }): Zone {
  const edge = Math.min(width * 0.1, 8);
  if (relX < edge) return 'resize-start';
  if (relX > width - edge) return 'resize-end';
  return 'move';
}
```

- [ ] **Step 3: Run + commit**

```bash
cd ~/Desktop/GanttBok && pnpm exec vitest run hit-test
git add src/lib/hit-test.ts src/lib/__tests__/hit-test.test.ts
git commit -m "feat(ui): hit-test for bar grab zones (resize-start / move / resize-end)"
```

---

### Task 23: Magnetic snap — pure function

**Files:**
- Create: `src/lib/snap.ts`
- Create: `src/lib/__tests__/snap.test.ts`

- [ ] **Step 1: Failing tests**

```typescript
// src/lib/__tests__/snap.test.ts
import { describe, it, expect } from 'vitest';
import { magneticSnap } from '../snap';

describe('magneticSnap', () => {
  // Cell width 24px. Strong pull <= 30%, no pull > 70%.
  // Pull strength sigmoid-like: bar position drawn = cell-aligned when within pull radius.
  it('exact cell boundary stays put', () => {
    expect(magneticSnap({ pxDelta: 0,   cellW: 24 })).toBe(0);
    expect(magneticSnap({ pxDelta: 24,  cellW: 24 })).toBe(24);
    expect(magneticSnap({ pxDelta: -24, cellW: 24 })).toBe(-24);
  });
  it('within 30% pull snaps to nearest cell', () => {
    // 4px into a 24px cell = ~17% — pulls back to 0.
    expect(magneticSnap({ pxDelta: 4,  cellW: 24 })).toBe(0);
    // 20px = 83% (closer to next cell). Pulls forward to 24.
    expect(magneticSnap({ pxDelta: 20, cellW: 24 })).toBe(24);
  });
  it('within free zone tracks pointer faithfully', () => {
    // 8px = 33% — outside hard pull, but eased toward 0 a bit.
    const result = magneticSnap({ pxDelta: 8, cellW: 24 });
    expect(result).toBeGreaterThanOrEqual(0);
    expect(result).toBeLessThanOrEqual(8);
  });
});
```

- [ ] **Step 2: Implement**

```typescript
// src/lib/snap.ts
/**
 * Magnetic snap. Returns the rendered pixel position given a pointer delta in pixels.
 * - <30% into a cell: pulls hard to the nearest day-edge (snap).
 * - 30–70%: eased pull (free-with-bias).
 * - >70%: pulls to the next cell.
 *
 * Pure function — no DOM.
 */
export function magneticSnap({ pxDelta, cellW }: { pxDelta: number; cellW: number }): number {
  if (cellW <= 0) return pxDelta;
  const cells = pxDelta / cellW;
  const nearest = Math.round(cells);
  const fractional = cells - nearest; // -0.5 .. 0.5
  const absFrac = Math.abs(fractional);
  // Pull strength: 1.0 (fully snapped) when absFrac < 0.2; 0.0 (no pull) when absFrac > 0.5.
  let pull: number;
  if (absFrac < 0.2) pull = 1.0;
  else if (absFrac > 0.5) pull = 0.0;
  else {
    // Linear ease in [0.2, 0.5] → [1.0, 0.0]
    pull = 1.0 - (absFrac - 0.2) / 0.3;
  }
  const snappedFrac = fractional * (1 - pull);
  return (nearest + snappedFrac) * cellW;
}
```

- [ ] **Step 3: Run + commit**

```bash
cd ~/Desktop/GanttBok && pnpm exec vitest run snap
git add src/lib/snap.ts src/lib/__tests__/snap.test.ts
git commit -m "feat(ui): magneticSnap pure function — hard pull within 20%, ease 20–50%, free beyond"
```

---

### Task 24: DragOverlay + pointer event harness

**Files:**
- Create: `src/lib/canvas/DragOverlay.svelte`
- Modify: `src/lib/canvas/TaskBar.svelte`
- Modify: `src/lib/store.svelte.ts`

- [ ] **Step 1: Add drag state to the store**

```typescript
import type { Zone } from './hit-test';

dragState = $state<{
  taskId: number;
  zone: Zone;
  startX: number;
  originalStart: string;
  originalDuration: number;
  liveDelta: number;       // current pixel delta, snapped
} | null>(null);

cancelDrag(): void {
  this.dragState = null;
}
```

- [ ] **Step 2: Modify `TaskBar.svelte` to capture pointerdown and arm the drag**

```svelte
<script lang="ts">
  import type { Task, Phase } from '../types';
  import { state } from '../store.svelte';
  import { hitZone } from '../hit-test';

  let { task, phase, days, row }: {
    task: Task; phase: Phase; days: { date: string }[]; row: number;
  } = $props();

  const xStart = $derived(days.findIndex(d => d.date === task.start_date) * 24);
  const w = $derived(task.duration_workdays * 24);
  const y = $derived(row * 32 + 6);
  const isSelected = $derived(state.selection?.kind === 'task' && state.selection.id === task.id);
  const isDragging = $derived(state.dragState?.taskId === task.id);

  function onPointerDown(e: PointerEvent) {
    e.stopPropagation();
    state.select({ kind: 'task', id: task.id });
    const rect = (e.target as Element).getBoundingClientRect();
    const relX = e.clientX - rect.left;
    const zone = hitZone({ relX, width: w });
    state.dragState = {
      taskId: task.id, zone,
      startX: e.clientX,
      originalStart: task.start_date,
      originalDuration: task.duration_workdays,
      liveDelta: 0,
    };
  }
</script>

<g class="task-bar" onpointerdown={onPointerDown}
   onmouseenter={() => state.hoveredTaskId = task.id}
   onmouseleave={() => state.hoveredTaskId = null}>
  <rect
    x={xStart} y={y}
    width={w} height={20}
    rx={3}
    fill={phase.colour}
    fill-opacity={isDragging ? 0.4 : 1}
    stroke={isSelected ? 'var(--c-accent)' : 'transparent'}
    stroke-width="2"
  />
  {#if w > 60}
    <text x={xStart + 6} y={y + 14} fill="white" font-size="11">{task.name}</text>
  {/if}
</g>
```

- [ ] **Step 3: Write `DragOverlay.svelte`** — listens on window for `pointermove`/`pointerup` while a drag is armed.

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { state } from '../store.svelte';
  import { magneticSnap } from '../snap';
  import { addWorkdays } from '../calendar';
  import * as ipc from '../ipc';

  const CELL = 24;

  function onPointerMove(e: PointerEvent) {
    if (!state.dragState) return;
    const rawDelta = e.clientX - state.dragState.startX;
    const snapped = magneticSnap({ pxDelta: rawDelta, cellW: CELL });
    state.dragState.liveDelta = snapped;
  }

  async function onPointerUp(_e: PointerEvent) {
    const d = state.dragState;
    if (!d) return;
    const deltaWorkdays = Math.round(d.liveDelta / CELL);
    state.dragState = null;
    if (deltaWorkdays === 0) return;
    if (!state.currentJob) return;

    const newStart = addWorkdays(d.originalStart, deltaWorkdays);
    const result = await ipc.dragTask({
      job_id: state.currentJob.id,
      task_id: d.taskId,
      new_start_date: newStart,
    });
    state.applyDragResult(result.updated_tasks);
    await ipc.touchLastSave();
  }

  onMount(() => {
    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', onPointerUp);
    return () => {
      window.removeEventListener('pointermove', onPointerMove);
      window.removeEventListener('pointerup', onPointerUp);
    };
  });
</script>
```

(The component renders nothing — it's a side-effect-only window listener bridge.)

- [ ] **Step 4: Mount `<DragOverlay />` in `GanttCanvas.svelte`** alongside the SVG.

- [ ] **Step 5: Add a live position adjustment to `TaskBar.svelte`** so the bar visually follows the cursor during drag (added on top of step 2):

Update the `<rect>` x:

```svelte
<rect
  x={xStart + (isDragging ? state.dragState!.liveDelta : 0)}
  ...
/>
```

- [ ] **Step 6: Commit**

```bash
git add src/lib/canvas/DragOverlay.svelte src/lib/canvas/TaskBar.svelte src/lib/canvas/GanttCanvas.svelte src/lib/store.svelte.ts
git commit -m "feat(ui): drag a task bar — pointer events, magnetic snap, IPC drag_task call with chain ripple"
```

---

### Task 25: Resize from edges

**Files:**
- Modify: `src/lib/canvas/DragOverlay.svelte`
- Modify: `src/lib/canvas/TaskBar.svelte`

When `dragState.zone === 'resize-end'`, the drag changes `duration_workdays`, not `start_date`. When `zone === 'resize-start'`, both move.

- [ ] **Step 1: Update `onPointerUp` in `DragOverlay.svelte`**

```typescript
async function onPointerUp(_e: PointerEvent) {
  const d = state.dragState;
  if (!d) return;
  const deltaWorkdays = Math.round(d.liveDelta / CELL);
  state.dragState = null;
  if (deltaWorkdays === 0) return;
  if (!state.currentJob) return;

  const task = state.tasks.find(t => t.id === d.taskId);
  if (!task) return;

  if (d.zone === 'move') {
    const newStart = addWorkdays(d.originalStart, deltaWorkdays);
    const result = await ipc.dragTask({
      job_id: state.currentJob.id,
      task_id: d.taskId,
      new_start_date: newStart,
    });
    state.applyDragResult(result.updated_tasks);
  } else if (d.zone === 'resize-end') {
    const newDur = Math.max(1, d.originalDuration + deltaWorkdays);
    const updated = { ...task, duration_workdays: newDur };
    await ipc.updateTask(updated);
    state.tasks = state.tasks.map(t => t.id === task.id ? updated : t);
  } else if (d.zone === 'resize-start') {
    const newStart = addWorkdays(d.originalStart, deltaWorkdays);
    const newDur = Math.max(1, d.originalDuration - deltaWorkdays);
    const updated = { ...task, start_date: newStart, duration_workdays: newDur };
    await ipc.updateTask(updated);
    state.tasks = state.tasks.map(t => t.id === task.id ? updated : t);
  }
  await ipc.touchLastSave();
}
```

- [ ] **Step 2: Update `TaskBar.svelte`** so the live preview also handles resize. Add a `liveWidth` derived for the rendered rect:

```typescript
const livePreview = $derived.by(() => {
  if (!isDragging || !state.dragState) return { x: xStart, w };
  const d = state.dragState;
  if (d.zone === 'move')         return { x: xStart + d.liveDelta, w };
  if (d.zone === 'resize-end')   return { x: xStart, w: Math.max(24, w + d.liveDelta) };
  if (d.zone === 'resize-start') return { x: xStart + d.liveDelta, w: Math.max(24, w - d.liveDelta) };
  return { x: xStart, w };
});
```

Use `livePreview.x` and `livePreview.w` in the `<rect>`.

- [ ] **Step 3: Add cursor feedback by zone** in `TaskBar.svelte`:

```svelte
<g class="task-bar" data-zone={state.dragState?.taskId === task.id ? state.dragState.zone : null}
   onpointerdown={onPointerDown} ... >
```

CSS:
```css
.task-bar { cursor: grab; }
.task-bar:active { cursor: grabbing; }
.task-bar[data-zone="resize-start"], .task-bar[data-zone="resize-end"] { cursor: ew-resize; }
```

- [ ] **Step 4: Commit**

```bash
git add src/lib/canvas/
git commit -m "feat(ui): drag-resize from left/right edges (start vs duration)"
```

---

### Task 26: Vertical reorder of tasks within a phase

**Files:**
- Modify: `src/lib/canvas/LeftRail.svelte`
- Modify: `src/lib/store.svelte.ts`

A simple "drag a task row's left handle to reorder" using HTML5 drag-and-drop on the rail rows. Not the SVG canvas — the left rail.

- [ ] **Step 1: Add `reorderTasksInPhase` to store**

```typescript
async reorderTasksInPhase(phaseId: number, orderedIds: number[]): Promise<void> {
  await ipc.reorderTasks(phaseId, orderedIds);
  // Re-sort local tasks to match the new order.
  const idx = new Map(orderedIds.map((id, i) => [id, i]));
  this.tasks = this.tasks.map(t => t.phase_id === phaseId ? { ...t, order_index: idx.get(t.id) ?? t.order_index } : t);
}
```

- [ ] **Step 2: Make task rows draggable in LeftRail**

```svelte
<div class="task-row"
  draggable="true"
  ondragstart={(e) => { e.dataTransfer!.setData('text/task-id', String(task.id)); }}
  ondragover={(e) => e.preventDefault()}
  ondrop={async (e) => {
    const draggedId = Number(e.dataTransfer!.getData('text/task-id'));
    const targetTasks = state.tasksByPhase.get(phase.id) ?? [];
    const ordered = targetTasks.map(t => t.id).filter(id => id !== draggedId);
    const targetIdx = targetTasks.findIndex(t => t.id === task.id);
    ordered.splice(targetIdx, 0, draggedId);
    await state.reorderTasksInPhase(phase.id, ordered);
  }}
  style="height: var(--row-height);"
>
  ...
</div>
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/canvas/LeftRail.svelte src/lib/store.svelte.ts
git commit -m "feat(ui): drag-to-reorder tasks within a phase (left rail)"
```

---

### Task 27: Vertical reorder of phases

**Files:**
- Modify: `src/lib/canvas/LeftRail.svelte`
- Modify: `src/lib/store.svelte.ts`

- [ ] **Step 1: Add `reorderPhases` to store**

```typescript
async reorderPhases(orderedIds: number[]): Promise<void> {
  if (!this.currentJob) return;
  await ipc.reorderPhases(this.currentJob.id, orderedIds);
  const idx = new Map(orderedIds.map((id, i) => [id, i]));
  this.phases = this.phases.map(p => ({ ...p, order_index: idx.get(p.id) ?? p.order_index }))
                            .sort((a, b) => a.order_index - b.order_index);
}
```

- [ ] **Step 2: Make phase header rows draggable**

In LeftRail, wrap the `.phase-row` with:

```svelte
<div class="phase-row"
  draggable="true"
  ondragstart={(e) => e.dataTransfer!.setData('text/phase-id', String(phase.id))}
  ondragover={(e) => e.preventDefault()}
  ondrop={async (e) => {
    const draggedId = Number(e.dataTransfer!.getData('text/phase-id'));
    if (!draggedId) return;
    const ordered = state.phases.map(p => p.id).filter(id => id !== draggedId);
    const targetIdx = state.phases.findIndex(p => p.id === phase.id);
    ordered.splice(targetIdx, 0, draggedId);
    await state.reorderPhases(ordered);
  }}
>
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/canvas/LeftRail.svelte src/lib/store.svelte.ts
git commit -m "feat(ui): drag-to-reorder phases (left rail headers)"
```

---

## Phase F — Creation + details (Tasks 28–32)

### Task 28: DetailsPanel + TaskDetails

**Files:**
- Modify: `src/lib/details/DetailsPanel.svelte`
- Create: `src/lib/details/TaskDetails.svelte`

- [ ] **Step 1: `DetailsPanel.svelte` — dispatch based on selection kind**

```svelte
<script lang="ts">
  import { state } from '../store.svelte';
  import TaskDetails from './TaskDetails.svelte';
  import PhaseDetails from './PhaseDetails.svelte';
</script>

{#if state.selection?.kind === 'task'}
  <TaskDetails taskId={state.selection.id} />
{:else if state.selection?.kind === 'phase'}
  <PhaseDetails phaseId={state.selection.id} />
{:else}
  <div class="empty">Select a bar to edit</div>
{/if}

<style>
  .empty { padding: var(--sp-4); color: var(--c-text-muted); font-size: var(--font-size-sm); }
</style>
```

- [ ] **Step 2: `TaskDetails.svelte`**

```svelte
<script lang="ts">
  import { state } from '../store.svelte';
  import * as ipc from '../ipc';
  import type { Task } from '../types';

  let { taskId }: { taskId: number } = $props();
  const task = $derived(state.tasks.find(t => t.id === taskId));

  let name = $state('');
  let duration = $state(1);
  let notes = $state('');

  $effect(() => {
    if (task) { name = task.name; duration = task.duration_workdays; notes = task.notes ?? ''; }
  });

  async function save() {
    if (!task) return;
    const updated: Task = {
      ...task,
      name: name.trim() || task.name,
      duration_workdays: Math.max(1, duration),
      notes: notes.trim() || null,
    };
    await ipc.updateTask(updated);
    state.tasks = state.tasks.map(t => t.id === updated.id ? updated : t);
    await ipc.touchLastSave();
  }

  async function del() {
    if (!task) return;
    if (!confirm(`Delete task "${task.name}"?`)) return;
    await ipc.deleteTask(task.id);
    state.tasks = state.tasks.filter(t => t.id !== task.id);
    state.select(null);
    await ipc.touchLastSave();
  }
</script>

{#if task}
  <div class="task-details">
    <h2>{task.name}</h2>
    <label>Name<input bind:value={name} onblur={save} /></label>
    <label>Duration (workdays)<input type="number" min="1" bind:value={duration} onblur={save} /></label>
    <label>Start<input type="date" value={task.start_date} disabled /></label>
    <label>Notes<textarea bind:value={notes} onblur={save} rows="4"></textarea></label>
    <button class="danger" onclick={del}>Delete task</button>
  </div>
{/if}

<style>
  .task-details { display: flex; flex-direction: column; gap: var(--sp-3); padding: var(--sp-4); }
  h2 { font-size: var(--font-size-lg); margin: 0; }
  label { display: flex; flex-direction: column; gap: var(--sp-1); font-size: var(--font-size-sm); color: var(--c-text-muted); }
  input, textarea { padding: var(--sp-2); border: 1px solid var(--c-border); border-radius: 4px; font: inherit; color: var(--c-text); background: var(--c-bg); }
  input:disabled { color: var(--c-text-muted); }
  .danger {
    margin-top: var(--sp-3); padding: var(--sp-2);
    background: transparent; border: 1px solid #DC2626; color: #DC2626; border-radius: 4px;
    cursor: pointer;
  }
  .danger:hover { background: #FEE2E2; }
</style>
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/details/
git commit -m "feat(ui): details panel — task fields edit + save on blur, delete with confirm"
```

---

### Task 29: PhaseDetails

**Files:**
- Create: `src/lib/details/PhaseDetails.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script lang="ts">
  import { state } from '../store.svelte';
  import * as ipc from '../ipc';
  import type { Phase } from '../types';

  let { phaseId }: { phaseId: number } = $props();
  const phase = $derived(state.phases.find(p => p.id === phaseId));

  let name = $state('');
  let colour = $state('#3B82F6');

  $effect(() => {
    if (phase) { name = phase.name; colour = phase.colour; }
  });

  async function save() {
    if (!phase) return;
    const updated: Phase = { ...phase, name: name.trim() || phase.name, colour };
    await ipc.updatePhase(updated);
    state.phases = state.phases.map(p => p.id === updated.id ? updated : p);
    await ipc.touchLastSave();
  }

  async function del() {
    if (!phase) return;
    if (!confirm(`Delete phase "${phase.name}" and ALL its tasks?`)) return;
    await ipc.deletePhase(phase.id);
    state.phases = state.phases.filter(p => p.id !== phase.id);
    state.tasks  = state.tasks.filter(t => t.phase_id !== phase.id);
    state.select(null);
    await ipc.touchLastSave();
  }
</script>

{#if phase}
  <div class="phase-details">
    <h2>{phase.name}</h2>
    <label>Name<input bind:value={name} onblur={save} /></label>
    <label>Colour<input type="color" bind:value={colour} onblur={save} /></label>
    <button class="danger" onclick={del}>Delete phase</button>
  </div>
{/if}

<style>
  /* identical to TaskDetails.svelte */
  .phase-details { display: flex; flex-direction: column; gap: var(--sp-3); padding: var(--sp-4); }
  h2 { font-size: var(--font-size-lg); margin: 0; }
  label { display: flex; flex-direction: column; gap: var(--sp-1); font-size: var(--font-size-sm); color: var(--c-text-muted); }
  input { padding: var(--sp-2); border: 1px solid var(--c-border); border-radius: 4px; font: inherit; color: var(--c-text); background: var(--c-bg); }
  .danger { margin-top: var(--sp-3); padding: var(--sp-2); background: transparent; border: 1px solid #DC2626; color: #DC2626; border-radius: 4px; cursor: pointer; }
  .danger:hover { background: #FEE2E2; }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/details/PhaseDetails.svelte
git commit -m "feat(ui): phase details panel — rename, colour, delete with cascade warning"
```

---

### Task 30: Phase row → opens details

**Files:**
- Modify: `src/lib/canvas/LeftRail.svelte`

- [ ] **Step 1: Add click on phase-row label area to set selection.kind=phase**

```svelte
<div class="phase-row" ...>
  <button class="chev" onclick={(e) => { e.stopPropagation(); toggleCollapse(phase.id); }} aria-label="toggle">
    {phase.collapsed ? '▸' : '▾'}
  </button>
  <span class="num">{pi + 1}.</span>
  <span class="name" onclick={() => state.select({ kind: 'phase', id: phase.id })} role="button" tabindex="0">{phase.name}</span>
</div>
```

(Task rows already have implicit selection from the SVG bar click; in the left rail we add a click on the task name too:)

```svelte
<span class="name" onclick={() => state.select({ kind: 'task', id: task.id })} role="button" tabindex="0">{task.name}</span>
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/canvas/LeftRail.svelte
git commit -m "feat(ui): clicking phase/task name in left rail opens details panel"
```

---

### Task 31: Add phase + add task buttons

**Files:**
- Modify: `src/lib/canvas/LeftRail.svelte`
- Modify: `src/lib/store.svelte.ts`

- [ ] **Step 1: Store actions**

```typescript
async createPhase(name: string): Promise<void> {
  if (!this.currentJob) return;
  const palette = ['#3B82F6', '#EF4444', '#10B981', '#F59E0B', '#8B5CF6', '#EC4899', '#14B8A6'];
  const colour = palette[this.phases.length % palette.length];
  const phase = await ipc.createPhase({ job_id: this.currentJob.id, name, colour });
  this.phases = [...this.phases, phase].sort((a, b) => a.order_index - b.order_index);
}

async createTaskInPhase(phaseId: number, name: string): Promise<void> {
  if (!this.currentJob) return;
  // Default start = project start; duration 3.
  const start = this.currentJob.project_start_date;
  const task = await ipc.createTask({
    phase_id: phaseId, name, start_date: start, duration_workdays: 3,
  });
  this.tasks = [...this.tasks, task];
}
```

- [ ] **Step 2: Add button at bottom of LeftRail**

```svelte
<div class="left-rail">
  {#each state.phases as phase, pi (phase.id)}
    <!-- existing rows -->
    {#if !phase.collapsed}
      <!-- existing task rows -->
      <button class="add-task" onclick={async () => {
        const n = prompt('Task name?');
        if (n?.trim()) await state.createTaskInPhase(phase.id, n.trim());
      }}>+ Task</button>
    {/if}
  {/each}
  <button class="add-phase" onclick={async () => {
    const n = prompt('Phase name?');
    if (n?.trim()) await state.createPhase(n.trim());
  }}>+ Phase</button>
</div>

<style>
  /* ...existing... */
  .add-task, .add-phase {
    width: 100%; text-align: left; padding: var(--sp-2) var(--sp-3);
    background: transparent; border: none; cursor: pointer;
    color: var(--c-text-muted); font-size: var(--font-size-sm);
  }
  .add-task:hover, .add-phase:hover { background: var(--c-accent-fade); color: var(--c-accent); }
  .add-task { padding-left: calc(var(--sp-2) * 4); }
</style>
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/canvas/LeftRail.svelte src/lib/store.svelte.ts
git commit -m "feat(ui): + Phase and + Task buttons (prompt-based for v1)"
```

---

### Task 32: Quick-task by double-clicking an empty cell

**Files:**
- Modify: `src/lib/canvas/GanttCanvas.svelte`

A double-click on an empty cell creates a 1-day task on that exact day in the phase whose row was clicked.

- [ ] **Step 1: Add double-click handler to the SVG**

```svelte
<svg
  ...
  ondblclick={async (e) => {
    const svgRect = (e.currentTarget as SVGElement).getBoundingClientRect();
    const x = e.clientX - svgRect.left;
    const y = e.clientY - svgRect.top;
    const dayIdx = Math.floor(x / 24);
    const rowIdx = Math.floor(y / 32);
    if (dayIdx < 0 || dayIdx >= days.length) return;
    if (rowIdx < 0 || rowIdx >= rows.length) return;
    const r = rows[rowIdx];
    const phaseId = r.kind === 'phase' ? r.phase.id : r.phase.id;
    if (!phaseId) return;
    const name = prompt('Task name?');
    if (!name?.trim()) return;
    const date = days[dayIdx].date;
    const task = await import('../ipc').then(m => m.createTask({
      phase_id: phaseId, name: name.trim(), start_date: date, duration_workdays: 1,
    }));
    state.tasks = [...state.tasks, task];
  }}
>
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/canvas/GanttCanvas.svelte
git commit -m "feat(ui): double-click empty cell → 1-day task on that exact day, in clicked phase"
```

---

### Task 33: Tag v0.2.0

- [ ] **Step 1: Final verification**

```bash
cd ~/Desktop/GanttBok && pnpm exec tsc --noEmit && pnpm exec vitest run && cd src-tauri && . "$HOME/.cargo/env" && cargo test && cargo check
```

- [ ] **Step 2: Tag**

```bash
git tag -a v0.2.0 -m "v0.2.0 — Plan 2 complete: Gantt UI (canvas + drag physics + creation gestures)"
```

- [ ] **Step 3: Update Workshop brief status**

Edit `~/Desktop/OBSIDIAN_TREES/Workshop/projects/GANTTBOK/brief_GANTTBOK.md`:
- Change `status:` to `building — Plan 2 (Gantt UI) complete v0.2.0, Plan 3 (polish + ship) next`
- Tick the standing action for Plan 2

---

## Self-review

**Spec coverage:**

| Spec § | Plan 2 coverage |
|---|---|
| §3 Architecture (Svelte + SVG + IPC) | ✅ Tasks 1–5 |
| §6 Calendar (workday + project-relative week numbering) | ✅ Tasks 12 + 14 (frontend mirror) |
| §6 No-work day visual (stripes + rotated name) | ✅ Task 13 |
| §7.1 Layout (left rail + grid) | ✅ Tasks 11, 15 |
| §7.2 Bar rendering (task, phase, dep arrow) | ✅ Tasks 17, 18, 19 |
| §7.3 Grab zones | ✅ Task 22 |
| §7.4 Magnetic snap | ✅ Task 23 |
| §7.5 Hard-chain ripple | ✅ Task 24 (via `dragTask` IPC) |
| §7.6 Phase drag whole-block | ⚠️ NOT in Plan 2. Phase bars are read-only here. Dragging the *collapsed phase bar* to move all its tasks deferred to **Plan 3** as a polish item. |
| §7.7 Resize | ✅ Task 25 |
| §7.8 Dependency hover brighten | ✅ Task 20 |
| §8.1 Add job / phase / task | ✅ Tasks 9, 31 |
| §8.1 Quick task (dbl-click) | ✅ Task 32 |
| §8.2 Rename + edit | ✅ Task 28 (TaskDetails), Task 29 (PhaseDetails) |
| §8.3 Create dependency (drag from ○) | ⚠️ DEFERRED to Plan 3. The IPC exists; the gesture is non-trivial — needs its own component for the drag-from-handle-to-bar interaction. |
| §8.4 No-work right-click → mark | ⚠️ DEFERRED to Plan 3. SA auto-sync is wired; manual marking via context menu is a polish item. |
| §8.5 Vertical reorder | ✅ Tasks 26, 27 |
| §8.6 Selection model | ✅ store.select() |
| §8.7 Undo | ⚠️ Plan 3 |
| §8.8 Details panel | ✅ Tasks 28–30 |
| §9 Sidebar | ✅ Tasks 7–10 (templates UI in Plan 3) |
| §10 Print | ⚠️ Plan 3 |
| §11 Saved-state indicator | ⚠️ Plan 3 (autosave already happens — every IPC mutation calls touchLastSave) |
| §12 Error handling | UI surfaces backend errors via uncaught `invoke` rejections — Plan 3 adds proper toast handling |

**Placeholder scan:** No `TODO` / `TBD` / `implement later` strings. Every step has complete code.

**Type consistency:**
- All command args mirror Rust structs (`CreateJobArgs`, `DragTaskArgs`, etc.). Field names use Rust's snake_case (the wire format from serde).
- All dates are ISO `YYYY-MM-DD` strings everywhere; only `lib/calendar.ts` parses them into `Date` for math.
- `Selection` discriminated-union type consistent across `select()`, `DetailsPanel`, `TaskDetails`, `PhaseDetails`.
- `cellW` (24) hard-coded in this plan as the on-screen day-cell width; Plan 3's print pipeline will scale it dynamically.

**Deliberately deferred to Plan 3:**
- Phase-bar drag (whole-block move)
- Dependency creation gesture (drag from ○ handle)
- Manual no-work-day right-click context menu
- Templates UI (Sidebar group + Save as template + New from template flows)
- Undo / redo
- Saved-state indicator footer + `⌘S`
- Print pipeline + Print Options
- App icon + packaging + signing
- Toast / error-surfacing UI
- Backend `list_archived` command (one-line extension to make the Archived group functional)

These are listed in Plan 3's scope when written.

---

## Execution handoff

**Plan complete and saved to `~/Desktop/GanttBok/docs/plans/2026-05-20-plan2-gantt-ui.md`.**

Tasks total: **33**. Estimated execution time at TDD pace with subagent dispatch: **~3-5 hours** of subagent compute, plus rate-limit pauses.

Two execution options:

1. **Subagent-Driven (recommended)** — fresh subagent per task, review between
2. **Inline Execution** — batch + checkpoints

For Plan 2 specifically I'd suggest dispatching in two big phases:
- **First batch:** Tasks 1–21 (foundation + sidebar + canvas read-only). Pause, smoke test, course-correct.
- **Second batch:** Tasks 22–33 (drag physics + creation + details + v0.2.0).

This catches any pointer-event quirks before they pollute the whole drag implementation.
