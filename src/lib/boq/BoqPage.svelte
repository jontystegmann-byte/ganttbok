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
