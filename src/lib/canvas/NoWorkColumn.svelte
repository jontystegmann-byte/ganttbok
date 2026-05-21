<script lang="ts">
  import type { ViewportDay } from '../calendar';
  import { store } from '../store.svelte';
  let { days, totalHeight }: { days: ViewportDay[]; totalHeight: number } = $props();

  const noWorkByDate = $derived.by(() => {
    const m = new Map<string, string>();
    const allowHolidays = store.currentJob?.holidays_block_work ?? true;
    for (const n of store.noWorkDays) {
      if (!allowHolidays && n.source === 'sa_public_holiday') continue;
      m.set(n.date, n.reason);
    }
    return m;
  });
</script>

{#each days as d, i (d.date)}
  {#if noWorkByDate.has(d.date)}
    <div
      class="no-work"
      style="left: {i * 24}px; width: 24px; height: {totalHeight}px;"
      title={noWorkByDate.get(d.date)}
    >
      <div class="label">{noWorkByDate.get(d.date)}</div>
    </div>
  {/if}
{/each}

<style>
  .no-work {
    position: absolute; top: 0; z-index: 0;
    background-image: repeating-linear-gradient(
      45deg,
      var(--c-no-work),
      var(--c-no-work) 4px,
      transparent 4px,
      transparent 8px
    );
    pointer-events: none;
  }
  .label {
    position: absolute;
    top: 8px;
    left: 0; right: 0;
    text-align: center;
    writing-mode: vertical-rl;
    font-size: 9px;
    color: var(--c-no-work-text);
    white-space: nowrap;
    pointer-events: auto;
  }
</style>
