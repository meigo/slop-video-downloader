<script lang="ts">
  import Download from "@lucide/svelte/icons/download";
  import Link2 from "@lucide/svelte/icons/link-2";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";

  interface Props {
    url: string;
    busy?: boolean;
    onFetch: () => void;
  }

  let { url = $bindable(), busy = false, onFetch }: Props = $props();

  function submit(event: Event) {
    event.preventDefault();
    if (!busy) onFetch();
  }
</script>

<form class="url-bar" onsubmit={submit}>
  <div class="input-wrap">
    <Link2 class="lead-icon" size={16} strokeWidth={2} aria-hidden="true" />
    <input
      type="url"
      placeholder="Paste a YouTube, Vimeo, or X URL…"
      bind:value={url}
      disabled={busy}
      aria-label="Video URL"
    />
  </div>
  <button type="submit" disabled={busy}>
    {#if busy}
      <LoaderCircle class="spin" size={16} strokeWidth={2} aria-hidden="true" />
      <span>Fetching…</span>
    {:else}
      <Download size={16} strokeWidth={2} aria-hidden="true" />
      <span>Fetch</span>
    {/if}
  </button>
</form>

<style>
  .url-bar {
    display: flex;
    gap: 0.5rem;
    width: 100%;
  }

  .input-wrap {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
  }

  .input-wrap :global(.lead-icon) {
    position: absolute;
    left: 0.75rem;
    color: var(--muted);
    pointer-events: none;
  }

  input {
    width: 100%;
    padding-left: 2.25rem;
  }

  button {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
  }

  button :global(.spin) {
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
