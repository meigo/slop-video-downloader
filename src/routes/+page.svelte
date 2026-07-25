<script lang="ts">
  import { onMount } from "svelte";
  import ExportPanel from "$lib/components/ExportPanel.svelte";
  import MissingDeps from "$lib/components/MissingDeps.svelte";
  import StatusLine from "$lib/components/StatusLine.svelte";
  import UrlBar from "$lib/components/UrlBar.svelte";
  import {
    checkDeps,
    defaultSaveDir,
    fetchMetadata,
    loadSettings,
    resolvePreview,
  } from "$lib/tauri";
  import { formatTimestamp } from "$lib/time";
  import type { AppSettings, DepsStatus, PreviewResult, VideoMeta } from "$lib/types";
  import { isYouTubeUrl } from "$lib/youtube";

  let deps = $state<DepsStatus | null>(null);
  let checkingDeps = $state(true);

  let url = $state("");
  let meta = $state<VideoMeta | null>(null);
  let preview = $state<PreviewResult | null>(null);
  let status = $state("Paste a YouTube URL and click Fetch");
  let error = $state<string | null>(null);
  let busy = $state(false);

  let inPoint = $state(0);
  let outPoint = $state(0);
  let maxHeight = $state<number | null>(1080);
  let includeAudio = $state(true);
  let savePath = $state("");

  const depsOk = $derived(!!deps && deps.ytdlp && deps.ffmpeg);
  const duration = $derived(meta?.duration_secs ?? 0);
  const canExport = $derived(
    !!meta && !!preview && outPoint > inPoint && savePath.trim().length > 0 && !busy,
  );

  async function refreshDeps() {
    checkingDeps = true;
    error = null;
    try {
      deps = await checkDeps();
    } catch (e) {
      deps = { ytdlp: false, ffmpeg: false, ytdlp_path: null, ffmpeg_path: null };
      error = e instanceof Error ? e.message : String(e);
    } finally {
      checkingDeps = false;
    }
  }

  async function initSettings() {
    try {
      const settings: AppSettings = await loadSettings();
      maxHeight = settings.max_height;
      includeAudio = settings.include_audio;
      if (settings.last_save_dir) {
        savePath = settings.last_save_dir;
      } else {
        savePath = await defaultSaveDir();
      }
    } catch {
      try {
        savePath = await defaultSaveDir();
      } catch {
        savePath = "";
      }
    }
  }

  async function onFetch() {
    error = null;
    const trimmed = url.trim();

    if (!isYouTubeUrl(trimmed)) {
      error = "Enter a valid YouTube URL";
      status = "Idle";
      return;
    }

    busy = true;
    meta = null;
    preview = null;
    inPoint = 0;
    outPoint = 0;

    try {
      status = "Fetching metadata…";
      meta = await fetchMetadata(trimmed);

      status = "Resolving preview…";
      preview = await resolvePreview(trimmed);

      inPoint = 0;
      outPoint = meta.duration_secs;

      if (preview.note) {
        status = `Ready — ${preview.note}`;
      } else {
        status = "Ready";
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      error = msg;
      status = "Error";
      meta = null;
      preview = null;
      inPoint = 0;
      outPoint = 0;
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    void (async () => {
      await refreshDeps();
      if (deps && deps.ytdlp && deps.ffmpeg) {
        await initSettings();
      }
    })();
  });
</script>

{#if checkingDeps && !deps}
  <div class="boot">Checking tools…</div>
{:else if deps && !depsOk}
  <MissingDeps
    {deps}
    onRecheck={() => {
      void (async () => {
        await refreshDeps();
        if (deps && deps.ytdlp && deps.ffmpeg) {
          await initSettings();
        }
      })();
    }}
  />
{:else}
  <div class="app">
    <header class="top">
      <UrlBar bind:url {busy} onFetch={onFetch} />
      <StatusLine
        {status}
        {error}
        title={meta?.title ?? null}
        durationLabel={meta ? formatTimestamp(meta.duration_secs) : null}
      />
    </header>

    <div class="main">
      <section class="preview" aria-label="Video preview">
        {#if preview}
          <!-- Placeholder until Task 9 wires the real player -->
          <div class="preview-placeholder ready">
            <p>Preview ready ({preview.mode})</p>
            <p class="preview-src" title={preview.url_or_path}>{preview.url_or_path}</p>
            <video controls={false} preload="none" aria-hidden="true"></video>
          </div>
        {:else}
          <div class="preview-placeholder">
            <p>Video preview</p>
            <p class="muted">Fetch a URL to load a preview</p>
          </div>
        {/if}
      </section>

      <ExportPanel
        {inPoint}
        {outPoint}
        {duration}
        bind:maxHeight
        bind:includeAudio
        bind:savePath
        {canExport}
      />
    </div>

    <footer class="timeline-placeholder" aria-label="Timeline">
      <span class="muted">Timeline (Task 9)</span>
    </footer>
  </div>
{/if}

<style>
  .boot {
    min-height: 100vh;
    display: grid;
    place-items: center;
    color: var(--muted);
  }

  .app {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 1rem 1.25rem 1.25rem;
    max-width: 1100px;
    margin: 0 auto;
  }

  .top {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .main {
    display: grid;
    grid-template-columns: minmax(0, 1.6fr) minmax(220px, 0.9fr);
    gap: 0.75rem;
    flex: 1;
    min-height: 280px;
  }

  .preview {
    min-height: 240px;
  }

  .preview-placeholder {
    height: 100%;
    min-height: 240px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.35rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 1rem;
    text-align: center;
  }

  .preview-placeholder.ready {
    align-items: stretch;
  }

  .preview-placeholder video {
    display: none;
  }

  .preview-src {
    margin: 0;
    font-size: 0.75rem;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .muted {
    color: var(--muted);
    font-size: 0.9rem;
  }

  .timeline-placeholder {
    border: 1px dashed var(--border);
    border-radius: 10px;
    padding: 0.85rem 1rem;
    text-align: center;
  }

  @media (max-width: 720px) {
    .main {
      grid-template-columns: 1fr;
    }
  }
</style>
