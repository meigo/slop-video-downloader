<script lang="ts">
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
    <span class="err">{error}</span>
  {:else}
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
  }

  .status-line.error {
    color: var(--danger);
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
