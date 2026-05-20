<script lang="ts">
  import { store } from '../store.svelte';
  import JobItem from './JobItem.svelte';
  let expanded = $state(false);
</script>

<section>
  <button class="header" onclick={() => expanded = !expanded}>
    {expanded ? '▾' : '▸'} Archived ({store.archivedJobs.length})
  </button>
  {#if expanded}
    {#each store.archivedJobs as job (job.id)}
      <JobItem {job} />
    {:else}
      <p class="hint">No archived jobs</p>
    {/each}
  {/if}
</section>

<style>
  .header {
    width: 100%; text-align: left; background: transparent; border: none; cursor: pointer;
    padding: var(--sp-2) var(--sp-3);
    font-size: var(--font-size-xs); text-transform: uppercase;
    color: var(--c-text-muted); letter-spacing: 0.06em;
  }
  .hint { color: var(--c-text-muted); padding: var(--sp-3); font-size: var(--font-size-sm); }
</style>
