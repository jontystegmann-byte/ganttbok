# BoQ View & Grid — Implementation Plan (Plan 2 of 3)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the Bill of Quantities as its own top-level view — a header view-switch (Schedule ⇄ Bill of Quantities) and a hand-rolled spreadsheet grid (frozen header + Item column, sort, status filter, text search, show/hide columns, inline edit, procurement control with delivery approval, add/delete rows) wired to the Plan 1 backend.

**Architecture:** Follow the existing frontend stack exactly — Svelte 5 runes, a global `store` singleton, thin `ipc.ts` wrappers over `invoke`, CSS custom properties for theming. Grid interaction *logic* (sort comparator, filter predicate, column config) lives in a plain-TS module `boq-grid.ts` with vitest unit tests; the `.svelte` components stay thin and are verified by running the app. Frozen panes use `position: sticky` (the technique already proven in the Gantt canvas).

**Tech Stack:** Svelte 5 (runes), TypeScript, Tauri `invoke`, vitest, CSS custom properties.

**Spec:** `docs/superpowers/specs/2026-07-06-boq-page-design.md` (§3.3, §5). **Depends on:** Plan 1 (backend) — merged/committed on `feat/boq-page`.

**Branch:** `feat/boq-page`.

**Out of scope (Plan 3):** the Financials panel, ODS/XLSX export, one-time `.ods` import. This plan ships the grid with no financials panel yet (the toolbar's Financials toggle is added in Plan 3).

---

## Conventions to match (from the existing code)

- **IPC arg casing:** Tauri converts JS camelCase ↔ Rust snake_case. A Rust command param `job_id: i64` is called as `invoke('cmd', { jobId })`. A Rust command whose single arg is a struct named `args` is called as `invoke('cmd', { args })`. A scalar `id: i64` stays `{ id }`.
- **Serde field names:** `BoqItem` JSON uses the Rust field names verbatim (snake_case): `order_index`, `full_spec`, `w_mm`, `delivered_date`, etc. `procurement` is one of `'not_ordered' | 'quoted' | 'ordered' | 'delivered'`.
- **Store slices** call `ipc.*` then update reactive `$state`, mirroring `refreshContacts`/`createContact` in `store.svelte.ts`.
- **CSS vars available:** `--c-bg --c-panel --c-border --c-text --c-text-muted --c-accent --c-accent-fade`, spacing `--sp-1..--sp-4`, `--font-size-xs/sm/base/xl`.

---

## File structure

| File | Responsibility | Action |
|---|---|---|
| `src/lib/types.ts` | `Procurement`, `BoqItem`, `SetProcurementArgs` types | Modify |
| `src/lib/ipc.ts` | BoQ invoke wrappers | Modify |
| `src/lib/boq/boq-grid.ts` | column config + sort/filter pure logic | Create |
| `src/lib/boq/__tests__/boq-grid.test.ts` | vitest for the above (repo convention: tests live under `__tests__/`) | Create |
| `package.json` | add `"test": "vitest run"` script | Modify |
| `src/lib/store.svelte.ts` | `activeView` + BoQ state/methods | Modify |
| `src/lib/components/AppHeader.svelte` | view switcher control | Modify |
| `src/App.svelte` | render `BoqPage` in main area when view = boq | Modify |
| `src/lib/boq/BoqPage.svelte` | page shell (toolbar + grid) | Create |
| `src/lib/boq/BoqToolbar.svelte` | search, status chips, columns menu, add | Create |
| `src/lib/boq/BoqGrid.svelte` | the grid (frozen panes, sort, inline edit) | Create |

---

## Task 1: Types + IPC wrappers

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/ipc.ts`

- [ ] **Step 1: Add the types**

Append to `src/lib/types.ts`:

```ts
export type Procurement = 'not_ordered' | 'quoted' | 'ordered' | 'delivered';

export interface BoqItem {
  id: number;
  job_id: number;
  order_index: number;
  item: string;
  qty: number | null;
  unit: string | null;
  rate: number | null;
  trade: string | null;
  full_spec: string | null;
  w_mm: number | null;
  d_mm: number | null;
  h_mm: number | null;
  dia_mm: number | null;
  supplier: string | null;
  location: string | null;
  procurement: Procurement;
  delivered_date: string | null;
  lead_weeks: number | null;
  invoice_no: string | null;
  tut_ref_no: string | null;
  organisation: string | null;
  created_at: string;
}

export interface SetProcurementArgs {
  id: number;
  procurement: Procurement;
  delivered_date: string | null;
}
```

- [ ] **Step 2: Add the IPC wrappers**

In `src/lib/ipc.ts`, add `BoqItem, SetProcurementArgs` to the type import block at the top, then append at the end of the file:

```ts
// Bill of Quantities
export const listBoqItems    = (jobId: number)              => invoke<BoqItem[]>('list_boq_items', { jobId });
export const createBoqItem   = (jobId: number)              => invoke<BoqItem>('create_boq_item', { jobId });
export const updateBoqItem   = (args: BoqItem)              => invoke<void>('update_boq_item', { args });
export const setBoqProcurement = (args: SetProcurementArgs) => invoke<void>('set_boq_procurement', { args });
export const reorderBoqItem  = (id: number, orderIndex: number) =>
  invoke<void>('reorder_boq_item', { args: { id, order_index: orderIndex } });
export const deleteBoqItem   = (id: number)                 => invoke<void>('delete_boq_item', { id });
export const setJobBudget    = (jobId: number, budget: number | null) =>
  invoke<void>('set_job_budget', { args: { job_id: jobId, budget } });
export const getJobBudget    = (jobId: number)              => invoke<number | null>('get_job_budget', { jobId });
```

- [ ] **Step 3: Verify it type-checks**

Run: `cd /Users/cncuser/Desktop/GanttBok && npm run check`
Expected: no new type errors introduced by these files (pre-existing warnings elsewhere are acceptable — compare against a clean baseline if unsure).

- [ ] **Step 4: Commit**

```bash
git add src/lib/types.ts src/lib/ipc.ts
git commit -m "feat(boq): frontend types + IPC wrappers"
```

---

## Task 2: Grid logic module (`boq-grid.ts`) + vitest

**Files:**
- Create: `src/lib/boq/boq-grid.ts`
- Create: `src/lib/boq/__tests__/boq-grid.test.ts`  (repo's `vitest.config.ts` only discovers `src/**/__tests__/**/*.test.ts`)
- Modify: `package.json` (add test script)

- [ ] **Step 1: Add a test script**

In `package.json` `scripts`, add:

```json
    "test": "vitest run",
```

- [ ] **Step 2: Write the failing tests**

Create `src/lib/boq/__tests__/boq-grid.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import {
  COLUMNS, DEFAULT_HIDDEN, cost, sortItems, filterItems, type ColumnKey,
} from '../boq-grid';
import type { BoqItem } from '../../types';

function mk(partial: Partial<BoqItem>): BoqItem {
  return {
    id: 1, job_id: 1, order_index: 0, item: '', qty: null, unit: null, rate: null,
    trade: null, full_spec: null, w_mm: null, d_mm: null, h_mm: null, dia_mm: null,
    supplier: null, location: null, procurement: 'not_ordered', delivered_date: null,
    lead_weeks: null, invoice_no: null, tut_ref_no: null, organisation: null,
    created_at: '2026-07-06T00:00:00', ...partial,
  };
}

describe('cost', () => {
  it('is qty*rate when both present, else null', () => {
    expect(cost(mk({ qty: 2, rate: 100 }))).toBe(200);
    expect(cost(mk({ qty: null, rate: 100 }))).toBeNull();
    expect(cost(mk({ qty: 2, rate: null }))).toBeNull();
  });
});

describe('COLUMNS / DEFAULT_HIDDEN', () => {
  it('includes item first and cost as computed', () => {
    expect(COLUMNS[0].key).toBe('item');
    expect(COLUMNS.find(c => c.key === 'cost')?.computed).toBe(true);
  });
  it('default-hides dimension + ref columns', () => {
    for (const k of ['full_spec','w_mm','d_mm','h_mm','dia_mm','invoice_no','tut_ref_no','organisation'] as ColumnKey[]) {
      expect(DEFAULT_HIDDEN).toContain(k);
    }
  });
});

describe('sortItems', () => {
  const items = [
    mk({ id: 1, item: 'Beta',  qty: 1, rate: 300 }), // cost 300
    mk({ id: 2, item: 'Alpha', qty: 2, rate: 50 }),  // cost 100
    mk({ id: 3, item: 'Gamma', qty: null, rate: null }), // cost null
  ];
  it('sorts numeric cost ascending with nulls last', () => {
    const out = sortItems(items, 'cost', 'asc').map(i => i.id);
    expect(out).toEqual([2, 1, 3]);
  });
  it('sorts numeric cost descending with nulls last', () => {
    const out = sortItems(items, 'cost', 'desc').map(i => i.id);
    expect(out).toEqual([1, 2, 3]);
  });
  it('sorts text case-insensitively', () => {
    const out = sortItems(items, 'item', 'asc').map(i => i.item);
    expect(out).toEqual(['Alpha', 'Beta', 'Gamma']);
  });
  it('returns original order when column is null', () => {
    const out = sortItems(items, null, 'asc').map(i => i.id);
    expect(out).toEqual([1, 2, 3]);
  });
});

describe('filterItems', () => {
  const items = [
    mk({ id: 1, item: 'Heat pump', supplier: 'Hydrofire', procurement: 'ordered' }),
    mk({ id: 2, item: 'Skylight', supplier: 'OZ', procurement: 'quoted' }),
    mk({ id: 3, item: 'Gate', full_spec: 'timber hobbit', procurement: 'not_ordered' }),
  ];
  it('filters by procurement status', () => {
    expect(filterItems(items, 'quoted', '').map(i => i.id)).toEqual([2]);
    expect(filterItems(items, 'all', '').length).toBe(3);
  });
  it('filters by case-insensitive search across item/supplier/full_spec/location/invoice', () => {
    expect(filterItems(items, 'all', 'hydro').map(i => i.id)).toEqual([1]);
    expect(filterItems(items, 'all', 'HOBBIT').map(i => i.id)).toEqual([3]);
  });
  it('combines status + search (AND)', () => {
    expect(filterItems(items, 'ordered', 'heat').map(i => i.id)).toEqual([1]);
    expect(filterItems(items, 'quoted', 'heat').length).toBe(0);
  });
});
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd /Users/cncuser/Desktop/GanttBok && npx vitest run boq-grid`
Expected: FAIL — `Cannot find module '../boq-grid'`.

- [ ] **Step 4: Implement the module**

Create `src/lib/boq/boq-grid.ts`:

```ts
import type { BoqItem, Procurement } from '../types';

export type ColumnKey =
  | 'item' | 'qty' | 'unit' | 'rate' | 'cost' | 'trade' | 'full_spec'
  | 'w_mm' | 'd_mm' | 'h_mm' | 'dia_mm' | 'supplier' | 'location'
  | 'procurement' | 'lead_weeks' | 'invoice_no' | 'tut_ref_no' | 'organisation';

export interface ColumnDef {
  key: ColumnKey;
  label: string;
  numeric: boolean;
  computed?: boolean; // cost is derived, never edited
}

export const COLUMNS: ColumnDef[] = [
  { key: 'item',        label: 'Item',        numeric: false },
  { key: 'qty',         label: 'Qty',         numeric: true  },
  { key: 'unit',        label: 'Unit',        numeric: false },
  { key: 'rate',        label: 'Rate',        numeric: true  },
  { key: 'cost',        label: 'Cost',        numeric: true, computed: true },
  { key: 'trade',       label: 'Trade',       numeric: false },
  { key: 'full_spec',   label: 'Full Spec',   numeric: false },
  { key: 'w_mm',        label: 'W (mm)',      numeric: true  },
  { key: 'd_mm',        label: 'D (mm)',      numeric: true  },
  { key: 'h_mm',        label: 'H (mm)',      numeric: true  },
  { key: 'dia_mm',      label: 'Ø (mm)',      numeric: true  },
  { key: 'supplier',    label: 'Supplier',    numeric: false },
  { key: 'location',    label: 'Location',    numeric: false },
  { key: 'procurement', label: 'Procurement', numeric: false },
  { key: 'lead_weeks',  label: 'Lead (wks)',  numeric: true  },
  { key: 'invoice_no',  label: 'Invoice #',   numeric: false },
  { key: 'tut_ref_no',  label: 'Tut Ref No',  numeric: false },
  { key: 'organisation',label: 'Organisation',numeric: false },
];

export const DEFAULT_HIDDEN: ColumnKey[] = [
  'full_spec', 'w_mm', 'd_mm', 'h_mm', 'dia_mm', 'invoice_no', 'tut_ref_no', 'organisation',
];

export const PROCUREMENT_LABELS: Record<Procurement, string> = {
  not_ordered: 'Not ordered',
  quoted: 'Quoted',
  ordered: 'Ordered',
  delivered: 'Delivered',
};

export type StatusFilter = 'all' | Procurement;
export type SortDir = 'asc' | 'desc';

export function cost(it: BoqItem): number | null {
  return it.qty != null && it.rate != null ? it.qty * it.rate : null;
}

/** Value used for sorting a given column. */
function sortValue(it: BoqItem, key: ColumnKey): number | string | null {
  if (key === 'cost') return cost(it);
  const v = (it as unknown as Record<string, unknown>)[key];
  return (v as number | string | null) ?? null;
}

/** Stable sort. Nulls always sort last regardless of direction. */
export function sortItems(items: BoqItem[], key: ColumnKey | null, dir: SortDir): BoqItem[] {
  if (!key) return items;
  const col = COLUMNS.find(c => c.key === key);
  const numeric = col?.numeric ?? false;
  const factor = dir === 'asc' ? 1 : -1;
  return items
    .map((it, i) => [it, i] as const)
    .sort(([a, ai], [b, bi]) => {
      const av = sortValue(a, key);
      const bv = sortValue(b, key);
      if (av == null && bv == null) return ai - bi;
      if (av == null) return 1;   // nulls last
      if (bv == null) return -1;  // nulls last
      let cmp: number;
      if (numeric) cmp = (av as number) - (bv as number);
      else cmp = String(av).localeCompare(String(bv), undefined, { sensitivity: 'base' });
      return cmp !== 0 ? cmp * factor : ai - bi;
    })
    .map(([it]) => it);
}

const SEARCH_FIELDS: (keyof BoqItem)[] = ['item', 'full_spec', 'supplier', 'location', 'invoice_no'];

export function filterItems(items: BoqItem[], status: StatusFilter, search: string): BoqItem[] {
  const q = search.trim().toLowerCase();
  return items.filter(it => {
    if (status !== 'all' && it.procurement !== status) return false;
    if (!q) return true;
    return SEARCH_FIELDS.some(f => {
      const v = it[f];
      return typeof v === 'string' && v.toLowerCase().includes(q);
    });
  });
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd /Users/cncuser/Desktop/GanttBok && npx vitest run boq-grid`
Expected: PASS — all cost/columns/sort/filter tests green.

- [ ] **Step 6: Commit**

```bash
git add src/lib/boq/boq-grid.ts src/lib/boq/__tests__/boq-grid.test.ts package.json
git commit -m "feat(boq): grid logic (columns/sort/filter/cost) + vitest"
```

---

## Task 3: Store slice — `activeView` + BoQ state

**Files:**
- Modify: `src/lib/store.svelte.ts`

- [ ] **Step 1: Add imports**

In `src/lib/store.svelte.ts`, add `BoqItem, Procurement` to the existing `import type { ... } from './types'` line.

- [ ] **Step 2: Add the view + state fields**

After the `activeTool` block (around line 69), add:

```ts
  // Top-level view: the schedule (Gantt) or the Bill of Quantities. Co-equal pages.
  activeView = $state<'schedule' | 'boq'>('schedule');
  setView(view: 'schedule' | 'boq'): void { this.activeView = view; }

  // Bill of Quantities line items for the open job.
  boqItems = $state<BoqItem[]>([]);
```

- [ ] **Step 3: Add the store methods**

Add these methods to the `Store` class (near `refreshContacts`):

```ts
  async refreshBoqItems(): Promise<void> {
    if (!this.currentJob) { this.boqItems = []; return; }
    this.boqItems = await ipc.listBoqItems(this.currentJob.id);
  }

  async createBoqItem(): Promise<void> {
    if (!this.currentJob) return;
    const created = await ipc.createBoqItem(this.currentJob.id);
    this.boqItems = [...this.boqItems, created];
    await ipc.touchLastSave();
  }

  async updateBoqItem(item: BoqItem): Promise<void> {
    await ipc.updateBoqItem($state.snapshot(item));
    this.boqItems = this.boqItems.map(b => b.id === item.id ? { ...item } : b);
    await ipc.touchLastSave();
  }

  async setBoqProcurement(id: number, procurement: Procurement, deliveredDate: string | null): Promise<void> {
    await ipc.setBoqProcurement({ id, procurement, delivered_date: deliveredDate });
    // Backend owns delivered_date: it stores it only when procurement === 'delivered'.
    const resolved = procurement === 'delivered'
      ? (deliveredDate ?? this.todayIso)
      : null;
    this.boqItems = this.boqItems.map(b =>
      b.id === id ? { ...b, procurement, delivered_date: resolved } : b);
    await ipc.touchLastSave();
  }

  async deleteBoqItem(id: number): Promise<void> {
    await ipc.deleteBoqItem(id);
    this.boqItems = this.boqItems.filter(b => b.id !== id);
    await ipc.touchLastSave();
  }
```

- [ ] **Step 4: Load BoQ items when a job opens**

In `openJob(jobId)`, after the `this.noWorkDays = await ipc.listNoWorkDays(jobId);` line, add:

```ts
    this.boqItems = await ipc.listBoqItems(jobId);
```

Also, in `openJob`, the view should reset to the schedule when switching jobs — add near the end of `openJob` (after `this.selection = null;`):

```ts
    this.activeView = 'schedule';
```

- [ ] **Step 5: Verify type-check**

Run: `cd /Users/cncuser/Desktop/GanttBok && npm run check`
Expected: no new errors.

- [ ] **Step 6: Commit**

```bash
git add src/lib/store.svelte.ts
git commit -m "feat(boq): store slice — activeView + boqItems state/methods"
```

---

## Task 4: View switcher + render BoqPage

**Files:**
- Modify: `src/lib/components/AppHeader.svelte`
- Create: `src/lib/boq/BoqPage.svelte` (minimal shell for now; grid added in Task 5)
- Modify: `src/App.svelte`

- [ ] **Step 1: Create a minimal BoqPage shell**

Create `src/lib/boq/BoqPage.svelte`:

```svelte
<script lang="ts">
  import { store } from '../store.svelte';
</script>

<div class="boq-page">
  {#if store.currentJob}
    <p class="placeholder">Bill of Quantities — {store.boqItems.length} line items (grid arrives in Task 5)</p>
  {:else}
    <p class="placeholder">Pick a job to see its Bill of Quantities.</p>
  {/if}
</div>

<style>
  .boq-page { height: 100%; overflow: auto; background: var(--c-bg); padding: var(--sp-4); }
  .placeholder { color: var(--c-text-muted); }
</style>
```

- [ ] **Step 2: Add the view switcher to the header**

In `src/lib/components/AppHeader.svelte`, inside `<div class="tools">`, BEFORE `<HeaderActions />`, add the segmented switch:

```svelte
    <div class="view-switch" role="tablist" aria-label="View">
      <button
        class="seg"
        class:on={store.activeView === 'schedule'}
        role="tab"
        aria-selected={store.activeView === 'schedule'}
        onclick={() => store.setView('schedule')}
      >Schedule</button>
      <button
        class="seg"
        class:on={store.activeView === 'boq'}
        role="tab"
        aria-selected={store.activeView === 'boq'}
        onclick={() => store.setView('boq')}
      >Bill of Quantities</button>
    </div>
```

Then add to the `<style>` block:

```css
  .view-switch {
    display: inline-flex;
    background: var(--c-accent-fade);
    border-radius: 6px;
    padding: 2px;
    gap: 2px;
    flex-shrink: 0;
  }
  .seg {
    border: 0;
    background: transparent;
    color: var(--c-text-muted);
    font: inherit;
    font-size: var(--font-size-sm);
    font-weight: 600;
    padding: var(--sp-1) var(--sp-3);
    border-radius: 4px;
    cursor: pointer;
  }
  .seg.on { background: var(--c-accent); color: #fff; }
```

- [ ] **Step 3: Render BoqPage in the main content area**

In `src/App.svelte`:

(a) add the import after the `GanttCanvas` import:

```ts
  import BoqPage from './lib/boq/BoqPage.svelte';
```

(b) replace the `<main class="canvas-pane"> ... </main>` block with a view-switched version:

```svelte
    <main class="canvas-pane">
      {#if store.activeView === 'boq'}
        <BoqPage />
      {:else if store.currentJob}
        <GanttCanvas />
      {:else}
        <div class="empty-state">
          <h1><span style="font-weight: 900">BLIK</span> <span style="font-weight: 300; color: var(--c-text-muted)">Plan</span></h1>
          <p>Pick a job to get started.</p>
          <JobSwitcher />
        </div>
      {/if}
    </main>
```

- [ ] **Step 4: Verify by running the app**

Run: `cd /Users/cncuser/Desktop/GanttBok && npm run tauri dev` (or the project's `run` skill).
Verify: the header shows a **Schedule | Bill of Quantities** switch; clicking "Bill of Quantities" replaces the Gantt with the BoQ placeholder showing the line-item count; clicking "Schedule" returns to the Gantt. Switching jobs resets to Schedule.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/AppHeader.svelte src/lib/boq/BoqPage.svelte src/App.svelte
git commit -m "feat(boq): top-level view switch + BoqPage shell"
```

---

## Task 5: The grid — toolbar, frozen panes, sort, filter, inline edit, procurement

**Files:**
- Create: `src/lib/boq/BoqToolbar.svelte`
- Create: `src/lib/boq/BoqGrid.svelte`
- Modify: `src/lib/boq/BoqPage.svelte` (compose toolbar + grid)

This task is verified by running the app (no unit tests — the pure logic it uses is already covered by Task 2's vitest). Build it, then drive it.

- [ ] **Step 1: Create the toolbar**

Create `src/lib/boq/BoqToolbar.svelte`:

```svelte
<script lang="ts">
  import { COLUMNS, PROCUREMENT_LABELS, type ColumnKey, type StatusFilter } from './boq-grid';
  import type { Procurement } from '../types';
  import { store } from '../store.svelte';

  let {
    status = $bindable(),
    search = $bindable(),
    hidden = $bindable(),
  }: {
    status: StatusFilter;
    search: string;
    hidden: Set<ColumnKey>;
  } = $props();

  let showColumns = $state(false);

  const STATUSES: StatusFilter[] = ['all', 'not_ordered', 'quoted', 'ordered', 'delivered'];
  function statusLabel(s: StatusFilter): string {
    return s === 'all' ? 'All' : PROCUREMENT_LABELS[s as Procurement];
  }
  function toggleColumn(key: ColumnKey): void {
    const next = new Set(hidden);
    if (next.has(key)) next.delete(key); else next.add(key);
    hidden = next;
  }
</script>

<div class="toolbar">
  <input class="search" type="search" placeholder="Search items, suppliers, specs…" bind:value={search} />

  <div class="chips">
    {#each STATUSES as s}
      <button class="chip" class:on={status === s} onclick={() => (status = s)}>{statusLabel(s)}</button>
    {/each}
  </div>

  <div class="cols">
    <button class="btn" onclick={() => (showColumns = !showColumns)}>Columns ▾</button>
    {#if showColumns}
      <div class="menu">
        {#each COLUMNS as c}
          <label class="menu-row">
            <input
              type="checkbox"
              checked={!hidden.has(c.key)}
              disabled={c.key === 'item'}
              onchange={() => toggleColumn(c.key)}
            />
            {c.label}
          </label>
        {/each}
      </div>
    {/if}
  </div>

  <button class="btn primary" onclick={() => store.createBoqItem()}>+ Add item</button>
</div>

<style>
  .toolbar { display: flex; align-items: center; gap: var(--sp-2); padding: var(--sp-2) var(--sp-3);
    border-bottom: 1px solid var(--c-border); background: var(--c-panel); flex-wrap: wrap; }
  .search { flex: 0 0 240px; padding: var(--sp-1) var(--sp-2); border: 1px solid var(--c-border);
    border-radius: 5px; background: var(--c-bg); color: var(--c-text); font: inherit; font-size: var(--font-size-sm); }
  .chips { display: flex; gap: 4px; }
  .chip { border: 1px solid var(--c-border); background: transparent; color: var(--c-text-muted);
    border-radius: 12px; padding: 2px 10px; font-size: var(--font-size-xs); cursor: pointer; }
  .chip.on { background: var(--c-accent); color: #fff; border-color: var(--c-accent); }
  .cols { position: relative; }
  .btn { border: 1px solid var(--c-border); background: transparent; color: var(--c-text);
    border-radius: 5px; padding: var(--sp-1) var(--sp-3); font: inherit; font-size: var(--font-size-sm); cursor: pointer; }
  .btn.primary { background: var(--c-accent); color: #fff; border-color: var(--c-accent); margin-left: auto; }
  .menu { position: absolute; top: 110%; left: 0; z-index: 30; background: var(--c-panel);
    border: 1px solid var(--c-border); border-radius: 6px; padding: var(--sp-2); min-width: 180px;
    box-shadow: 0 6px 20px rgba(0,0,0,0.18); max-height: 320px; overflow: auto; }
  .menu-row { display: flex; align-items: center; gap: var(--sp-2); padding: 3px 2px;
    font-size: var(--font-size-sm); color: var(--c-text); white-space: nowrap; }
</style>
```

- [ ] **Step 2: Create the grid**

Create `src/lib/boq/BoqGrid.svelte`:

```svelte
<script lang="ts">
  import { store } from '../store.svelte';
  import type { BoqItem, Procurement } from '../types';
  import {
    COLUMNS, PROCUREMENT_LABELS, cost, sortItems, filterItems,
    type ColumnKey, type StatusFilter, type SortDir,
  } from './boq-grid';

  let {
    status, search, hidden,
  }: { status: StatusFilter; search: string; hidden: Set<ColumnKey> } = $props();

  let sortKey = $state<ColumnKey | null>(null);
  let sortDir = $state<SortDir>('asc');

  const visibleColumns = $derived(COLUMNS.filter(c => !hidden.has(c.key)));
  const rows = $derived(sortItems(filterItems(store.boqItems, status, search), sortKey, sortDir));

  const PROCUREMENTS: Procurement[] = ['not_ordered', 'quoted', 'ordered', 'delivered'];

  function toggleSort(key: ColumnKey): void {
    if (sortKey !== key) { sortKey = key; sortDir = 'asc'; }
    else if (sortDir === 'asc') { sortDir = 'desc'; }
    else { sortKey = null; sortDir = 'asc'; }
  }

  function display(it: BoqItem, key: ColumnKey): string {
    if (key === 'cost') { const c = cost(it); return c == null ? '' : c.toLocaleString('en-ZA'); }
    if (key === 'procurement') return PROCUREMENT_LABELS[it.procurement];
    const v = (it as unknown as Record<string, unknown>)[key];
    return v == null ? '' : String(v);
  }

  // Inline editing --------------------------------------------------------
  let editing = $state<{ id: number; key: ColumnKey } | null>(null);
  let draft = $state('');

  function startEdit(it: BoqItem, key: ColumnKey): void {
    const col = COLUMNS.find(c => c.key === key)!;
    if (col.computed || key === 'procurement') return; // cost read-only; procurement uses dropdown
    editing = { id: it.id, key };
    draft = display(it, key);
  }

  async function commitEdit(it: BoqItem): Promise<void> {
    if (!editing) return;
    const key = editing.key;
    const col = COLUMNS.find(c => c.key === key)!;
    const next: BoqItem = { ...it };
    if (col.numeric) {
      const n = draft.trim() === '' ? null : Number(draft.replace(/[, ]/g, ''));
      (next as unknown as Record<string, unknown>)[key] = Number.isFinite(n as number) ? n : null;
    } else {
      (next as unknown as Record<string, unknown>)[key] = draft.trim() === '' ? null : draft;
    }
    if (key === 'item') next.item = draft; // item is NOT NULL; keep empty string not null
    editing = null;
    await store.updateBoqItem(next);
  }

  async function changeProcurement(it: BoqItem, value: Procurement): Promise<void> {
    const deliveredDate = value === 'delivered' ? store.todayIso : null;
    await store.setBoqProcurement(it.id, value, deliveredDate);
  }

  async function remove(it: BoqItem): Promise<void> {
    if (confirm(`Delete "${it.item || 'this line'}"?`)) await store.deleteBoqItem(it.id);
  }
</script>

<div class="grid-wrap">
  <table class="boq">
    <thead>
      <tr>
        {#each visibleColumns as c (c.key)}
          <th
            class:frozen-col={c.key === 'item'}
            class:num={c.numeric}
            onclick={() => toggleSort(c.key)}
            title="Sort by {c.label}"
          >
            {c.label}{#if sortKey === c.key}<span class="arrow">{sortDir === 'asc' ? ' ▲' : ' ▼'}</span>{/if}
          </th>
        {/each}
        <th class="row-actions"></th>
      </tr>
    </thead>
    <tbody>
      {#each rows as it (it.id)}
        <tr>
          {#each visibleColumns as c (c.key)}
            <td
              class:frozen-col={c.key === 'item'}
              class:num={c.numeric}
              ondblclick={() => startEdit(it, c.key)}
            >
              {#if editing && editing.id === it.id && editing.key === c.key}
                <input
                  class="cell-input"
                  bind:value={draft}
                  onblur={() => commitEdit(it)}
                  onkeydown={(e) => { if (e.key === 'Enter') commitEdit(it); if (e.key === 'Escape') editing = null; }}
                  autofocus
                />
              {:else if c.key === 'procurement'}
                <select
                  class="proc proc-{it.procurement}"
                  value={it.procurement}
                  onchange={(e) => changeProcurement(it, (e.currentTarget as HTMLSelectElement).value as Procurement)}
                >
                  {#each PROCUREMENTS as p}
                    <option value={p}>{PROCUREMENT_LABELS[p]}</option>
                  {/each}
                </select>
              {:else}
                {display(it, c.key)}
              {/if}
            </td>
          {/each}
          <td class="row-actions"><button class="del" title="Delete row" onclick={() => remove(it)}>×</button></td>
        </tr>
      {/each}
      {#if rows.length === 0}
        <tr><td class="empty" colspan={visibleColumns.length + 1}>No line items match. Add one with “+ Add item”.</td></tr>
      {/if}
    </tbody>
  </table>
</div>

<style>
  .grid-wrap { overflow: auto; height: 100%; }
  table.boq { border-collapse: separate; border-spacing: 0; font-size: var(--font-size-sm); }
  th, td { border-right: 1px solid var(--c-border); border-bottom: 1px solid var(--c-border);
    padding: 4px 8px; white-space: nowrap; text-align: left; background: var(--c-bg); }
  th { position: sticky; top: 0; z-index: 2; background: var(--c-panel); cursor: pointer;
    font-weight: 600; user-select: none; }
  td.num, th.num { text-align: right; font-variant-numeric: tabular-nums; }
  /* Frozen first column (Item) — sticky on the left; header cell sticky on both axes. */
  .frozen-col { position: sticky; left: 0; z-index: 1; background: var(--c-bg); font-weight: 600; }
  th.frozen-col { z-index: 3; background: var(--c-panel); }
  .arrow { color: var(--c-accent); }
  .cell-input { width: 100%; box-sizing: border-box; border: 1px solid var(--c-accent);
    border-radius: 3px; padding: 1px 4px; font: inherit; background: var(--c-bg); color: var(--c-text); }
  .proc { font: inherit; font-size: var(--font-size-xs); border: 1px solid var(--c-border);
    border-radius: 10px; padding: 1px 6px; background: var(--c-bg); color: var(--c-text); cursor: pointer; }
  .proc-not_ordered { color: var(--c-text-muted); }
  .proc-quoted   { color: #c9962f; border-color: #c9962f; }
  .proc-ordered  { color: #57b083; border-color: #57b083; }
  .proc-delivered{ color: #2f7d54; border-color: #2f7d54; font-weight: 600; }
  .row-actions { border-right: 0; text-align: center; }
  .del { border: 0; background: transparent; color: var(--c-text-muted); cursor: pointer; font-size: 15px; visibility: hidden; }
  tr:hover .del { visibility: visible; }
  .del:hover { color: var(--c-accent); }
  .empty { text-align: center; color: var(--c-text-muted); padding: var(--sp-4); }
</style>
```

- [ ] **Step 3: Compose toolbar + grid in BoqPage**

Replace `src/lib/boq/BoqPage.svelte` entirely with:

```svelte
<script lang="ts">
  import { store } from '../store.svelte';
  import BoqToolbar from './BoqToolbar.svelte';
  import BoqGrid from './BoqGrid.svelte';
  import { DEFAULT_HIDDEN, type ColumnKey, type StatusFilter } from './boq-grid';

  let status = $state<StatusFilter>('all');
  let search = $state('');
  let hidden = $state<Set<ColumnKey>>(new Set(DEFAULT_HIDDEN));
</script>

<div class="boq-page">
  {#if store.currentJob}
    <BoqToolbar bind:status bind:search bind:hidden />
    <BoqGrid {status} {search} {hidden} />
  {:else}
    <p class="placeholder">Pick a job to see its Bill of Quantities.</p>
  {/if}
</div>

<style>
  .boq-page { display: flex; flex-direction: column; height: 100%; overflow: hidden; background: var(--c-bg); }
  .placeholder { color: var(--c-text-muted); padding: var(--sp-4); }
</style>
```

- [ ] **Step 4: Verify by running the app**

Run: `cd /Users/cncuser/Desktop/GanttBok && npm run tauri dev`
Verify, on a job with BoQ rows (create a few with "+ Add item"):
1. **Frozen panes:** scroll right — the Item column stays pinned; scroll down — the header stays pinned.
2. **Add:** "+ Add item" appends an editable blank row.
3. **Inline edit:** double-click a cell (e.g. Rate) → type `49444.25` → Enter → Cost shows `49 444.25`; Escape cancels.
4. **Procurement:** the Procurement cell is a coloured dropdown; changing it to Delivered persists (reload the app — it sticks); the colour changes per status.
5. **Sort:** click the Rate/Cost header → rows sort numeric asc, click again desc, third click clears; nulls sit last.
6. **Filter:** status chips narrow rows; search matches item/supplier/spec; combining chip + search ANDs.
7. **Columns:** the Columns ▾ menu hides/shows columns; Item can't be hidden; the dimension columns start hidden.
8. **Delete:** hovering a row shows ×; deleting removes it after confirm.
9. **View persistence:** the procurement/edit changes survive a switch to Schedule and back, and an app restart.

- [ ] **Step 5: Commit**

```bash
git add src/lib/boq/BoqToolbar.svelte src/lib/boq/BoqGrid.svelte src/lib/boq/BoqPage.svelte
git commit -m "feat(boq): grid — frozen panes, sort, filter, columns, inline edit, procurement"
```

---

## Task 6: Full verification

- [ ] **Step 1: Type-check + unit tests**

Run: `cd /Users/cncuser/Desktop/GanttBok && npm run check && npm run test`
Expected: type-check clean (no new errors); vitest green (boq-grid tests pass).

- [ ] **Step 2: Rust still green (nothing backend changed, sanity only)**

Run: `cd /Users/cncuser/Desktop/GanttBok/src-tauri && cargo test --lib`
Expected: all green (unchanged from Plan 1).

- [ ] **Step 3: Manual smoke of the whole flow** (per Task 5 Step 4 checklist) — confirm nothing regressed in the Schedule view.

---

## Done criteria

- Header has a Schedule ⇄ Bill of Quantities switch; BoQ owns the content area when active; switching jobs resets to Schedule.
- Grid renders all visible columns with a frozen header row and frozen Item column.
- Sort (numeric + text, nulls last, tri-state), status-chip filter, text search, and column show/hide all work.
- Inline edit persists via `update_boq_item` (never altering procurement); Cost is read-only and computed.
- Procurement dropdown persists via `set_boq_procurement`; Delivered stamps today's date; colours reflect status.
- Add/delete rows work.
- `npm run check`, `npm run test`, and `cargo test --lib` all green.

**Next:** Plan 3 — Financials panel (budget bar + sector rollups), ODS/XLSX export with live formulas, one-time `.ods` import.
