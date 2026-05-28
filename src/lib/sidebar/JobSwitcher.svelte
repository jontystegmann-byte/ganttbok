<script lang="ts">
  import { store } from '../store.svelte';
  import { portal } from '../portal';
  import JobItem from './JobItem.svelte';
  import TemplatesGroup from './TemplatesGroup.svelte';
  import ArchivedGroup from './ArchivedGroup.svelte';
  let open = $state(false);
  let triggerEl = $state<HTMLButtonElement | null>(null);
  let menuPos = $state<{ left: number; top: number; width: number }>({ left: 0, top: 0, width: 280 });

  const label = $derived(store.currentJob?.name ?? 'No job selected');

  function toggle() {
    if (!open) {
      const r = triggerEl?.getBoundingClientRect();
      if (r) {
        menuPos = { left: r.left, top: r.bottom + 4, width: Math.max(r.width, 280) };
      }
    }
    open = !open;
  }

  function closeOnPickerClick(e: MouseEvent) {
    // The job-item button stops propagation, but its inner select handler runs first.
    // We rely on store.currentJob changing to detect a switch (handled in effect below).
    void e;
  }

  // Close menu when the currentJob id changes (i.e. user picked a different job).
  let lastJobId = $state<number | null>(null);
  $effect(() => {
    const id = store.currentJob?.id ?? null;
    if (open && lastJobId !== null && id !== null && id !== lastJobId) {
      open = false;
    }
    lastJobId = id;
  });
</script>

<button class="trigger" bind:this={triggerEl} onclick={toggle} aria-expanded={open}>
  <span class="job-label">{label}</span>
  <span class="chev" class:open>▾</span>
</button>

{#if open}
  <button class="scrim" onclick={() => (open = false)} aria-label="Close menu"></button>
  <div
    use:portal
    class="menu"
    role="menu"
    style="left: {menuPos.left}px; top: {menuPos.top}px; min-width: {menuPos.width}px;"
    onclick={closeOnPickerClick}
  >
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
      <button class="new-job" onclick={() => { store.showNewJobModal = true; open = false; }}>
        + New job
      </button>
    </footer>
  </div>
{/if}

<style>
  .trigger {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-2);
    padding: var(--sp-1) var(--sp-3);
    background: var(--c-bg);
    border: 1px solid var(--c-border);
    border-radius: 6px;
    cursor: pointer;
    font: inherit;
    color: var(--c-text);
    max-width: 320px;
  }
  .trigger:hover { background: var(--c-accent-fade); }
  .job-label {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chev { color: var(--c-text-muted); font-size: 10px; transition: transform 120ms; }
  .chev.open { transform: rotate(180deg); }

  .scrim {
    position: fixed; inset: 0;
    background: transparent;
    border: none;
    cursor: default;
    z-index: 90;
  }
  .menu {
    position: fixed;
    background: var(--c-panel);
    border: 1px solid var(--c-border);
    border-radius: 8px;
    box-shadow: 0 12px 36px rgba(0,0,0,0.18);
    z-index: 91;
    padding: var(--sp-2) 0;
    max-height: 70vh;
    overflow-y: auto;
  }
  .menu section { padding: var(--sp-1) 0; }
  .menu h3 {
    font-size: var(--font-size-xs);
    text-transform: uppercase;
    color: var(--c-text-muted);
    letter-spacing: 0.06em;
    padding: var(--sp-1) var(--sp-3);
    margin: 0;
  }
  .menu footer { padding: var(--sp-2) var(--sp-3); border-top: 1px solid var(--c-border); margin-top: var(--sp-1); }
  .new-job {
    width: 100%; padding: var(--sp-2);
    border: 1px solid var(--c-border);
    background: var(--c-bg);
    border-radius: 4px; cursor: pointer;
    font: inherit;
  }
  .new-job:hover { background: var(--c-accent-fade); }
  .hint { color: var(--c-text-muted); padding: var(--sp-2) var(--sp-3); font-size: var(--font-size-sm); margin: 0; }
</style>
