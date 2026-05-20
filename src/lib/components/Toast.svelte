<script lang="ts">
  import { toast } from '../toast.svelte';
</script>

<div class="toast-stack">
  {#each toast.list as t (t.id)}
    <div class="toast {t.kind}">{t.message}</div>
  {/each}
</div>

<style>
  .toast-stack {
    position: fixed; bottom: 32px; left: 50%;
    transform: translateX(-50%);
    z-index: 999;
    display: flex; flex-direction: column; gap: var(--sp-2);
    pointer-events: none;
  }
  .toast {
    padding: var(--sp-2) var(--sp-3);
    border-radius: 4px;
    box-shadow: 0 4px 12px var(--c-shadow);
    font-size: var(--font-size-sm);
    min-width: 280px;
    pointer-events: auto;
  }
  .toast.error { background: #FEE2E2; color: #991B1B; border: 1px solid #FCA5A5; }
  .toast.info  { background: var(--c-accent-fade); color: var(--c-accent); }
</style>
