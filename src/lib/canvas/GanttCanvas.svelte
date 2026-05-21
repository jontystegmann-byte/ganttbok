<script lang="ts">
  import { store } from '../store.svelte';
  import HeaderStrip from './HeaderStrip.svelte';
  import LeftRail from './LeftRail.svelte';
  import NoWorkColumn from './NoWorkColumn.svelte';
  import WeekGridLines from './WeekGridLines.svelte';
  import HoverColumn from './HoverColumn.svelte';
  import TaskBar from './TaskBar.svelte';
  import PhaseBar from './PhaseBar.svelte';
  import DependencyArrow from './DependencyArrow.svelte';
  import DragOverlay from './DragOverlay.svelte';
  import DepCreator from './DepCreator.svelte';
  import { computeViewportDays } from '../calendar';
  import type { Phase, Task } from '../types';
  import * as ipc from '../ipc';

  const days = $derived.by(() => {
    if (!store.currentJob) return [];
    return computeViewportDays(
      store.currentJob.project_start_date,
      store.tasks,
      store.includeWeekends,
    );
  });

  type Row =
    | { kind: 'phase'; phase: Phase }
    | { kind: 'task'; task: Task; phase: Phase }
    | { kind: 'add-task'; phase: Phase };

  const rows = $derived.by((): Row[] => {
    const out: Row[] = [];
    for (const phase of store.phases) {
      out.push({ kind: 'phase', phase });
      if (!phase.collapsed) {
        for (const task of (store.tasksByPhase.get(phase.id) ?? [])) {
          out.push({ kind: 'task', task, phase });
        }
        out.push({ kind: 'add-task', phase });
      }
    }
    return out;
  });

  const ROW_H = 32;
  const CELL = 24;
  const totalHeight = $derived(rows.length * ROW_H);

  const rowIndexMap = $derived.by(() => {
    const m = new Map<number, number>();
    rows.forEach((r, i) => { if (r.kind === 'task') m.set(r.task.id, i); });
    return m;
  });
</script>

<div class="gantt" style="--cell-w: {CELL}px;">
  <DragOverlay />
  <DepCreator />
  <LeftRail />
  <div class="grid-area" style="--total-w: {days.length * CELL}px;">
    <HeaderStrip {days} />
    <div
      class="rows"
      style="height: {totalHeight}px;"
      onpointermove={(e) => {
        // Proportional math relative to .rows (the same container the highlight renders in).
        const r = (e.currentTarget as HTMLDivElement).getBoundingClientRect();
        const idx = Math.floor((e.clientX - r.left) / r.width * days.length);
        store.hoveredDayIndex = (idx >= 0 && idx < days.length) ? idx : null;
      }}
      onpointerleave={() => { store.hoveredDayIndex = null; }}
    >
      <NoWorkColumn {days} {totalHeight} />
      <WeekGridLines {days} {totalHeight} cellWidth={CELL} />
      <HoverColumn {totalHeight} cellWidth={CELL} />
      <svg
        width={days.length * CELL}
        height={totalHeight}
        class="canvas-svg"
        onclick={() => store.select(null)}
        ondblclick={async (e) => {
          const svgRect = (e.currentTarget as SVGElement).getBoundingClientRect();
          const x = e.clientX - svgRect.left;
          const y = e.clientY - svgRect.top;
          const dayIdx = Math.floor(x / CELL);
          const rowIdx = Math.floor(y / ROW_H);
          if (dayIdx < 0 || dayIdx >= days.length) return;
          if (rowIdx < 0 || rowIdx >= rows.length) return;
          const r = rows[rowIdx];
          if (r.kind === 'add-task') return;
          const phaseId = r.phase.id;
          if (!phaseId) return;
          const date = days[dayIdx].date;
          const task = await ipc.createTask({
            phase_id: phaseId, name: 'New task', start_date: date, duration_workdays: 1,
          });
          store.tasks = [...store.tasks, task];
          store.selection = { kind: 'task', id: task.id };
          await ipc.touchLastSave();
        }}
      >
        <defs>
          <marker id="arrowhead" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
            <path d="M 0 0 L 10 5 L 0 10 z" fill="var(--c-border-bold)" />
          </marker>
        </defs>
        {#each rows as r, ri (r.kind === 'phase' ? `p${r.phase.id}` : r.kind === 'task' ? `t${r.task.id}` : `a${r.phase.id}`)}
          {#if r.kind === 'phase' && r.phase.collapsed}
            <PhaseBar phase={r.phase} tasks={store.tasksByPhase.get(r.phase.id) ?? []} {days} row={ri} />
          {:else if r.kind === 'task'}
            <TaskBar task={r.task} phase={r.phase} {days} row={ri} />
          {/if}
        {/each}
        {#each store.dependencies as dep (dep.id)}
          <DependencyArrow {dep} tasks={store.tasks} rowIndex={rowIndexMap} {days} />
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
