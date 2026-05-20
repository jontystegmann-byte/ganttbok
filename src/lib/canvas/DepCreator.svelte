<script lang="ts">
  import { onMount } from 'svelte';
  import { store } from '../store.svelte';
  import * as ipc from '../ipc';
  import { toast } from '../toast.svelte';

  function onMove(e: PointerEvent) {
    if (!store.depCreator) return;
    store.depCreator.mouseX = e.clientX;
    store.depCreator.mouseY = e.clientY;
  }
  async function onUp(_e: PointerEvent) {
    const d = store.depCreator;
    if (!d) return;
    store.depCreator = null;
    if (d.hoverTaskId === null || d.hoverTaskId === d.fromTaskId) return;
    try {
      const dep = await ipc.createDependency({
        predecessor_id: d.fromTaskId,
        successor_id: d.hoverTaskId,
        lag_days: 0,
      });
      store.dependencies = [...store.dependencies, dep];
      store.recordHistory();
      await ipc.touchLastSave();
    } catch (e) {
      const msg = String((e as { message?: string }).message ?? e);
      if (msg.toLowerCase().includes('cycle')) {
        toast.show('error', "Can't create that — it would make a circular chain.");
      } else {
        toast.show('error', `Couldn't add dependency: ${msg}`);
      }
    }
  }

  onMount(() => {
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    return () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
    };
  });
</script>

{#if store.depCreator}
  <svg class="dep-creator-overlay">
    <line
      x1={store.depCreator.fromX} y1={store.depCreator.fromY}
      x2={store.depCreator.mouseX} y2={store.depCreator.mouseY}
      stroke="var(--c-accent)"
      stroke-width="2"
      stroke-dasharray="4 2"
    />
  </svg>
{/if}

<style>
  .dep-creator-overlay {
    position: fixed; inset: 0;
    width: 100vw; height: 100vh;
    pointer-events: none;
    z-index: 50;
  }
</style>
