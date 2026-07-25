<script lang="ts">
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
    <h1>Missing tools</h1>
    <p>This app needs yt-dlp and ffmpeg on your PATH.</p>

    <pre class="hint">brew install yt-dlp ffmpeg</pre>

    <ul class="tools">
      <li>
        <strong>yt-dlp:</strong>
        {toolLine(deps.ytdlp, deps.ytdlp_path)}
      </li>
      <li>
        <strong>ffmpeg:</strong>
        {toolLine(deps.ffmpeg, deps.ffmpeg_path)}
      </li>
    </ul>

    <button type="button" onclick={onRecheck}>Recheck</button>
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
  }

  .tools {
    list-style: none;
    margin: 0 0 1.5rem;
    padding: 0;
    color: var(--muted);
    font-size: 0.95rem;
  }

  .tools li {
    margin-bottom: 0.35rem;
  }

  .tools strong {
    color: var(--text);
  }
</style>
