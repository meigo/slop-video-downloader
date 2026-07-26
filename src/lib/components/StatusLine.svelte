<script lang="ts">
  import CircleAlert from "@lucide/svelte/icons/circle-alert";
  import Info from "@lucide/svelte/icons/info";

  interface Props {
    status: string;
    error: string | null;
    title?: string | null;
    durationLabel?: string | null;
  }

  let { status, error, title = null, durationLabel = null }: Props = $props();
</script>

<div class="status-line" class:error={!!error} role="status">
  {#if error}
    <CircleAlert size={15} strokeWidth={2} aria-hidden="true" />
    <span class="err">{error}</span>
  {:else}
    <Info size={15} strokeWidth={2} class="info-icon" aria-hidden="true" />
    <span class="meta">
      {#if title}
        <strong>{title}</strong>
        {#if durationLabel}
          <span class="sep">·</span>
          <span>{durationLabel}</span>
        {/if}
        <span class="sep">·</span>
      {/if}
      <span class="status">{status}</span>
    </span>
  {/if}
</div>

<style>
  .status-line {
    min-height: 1.5rem;
    font-size: 0.9rem;
    color: var(--muted);
    display: flex;
    align-items: flex-start;
    gap: 0.4rem;
  }

  .status-line.error {
    color: var(--danger);
  }

  .status-line :global(.info-icon) {
    flex-shrink: 0;
    margin-top: 0.15rem;
    opacity: 0.75;
  }

  .status-line :global(svg) {
    flex-shrink: 0;
    margin-top: 0.15rem;
  }

  .meta {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    align-items: baseline;
  }

  .meta strong {
    color: var(--text);
    font-weight: 600;
  }

  .sep {
    opacity: 0.5;
  }

  .err {
    color: var(--danger);
  }
</style>
