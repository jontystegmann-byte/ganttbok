<script lang="ts">
  import type { Dependency, Task } from '../types';
  import { addWorkdays } from '../calendar';
  import { store } from '../store.svelte';

  let { dep, tasks, rowIndex, days }: {
    dep: Dependency;
    tasks: Task[];
    rowIndex: Map<number, number>;
    days: { date: string }[];
  } = $props();

  const path = $derived.by(() => {
    const pre = tasks.find(t => t.id === dep.predecessor_id);
    const suc = tasks.find(t => t.id === dep.successor_id);
    if (!pre || !suc) return null;
    const preRow = rowIndex.get(pre.id);
    const sucRow = rowIndex.get(suc.id);
    if (preRow === undefined || sucRow === undefined) return null;

    const preEndDate = addWorkdays(pre.start_date, Math.max(0, pre.duration_workdays - 1));
    const preEndIdx  = days.findIndex(d => d.date === preEndDate);
    const sucStartIdx = days.findIndex(d => d.date === suc.start_date);
    if (preEndIdx < 0 || sucStartIdx < 0) return null;

    const x1 = (preEndIdx + 1) * 24;         // right edge of predecessor
    const y1 = preRow * 32 + 16;             // vertical centre
    const x2 = sucStartIdx * 24;             // left edge of successor
    const y2 = sucRow * 32 + 16;
    // Right-angle elbow path
    return `M ${x1} ${y1} L ${x1 + 6} ${y1} L ${x1 + 6} ${y2} L ${x2} ${y2}`;
  });

  const isLit = $derived(
    store.hoveredTaskId === dep.predecessor_id || store.hoveredTaskId === dep.successor_id
  );
</script>

{#if path}
  <path
    d={path}
    class="dep-line"
    class:lit={isLit}
    stroke={isLit ? 'var(--c-accent)' : 'var(--c-border-bold)'}
    stroke-width={isLit ? 2 : 1}
    fill="none"
    marker-end="url(#arrowhead)"
  />
{/if}
