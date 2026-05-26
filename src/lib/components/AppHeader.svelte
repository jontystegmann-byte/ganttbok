<script lang="ts">
  import { store } from '../store.svelte';
  import JobSwitcher from '../sidebar/JobSwitcher.svelte';
  import HeaderActions from './HeaderActions.svelte';
  import SavedIndicator from '../footer/SavedIndicator.svelte';
  import VersionButton from './VersionButton.svelte';
</script>

<header class="app-header">
  <!-- Row 1: status strip — logo · job switcher · saved + version -->
  <div class="status-row">
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

    <div class="job-area">
      <JobSwitcher />
    </div>

    <div class="status-end">
      <SavedIndicator />
      <VersionButton />
    </div>
  </div>

  <!-- Row 2: labelled toolbar — every tool visible -->
  <div class="toolbar-row">
    <HeaderActions />
    <button class="tool-btn" onclick={() => (store.showPrintOptions = true)} title="Print">
      Print
    </button>
  </div>
</header>

<style>
  .app-header {
    display: flex;
    flex-direction: column;
    background: var(--c-panel);
    border-bottom: 1px solid var(--c-border);
    flex-shrink: 0;
    z-index: 20;
    position: relative;
  }

  .status-row {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    padding: var(--sp-2) var(--sp-4);
    height: 40px;
  }
  .brand { display: flex; align-items: center; gap: var(--sp-2); flex-shrink: 0; }
  .brand-mark { flex-shrink: 0; }
  .wordmark { margin: 0; font-size: var(--font-size-base); letter-spacing: -0.02em; line-height: 1; }
  .wordmark .blik { font-weight: 900; }
  .wordmark .plan { font-weight: 300; color: var(--c-text-muted); }
  .job-area { flex: 1; display: flex; }
  .status-end {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    flex-shrink: 0;
    color: var(--c-text-muted);
    font-size: var(--font-size-xs);
  }

  .toolbar-row {
    display: flex;
    align-items: center;
    gap: var(--sp-1);
    padding: 0 var(--sp-4) var(--sp-2);
    border-top: 1px solid var(--c-border);
    padding-top: var(--sp-2);
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
</style>
