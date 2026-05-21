<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { store } from '../store.svelte';
  import * as ipc from '../ipc';

  let open = $state(false);

  async function saveNotes(phaseId: number, value: string) {
    const phase = store.phases.find((p) => p.id === phaseId);
    if (!phase || phase.notes === value) return;
    phase.notes = value;
    await ipc.updatePhase($state.snapshot(phase));
    await ipc.touchLastSave();
  }

  async function printTodo() {
    document.body.classList.add('print-todo-mode');
    await new Promise((r) => setTimeout(r, 80));
    try {
      await invoke('print_window_portrait');
    } catch (e) {
      console.error('todo print failed', e);
      window.print();
    }
    setTimeout(() => document.body.classList.remove('print-todo-mode'), 1500);
  }
</script>

<button class="toggle-btn" onclick={() => (open = !open)} title="Notes" aria-label="Notes">
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
    <polyline points="14 2 14 8 20 8"/>
    <line x1="9" y1="13" x2="15" y2="13"/>
    <line x1="9" y1="17" x2="15" y2="17"/>
  </svg>
</button>

{#if open}
  <aside class="todo-panel" class:print-mode={true}>
    <header class="screen-only">
      <h2>Notes — {store.currentJob?.name ?? ''}</h2>
      <div class="actions">
        <button onclick={printTodo} title="Print A4 portrait">Print</button>
        <button onclick={() => (open = false)} class="close" aria-label="Close">×</button>
      </div>
    </header>

    <div class="print-only print-header-todo">
      <h1>{store.currentJob?.name ?? 'Notes'}</h1>
      <p class="meta">
        {#if store.currentJob?.client}Client: {store.currentJob.client} · {/if}
        Printed: {new Date().toLocaleDateString('en-GB', { day: '2-digit', month: 'short', year: 'numeric' })}
      </p>
    </div>

    <div class="content">
      {#each store.phases as phase (phase.id)}
        <section class="phase-block">
          <h3 style="border-left-color: {phase.colour}; color: {phase.colour}">
            <span class="swatch" style="background: {phase.colour}"></span>
            {phase.name}
          </h3>
          <textarea
            class="screen-only"
            placeholder="• Notes for {phase.name}…"
            value={phase.notes}
            onblur={(e) => saveNotes(phase.id, (e.currentTarget as HTMLTextAreaElement).value)}
          ></textarea>
          <pre class="print-only">{phase.notes || ''}</pre>
        </section>
      {/each}
    </div>
  </aside>
{/if}

<style>
  .toggle-btn {
    position: fixed; bottom: 8px; left: 108px;
    z-index: 5;
    background: transparent; border: none;
    color: var(--c-text-muted);
    padding: var(--sp-1) var(--sp-2);
    cursor: pointer;
    border-radius: 4px;
    display: flex; align-items: center;
  }
  .toggle-btn:hover { background: var(--c-accent-fade); color: var(--c-accent); }

  .todo-panel {
    position: fixed; top: 0; right: 0; bottom: 0;
    width: 360px;
    background: var(--c-panel);
    border-left: 1px solid var(--c-border);
    z-index: 40;
    display: flex; flex-direction: column;
    box-shadow: -6px 0 18px rgba(0,0,0,0.06);
  }
  header {
    display: flex; justify-content: space-between; align-items: center;
    padding: var(--sp-3) var(--sp-4);
    border-bottom: 1px solid var(--c-border);
  }
  header h2 { margin: 0; font-size: var(--font-size-md); }
  .actions { display: flex; gap: var(--sp-2); align-items: center; }
  .actions button {
    background: var(--c-bg); border: 1px solid var(--c-border);
    padding: var(--sp-1) var(--sp-3); border-radius: 4px; cursor: pointer;
    font-size: var(--font-size-sm);
  }
  .close { background: transparent !important; border: none !important; font-size: 22px; line-height: 1; padding: 0 var(--sp-1) !important; color: var(--c-text-muted); }

  .content { overflow-y: auto; padding: var(--sp-3) var(--sp-4); flex: 1; }
  .phase-block { margin-bottom: var(--sp-6); page-break-inside: avoid; }
  .phase-block h3 {
    display: flex; align-items: center; gap: var(--sp-2);
    margin: 0 0 var(--sp-2);
    padding: var(--sp-1) var(--sp-2);
    border-left: 4px solid var(--c-accent);
    font-size: var(--font-size-base);
    font-weight: 700;
  }
  .swatch { width: 12px; height: 12px; border-radius: 2px; display: inline-block; }
  textarea {
    width: 100%; min-height: 100px;
    padding: var(--sp-2);
    border: 1px solid var(--c-border);
    border-radius: 4px;
    font-family: inherit; font-size: var(--font-size-sm);
    resize: vertical;
    background: var(--c-bg);
  }
  textarea:focus { outline: 2px solid var(--c-accent-fade); border-color: var(--c-accent); }

  .print-only { display: none; }
  .print-header-todo h1 { margin: 0 0 4mm; font-size: 18pt; }
  .print-header-todo .meta { margin: 0 0 6mm; font-size: 10pt; color: #555; }
  .print-header-todo { display: none; }

  @media print {
    body.print-todo-mode .screen-only { display: none !important; }
    body.print-todo-mode .print-only { display: block !important; }
    body.print-todo-mode .print-header-todo { display: block !important; }
    body.print-todo-mode .todo-panel {
      position: static; width: 100%; height: auto;
      border: none; box-shadow: none;
      padding: 0;
    }
    body.print-todo-mode .content { overflow: visible; padding: 0; }
    body.print-todo-mode .phase-block h3 { color: black !important; font-size: 14pt; }
    body.print-todo-mode pre {
      font-family: inherit; font-size: 11pt;
      white-space: pre-wrap; line-height: 1.5;
      margin: 0 0 2mm 6mm;
    }
    body.print-todo-mode .app-shell,
    body.print-todo-mode .print-header,
    body.print-todo-mode .print-footer { display: none !important; }
  }
</style>
