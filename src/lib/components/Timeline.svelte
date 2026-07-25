<script lang="ts">
  import { clampRange, formatTimestamp } from "$lib/time";

  interface Props {
    duration: number;
    currentTime: number;
    inPoint: number;
    outPoint: number;
    /** Whether the main player is currently playing (for button label). */
    playing?: boolean;
    onSeek?: (t: number) => void;
    onSetIn?: () => void;
    onSetOut?: () => void;
    onPlayPause?: () => void;
    onStop?: () => void;
    onPlaySelection?: () => void;
    onInOutChange?: (inPoint: number, outPoint: number) => void;
  }

  let {
    duration,
    currentTime = $bindable(),
    inPoint = $bindable(),
    outPoint = $bindable(),
    playing = false,
    onSeek,
    onSetIn,
    onSetOut,
    onPlayPause,
    onStop,
    onPlaySelection,
    onInOutChange,
  }: Props = $props();

  let trackEl = $state<HTMLDivElement | null>(null);
  let dragMode = $state<"playhead" | "in" | "out" | null>(null);

  function pct(t: number): number {
    if (!(duration > 0)) return 0;
    return Math.min(100, Math.max(0, (t / duration) * 100));
  }

  function timeFromClientX(clientX: number): number {
    if (!trackEl || !(duration > 0)) return 0;
    const rect = trackEl.getBoundingClientRect();
    const ratio = Math.min(1, Math.max(0, (clientX - rect.left) / rect.width));
    return ratio * duration;
  }

  function applySeek(t: number) {
    const clamped = Math.min(Math.max(0, t), Math.max(0, duration));
    currentTime = clamped;
    onSeek?.(clamped);
  }

  function applyInOut(nextIn: number, nextOut: number) {
    const { start, end } = clampRange(nextIn, nextOut, duration);
    inPoint = start;
    outPoint = end;
    onInOutChange?.(start, end);
  }

  function onPointerDown(event: PointerEvent) {
    if (!(duration > 0) || !trackEl) return;
    const target = event.target as HTMLElement;
    const handle = target.dataset.handle as "in" | "out" | "playhead" | undefined;
    dragMode = handle ?? "playhead";
    trackEl.setPointerCapture(event.pointerId);
    handleDrag(event.clientX);
    event.preventDefault();
  }

  function onPointerMove(event: PointerEvent) {
    if (!dragMode) return;
    handleDrag(event.clientX);
  }

  function onPointerUp(event: PointerEvent) {
    if (!dragMode || !trackEl) return;
    trackEl.releasePointerCapture(event.pointerId);
    dragMode = null;
  }

  function handleDrag(clientX: number) {
    const t = timeFromClientX(clientX);
    if (dragMode === "playhead") {
      applySeek(t);
    } else if (dragMode === "in") {
      applyInOut(t, outPoint);
    } else if (dragMode === "out") {
      applyInOut(inPoint, t);
    }
  }

  const inPct = $derived(pct(inPoint));
  const outPct = $derived(pct(outPoint));
  const playPct = $derived(pct(currentTime));
  const rangeWidth = $derived(Math.max(0, outPct - inPct));
</script>

<footer class="timeline" aria-label="Timeline">
  <div class="times">
    <span>{formatTimestamp(currentTime)}</span>
    <span class="muted">In {formatTimestamp(inPoint)} · Out {formatTimestamp(outPoint)}</span>
    <span>{formatTimestamp(duration)}</span>
  </div>

  <div
    class="track"
    bind:this={trackEl}
    role="slider"
    tabindex="0"
    aria-valuemin={0}
    aria-valuemax={duration}
    aria-valuenow={currentTime}
    aria-label="Seek and selection"
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    onpointercancel={onPointerUp}
  >
    <div class="rail"></div>
    <div
      class="range"
      style="left: {inPct}%; width: {rangeWidth}%;"
    ></div>
    <div
      class="handle in"
      data-handle="in"
      style="left: {inPct}%;"
      title="In point"
    ></div>
    <div
      class="handle out"
      data-handle="out"
      style="left: {outPct}%;"
      title="Out point"
    ></div>
    <div
      class="playhead"
      data-handle="playhead"
      style="left: {playPct}%;"
      title="Playhead"
    ></div>
  </div>

  <div class="actions">
    <div class="transport">
      <button
        type="button"
        class="secondary play"
        disabled={!(duration > 0)}
        title={playing ? "Pause (Space)" : "Play (Space)"}
        onclick={() => onPlayPause?.()}
      >
        {playing ? "Pause" : "Play"}
      </button>
      <button
        type="button"
        class="secondary"
        disabled={!(duration > 0)}
        title="Stop and return to start"
        onclick={() => onStop?.()}
      >
        Stop
      </button>
      <button
        type="button"
        class="secondary"
        disabled={!(outPoint > inPoint)}
        title="Loop play between in and out"
        onclick={() => onPlaySelection?.()}
      >
        Play selection
      </button>
    </div>
    <div class="markers">
      <button type="button" class="secondary" onclick={() => onSetIn?.()}>Set In (I)</button>
      <button type="button" class="secondary" onclick={() => onSetOut?.()}>Set Out (O)</button>
    </div>
  </div>
</footer>

<style>
  .timeline {
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
    padding: 0.85rem 1rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
  }

  .times {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    font-variant-numeric: tabular-nums;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.8rem;
  }

  .muted {
    color: var(--muted);
  }

  .track {
    position: relative;
    height: 28px;
    cursor: pointer;
    touch-action: none;
    user-select: none;
  }

  .rail {
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    height: 6px;
    transform: translateY(-50%);
    background: var(--border);
    border-radius: 3px;
  }

  .range {
    position: absolute;
    top: 50%;
    height: 6px;
    transform: translateY(-50%);
    background: color-mix(in srgb, var(--accent) 55%, transparent);
    border-radius: 3px;
    pointer-events: none;
  }

  .handle {
    position: absolute;
    top: 50%;
    width: 10px;
    height: 18px;
    margin-left: -5px;
    transform: translateY(-50%);
    background: var(--text);
    border-radius: 2px;
    z-index: 2;
    cursor: ew-resize;
  }

  .handle.in {
    background: var(--accent);
  }

  .handle.out {
    background: var(--accent-hover);
  }

  .playhead {
    position: absolute;
    top: 2px;
    bottom: 2px;
    width: 2px;
    margin-left: -1px;
    background: #fff;
    z-index: 3;
    pointer-events: none;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.35);
  }

  .playhead::after {
    content: "";
    position: absolute;
    top: -2px;
    left: 50%;
    width: 8px;
    height: 8px;
    margin-left: -4px;
    background: #fff;
    border-radius: 1px;
    transform: rotate(45deg);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .transport,
  .markers {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  button.secondary {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text);
    font-weight: 500;
  }

  button.secondary.play {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    border-color: var(--accent);
  }

  button.secondary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-color: var(--accent);
  }

  button.secondary:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
