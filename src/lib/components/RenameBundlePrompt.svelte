<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  type RenameInfo = {
    current_path: string;
    current_name: string;
    desired_path: string;
    desired_name: string;
  };

  let info = $state<RenameInfo | null>(null);
  let dismissed = $state(false);
  let renaming = $state(false);
  let error = $state('');

  onMount(async () => {
    try {
      const result = await invoke<RenameInfo | null>('bundle_rename_needed');
      if (result) info = result;
    } catch (e) {
      // Silently ignore — this is a polish nicety, not critical.
      console.warn('bundle_rename_needed failed', e);
    }
  });

  async function rename() {
    renaming = true;
    error = '';
    try {
      await invoke('rename_bundle_and_restart');
      // App quits before we get here.
    } catch (e) {
      renaming = false;
      error = String(e);
    }
  }
</script>

{#if info && !dismissed}
  <div class="banner-wrap">
    <div class="banner">
      <div class="text">
        <strong>Rebranded.</strong>
        Your app file is still called <code>{info.current_name}</code>. Click below to rename it to
        <code>{info.desired_name}</code> and relaunch — takes about a second.
      </div>
      <div class="actions">
        <button class="primary" onclick={rename} disabled={renaming}>
          {renaming ? 'Renaming…' : 'Rename & relaunch'}
        </button>
        <button class="ghost" onclick={() => (dismissed = true)}>Not now</button>
      </div>
      {#if error}<div class="err">{error}</div>{/if}
    </div>
  </div>
{/if}

<style>
  .banner-wrap {
    position: fixed; top: 12px; left: 50%; transform: translateX(-50%);
    z-index: 60; max-width: 540px; width: calc(100% - 32px);
  }
  .banner {
    background: var(--c-panel);
    border: 1px solid var(--c-accent);
    border-left: 4px solid var(--c-accent);
    border-radius: 8px;
    padding: var(--sp-3) var(--sp-4);
    box-shadow: 0 12px 30px rgba(225, 29, 42, 0.18);
    display: flex; flex-direction: column; gap: var(--sp-3);
    font-size: var(--font-size-sm);
  }
  .text { line-height: 1.5; }
  .text code {
    font-family: var(--font-mono); font-size: 0.9em;
    background: var(--c-accent-fade); color: var(--c-accent);
    padding: 1px 6px; border-radius: 3px;
  }
  .actions { display: flex; gap: var(--sp-2); }
  .actions button {
    padding: var(--sp-2) var(--sp-3); border-radius: 4px; cursor: pointer; font-size: var(--font-size-sm);
    border: 1px solid var(--c-border);
  }
  .actions .primary { background: var(--c-accent); color: white; border-color: var(--c-accent); font-weight: 600; }
  .actions .primary:disabled { opacity: 0.6; cursor: not-allowed; }
  .actions .ghost { background: transparent; color: var(--c-text-muted); }
  .actions .ghost:hover { background: var(--c-accent-fade); color: var(--c-accent); }
  .err { color: #C8121E; font-size: var(--font-size-xs); }
</style>
