<script lang="ts">
  import type { Phase, Task } from '../types';
  import { addWorkdays } from '../calendar';
  import { store } from '../store.svelte';

  let { phase, tasks, days, row }: {
    phase: Phase; tasks: Task[]; days: { date: string }[]; row: number;
  } = $props();

  const span = $derived.by(() => {
    if (tasks.length === 0) return null;
    const starts = tasks.map(t => t.start_date).sort();
    const ends   = tasks.map(t => addWorkdays(t.start_date, Math.max(0, t.duration_workdays - 1))).sort();
    const startIdx = days.findIndex(d => d.date === starts[0]);
    const endIdx   = days.findIndex(d => d.date === ends[ends.length - 1]);
    if (startIdx < 0 || endIdx < 0) return null;
    return { x: startIdx * 24, w: (endIdx - startIdx + 1) * 24 };
  });

  const y = $derived(row * 32 + 6);
  const isDragging = $derived(store.dragState?.taskId === -phase.id);

  const liveX = $derived.by(() => {
    if (!span) return 0;
    if (!isDragging || !store.dragState) return span.x;
    return span.x + store.dragState.liveDelta;
  });

  function onPointerDown(e: PointerEvent) {
    if (tasks.length === 0) return;
    e.stopPropagation();
    store.dragState = {
      taskId: -phase.id, // negative = phase id sentinel
      zone: 'move',
      startX: e.clientX,
      originalStart: tasks[0]?.start_date ?? '',
      originalDuration: 0,
      liveDelta: 0,
    };
  }
</script>

{#if span}
  <rect
    class="phase-bar"
    x={liveX} y={y}
    width={span.w} height={20}
    rx={3}
    fill={phase.colour}
    fill-opacity={isDragging ? 0.35 : 0.18}
    stroke={phase.colour}
    stroke-opacity="0.5"
    stroke-width="1"
    onpointerdown={onPointerDown}
  />
{/if}

<style>
  .phase-bar { cursor: grab; }
  .phase-bar:active { cursor: grabbing; }
</style>
