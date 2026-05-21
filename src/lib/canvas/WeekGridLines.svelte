<script lang="ts">
  import type { ViewportDay } from '../calendar';
  let { days, totalHeight, cellWidth }: { days: ViewportDay[]; totalHeight: number; cellWidth: number } = $props();

  const lines = $derived.by(() => {
    const out: number[] = [];
    for (let i = 0; i < days.length; i++) {
      if (i === 0) continue;
      if (days[i].projectWeekNumber !== days[i - 1].projectWeekNumber) {
        out.push(i * cellWidth);
      }
    }
    return out;
  });
</script>

{#each lines as x}
  <div class="week-line" style="left: {x}px; height: {totalHeight}px;"></div>
{/each}

<style>
  .week-line {
    position: absolute;
    top: 0;
    width: 1px;
    background: var(--c-border);
    pointer-events: none;
    z-index: 1;
  }
</style>
