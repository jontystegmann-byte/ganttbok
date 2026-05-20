<script lang="ts">
  import { store } from '../store.svelte';
  import * as ipc from '../ipc';
  import type { Phase } from '../types';

  let { phaseId }: { phaseId: number } = $props();
  const phase = $derived(store.phases.find(p => p.id === phaseId));

  let name = $state('');
  let colour = $state('#3B82F6');
  let confirmingDelete = $state(false);

  $effect(() => {
    if (phase) {
      name = phase.name;
      colour = phase.colour;
      confirmingDelete = false;
    }
  });

  async function save() {
    if (!phase) return;
    const updated: Phase = { ...phase, name: name.trim() || phase.name, colour };
    await ipc.updatePhase($state.snapshot(updated));
    store.phases = store.phases.map(p => p.id === updated.id ? updated : p);
    await ipc.touchLastSave();
  }

  async function del() {
    if (!phase) return;
    if (!confirmingDelete) {
      confirmingDelete = true;
      setTimeout(() => { confirmingDelete = false; }, 3000);
      return;
    }
    await ipc.deletePhase(phase.id);
    store.phases = store.phases.filter(p => p.id !== phase.id);
    store.tasks  = store.tasks.filter(t => t.phase_id !== phase.id);
    store.select(null);
    await ipc.touchLastSave();
  }
</script>

{#if phase}
  <div class="phase-details">
    <h2>{phase.name}</h2>
    <label>Name<input bind:value={name} onblur={save} /></label>
    <label>Colour<input type="color" bind:value={colour} onblur={save} /></label>
    <button class="danger" onclick={del}>{confirmingDelete ? 'Click again — deletes ALL tasks in this phase' : 'Delete phase'}</button>
  </div>
{/if}

<style>
  .phase-details { display: flex; flex-direction: column; gap: var(--sp-3); padding: var(--sp-4); }
  h2 { font-size: var(--font-size-lg); margin: 0; }
  label { display: flex; flex-direction: column; gap: var(--sp-1); font-size: var(--font-size-sm); color: var(--c-text-muted); }
  input { padding: var(--sp-2); border: 1px solid var(--c-border); border-radius: 4px; font: inherit; color: var(--c-text); background: var(--c-bg); }
  .danger { margin-top: var(--sp-3); padding: var(--sp-2); background: transparent; border: 1px solid #DC2626; color: #DC2626; border-radius: 4px; cursor: pointer; }
  .danger:hover { background: #FEE2E2; }
</style>
