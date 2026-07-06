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
