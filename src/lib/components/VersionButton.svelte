<script lang="ts">
  import { onMount } from 'svelte';
  import { check, type Update } from '@tauri-apps/plugin-updater';
  import { relaunch } from '@tauri-apps/plugin-process';
  import { getVersion } from '@tauri-apps/api/app';

  type UpdPhase = 'idle' | 'checking' | 'no-update' | 'available' | 'downloading' | 'installed' | 'error';
  let updPhase = $state<UpdPhase>('idle');
  let appVersion = $state('');
  let pending = $state<Update | null>(null);
  let downloaded = $state(0);
  let contentLength = $state(0);
  let updateErr = $state('');
  let updateModalOpen = $state(false);

  async function silentUpdateCheck() {
    try {
      const u = await check();
      if (u) { pending = u; updPhase = 'available'; }
      else updPhase = 'no-update';
    } catch (e) { updPhase = 'error'; updateErr = String(e); }
  }
  async function manualUpdateCheck() {
    updPhase = 'checking'; updateModalOpen = true;
    try {
      const u = await check();
      if (u) { pending = u; updPhase = 'available'; }
      else updPhase = 'no-update';
    } catch (e) { updPhase = 'error'; updateErr = String(e); }
  }
  async function installAndRestart() {
    if (!pending) return;
    updPhase = 'downloading'; downloaded = 0; contentLength = 0;
    try {
      await pending.downloadAndInstall((event) => {
        if (event.event === 'Started') contentLength = event.data.contentLength ?? 0;
        else if (event.event === 'Progress') downloaded += event.data.chunkLength;
      });
      updPhase = 'installed';
      await relaunch();
    } catch (e) { updPhase = 'error'; updateErr = String(e); }
  }
  const pct = $derived(contentLength > 0 ? Math.min(100, Math.round((downloaded / contentLength) * 100)) : 0);

  onMount(async () => {
    appVersion = await getVersion();
    setTimeout(silentUpdateCheck, 3000);
  });
</script>

<button
  class="version-btn"
  class:has-update={updPhase === 'available'}
  onclick={manualUpdateCheck}
  title="Check for updates"
>
  {#if updPhase === 'available'}
    Update v{pending?.version} →
  {:else}
    v{appVersion}
  {/if}
</button>

{#if updateModalOpen}
  <div class="overlay" role="dialog" aria-modal="true">
    <div class="modal">
      <header>
        <h2>
          {#if updPhase === 'checking'}Checking for updates…
          {:else if updPhase === 'no-update'}You're up to date
          {:else if updPhase === 'available'}Update available
          {:else if updPhase === 'downloading'}Installing update…
          {:else if updPhase === 'installed'}Restarting…
          {:else if updPhase === 'error'}Update error
          {/if}
        </h2>
        <button class="close" onclick={() => (updateModalOpen = false)} disabled={updPhase === 'downloading'}>×</button>
      </header>
      <div class="body">
        {#if updPhase === 'checking'}<p>Looking for a newer version…</p>
        {:else if updPhase === 'no-update'}<p>You're running the latest version (v{appVersion}).</p>
        {:else if updPhase === 'available' && pending}
          <p>A new version is available: <strong>v{pending.version}</strong> (current: v{appVersion}).</p>
          {#if pending.body}<h3>Release notes</h3><pre class="notes">{pending.body}</pre>{/if}
          <p class="hint">Your jobs and data are preserved. The app will restart after install.</p>
        {:else if updPhase === 'downloading'}
          <p>Downloading v{pending?.version}…</p>
          <div class="progress"><div class="bar" style="width: {pct}%"></div></div>
          <p class="pct">{pct}%</p>
        {:else if updPhase === 'error'}<pre class="err">{updateErr}</pre>{/if}
      </div>
      {#if updPhase === 'available'}
        <footer>
          <button onclick={() => (updateModalOpen = false)}>Later</button>
          <button class="primary" onclick={installAndRestart}>Install &amp; restart</button>
        </footer>
      {/if}
    </div>
  </div>
{/if}

<style>
  .version-btn {
    background: transparent; border: none;
    padding: var(--sp-1) var(--sp-2);
    cursor: pointer; border-radius: 4px;
    color: var(--c-text-muted);
    font-size: var(--font-size-xs);
    font-family: var(--font-mono);
    width: 100%; text-align: left;
  }
  .version-btn:hover { background: var(--c-accent-fade); color: var(--c-accent); }
  .version-btn.has-update {
    color: var(--c-accent); background: var(--c-accent-fade);
    font-weight: 600; font-family: inherit;
  }

  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.4); z-index: 100; display: flex; align-items: center; justify-content: center; }
  .modal { background: var(--c-panel); border: 1px solid var(--c-border); border-radius: 8px; width: min(520px, 90vw); max-height: 80vh; display: flex; flex-direction: column; box-shadow: 0 20px 60px rgba(0,0,0,0.2); }
  .modal header { display: flex; justify-content: space-between; align-items: center; padding: var(--sp-3) var(--sp-4); border-bottom: 1px solid var(--c-border); }
  .modal header h2 { margin: 0; font-size: var(--font-size-base); }
  .close { background: none; border: none; font-size: 22px; cursor: pointer; color: var(--c-text-muted); line-height: 1; padding: 0 var(--sp-2); }
  .close:disabled { opacity: 0.3; cursor: not-allowed; }
  .body { padding: var(--sp-4); overflow-y: auto; }
  .body p { margin: 0 0 var(--sp-3); }
  .body h3 { font-size: var(--font-size-sm); margin: var(--sp-3) 0 var(--sp-2); }
  .notes, .err { background: var(--c-bg); border: 1px solid var(--c-border); border-radius: 4px; padding: var(--sp-2) var(--sp-3); font-size: var(--font-size-xs); white-space: pre-wrap; max-height: 200px; overflow-y: auto; }
  .err { color: #DC2626; }
  .hint { font-size: var(--font-size-xs); color: var(--c-text-muted); }
  .progress { height: 8px; background: var(--c-bg); border-radius: 4px; overflow: hidden; border: 1px solid var(--c-border); }
  .bar { height: 100%; background: var(--c-accent); transition: width 0.2s; }
  .pct { text-align: center; font-family: var(--font-mono); font-size: var(--font-size-xs); margin-top: var(--sp-2); }
  .modal footer { display: flex; gap: var(--sp-2); justify-content: flex-end; padding: var(--sp-3) var(--sp-4); border-top: 1px solid var(--c-border); }
  .modal footer button { padding: var(--sp-2) var(--sp-3); border: 1px solid var(--c-border); background: var(--c-panel); border-radius: 4px; cursor: pointer; font-size: var(--font-size-sm); }
  .modal footer button.primary { background: var(--c-accent); color: white; border-color: var(--c-accent); }
  .modal footer button:hover { background: var(--c-accent-fade); }
</style>
