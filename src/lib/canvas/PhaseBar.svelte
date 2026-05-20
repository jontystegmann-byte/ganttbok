<script lang="ts">
  import type { Phase, Task } from '../types';
  import { addWorkdays } from '../calendar';

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
</script>

{#if span}
  <rect
    class="phase-bar"
    x={span.x} y={y}
    width={span.w} height={20}
    rx={3}
    fill={phase.colour}
    fill-opacity="0.18"
    stroke={phase.colour}
    stroke-opacity="0.5"
    stroke-width="1"
  />
{/if}

<style>
  .phase-bar { pointer-events: none; }
</style>
