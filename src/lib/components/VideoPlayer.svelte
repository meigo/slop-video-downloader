<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { onDestroy } from "svelte";

  interface Props {
    /** Raw URL or filesystem path from preview. */
    src: string;
    /** `"stream"` uses src as-is; `"file"` goes through convertFileSrc. */
    mode: "stream" | "file";
    currentTime?: number;
    onError?: () => void;
    onDuration?: (duration: number) => void;
  }

  let {
    src,
    mode,
    currentTime = $bindable(0),
    onError,
    onDuration,
  }: Props = $props();

  let el = $state<HTMLVideoElement | null>(null);
  let seeking = false;

  const displaySrc = $derived(
    mode === "file" ? convertFileSrc(src) : src,
  );

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
    if (Number.isFinite(el.duration)) {
      onDuration?.(el.duration);
    }
  }

  function handleError() {
    onError?.();
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
      onerror={handleError}
      controls={false}
      playsinline
    ></video>
  {:else}
    <div class="empty">No preview</div>
  {/if}
</div>

<style>
  .player {
    height: 100%;
    min-height: 240px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #0a0a0c;
    border: 1px solid var(--border);
    border-radius: 10px;
    overflow: hidden;
  }

  video {
    width: 100%;
    height: 100%;
    max-height: 420px;
    object-fit: contain;
    background: #000;
    display: block;
  }

  .empty {
    color: var(--muted);
    font-size: 0.9rem;
  }
</style>
