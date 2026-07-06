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
