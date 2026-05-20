<script lang="ts">
  import { store } from '../store.svelte';

  let name = $state('');
  let client = $state('');
  let address = $state('');
  let startDate = $state(new Date().toISOString().slice(0, 10));
  let submitting = $state(false);

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!name.trim()) return;
    submitting = true;
    try {
      await store.createJob({
        name: name.trim(),
        client: client.trim() || null,
        address: address.trim() || null,
        project_start_date: startDate,
      });
    } finally {
      submitting = false;
    }
  }

  function cancel() { store.showNewJobModal = false; }
</script>

<div class="backdrop" onclick={cancel} role="presentation"></div>
<form class="modal" onsubmit={submit}>
  <h2>New job</h2>
  <label>Name<input bind:value={name} autofocus required /></label>
  <label>Client<input bind:value={client} placeholder="optional" /></label>
  <label>Address<input bind:value={address} placeholder="optional" /></label>
  <label>Project start<input type="date" bind:value={startDate} required /></label>
  <div class="actions">
    <button type="button" onclick={cancel}>Cancel</button>
    <button type="submit" class="primary" disabled={submitting || !name.trim()}>Create</button>
  </div>
</form>

<style>
  .backdrop {
    position: fixed; inset: 0; background: rgba(0, 0, 0, 0.3);
    z-index: 10;
  }
  .modal {
    position: fixed; top: 50%; left: 50%;
    transform: translate(-50%, -50%);
    background: var(--c-panel); border-radius: 8px;
    padding: var(--sp-6); box-shadow: 0 16px 48px var(--c-shadow);
    z-index: 11; min-width: 360px;
    display: flex; flex-direction: column; gap: var(--sp-3);
  }
  .modal h2 { margin: 0 0 var(--sp-2); font-size: var(--font-size-lg); }
  label { display: flex; flex-direction: column; gap: var(--sp-1); font-size: var(--font-size-sm); color: var(--c-text-muted); }
  input { padding: var(--sp-2); border: 1px solid var(--c-border); border-radius: 4px; }
  .actions { display: flex; justify-content: flex-end; gap: var(--sp-2); margin-top: var(--sp-2); }
  .actions button { padding: var(--sp-2) var(--sp-3); border: 1px solid var(--c-border); background: var(--c-bg); border-radius: 4px; cursor: pointer; }
  .actions .primary { background: var(--c-accent); color: white; border-color: var(--c-accent); }
  .actions .primary:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
