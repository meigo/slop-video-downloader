<script lang="ts">
  import Check from "@lucide/svelte/icons/check";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import Terminal from "@lucide/svelte/icons/terminal";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import X from "@lucide/svelte/icons/x";
  import type { DepsStatus } from "$lib/types";

  interface Props {
    deps: DepsStatus;
    onRecheck: () => void;
  }

  let { deps, onRecheck }: Props = $props();

  function toolLine(found: boolean, path: string | null): string {
    if (found && path) return `found at ${path}`;
    if (found) return "found";
    return "not found";
  }
</script>

<div class="missing-deps">
  <div class="card">
    <h1>
      <TriangleAlert size={22} strokeWidth={2} aria-hidden="true" />
      <span>Missing tools</span>
    </h1>
    <p>This app needs yt-dlp and ffmpeg on your PATH.</p>

    <pre class="hint">
      <Terminal size={16} strokeWidth={2} class="hint-icon" aria-hidden="true" />
      <code>brew install yt-dlp ffmpeg</code>
    </pre>

    <ul class="tools">
      <li class:ok={deps.ytdlp} class:bad={!deps.ytdlp}>
        {#if deps.ytdlp}
          <Check size={16} strokeWidth={2.25} aria-hidden="true" />
        {:else}
          <X size={16} strokeWidth={2.25} aria-hidden="true" />
        {/if}
        <span>
          <strong>yt-dlp:</strong>
          {toolLine(deps.ytdlp, deps.ytdlp_path)}
        </span>
      </li>
      <li class:ok={deps.ffmpeg} class:bad={!deps.ffmpeg}>
        {#if deps.ffmpeg}
          <Check size={16} strokeWidth={2.25} aria-hidden="true" />
        {:else}
          <X size={16} strokeWidth={2.25} aria-hidden="true" />
        {/if}
        <span>
          <strong>ffmpeg:</strong>
          {toolLine(deps.ffmpeg, deps.ffmpeg_path)}
        </span>
      </li>
    </ul>

    <button type="button" onclick={onRecheck}>
      <RefreshCw size={16} strokeWidth={2} aria-hidden="true" />
      <span>Recheck</span>
    </button>
  </div>
</div>

<style>
  .missing-deps {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
  }

  .card {
    max-width: 28rem;
    width: 100%;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 1.75rem 2rem;
  }

  h1 {
    margin: 0 0 0.5rem;
    font-size: 1.35rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--danger);
  }

  p {
    margin: 0 0 1rem;
    color: var(--muted);
  }

  .hint {
    margin: 0 0 1.25rem;
    padding: 0.75rem 1rem;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.9rem;
    overflow-x: auto;
    display: flex;
    align-items: center;
    gap: 0.55rem;
  }

  .hint :global(.hint-icon) {
    flex-shrink: 0;
    color: var(--muted);
  }

  .hint code {
    font: inherit;
  }

  .tools {
    list-style: none;
    margin: 0 0 1.5rem;
    padding: 0;
    color: var(--muted);
    font-size: 0.95rem;
  }

  .tools li {
    margin-bottom: 0.5rem;
    display: flex;
    align-items: flex-start;
    gap: 0.45rem;
  }

  .tools li.ok {
    color: #7dcea0;
  }

  .tools li.bad {
    color: var(--danger);
  }

  .tools strong {
    color: var(--text);
  }

  button {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
  }
</style>
