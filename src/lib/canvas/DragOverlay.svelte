<script lang="ts">
  import { onMount } from 'svelte';
  import { store } from '../store.svelte';
  import { addWorkdays } from '../calendar';
  import type { ViewportDay } from '../calendar';
  import { computeGhostDate } from './drag-physics';
  import * as ipc from '../ipc';

  let { cellW, days }: { cellW: number; days: ViewportDay[] } = $props();

  /** Effective no-work set: holidays only block when the per-job flag is on. */
  function effectiveNoWorkSet(): Set<string> {
    if (!store.currentJob?.holidays_block_work) return new Set<string>();
    return new Set(store.noWorkDays.map(n => n.date));
  }

  /** Count workdays between two ISO dates (signed; respects no-work + weekend flag). */
  function workdaysBetween(startIso: string, endIso: string, noWork: Set<string>, incWknd: boolean): number {
    if (startIso === endIso) return 0;
    const forward = endIso > startIso;
    let cur = startIso;
    let count = 0;
    let guard = 0;
    while (cur !== endIso && guard < 10_000) {
      cur = addWorkdays(cur, forward ? 1 : -1, incWknd);
      while (noWork.has(cur) && cur !== endIso) {
        cur = addWorkdays(cur, forward ? 1 : -1, incWknd);
      }
      count += forward ? 1 : -1;
      guard++;
    }
    return count;
  }

  function ghostFor(originalStart: string, liveDelta: number): string {
    return computeGhostDate({
      originalStart,
      pxDelta: liveDelta,
      cellW,
      days,
      noWorkSet: effectiveNoWorkSet(),
      includeWeekends: store.includeWeekends,
    });
  }

  function onPointerMove(e: PointerEvent) {
    if (!store.dragState) return;
    // Free 1:1 pixel tracking — no snap during preview.
    store.dragState.liveDelta = e.clientX - store.dragState.startX;
  }

  async function onPointerUp(_e: PointerEvent) {
    const d = store.dragState;
    if (!d) return;
    store.dragState = null;
    if (!store.currentJob) return;
    const noWork = effectiveNoWorkSet();

    // Phase drag: shift every task in the phase by the same workday delta.
    if (d.taskId < 0) {
      const phaseId = -d.taskId;
      const phaseTasks = store.tasksByPhase.get(phaseId) ?? [];
      if (phaseTasks.length === 0) return;
      const ghost = ghostFor(d.originalStart, d.liveDelta);
      if (ghost === d.originalStart) return;
      const wkDelta = workdaysBetween(d.originalStart, ghost, noWork, store.includeWeekends);
      for (const t of phaseTasks) {
        const newStart = addWorkdays(t.start_date, wkDelta, store.includeWeekends);
        await ipc.dragTask({
          job_id: store.currentJob.id,
          task_id: t.id,
          new_start_date: newStart,
        });
      }
      await ipc.touchLastSave();
      await store.openJob(store.currentJob.id);
      return;
    }

    const task = store.tasks.find(t => t.id === d.taskId);
    if (!task) return;

    if (d.zone === 'move') {
      const ghost = ghostFor(d.originalStart, d.liveDelta);
      if (ghost === d.originalStart) return;
      const result = await ipc.dragTask({
        job_id: store.currentJob.id,
        task_id: d.taskId,
        new_start_date: ghost,
      });
      store.applyDragResult(result.updated_tasks);
    } else if (d.zone === 'resize-end') {
      // Treat the bar's end as the drag origin: snap the end column, then derive new duration.
      const originalEnd = addWorkdays(d.originalStart, Math.max(0, d.originalDuration - 1), store.includeWeekends);
      const ghostEnd = ghostFor(originalEnd, d.liveDelta);
      if (ghostEnd === originalEnd) return;
      const wkDelta = workdaysBetween(originalEnd, ghostEnd, noWork, store.includeWeekends);
      const newDur = Math.max(1, d.originalDuration + wkDelta);
      const updated = { ...task, duration_workdays: newDur };
      await ipc.updateTask($state.snapshot(updated));
      store.tasks = store.tasks.map(t => t.id === task.id ? updated : t);
    } else if (d.zone === 'resize-start') {
      const ghostStart = ghostFor(d.originalStart, d.liveDelta);
      if (ghostStart === d.originalStart) return;
      const wkDelta = workdaysBetween(d.originalStart, ghostStart, noWork, store.includeWeekends);
      const newDur = Math.max(1, d.originalDuration - wkDelta);
      const updated = { ...task, start_date: ghostStart, duration_workdays: newDur };
      await ipc.updateTask($state.snapshot(updated));
      store.tasks = store.tasks.map(t => t.id === task.id ? updated : t);
    }
    await ipc.touchLastSave();
  }

  onMount(() => {
    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', onPointerUp);
    return () => {
      window.removeEventListener('pointermove', onPointerMove);
      window.removeEventListener('pointerup', onPointerUp);
    };
  });
</script>
