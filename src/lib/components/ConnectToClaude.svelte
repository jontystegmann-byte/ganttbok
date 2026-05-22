<script lang="ts">
  import { onMount } from 'svelte';
  import type { ClaudeDetection, ClaudeSurface } from '../types';
  import { detectClaudeSurfaces, connectToClaude, disconnectFromClaude } from '../ipc';

  let surfaces: ClaudeDetection[] = $state([]);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let lastRefreshed = $state<Date | null>(null);

  async function refresh() {
    busy = true;
    error = null;
    try {
      const result = await detectClaudeSurfaces();
      surfaces = result.surfaces;
      lastRefreshed = new Date();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function connectAll() {
    const targets = surfaces.filter((s) => s.config_exists).map((s) => s.surface);
    if (targets.length === 0) return;
    busy = true;
    error = null;
    try {
      const result = await connectToClaude(targets);
      surfaces = result.surfaces;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function disconnectAll() {
    const targets = surfaces.filter((s) => s.blikplan_connected).map((s) => s.surface);
    if (targets.length === 0) return;
    busy = true;
    error = null;
    try {
      const result = await disconnectFromClaude(targets);
      surfaces = result.surfaces;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  onMount(refresh);

  let allConnected = $derived(
    surfaces.length > 0 && surfaces.filter((s) => s.config_exists).every((s) => s.blikplan_connected)
  );
  let anyConnected = $derived(surfaces.some((s) => s.blikplan_connected));
  let noneDetected = $derived(surfaces.length > 0 && surfaces.every((s) => !s.config_exists));
</script>

<div class="connect-claude">
  <div class="header">
    <h3>
      Connect to Claude
      <span class="beta">beta</span>
    </h3>
    <button class="refresh" onclick={refresh} disabled={busy} title="Refresh detection">↻</button>
  </div>

  <p class="hint">
    Let Claude Code or Claude Desktop read your Blik Plan schedule and propose
    updates from meeting transcripts. Proposals appear in the Inbox panel for
    you to accept or reject — Claude never writes directly.
  </p>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if noneDetected}
    <p class="empty">No Claude installation detected. Install Claude Code or Claude Desktop, then click refresh.</p>
  {:else}
    <ul class="surfaces">
      {#each surfaces as s (s.surface)}
        <li class:detected={s.config_exists}>
          <span class="checkbox" aria-hidden="true">
            {#if s.blikplan_connected}✓{:else if s.config_exists}·{:else}—{/if}
          </span>
          <span class="name">{s.display_name}</span>
          <span class="status">
            {#if s.blikplan_connected}
              connected
            {:else if s.config_exists}
              not connected
            {:else}
              not detected
            {/if}
          </span>
        </li>
      {/each}
    </ul>
  {/if}

  <div class="actions">
    {#if allConnected}
      <button class="disconnect" onclick={disconnectAll} disabled={busy}>Disconnect</button>
    {:else if anyConnected}
      <button class="connect" onclick={connectAll} disabled={busy}>Update connection</button>
      <button class="disconnect" onclick={disconnectAll} disabled={busy}>Disconnect</button>
    {:else}
      <button
        class="connect primary"
        onclick={connectAll}
        disabled={busy || noneDetected}
      >Connect</button>
    {/if}
  </div>

  {#if allConnected}
    <p class="next-steps">
      Restart Claude Desktop and start a new Claude Code session. Then try:
      <em>"What's on my Blik Plan schedule this week?"</em>
    </p>
  {/if}

  {#if lastRefreshed}
    <p class="last-refreshed">Last checked: {lastRefreshed.toLocaleTimeString()}</p>
  {/if}
</div>

<style>
  .connect-claude { display: flex; flex-direction: column; gap: 0.75rem; }
  .header { display: flex; align-items: center; justify-content: space-between; }
  .header h3 { margin: 0; font-size: 0.95rem; }
  .beta {
    background: var(--accent, #ff8c00); color: white;
    font-size: 0.65rem; padding: 0.1rem 0.35rem; border-radius: 4px;
    text-transform: uppercase; letter-spacing: 0.05em; margin-left: 0.4rem;
    vertical-align: middle;
  }
  .refresh {
    background: none; border: 1px solid var(--border, #ccc); border-radius: 4px;
    width: 1.6rem; height: 1.6rem; cursor: pointer; font-size: 0.9rem;
  }
  .refresh:disabled { opacity: 0.4; cursor: wait; }
  .hint { font-size: 0.8rem; color: var(--muted, #666); margin: 0; }
  .surfaces { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 0.3rem; }
  .surfaces li { display: grid; grid-template-columns: 1.2rem 1fr auto; gap: 0.5rem; align-items: center; font-size: 0.85rem; padding: 0.3rem 0.5rem; border: 1px solid var(--border, #eee); border-radius: 4px; }
  .surfaces li.detected { border-color: var(--border-strong, #ccc); }
  .checkbox { font-family: monospace; text-align: center; }
  .status { color: var(--muted, #666); font-size: 0.75rem; }
  .actions { display: flex; gap: 0.5rem; }
  .actions button { padding: 0.4rem 0.8rem; border-radius: 4px; border: 1px solid var(--border, #ccc); cursor: pointer; font-size: 0.85rem; background: white; }
  .actions button.primary { background: var(--accent, #ff8c00); color: white; border-color: var(--accent, #ff8c00); }
  .actions button:disabled { opacity: 0.5; cursor: not-allowed; }
  .next-steps { font-size: 0.8rem; padding: 0.5rem; background: var(--accent-soft, #fff4e6); border-radius: 4px; margin: 0; }
  .next-steps em { color: var(--accent, #ff8c00); }
  .error { font-size: 0.8rem; color: #b00020; background: #fee; padding: 0.4rem; border-radius: 4px; }
  .empty { font-size: 0.8rem; color: var(--muted, #666); font-style: italic; }
  .last-refreshed { font-size: 0.7rem; color: var(--muted, #999); margin: 0; }
</style>
