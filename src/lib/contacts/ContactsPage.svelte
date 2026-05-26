<script lang="ts">
  import { store } from '../store.svelte';
  import * as ipc from '../ipc';
  import type { Contact } from '../types';

  type Form = {
    id: number | null;
    name: string;
    telegram_chat_id: string;
    telegram_handle: string;
    notes: string;
  };

  let form = $state<Form>(blankForm());
  let editing = $state<Contact | null>(null);
  let testStatus = $state<Record<number, 'idle' | 'sending' | 'ok' | 'error'>>({});
  let testError = $state<Record<number, string>>({});
  let showHelp = $state(false);
  let saving = $state(false);

  function blankForm(): Form {
    return { id: null, name: '', telegram_chat_id: '', telegram_handle: '', notes: '' };
  }

  function startEdit(c: Contact) {
    editing = c;
    form = {
      id: c.id, name: c.name,
      telegram_chat_id: c.telegram_chat_id ?? '',
      telegram_handle: c.telegram_handle ?? '',
      notes: c.notes,
    };
  }

  function cancelEdit() {
    editing = null;
    form = blankForm();
  }

  async function save() {
    if (!form.name.trim()) return;
    saving = true;
    try {
      const payload = {
        name: form.name.trim(),
        telegram_chat_id: form.telegram_chat_id.trim() || null,
        telegram_handle: form.telegram_handle.trim() || null,
        notes: form.notes,
      };
      if (form.id !== null && editing) {
        await store.updateContact({ ...editing, ...payload });
      } else {
        await store.createContact(payload);
      }
      cancelEdit();
    } finally {
      saving = false;
    }
  }

  async function remove(c: Contact) {
    if (!confirm(`Delete contact "${c.name}"? Any tasks assigned to them will be un-assigned.`)) return;
    await store.deleteContact(c.id);
  }

  async function testPing(c: Contact) {
    if (!c.telegram_chat_id) {
      testStatus[c.id] = 'error';
      testError[c.id] = 'No chat_id set';
      return;
    }
    testStatus[c.id] = 'sending';
    try {
      // Pulls token from meta inside test_telegram — we need to fetch it here.
      const token = prompt('Paste your Telegram bot token (it will be saved):', '');
      if (!token) {
        testStatus[c.id] = 'idle';
        return;
      }
      await ipc.testTelegram({ token, chat_id: c.telegram_chat_id });
      testStatus[c.id] = 'ok';
    } catch (e) {
      testStatus[c.id] = 'error';
      testError[c.id] = String(e);
    }
  }
</script>

<aside class="page">
  <header>
    <h1>Contacts</h1>
    <button class="help-btn" onclick={() => (showHelp = !showHelp)}>
      {showHelp ? 'Hide' : 'How does this work?'}
    </button>
    <button class="close-btn" onclick={() => (store.activeTool = null)} aria-label="Close contacts">×</button>
  </header>

  {#if showHelp}
    <div class="help">
      <h2>Setting up Telegram chasers</h2>
      <ol>
        <li>In Telegram, search for <code>@BotFather</code>. Send <code>/newbot</code> and follow the prompts. You'll get a bot token like <code>123456:ABC-def…</code>.</li>
        <li>Open <strong>Settings → Chaser</strong> in Blik Plan, paste the token, and hit <em>Test</em>.</li>
        <li>For each contact you want to nudge: have <em>them</em> search Telegram for <code>@userinfobot</code>, hit Start, and send you their numeric ID. Paste it into <strong>Telegram chat_id</strong> below.</li>
        <li>The contact must also send <code>/start</code> to <em>your</em> bot once so the bot is allowed to message them.</li>
        <li>Assign the contact to a task — done. They'll get pinged when deadlines approach.</li>
      </ol>
    </div>
  {/if}

  <div class="layout">
    <!-- Form column -->
    <section class="form">
      <h2>{form.id !== null ? 'Edit contact' : 'Add a contact'}</h2>
      <label>
        Name
        <input bind:value={form.name} placeholder="e.g. Caleb" />
      </label>
      <label>
        Telegram chat_id <span class="muted">(numeric, from @userinfobot)</span>
        <input bind:value={form.telegram_chat_id} placeholder="987654321" />
      </label>
      <label>
        Telegram handle <span class="muted">(@username — display only)</span>
        <input bind:value={form.telegram_handle} placeholder="@caleb_carpenter" />
      </label>
      <label>
        Notes
        <textarea bind:value={form.notes} rows="3" placeholder="Optional context"></textarea>
      </label>
      <div class="actions">
        <button class="primary" onclick={save} disabled={saving || !form.name.trim()}>
          {form.id !== null ? 'Save' : 'Add contact'}
        </button>
        {#if form.id !== null}
          <button onclick={cancelEdit}>Cancel</button>
        {/if}
      </div>
    </section>

    <!-- List column -->
    <section class="list">
      <h2>{store.contacts.length} contact{store.contacts.length === 1 ? '' : 's'}</h2>
      {#if store.contacts.length === 0}
        <p class="empty">No contacts yet. Add one on the left to start sending chasers.</p>
      {:else}
        {#each store.contacts as c (c.id)}
          <article class="card">
            <div class="head">
              <div>
                <h3>{c.name}</h3>
                {#if c.telegram_handle}<p class="handle">{c.telegram_handle}</p>{/if}
              </div>
              <div class="head-actions">
                <button onclick={() => startEdit(c)}>Edit</button>
                <button class="danger" onclick={() => remove(c)}>Delete</button>
              </div>
            </div>
            <div class="meta">
              <span>chat_id: <code>{c.telegram_chat_id || '—'}</code></span>
            </div>
            {#if c.notes}
              <p class="notes">{c.notes}</p>
            {/if}
            <div class="test-row">
              <button
                disabled={!c.telegram_chat_id || testStatus[c.id] === 'sending'}
                onclick={() => testPing(c)}
              >
                {#if testStatus[c.id] === 'sending'}Sending…
                {:else if testStatus[c.id] === 'ok'}✓ Sent
                {:else if testStatus[c.id] === 'error'}✗ Failed
                {:else}Send test ping{/if}
              </button>
              {#if testStatus[c.id] === 'error'}
                <span class="err">{testError[c.id]}</span>
              {/if}
            </div>
          </article>
        {/each}
      {/if}
    </section>
  </div>
</aside>

<style>
  .page {
    position: fixed; top: 0; right: 0; bottom: 0;
    width: 420px;
    background: var(--c-panel);
    border-left: 1px solid var(--c-border);
    box-shadow: -6px 0 18px rgba(0, 0, 0, 0.08);
    z-index: 70;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    padding: var(--sp-3) var(--sp-4);
  }
  header {
    display: flex; align-items: center; gap: var(--sp-3);
    margin-bottom: var(--sp-6);
  }
  header h1 { flex: 1; margin: 0; font-size: var(--font-size-xl); letter-spacing: -0.02em; }
  .back, .help-btn {
    background: transparent; border: 1px solid var(--c-border);
    padding: var(--sp-2) var(--sp-3); border-radius: 4px; cursor: pointer;
    font-size: var(--font-size-sm); color: var(--c-text);
  }
  .back:hover, .help-btn:hover { background: var(--c-accent-fade); color: var(--c-accent); }

  .help {
    background: var(--c-panel); border: 1px solid var(--c-border);
    border-left: 4px solid var(--c-accent);
    border-radius: 8px; padding: var(--sp-3) var(--sp-4);
    margin-bottom: var(--sp-4);
    font-size: var(--font-size-sm);
    line-height: 1.55;
  }
  .help h2 { margin: 0 0 var(--sp-2); font-size: var(--font-size-base); }
  .help ol { margin: 0; padding-left: var(--sp-4); }
  .help li { margin-bottom: 4px; }
  .help code { font-family: var(--font-mono); background: var(--c-accent-fade); color: var(--c-accent); padding: 1px 6px; border-radius: 3px; }

  .layout { display: flex; flex-direction: column; gap: var(--sp-3); }
  .close-btn {
    background: none; border: none; font-size: 22px; line-height: 1;
    cursor: pointer; color: var(--c-text-muted); padding: 0 var(--sp-2);
  }
  .close-btn:hover { color: var(--c-text); }

  section.form, section.list {
    background: var(--c-panel); border: 1px solid var(--c-border);
    border-radius: 8px; padding: var(--sp-4);
  }
  section h2 { margin: 0 0 var(--sp-3); font-size: var(--font-size-base); }

  .form label {
    display: flex; flex-direction: column; gap: 6px;
    font-size: var(--font-size-sm); margin-bottom: var(--sp-3);
  }
  .form input, .form textarea {
    padding: var(--sp-2); border: 1px solid var(--c-border); border-radius: 4px;
    font-family: inherit; font-size: var(--font-size-sm); background: var(--c-bg);
  }
  .form textarea { resize: vertical; min-height: 60px; }
  .muted { color: var(--c-text-muted); font-weight: 400; }
  .actions { display: flex; gap: var(--sp-2); }
  .actions button {
    padding: var(--sp-2) var(--sp-3); border-radius: 4px; cursor: pointer; font-size: var(--font-size-sm);
    border: 1px solid var(--c-border); background: var(--c-bg);
  }
  .actions .primary { background: var(--c-accent); color: white; border-color: var(--c-accent); font-weight: 600; }
  .actions .primary:disabled { opacity: 0.5; cursor: not-allowed; }

  .empty { color: var(--c-text-muted); font-size: var(--font-size-sm); }
  .card {
    border: 1px solid var(--c-border); border-radius: 6px;
    padding: var(--sp-3); margin-bottom: var(--sp-3); background: var(--c-bg);
  }
  .head { display: flex; justify-content: space-between; align-items: flex-start; }
  .head h3 { margin: 0; font-size: var(--font-size-base); }
  .handle { margin: 2px 0 0; color: var(--c-text-muted); font-size: var(--font-size-xs); font-family: var(--font-mono); }
  .head-actions { display: flex; gap: 4px; }
  .head-actions button {
    background: transparent; border: 1px solid var(--c-border); padding: 3px 8px;
    border-radius: 3px; cursor: pointer; font-size: var(--font-size-xs);
  }
  .head-actions .danger { color: #C8121E; }
  .head-actions .danger:hover { background: #FCE4E6; }
  .meta { margin: var(--sp-2) 0; font-size: var(--font-size-xs); color: var(--c-text-muted); }
  .meta code { font-family: var(--font-mono); }
  .notes { margin: var(--sp-2) 0 0; font-size: var(--font-size-sm); color: var(--c-text); }
  .test-row { display: flex; align-items: center; gap: var(--sp-2); margin-top: var(--sp-2); }
  .test-row button {
    background: var(--c-bg); border: 1px solid var(--c-border);
    padding: 4px 10px; border-radius: 3px; cursor: pointer; font-size: var(--font-size-xs);
  }
  .test-row button:disabled { opacity: 0.5; cursor: not-allowed; }
  .err { color: #C8121E; font-size: var(--font-size-xs); }
</style>
