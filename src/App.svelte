<script lang="ts">
  import { onMount } from 'svelte';
  import { store } from './lib/store.svelte';
  import Sidebar from './lib/sidebar/Sidebar.svelte';
  import GanttCanvas from './lib/canvas/GanttCanvas.svelte';
  import DetailsPanel from './lib/details/DetailsPanel.svelte';
  import SavedIndicator from './lib/footer/SavedIndicator.svelte';

  onMount(async () => {
    await store.bootstrap();

    function onKey(e: KeyboardEvent) {
      const meta = e.metaKey || e.ctrlKey;
      if (!meta) return;
      if (e.key === 'z' && !e.shiftKey) {
        if (store.canUndo()) { e.preventDefault(); store.undo(); }
      } else if (e.key === 'Z' || (e.key === 'z' && e.shiftKey)) {
        if (store.canRedo()) { e.preventDefault(); store.redo(); }
      } else if (e.key === 's') {
        e.preventDefault();
        void store.resyncJobState();
      }
    }

    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
</script>

<div class="app-shell">
  <aside class="sidebar" style="width: {store.sidebarWidth}px">
    <Sidebar />
  </aside>

  <main class="canvas-pane">
    {#if store.currentJob}
      <GanttCanvas />
    {:else}
      <div class="empty-state">
        <h1>Gantt Bok</h1>
        <p>Pick a job from the left, or create a new one.</p>
      </div>
    {/if}
  </main>

  {#if store.selection}
    <aside class="details">
      <DetailsPanel />
    </aside>
  {/if}
</div>

<SavedIndicator />

<style>
  .app-shell {
    display: grid;
    grid-template-columns: auto 1fr auto;
    height: 100vh;
    overflow: hidden;
  }
  .sidebar {
    border-right: 1px solid var(--c-border);
    background: var(--c-panel);
    overflow-y: auto;
    min-width: 180px;
    max-width: 480px;
  }
  .canvas-pane {
    overflow: auto;
    background: var(--c-bg);
  }
  .details {
    width: var(--details-width);
    border-left: 1px solid var(--c-border);
    background: var(--c-panel);
    overflow-y: auto;
  }
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--c-text-muted);
  }
  .empty-state h1 {
    font-size: var(--font-size-xl);
    font-weight: 600;
    margin: 0 0 var(--sp-2);
    color: var(--c-text);
  }
</style>
