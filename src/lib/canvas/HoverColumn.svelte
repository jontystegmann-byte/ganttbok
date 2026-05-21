<script lang="ts">
  import { store } from '../store.svelte';
  let { totalHeight, cellWidth }: { totalHeight: number; cellWidth: number } = $props();
</script>

{#if store.hoveredDayIndex !== null}
  <div
    class="hover-col"
    style="left: {store.hoveredDayIndex * cellWidth}px; width: {cellWidth}px; height: {totalHeight}px;"
  ></div>
{/if}

<style>
  .hover-col {
    position: absolute;
    top: 0;
    background: rgba(37, 99, 235, 0.08);
    border-left: 1px solid rgba(37, 99, 235, 0.25);
    border-right: 1px solid rgba(37, 99, 235, 0.25);
    pointer-events: none;
    z-index: 3;
  }
</style>
