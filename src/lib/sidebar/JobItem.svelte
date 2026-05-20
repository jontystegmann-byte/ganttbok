<script lang="ts">
  import type { Job } from '../types';
  import { state } from '../store.svelte';
  let { job }: { job: Job } = $props();
  const isOpen = $derived(state.currentJob?.id === job.id);
  async function open() { await state.openJob(job.id); }
</script>

<button class="job-item" class:open={isOpen} onclick={open}>
  {#if isOpen}<span class="indicator">●</span>{/if}
  <span class="job-name">{job.name}</span>
</button>

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
