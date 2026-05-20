<script lang="ts">
  import { store } from '../store.svelte';

  let lastSavedAt = $state<string | null>(null);
  let savingState = $state<'saved' | 'saving' | 'failed'>('saved');

  $effect(() => {
    if (store.hasUnsavedUndo) savingState = 'saving';
    else savingState = 'saved';
  });

  async function manualSave() {
    savingState = 'saving';
    try {
      await store.resyncJobState();
      lastSavedAt = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
      savingState = 'saved';
    } catch {
      savingState = 'failed';
    }
  }
</script>

<button class="indicator state-{savingState}" onclick={manualSave} title="Manual save (⌘S)">
  {#if savingState === 'saved'}
    Saved {lastSavedAt ?? 'now'}
  {:else if savingState === 'saving'}
    Saving…
  {:else}
    Save failed — click to retry
  {/if}
  <span class="hint">⌘S</span>
</button>

<style>
  .indicator {
    position: fixed; bottom: 8px; right: 12px;
    z-index: 5;
    background: transparent; border: none;
    font-size: var(--font-size-xs);
    color: var(--c-text-muted);
    cursor: pointer;
    padding: var(--sp-1) var(--sp-2);
    border-radius: 4px;
    display: flex; align-items: center; gap: var(--sp-2);
  }
  .indicator:hover { background: var(--c-accent-fade); color: var(--c-accent); }
  .indicator.state-saving { opacity: 0.5; }
  .indicator.state-failed { color: #DC2626; background: #FEE2E2; }
  .hint { font-family: var(--font-mono); opacity: 0.6; }
</style>
