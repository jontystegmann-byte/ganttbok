<script lang="ts">
  import type { ViewportDay } from '../calendar';
  let { days }: { days: ViewportDay[] } = $props();
</script>

<div class="header-strip" style="--total-w: {days.length * 24}px;">
  {#each days as d (d.date)}
    <div class="cell" class:week-start={d.weekday === 'M'}>
      {#if d.weekday === 'M'}
        <div class="week-num">Week {d.projectWeekNumber}</div>
      {/if}
      <div class="day-letter">{d.weekday}</div>
      {#if d.weekday === 'M'}
        <div class="day-number">{d.dayOfMonth}</div>
      {/if}
    </div>
  {/each}
</div>

<style>
  .header-strip {
    display: flex;
    width: var(--total-w);
    height: var(--header-height);
    border-bottom: 1px solid var(--c-border-bold);
    background: var(--c-panel);
    position: sticky; top: 0; z-index: 2;
  }
  .cell {
    width: var(--cell-w, 24px);
    border-right: 1px solid var(--c-border);
    display: flex; flex-direction: column;
    align-items: center; justify-content: flex-end;
    padding-bottom: 4px;
    position: relative;
  }
  .cell.week-start { border-right-color: var(--c-border); border-left: 2px solid var(--c-border-bold); }
  .week-num {
    position: absolute; top: 4px; left: 0;
    font-size: var(--font-size-xs); color: var(--c-text-muted);
    text-transform: uppercase; letter-spacing: 0.04em;
    white-space: nowrap;
    padding-left: 4px;
  }
  .day-letter { font-size: var(--font-size-xs); color: var(--c-text-muted); }
  .day-number { font-size: var(--font-size-xs); font-weight: 600; color: var(--c-accent); }
</style>
