<script lang="ts">
  import { onMount } from 'svelte';
  import { check, type Update } from '@tauri-apps/plugin-updater';
  import { relaunch } from '@tauri-apps/plugin-process';
  import { getVersion } from '@tauri-apps/api/app';

  type Phase = 'idle' | 'checking' | 'no-update' | 'available' | 'downloading' | 'installed' | 'error';

  let phase = $state<Phase>('idle');
  let currentVersion = $state('');
  let pending = $state<Update | null>(null);
  let downloaded = $state(0);
  let contentLength = $state(0);
  let errorMsg = $state('');
  let modalOpen = $state(false);

  async function silentCheck() {
    try {
      const u = await check();
      if (u) {
        pending = u;
        phase = 'available';
      } else {
        phase = 'no-update';
      }
    } catch (e) {
      phase = 'error';
      errorMsg = String(e);
    }
  }

  async function manualCheck() {
    phase = 'checking';
    modalOpen = true;
    try {
      const u = await check();
      if (u) {
        pending = u;
        phase = 'available';
      } else {
        phase = 'no-update';
      }
    } catch (e) {
      phase = 'error';
      errorMsg = String(e);
    }
  }

  async function installAndRestart() {
    if (!pending) return;
    phase = 'downloading';
    downloaded = 0;
    contentLength = 0;
    try {
      await pending.downloadAndInstall((event) => {
        if (event.event === 'Started') contentLength = event.data.contentLength ?? 0;
        else if (event.event === 'Progress') downloaded += event.data.chunkLength;
      });
      phase = 'installed';
      await relaunch();
    } catch (e) {
      phase = 'error';
      errorMsg = String(e);
    }
  }

  onMount(async () => {
    currentVersion = await getVersion();
    setTimeout(silentCheck, 3000);
  });

  const pct = $derived(contentLength > 0 ? Math.min(100, Math.round((downloaded / contentLength) * 100)) : 0);
</script>

<button class="version-tag phase-{phase}" onclick={manualCheck} title="Check for updates">
  {#if phase === 'available'}
    Update to v{pending?.version} →
  {:else}
    v{currentVersion}
  {/if}
</button>

{#if modalOpen}
  <div class="overlay" role="dialog" aria-modal="true">
    <div class="modal">
      <header>
        <h2>
          {#if phase === 'checking'}Checking for updates…
          {:else if phase === 'no-update'}You're up to date
          {:else if phase === 'available'}Update available
          {:else if phase === 'downloading'}Installing update…
          {:else if phase === 'installed'}Restarting…
          {:else if phase === 'error'}Update error
          {/if}
        </h2>
        <button class="close" onclick={() => (modalOpen = false)} disabled={phase === 'downloading'}>×</button>
      </header>

      <div class="body">
        {#if phase === 'checking'}
          <p>Looking for a newer version…</p>
        {:else if phase === 'no-update'}
          <p>You're running the latest version (v{currentVersion}).</p>
        {:else if phase === 'available' && pending}
          <p>A new version is available: <strong>v{pending.version}</strong> (current: v{currentVersion}).</p>
          {#if pending.body}
            <h3>Release notes</h3>
            <pre class="notes">{pending.body}</pre>
          {/if}
          <p class="hint">Your jobs and data will be preserved. The app will restart after install.</p>
        {:else if phase === 'downloading'}
          <p>Downloading v{pending?.version}…</p>
          <div class="progress"><div class="bar" style="width: {pct}%"></div></div>
          <p class="pct">{pct}%</p>
        {:else if phase === 'installed'}
          <p>Update installed. Restarting…</p>
        {:else if phase === 'error'}
          <p>Something went wrong:</p>
          <pre class="err">{errorMsg}</pre>
        {/if}
      </div>

      <footer>
        {#if phase === 'available'}
          <button class="primary" onclick={installAndRestart}>Install & Restart</button>
          <button onclick={() => (modalOpen = false)}>Later</button>
        {:else if phase !== 'downloading'}
          <button onclick={() => (modalOpen = false)}>Close</button>
        {/if}
      </footer>
    </div>
  </div>
{/if}

<style>
  .version-tag {
    position: fixed;
    bottom: 8px;
    left: 12px;
    z-index: 5;
    background: transparent;
    border: none;
    font-size: var(--font-size-xs);
    color: var(--c-text-muted);
    cursor: pointer;
    padding: var(--sp-1) var(--sp-2);
    border-radius: 4px;
    font-family: var(--font-mono);
  }
  .version-tag:hover { background: var(--c-accent-fade); color: var(--c-accent); }
  .version-tag.phase-available {
    color: var(--c-accent);
    background: var(--c-accent-fade);
    font-weight: 600;
    font-family: inherit;
  }

  .overlay {
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.4);
    z-index: 100;
    display: flex; align-items: center; justify-content: center;
  }
  .modal {
    background: var(--c-panel);
    border: 1px solid var(--c-border);
    border-radius: 8px;
    width: min(520px, 90vw);
    max-height: 80vh;
    display: flex; flex-direction: column;
    box-shadow: 0 20px 60px rgba(0,0,0,0.2);
  }
  header {
    display: flex; justify-content: space-between; align-items: center;
    padding: var(--sp-3) var(--sp-4);
    border-bottom: 1px solid var(--c-border);
  }
  header h2 { margin: 0; font-size: var(--font-size-md); }
  .close {
    background: none; border: none; font-size: 22px; cursor: pointer; color: var(--c-text-muted);
    line-height: 1; padding: 0 var(--sp-2);
  }
  .close:disabled { opacity: 0.3; cursor: not-allowed; }
  .body { padding: var(--sp-4); overflow-y: auto; }
  .body p { margin: 0 0 var(--sp-3); }
  .body h3 { font-size: var(--font-size-sm); margin: var(--sp-3) 0 var(--sp-2); }
  .notes, .err {
    background: var(--c-bg);
    border: 1px solid var(--c-border);
    border-radius: 4px;
    padding: var(--sp-2) var(--sp-3);
    font-size: var(--font-size-xs);
    white-space: pre-wrap;
    max-height: 200px;
    overflow-y: auto;
  }
  .err { color: #DC2626; }
  .hint { font-size: var(--font-size-xs); color: var(--c-text-muted); }
  .progress {
    height: 8px; background: var(--c-bg); border-radius: 4px; overflow: hidden;
    border: 1px solid var(--c-border);
  }
  .bar { height: 100%; background: var(--c-accent); transition: width 0.2s; }
  .pct { text-align: center; font-family: var(--font-mono); font-size: var(--font-size-xs); margin-top: var(--sp-2); }
  footer {
    display: flex; gap: var(--sp-2); justify-content: flex-end;
    padding: var(--sp-3) var(--sp-4);
    border-top: 1px solid var(--c-border);
  }
  footer button {
    padding: var(--sp-2) var(--sp-3);
    border: 1px solid var(--c-border);
    background: var(--c-panel);
    border-radius: 4px;
    cursor: pointer;
    font-size: var(--font-size-sm);
  }
  footer button.primary {
    background: var(--c-accent);
    color: white;
    border-color: var(--c-accent);
  }
  footer button:hover { background: var(--c-accent-fade); }
  footer button.primary:hover { opacity: 0.9; }
</style>
