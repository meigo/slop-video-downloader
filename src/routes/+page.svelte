<script lang="ts">
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onDestroy, onMount } from "svelte";
  import Film from "@lucide/svelte/icons/film";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import ExportPanel from "$lib/components/ExportPanel.svelte";
  import MissingDeps from "$lib/components/MissingDeps.svelte";
  import StatusLine from "$lib/components/StatusLine.svelte";
  import Timeline from "$lib/components/Timeline.svelte";
  import UrlBar from "$lib/components/UrlBar.svelte";
  import VideoPlayer from "$lib/components/VideoPlayer.svelte";
  import {
    checkDeps,
    defaultSaveDir,
    exportClip,
    fetchMetadata,
    loadSettings,
    resolvePreview,
    revealInFolder,
    saveSettings,
  } from "$lib/tauri";
  import { clampRange, formatTimestamp } from "$lib/time";
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
  let exporting = $state(false);

  let inPoint = $state(0);
  let outPoint = $state(0);
  let currentTime = $state(0);
  let maxHeight = $state<number | null>(1080);
  let includeAudio = $state(true);
  let savePath = $state("");

  let player = $state<ReturnType<typeof VideoPlayer> | null>(null);
  let loopSelection = $state(false);
  let forceFileFallbackBusy = $state(false);

  const depsOk = $derived(!!deps && deps.ytdlp && deps.ffmpeg);
  const duration = $derived(meta?.duration_secs ?? 0);
  const canExport = $derived(
    !!meta && !!preview && outPoint > inPoint && savePath.trim().length > 0 && !busy && !exporting,
  );

  let unlistenProgress: UnlistenFn | null = null;

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
    currentTime = 0;
    loopSelection = false;
    forceFileFallbackBusy = false;

    try {
      status = "Fetching metadata…";
      meta = await fetchMetadata(trimmed);

      status = "Resolving preview…";
      preview = await resolvePreview(trimmed);

      inPoint = 0;
      outPoint = meta.duration_secs;
      currentTime = 0;

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

  async function onPreviewError() {
    if (!preview || preview.mode !== "stream" || forceFileFallbackBusy) return;
    const trimmed = url.trim();
    if (!trimmed) return;

    forceFileFallbackBusy = true;
    status = "Stream preview failed — downloading local preview…";
    error = null;
    try {
      preview = await resolvePreview(trimmed, true);
      if (preview.note) {
        status = `Ready — ${preview.note}`;
      } else {
        status = "Ready — local preview file";
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      error = msg;
      status = "Error";
    } finally {
      forceFileFallbackBusy = false;
    }
  }

  function setInFromPlayhead() {
    if (!(duration > 0)) return;
    const { start, end } = clampRange(currentTime, outPoint, duration);
    inPoint = start;
    outPoint = end;
  }

  function setOutFromPlayhead() {
    if (!(duration > 0)) return;
    const { start, end } = clampRange(inPoint, currentTime, duration);
    inPoint = start;
    outPoint = end;
  }

  /**
   * Seek playhead across the full video (0…duration).
   * Selection in/out only affects export / Play selection — not free scrubbing.
   * Manual seeks exit selection-loop mode so scrubbing past out is never blocked.
   */
  function onSeek(t: number) {
    loopSelection = false;
    const max = duration > 0 ? duration : t;
    const clamped = Math.min(Math.max(0, t), max);
    currentTime = clamped;
    player?.seek(clamped);
  }

  /** Fixed skip amounts — standard for most video UIs (YouTube etc. use ~5s). */
  const SKIP_SHORT_SECS = 5;
  const SKIP_LONG_SECS = 10;

  function skipBy(deltaSecs: number) {
    if (!(duration > 0)) return;
    const next = Math.min(Math.max(0, currentTime + deltaSecs), duration);
    onSeek(next);
  }

  function onPlayPause() {
    if (loopSelection && player && !player.isPaused()) {
      loopSelection = false;
      player.pause();
      return;
    }
    loopSelection = false;
    player?.togglePlay();
  }

  function onStop() {
    loopSelection = false;
    player?.pause();
    onSeek(0);
  }

  function onPlaySelection() {
    if (!(outPoint > inPoint)) return;
    loopSelection = true;
    player?.seek(inPoint);
    currentTime = inPoint;
    player?.play();
  }

  /** Navigation only — never start playback (unlike Play selection). */
  function jumpToIn() {
    loopSelection = false;
    onSeek(inPoint);
  }

  function jumpToOut() {
    loopSelection = false;
    onSeek(outPoint);
  }

  // Loop only while "Play selection" is actively playing — never while paused/scrubbing.
  // Selection range must not prevent watching or scrubbing the full video.
  $effect(() => {
    if (!loopSelection) return;
    if (!player || player.isPaused()) return;
    if (currentTime < outPoint - 0.04) return;
    // Keep loop mode (do not go through onSeek, which clears it).
    player.seek(inPoint);
    currentTime = inPoint;
    player.play();
  });

  function onKeyDown(event: KeyboardEvent) {
    const target = event.target as HTMLElement | null;
    const tag = target?.tagName?.toLowerCase();
    if (tag === "input" || tag === "textarea" || tag === "select" || target?.isContentEditable) {
      return;
    }

    if (event.key === " " || event.code === "Space") {
      event.preventDefault();
      onPlayPause();
      return;
    }

    // ← / → skip a fixed step (not % of duration): predictable on short and long clips.
    // Shift+arrow uses a longer step for coarse scrubbing.
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      event.preventDefault();
      const step = event.shiftKey ? SKIP_LONG_SECS : SKIP_SHORT_SECS;
      skipBy(event.key === "ArrowLeft" ? -step : step);
      return;
    }

    if (event.key === "i" || event.key === "I") {
      event.preventDefault();
      setInFromPlayhead();
      return;
    }

    if (event.key === "o" || event.key === "O") {
      event.preventDefault();
      setOutFromPlayhead();
    }
  }

  async function onExport() {
    if (!meta || !canExport) return;

    error = null;
    exporting = true;
    status = "Exporting…";

    try {
      unlistenProgress?.();
      unlistenProgress = await listen<{ phase: string; message: string; pct?: number | null }>(
        "export-progress",
        (event) => {
          const p = event.payload;
          const pct =
            p.pct != null && Number.isFinite(p.pct) ? ` (${Math.round(p.pct)}%)` : "";
          status = `${p.message}${pct}`;
        },
      );

      const result = await exportClip({
        url: url.trim(),
        title: meta.title,
        start_secs: inPoint,
        end_secs: outPoint,
        max_height: maxHeight,
        include_audio: includeAudio,
        out_dir: savePath.trim(),
      });

      status = `Saved: ${result.output_path}`;

      try {
        await revealInFolder(result.output_path);
      } catch {
        // Non-fatal: export succeeded
      }

      try {
        await saveSettings({
          last_save_dir: savePath.trim(),
          max_height: maxHeight,
          include_audio: includeAudio,
        });
      } catch {
        // Non-fatal
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      error = msg;
      status = "Export failed";
    } finally {
      exporting = false;
      unlistenProgress?.();
      unlistenProgress = null;
    }
  }

  onMount(() => {
    window.addEventListener("keydown", onKeyDown);
    void (async () => {
      await refreshDeps();
      if (deps && deps.ytdlp && deps.ffmpeg) {
        await initSettings();
      }
    })();
  });

  onDestroy(() => {
    window.removeEventListener("keydown", onKeyDown);
    unlistenProgress?.();
  });
</script>

{#if checkingDeps && !deps}
  <div class="boot">
    <LoaderCircle class="spin" size={22} strokeWidth={2} aria-hidden="true" />
    <span>Checking tools…</span>
  </div>
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
          <VideoPlayer
            bind:this={player}
            src={preview.url_or_path}
            mode={preview.mode}
            bind:currentTime
            onError={onPreviewError}
          />
        {:else}
          <div class="preview-placeholder">
            <Film size={36} strokeWidth={1.5} class="placeholder-icon" aria-hidden="true" />
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
        {exporting}
        onExport={() => void onExport()}
      />
    </div>

    <Timeline
      {duration}
      bind:currentTime
      bind:inPoint
      bind:outPoint
      onSeek={onSeek}
      onSetIn={setInFromPlayhead}
      onSetOut={setOutFromPlayhead}
      onJumpToIn={jumpToIn}
      onJumpToOut={jumpToOut}
      onPlaySelection={onPlaySelection}
    />
  </div>
{/if}

<style>
  .boot {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.65rem;
    color: var(--muted);
  }

  .boot :global(.spin) {
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
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

  .preview-placeholder :global(.placeholder-icon) {
    color: var(--muted);
    opacity: 0.7;
    margin-bottom: 0.35rem;
  }

  .muted {
    color: var(--muted);
    font-size: 0.9rem;
  }

  @media (max-width: 720px) {
    .main {
      grid-template-columns: 1fr;
    }
  }
</style>
