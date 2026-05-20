<script lang="ts">
  import { state } from '../store.svelte';
  import HeaderStrip from './HeaderStrip.svelte';
  import LeftRail from './LeftRail.svelte';
  import NoWorkColumn from './NoWorkColumn.svelte';
  import { computeViewportDays } from '../calendar';

  const days = $derived.by(() => {
    if (!state.currentJob) return [];
    return computeViewportDays(
      state.currentJob.project_start_date,
      state.tasks,
    );
  });

  // Cell width pulled from CSS variable so print can override.
  const CELL = 24;
</script>

<div class="gantt" style="--cell-w: {CELL}px;">
  <LeftRail />
  <div class="grid-area" style="--total-w: {days.length * CELL}px;">
    <HeaderStrip {days} />
    <div class="rows">
      <NoWorkColumn {days} totalHeight={320} />
    </div>
  </div>
</div>

<style>
  .gantt {
    display: grid;
    grid-template-columns: var(--left-rail-width) 1fr;
    height: 100%;
    overflow: hidden;
  }
  .grid-area {
    position: relative;
    overflow-x: auto;
    overflow-y: auto;
  }
  .rows {
    position: relative;
    width: var(--total-w);
  }
</style>
