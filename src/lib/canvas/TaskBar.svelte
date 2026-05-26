<script lang="ts">
  import type { Task, Phase, TaskStatus } from '../types';
  import { store } from '../store.svelte';
  import { hitZone, EDGE_PX } from '../hit-test';

  // Status visual map. Literal hex codes — SVG `fill` attribute does not reliably
  // resolve CSS var() references in this WebKit, so we paint the values directly.
  function statusFill(s: TaskStatus | undefined | null): string {
    switch (s) {
      case 'late':     return '#E11D2A';
      case 'done':     return '#9CA3AF';
      case 'on_track':
      default:         return '#10B981';
    }
  }
  function statusTextColor(_s: TaskStatus | undefined | null): string {
    return 'white';
  }

  let { task, phase, days, row }: {
    task: Task; phase: Phase; days: { date: string }[]; row: number;
  } = $props();

  import { occupiedWorkdays, groupConsecutive } from '../calendar';

  const skipNoWork = $derived(store.currentJob?.holidays_block_work ?? true);
  const noWorkSet = $derived.by(() => new Set(store.noWorkDays.map((n) => n.date)));

  const occupied = $derived(occupiedWorkdays(task.start_date, task.duration_workdays, noWorkSet, skipNoWork, store.includeWeekends));
  /** Task is "past" once its very last occupied day is strictly before today. */
  const isPast = $derived(occupied.length > 0 && occupied[occupied.length - 1] < store.todayIso);
  const dayIndexOf = $derived.by(() => {
    const m = new Map<string, number>();
    days.forEach((d, i) => m.set(d.date, i));
    return m;
  });
  const occupiedIndices = $derived(occupied.map((iso) => dayIndexOf.get(iso) ?? -1).filter((i) => i >= 0));
  const segments = $derived(groupConsecutive(occupiedIndices));

  const xStart = $derived(occupiedIndices[0] !== undefined ? occupiedIndices[0] * 24 : 0);
  const w = $derived(task.duration_workdays * 24);
  const y = $derived(row * 32 + 6);
  const totalSpanW = $derived(
    segments.length > 0 ? (segments[segments.length - 1].start + segments[segments.length - 1].len) * 24 - xStart : w,
  );
  const isSelected = $derived(store.selection?.kind === 'task' && store.selection.id === task.id);
  const isDragging = $derived(store.dragState?.taskId === task.id);

  const livePreview = $derived.by(() => {
    if (!isDragging || !store.dragState) return { x: xStart, w: totalSpanW };
    const d = store.dragState;
    if (d.zone === 'move')         return { x: xStart + d.liveDelta, w: totalSpanW };
    if (d.zone === 'resize-end')   return { x: xStart, w: Math.max(24, totalSpanW + d.liveDelta) };
    if (d.zone === 'resize-start') return { x: xStart + d.liveDelta, w: Math.max(24, totalSpanW - d.liveDelta) };
    return { x: xStart, w: totalSpanW };
  });

  /** Visual edge width — wider than EDGE_PX hit zone so the handle is obvious. */
  const edgeW = $derived(Math.min(Math.max(livePreview.w * 0.15, EDGE_PX), 14));

  function startDrag(zone: 'move' | 'resize-start' | 'resize-end', e: PointerEvent) {
    e.stopPropagation();
    store.select({ kind: 'task', id: task.id });
    store.dragState = {
      taskId: task.id,
      zone,
      startX: e.clientX,
      originalStart: task.start_date,
      originalDuration: task.duration_workdays,
      liveDelta: 0,
    };
  }

  function onBodyDown(e: PointerEvent) {
    // Use bar-body's own rect to pick the zone (move vs edge if the body extends under handles).
    const r = (e.currentTarget as Element).getBoundingClientRect();
    startDrag(hitZone({ relX: e.clientX - r.left, width: r.width }), e);
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
  {#if isDragging}
    <!-- Single live-preview rect during drag — segmentation hidden while moving for clarity. -->
    <rect
      x={livePreview.x} y={y}
      width={livePreview.w} height={20}
      rx={3}
      fill={statusFill(task.status)}
      fill-opacity={0.4}
      stroke={isSelected ? '#E11D2A' : 'transparent'}
      stroke-width="2"
      class="bar-body zone-move"
      onpointerdown={onBodyDown}
    />
  {:else}
    {#each segments as seg, si}
      <rect
        x={seg.start * 24} y={y}
        width={seg.len * 24} height={20}
        rx={3}
        fill={statusFill(task.status)}
        fill-opacity={isPast ? 0.25 : 1}
        stroke={isSelected && si === 0 ? '#E11D2A' : 'transparent'}
        stroke-width={isSelected && si === 0 ? 2 : 0}
        class="bar-body zone-move"
        class:past={isPast}
        onpointerdown={onBodyDown}
      />
    {/each}
  {/if}

  <!-- Resize edge zones: visible tinted overlays so handles are obvious -->
  <rect
    x={livePreview.x} y={y}
    width={edgeW} height={20}
    rx={3}
    class="edge-handle zone-resize-start"
    class:visible={store.hoveredTaskId === task.id || isSelected}
    onpointerdown={(e) => startDrag('resize-start', e)}
  />
  <rect
    x={livePreview.x + livePreview.w - edgeW} y={y}
    width={edgeW} height={20}
    rx={3}
    class="edge-handle zone-resize-end"
    class:visible={store.hoveredTaskId === task.id || isSelected}
    onpointerdown={(e) => startDrag('resize-end', e)}
  />

  <!-- Invisible hover-extension rect: covers gap to the dep port so port stays visible across the gap. -->
  <rect
    x={livePreview.x + livePreview.w}
    y={y - 4}
    width={DEP_OFFSET + DEP_R * 2 + 4}
    height={28}
    fill="transparent"
    pointer-events="all"
  />

  {#if livePreview.w > 60}
    <text
      x={livePreview.x + 6} y={y + 14}
      fill={isPast ? 'rgba(0,0,0,0.4)' : statusTextColor(task.status)}
      font-size="11" pointer-events="none"
    >{task.name}</text>
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
