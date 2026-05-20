<script lang="ts">
  import type { Task, Phase } from '../types';
  import { store } from '../store.svelte';
  import { hitZone } from '../hit-test';

  let { task, phase, days, row }: {
    task: Task; phase: Phase; days: { date: string }[]; row: number;
  } = $props();

  const xStart = $derived(days.findIndex(d => d.date === task.start_date) * 24);
  const w = $derived(task.duration_workdays * 24);
  const y = $derived(row * 32 + 6);
  const isSelected = $derived(store.selection?.kind === 'task' && store.selection.id === task.id);
  const isDragging = $derived(store.dragState?.taskId === task.id);

  const livePreview = $derived.by(() => {
    if (!isDragging || !store.dragState) return { x: xStart, w };
    const d = store.dragState;
    if (d.zone === 'move')         return { x: xStart + d.liveDelta, w };
    if (d.zone === 'resize-end')   return { x: xStart, w: Math.max(24, w + d.liveDelta) };
    if (d.zone === 'resize-start') return { x: xStart + d.liveDelta, w: Math.max(24, w - d.liveDelta) };
    return { x: xStart, w };
  });

  function onPointerDown(e: PointerEvent) {
    e.stopPropagation();
    store.select({ kind: 'task', id: task.id });
    const rect = (e.currentTarget as Element).getBoundingClientRect();
    const relX = e.clientX - rect.left;
    const zone = hitZone({ relX, width: w });
    store.dragState = {
      taskId: task.id,
      zone,
      startX: e.clientX,
      originalStart: task.start_date,
      originalDuration: task.duration_workdays,
      liveDelta: 0,
    };
  }

  function onHandleDown(e: PointerEvent) {
    e.stopPropagation();
    store.depCreator = {
      fromTaskId: task.id,
      fromX: e.clientX,
      fromY: e.clientY,
      mouseX: e.clientX,
      mouseY: e.clientY,
      hoverTaskId: null,
    };
  }

  const showHandle = $derived(
    store.hoveredTaskId === task.id && !store.dragState && !store.depCreator,
  );
</script>

<g
  class="task-bar"
  data-zone={store.dragState?.taskId === task.id ? store.dragState.zone : null}
  onpointerdown={onPointerDown}
  onmouseenter={() => {
    store.hoveredTaskId = task.id;
    if (store.depCreator) store.depCreator.hoverTaskId = task.id;
  }}
  onmouseleave={() => {
    if (store.hoveredTaskId === task.id) store.hoveredTaskId = null;
    if (store.depCreator?.hoverTaskId === task.id) store.depCreator.hoverTaskId = null;
  }}
  role="button"
  tabindex="0"
>
  <rect
    x={livePreview.x} y={y}
    width={livePreview.w} height={20}
    rx={3}
    fill={phase.colour}
    fill-opacity={isDragging ? 0.4 : 1}
    stroke={isSelected ? 'var(--c-accent)' : 'transparent'}
    stroke-width="2"
  />
  {#if livePreview.w > 60}
    <text x={livePreview.x + 6} y={y + 14} fill="white" font-size="11">{task.name}</text>
  {/if}
  {#if showHandle}
    <circle
      cx={livePreview.x + livePreview.w}
      cy={y + 10}
      r={5}
      fill="white"
      stroke="var(--c-accent)"
      stroke-width="2"
      class="dep-handle"
      onpointerdown={onHandleDown}
    />
  {/if}
</g>

<style>
  .task-bar { cursor: grab; }
  .task-bar:active { cursor: grabbing; }
  .task-bar[data-zone="resize-start"],
  .task-bar[data-zone="resize-end"] { cursor: ew-resize; }
  .dep-handle { cursor: crosshair; }
</style>
