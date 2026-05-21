<script lang="ts">
  import { store } from '../store.svelte';
  import JobItem from './JobItem.svelte';
  import NewJobModal from './NewJobModal.svelte';
  import ArchivedGroup from './ArchivedGroup.svelte';
  import TemplatesGroup from './TemplatesGroup.svelte';
  import BottomToolbar from '../components/BottomToolbar.svelte';
</script>

<div class="sidebar">
  <header>
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
      <h2 class="wordmark"><span class="blik">BLIK</span> <span class="plan">Plan</span></h2>
    </div>
  </header>
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
    <BottomToolbar />
  </footer>
</div>

{#if store.showNewJobModal}
  <NewJobModal />
{/if}

<style>
  .sidebar { display: flex; flex-direction: column; height: 100%; }
  header   { padding: var(--sp-3); border-bottom: 1px solid var(--c-border); }
  header .brand { display: flex; align-items: center; gap: var(--sp-2); }
  header .brand-mark { flex-shrink: 0; }
  header .wordmark { margin: 0; font-size: var(--font-size-base); letter-spacing: -0.02em; line-height: 1; }
  header .wordmark .blik { font-weight: 900; }
  header .wordmark .plan { font-weight: 300; color: var(--c-text-muted); }
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
