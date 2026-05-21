<script lang="ts">
  import type { Task, Phase } from '../types';
  import { store } from '../store.svelte';
  import { hitZone, EDGE_PX } from '../hit-test';

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

  /** Visual edge width — wider than EDGE_PX hit zone so the handle is obvious. */
  const edgeW = $derived(Math.min(Math.max(livePreview.w * 0.15, EDGE_PX), 14));

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

  function onDepPortDown(e: PointerEvent) {
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

  const showDepPort = $derived(
    (store.hoveredTaskId === task.id || isSelected) && !store.dragState && !store.depCreator,
  );

  const DEP_OFFSET = 10;
  const DEP_R = 6;
</script>

<g
  class="task-bar"
  data-zone={store.dragState?.taskId === task.id ? store.dragState.zone : null}
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
  <!-- Main bar body -->
  <rect
    x={livePreview.x} y={y}
    width={livePreview.w} height={20}
    rx={3}
    fill={phase.colour}
    fill-opacity={isDragging ? 0.4 : 1}
    stroke={isSelected ? 'var(--c-accent)' : 'transparent'}
    stroke-width="2"
    class="bar-body zone-move"
    onpointerdown={onPointerDown}
  />

  <!-- Resize edge zones: visible tinted overlays so handles are obvious -->
  <rect
    x={livePreview.x} y={y}
    width={edgeW} height={20}
    rx={3}
    class="edge-handle zone-resize-start"
    class:visible={store.hoveredTaskId === task.id || isSelected}
    onpointerdown={onPointerDown}
  />
  <rect
    x={livePreview.x + livePreview.w - edgeW} y={y}
    width={edgeW} height={20}
    rx={3}
    class="edge-handle zone-resize-end"
    class:visible={store.hoveredTaskId === task.id || isSelected}
    onpointerdown={onPointerDown}
  />

  {#if livePreview.w > 60}
    <text x={livePreview.x + 6} y={y + 14} fill="white" font-size="11" pointer-events="none">{task.name}</text>
  {/if}

  <!-- Dep creation port: clearly separated, outside the right edge -->
  {#if showDepPort}
    <line
      x1={livePreview.x + livePreview.w}
      y1={y + 10}
      x2={livePreview.x + livePreview.w + DEP_OFFSET - DEP_R}
      y2={y + 10}
      stroke="var(--c-accent)"
      stroke-width="2"
      pointer-events="none"
    />
    <circle
      cx={livePreview.x + livePreview.w + DEP_OFFSET}
      cy={y + 10}
      r={DEP_R}
      fill="var(--c-accent)"
      stroke="white"
      stroke-width="2"
      class="dep-port"
      onpointerdown={onDepPortDown}
    />
  {/if}
</g>

<style>
  .task-bar { cursor: grab; }
  .task-bar:active { cursor: grabbing; }
  .bar-body { cursor: grab; }
  .bar-body:active { cursor: grabbing; }

  .edge-handle {
    fill: white;
    fill-opacity: 0;
    cursor: ew-resize;
    transition: fill-opacity 120ms;
  }
  .edge-handle.visible { fill-opacity: 0.25; }
  .edge-handle:hover { fill-opacity: 0.5; }

  .task-bar[data-zone="resize-start"],
  .task-bar[data-zone="resize-end"] { cursor: ew-resize; }

  .dep-port {
    cursor: crosshair;
    filter: drop-shadow(0 1px 2px rgba(0,0,0,0.25));
    transition: r 100ms;
  }
  .dep-port:hover { r: 8; }
</style>
