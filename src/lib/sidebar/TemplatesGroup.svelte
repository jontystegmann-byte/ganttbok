<script lang="ts">
  import { store } from '../store.svelte';
  import JobItem from './JobItem.svelte';
  let expanded = $state(true);
</script>

<section>
  <button class="header" onclick={() => expanded = !expanded}>
    {expanded ? '▾' : '▸'} Templates ({store.templates.length})
  </button>
  {#if expanded}
    {#each store.templates as job (job.id)}
      <JobItem {job} />
    {:else}
      <p class="hint">No templates yet — right-click any job to save as template</p>
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
  .hint { color: var(--c-text-muted); padding: var(--sp-3); font-size: var(--font-size-sm); font-style: italic; }
</style>
