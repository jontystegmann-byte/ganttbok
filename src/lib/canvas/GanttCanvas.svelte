<script lang="ts">
  import { store } from '../store.svelte';
  import HeaderStrip from './HeaderStrip.svelte';
  import LeftRail from './LeftRail.svelte';
  import JobSwitcher from '../sidebar/JobSwitcher.svelte';
  import NoWorkColumn from './NoWorkColumn.svelte';
  import WeekGridLines from './WeekGridLines.svelte';
  import HoverColumn from './HoverColumn.svelte';
  import TodayLine from './TodayLine.svelte';
  import TaskBar from './TaskBar.svelte';
  import PhaseBar from './PhaseBar.svelte';
  import DependencyArrow from './DependencyArrow.svelte';
  import DragOverlay from './DragOverlay.svelte';
  import DepCreator from './DepCreator.svelte';
  import GhostMarker from './GhostMarker.svelte';
  import { computeViewportDays, addWorkdays } from '../calendar';
  import { computeGhostDate } from './drag-physics';
  import { dateToPx } from './timeline';
  import type { Phase, Task } from '../types';
  import * as ipc from '../ipc';

  function computeOriginalEnd(start: string, duration: number): string {
    // Last occupied workday of a duration-N task = start + (N-1) workdays.
    return addWorkdays(start, Math.max(0, duration - 1), store.includeWeekends);
  }

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
  const cellW = 24;
  const totalHeight = $derived(rows.length * ROW_H);

  const rowIndexMap = $derived.by(() => {
    const m = new Map<number, number>();
    rows.forEach((r, i) => { if (r.kind === 'task') m.set(r.task.id, i); });
    return m;
  });

  // Phase dividers: one bold horizontal line at every phase boundary
  // (top of each phase + bottom of the very last phase).
  const phaseDividerYs = $derived.by((): number[] => {
    if (rows.length === 0) return [];
    const ys: number[] = [0]; // top of the first phase
    for (let i = 1; i < rows.length; i++) {
      if (rows[i].phase.id !== rows[i - 1].phase.id) {
        ys.push(i * ROW_H);
      }
    }
    ys.push(rows.length * ROW_H); // bottom of the last phase
    return ys;
  });

  /** Effective no-work set when dragging: respects the per-job holidays_block_work flag. */
  const dragNoWorkSet = $derived.by(() => {
    if (!store.currentJob?.holidays_block_work) return new Set<string>();
    return new Set(store.noWorkDays.map(n => n.date));
  });

  /** Row index of the bar being dragged (handles taskId < 0 = phase sentinel). */
  function rowIndexForDrag(taskId: number): number | undefined {
    if (taskId >= 0) return rowIndexMap.get(taskId);
    const phaseId = -taskId;
    const idx = rows.findIndex(r => r.kind === 'phase' && r.phase.id === phaseId);
    return idx >= 0 ? idx : undefined;
  }

  /** Ghost geometry derived from the live drag state. Returns null when nothing to draw. */
  const ghostGeom = $derived.by((): { x: number; top: number; height: number } | null => {
    const d = store.dragState;
    if (!d) return null;

    let originRef: string;
    if (d.zone === 'resize-end') {
      originRef = computeOriginalEnd(d.originalStart, d.originalDuration);
    } else {
      originRef = d.originalStart;
    }

    const ghost = computeGhostDate({
      originalStart: originRef,
      pxDelta: d.liveDelta,
      cellW,
      days,
      noWorkSet: dragNoWorkSet,
      includeWeekends: store.includeWeekends,
    });
    if (ghost === originRef) return null; // hide when commit delta is 0

    const x = dateToPx(ghost, cellW, days);
    if (x < 0) return null;

    const rowIdx = rowIndexForDrag(d.taskId);
    if (rowIdx === undefined || rowIdx < 0) return null;
    return { x, top: rowIdx * ROW_H, height: ROW_H };
  });

</script>

<div class="gantt" style="--cell-w: {cellW}px; --total-w: {days.length * cellW}px;">
  <DragOverlay {cellW} {days} />
  <DepCreator />
  <div class="corner"><JobSwitcher /></div>
  <div class="header-row"><HeaderStrip {days} /></div>
  <div class="rail-col"><LeftRail /></div>
  <div class="time-col">
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
      <WeekGridLines {days} {totalHeight} cellWidth={cellW} />
      <HoverColumn {totalHeight} cellWidth={cellW} />
      <TodayLine {days} {totalHeight} cellWidth={cellW} />
      <svg
        width={days.length * cellW}
        height={totalHeight}
        class="canvas-svg"
        onclick={() => store.select(null)}
        ondblclick={async (e) => {
          const svgRect = (e.currentTarget as SVGElement).getBoundingClientRect();
          const x = e.clientX - svgRect.left;
          const y = e.clientY - svgRect.top;
          const dayIdx = Math.floor(x / cellW);
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
        <!-- Phase dividers — drawn last so they sit on top of everything else -->
        {#each phaseDividerYs as dy}
          <line
            x1="0" y1={dy}
            x2={days.length * cellW} y2={dy}
            stroke="#4B5563" stroke-width="1"
            pointer-events="none"
          />
        {/each}
        {#if ghostGeom}
          <GhostMarker x={ghostGeom.x} top={ghostGeom.top} height={ghostGeom.height} />
        {/if}
      </svg>
    </div>
  </div>
</div>

<style>
  .gantt {
    display: grid;
    grid-template-columns: var(--left-rail-width) max-content;
    grid-template-rows: var(--header-height) max-content;
    height: 100%;
    overflow: auto;
    isolation: isolate; /* contain the sticky elements' z-indexes so they don't beat the AppHeader / InboxPanel */
  }
  .corner {
    grid-row: 1; grid-column: 1;
    position: sticky;
    top: 0; left: 0;
    z-index: 30;
    background: var(--c-panel);
    border-right: 1px solid var(--c-border);
    border-bottom: 1px solid var(--c-border-bold);
    display: flex;
    align-items: center;
    padding: 0 var(--sp-2);
  }
  .header-row {
    grid-row: 1; grid-column: 2;
    position: sticky;
    top: 0;
    z-index: 20;
    background: var(--c-panel);
  }
  .rail-col {
    grid-row: 2; grid-column: 1;
    position: sticky;
    left: 0;
    z-index: 10;
    background: var(--c-panel);
    border-right: 1px solid var(--c-border);
  }
  .time-col {
    grid-row: 2; grid-column: 2;
    position: relative;
  }
  .rows {
    position: relative;
    width: var(--total-w);
  }
  .canvas-svg { display: block; }
</style>
