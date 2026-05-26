<script lang="ts">
  import { store } from '../store.svelte';
  import * as ipc from '../ipc';
  import type { Task, TaskStatus } from '../types';

  let { taskId }: { taskId: number } = $props();
  const task = $derived(store.tasks.find(t => t.id === taskId));

  let name = $state('');
  let duration = $state(1);
  let notes = $state('');
  let confirmingDelete = $state(false);
  let pingMenuOpen = $state(false);
  let customText = $state('');
  let sendingPing = $state(false);
  let pingError = $state('');
  let pingSentLabel = $state('');

  $effect(() => {
    if (task) {
      name = task.name;
      duration = task.duration_workdays;
      notes = task.notes ?? '';
      confirmingDelete = false;
    }
  });

  async function save() {
    if (!task) return;
    const updated: Task = {
      ...task,
      name: name.trim() || task.name,
      duration_workdays: Math.max(1, duration),
      notes: notes.trim() || null,
    };
    await ipc.updateTask($state.snapshot(updated));
    store.tasks = store.tasks.map(t => t.id === updated.id ? updated : t);
    await ipc.touchLastSave();
  }

  async function onStatusChange(e: Event) {
    if (!task) return;
    const newStatus = (e.currentTarget as HTMLSelectElement).value as TaskStatus;
    const completion = newStatus === 'done'
      ? new Date().toISOString().slice(0, 10)
      : null;
    await store.setTaskStatus(task.id, newStatus, completion);
  }

  async function assignContact(e: Event) {
    if (!task) return;
    const val = (e.currentTarget as HTMLSelectElement).value;
    const contact_id = val === '' ? null : Number(val);
    await store.assignTaskContact(task.id, contact_id);
  }

  async function sendPing(templateKey: 'manual' | 'approaching' | 'overdue' | 'custom') {
    if (!task) return;
    sendingPing = true;
    pingError = '';
    pingSentLabel = '';
    try {
      const customSnap = templateKey === 'custom' ? customText.trim() : undefined;
      if (templateKey === 'custom' && !customSnap) {
        pingError = 'Write a message first';
        return;
      }
      await ipc.sendChaser({ task_id: task.id, template_key: templateKey, custom_text: customSnap });
      pingSentLabel = '✓ Sent';
      pingMenuOpen = false;
      customText = '';
      setTimeout(() => { pingSentLabel = ''; }, 3000);
    } catch (e) {
      pingError = String(e);
    } finally {
      sendingPing = false;
    }
  }

  const assignedContact = $derived(task?.contact_id != null
    ? store.contacts.find(c => c.id === task.contact_id) ?? null
    : null);

  async function del() {
    if (!task) return;
    if (!confirmingDelete) {
      confirmingDelete = true;
      setTimeout(() => { confirmingDelete = false; }, 3000);
      return;
    }
    await ipc.deleteTask(task.id);
    store.tasks = store.tasks.filter(t => t.id !== task.id);
    store.select(null);
    await ipc.touchLastSave();
  }
</script>

{#if task}
  <div class="task-details">
    <h2>{task.name}</h2>
    <label>Name<input bind:value={name} onblur={save} /></label>
    <label>Duration (workdays)<input type="number" min="1" bind:value={duration} onblur={save} /></label>
    <label>Start<input type="date" value={task.start_date} disabled /></label>
    <label>Status
      <select value={task.status ?? 'not_started'} onchange={onStatusChange}>
        <option value="not_started">Not Started</option>
        <option value="on_track">On Track</option>
        <option value="late">Late</option>
        <option value="done">Done</option>
      </select>
    </label>
    <label>Notes<textarea bind:value={notes} onblur={save} rows="4"></textarea></label>

    <section class="chaser">
      <label>
        Assigned to
        <select onchange={assignContact} value={task.contact_id ?? ''}>
          <option value="">— No one —</option>
          {#each store.contacts as c (c.id)}
            <option value={c.id}>{c.name}{c.telegram_handle ? ` (${c.telegram_handle})` : ''}</option>
          {/each}
        </select>
      </label>

      {#if assignedContact}
        <div class="ping-row">
          <button class="ping-btn" onclick={() => (pingMenuOpen = !pingMenuOpen)} disabled={sendingPing}>
            {#if sendingPing}Sending…
            {:else if pingSentLabel}{pingSentLabel}
            {:else}Ping {assignedContact.name} ▾{/if}
          </button>
          {#if pingMenuOpen}
            <div class="ping-menu">
              <button onclick={() => sendPing('manual')}>Manual update</button>
              <button onclick={() => sendPing('approaching')}>Deadline approaching</button>
              <button onclick={() => sendPing('overdue')}>Behind schedule</button>
              <div class="custom">
                <textarea bind:value={customText} rows="2" placeholder="Custom message…"></textarea>
                <button class="send-custom" onclick={() => sendPing('custom')} disabled={!customText.trim()}>Send custom</button>
              </div>
            </div>
          {/if}
        </div>
        {#if pingError}<p class="err">{pingError}</p>{/if}
        {#if task.last_chaser_sent_at}
          <p class="muted">Last pinged: {task.last_chaser_sent_at}</p>
        {/if}
      {/if}
    </section>

    <button class="danger" onclick={del}>{confirmingDelete ? 'Click again to confirm delete' : 'Delete task'}</button>
  </div>
{/if}

<style>
  .task-details { display: flex; flex-direction: column; gap: var(--sp-3); padding: var(--sp-4); }
  h2 { font-size: var(--font-size-lg); margin: 0; }
  label { display: flex; flex-direction: column; gap: var(--sp-1); font-size: var(--font-size-sm); color: var(--c-text-muted); }
  input, textarea { padding: var(--sp-2); border: 1px solid var(--c-border); border-radius: 4px; font: inherit; color: var(--c-text); background: var(--c-bg); }
  input:disabled { color: var(--c-text-muted); }
  .danger {
    margin-top: var(--sp-3); padding: var(--sp-2);
    background: transparent; border: 1px solid #DC2626; color: #DC2626; border-radius: 4px;
    cursor: pointer;
  }
  .danger:hover { background: #FEE2E2; }

  .chaser {
    display: flex; flex-direction: column; gap: var(--sp-2);
    padding: var(--sp-3); margin-top: var(--sp-2);
    background: var(--c-panel); border: 1px solid var(--c-border); border-radius: 6px;
  }
  .chaser select { padding: var(--sp-2); border: 1px solid var(--c-border); border-radius: 4px; background: var(--c-bg); font: inherit; color: var(--c-text); }
  .ping-row { position: relative; }
  .ping-btn {
    width: 100%; padding: var(--sp-2) var(--sp-3);
    background: var(--c-accent); color: white; border: none;
    border-radius: 4px; cursor: pointer; font-weight: 600;
    font-size: var(--font-size-sm);
  }
  .ping-btn:hover { opacity: 0.9; }
  .ping-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .ping-menu {
    margin-top: var(--sp-2);
    background: var(--c-bg); border: 1px solid var(--c-border); border-radius: 4px;
    padding: var(--sp-2); display: flex; flex-direction: column; gap: var(--sp-1);
  }
  .ping-menu > button {
    text-align: left; background: transparent; border: none; cursor: pointer;
    padding: var(--sp-2); border-radius: 3px; font-size: var(--font-size-sm); color: var(--c-text);
  }
  .ping-menu > button:hover { background: var(--c-accent-fade); color: var(--c-accent); }
  .custom { display: flex; flex-direction: column; gap: var(--sp-1); padding-top: var(--sp-2); border-top: 1px solid var(--c-border); margin-top: var(--sp-1); }
  .custom textarea { padding: var(--sp-2); border: 1px solid var(--c-border); border-radius: 3px; font: inherit; resize: vertical; }
  .send-custom { background: var(--c-accent); color: white; border: none; padding: var(--sp-1) var(--sp-2); border-radius: 3px; cursor: pointer; font-size: var(--font-size-xs); align-self: flex-end; }
  .send-custom:disabled { opacity: 0.5; cursor: not-allowed; }
  .err { color: #C8121E; font-size: var(--font-size-xs); margin: 0; }
  .muted { color: var(--c-text-muted); font-size: var(--font-size-xs); margin: 0; font-family: var(--font-mono); }
</style>
