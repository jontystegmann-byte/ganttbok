<script lang="ts">
  import { store } from '../store.svelte';
  import TaskDetails from './TaskDetails.svelte';
  import PhaseDetails from './PhaseDetails.svelte';
  import JobDetails from './JobDetails.svelte';
</script>

{#if store.selection?.kind === 'task'}
  <TaskDetails taskId={store.selection.id} />
{:else if store.selection?.kind === 'phase'}
  <PhaseDetails phaseId={store.selection.id} />
{:else if store.currentJob}
  <JobDetails />
{:else}
  <div class="empty">Select a bar to edit</div>
{/if}

<style>
  .empty { padding: var(--sp-4); color: var(--c-text-muted); font-size: var(--font-size-sm); }
</style>
