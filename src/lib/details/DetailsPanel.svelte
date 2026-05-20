<script lang="ts">
  import { state } from '../store.svelte';
  import TaskDetails from './TaskDetails.svelte';
  import PhaseDetails from './PhaseDetails.svelte';
</script>

{#if state.selection?.kind === 'task'}
  <TaskDetails taskId={state.selection.id} />
{:else if state.selection?.kind === 'phase'}
  <PhaseDetails phaseId={state.selection.id} />
{:else}
  <div class="empty">Select a bar to edit</div>
{/if}

<style>
  .empty { padding: var(--sp-4); color: var(--c-text-muted); font-size: var(--font-size-sm); }
</style>
