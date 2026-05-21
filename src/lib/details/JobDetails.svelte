<script lang="ts">
  import { store } from '../store.svelte';

  const job = $derived(store.currentJob);
</script>

{#if job}
  <div class="job-details">
    <header>
      <h2>{job.name}</h2>
      {#if job.client}<p class="meta">Client: {job.client}</p>{/if}
      {#if job.address}<p class="meta">{job.address}</p>{/if}
      <p class="meta">Start: {job.project_start_date}</p>
    </header>

    <section>
      <h3>Public holidays</h3>
      <label class="toggle">
        <input
          type="checkbox"
          checked={job.holidays_block_work}
          onchange={(e) => store.setJobHolidaysBlockWork((e.currentTarget as HTMLInputElement).checked)}
        />
        <span class="label">SA holidays count as no-work days</span>
      </label>
      <p class="hint">
        {#if job.holidays_block_work}
          Bars step around public holidays — work doesn't happen on these days.
        {:else}
          Public holidays are visual only — work continues through them.
        {/if}
        New jobs will default to this setting.
      </p>
    </section>

    <p class="hint">Click a bar to edit its details.</p>
  </div>
{/if}

<style>
  .job-details { padding: var(--sp-4); display: flex; flex-direction: column; gap: var(--sp-4); }
  header h2 { margin: 0 0 var(--sp-2); font-size: var(--font-size-lg); }
  .meta { margin: 0; color: var(--c-text-muted); font-size: var(--font-size-sm); }
  section h3 { margin: 0 0 var(--sp-2); font-size: var(--font-size-sm); text-transform: uppercase; letter-spacing: 0.05em; color: var(--c-text-muted); }
  .toggle { display: flex; align-items: center; gap: var(--sp-2); cursor: pointer; font-size: var(--font-size-sm); }
  .toggle input { width: 16px; height: 16px; cursor: pointer; }
  .hint { color: var(--c-text-muted); font-size: var(--font-size-xs); margin: var(--sp-2) 0 0; line-height: 1.4; }
</style>
