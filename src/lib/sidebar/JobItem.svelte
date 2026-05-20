<script lang="ts">
  import type { Job } from '../types';
  import { store } from '../store.svelte';
  import * as ipc from '../ipc';
  import ContextMenu from '../components/ContextMenu.svelte';

  let { job }: { job: Job } = $props();
  const isOpen = $derived(store.currentJob?.id === job.id);
  let menu = $state<{ x: number; y: number } | null>(null);

  async function open() { await store.openJob(job.id); }

  function onContext(e: MouseEvent) {
    e.preventDefault();
    menu = { x: e.clientX, y: e.clientY };
  }

  const items = $derived(job.is_template ? [
    { label: 'Edit template…', action: open },
    { label: 'Delete template', action: async () => { await ipc.deleteJob(job.id); await store.refreshSidebar(); }, danger: true },
  ] : [
    { label: 'Open', action: open },
    { label: 'Save as template…', action: async () => {
        await ipc.saveAsTemplate(job.id, `${job.name} (template)`);
        await store.refreshSidebar();
    } },
    { label: job.archived ? 'Unarchive' : 'Archive', action: async () => {
        await ipc.archiveJob(job.id, !job.archived);
        await store.refreshSidebar();
        await store.refreshArchived();
    } },
    { label: 'Delete job', action: async () => { await ipc.deleteJob(job.id); await store.refreshSidebar(); await store.refreshArchived(); }, danger: true },
  ]);
</script>

<button class="job-item" class:open={isOpen} onclick={open} oncontextmenu={onContext}>
  {#if isOpen}<span class="indicator">●</span>{/if}
  <span class="job-name">{job.name}</span>
</button>

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} {items} onclose={() => menu = null} />
{/if}

<style>
  .job-item {
    display: flex; align-items: center; gap: var(--sp-2);
    width: 100%; padding: var(--sp-2) var(--sp-3);
    border: none; background: transparent; cursor: pointer;
    text-align: left; font-size: var(--font-size-sm);
    border-left: 3px solid transparent;
  }
  .job-item:hover { background: var(--c-accent-fade); }
  .job-item.open { background: var(--c-accent-fade); border-left-color: var(--c-accent); }
  .indicator { color: var(--c-accent); font-size: 8px; }
  .job-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
