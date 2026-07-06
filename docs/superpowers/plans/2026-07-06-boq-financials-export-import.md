# BoQ Financials, Export & Import — Implementation Plan (Plan 3 of 3)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the Bill of Quantities feature — a live financials panel (budget bar + sector rollups), spreadsheet export (XLSX with live formulas; ODS via LibreOffice conversion), and a one-time import of the existing `.ods`.

**Architecture:** Financials math lives in a pure-TS module `boq-financials.ts` with vitest; a docked, collapsible `FinancialsPanel.svelte` renders it, toggled from the toolbar. Export is a Rust command that builds an `.xlsx` with `rust_xlsxwriter` (live `=Qty*Rate` + grand-total `=SUM` formulas), writes it to Downloads, and reveals it via the opener plugin; ODS is produced by converting that XLSX with headless LibreOffice. Import is a one-off Python script that reads the current `.ods` and inserts `boq_item` rows.

**Tech Stack:** Svelte 5 (runes), TypeScript, vitest, Rust (`rust_xlsxwriter`, `rusqlite`, `dirs`), `tauri-plugin-opener`, Python 3 (import script).

**Spec:** `docs/superpowers/specs/2026-07-06-boq-page-design.md` (§5.4, §7, §9). **Depends on:** Plan 1 (backend, incl. `get_job_budget`/`set_job_budget` + `export_boq` stub slot) and Plan 2 (grid + `boq-grid.ts` `cost()`), both on `feat/boq-page`.

**Branch:** `feat/boq-page`.

---

## Conventions (recap from Plans 1–2)

- vitest tests live under `src/**/__tests__/` (repo `vitest.config.ts`).
- IPC: Rust `job_id` param → JS `{ jobId }`; struct arg named `args` → `{ args }`; scalar `id` → `{ id }`.
- CSS vars: `--c-bg --c-panel --c-border --c-text --c-text-muted --c-accent --c-accent-fade`, `--sp-1..4`, `--font-size-xs/sm/base/xl`.
- Money is `number | null`; `cost(item)` from `src/lib/boq/boq-grid.ts` = `qty*rate` or null.
- `store.boqItems` holds the open job's line items; `ipc.getJobBudget/setJobBudget` already exist (Plan 1).

---

## File structure

| File | Responsibility | Action |
|---|---|---|
| `src/lib/boq/boq-financials.ts` | financial totals + sector rollups (pure) | Create |
| `src/lib/boq/__tests__/boq-financials.test.ts` | vitest | Create |
| `src/lib/store.svelte.ts` | `boqBudget` state + load/set methods | Modify |
| `src/lib/boq/FinancialsPanel.svelte` | docked panel (budget bar + rollups) | Create |
| `src/lib/boq/BoqToolbar.svelte` | add Financials toggle + Export menu | Modify |
| `src/lib/boq/BoqPage.svelte` | grid + panel side-by-side layout | Modify |
| `src/lib/ipc.ts` | `exportBoq` wrapper | Modify |
| `src-tauri/Cargo.toml` | add `rust_xlsxwriter` | Modify |
| `src-tauri/src/commands/boq_export.rs` | build xlsx + `export_boq` command | Create |
| `src-tauri/src/commands/mod.rs` | declare `pub mod boq_export;` | Modify |
| `src-tauri/src/lib.rs` | register `export_boq` | Modify |
| `scripts/import_boq_from_ods.py` | one-time importer | Create |

---

## Task 1: Financials logic module + vitest

**Files:**
- Create: `src/lib/boq/boq-financials.ts`
- Create: `src/lib/boq/__tests__/boq-financials.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `src/lib/boq/__tests__/boq-financials.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { financials, sectorRollups } from '../boq-financials';
import type { BoqItem } from '../../types';

function mk(p: Partial<BoqItem>): BoqItem {
  return {
    id: 1, job_id: 1, order_index: 0, item: '', qty: null, unit: null, rate: null,
    trade: null, full_spec: null, w_mm: null, d_mm: null, h_mm: null, dia_mm: null,
    supplier: null, location: null, procurement: 'not_ordered', delivered_date: null,
    lead_weeks: null, invoice_no: null, tut_ref_no: null, organisation: null,
    created_at: '2026-07-06', ...p,
  };
}

const items: BoqItem[] = [
  mk({ id: 1, qty: 1, rate: 510000, trade: 'HVAC', procurement: 'delivered' }),
  mk({ id: 2, qty: 1, rate: 130000, trade: 'HVAC', procurement: 'ordered' }),
  mk({ id: 3, qty: 1, rate: 240000, trade: 'GLAZING', procurement: 'quoted' }),
  mk({ id: 4, qty: 1, rate: 99999, trade: 'CARPENTER', procurement: 'not_ordered' }),
];

describe('financials', () => {
  it('spent = ordered + delivered; quoted separate; not_ordered excluded', () => {
    const f = financials(items, 2_000_000);
    expect(f.delivered).toBe(510000);
    expect(f.ordered).toBe(130000);
    expect(f.spent).toBe(640000);
    expect(f.quoted).toBe(240000);
    expect(f.remaining).toBe(1_360_000);
    expect(f.projected).toBe(880000);
    expect(f.overBudget).toBe(false);
  });

  it('remaining is null when no budget set', () => {
    const f = financials(items, null);
    expect(f.remaining).toBeNull();
    expect(f.spent).toBe(640000);
    expect(f.overBudget).toBe(false);
  });

  it('flags over budget when spent + quoted exceeds budget', () => {
    const f = financials(items, 700_000);
    expect(f.overBudget).toBe(true); // 640k + 240k = 880k > 700k
  });
});

describe('sectorRollups', () => {
  it('groups committed (ordered+delivered) and quoted by trade, sorted by committed desc', () => {
    const rollups = sectorRollups(items);
    expect(rollups[0]).toEqual({ trade: 'HVAC', committed: 640000, quoted: 0 });
    const glazing = rollups.find(r => r.trade === 'GLAZING');
    expect(glazing).toEqual({ trade: 'GLAZING', committed: 0, quoted: 240000 });
    const carpenter = rollups.find(r => r.trade === 'CARPENTER');
    expect(carpenter).toEqual({ trade: 'CARPENTER', committed: 0, quoted: 0 });
  });

  it('buckets null trade as "Untraded"', () => {
    const rollups = sectorRollups([mk({ qty: 1, rate: 100, trade: null, procurement: 'ordered' })]);
    expect(rollups[0].trade).toBe('Untraded');
    expect(rollups[0].committed).toBe(100);
  });
});
```

- [ ] **Step 2: Run to verify fail**

Run: `cd /Users/cncuser/Desktop/GanttBok && npx vitest run boq-financials`
Expected: FAIL — `Cannot find module '../boq-financials'`.

- [ ] **Step 3: Implement the module**

Create `src/lib/boq/boq-financials.ts`:

```ts
import type { BoqItem } from '../types';
import { cost } from './boq-grid';

export interface Financials {
  delivered: number;   // Σ cost where procurement = delivered
  ordered: number;     // Σ cost where procurement = ordered
  spent: number;       // delivered + ordered (money that left the account)
  quoted: number;      // Σ cost where procurement = quoted (provisional)
  remaining: number | null; // budget - spent, or null when no budget
  projected: number;   // spent + quoted
  overBudget: boolean; // budget set AND spent + quoted > budget
}

export interface SectorRollup {
  trade: string;
  committed: number; // ordered + delivered within the trade
  quoted: number;
}

function sumWhere(items: BoqItem[], pred: (it: BoqItem) => boolean): number {
  return items.reduce((acc, it) => pred(it) ? acc + (cost(it) ?? 0) : acc, 0);
}

export function financials(items: BoqItem[], budget: number | null): Financials {
  const delivered = sumWhere(items, it => it.procurement === 'delivered');
  const ordered = sumWhere(items, it => it.procurement === 'ordered');
  const spent = delivered + ordered;
  const quoted = sumWhere(items, it => it.procurement === 'quoted');
  const projected = spent + quoted;
  return {
    delivered, ordered, spent, quoted, projected,
    remaining: budget == null ? null : budget - spent,
    overBudget: budget != null && projected > budget,
  };
}

export function sectorRollups(items: BoqItem[]): SectorRollup[] {
  const map = new Map<string, SectorRollup>();
  for (const it of items) {
    const trade = it.trade ?? 'Untraded';
    const r = map.get(trade) ?? { trade, committed: 0, quoted: 0 };
    const c = cost(it) ?? 0;
    if (it.procurement === 'ordered' || it.procurement === 'delivered') r.committed += c;
    else if (it.procurement === 'quoted') r.quoted += c;
    map.set(trade, r);
  }
  return [...map.values()].sort((a, b) => b.committed - a.committed || b.quoted - a.quoted);
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cd /Users/cncuser/Desktop/GanttBok && npx vitest run boq-financials`
Expected: PASS — all financials + sectorRollups tests green.

- [ ] **Step 5: Commit**

```bash
git add src/lib/boq/boq-financials.ts src/lib/boq/__tests__/boq-financials.test.ts
git commit -m "feat(boq): financials logic (totals + sector rollups) + vitest"
```

---

## Task 2: Store — budget state

**Files:**
- Modify: `src/lib/store.svelte.ts`

- [ ] **Step 1: Add state + methods**

After the `boqItems` state field, add:

```ts
  boqBudget = $state<number | null>(null);
```

Add methods near `refreshBoqItems`:

```ts
  async refreshBoqBudget(): Promise<void> {
    if (!this.currentJob) { this.boqBudget = null; return; }
    this.boqBudget = await ipc.getJobBudget(this.currentJob.id);
  }

  async setBoqBudget(budget: number | null): Promise<void> {
    if (!this.currentJob) return;
    await ipc.setJobBudget(this.currentJob.id, budget);
    this.boqBudget = budget;
    await ipc.touchLastSave();
  }
```

- [ ] **Step 2: Load budget on job open**

In `openJob(jobId)`, right after the `this.boqItems = await ipc.listBoqItems(jobId);` line (added in Plan 2), add:

```ts
    this.boqBudget = await ipc.getJobBudget(jobId);
```

- [ ] **Step 3: Verify type-check**

Run: `cd /Users/cncuser/Desktop/GanttBok && npm run check`
Expected: no new errors (baseline 3 pre-existing).

- [ ] **Step 4: Commit**

```bash
git add src/lib/store.svelte.ts
git commit -m "feat(boq): store budget state + load/set"
```

---

## Task 3: Financials panel + toolbar toggle + layout

**Files:**
- Create: `src/lib/boq/FinancialsPanel.svelte`
- Modify: `src/lib/boq/BoqToolbar.svelte`, `src/lib/boq/BoqPage.svelte`

- [ ] **Step 1: Create the panel**

Create `src/lib/boq/FinancialsPanel.svelte`:

```svelte
<script lang="ts">
  import { store } from '../store.svelte';
  import { financials, sectorRollups } from './boq-financials';

  const f = $derived(financials(store.boqItems, store.boqBudget));
  const rollups = $derived(sectorRollups(store.boqItems));
  const budget = $derived(store.boqBudget);

  let editingBudget = $state(false);
  let budgetDraft = $state('');
  const expanded = $state<Set<string>>(new Set());

  function fmt(n: number): string { return 'R ' + Math.round(n).toLocaleString('en-ZA'); }
  function pct(n: number): number {
    const denom = budget && budget > 0 ? budget : (f.projected || 1);
    return Math.max(0, Math.min(100, (n / denom) * 100));
  }
  function startBudget(): void { editingBudget = true; budgetDraft = budget == null ? '' : String(budget); }
  async function commitBudget(): Promise<void> {
    const n = budgetDraft.trim() === '' ? null : Number(budgetDraft.replace(/[, ]/g, ''));
    editingBudget = false;
    await store.setBoqBudget(Number.isFinite(n as number) ? n : null);
  }
  function toggle(trade: string): void {
    const next = new Set(expanded);
    next.has(trade) ? next.delete(trade) : next.add(trade);
    // reassign so Svelte tracks it
    expanded.clear(); next.forEach(t => expanded.add(t));
  }
  function itemsForTrade(trade: string) {
    return store.boqItems.filter(i => (i.trade ?? 'Untraded') === trade);
  }
</script>

<aside class="fin">
  <header class="fin-head">
    <h4>Financials</h4>
    <button class="x" title="Hide" onclick={() => (store.showBoqFinancials = false)}>×</button>
  </header>

  <div class="row">
    <span>Budget</span>
    {#if editingBudget}
      <input class="binput" bind:value={budgetDraft} autofocus
        onblur={commitBudget}
        onkeydown={(e) => { if (e.key === 'Enter') commitBudget(); if (e.key === 'Escape') editingBudget = false; }} />
    {:else}
      <button class="budget" onclick={startBudget}>{budget == null ? 'Set budget' : fmt(budget)}</button>
    {/if}
  </div>

  <div class="hero">
    <div class="label">Spent — left the account</div>
    <div class="big">{fmt(f.spent)}</div>
    <div class="bar" class:over={f.overBudget}>
      <div class="seg del" style="width:{pct(f.delivered)}%"></div>
      <div class="seg ord" style="width:{pct(f.ordered)}%"></div>
      <div class="seg quo" style="width:{pct(f.quoted)}%"></div>
    </div>
    <div class="legend">
      <span><i class="del"></i>Delivered {fmt(f.delivered)}</span>
      <span><i class="ord"></i>Ordered {fmt(f.ordered)}</span>
      <span><i class="quo"></i>Quoted {fmt(f.quoted)}</span>
    </div>
  </div>

  {#if budget != null}
    <div class="row"><span>Remaining budget</span><b class:neg={f.remaining! < 0}>{fmt(f.remaining!)}</b></div>
  {/if}
  <div class="row sub"><span>Projected if all quotes taken</span><span>{fmt(f.projected)}</span></div>
  {#if f.overBudget}<div class="warn">⚠ Over budget — quotes + spend exceed the budget.</div>{/if}

  <hr />
  <h5>By sector (trade)</h5>
  <div class="sectors">
    {#each rollups as r (r.trade)}
      <div class="sec-row" onclick={() => toggle(r.trade)} role="button" tabindex="0"
           onkeydown={(e) => { if (e.key === 'Enter') toggle(r.trade); }}>
        <span>{expanded.has(r.trade) ? '▾' : '▸'} {r.trade}</span>
        <b>{fmt(r.committed)}{#if r.quoted > 0}<span class="q"> (+{Math.round(r.quoted/1000)}k q)</span>{/if}</b>
      </div>
      {#if expanded.has(r.trade)}
        {#each itemsForTrade(r.trade) as it (it.id)}
          <div class="sec-child"><span>{it.item || '(unnamed)'}</span><span>{it.qty != null && it.rate != null ? fmt(it.qty * it.rate) : ''}</span></div>
        {/each}
      {/if}
    {/each}
  </div>
</aside>

<style>
  .fin { width: 320px; flex-shrink: 0; border-left: 1px solid var(--c-border); background: var(--c-panel);
    overflow-y: auto; padding: var(--sp-3); box-sizing: border-box; font-size: var(--font-size-sm); }
  .fin-head { display: flex; align-items: center; justify-content: space-between; }
  .fin-head h4 { margin: 0; font-size: var(--font-size-base); }
  .x { border: 0; background: transparent; color: var(--c-text-muted); font-size: 18px; cursor: pointer; }
  .row { display: flex; align-items: center; justify-content: space-between; padding: var(--sp-1) 0; }
  .row.sub { color: var(--c-text-muted); font-size: var(--font-size-xs); }
  .budget { border: 0; background: transparent; color: var(--c-text); font: inherit; font-weight: 600; cursor: pointer; text-decoration: underline dotted; }
  .binput { width: 120px; text-align: right; font: inherit; border: 1px solid var(--c-accent); border-radius: 4px; background: var(--c-bg); color: var(--c-text); }
  .hero { margin: var(--sp-2) 0; }
  .hero .label { font-size: var(--font-size-xs); text-transform: uppercase; letter-spacing: .5px; color: var(--c-text-muted); }
  .big { font-size: 24px; font-weight: 700; }
  .bar { display: flex; height: 16px; border-radius: 8px; overflow: hidden; background: rgba(128,128,128,0.22); margin: var(--sp-2) 0; }
  .bar.over { outline: 2px solid #d64545; }
  .seg.del { background: #2f7d54; } .seg.ord { background: #57b083; }
  .seg.quo { background: repeating-linear-gradient(45deg,#d9a441,#d9a441 5px,#c9962f 5px,#c9962f 10px); }
  .legend { display: flex; flex-wrap: wrap; gap: var(--sp-2); font-size: var(--font-size-xs); color: var(--c-text-muted); }
  .legend i { display: inline-block; width: 10px; height: 10px; border-radius: 2px; margin-right: 4px; vertical-align: middle; }
  .legend i.del { background: #2f7d54; } .legend i.ord { background: #57b083; } .legend i.quo { background: #d9a441; }
  .neg { color: #d64545; }
  .warn { color: #d64545; font-size: var(--font-size-xs); margin-top: var(--sp-1); }
  hr { border: 0; border-top: 1px solid var(--c-border); margin: var(--sp-3) 0; }
  h5 { margin: 0 0 var(--sp-1); font-size: var(--font-size-xs); text-transform: uppercase; letter-spacing: .5px; color: var(--c-text-muted); }
  .sec-row { display: flex; justify-content: space-between; padding: var(--sp-1) 0; border-bottom: 1px dotted var(--c-border); cursor: pointer; }
  .sec-child { display: flex; justify-content: space-between; padding: 2px 0 2px var(--sp-3); font-size: var(--font-size-xs); color: var(--c-text-muted); }
  .q { color: #c9962f; font-size: var(--font-size-xs); }
</style>
```

- [ ] **Step 2: Add the state flag + toggle + Export menu to the toolbar**

First, add a UI flag to the store (in `src/lib/store.svelte.ts`, next to `boqBudget`):

```ts
  showBoqFinancials = $state<boolean>(false);
```

In `src/lib/boq/BoqToolbar.svelte`, add `exportBoq` import and Financials toggle. Change the script imports to add store is already imported. After the `+ Add item` button, add the Export dropdown and Financials toggle:

```svelte
  <div class="export">
    <button class="btn" onclick={() => (showExport = !showExport)}>⤓ Export ▾</button>
    {#if showExport}
      <div class="menu">
        <button class="menu-btn" onclick={() => doExport('xlsx')}>Export .xlsx</button>
        <button class="menu-btn" onclick={() => doExport('ods')}>Export .ods</button>
      </div>
    {/if}
  </div>
  <button class="btn" class:on={store.showBoqFinancials} onclick={() => (store.showBoqFinancials = !store.showBoqFinancials)}>◧ Financials</button>
```

Add to the toolbar `<script>`:

```ts
  import * as ipc from '../ipc';
  import { revealItemInDir } from '@tauri-apps/plugin-opener';
  let showExport = $state(false);
  async function doExport(format: 'xlsx' | 'ods'): Promise<void> {
    showExport = false;
    if (!store.currentJob) return;
    try {
      const path = await ipc.exportBoq(store.currentJob.id, format);
      await revealItemInDir(path);
    } catch (e) {
      (window as unknown as { __toast?: (m: string) => void }).__toast?.(`Export failed: ${e}`);
    }
  }
```

Add styles:

```css
  .export { position: relative; }
  .menu { position: absolute; top: 110%; right: 0; z-index: 30; background: var(--c-panel);
    border: 1px solid var(--c-border); border-radius: 6px; padding: var(--sp-1); box-shadow: 0 6px 20px rgba(0,0,0,0.18); }
  .menu-btn { display: block; width: 100%; text-align: left; border: 0; background: transparent; color: var(--c-text);
    font: inherit; font-size: var(--font-size-sm); padding: var(--sp-1) var(--sp-2); cursor: pointer; white-space: nowrap; border-radius: 4px; }
  .menu-btn:hover { background: var(--c-accent-fade); }
  .btn.on { background: var(--c-accent); color: #fff; border-color: var(--c-accent); }
```

Note: the `.btn.primary` rule sets `margin-left:auto` on "+ Add item"; keep Export + Financials AFTER it so they sit at the right end.

- [ ] **Step 3: Lay out grid + panel side-by-side in BoqPage**

Replace `src/lib/boq/BoqPage.svelte` with:

```svelte
<script lang="ts">
  import { store } from '../store.svelte';
  import BoqToolbar from './BoqToolbar.svelte';
  import BoqGrid from './BoqGrid.svelte';
  import FinancialsPanel from './FinancialsPanel.svelte';
  import { DEFAULT_HIDDEN, type ColumnKey, type StatusFilter } from './boq-grid';

  let status = $state<StatusFilter>('all');
  let search = $state('');
  let hidden = $state<Set<ColumnKey>>(new Set(DEFAULT_HIDDEN));
</script>

<div class="boq-page">
  {#if store.currentJob}
    <BoqToolbar bind:status bind:search bind:hidden />
    <div class="body">
      <div class="grid-col"><BoqGrid {status} {search} {hidden} /></div>
      {#if store.showBoqFinancials}<FinancialsPanel />{/if}
    </div>
  {:else}
    <p class="placeholder">Pick a job to see its Bill of Quantities.</p>
  {/if}
</div>

<style>
  .boq-page { display: flex; flex-direction: column; height: 100%; overflow: hidden; background: var(--c-bg); }
  .body { display: flex; flex: 1; min-height: 0; overflow: hidden; }
  .grid-col { flex: 1; min-width: 0; overflow: hidden; }
  .placeholder { color: var(--c-text-muted); padding: var(--sp-4); }
</style>
```

- [ ] **Step 4: Verify**

Run: `cd /Users/cncuser/Desktop/GanttBok && npm run check && npm run build`
Expected: no new type errors; build succeeds.

- [ ] **Step 5: Commit**

```bash
git add src/lib/boq/FinancialsPanel.svelte src/lib/boq/BoqToolbar.svelte src/lib/boq/BoqPage.svelte src/lib/store.svelte.ts
git commit -m "feat(boq): financials panel + toolbar toggle + export menu"
```

---

## Task 4: XLSX export (Rust)

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/commands/boq_export.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the dependency**

In `src-tauri/Cargo.toml`, under `[dependencies]`, add:

```toml
rust_xlsxwriter = "0.79"
```

- [ ] **Step 2: Write the failing test + implementation**

Create `src-tauri/src/commands/boq_export.rs` (implementation + tests together — the testable unit is `build_xlsx`):

```rust
use std::path::PathBuf;
use rust_xlsxwriter::{Workbook, Formula};
use tauri::State;

use crate::commands::Db;
use crate::db::models::{BoqItem, Procurement};
use crate::repo::boq as boq_repo;
use crate::{GbError, GbResult};

const HEADERS: &[&str] = &[
    "Item", "Qty", "Unit", "Rate", "Cost", "Trade", "Full Spec",
    "W (mm)", "D (mm)", "H (mm)", "Ø (mm)", "Supplier", "Location",
    "Procurement", "Lead (wks)", "Invoice #", "Tut Ref No", "Organisation",
];

fn proc_label(p: Procurement) -> &'static str {
    match p {
        Procurement::NotOrdered => "Not ordered",
        Procurement::Quoted => "Quoted",
        Procurement::Ordered => "Ordered",
        Procurement::Delivered => "Delivered",
    }
}

/// Build an .xlsx workbook (one BoQ sheet) as bytes.
/// Cost cells are LIVE formulas (=Qty*Rate); a grand-total row uses =SUM.
pub fn build_xlsx(items: &[BoqItem]) -> GbResult<Vec<u8>> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet().set_name("BoQ").map_err(xlsx_err)?;

    for (c, h) in HEADERS.iter().enumerate() {
        ws.write_string(0, c as u16, *h).map_err(xlsx_err)?;
    }

    for (i, it) in items.iter().enumerate() {
        let r = (i + 1) as u32; // 0-based row index; excel row = r+1
        let xl = r + 1;
        ws.write_string(r, 0, &it.item).map_err(xlsx_err)?;
        if let Some(q) = it.qty { ws.write_number(r, 1, q).map_err(xlsx_err)?; }
        if let Some(u) = &it.unit { ws.write_string(r, 2, u).map_err(xlsx_err)?; }
        if let Some(rate) = it.rate { ws.write_number(r, 3, rate).map_err(xlsx_err)?; }
        // Cost = Qty*Rate as a live formula (blank-safe: shows 0 when either is empty).
        ws.write_formula(r, 4, Formula::new(format!("=IF(OR(B{xl}=\"\",D{xl}=\"\"),\"\",B{xl}*D{xl})"))).map_err(xlsx_err)?;
        if let Some(t) = &it.trade { ws.write_string(r, 5, t).map_err(xlsx_err)?; }
        if let Some(s) = &it.full_spec { ws.write_string(r, 6, s).map_err(xlsx_err)?; }
        if let Some(v) = it.w_mm { ws.write_number(r, 7, v).map_err(xlsx_err)?; }
        if let Some(v) = it.d_mm { ws.write_number(r, 8, v).map_err(xlsx_err)?; }
        if let Some(v) = it.h_mm { ws.write_number(r, 9, v).map_err(xlsx_err)?; }
        if let Some(v) = it.dia_mm { ws.write_number(r, 10, v).map_err(xlsx_err)?; }
        if let Some(s) = &it.supplier { ws.write_string(r, 11, s).map_err(xlsx_err)?; }
        if let Some(s) = &it.location { ws.write_string(r, 12, s).map_err(xlsx_err)?; }
        ws.write_string(r, 13, proc_label(it.procurement)).map_err(xlsx_err)?;
        if let Some(v) = it.lead_weeks { ws.write_number(r, 14, v).map_err(xlsx_err)?; }
        if let Some(s) = &it.invoice_no { ws.write_string(r, 15, s).map_err(xlsx_err)?; }
        if let Some(s) = &it.tut_ref_no { ws.write_string(r, 16, s).map_err(xlsx_err)?; }
        if let Some(s) = &it.organisation { ws.write_string(r, 17, s).map_err(xlsx_err)?; }
    }

    // Grand total (live SUM over the Cost column), one blank row below the data.
    if !items.is_empty() {
        let total_row = (items.len() + 2) as u32;
        ws.write_string(total_row, 3, "TOTAL").map_err(xlsx_err)?;
        ws.write_formula(total_row, 4, Formula::new(format!("=SUM(E2:E{})", items.len() + 1))).map_err(xlsx_err)?;
    }

    wb.save_to_buffer().map_err(xlsx_err)
}

fn xlsx_err(e: rust_xlsxwriter::XlsxError) -> GbError {
    GbError::Validation(format!("xlsx: {e}"))
}

/// Export a job's BoQ to Downloads. `format` is "xlsx" or "ods".
/// XLSX is written directly; ODS is produced by converting the XLSX via
/// headless LibreOffice (`soffice`), which must be installed for ODS.
#[tauri::command]
pub fn export_boq(db: State<Db>, job_id: i64, format: String) -> GbResult<String> {
    let items = {
        let conn = db.0.lock().unwrap();
        boq_repo::list_by_job(&conn, job_id)?
    };
    let bytes = build_xlsx(&items)?;

    let dir = dirs::download_dir()
        .ok_or_else(|| GbError::Validation("no Downloads directory".into()))?;
    let xlsx_path = dir.join("Bill_of_Quantities_export.xlsx");
    std::fs::write(&xlsx_path, &bytes)?;

    match format.as_str() {
        "xlsx" => Ok(xlsx_path.to_string_lossy().into_owned()),
        "ods" => convert_to_ods(&xlsx_path, &dir),
        other => Err(GbError::Validation(format!("unknown export format: {other}"))),
    }
}

/// Convert an .xlsx to .ods using headless LibreOffice. Returns the .ods path.
fn convert_to_ods(xlsx_path: &std::path::Path, out_dir: &std::path::Path) -> GbResult<String> {
    let status = std::process::Command::new("soffice")
        .args(["--headless", "--convert-to", "ods", "--outdir"])
        .arg(out_dir)
        .arg(xlsx_path)
        .status()
        .map_err(|e| GbError::Validation(format!("LibreOffice (soffice) not found for ODS export: {e}")))?;
    if !status.success() {
        return Err(GbError::Validation("soffice conversion to ODS failed".into()));
    }
    let ods_path: PathBuf = out_dir.join("Bill_of_Quantities_export.ods");
    Ok(ods_path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::Procurement;

    fn mk(item: &str, qty: Option<f64>, rate: Option<f64>, proc: Procurement) -> BoqItem {
        BoqItem {
            id: 1, job_id: 1, order_index: 0, item: item.into(), qty, unit: None, rate,
            trade: Some("HVAC".into()), full_spec: None, w_mm: None, d_mm: None, h_mm: None,
            dia_mm: None, supplier: None, location: None, procurement: proc, delivered_date: None,
            lead_weeks: None, invoice_no: None, tut_ref_no: None, organisation: None,
            created_at: "2026-07-06".into(),
        }
    }

    #[test]
    fn build_xlsx_produces_a_valid_zip() {
        let items = vec![
            mk("Heat pump", Some(1.0), Some(49444.25), Procurement::Ordered),
            mk("Buffer tank", Some(2.0), Some(48836.0), Procurement::Delivered),
        ];
        let bytes = build_xlsx(&items).unwrap();
        // .xlsx is a zip → starts with "PK\x03\x04".
        assert!(bytes.len() > 100);
        assert_eq!(&bytes[0..2], b"PK");
    }

    #[test]
    fn build_xlsx_handles_empty() {
        let bytes = build_xlsx(&[]).unwrap();
        assert_eq!(&bytes[0..2], b"PK");
    }
}
```

- [ ] **Step 3: Declare + register**

In `src-tauri/src/commands/mod.rs`, add `pub mod boq_export;`.
In `src-tauri/src/lib.rs`, add to `generate_handler![ ... ]` after the boq commands:

```rust
            commands::boq_export::export_boq,
```

- [ ] **Step 4: Run tests**

Run: `cd /Users/cncuser/Desktop/GanttBok/src-tauri && cargo test --lib boq_export && cargo build`
Expected: the two `build_xlsx` tests pass; build succeeds (rust_xlsxwriter compiles).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/commands/boq_export.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(boq): XLSX export (live formulas) + ODS via LibreOffice"
```

---

## Task 5: Frontend export wrapper

**Files:**
- Modify: `src/lib/ipc.ts`

(The toolbar UI calling `ipc.exportBoq` was added in Task 3; this task adds the wrapper it calls. If Task 3's `npm run build` failed on the missing `exportBoq`, do this task first — the two are a pair.)

- [ ] **Step 1: Add the wrapper**

In `src/lib/ipc.ts`, append:

```ts
export const exportBoq = (jobId: number, format: 'xlsx' | 'ods') =>
  invoke<string>('export_boq', { jobId, format });
```

- [ ] **Step 2: Verify**

Run: `cd /Users/cncuser/Desktop/GanttBok && npm run check && npm run build`
Expected: no new type errors; build succeeds.

- [ ] **Step 3: Commit**

```bash
git add src/lib/ipc.ts
git commit -m "feat(boq): exportBoq IPC wrapper"
```

> **Note on `revealItemInDir`:** it's part of `@tauri-apps/plugin-opener` (already a dependency) and the `opener:default` capability is already granted. If `revealItemInDir` isn't exported in the installed version, fall back to `import { openPath } from '@tauri-apps/plugin-opener'` and open the containing directory. Verify against the installed package before committing Task 3/5.

---

## Task 6: One-time import script

**Files:**
- Create: `scripts/import_boq_from_ods.py`

This is a one-off, run manually after the Noordhoek job exists in the app. It reads the existing `.ods` and inserts `boq_item` rows.

- [ ] **Step 1: Write the script**

Create `scripts/import_boq_from_ods.py`:

```python
#!/usr/bin/env python3
"""One-time import of the LibreOffice BoQ sheet into Blik Plan's SQLite DB.

Usage:
    python3 scripts/import_boq_from_ods.py <job_id> [--ods PATH] [--db PATH] [--dry-run]

Maps the old free-text Status column onto the Procurement lifecycle:
    Complete                                   -> delivered
    In Progress                                -> ordered
    everything else (Not Started / Awaiting
    Decision / Ready to order / blank)         -> not_ordered
Rows with a filled Rate but Status 'Complete' stay 'delivered'.
Review + correct edge cases in-app after import.
"""
import argparse, os, sqlite3, sys, zipfile
import xml.etree.ElementTree as ET

T = 'urn:oasis:names:tc:opendocument:xmlns:table:1.0'
TEXTNS = 'urn:oasis:names:tc:opendocument:xmlns:text:1.0'
OFFICE = 'urn:oasis:names:tc:opendocument:xmlns:office:1.0'

DEFAULT_ODS = os.path.expanduser('~/Downloads/Bill_of_Quantities.ods')

def default_db():
    base = os.path.expanduser('~/Library/Application Support')
    for name in ('Blik Plan', 'Gantt Bok'):
        p = os.path.join(base, name, 'ganttbok.db')
        if os.path.exists(p):
            return p
    return os.path.join(base, 'Gantt Bok', 'ganttbok.db')

# Sheet column index (0-based) -> boq_item column.
COLS = ['item','qty','unit','rate',None,'trade','full_spec','w_mm','d_mm','h_mm',
        'dia_mm','supplier','location','status','lead_weeks','invoice_no','tut_ref_no','organisation']
NUMERIC = {'qty','rate','w_mm','d_mm','h_mm','dia_mm','lead_weeks'}

def cell_text(c):
    return ' '.join(''.join(p.itertext()) for p in c.iter(f'{{{TEXTNS}}}p')).strip()

def read_boq_rows(ods_path):
    z = zipfile.ZipFile(ods_path)
    root = ET.fromstring(z.read('content.xml'))
    for tbl in root.iter(f'{{{T}}}table'):
        if tbl.get(f'{{{T}}}name') != 'BoQ':
            continue
        rows = []
        for ri, row in enumerate(tbl.iter(f'{{{T}}}table-row')):
            if ri == 0:
                continue  # header
            cells = []
            for c in row.findall(f'{{{T}}}table-cell'):
                rep = int(c.get(f'{{{T}}}number-columns-repeated', '1'))
                val = c.get(f'{{{OFFICE}}}value') or cell_text(c)
                cells.extend([val] * min(rep, 50))
            if any(x for x in cells):
                rows.append(cells)
        return rows
    raise SystemExit('No "BoQ" sheet found in the .ods')

def to_procurement(status):
    s = (status or '').strip().lower()
    if s == 'complete':
        return 'delivered'
    if s == 'in progress':
        return 'ordered'
    return 'not_ordered'

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('job_id', type=int)
    ap.add_argument('--ods', default=DEFAULT_ODS)
    ap.add_argument('--db', default=default_db())
    ap.add_argument('--dry-run', action='store_true')
    a = ap.parse_args()

    rows = read_boq_rows(a.ods)
    conn = sqlite3.connect(a.db)
    conn.execute('PRAGMA foreign_keys = ON')
    if not conn.execute('SELECT 1 FROM job WHERE id = ?', (a.job_id,)).fetchone():
        sys.exit(f'job {a.job_id} not found in {a.db}')

    start = conn.execute(
        'SELECT COALESCE(MAX(order_index)+1, 0) FROM boq_item WHERE job_id = ?', (a.job_id,)
    ).fetchone()[0]

    inserted = 0
    for i, cells in enumerate(rows):
        rec = {'job_id': a.job_id, 'order_index': start + i, 'procurement': 'not_ordered'}
        for ci, key in enumerate(COLS):
            if key is None or ci >= len(cells):
                continue
            raw = (cells[ci] or '').strip()
            if key == 'status':
                rec['procurement'] = to_procurement(raw)
            elif key in NUMERIC:
                try: rec[key] = float(raw) if raw else None
                except ValueError: rec[key] = None
            else:
                rec[key] = raw or ('' if key == 'item' else None)
        cols = ','.join(rec.keys())
        ph = ','.join(['?'] * len(rec))
        if a.dry_run:
            print(rec.get('item'), '->', rec['procurement'])
        else:
            conn.execute(f'INSERT INTO boq_item ({cols}) VALUES ({ph})', list(rec.values()))
            inserted += 1
    if not a.dry_run:
        conn.commit()
    print(f"{'(dry-run) ' if a.dry_run else ''}rows read: {len(rows)}, inserted: {inserted}")

if __name__ == '__main__':
    main()
```

- [ ] **Step 2: Dry-run to verify parsing (does not write)**

Run: `cd /Users/cncuser/Desktop/GanttBok && python3 scripts/import_boq_from_ods.py 1 --dry-run`
Expected: prints each item name → mapped procurement, and a summary count. (Use the real Noordhoek job id in place of `1` for the real run; back up `ganttbok.db` first.)

- [ ] **Step 3: Commit**

```bash
git add scripts/import_boq_from_ods.py
git commit -m "feat(boq): one-time .ods → boq_item import script"
```

---

## Task 7: Full verification

- [ ] **Step 1: Frontend**

Run: `cd /Users/cncuser/Desktop/GanttBok && npm run check && npm run test && npm run build`
Expected: no new type errors; all vitest green (boq-grid + boq-financials); build succeeds.

- [ ] **Step 2: Rust**

Run: `cd /Users/cncuser/Desktop/GanttBok/src-tauri && cargo test --lib`
Expected: all green (incl. `boq_export` tests).

- [ ] **Step 3: Manual smoke (JT, in the running app)**

1. Toggle **◧ Financials** — panel docks right; toggling hides it.
2. Click **Budget** → set `2000000` → the bar fills and Remaining shows.
3. Set a few items to Ordered/Delivered/Quoted → Spent, the bar segments, and sector rollups update live; expand a sector to see its items.
4. Over-budget: set a low budget → the bar shows the red over-budget outline + warning.
5. **Export .xlsx** → file appears in Downloads and is revealed; open it — Cost cells are live `=Qty*Rate`, grand total is `=SUM`.
6. **Export .ods** → (requires LibreOffice) an `.ods` is produced in Downloads.

---

## Done criteria

- Financials panel: budget (editable), Spent = Ordered+Delivered headline, budget bar (delivered/ordered/quoted/free) with over-budget red state, Remaining, Projected, collapsible sector rollups. Logic vitest-covered.
- Export: `.xlsx` with live formulas written to Downloads and revealed; `.ods` via LibreOffice conversion.
- One-time import script reads the real `.ods` and inserts mapped `boq_item` rows (dry-run verified).
- `npm run check` / `npm run test` / `npm run build` / `cargo test --lib` all green.

**Feature complete** (Plans 1–3). Remaining follow-ups tracked separately: persist column show/hide to `app_meta`; pre-existing `chaser/telegram.rs` doctest + `App.svelte` type errors; Phase 2 (invoice→BoQ→Deslin automation).
