<script lang="ts">
  import { store } from '../store.svelte';
  import * as ipc from '../ipc';
  import { addWorkdays } from '../calendar';
  import type { Task } from '../types';

  async function toggleCollapse(phaseId: number) {
    const phase = store.phases.find(p => p.id === phaseId);
    if (!phase) return;
    phase.collapsed = !phase.collapsed;
    await ipc.updatePhase($state.snapshot(phase));
  }

  /** Format workdays as a weeks string with 1 decimal place ("0.6w", "1w", "2.4w"). */
  function fmtWeeks(workdays: number): string {
    if (workdays <= 0) return '';
    const weeks = workdays / 5;
    // Strip trailing .0 so whole weeks read clean.
    const rounded = Math.round(weeks * 10) / 10;
    return rounded === Math.floor(rounded) ? `${rounded}w` : `${rounded.toFixed(1)}w`;
  }

  /**
   * Phase duration in workdays — span from earliest task start to latest task end.
   * Matches the PhaseBar's visual span (not the sum of individual task durations).
   */
  function phaseWorkdays(tasks: Task[]): number {
    if (tasks.length === 0) return 0;
    const starts = tasks.map(t => t.start_date).sort();
    const ends = tasks.map(t => addWorkdays(t.start_date, Math.max(0, t.duration_workdays - 1))).sort();
    const start = starts[0];
    const end = ends[ends.length - 1];
    // Count workdays inclusive of both endpoints.
    let cur = start;
    let n = 0;
    while (cur <= end) {
      n++;
      cur = addWorkdays(cur, 1);
    }
    return n;
  }
</script>

<div class="left-rail">
  {#each store.phases as phase, pi (phase.id)}
    <div class="phase-row"
      style="height: var(--row-height);"
      draggable="true"
      ondragstart={(e) => e.dataTransfer!.setData('text/phase-id', String(phase.id))}
      ondragover={(e) => e.preventDefault()}
      ondrop={async (e) => {
        e.preventDefault();
        const draggedId = Number(e.dataTransfer!.getData('text/phase-id'));
        if (!draggedId || draggedId === phase.id) return;
        const ordered = store.phases.map(p => p.id).filter(id => id !== draggedId);
        const targetIdx = store.phases.findIndex(p => p.id === phase.id);
        ordered.splice(targetIdx, 0, draggedId);
        await store.reorderPhases(ordered);
      }}
    >
      <button class="chev" onclick={() => toggleCollapse(phase.id)} aria-label="toggle">
        {phase.collapsed ? '▸' : '▾'}
      </button>
      <span class="num">{pi + 1}.</span>
      <span class="name" onclick={() => store.select({ kind: 'phase', id: phase.id })} role="button" tabindex="0">{phase.name}</span>
      <span class="weeks">{fmtWeeks(phaseWorkdays(store.tasksByPhase.get(phase.id) ?? []))}</span>
    </div>
    {#if !phase.collapsed}
      {@const phaseTasks = store.tasksByPhase.get(phase.id) ?? []}
      {#each phaseTasks as task, ti (task.id)}
        <div class="task-row"
          style="height: var(--row-height);"
          draggable="true"
          ondragstart={(e) => { e.dataTransfer!.setData('text/task-id', String(task.id)); }}
          ondragover={(e) => e.preventDefault()}
          ondrop={async (e) => {
            e.preventDefault();
            const draggedId = Number(e.dataTransfer!.getData('text/task-id'));
            if (!draggedId || draggedId === task.id) return;
            const targetTasks = store.tasksByPhase.get(phase.id) ?? [];
            // Only reorder within same phase.
            if (!targetTasks.some(t => t.id === draggedId)) return;
            const ordered = targetTasks.map(t => t.id).filter(id => id !== draggedId);
            const targetIdx = targetTasks.findIndex(t => t.id === task.id);
            ordered.splice(targetIdx, 0, draggedId);
            await store.reorderTasksInPhase(phase.id, ordered);
          }}
        >
          <span class="num">{pi + 1}.{ti + 1}</span>
          <span class="name" onclick={() => store.select({ kind: 'task', id: task.id })} role="button" tabindex="0">{task.name}</span>
          <span class="weeks">{fmtWeeks(task.duration_workdays)}</span>
        </div>
      {/each}
      <button class="add-task" onclick={async () => {
        await store.createTaskInPhase(phase.id, 'New task');
      }}>+ Task</button>
    {/if}
  {/each}
  <button class="add-phase" onclick={async () => {
    await store.createPhase('New phase');
  }}>+ Phase</button>
</div>

<style>
  .left-rail {
    width: var(--left-rail-width);
    border-right: 1px solid var(--c-border);
    background: var(--c-panel);
    overflow-y: auto;
    padding-top: var(--header-height);
  }
  .phase-row, .task-row {
    display: flex; align-items: center; gap: var(--sp-2);
    padding: 0 var(--sp-2);
    border-bottom: 1px solid var(--c-border);
    font-size: var(--font-size-sm);
  }
  .task-row { padding-left: calc(var(--sp-2) * 4); color: var(--c-text-muted); }
  .chev {
    background: transparent; border: none; cursor: pointer;
    font-size: 10px; color: var(--c-text-muted);
    padding: 0; width: 14px;
  }
  .num { font-variant-numeric: tabular-nums; color: var(--c-text-muted); min-width: 28px; }
  .name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .weeks {
    font-variant-numeric: tabular-nums;
    color: var(--c-text-muted);
    opacity: 0.6;
    font-size: var(--font-size-xs);
    padding-right: var(--sp-1);
    flex-shrink: 0;
  }
  .add-task, .add-phase {
    width: 100%; text-align: left; padding: var(--sp-2) var(--sp-3);
    background: transparent; border: none; cursor: pointer;
    color: var(--c-text-muted); font-size: var(--font-size-sm);
  }
  .add-task:hover, .add-phase:hover { background: var(--c-accent-fade); color: var(--c-accent); }
  .add-task { padding-left: calc(var(--sp-2) * 4); }
</style>
