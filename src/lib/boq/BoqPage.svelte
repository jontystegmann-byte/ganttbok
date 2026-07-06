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
