<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { store } from '../store.svelte';
  import * as ipc from '../ipc';
  import InboxPanel from './InboxPanel.svelte';
  import ConnectToClaude from './ConnectToClaude.svelte';

  /* ---------- Settings popover ---------- */
  let settingsOpen = $state(false);

  /* ---------- Chaser settings (lazy-loaded when popover opens) ---------- */
  let chaserLoaded = $state(false);
  let chaserToken = $state('');
  let chaserThreshold = $state(3);
  let chaserAutoEnabled = $state(true);
  let templateManual = $state('');
  let templateApproaching = $state('');
  let templateOverdue = $state('');
  let testChatId = $state('');
  let testStatus = $state<'idle' | 'sending' | 'ok' | 'error'>('idle');
  let testError = $state('');

  async function loadChaserSettings() {
    if (chaserLoaded) return;
    chaserToken         = (await ipc.getMetaValue('telegram_bot_token')) ?? '';
    const t             = await ipc.getMetaValue('chaser_threshold_days');
    chaserThreshold     = t ? parseInt(t) || 3 : 3;
    const auto          = await ipc.getMetaValue('chaser_auto_enabled');
    chaserAutoEnabled   = auto !== '0';
    templateManual      = (await ipc.getMetaValue('chaser_template_manual')) ?? 'Update me on *{task}* — what\'s the latest?';
    templateApproaching = (await ipc.getMetaValue('chaser_template_approaching')) ?? '*{task}* deadline is in {days} days — still on track?';
    templateOverdue     = (await ipc.getMetaValue('chaser_template_overdue')) ?? '*{task}* was due {days_abs} days ago — what\'s the blocker?';
    chaserLoaded = true;
  }

  async function saveChaserField(key: string, value: string) {
    try { await ipc.setMetaValue(key, value); } catch (e) { console.error('saveChaserField', e); }
  }

  async function runTest() {
    testStatus = 'sending'; testError = '';
    try {
      await ipc.testTelegram({ token: chaserToken, chat_id: testChatId });
      testStatus = 'ok';
    } catch (e) {
      testStatus = 'error'; testError = String(e);
    }
  }

  $effect(() => {
    if (settingsOpen) loadChaserSettings();
  });

  /* ---------- Notes panel ---------- */
  let notesOpen = $state(false);

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
    try { await invoke('print_window_portrait'); }
    catch { window.print(); }
    setTimeout(() => document.body.classList.remove('print-todo-mode'), 1500);
  }
</script>

<div class="header-actions">
  <button class="icon-btn" onclick={() => (settingsOpen = !settingsOpen)} title="Settings" aria-label="Settings">
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="12" cy="12" r="3"/>
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
    </svg>
  </button>

  <button class="icon-btn" onclick={() => (notesOpen = !notesOpen)} title="Notes" aria-label="Notes">
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
      <polyline points="14 2 14 8 20 8"/>
      <line x1="9" y1="13" x2="15" y2="13"/>
      <line x1="9" y1="17" x2="15" y2="17"/>
    </svg>
  </button>

  <button class="icon-btn" onclick={() => (store.showContactsPage = true)} title="Contacts" aria-label="Contacts">
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/>
      <circle cx="9" cy="7" r="4"/>
      <path d="M23 21v-2a4 4 0 0 0-3-3.87"/>
      <path d="M16 3.13a4 4 0 0 1 0 7.75"/>
    </svg>
  </button>

  <button
    class="icon-btn inbox-btn"
    class:has-proposals={store.inboxPatches.length > 0}
    onclick={() => (store.inboxOpen = !store.inboxOpen)}
    title="Inbox — {store.inboxPatches.length} pending proposal{store.inboxPatches.length === 1 ? '' : 's'}"
    aria-label="Open Inbox"
  >
    <!-- Envelope icon -->
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z"/>
      <polyline points="22,6 12,13 2,6"/>
    </svg>
    {#if store.inboxPatches.length > 0}
      <span class="badge">{store.inboxPatches.length}</span>
    {/if}
  </button>
</div>

<!-- ============= Settings popover ============= -->
{#if settingsOpen}
  <div class="backdrop" onclick={() => (settingsOpen = false)} role="presentation"></div>
  <div class="popover settings-popover" role="dialog" aria-label="Settings">
    <header>
      <h2>Settings</h2>
      <button class="close" onclick={() => (settingsOpen = false)} aria-label="Close">×</button>
    </header>

    {#if store.currentJob}
      <section>
        <h3>Job name</h3>
        <input type="text" class="text-input"
          value={store.currentJob.name}
          onblur={(e) => store.renameCurrentJob((e.currentTarget as HTMLInputElement).value)}
          onkeydown={(e) => { if (e.key === 'Enter') (e.currentTarget as HTMLInputElement).blur(); }} />
        <h3 style="margin-top: var(--sp-4)">Project start date</h3>
        <input type="date" class="text-input"
          value={store.currentJob.project_start_date}
          onchange={(e) => store.setCurrentJobStartDate((e.currentTarget as HTMLInputElement).value)} />
      </section>
    {/if}

    <section>
      <h3>Duration display</h3>
      <div class="seg-toggle">
        <button class:active={store.durationUnit === 'weeks'}
          onclick={() => { if (store.durationUnit !== 'weeks') store.toggleDurationUnit(); }}>Weeks</button>
        <button class:active={store.durationUnit === 'days'}
          onclick={() => { if (store.durationUnit !== 'days') store.toggleDurationUnit(); }}>Days</button>
      </div>
    </section>

    <section>
      <h3>Weekends</h3>
      <label class="toggle">
        <input type="checkbox" checked={store.includeWeekends}
          onchange={(e) => store.setIncludeWeekends((e.currentTarget as HTMLInputElement).checked)} />
        <span>Show Saturday + Sunday columns</span>
      </label>
      <p class="hint">For projects where you work on weekends.</p>
    </section>

    <!-- Zoom slider is removed pending a clean re-implementation (CSS `zoom` was
         breaking pointer-coord calculations for the column-hover highlight). -->
    <!--
    <section>
      <h3>Zoom — {Math.round(store.uiScale * 100)}%</h3>
      <input type="range" min="0.75" max="1.5" step="0.05"
        value={store.uiScale}
        oninput={(e) => store.setUiScale(parseFloat((e.currentTarget as HTMLInputElement).value))} class="slider" />
      <div class="slider-labels"><span>75%</span><span>100%</span><span>150%</span></div>
    </section>
    -->


    <section>
      <h3>Chaser (Telegram)</h3>
      <label class="seg-label">Bot token</label>
      <input
        type="password"
        class="text-input"
        bind:value={chaserToken}
        onblur={() => saveChaserField('telegram_bot_token', chaserToken)}
        placeholder="123456:ABC-def..."
      />
      <p class="hint">Get from <code>@BotFather</code> in Telegram. Stored locally only.</p>

      <label class="seg-label" style="margin-top: var(--sp-3)">Test chat_id</label>
      <div style="display: flex; gap: var(--sp-2)">
        <input type="text" class="text-input" bind:value={testChatId} placeholder="987654321" />
        <button class="seg-button" onclick={runTest} disabled={!chaserToken || !testChatId || testStatus === 'sending'}>
          {#if testStatus === 'sending'}…
          {:else if testStatus === 'ok'}✓
          {:else if testStatus === 'error'}✗
          {:else}Test{/if}
        </button>
      </div>
      {#if testStatus === 'error'}<p class="hint err">{testError}</p>{/if}

      <label class="seg-label" style="margin-top: var(--sp-3)">Nudge me when deadline is within {chaserThreshold} day{chaserThreshold === 1 ? '' : 's'}</label>
      <input type="range" min="1" max="14" step="1" bind:value={chaserThreshold}
        class="slider"
        onchange={() => saveChaserField('chaser_threshold_days', String(chaserThreshold))} />

      <label class="toggle" style="margin-top: var(--sp-3)">
        <input type="checkbox" bind:checked={chaserAutoEnabled}
          onchange={() => saveChaserField('chaser_auto_enabled', chaserAutoEnabled ? '1' : '0')} />
        <span>Send chasers automatically on app launch + focus</span>
      </label>

      <details style="margin-top: var(--sp-3)">
        <summary class="seg-label" style="cursor: pointer">Message templates</summary>
        <label class="seg-label" style="margin-top: var(--sp-2)">Manual ping</label>
        <textarea class="text-input" rows="2" bind:value={templateManual}
          onblur={() => saveChaserField('chaser_template_manual', templateManual)}></textarea>
        <label class="seg-label" style="margin-top: var(--sp-2)">Deadline approaching</label>
        <textarea class="text-input" rows="2" bind:value={templateApproaching}
          onblur={() => saveChaserField('chaser_template_approaching', templateApproaching)}></textarea>
        <label class="seg-label" style="margin-top: var(--sp-2)">Overdue</label>
        <textarea class="text-input" rows="2" bind:value={templateOverdue}
          onblur={() => saveChaserField('chaser_template_overdue', templateOverdue)}></textarea>
        <p class="hint">Placeholders: <code>{'{task}'}</code>, <code>{'{days}'}</code>, <code>{'{days_abs}'}</code>, <code>{'{contact_name}'}</code>, <code>{'{job_name}'}</code></p>
      </details>
    </section>

    {#if store.currentJob}
      <section>
        <h3>Region — this job</h3>
        <select class="text-input"
          value={store.currentJob.region}
          onchange={(e) => store.setCurrentJobRegion((e.currentTarget as HTMLSelectElement).value)}>
          <option value="ZA">🇿🇦 South Africa</option>
          <option value="US">🇺🇸 United States</option>
          <option value="GB">🇬🇧 United Kingdom</option>
          <option value="IN">🇮🇳 India</option>
          <option value="CN">🇨🇳 China</option>
        </select>
        <p class="hint">Sets which public holidays are auto-synced. New jobs default to this setting.</p>
      </section>

      <section>
        <h3>Public holidays — this job</h3>
        <label class="toggle">
          <input type="checkbox" checked={store.currentJob.holidays_block_work}
            onchange={(e) => store.setJobHolidaysBlockWork((e.currentTarget as HTMLInputElement).checked)} />
          <span>Split bars around public holidays</span>
        </label>
        <p class="hint">
          {#if store.currentJob.holidays_block_work}Bars step around holidays — task extends one day.
          {:else}Holidays run through bars uninterrupted.{/if}
          New jobs default to this setting.
        </p>
      </section>
    {/if}

    <section>
      <ConnectToClaude />
    </section>
  </div>
{/if}

<!-- ============= Notes side panel ============= -->
{#if notesOpen}
  <aside class="todo-panel">
    <header class="screen-only">
      <h2>Notes — {store.currentJob?.name ?? ''}</h2>
      <div class="actions">
        <button onclick={printTodo} title="Print A4 portrait">Print</button>
        <button onclick={() => (notesOpen = false)} class="close" aria-label="Close">×</button>
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
          <textarea class="screen-only" placeholder="• Notes for {phase.name}…"
            value={phase.notes}
            onblur={(e) => saveNotes(phase.id, (e.currentTarget as HTMLTextAreaElement).value)}></textarea>
          <pre class="print-only">{phase.notes || ''}</pre>
        </section>
      {/each}
    </div>
  </aside>
{/if}

{#if store.inboxOpen}
  <InboxPanel />
{/if}

<style>
  .header-actions {
    display: flex; align-items: center; gap: var(--sp-1);
  }
  .icon-btn {
    background: transparent; border: none;
    padding: var(--sp-1) var(--sp-2);
    cursor: pointer; border-radius: 4px;
    color: var(--c-text-muted);
    font-size: var(--font-size-xs);
    display: flex; align-items: center;
    font-family: var(--font-mono);
    position: relative;
  }
  .icon-btn:hover { background: var(--c-accent-fade); color: var(--c-accent); }
  .inbox-btn .badge {
    position: absolute; top: -2px; right: -2px;
    min-width: 14px; height: 14px;
    border-radius: 7px;
    background: var(--c-accent); color: white;
    font-size: 10px; font-weight: 600;
    display: flex; align-items: center; justify-content: center;
    padding: 0 4px;
  }

  .close { background: none; border: none; font-size: 22px; cursor: pointer; color: var(--c-text-muted); line-height: 1; padding: 0 var(--sp-2); }

  /* ============ Settings popover ============ */
  .backdrop { position: fixed; inset: 0; background: transparent; z-index: 50; }
  .settings-popover {
    position: fixed; top: 56px; right: 16px; z-index: 51;
    background: var(--c-panel); border: 1px solid var(--c-border); border-radius: 8px;
    min-width: 320px; max-width: 380px;
    box-shadow: 0 12px 36px rgba(0,0,0,0.18);
    display: flex; flex-direction: column;
    max-height: 75vh; overflow-y: auto;
  }
  .settings-popover header { display: flex; justify-content: space-between; align-items: center; padding: var(--sp-3) var(--sp-4); border-bottom: 1px solid var(--c-border); }
  .settings-popover header h2 { margin: 0; font-size: var(--font-size-base); }
  .settings-popover section { padding: var(--sp-3) var(--sp-4); border-bottom: 1px solid var(--c-border); }
  .settings-popover section:last-child { border-bottom: none; }
  .settings-popover section h3 { margin: 0 0 var(--sp-2); font-size: var(--font-size-xs); text-transform: uppercase; letter-spacing: 0.05em; color: var(--c-text-muted); }
  .seg-toggle { display: flex; border: 1px solid var(--c-border); border-radius: 6px; overflow: hidden; width: max-content; }
  .seg-toggle button { background: var(--c-panel); border: none; padding: var(--sp-2) var(--sp-3); cursor: pointer; font-size: var(--font-size-sm); color: var(--c-text-muted); }
  .seg-toggle button.active { background: var(--c-accent); color: white; }
  .seg-toggle button:not(.active):hover { background: var(--c-accent-fade); color: var(--c-accent); }
  .toggle { display: flex; align-items: center; gap: var(--sp-2); cursor: pointer; font-size: var(--font-size-sm); }
  .toggle input { width: 16px; height: 16px; cursor: pointer; }
  .slider { width: 100%; }
  .slider-labels { display: flex; justify-content: space-between; font-size: var(--font-size-xs); color: var(--c-text-muted); margin-top: var(--sp-1); }
  .text-input { width: 100%; padding: var(--sp-2); border: 1px solid var(--c-border); border-radius: 4px; font-size: var(--font-size-sm); font-family: inherit; }
  textarea.text-input { resize: vertical; min-height: 44px; }
  .seg-label { display: block; font-size: var(--font-size-xs); color: var(--c-text-muted); margin-bottom: 4px; text-transform: uppercase; letter-spacing: 0.05em; }
  .seg-button { background: var(--c-bg); border: 1px solid var(--c-border); padding: var(--sp-2) var(--sp-3); border-radius: 4px; cursor: pointer; font-size: var(--font-size-sm); }
  .seg-button:disabled { opacity: 0.5; cursor: not-allowed; }
  .hint code { font-family: var(--font-mono); background: var(--c-accent-fade); color: var(--c-accent); padding: 1px 4px; border-radius: 2px; }
  .hint.err { color: #C8121E; }

  /* ============ Notes panel ============ */
  .todo-panel {
    position: fixed; top: 0; right: 0; bottom: 0;
    width: 360px;
    background: var(--c-panel); border-left: 1px solid var(--c-border);
    z-index: 40;
    display: flex; flex-direction: column;
    box-shadow: -6px 0 18px rgba(0,0,0,0.06);
  }
  .todo-panel header { display: flex; justify-content: space-between; align-items: center; padding: var(--sp-3) var(--sp-4); border-bottom: 1px solid var(--c-border); }
  .todo-panel header h2 { margin: 0; font-size: var(--font-size-base); }
  .actions { display: flex; gap: var(--sp-2); align-items: center; }
  .actions button { background: var(--c-bg); border: 1px solid var(--c-border); padding: var(--sp-1) var(--sp-3); border-radius: 4px; cursor: pointer; font-size: var(--font-size-sm); }
  .todo-panel .close { background: transparent !important; border: none !important; font-size: 22px; line-height: 1; padding: 0 var(--sp-1) !important; color: var(--c-text-muted); }
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
  textarea { width: 100%; min-height: 100px; padding: var(--sp-2); border: 1px solid var(--c-border); border-radius: 4px; font-family: inherit; font-size: var(--font-size-sm); resize: vertical; background: var(--c-bg); }
  textarea:focus { outline: 2px solid var(--c-accent-fade); border-color: var(--c-accent); }
  .print-only { display: none; }
  .print-header-todo h1 { margin: 0 0 4mm; font-size: 18pt; }
  .print-header-todo .meta { margin: 0 0 6mm; font-size: 10pt; color: #555; }

  @media print {
    body.print-todo-mode .screen-only { display: none !important; }
    body.print-todo-mode .print-only { display: block !important; }
    body.print-todo-mode .todo-panel { position: static; width: 100%; height: auto; border: none; box-shadow: none; padding: 0; }
    body.print-todo-mode .content { overflow: visible; padding: 0; }
    body.print-todo-mode .phase-block h3 { color: black !important; font-size: 14pt; }
    body.print-todo-mode pre { font-family: inherit; font-size: 11pt; white-space: pre-wrap; line-height: 1.5; margin: 0 0 2mm 6mm; }
    body.print-todo-mode .app-shell, body.print-todo-mode .print-header, body.print-todo-mode .print-footer { display: none !important; }
  }

  /* ============ Inbox badge button ============ */
  .inbox-btn {
    position: relative;
  }
  .inbox-btn.has-proposals {
    color: var(--c-accent);
  }
  .badge {
    position: absolute;
    top: -2px;
    right: -4px;
    background: var(--c-accent);
    color: white;
    border-radius: 50%;
    width: 14px;
    height: 14px;
    font-size: 9px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-mono);
    font-weight: 700;
    line-height: 1;
  }
</style>
