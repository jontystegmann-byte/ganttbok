<script lang="ts">
  import HeaderActions from './HeaderActions.svelte';
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
  <div class="spacer"></div>
  <HeaderActions />
</header>

<style>
  .app-header {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-2) var(--sp-4);
    background: var(--c-panel);
    border-bottom: 1px solid var(--c-border);
    height: 44px;
    flex-shrink: 0;
    z-index: 20;
    position: relative;
  }
  .brand { display: flex; align-items: center; gap: var(--sp-2); }
  .brand-mark { flex-shrink: 0; }
  .wordmark { margin: 0; font-size: var(--font-size-base); letter-spacing: -0.02em; line-height: 1; }
  .wordmark .blik { font-weight: 900; }
  .wordmark .plan { font-weight: 300; color: var(--c-text-muted); }
  .spacer { flex: 1; }
</style>
