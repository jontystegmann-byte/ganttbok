<script lang="ts">
  import type { Task, Phase } from '../types';
  import { state } from '../store.svelte';

  let { task, phase, days, row }: {
    task: Task; phase: Phase; days: { date: string }[]; row: number;
  } = $props();

  const xStart = $derived(days.findIndex(d => d.date === task.start_date) * 24);
  const w = $derived(task.duration_workdays * 24);
  const y = $derived(row * 32 + 6);   // 6px vertical padding inside row
  const isSelected = $derived(state.selection?.kind === 'task' && state.selection.id === task.id);

  function select(e: MouseEvent) {
    e.stopPropagation();
    state.select({ kind: 'task', id: task.id });
  }
</script>

<g
  class="task-bar"
  onclick={select}
  onmouseenter={() => state.hoveredTaskId = task.id}
  onmouseleave={() => state.hoveredTaskId = null}
  role="button"
  tabindex="0"
>
  <rect
    x={xStart} y={y}
    width={w} height={20}
    rx={3}
    fill={phase.colour}
    stroke={isSelected ? 'var(--c-accent)' : 'transparent'}
    stroke-width="2"
  />
  {#if w > 60}
    <text x={xStart + 6} y={y + 14} fill="white" font-size="11">{task.name}</text>
  {/if}
</g>

<style>
  .task-bar { cursor: grab; }
  .task-bar:active { cursor: grabbing; }
</style>
