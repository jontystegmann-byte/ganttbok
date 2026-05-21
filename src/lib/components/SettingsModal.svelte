<script lang="ts">
  import { store } from '../store.svelte';

  let open = $state(false);

  function close() { open = false; }
</script>

<button class="cog" onclick={() => (open = !open)} title="Settings" aria-label="Settings">
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="3"/>
    <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
  </svg>
</button>

{#if open}
  <div class="backdrop" onclick={close} role="presentation"></div>
  <div class="popover" role="dialog" aria-label="Settings">
    <header>
      <h2>Settings</h2>
      <button class="close" onclick={close} aria-label="Close">×</button>
    </header>

    <section>
      <h3>Duration display</h3>
      <div class="seg-toggle">
        <button
          class:active={store.durationUnit === 'weeks'}
          onclick={() => { if (store.durationUnit !== 'weeks') store.toggleDurationUnit(); }}
        >Weeks</button>
        <button
          class:active={store.durationUnit === 'days'}
          onclick={() => { if (store.durationUnit !== 'days') store.toggleDurationUnit(); }}
        >Days</button>
      </div>
    </section>

    <section>
      <h3>Weekends</h3>
      <label class="toggle">
        <input
          type="checkbox"
          checked={store.includeWeekends}
          onchange={(e) => store.setIncludeWeekends((e.currentTarget as HTMLInputElement).checked)}
        />
        <span>Show Saturday + Sunday columns</span>
      </label>
      <p class="hint">For projects where you work on weekends.</p>
    </section>

    <section>
      <h3>Zoom — {Math.round(store.uiScale * 100)}%</h3>
      <input
        type="range"
        min="0.75" max="1.5" step="0.05"
        value={store.uiScale}
        oninput={(e) => store.setUiScale(parseFloat((e.currentTarget as HTMLInputElement).value))}
        class="slider"
      />
      <div class="slider-labels"><span>75%</span><span>100%</span><span>150%</span></div>
    </section>

    {#if store.currentJob}
      <section>
        <h3>Job name</h3>
        <input
          type="text"
          class="job-name-input"
          value={store.currentJob.name}
          onblur={(e) => store.renameCurrentJob((e.currentTarget as HTMLInputElement).value)}
          onkeydown={(e) => { if (e.key === 'Enter') (e.currentTarget as HTMLInputElement).blur(); }}
        />
        <h3 style="margin-top: var(--sp-4)">Project start date</h3>
        <input
          type="date"
          class="job-name-input"
          value={store.currentJob.project_start_date}
          onchange={(e) => store.setCurrentJobStartDate((e.currentTarget as HTMLInputElement).value)}
        />
      </section>

      <section>
        <h3>Public holidays — this job</h3>
        <label class="toggle">
          <input
            type="checkbox"
            checked={store.currentJob.holidays_block_work}
            onchange={(e) => store.setJobHolidaysBlockWork((e.currentTarget as HTMLInputElement).checked)}
          />
          <span>Split bars around SA public holidays</span>
        </label>
        <p class="hint">
          {#if store.currentJob.holidays_block_work}
            Bars step around public holidays — the task gets an extra day to compensate.
          {:else}
            Public holidays run through bars uninterrupted (visual hatch only).
          {/if}
          New jobs default to this setting.
        </p>
      </section>
    {/if}
  </div>
{/if}

<style>
  .cog {
    position: fixed; bottom: 8px; left: 76px;
    z-index: 5;
    background: transparent; border: none;
    color: var(--c-text-muted);
    padding: var(--sp-1) var(--sp-2);
    cursor: pointer;
    border-radius: 4px;
    display: flex; align-items: center;
  }
  .cog:hover { background: var(--c-accent-fade); color: var(--c-accent); }

  .backdrop { position: fixed; inset: 0; background: transparent; z-index: 50; }

  .popover {
    position: fixed; bottom: 38px; left: 16px;
    z-index: 51;
    background: var(--c-panel);
    border: 1px solid var(--c-border);
    border-radius: 8px;
    min-width: 320px; max-width: 380px;
    box-shadow: 0 12px 36px rgba(0,0,0,0.18);
    display: flex; flex-direction: column;
  }
  header {
    display: flex; justify-content: space-between; align-items: center;
    padding: var(--sp-3) var(--sp-4);
    border-bottom: 1px solid var(--c-border);
  }
  header h2 { margin: 0; font-size: var(--font-size-md); }
  .close { background: none; border: none; font-size: 22px; line-height: 1; cursor: pointer; color: var(--c-text-muted); padding: 0 var(--sp-1); }
  section { padding: var(--sp-3) var(--sp-4); border-bottom: 1px solid var(--c-border); }
  section:last-child { border-bottom: none; }
  section h3 { margin: 0 0 var(--sp-2); font-size: var(--font-size-xs); text-transform: uppercase; letter-spacing: 0.05em; color: var(--c-text-muted); }

  .seg-toggle { display: flex; gap: 0; border: 1px solid var(--c-border); border-radius: 6px; overflow: hidden; width: max-content; }
  .seg-toggle button {
    background: var(--c-panel); border: none; padding: var(--sp-2) var(--sp-3);
    cursor: pointer; font-size: var(--font-size-sm);
    color: var(--c-text-muted);
  }
  .seg-toggle button.active { background: var(--c-accent); color: white; }
  .seg-toggle button:not(.active):hover { background: var(--c-accent-fade); color: var(--c-accent); }

  .toggle { display: flex; align-items: center; gap: var(--sp-2); cursor: pointer; font-size: var(--font-size-sm); }
  .toggle input { width: 16px; height: 16px; cursor: pointer; }
  .hint { color: var(--c-text-muted); font-size: var(--font-size-xs); margin: var(--sp-2) 0 0; line-height: 1.4; }
  .slider { width: 100%; }
  .slider-labels { display: flex; justify-content: space-between; font-size: var(--font-size-xs); color: var(--c-text-muted); margin-top: var(--sp-1); }
  .job-name-input { width: 100%; padding: var(--sp-2); border: 1px solid var(--c-border); border-radius: 4px; font-size: var(--font-size-sm); }
</style>
