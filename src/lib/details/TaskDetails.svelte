<script lang="ts">
  import { state } from '../store.svelte';
  import * as ipc from '../ipc';
  import type { Task } from '../types';

  let { taskId }: { taskId: number } = $props();
  const task = $derived(state.tasks.find(t => t.id === taskId));

  let name = $state('');
  let duration = $state(1);
  let notes = $state('');

  $effect(() => {
    if (task) {
      name = task.name;
      duration = task.duration_workdays;
      notes = task.notes ?? '';
    }
  });

  async function save() {
    if (!task) return;
    const updated: Task = {
      ...task,
      name: name.trim() || task.name,
      duration_workdays: Math.max(1, duration),
      notes: notes.trim() || null,
    };
    await ipc.updateTask($state.snapshot(updated));
    state.tasks = state.tasks.map(t => t.id === updated.id ? updated : t);
    await ipc.touchLastSave();
  }

  async function del() {
    if (!task) return;
    if (!confirm(`Delete task "${task.name}"?`)) return;
    await ipc.deleteTask(task.id);
    state.tasks = state.tasks.filter(t => t.id !== task.id);
    state.select(null);
    await ipc.touchLastSave();
  }
</script>

{#if task}
  <div class="task-details">
    <h2>{task.name}</h2>
    <label>Name<input bind:value={name} onblur={save} /></label>
    <label>Duration (workdays)<input type="number" min="1" bind:value={duration} onblur={save} /></label>
    <label>Start<input type="date" value={task.start_date} disabled /></label>
    <label>Notes<textarea bind:value={notes} onblur={save} rows="4"></textarea></label>
    <button class="danger" onclick={del}>Delete task</button>
  </div>
{/if}

<style>
  .task-details { display: flex; flex-direction: column; gap: var(--sp-3); padding: var(--sp-4); }
  h2 { font-size: var(--font-size-lg); margin: 0; }
  label { display: flex; flex-direction: column; gap: var(--sp-1); font-size: var(--font-size-sm); color: var(--c-text-muted); }
  input, textarea { padding: var(--sp-2); border: 1px solid var(--c-border); border-radius: 4px; font: inherit; color: var(--c-text); background: var(--c-bg); }
  input:disabled { color: var(--c-text-muted); }
  .danger {
    margin-top: var(--sp-3); padding: var(--sp-2);
    background: transparent; border: 1px solid #DC2626; color: #DC2626; border-radius: 4px;
    cursor: pointer;
  }
  .danger:hover { background: #FEE2E2; }
</style>
