<script lang="ts">
  let { x, y, items, onclose }: {
    x: number; y: number;
    items: { label: string; action: () => void; danger?: boolean }[];
    onclose: () => void;
  } = $props();

  function dispatch(action: () => void) {
    action();
    onclose();
  }
</script>

<div class="backdrop" onclick={onclose} oncontextmenu={(e) => { e.preventDefault(); onclose(); }} role="presentation"></div>
<div class="menu" style="left: {x}px; top: {y}px;">
  {#each items as item}
    <button onclick={() => dispatch(item.action)} class:danger={item.danger}>{item.label}</button>
  {/each}
</div>

<style>
  .backdrop { position: fixed; inset: 0; z-index: 100; }
  .menu {
    position: fixed; z-index: 101;
    background: var(--c-panel);
    border: 1px solid var(--c-border);
    border-radius: 6px;
    box-shadow: 0 4px 16px var(--c-shadow);
    padding: 4px 0;
    min-width: 180px;
  }
  .menu button {
    display: block; width: 100%; text-align: left;
    background: transparent; border: none;
    padding: var(--sp-2) var(--sp-3);
    cursor: pointer; font-size: var(--font-size-sm);
  }
  .menu button:hover { background: var(--c-accent-fade); }
  .menu button.danger { color: #DC2626; }
</style>
