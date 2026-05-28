<script lang="ts">
  let { x, top, height }: { x: number; top: number; height: number } = $props();
</script>

<g class="ghost-marker" transform="translate({x}, {top})" aria-hidden="true">
  <!-- Top cap -->
  <line x1="-3" y1="0" x2="3" y2="0" stroke="#E11D2A" stroke-opacity="0.4" stroke-width="1" />
  <!-- Vertical line -->
  <line x1="0" y1="0" x2="0" y2={height} stroke="#E11D2A" stroke-opacity="0.4" stroke-width="1" />
  <!-- Bottom cap -->
  <line x1="-3" y1={height} x2="3" y2={height} stroke="#E11D2A" stroke-opacity="0.4" stroke-width="1" />
</g>
