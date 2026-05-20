<script lang="ts">
  import { state } from '../store.svelte';
  import HeaderStrip from './HeaderStrip.svelte';
  import LeftRail from './LeftRail.svelte';
  import NoWorkColumn from './NoWorkColumn.svelte';
  import TaskBar from './TaskBar.svelte';
  import PhaseBar from './PhaseBar.svelte';
  import { computeViewportDays } from '../calendar';
  import type { Phase, Task } from '../types';

  const days = $derived.by(() => {
    if (!state.currentJob) return [];
    return computeViewportDays(
      state.currentJob.project_start_date,
      state.tasks,
    );
  });

  type Row =
    | { kind: 'phase'; phase: Phase }
    | { kind: 'task'; task: Task; phase: Phase };

  const rows = $derived.by((): Row[] => {
    const out: Row[] = [];
    for (const phase of state.phases) {
      out.push({ kind: 'phase', phase });
      if (!phase.collapsed) {
        for (const task of (state.tasksByPhase.get(phase.id) ?? [])) {
          out.push({ kind: 'task', task, phase });
        }
      }
    }
    return out;
  });

  const ROW_H = 32;
  const CELL = 24;
  const totalHeight = $derived(rows.length * ROW_H);
</script>

<div class="gantt" style="--cell-w: {CELL}px;">
  <LeftRail />
  <div class="grid-area" style="--total-w: {days.length * CELL}px;">
    <HeaderStrip {days} />
    <div class="rows" style="height: {totalHeight}px;">
      <NoWorkColumn {days} {totalHeight} />
      <svg
        width={days.length * CELL}
        height={totalHeight}
        class="canvas-svg"
        onclick={() => state.select(null)}
      >
        {#each rows as r, ri (r.kind === 'phase' ? `p${r.phase.id}` : `t${r.task.id}`)}
          {#if r.kind === 'phase' && r.phase.collapsed}
            <PhaseBar phase={r.phase} tasks={state.tasksByPhase.get(r.phase.id) ?? []} {days} row={ri} />
          {:else if r.kind === 'task'}
            <TaskBar task={r.task} phase={r.phase} {days} row={ri} />
          {/if}
        {/each}
      </svg>
    </div>
  </div>
</div>

<style>
  .gantt {
    display: grid;
    grid-template-columns: var(--left-rail-width) 1fr;
    height: 100%;
    overflow: hidden;
  }
  .grid-area {
    position: relative;
    overflow-x: auto;
    overflow-y: auto;
  }
  .rows {
    position: relative;
    width: var(--total-w);
  }
  .canvas-svg { display: block; }
</style>
