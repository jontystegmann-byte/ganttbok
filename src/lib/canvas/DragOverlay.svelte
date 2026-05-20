<script lang="ts">
  import { onMount } from 'svelte';
  import { state } from '../store.svelte';
  import { magneticSnap } from '../snap';
  import { addWorkdays } from '../calendar';
  import * as ipc from '../ipc';

  const CELL = 24;

  function onPointerMove(e: PointerEvent) {
    if (!state.dragState) return;
    const rawDelta = e.clientX - state.dragState.startX;
    const snapped = magneticSnap({ pxDelta: rawDelta, cellW: CELL });
    state.dragState.liveDelta = snapped;
  }

  async function onPointerUp(_e: PointerEvent) {
    const d = state.dragState;
    if (!d) return;
    const deltaWorkdays = Math.round(d.liveDelta / CELL);
    state.dragState = null;
    if (deltaWorkdays === 0) return;
    if (!state.currentJob) return;

    const task = state.tasks.find(t => t.id === d.taskId);
    if (!task) return;

    if (d.zone === 'move') {
      const newStart = addWorkdays(d.originalStart, deltaWorkdays);
      const result = await ipc.dragTask({
        job_id: state.currentJob.id,
        task_id: d.taskId,
        new_start_date: newStart,
      });
      state.applyDragResult(result.updated_tasks);
    } else if (d.zone === 'resize-end') {
      const newDur = Math.max(1, d.originalDuration + deltaWorkdays);
      const updated = { ...task, duration_workdays: newDur };
      await ipc.updateTask($state.snapshot(updated));
      state.tasks = state.tasks.map(t => t.id === task.id ? updated : t);
    } else if (d.zone === 'resize-start') {
      const newStart = addWorkdays(d.originalStart, deltaWorkdays);
      const newDur = Math.max(1, d.originalDuration - deltaWorkdays);
      const updated = { ...task, start_date: newStart, duration_workdays: newDur };
      await ipc.updateTask($state.snapshot(updated));
      state.tasks = state.tasks.map(t => t.id === task.id ? updated : t);
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
