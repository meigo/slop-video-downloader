<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import FastForward from "@lucide/svelte/icons/fast-forward";
  import Pause from "@lucide/svelte/icons/pause";
  import Play from "@lucide/svelte/icons/play";
  import Rewind from "@lucide/svelte/icons/rewind";
  import Square from "@lucide/svelte/icons/square";
  import Volume1 from "@lucide/svelte/icons/volume-1";
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import VolumeX from "@lucide/svelte/icons/volume-x";
  import { onDestroy } from "svelte";
  import { formatTimestamp } from "$lib/time";

  interface Props {
    /** Raw URL or filesystem path from preview. */
    src: string;
    /** `"stream"` uses src as-is; `"file"` goes through convertFileSrc. */
    mode: "stream" | "file";
    currentTime?: number;
    onError?: () => void;
    onDuration?: (duration: number) => void;
    /** Fired when play/pause state changes (for parent keyboard / loop logic). */
    onPlayState?: (playing: boolean) => void;
  }

  let {
    src,
    mode,
    currentTime = $bindable(0),
    onError,
    onDuration,
    onPlayState,
  }: Props = $props();

  let el = $state<HTMLVideoElement | null>(null);
  let seeking = false;
  let paused = $state(true);
  let muted = $state(false);
  let videoDuration = $state(0);
  let volume = $state(1);

  const displaySrc = $derived(mode === "file" ? convertFileSrc(src) : src);

  const SKIP_SECS = 5;
  const ICON = 18;

  export function play() {
    void el?.play();
  }

  export function pause() {
    el?.pause();
  }

  export function togglePlay() {
    if (!el) return;
    if (el.paused) void el.play();
    else el.pause();
  }

  export function seek(t: number) {
    if (!el) return;
    const d = Number.isFinite(el.duration) ? el.duration : t;
    el.currentTime = Math.min(Math.max(0, t), d);
    currentTime = el.currentTime;
  }

  export function isPaused(): boolean {
    return el?.paused ?? true;
  }

  // Parent-driven seek (timeline scrub) when currentTime changes externally.
  $effect(() => {
    const t = currentTime;
    if (!el || seeking) return;
    if (Math.abs(el.currentTime - t) > 0.05) {
      seeking = true;
      el.currentTime = t;
      seeking = false;
    }
  });

  function onTimeUpdate() {
    if (!el || seeking) return;
    currentTime = el.currentTime;
  }

  function onLoadedMetadata() {
    if (!el) return;
    currentTime = el.currentTime;
    videoDuration = Number.isFinite(el.duration) ? el.duration : 0;
    if (Number.isFinite(el.duration)) {
      onDuration?.(el.duration);
    }
  }

  function syncPlayState() {
    if (!el) return;
    paused = el.paused;
    onPlayState?.(!el.paused);
  }

  function handleError() {
    onError?.();
  }

  function onTogglePlay() {
    togglePlay();
  }

  function onStop() {
    if (!el) return;
    el.pause();
    seek(0);
  }

  function onSkip(delta: number) {
    if (!el) return;
    const d = Number.isFinite(el.duration) ? el.duration : 0;
    seek(Math.min(Math.max(0, el.currentTime + delta), d));
  }

  function onToggleMute() {
    if (!el) return;
    el.muted = !el.muted;
    muted = el.muted;
  }

  function onVolumeInput(event: Event) {
    if (!el) return;
    const value = Number((event.target as HTMLInputElement).value);
    volume = value;
    el.volume = value;
    if (value > 0 && el.muted) {
      el.muted = false;
      muted = false;
    }
  }

  function onVideoClick() {
    togglePlay();
  }

  onDestroy(() => {
    el?.pause();
  });
</script>

<div class="player">
  {#if src}
    <!-- svelte-ignore a11y_media_has_caption -->
    <video
      bind:this={el}
      src={displaySrc}
      ontimeupdate={onTimeUpdate}
      onloadedmetadata={onLoadedMetadata}
      onplay={syncPlayState}
      onpause={syncPlayState}
      onended={syncPlayState}
      onerror={handleError}
      onclick={onVideoClick}
      controls={false}
      playsinline
    ></video>

    <div class="controls" role="toolbar" aria-label="Playback controls">
      <div class="transport">
        <button
          type="button"
          class="ctrl"
          title="Skip back {SKIP_SECS}s"
          aria-label="Skip back {SKIP_SECS} seconds"
          onclick={() => onSkip(-SKIP_SECS)}
        >
          <Rewind size={ICON} strokeWidth={2} aria-hidden="true" />
        </button>
        <button
          type="button"
          class="ctrl primary"
          title={paused ? "Play (Space)" : "Pause (Space)"}
          aria-label={paused ? "Play" : "Pause"}
          onclick={onTogglePlay}
        >
          {#if paused}
            <Play size={ICON} strokeWidth={2} aria-hidden="true" />
          {:else}
            <Pause size={ICON} strokeWidth={2} aria-hidden="true" />
          {/if}
        </button>
        <button type="button" class="ctrl" title="Stop" aria-label="Stop" onclick={onStop}>
          <Square size={16} strokeWidth={2.25} aria-hidden="true" />
        </button>
        <button
          type="button"
          class="ctrl"
          title="Skip forward {SKIP_SECS}s"
          aria-label="Skip forward {SKIP_SECS} seconds"
          onclick={() => onSkip(SKIP_SECS)}
        >
          <FastForward size={ICON} strokeWidth={2} aria-hidden="true" />
        </button>
      </div>

      <div class="time" aria-live="off">
        <span>{formatTimestamp(currentTime)}</span>
        <span class="sep">/</span>
        <span>{formatTimestamp(videoDuration)}</span>
      </div>

      <div class="volume">
        <button
          type="button"
          class="ctrl"
          title={muted || volume === 0 ? "Unmute" : "Mute"}
          aria-label={muted || volume === 0 ? "Unmute" : "Mute"}
          onclick={onToggleMute}
        >
          {#if muted || volume === 0}
            <VolumeX size={ICON} strokeWidth={2} aria-hidden="true" />
          {:else if volume < 0.5}
            <Volume1 size={ICON} strokeWidth={2} aria-hidden="true" />
          {:else}
            <Volume2 size={ICON} strokeWidth={2} aria-hidden="true" />
          {/if}
        </button>
        <input
          type="range"
          min="0"
          max="1"
          step="0.05"
          value={muted ? 0 : volume}
          aria-label="Volume"
          oninput={onVolumeInput}
        />
      </div>
    </div>
  {:else}
    <div class="empty">No preview</div>
  {/if}
</div>

<style>
  .player {
    height: 100%;
    min-height: 240px;
    display: flex;
    flex-direction: column;
    background: #0a0a0c;
    border: 1px solid var(--border);
    border-radius: 10px;
    overflow: hidden;
  }

  video {
    width: 100%;
    flex: 1;
    min-height: 180px;
    max-height: 380px;
    object-fit: contain;
    background: #000;
    display: block;
    cursor: pointer;
  }

  .controls {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.65rem 1rem;
    padding: 0.55rem 0.75rem;
    background: color-mix(in srgb, var(--surface) 92%, #000);
    border-top: 1px solid var(--border);
  }

  .transport {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  button.ctrl {
    min-width: 2.25rem;
    height: 2.25rem;
    padding: 0 0.45rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text);
    line-height: 1;
    border-radius: 8px;
  }

  button.ctrl:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-color: var(--accent);
  }

  button.ctrl.primary {
    min-width: 2.75rem;
    background: var(--accent);
    border-color: transparent;
    color: #fff;
  }

  button.ctrl.primary:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .time {
    font-variant-numeric: tabular-nums;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.8rem;
    color: var(--muted);
    display: flex;
    gap: 0.25rem;
  }

  .sep {
    opacity: 0.6;
  }

  .volume {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    min-width: 8rem;
  }

  .volume input[type="range"] {
    width: 6rem;
    accent-color: var(--accent);
    cursor: pointer;
  }

  .empty {
    flex: 1;
    display: grid;
    place-items: center;
    color: var(--muted);
    font-size: 0.9rem;
    min-height: 240px;
  }
</style>
