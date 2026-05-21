<script lang="ts">
  import type { ViewportDay } from '../calendar';
  import { store } from '../store.svelte';

  let { days, totalHeight, cellWidth }:
    { days: ViewportDay[]; totalHeight: number; cellWidth: number } = $props();

  const idx = $derived(days.findIndex((d) => d.date === store.todayIso));
  const x = $derived(idx >= 0 ? idx * cellWidth : -1);

  // Friendly label for the flag (e.g. "Today · 21 May")
  const label = $derived.by(() => {
    const d = new Date(store.todayIso);
    return d.toLocaleDateString('en-GB', { day: '2-digit', month: 'short' });
  });
</script>

{#if x >= 0}
  <div class="today-line" style="left: {x}px; height: {totalHeight}px; width: {cellWidth}px;">
    <div class="flag">Today · {label}</div>
    <div class="bar"></div>
  </div>
{/if}

<style>
  .today-line {
    position: absolute;
    top: 0;
    pointer-events: none;
    z-index: 2;
  }
  .bar {
    position: absolute;
    left: 50%;
    top: 0;
    width: 2px;
    height: 100%;
    background: #DC2626;
    transform: translateX(-50%);
  }
  .flag {
    position: absolute;
    top: -22px;
    left: 50%;
    transform: translateX(-50%);
    background: #DC2626;
    color: white;
    font-size: 10px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 3px;
    white-space: nowrap;
    box-shadow: 0 1px 3px rgba(0,0,0,0.2);
  }
  .flag::after {
    content: '';
    position: absolute;
    left: 50%;
    bottom: -4px;
    transform: translateX(-50%);
    width: 0; height: 0;
    border-left: 4px solid transparent;
    border-right: 4px solid transparent;
    border-top: 4px solid #DC2626;
  }
</style>
