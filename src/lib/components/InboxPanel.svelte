<script lang="ts">
  import { onDestroy } from 'svelte';
  import { store } from '../store.svelte';
  import { renderPatchOp } from '../inbox-diff';

  // Stop polling when this panel is destroyed (e.g. if it's conditionally mounted).
  // The poll is started in store.bootstrap(), so we only stop it here if explicitly needed.
  // In this implementation the panel is always mounted alongside BottomToolbar, so we
  // rely on the store's stopInboxPoll() only when the user explicitly closes/reopens.
  onDestroy(() => {
    store.stopInboxPoll();
  });

  let acceptingId = $state<string | null>(null);
  let rejectingId = $state<string | null>(null);
  let clearing    = $state(false);
  let actionError = $state<string | null>(null);

  async function accept(id: string) {
    acceptingId = id;
    actionError = null;
    try {
      await store.acceptInboxPatch(id);
    } catch (e) {
      actionError = `Accept failed: ${e}`;
    } finally {
      acceptingId = null;
    }
  }

  async function reject(id: string) {
    rejectingId = id;
    actionError = null;
    try {
      await store.rejectInboxPatch(id);
    } catch (e) {
      actionError = `Reject failed: ${e}`;
    } finally {
      rejectingId = null;
    }
  }

  async function clearResolved() {
    clearing = true;
    actionError = null;
    try {
      await store.clearResolvedPatches();
    } catch (e) {
      actionError = `Clear failed: ${e}`;
    } finally {
      clearing = false;
    }
  }

  function formatDate(ts: number): string {
    return new Date(ts * 1000).toLocaleDateString('en-GB', {
      day: '2-digit', month: 'short', year: 'numeric',
      hour: '2-digit', minute: '2-digit',
    });
  }

  // ISO YYYY-MM-DD → DD/MM/YYYY
  function formatDmy(iso: string): string {
    const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(iso);
    return m ? `${m[3]}/${m[2]}/${m[1]}` : iso;
  }

  // Per-card "Mark Done" picker state — taskId is the open one (null = closed).
  let pickerOpenFor = $state<number | null>(null);
  let pickerDate = $state<string>('');
  let submitting = $state<number | null>(null);

  function openPicker(taskId: number, defaultIso: string) {
    pickerOpenFor = taskId;
    pickerDate = defaultIso;
  }
  function cancelPicker() {
    pickerOpenFor = null;
    pickerDate = '';
  }
  async function confirmDone(taskId: number) {
    if (!pickerDate) return;
    submitting = taskId;
    try {
      await store.resolveOverdueAsDone(taskId, pickerDate);
      pickerOpenFor = null;
      pickerDate = '';
    } catch (e) {
      actionError = String(e);
    } finally {
      submitting = null;
    }
  }

  async function markRunningLate(taskId: number) {
    submitting = taskId;
    try {
      await store.resolveOverdueAsRunningLate(taskId);
    } catch (e) {
      actionError = String(e);
    } finally {
      submitting = null;
    }
  }
</script>

<aside class="inbox-panel">
  <header class="inbox-header">
    <h2>Inbox</h2>
    <button class="close-btn" onclick={() => (store.inboxOpen = false)} aria-label="Close inbox">×</button>
  </header>

  {#if actionError}
    <div class="action-error">{actionError}</div>
  {/if}

  <div class="inbox-body">

  {#if store.overdueReviews.length > 0}
    <section class="review-section">
      <h3 class="section-title">Tasks needing review</h3>
      {#each store.overdueReviews as r (r.task_id)}
        {@const phaseName = store.phases.find(p => p.id === r.phase_id)?.name ?? '—'}
        <article class="review-card">
          <div class="review-phase">{phaseName}</div>
          <div class="review-name">{r.task_name}</div>
          <div class="review-meta">was due {formatDmy(r.planned_end_date)}</div>
          {#if pickerOpenFor === r.task_id}
            <div class="picker-row">
              <input type="date" bind:value={pickerDate} max={store.todayIso} />
              <button
                class="accept-btn"
                disabled={!pickerDate || submitting === r.task_id}
                onclick={() => confirmDone(r.task_id)}
              >{submitting === r.task_id ? 'Saving…' : 'Confirm'}</button>
              <button class="reject-btn" onclick={cancelPicker}>Cancel</button>
            </div>
          {:else}
            <div class="review-actions">
              <button
                class="accept-btn"
                disabled={submitting === r.task_id}
                onclick={() => openPicker(r.task_id, r.planned_end_date)}
              >Mark Done…</button>
              <button
                class="reject-btn"
                disabled={submitting === r.task_id}
                onclick={() => markRunningLate(r.task_id)}
              >{submitting === r.task_id ? 'Saving…' : 'Running late'}</button>
            </div>
          {/if}
        </article>
      {/each}
    </section>
  {/if}

  {#if store.inboxPatches.length === 0 && store.overdueReviews.length === 0}
    <div class="empty-state">
      <p>Inbox is clear.</p>
      <p class="hint">Tasks needing a finish-date confirmation show up here. Claude proposals also land here when wired in Settings.</p>
    </div>
  {/if}

  {#if store.inboxPatches.length > 0}
    <div class="patch-list">
      {#each store.inboxPatches as patch (patch.id)}
        <article class="patch-card">
          <header class="patch-header">
            <span class="patch-summary">{patch.summary}</span>
            <span class="patch-meta">{formatDate(patch.created_at)}</span>
          </header>

          <ul class="op-list">
            {#each patch.patch.ops as op}
              <li class="op-line">
                {renderPatchOp(op, store.phases, store.tasks, store.contacts)}
              </li>
            {/each}
          </ul>

          <footer class="patch-actions">
            <button
              class="accept-btn"
              disabled={acceptingId === patch.id || rejectingId === patch.id}
              onclick={() => accept(patch.id)}
            >
              {acceptingId === patch.id ? 'Applying…' : 'Accept'}
            </button>
            <button
              class="reject-btn"
              disabled={acceptingId === patch.id || rejectingId === patch.id}
              onclick={() => reject(patch.id)}
            >
              {rejectingId === patch.id ? 'Rejecting…' : 'Reject'}
            </button>
          </footer>
        </article>
      {/each}
    </div>
  {/if}

  </div>

  <footer class="inbox-footer">

    <button class="clear-btn" disabled={clearing} onclick={clearResolved}>
      {clearing ? 'Clearing…' : 'Clear resolved'}
    </button>
  </footer>
</aside>

<style>
  .inbox-panel {
    position: fixed;
    top: 0; right: 0; bottom: 0;
    width: 380px;
    background: var(--c-panel);
    border-left: 1px solid var(--c-border);
    z-index: 40;
    display: flex;
    flex-direction: column;
    box-shadow: -6px 0 18px rgba(0, 0, 0, 0.08);
  }

  .inbox-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--sp-3) var(--sp-4);
    border-bottom: 1px solid var(--c-border);
    flex-shrink: 0;
  }

  .inbox-header h2 {
    margin: 0;
    font-size: var(--font-size-base);
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 22px;
    cursor: pointer;
    color: var(--c-text-muted);
    line-height: 1;
    padding: 0 var(--sp-1);
  }

  .close-btn:hover { color: var(--c-text); }

  .action-error {
    background: #FEE2E2;
    color: #C8121E;
    font-size: var(--font-size-xs);
    padding: var(--sp-2) var(--sp-4);
    border-bottom: 1px solid #FCA5A5;
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--sp-6) var(--sp-4);
    text-align: center;
    color: var(--c-text-muted);
  }

  .empty-state p { margin: 0 0 var(--sp-2); }

  .hint {
    font-size: var(--font-size-xs);
    color: var(--c-text-muted);
  }

  .inbox-body {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  .patch-list {
    padding: var(--sp-3) var(--sp-4);
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
  }

  .review-section {
    padding: var(--sp-3) var(--sp-4) 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
  }
  .section-title {
    font-size: var(--font-size-xs);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--c-text-muted);
    margin: 0 0 var(--sp-1);
  }
  .review-card {
    border: 1px solid var(--c-border);
    border-radius: 6px;
    background: var(--c-bg);
    padding: var(--sp-2) var(--sp-3);
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
  }
  .review-phase {
    font-size: var(--font-size-xs);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--c-text-muted);
  }
  .review-name { font-weight: 600; font-size: var(--font-size-sm); }
  .review-meta { font-size: var(--font-size-xs); color: var(--c-text-muted); }
  .review-actions { display: flex; gap: var(--sp-2); margin-top: var(--sp-1); }
  .review-actions button { flex: 1; }
  .picker-row {
    display: flex;
    gap: var(--sp-2);
    margin-top: var(--sp-1);
    align-items: center;
  }
  .picker-row input[type="date"] {
    flex: 1;
    padding: var(--sp-1) var(--sp-2);
    border: 1px solid var(--c-border);
    border-radius: 4px;
    font: inherit;
  }
  .picker-row button { padding: var(--sp-1) var(--sp-2); }

  .patch-card {
    border: 1px solid var(--c-border);
    border-radius: 6px;
    background: var(--c-bg);
    overflow: hidden;
  }

  .patch-header {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
    padding: var(--sp-2) var(--sp-3);
    background: var(--c-panel);
    border-bottom: 1px solid var(--c-border);
  }

  .patch-summary {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--c-text);
  }

  .patch-meta {
    font-size: var(--font-size-xs);
    color: var(--c-text-muted);
    font-family: var(--font-mono);
  }

  .op-list {
    list-style: none;
    margin: 0;
    padding: var(--sp-2) var(--sp-3);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .op-line {
    font-size: var(--font-size-xs);
    color: var(--c-text-muted);
    line-height: 1.4;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .patch-actions {
    display: flex;
    gap: var(--sp-2);
    padding: var(--sp-2) var(--sp-3);
    border-top: 1px solid var(--c-border);
  }

  .accept-btn {
    flex: 1;
    background: var(--c-accent);
    color: white;
    border: none;
    border-radius: 4px;
    padding: var(--sp-2) var(--sp-3);
    cursor: pointer;
    font-size: var(--font-size-sm);
    font-family: inherit;
  }

  .accept-btn:hover:not(:disabled) { filter: brightness(1.1); }
  .accept-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .reject-btn {
    background: var(--c-bg);
    color: var(--c-text-muted);
    border: 1px solid var(--c-border);
    border-radius: 4px;
    padding: var(--sp-2) var(--sp-3);
    cursor: pointer;
    font-size: var(--font-size-sm);
    font-family: inherit;
  }

  .reject-btn:hover:not(:disabled) {
    background: #FEE2E2;
    color: #C8121E;
    border-color: #FCA5A5;
  }

  .reject-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .inbox-footer {
    padding: var(--sp-3) var(--sp-4);
    border-top: 1px solid var(--c-border);
    flex-shrink: 0;
  }

  .clear-btn {
    width: 100%;
    background: var(--c-bg);
    border: 1px solid var(--c-border);
    border-radius: 4px;
    padding: var(--sp-2) var(--sp-3);
    cursor: pointer;
    font-size: var(--font-size-xs);
    color: var(--c-text-muted);
    font-family: inherit;
  }

  .clear-btn:hover:not(:disabled) {
    background: var(--c-accent-fade);
    color: var(--c-accent);
  }

  .clear-btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
