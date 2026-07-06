# Bill of Quantities page — design spec

**Date:** 2026-07-06
**App:** Blik Plan (Tauri + Svelte 5 runes + Rust/rusqlite/SQLite)
**Status:** Approved design, ready for implementation planning
**Scope:** Half A only (the BoQ page). The invoice-ingestion automation is a separate follow-up — see [Phase 2](#phase-2-out-of-scope-here).

---

## 1. Problem

Jonty runs the Noordhoek project's Bill of Quantities in LibreOffice (`~/Downloads/Bill_of_Quantities.ods`). It's hard to remember what's been ordered, what's been delivered, by whom, and when — and it lives outside the project-management app that already tracks the schedule. Blik Plan should be the single place the project is controlled from, including procurement and cost.

This spec brings the BoQ into Blik Plan as a first-class per-project page: a spreadsheet-like grid, a procurement lifecycle, a live financials view, and spreadsheet export.

## 2. Decisions locked during brainstorming

1. **Source of truth = the app.** The `.ods` is retired after a one-time import. An **Export** button regenerates a spreadsheet on demand (ODS *or* XLSX), so LibreOffice-based modelling (Room Comparison, Rates) and sharing still work.
2. **Scope split.** This build = the BoQ page (grid, financials, delivery approval, export). The invoice→BoQ→email-to-Deslin automation is Phase 2, built on this data model + the existing propose/approve inbox mechanism.
3. **BoQ is its own top-level section.** A header **view switch (Schedule ⇄ Bill of Quantities)**, co-equal with the Gantt — not a dismissible overlay. When BoQ is active it owns the whole content area; you switch back via the same control (no × close). Grid gets full width by default; **financials is a collapsible right-hand docked panel** inside the view, opened from the toolbar (not always visible).
4. **One status column: the Procurement lifecycle** (replaces the current free-text Status). `Not ordered → Quoted → Ordered → Delivered`. `Ordered` means *invoice received and paid* (money has left the account). Items may skip `Quoted`.
5. **Single Rate, overwrite semantics.** When a quote becomes an invoice, the Rate is overwritten with the final number — no quote/actual variance tracking kept. Only money that has actually left the account matters.
6. **Financials.** Headline **Spent = Ordered + Delivered**. Quotes are provisional and shown *inside the budget bar* (hatched amber), never in Spent. Sector rollups group on Trade.

## 3. Data model

### 3.1 New table `boq_item` (migration v10)

Scoped to a job exactly like `phase` (`job_id INTEGER NOT NULL REFERENCES job(id) ON DELETE CASCADE`).

| Column | Type | Notes |
|---|---|---|
| `id` | INTEGER PK | |
| `job_id` | INTEGER NOT NULL FK → job(id) ON DELETE CASCADE | |
| `order_index` | INTEGER NOT NULL | manual ordering; default = append |
| `item` | TEXT NOT NULL DEFAULT '' | the frozen first column |
| `qty` | REAL | nullable (some rows have no qty yet) |
| `unit` | TEXT | item / m² / m … |
| `rate` | REAL | current known price (quoted price while Quoted, invoiced once Ordered) |
| `trade` | TEXT | sector key: HVAC, GLAZING, CARPENTER, FLOORING, Pool … |
| `full_spec` | TEXT | |
| `w_mm` `d_mm` `h_mm` `dia_mm` | REAL | dimensions (mostly empty) |
| `supplier` | TEXT | |
| `location` | TEXT | |
| `procurement` | TEXT NOT NULL DEFAULT 'not_ordered' | CHECK IN ('not_ordered','quoted','ordered','delivered') |
| `delivered_date` | TEXT | ISO date, set when → delivered |
| `lead_weeks` | REAL | |
| `invoice_no` | TEXT | |
| `tut_ref_no` | TEXT | |
| `organisation` | TEXT | |
| `created_at` | TEXT NOT NULL | |

**Cost is not stored.** `cost = qty × rate`, computed live in the frontend and written as a live formula on export (honours the "totals must be live formulas" rule).

Monetary values stored as `REAL` rand (2dp). Acceptable for this personal tool; noted as a known simplification.

### 3.2 `job.budget` (same v10 migration)

Add `budget REAL` to the `job` table — one editable budget number per project. Nullable (no budget set → panel shows Spent/Quoted without a budget bar).

### 3.3 Column-visibility preference

Show/hide state persists in `app_meta` under key `boq_hidden_columns` (JSON array of column keys). App-global, matching how contacts are global. **Default-hidden:** `full_spec, w_mm, d_mm, h_mm, dia_mm, invoice_no, tut_ref_no, organisation`. **Default-visible:** `item, qty, unit, rate, cost, trade, supplier, location, procurement, lead_weeks`.

### 3.4 Rust structs & enum

- `BoqItem` struct in `db/models.rs` mirroring the table.
- `Procurement` enum (`NotOrdered | Quoted | Ordered | Delivered`) with string mapping, following the existing `TaskStatus` pattern (`models.rs:4-35`).

## 4. Backend (Rust / Tauri commands)

New `repo/boq.rs` (declared in `repo/mod.rs`) and `commands/boq.rs` (declared in `commands/mod.rs`, every command registered in `lib.rs` `invoke_handler`).

| Command | Purpose |
|---|---|
| `list_boq_items(job_id)` | all rows for a job, ordered by `order_index` |
| `create_boq_item(args)` | append a blank/partial row, returns it |
| `update_boq_item(args)` | edit content fields. **Must NOT change `procurement`/`delivered_date`** — mirror the `update_task_inner` guard (`task.rs:55-67`) so cell edits never clobber procurement state |
| `set_boq_procurement(id, status, delivered_date?)` | the only writer of `procurement`; sets/clears `delivered_date` when → delivered |
| `delete_boq_item(id)` | with confirm on the frontend |
| `reorder_boq_item(id, order_index)` | manual reorder |
| `set_job_budget(job_id, budget)` | editable budget |
| `export_boq(job_id, format)` | `format ∈ {"ods","xlsx"}`; returns bytes/path (see §7) |

Loaded into `store.boqItems` during `store.bootstrap()` (like `store.contacts`), with a store slice: `refreshBoqItems`, `createBoqItem`, `updateBoqItem`, `setBoqProcurement`, `deleteBoqItem`, `setJobBudget`, `exportBoq`. Frontend typed wrappers in `ipc.ts`.

## 5. Frontend — the BoQ page

### 5.1 Wiring — new top-level view

BoQ is a co-equal view to the schedule, **not** an `activeTool` drawer.

- Add `activeView: 'schedule' | 'boq'` to `store.svelte.ts` (default `'schedule'`) with a `setView(view)` method. The Gantt canvas renders when `activeView === 'schedule'`; `<BoqPage />` renders when `activeView === 'boq'`, occupying the main content area in `App.svelte` in place of the Gantt (full-width).
- Add a **view switcher** (segmented control: Schedule | Bill of Quantities) in the app header (`AppHeader` / `HeaderActions.svelte`), bound to `store.activeView`.
- `activeTool` (Notes / Contacts / Inbox / Settings drawers) is unchanged and remains available from the header on either view.
- New component tree under `src/lib/boq/`: `BoqPage.svelte`, `BoqGrid.svelte`, `BoqToolbar.svelte`, `FinancialsPanel.svelte`, `ColumnsMenu.svelte`.

### 5.2 Toolbar (top bar)

title · **🔍 search** (matches item/full_spec/supplier/location/invoice_no) · **Status filter** chips (All · Not ordered · Quoted · Ordered · Delivered) · **Columns ▾** (checkbox menu, persists to `app_meta`) · **⤓ Export ▾** (ODS / XLSX) · **+ Add item** · (right) **◧ Financials** toggle.

### 5.3 Grid

Hand-rolled Svelte + CSS — no grid library. Rationale: the app is dependency-free on the JS side and the frozen-pane technique (`position: sticky` + `isolation: isolate`) is already proven in the Gantt canvas (`GanttCanvas.svelte:212`). A library (ag-grid/TanStack) would add weight/licensing for features we can build directly.

- **Frozen header row** (`position: sticky; top: 0`) and **frozen first column** `Item` (`position: sticky; left: 0`); the top-left cell is sticky on both axes.
- **Columns** in sheet order: Item · Qty · Unit · Rate · **Cost** (computed, read-only) · Trade · Full Spec · W · D · H · Ø · Supplier · Location · **Procurement** · Lead wks · Invoice # · Tut Ref No · Organisation. Hidden ones omitted per the visibility set.
- **Sort:** click a column header to sort asc → desc → unsorted. Numeric columns sort numerically (so "sort by price" works). Grid stays **flat** (not grouped) so global sort is unambiguous; sector grouping lives in the financials panel.
- **Inline editing:** click a cell → input; Enter/blur commits via `updateBoqItem`; Tab moves right. Numeric cells use numeric inputs. **Cost** is never editable. **Procurement** renders as a small dropdown/segmented control in-cell.
- **Delivery approval:** advancing Procurement `Ordered → Delivered` from the in-cell control calls `setBoqProcurement`, capturing today's `delivered_date` (mirrors the inbox "Mark Done" feel). *(Optional future: also surface "deliveries to confirm" as inbox cards.)*
- **Add row:** `+ Add item` appends a blank editable row via `createBoqItem`.
- **Delete row:** trailing `×` on row hover → confirm → `deleteBoqItem`.
- Procurement cells carry a colour dot (grey / amber / light-green / dark-green) so status reads at a glance.

### 5.4 Financials panel (collapsible, docked right)

Opened from the toolbar toggle. Everything computed live from `store.boqItems` + `job.budget`:

- **Budget** — click to edit (`setJobBudget`).
- **Spent — left the account** = Σ cost where `procurement ∈ {ordered, delivered}`. Headline number.
- **Budget bar** spanning the full budget, filled left→right: **Delivered** (dark green) · **Ordered** (light green) · **Quoted** (hatched amber, = Σ cost where `procurement = quoted`) · **Free** (grey remainder). If Spent + Quoted exceeds Budget, the overflow segment renders **red** as an over-budget flag.
- **Remaining budget** = Budget − Spent.
- **Projected if all quotes taken** = Spent + Quoted (small subtext line).
- **By sector (Trade):** collapsible groups, each showing committed rands (bold) and open quotes as `(+Xk q)`; expand to line items.

## 6. MCP server

Add a **read** tool now (writes are Phase 2):

- `crates/blikplan-mcp/src/tools/read.rs`: `query_list_boq(job_id)` returning a serialisable summary (item, qty, rate, cost, trade, supplier, procurement, location), following `query_list_tasks` (`read.rs:170`).
- Register `list_boq` in `server.rs` `#[tool_router]`.
- Mirror the new table into the MCP test fixture `FIXTURE_SCHEMA_FOR_TEST` (`crates/blikplan-mcp/src/db.rs:62`).

This lets Claude answer "what's been ordered / delivered / what's outstanding" against the live DB immediately, and is the foundation the Phase 2 writer builds on.

## 7. Export

`export_boq(job_id, format)` regenerates a spreadsheet with **live formulas** (per the standing spreadsheet rule):

- **XLSX** via `rust_xlsxwriter` (pure-Rust, styled, supports formulas). `Cost` cells written as `=Qty*Rate`; a per-Trade totals block via `SUMIFS`. This is the primary, lowest-risk path.
- **ODS** authored as OpenDocument XML directly (a zipped `content.xml`; structure already understood from parsing the existing file), with the same `Cost`/total formulas.

Both produce a single **BoQ** sheet mirroring the current layout. The satellite sheets (Room Comparison, Rates & Assumptions) are **not** regenerated — they remain the user's LibreOffice-side modelling, fed by the exported BoQ sheet. *(If ODS hand-authoring proves fiddly, XLSX ships first and ODS follows immediately; LibreOffice opens XLSX natively either way.)*

## 8. Testing

- **Rust:** repo CRUD round-trips; the `update_boq_item` guard (content edit preserves `procurement`); `set_boq_procurement` sets/clears `delivered_date`; cascade delete with job; budget set/read; export produces a valid file with correct formula strings.
- **MCP:** `list_boq` integration test against the fixture schema.
- **Frontend:** financials math (Spent/Quoted/Remaining/Projected, overflow→red); sort correctness on numeric vs text columns; filter + search; column visibility persistence; frozen-pane render.
- **Manual:** one-time import of the real `Bill_of_Quantities.ods` → visual parity check against LibreOffice; export round-trip opens cleanly in LibreOffice.

## 9. One-time data import

A one-off migration/script reads the current `Bill_of_Quantities.ods` **BoQ** sheet and populates `boq_item` for the Noordhoek job, mapping the existing free-text Status values into the new Procurement lifecycle:

| Old Status | → Procurement |
|---|---|
| Not Started, Awaiting Decision, Ready to order | `not_ordered` |
| In Progress | `ordered` |
| Complete | `delivered` |

Items with a filled `Invoice #`/`Rate` but old status "Complete" → `delivered`; those quoted but unpaid → set to `quoted` on review. Jonty eyeballs and corrects edge cases after import (one-time).

## 10. Phase 2 (out of scope here)

Invoice-ingestion automation: parse invoice/quote PDFs or emails (Ozzy et al.) → propose `boq_item` additions/updates as a new `PatchOp` variant (`gb-patches`) → land in the **Inbox** for approval → on approval, draft the record email to **Deslin** (Tutiphase accounts) for Jonty to approve and send. Repoints the existing `boq-ingest` skill at the app instead of LibreOffice. Separate spec.

## 11. Files touched (summary)

- **DB:** `src-tauri/src/db/migrations.rs` (v10: `boq_item` + `job.budget`); `db/models.rs` (struct + `Procurement` enum).
- **Rust:** `repo/boq.rs` (+`repo/mod.rs`); `commands/boq.rs` (+`commands/mod.rs`); `lib.rs` (register commands); export impl (+`rust_xlsxwriter` dep).
- **Frontend:** `store.svelte.ts` (`activeView` + BoQ slice); `ipc.ts` (wrappers); `HeaderActions.svelte` / header (view switcher); `App.svelte` (render BoQ view in main content area); new `src/lib/boq/` components.
- **MCP:** `tools/read.rs`, `server.rs`, `db.rs` fixture.
