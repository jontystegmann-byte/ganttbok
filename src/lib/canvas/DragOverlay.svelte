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

    const newStart = addWorkdays(d.originalStart, deltaWorkdays);
    const result = await ipc.dragTask({
      job_id: state.currentJob.id,
      task_id: d.taskId,
      new_start_date: newStart,
    });
    state.applyDragResult(result.updated_tasks);
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
