<script lang="ts">
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
  <input
    type="url"
    placeholder="Paste a YouTube URL…"
    bind:value={url}
    disabled={busy}
    aria-label="YouTube URL"
  />
  <button type="submit" disabled={busy}>
    {busy ? "Fetching…" : "Fetch"}
  </button>
</form>

<style>
  .url-bar {
    display: flex;
    gap: 0.5rem;
    width: 100%;
  }

  input {
    flex: 1;
    min-width: 0;
  }

  button {
    flex-shrink: 0;
  }
</style>
