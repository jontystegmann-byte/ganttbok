<script lang="ts">
  import { store } from '../store.svelte';

  function cancel() { store.showPrintOptions = false; }

  function print() {
    document.body.classList.add('print-scaling-' + store.printScaling);
    if (store.printShowNotes) document.body.classList.add('print-with-notes');
    store.showPrintOptions = false;
    setTimeout(() => {
      window.print();
      setTimeout(() => {
        document.body.classList.remove('print-scaling-fit', 'print-scaling-multi', 'print-with-notes');
      }, 1000);
    }, 50);
  }
</script>

<div class="backdrop" onclick={cancel} role="presentation"></div>
<div class="modal">
  <h2>Print Plan</h2>
  <label>Page size
    <select disabled><option>A3 landscape</option></select>
  </label>
  <fieldset>
    <legend>Scaling</legend>
    <label><input type="radio" bind:group={store.printScaling} value="fit" /> Fit to page</label>
    <label><input type="radio" bind:group={store.printScaling} value="multi" /> Multi-page</label>
  </fieldset>
  <label class="check-row"><input type="checkbox" bind:checked={store.printShowNotes} /> Show notes</label>
  <div class="actions">
    <button onclick={cancel}>Cancel</button>
    <button class="primary" onclick={print}>Print →</button>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.3); z-index: 10; }
  .modal {
    position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%);
    background: var(--c-panel); border-radius: 8px; padding: var(--sp-6);
    box-shadow: 0 16px 48px var(--c-shadow); z-index: 11; min-width: 360px;
    display: flex; flex-direction: column; gap: var(--sp-3);
  }
  h2 { margin: 0 0 var(--sp-2); font-size: var(--font-size-lg); }
  label { display: flex; flex-direction: column; gap: var(--sp-1); font-size: var(--font-size-sm); color: var(--c-text-muted); }
  fieldset { border: 1px solid var(--c-border); border-radius: 4px; padding: var(--sp-2); }
  fieldset label { flex-direction: row; align-items: center; gap: var(--sp-2); color: var(--c-text); }
  .check-row { flex-direction: row; align-items: center; gap: var(--sp-2); color: var(--c-text); }
  select { padding: var(--sp-2); border: 1px solid var(--c-border); border-radius: 4px; }
  .actions { display: flex; justify-content: flex-end; gap: var(--sp-2); margin-top: var(--sp-2); }
  .actions button { padding: var(--sp-2) var(--sp-3); border: 1px solid var(--c-border); background: var(--c-bg); border-radius: 4px; cursor: pointer; }
  .actions .primary { background: var(--c-accent); color: white; border-color: var(--c-accent); }
</style>
