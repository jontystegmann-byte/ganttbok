# Gantt Bok — Design Specification

**Date:** 2026-05-19
**Author:** Jonty Stegmann (with Mr Crabs)
**Status:** Approved design, ready for implementation planning
**Target user:** Gray Robertson (architect, project manager, Cape Town apartment renovations)

---

## 1. Purpose

Gantt Bok is a single-user macOS desktop app that lets Gray plan, manage, and print Gantt charts for apartment renovations. It is offline, self-contained, lives in the Mac dock, and is built around the specific way Gray thinks about a project — week-numbered, Monday-to-Friday, with collapsible phases, hard dependency chains, and physically intuitive drag interactions.

The app replaces ad-hoc planning (paper, spreadsheets, PDFs) with one piece of software that mirrors his mental model and prints a clean A3-landscape plan he can take to site.

---

## 2. Scope

### In scope (v1)
- macOS dock app, self-contained binary, offline-only
- Single-user (Gray); no auth, no sync, no sharing
- Built-in library of jobs (multiple jobs managed inside one app)
- One Gantt chart per job
- Phase / task two-level hierarchy with collapsible phases
- Finish-to-Start dependencies between tasks, with hard-chain ripple on drag
- Magnetic-snap drag-to-move and drag-to-resize of bars
- Drag-to-reorder vertical ordering of phases and tasks
- Hierarchical numeric labels (1, 1.1, 1.2, 2, 2.1, …)
- Workday-only calendar (Mon-Fri); manual no-work day overrides
- Auto-synced South African public holidays
- Project-relative week numbering (Week 1, Week 2, …)
- Week-numbered header showing `M T W T F` single-letter day labels; only Monday cell shows date-of-month
- Templates (phases + tasks only — no dependencies, no durations, no dates)
- A3-landscape print pipeline (PDF via native macOS print dialog)
- Autosave with debounce + visible saved-state indicator + manual `⌘S`
- Unlimited session-scoped undo / redo

### Explicitly out of scope (v1)
- Resource / assignee / crew management
- Cost tracking, billing, quoting
- Percent-complete or progress tracking
- Multi-user, sync, cloud, sharing
- Dependency types other than Finish-to-Start
- iOS / iPad / Windows / Linux versions
- File attachments to tasks (photos, PDFs)
- Persisted undo across sessions

These may come in v2; the data model accommodates them without breaking changes.

---

## 3. Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Gantt Bok .app  (Mac dock app, ~12 MB, self-contained) │
├─────────────────────────────────────────────────────────┤
│                                                         │
│   Tauri Shell  (Rust)                                   │
│   ├── Window / dock / menus / native print              │
│   ├── SQLite database  (~/Library/.../ganttbok.db)      │
│   └── Backend commands  (load_jobs, save_task, …)       │
│                                                         │
│   ─────────  IPC boundary  (JSON commands)  ─────────   │
│                                                         │
│   WebView Frontend  (Svelte + SVG)                      │
│   ├── Sidebar  (job library)                            │
│   ├── Gantt canvas  (SVG, the heart of the app)         │
│   ├── Interaction layer  (Motion One physics)           │
│   └── Print view  (separate stylesheet, A3 landscape)   │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**Three clean layers, one boundary:**

1. **Tauri shell (Rust)** — window management, SQLite, file I/O, native print dialog, dock icon. Exposes a small set of typed commands to the frontend.
2. **IPC boundary** — frontend calls `invoke('save_task', { … })`, gets a typed result back. Single point of contact between the two halves.
3. **WebView frontend (Svelte + SVG)** — smooth drag, magnetic snap, dependency-chain ripple, hot styling, print stylesheet.

**Why Tauri** (not Electron, not native SwiftUI):
- **vs Electron:** ~12 MB bundle vs ~120 MB; faster cold-start; same web-stack ergonomics.
- **vs native SwiftUI:** the Gantt canvas (custom drag, dependency lines, magnetic snap, ripple) is significantly easier in SVG + JS than in SwiftUI's custom drawing primitives. SwiftUI's drag-and-drop story for arbitrary canvas elements is rough.
- **Print pipeline:** the WebView triggers the native macOS print dialog using a print-specific CSS stylesheet. Single source of truth (SVG → screen *and* paper); no second render path to maintain.

**Why Svelte** (not React, not Solid):
- Compiler emits direct DOM updates (no virtual DOM overhead) — matters during 60 fps drag where ten downstream bars move every frame.
- Reactive `$:` syntax maps cleanly onto the dependency engine ("when this changes, recompute that").

**Why SVG** (not `<canvas>`):
- Each bar is a real DOM node → hover / cursor / tooltip / click come for free.
- Printed output is real vector graphics — sharp at any zoom.
- Performance is fine up to a few thousand bars. Gray's jobs are ~50.

**Why no Gantt library** (no `dhtmlx-gantt`, `bryntum`, `frappe-gantt`):
- None of them do week-numbered *project-relative* headers with single-letter day labels and Monday-only date numbers. Gray's UX is too specific.
- Building SVG + drag from scratch is ~600 lines and gets us exactly the model we want.

---

## 4. Data model

SQLite, normalised, foreign-keys-on. Six tables.

```sql
─── job ──────────────────────────────────────────────────────
  id                 INTEGER  PK
  name               TEXT     "Sea Point apartment reno"
  client             TEXT     (optional, for print header)
  address            TEXT     (optional, for print header)
  project_start_date DATE     real calendar date, any weekday
  is_template        BOOLEAN  default 0
  archived           BOOLEAN  default 0
  created_at         DATETIME

─── phase ────────────────────────────────────────────────────
  id           INTEGER  PK
  job_id       INTEGER  FK → job.id   ON DELETE CASCADE
  name         TEXT     "Plumbing"
  colour       TEXT     hex; defaults from a palette
  order_index  INTEGER  drives "1, 2, 3..." numbering + vertical position
  collapsed    BOOLEAN  default 1    (collapsed by default)

─── task ─────────────────────────────────────────────────────
  id                INTEGER  PK
  phase_id          INTEGER  FK → phase.id  ON DELETE CASCADE
  name              TEXT     "First-fix"
  start_date        DATE     real calendar date
  duration_workdays INTEGER  ≥1, in workdays (excludes Sat/Sun)
  order_index       INTEGER  drives "1, 2, 3..." numbering + vertical position within phase
  notes             TEXT     optional, free text

─── dependency ───────────────────────────────────────────────
  id              INTEGER  PK
  predecessor_id  INTEGER  FK → task.id   ON DELETE CASCADE
  successor_id    INTEGER  FK → task.id   ON DELETE CASCADE
  type            TEXT     'FS' for v1 (Finish-to-Start)
  lag_days        INTEGER  workdays; can be 0 or positive
  UNIQUE(predecessor_id, successor_id)

─── no_work_day ──────────────────────────────────────────────
  id      INTEGER  PK
  job_id  INTEGER  FK → job.id    ON DELETE CASCADE
  date    DATE     a specific date marked non-working
  reason  TEXT     "Youth Day", "Rain", "Site closed"
  source  TEXT     'sa_public_holiday' | 'manual'

─── app_meta ─────────────────────────────────────────────────
  key    TEXT  PK   (schema_version, last_open_job_id, sidebar_width, …)
  value  TEXT
```

### Design decisions

1. **`start_date` is a real DATE**, not a workday-index. Gray thinks in calendar dates; the model stores them as such. The renderer computes the visible end of a bar by walking forward `duration_workdays` workdays, skipping Saturdays, Sundays, and `no_work_day` entries.

2. **Phase membership is exactly one-to-many.** A task belongs to exactly one phase via `phase_id`. To re-parent, update the FK. No many-to-many — apartment renos don't need it.

3. **Dependencies live at the leaf (task) level.** The "Plumbing → Tiling" arrow visible on the collapsed view is computed from the last-task-of-Plumbing → first-task-of-Tiling dependency.

4. **No-work days are visual only.** They do *not* shift bars or trigger dependency ripple. Gray's team makes up the time; the calendar stays honest. SA public holidays auto-populate; manual entries are also supported.

5. **Templates are `is_template=true` jobs** carrying only phases + tasks (no dependencies, no durations, no dates). Instantiation copies the skeleton; everything else is set fresh by Gray for the new reno.

### What's deliberately omitted (YAGNI)
- No `assignee` / `crew` / `cost` columns.
- No `task.percent_complete`.
- No dependency types beyond Finish-to-Start.
- No multi-user / sync columns.

---

## 5. Hierarchical numbering

Pure numeric, derived from `order_index` (never stored as a column):

```
1.    Plumbing            (phase, order_index=0)
  1.1  Toilet rough-in    (task,  order_index=0)
  1.2  Sink rough-in      (task,  order_index=1)
  1.3  Pressure test      (task,  order_index=2)
2.    Electrical          (phase, order_index=1)
  2.1  First-fix wiring   (task,  order_index=0)
  2.2  Second-fix         (task,  order_index=1)
```

Numbers re-derive automatically on reorder. The number prints in both the on-screen left rail and the printed sheet — Gray references tasks verbally by number ("get 2.2 done before the painter arrives").

---

## 6. Calendar and week numbering

### Workdays
- Mon-Fri are workdays. Sat/Sun are never shown on the chart.
- A "no-work day" is a date marked off via `no_work_day` (auto-populated SA public holidays + Gray's manual entries). No-work days render with diagonal-stripe grey backgrounds across the entire vertical column.

### Project-relative week numbering
- **Week 1** = the week containing `job.project_start_date`.
- Subsequent weeks numbered sequentially (Week 2, Week 3, …, Week 18).
- This is *not* the ISO week number — it's relative to the project. Builders talk in project weeks ("we're in week 6").

### Header rendering
```
│ Week 1                  │ Week 2                  │ Week 3
│ M    T  W  T  F         │ M    T  W  T  F         │ M    T  W  T  F
│ 03   ·  ·  ·  ·         │ 09   ·  ·  ·  ·         │ 16  ·  ·  ·  ·
```

- Each week column = 5 day cells labelled with single letters `M T W T F`.
- Only the Monday cell shows the date-of-month number. The others are blank — Gray's brain fills them in.
- Cells before `project_start_date` in Week 1 render normally (just empty bars in those rows). No special treatment.

### South African public holidays (auto-sync)

Computed in Rust on job-create and re-checked when the project extends past December. Twelve holidays per year:

| Holiday | Date rule |
|---|---|
| New Year's Day | 1 Jan |
| Human Rights Day | 21 Mar |
| Good Friday | Easter Sunday − 2 |
| Family Day | Easter Sunday + 1 |
| Freedom Day | 27 Apr |
| Workers' Day | 1 May |
| Youth Day | 16 Jun |
| National Women's Day | 9 Aug |
| Heritage Day | 24 Sep |
| Day of Reconciliation | 16 Dec |
| Christmas Day | 25 Dec |
| Day of Goodwill | 26 Dec |

Plus the **SA Public Holidays Act 1994** rule: if a fixed holiday falls on a Sunday, the following Monday is observed.

Inserted into `no_work_day` with `source = 'sa_public_holiday'`. Gray can delete one (his team works that day) or add manual entries (`source = 'manual'`); the auto-sync never overwrites manual entries.

---

## 7. Gantt canvas — layout and interaction

### 7.1 Layout

```
┌──────────────────┬──────────────────────────────────────────────────────┐
│  Left rail       │  Time grid  (Week 1, 2, 3 … horizontal scroll)       │
│  (sticky)        │                                                       │
├──────────────────┼──────────────────────────────────────────────────────┤
│ 1.  Plumbing  ▾ │ ████████████                                          │ ← phase bar
│   1.1 First-fix │   ███                                                  │
│   1.2 Sink      │       █                                                │
│   1.3 Pressure  │         █                                              │
├──────────────────┼──────────────────────────────────────────────────────┤
│ 2.  Electrical ▸ │              ██████████                              │ ← collapsed phase
├──────────────────┼──────────────────────────────────────────────────────┤
│ 3.  Tiling     ▸ │                          ████████                    │
└──────────────────┴──────────────────────────────────────────────────────┘
```

- **Left rail** (fixed width, sticky on horizontal scroll) — phase/task labels with hierarchical number, expand/collapse chevron, drag handle for vertical reorder.
- **Time grid** — horizontal columns: one per workday, grouped visually into weeks. Subtle vertical lines between days; bolder between weeks.
- **Rows** — every phase is a row; every task is a row when its phase is expanded. Fixed height (~32 px on screen, ~8 mm on A3 print).
- **All phases collapsed by default** when a job is opened. Collapsed state stored per-phase (`phase.collapsed`).

### 7.2 Bar rendering (SVG)

| Bar type | Look | Width | Special |
|---|---|---|---|
| **Task bar** | Filled rect, phase colour, rounded 3 px corners | `duration_workdays × day_cell_width` | Hover → drop shadow + edge resize handles appear |
| **Phase bar** | Hollow / lighter-tint rect spanning earliest-task-start to latest-task-end | computed | Only visible when phase is collapsed |
| **Dependency line** | SVG path: right-edge of predecessor → ↓ → ← left-edge of successor, arrowhead at end | n/a | Subtle grey; brightens to phase colour when either endpoint is hovered |

### 7.3 Grab zones and drag gestures

```
       ┌─┬──────────────┬─┐
       │↔│   move grip  │↔│
       └─┴──────────────┴─┘
       left ~10%       right ~10%
       = resize start  = resize end
       middle ~80%     = move
```

### 7.4 Magnetic snap

While dragging:
- Bar follows cursor with sub-pixel precision.
- A snap force pulls toward the nearest day-edge — strength scales with distance (sigmoid pull): ≤30% of cell-width = magnetic pull, >70% = no pull (free movement).
- On release, the bar locks to the nearest day.

### 7.5 Hard-chain ripple

When the dragged bar's `start_date` shifts by N workdays, **same frame**:
- Every transitively-dependent successor task has its `start_date` provisionally shifted by N (respecting its own lag).
- A pre-computed dependency graph (built once on job-load, mutated on dependency-add/delete) lets us walk in O(downstream-task-count). Trivial for 50-task jobs.
- All shifted bars re-render in the same `requestAnimationFrame` tick. Rigid, no animation between frames.

### 7.6 Phase drag = whole-block move

Grabbing a collapsed phase bar moves every task inside it as a rigid group, internal gaps preserved. Then the chain ripple fires from the phase's downstream dependencies.

### 7.7 Resize

- **Right edge:** only `duration_workdays` changes; `start_date` stays. Chain ripple fires.
- **Left edge:** both `start_date` and `duration_workdays` change so the right edge stays put. Chain ripple fires from the new start.
- Minimum duration: 1 workday (clamped — bar can't shrink below).

### 7.8 Dependency lines

- Default: subtle grey.
- Hover over either endpoint bar → connected line(s) + connected bar(s) brighten to the phase colour, instantly showing the chain.

### 7.9 Physics library

**Motion One** (~12 KB, successor to Popmotion, from the Framer Motion author).
- Used for non-drag animations: scroll-into-view on new-task-create, collapse/expand phase animations, undo flashes.
- Drag itself uses a hand-written RAF loop — we write `x` directly each frame for rigid same-frame ripple. Motion's API isn't on the hot path.

---

## 8. Creation and editing gestures

### 8.1 Adding things

| Action | Gesture | Result |
|---|---|---|
| New job | `⌘N` or "+ New job" in sidebar | Modal: name, client (opt), address (opt), project start date, start from `Blank` or template. |
| New phase | `⌘⇧P` or "+ Phase" button at bottom of left rail | Appended to end. Default name "New phase", default colour next in palette. Inline-rename auto-activates. |
| New task | `⌘T` while a phase is selected, OR "+" button on phase row | Appended to end of that phase. Default duration 3 workdays. Default start = end-date of previous task in phase, OR phase start if first. Inline-rename auto-activates. |
| Quick task | Double-click an empty cell in the grid | Creates a 1-day task on that exact day, in whichever phase row the click lands in. |

### 8.2 Editing things

- **Rename:** double-click the label in the left rail → inline text field.
- **Change duration:** drag the right edge of the bar, OR right-hand details panel (numeric input).
- **Change start:** drag the bar middle, OR right-hand details panel (date picker).
- **Notes:** in the right-hand details panel. A small `•` indicator on the bar shows when notes exist.
- **Delete:** select bar/row → `Backspace`. Confirm modal only if the task has dependencies.

### 8.3 Dependencies

| Action | Gesture |
|---|---|
| **Create dependency** | Hover the right-edge of bar A → small `○` handle appears. Drag from `○` onto bar B → creates Finish-to-Start dependency A → B. Circular chains rejected with a red flash (no modal). |
| **Add lag** | Click the dependency line → details panel shows lag days. Default 0. |
| **Delete dependency** | Click the line → `Backspace` or trash icon in details panel. |
| **See connections** | Hover any bar → all its dependency lines and connected bars brighten. |

### 8.4 No-work days

- SA public holidays appear automatically — diagonal-stripe column background, vertically-rotated holiday name in column header.
- Manual override: right-click any day-column header → "Mark non-working day…" / "Mark working day". Stored with `source = 'manual'`.
- Hover a striped column → tooltip with reason ("Youth Day", "Site closed", etc.).

### 8.5 Vertical reorder

- **Phase reorder:** grab the phase row's left-rail label area (not the bar), drag up/down. Phase numbers (1, 2, 3, …) renumber automatically.
- **Task reorder within phase:** expand phase, grab a task row's left handle, drag up/down inside that phase. Numbers (1.1, 1.2, …) renumber automatically.
- Reorder is **vertical only**; it does *not* change a task's date. Horizontal drag is for time.

### 8.6 Selection model

- Single-click a bar/row → selected (blue outline). Right-hand details panel slides in.
- Click empty space → deselect; panel slides away.
- `⌘`-click multiple bars → multi-select (data model supports it; bulk actions are not a v1 priority).

### 8.7 Undo

- `⌘Z` / `⌘⇧Z` for unlimited undo/redo, **session-scoped**.
- Every state-changing action (drag, create, delete, rename, dependency-add, no-work-mark, reorder) pushes onto the stack.
- Closed app = stack cleared. (Not persisted across sessions.)

### 8.8 Right-hand details panel

```
┌──────────────┬─────────────────────────┬───────────────┐
│  Sidebar     │   Gantt canvas          │  Details      │
│  (jobs list) │                         │  Name: ___    │
│              │   ████████              │  Start: ___   │
│              │                         │  Duration: __ │
│              │                         │  Notes: ___   │
│              │                         │  Depends on:  │
│              │                         │   • [list]    │
│              │                         │  [Delete]     │
└──────────────┴─────────────────────────┴───────────────┘
```

- Slides in from the right when a bar / row / dependency line is selected.
- Width fixed (~300 px). Slides away when selection clears.

---

## 9. Sidebar / job library

```
┌──────────────────────┐
│ Gantt Bok            │
│ ─────────────────────│
│ ⌕  Search jobs...    │
│                      │
│ ACTIVE               │
│  ● Sea Point reno    │ ← currently open (dot indicator)
│    Tamboerskloof     │
│    Camps Bay deck    │
│    Vredehoek garden  │
│                      │
│ TEMPLATES            │
│    Standard apt reno │
│    Bathroom only     │
│    Kitchen overhaul  │
│                      │
│ ARCHIVED  ▸          │
│                      │
│ ─────────────────────│
│  + New job           │
└──────────────────────┘
```

- **Search** filters in-place across job name, client, address.
- **Active** ordered by most-recently-opened. The open job is highlighted with a coloured left-edge bar.
- **Templates** — own group. Editing one opens it on the Gantt canvas with a thin "Template" banner.
- **Archived** — collapsed by default; expandable.
- **Per-job context menu** (right-click): Rename, Duplicate, Archive (or Unarchive), Save as template, Export to PDF, Delete (with confirm).
- **Sidebar width** resizable by dragging the divider. Stored in `app_meta`.

### Templates (full specification)

A template carries **only**:
- Phase names + order + colours
- Task names + order within each phase

It does **not** carry:
- Dependencies
- Durations
- Start dates / project start date

When Gray instantiates a template via "+ New job" → "Start from: Template X":
1. Modal asks for new job name, client, address, project start date.
2. Phases copied with names + order + colours.
3. Tasks copied with names + order; every task created with `duration_workdays = 1` and `start_date = project_start_date`.
4. All tasks stack on the leftmost day until Gray drags them out — forcing conscious decisions about duration and order for *this* reno.

### Duplicate (separate from templates)

"Duplicate" on any job clones it preserving its current dates, dependencies, and durations. Use case: "this new reno is essentially Camps Bay shifted 2 weeks later."

---

## 10. Print pipeline

### 10.1 Printed sheet layout

```
┌───────────────────────────────────────────────────────────────────────┐
│  SEA POINT APARTMENT RENOVATION                                       │
│  Client: M. Botha   ·   Address: 12 Marine Rd   ·   Printed: 19 May   │
├───────────────────────────────────────────────────────────────────────┤
│           │ Week 1          │ Week 2          │ Week 3          │ ... │
│           │ M  T  W  T  F   │ M  T  W  T  F   │ M  T  W  T  F   │     │
│           │ 03 ·  ·  ·  ·   │ 09 ·  ·  ·  ·   │ 16 ·  ·  ·  ·   │     │
│ 1.Plumb.  │     ████████    │ ████            │                 │     │
│ 1.1 Toilet│     ███         │                 │                 │     │
│ 1.2 Sink  │         █       │                 │                 │     │
│ 2.Elec.   │                 │ █████████████   │ ████            │     │
│ 3.Tiling  │                 │                 │ ████████████    │     │
├───────────────────────────────────────────────────────────────────────┤
│  Public holidays in this range: 16 Jun (Youth Day)                    │
└───────────────────────────────────────────────────────────────────────┘
```

- **Header strip:** job name, client, address, print date.
- **Chart:** identical visual language to on-screen.
- **What's collapsed on screen prints collapsed.** Gray controls print resolution by deciding what to expand before printing.
- **Footer:** public holidays in the project's span, listed by date. Redundant insurance over the in-chart marking.

### 10.2 Public holiday visual treatment (on screen and on print)

- **Day-column background:** diagonal-stripe grey fill across the entire vertical span of the chart for that column.
- **Vertically-rotated holiday name** inside the column header (fine print).
- **Bars draw continuously across** — they do not visually break around the no-work day (the team makes up the time).
- On-screen only: hover tooltip with full reason text.

### 10.3 How printing works

**WebView native print** (single source of truth — the SVG):
- The print-specific CSS stylesheet hides the sidebar, details panel, scrollbars, app chrome.
- `@page { size: A3 landscape; margin: 10mm; }`.
- Chart is scaled (via `transform: scale(...)`) to fit page width when "Fit to page" is selected.
- Colours are locked to print-friendly versions (avoids ink-flood).

Gray hits `⌘P` → in-app "Print Options" sheet → native macOS print dialog.

### 10.4 Print Options sheet

```
┌──────────────────────────────────────┐
│  Print Plan                          │
│                                      │
│  Page size:    [ A3 landscape  ▾ ]   │
│  Scaling:      ● Fit to page         │
│                ○ Multi-page          │
│  Show notes:   ☐ (off by default)    │
│                                      │
│         [ Cancel ]   [ Print → ]     │
└──────────────────────────────────────┘
```

- **Fit to page** (default): chart scales to fit A3 width, no matter how cramped.
- **Multi-page**: maintains a minimum 6 mm per day-column; long jobs spill across multiple sheets with a "continued" indicator.
- **Show notes**: appends a notes section per task at the bottom.

After Print → standard macOS print dialog (printer, AirPrint, Save as PDF).

---

## 11. Persistence and saved-state indicator

### 11.1 Autosave

- All state-changing actions persist immediately (via Tauri command → SQLite).
- Drags debounced 500 ms (only the final position writes).
- SQLite writes wrapped in transactions for multi-row mutations (drag-with-chain-ripple = one transaction).

### 11.2 Manual save

- `⌘S` or click the saved-state indicator forces immediate flush.

### 11.3 Saved-state indicator

Bottom of the window, right-aligned, fine print:

```
…                                                Saved 16:42  ⌘S
```

States:
- `Saved HH:MM` — normal.
- `Saving…` (dimmed) — during the 500ms debounce window.
- `Save failed — click to retry` (red) — if disk full / perms / lock contention.

### 11.4 Backup

No app-side backup logic. Time Machine handles it. The SQLite file lives at:

```
~/Library/Application Support/Gantt Bok/ganttbok.db
```

---

## 12. Error handling

### 12.1 Data integrity

- All multi-row mutations in transactions.
- `PRAGMA foreign_keys = ON;`. Cascades on phase / task delete.
- Schema migrations versioned in `app_meta.schema_version`. On app start, Rust checks and applies pending migrations inside a transaction. On migration failure: error dialog pointing at Time Machine.

### 12.2 Concurrent open

- SQLite opened with exclusive locking. Second app instance refuses to launch with a clear "Gantt Bok is already running" message.

### 12.3 Drag conflicts

- **Circular dependency creation** (A → B → A): detected at drop, brief red flash, link rejected silently.
- **Resize below 1 workday**: clamped at 1 workday minimum. Bar cannot shrink further.
- **Dragging too far left**: soft floor at `project_start_date - 30 days`. Bar cannot pass.

### 12.4 UI errors

- **Crash recovery:** on app launch, check `app_meta.clean_shutdown`. If false, banner: "Gantt Bok didn't close properly last time — your last save was 19 May 16:42. Continue?"
- **Empty name on rename:** field stays open until typed or Escaped (Escape cancels, restores previous).
- **No "unsaved changes" dialog ever** — every action autosaves.

### 12.5 Print errors

- If OS print dialog fails (no printers): fall back to "Save as PDF". Never silently fail.

---

## 13. Testing strategy

### 13.1 Layered

| Layer | Type | What gets covered |
|---|---|---|
| **Rust core** | `cargo test` | Dependency graph: shift, cycle detection, ripple ordering. Date math: workday-walk, public-holiday computation (Easter algorithm), week-number-from-project-start. SQLite layer: transaction rollback, FK cascades. |
| **Frontend logic** | Vitest | Snap function, hit-test (grab-zone resolution), magnetic snap curve, hierarchical numbering, template instantiation. |
| **Frontend components** | Playwright component tests | Bar renders correctly, hover shows handles, drag updates store, right-click context menus open. |
| **End-to-end** | Playwright on Tauri | "Create job → add phase → add 3 tasks → link them → drag the first → all three move → print" — one big happy-path. Plus edge cases (no-work day, template instantiate, undo/redo). |

### 13.2 Not tested

- Print PDF byte-for-byte (too brittle). Snapshot-test the *print stylesheet's computed SVG* instead.
- macOS native print dialog (Apple's job).
- Detailed performance benchmarks. Smoke check only: "60 fps drag with 50 bars", measured once per release.

### 13.3 Pre-release manual ritual (~10 min)

1. Create blank job → add 2 phases / 5 tasks → save → close → reopen → state correct.
2. Drag a bar with 3 downstream dependencies → all three move same-frame.
3. Mark a no-work day → diagonal stripes appear → bar still draws through it.
4. Save as template → new job from template → tasks stack at start, dependencies absent, durations all 1 day.
5. Print to PDF → A3 landscape → header strip, footer strip, vertically-rotated holiday name all visible.

---

## 14. File layout (on disk)

```
~/Desktop/GanttBok/                          ← project root
├── docs/
│   └── specs/
│       └── 2026-05-19-ganttbok-design.md    ← this document
├── src-tauri/                               ← Rust crate (Tauri shell)
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs
│       ├── db/                              ← SQLite layer + migrations
│       ├── commands/                        ← IPC commands exposed to frontend
│       └── calendar/                        ← workday math + SA holidays
├── src/                                     ← Svelte frontend
│   ├── lib/
│   │   ├── gantt/                           ← canvas, bars, drag, ripple
│   │   ├── sidebar/
│   │   ├── details-panel/
│   │   └── stores/
│   ├── routes/
│   └── app.html
├── tests/
│   ├── rust/                                ← cargo tests
│   └── e2e/                                 ← Playwright
├── package.json
├── pnpm-lock.yaml
└── README.md
```

Data file (runtime, not in the repo):
```
~/Library/Application Support/Gantt Bok/ganttbok.db
```

---

## 15. Open questions / parked items for v2

- **Resource / crew assignments** — Gray's hint was "add later if needed". Data model has room for a `crew` table + `task_crew` join.
- **File attachments per task** — photos of site, PDFs of approved drawings.
- **Cost / billable hours** — if Gantt Bok ever becomes Gray's quoting tool.
- **Export to other formats** — CSV, MS Project XML (`.mpp`) — for cases where Gray needs to share with a contractor who uses something else.
- **iPad companion** — read-only view of plans on site.
- **Persisted undo** — survive app restart.
- **Multi-dependency types** — Start-to-Start, Finish-to-Finish, Start-to-Finish.

None of these block v1.

---

## 16. Approved decisions log

(For the record — these are the choices made during the brainstorming session on 2026-05-19.)

| # | Question | Answer |
|---|---|---|
| Q1 | Job scale | One Gantt per job; small apartment renos |
| Q2 | Users | Just Gray |
| Q3 | Job structure | Phases → tasks, collapsible, default-collapsed, FS dependencies |
| Q4 | Drag-vs-dependency | Hard chain |
| Q5 | Resourcing | None (YAGNI) |
| Q6 | Calendar | Mon-Fri only, manual overrides |
| Q7 | Print target | A3 landscape, single sheet (Print Options: Fit-to-page default) |
| Q8 | Runtime | Mac desktop, dock, self-contained, offline |
| Q9 | Backup | Time Machine only |
| Q10 | Snap behaviour | Magnetic (free drag with soft pull) |
| Q11 | Dependency ripple | Rigid, same-frame |
| Q12 | Phase drag | Move-as-block |
| Q13 | No-work day inside task | Visual only, no chain shift, no bar gap |
| Q14 | Pre-project cells in Week 1 | Render normally, just empty |
| — | Header numbering | Project-relative (Week 1, 2, …), not ISO |
| — | Hierarchical numbering | Numeric (1, 1.1, 1.2, 2, 2.1, …) |
| — | Anchor reframed as | `project_start_date` (any weekday) |
| — | Public holidays | SA Public Holidays Act 1994, auto-synced |
| — | Bar grab zones | 10% / 80% / 10% (resize / move / resize) |
| — | Dependency creation | Drag from `○` handle on right edge of bar A to bar B |
| — | Dependency visual | Subtle grey, brighten on hover |
| — | Quick task | Double-click empty cell → 1-day task in that row's phase |
| — | Editing UI | Right-hand details panel |
| — | New phases | Created expanded; no auto-collapse |
| — | Undo | Session-scoped, unlimited |
| — | Templates | Phases + tasks only (no deps, durations, dates) |
| — | Duplicate (separate) | Clones with dates/deps/durations preserved |
| — | Saved-state indicator | Visible bottom-right; states: Saved / Saving / Failed |
| — | Manual save | `⌘S` in addition to autosave |
| — | Pre-release ritual | 5 manual checks before each version |
| — | Name | Gantt Bok (slug `ganttbok`) |
| — | Project location | `~/Desktop/GanttBok/` |

---

*Spec finalised 2026-05-19. Next step: implementation plan via the writing-plans skill.*
