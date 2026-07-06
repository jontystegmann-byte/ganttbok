<script lang="ts">
  import { store } from '../store.svelte';
  import HeaderActions from './HeaderActions.svelte';
  import SavedIndicator from '../footer/SavedIndicator.svelte';
  import VersionButton from './VersionButton.svelte';
</script>

<header class="app-header">
  <div class="brand">
    <svg class="brand-mark" width="22" height="18" viewBox="0 0 240 200" aria-hidden="true">
      <g fill="var(--c-accent)">
        <rect x="10" y="20" width="34" height="160"/>
        <path d="M44 20 H86 a40 40 0 0 1 0 80 H44 Z"/>
        <path d="M44 100 H92 a40 40 0 0 1 0 80 H44 Z"/>
        <rect x="44" y="44" width="40" height="32" fill="var(--c-panel)"/>
        <rect x="44" y="124" width="46" height="32" fill="var(--c-panel)"/>
        <rect x="140" y="20" width="34" height="160"/>
        <path d="M174 20 H216 a44 44 0 0 1 0 88 H174 Z"/>
        <rect x="174" y="44" width="40" height="40" fill="var(--c-panel)"/>
      </g>
    </svg>
    <h1 class="wordmark"><span class="blik">BLIK</span> <span class="plan">Plan</span></h1>
  </div>

  <div class="tools">
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
    <HeaderActions />
    <button class="tool-btn" onclick={() => (store.showPrintOptions = true)} title="Print">
      Print
    </button>
  </div>

  <div class="status-end">
    <SavedIndicator />
    <VersionButton />
  </div>
</header>

<style>
  .app-header {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    padding: var(--sp-2) var(--sp-4);
    background: var(--c-panel);
    border-bottom: 1px solid var(--c-border);
    flex-shrink: 0;
    z-index: 20;
    position: relative;
    height: 44px;
  }
  .brand { display: flex; align-items: center; gap: var(--sp-2); flex-shrink: 0; }
  .brand-mark { flex-shrink: 0; }
  .wordmark { margin: 0; font-size: var(--font-size-base); letter-spacing: -0.02em; line-height: 1; }
  .wordmark .blik { font-weight: 900; }
  .wordmark .plan { font-weight: 300; color: var(--c-text-muted); }
  .tools {
    display: flex;
    align-items: center;
    gap: var(--sp-1);
    flex: 1;
  }
  .status-end {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    flex-shrink: 0;
    color: var(--c-text-muted);
    font-size: var(--font-size-xs);
  }
  .tool-btn {
    background: transparent;
    border: 1px solid transparent;
    padding: var(--sp-1) var(--sp-3);
    cursor: pointer;
    border-radius: 4px;
    color: var(--c-text);
    font: inherit;
    font-size: var(--font-size-sm);
  }
  .tool-btn:hover { background: var(--c-accent-fade); color: var(--c-accent); }
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
</style>
