<script lang="ts">
  import { COLUMNS, PROCUREMENT_LABELS, type ColumnKey, type StatusFilter } from './boq-grid';
  import type { Procurement } from '../types';
  import { store } from '../store.svelte';
  import * as ipc from '../ipc';
  import { revealItemInDir } from '@tauri-apps/plugin-opener';

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

  <div class="export">
    <button class="btn" onclick={() => (showExport = !showExport)}>⤓ Export ▾</button>
    {#if showExport}
      <div class="export-menu">
        <button class="menu-btn" onclick={() => doExport('xlsx')}>Export .xlsx</button>
        <button class="menu-btn" onclick={() => doExport('ods')}>Export .ods</button>
      </div>
    {/if}
  </div>
  <button class="btn" class:on={store.showBoqFinancials} onclick={() => (store.showBoqFinancials = !store.showBoqFinancials)}>◧ Financials</button>
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
  .export { position: relative; }
  .export-menu { position: absolute; top: 110%; right: 0; z-index: 30; background: var(--c-panel);
    border: 1px solid var(--c-border); border-radius: 6px; padding: var(--sp-1); box-shadow: 0 6px 20px rgba(0,0,0,0.18); }
  .menu-btn { display: block; width: 100%; text-align: left; border: 0; background: transparent; color: var(--c-text);
    font: inherit; font-size: var(--font-size-sm); padding: var(--sp-1) var(--sp-2); cursor: pointer; white-space: nowrap; border-radius: 4px; }
  .menu-btn:hover { background: var(--c-accent-fade); }
  .btn.on { background: var(--c-accent); color: #fff; border-color: var(--c-accent); }
</style>
