<script lang="ts">
  import { state } from '../store.svelte';
  import JobItem from './JobItem.svelte';
</script>

<div class="sidebar">
  <header><h2>Gantt Bok</h2></header>

  <section>
    <h3>Active</h3>
    {#each state.jobs as job (job.id)}
      <JobItem {job} />
    {:else}
      <p class="hint">No jobs yet</p>
    {/each}
  </section>

  <footer>
    <button class="new-job">+ New job</button>
  </footer>
</div>

<style>
  .sidebar { display: flex; flex-direction: column; height: 100%; }
  header   { padding: var(--sp-3); border-bottom: 1px solid var(--c-border); }
  header h2 { font-size: var(--font-size-base); font-weight: 600; margin: 0; }
  section  { flex: 1; padding: var(--sp-2) 0; overflow-y: auto; }
  section h3 {
    font-size: var(--font-size-xs); text-transform: uppercase;
    color: var(--c-text-muted); letter-spacing: 0.06em;
    padding: var(--sp-2) var(--sp-3); margin: 0;
  }
  footer   { padding: var(--sp-2); border-top: 1px solid var(--c-border); }
  .new-job {
    width: 100%; padding: var(--sp-2); border: 1px solid var(--c-border);
    background: var(--c-bg); border-radius: 4px; cursor: pointer;
  }
  .new-job:hover { background: var(--c-accent-fade); }
  .hint    { color: var(--c-text-muted); padding: var(--sp-3); font-size: var(--font-size-sm); }
</style>
