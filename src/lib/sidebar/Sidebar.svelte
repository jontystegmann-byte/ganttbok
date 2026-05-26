<script lang="ts">
  import { store } from '../store.svelte';
  import JobItem from './JobItem.svelte';
  import NewJobModal from './NewJobModal.svelte';
  import ArchivedGroup from './ArchivedGroup.svelte';
  import TemplatesGroup from './TemplatesGroup.svelte';
  import VersionButton from '../components/VersionButton.svelte';
</script>

<div class="sidebar" class:collapsed={store.sidebarCollapsed}>
  {#if !store.sidebarCollapsed}
    <section>
      <h3>Active</h3>
      {#each store.jobs as job (job.id)}
        <JobItem {job} />
      {:else}
        <p class="hint">No jobs yet</p>
      {/each}
    </section>
    <TemplatesGroup />
    <ArchivedGroup />
    <footer>
      <button class="new-job" onclick={() => store.showNewJobModal = true}>+ New job</button>
      <VersionButton />
    </footer>
  {/if}
  <button
    class="collapse-toggle"
    onclick={() => store.sidebarCollapsed = !store.sidebarCollapsed}
    aria-label={store.sidebarCollapsed ? 'Show jobs sidebar' : 'Hide jobs sidebar'}
    title={store.sidebarCollapsed ? 'Show jobs' : 'Hide jobs'}
  >
    <svg width="10" height="14" viewBox="0 0 10 14" aria-hidden="true">
      <path
        d={store.sidebarCollapsed ? 'M2 1 L8 7 L2 13' : 'M8 1 L2 7 L8 13'}
        fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
      />
    </svg>
  </button>
</div>

{#if store.showNewJobModal}
  <NewJobModal />
{/if}

<style>
  .sidebar { display: flex; flex-direction: column; height: 100%; position: relative; }
  section  { flex: 1; padding: var(--sp-2) 0; overflow-y: auto; }

  .collapse-toggle {
    position: absolute;
    top: 50%;
    right: -12px;
    transform: translateY(-50%);
    width: 24px; height: 32px;
    border: 1px solid var(--c-border);
    background: var(--c-panel);
    color: var(--c-text-muted);
    border-radius: 4px;
    cursor: pointer;
    display: flex; align-items: center; justify-content: center;
    padding: 0;
    box-shadow: 0 1px 3px var(--c-shadow);
    z-index: 10;
  }
  .collapse-toggle:hover { color: var(--c-text); background: var(--c-bg); }
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
