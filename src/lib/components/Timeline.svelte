<script lang="ts">
  import ArrowLeftToLine from "@lucide/svelte/icons/arrow-left-to-line";
  import ArrowRightToLine from "@lucide/svelte/icons/arrow-right-to-line";
  import Repeat from "@lucide/svelte/icons/repeat";
  import SkipBack from "@lucide/svelte/icons/skip-back";
  import SkipForward from "@lucide/svelte/icons/skip-forward";
  import { clampRange, formatTimestamp } from "$lib/time";

  type DragMode = "playhead" | "in" | "out";

  interface Props {
    duration: number;
    currentTime: number;
    inPoint: number;
    outPoint: number;
    onSeek?: (t: number) => void;
    onSetIn?: () => void;
    onSetOut?: () => void;
    onJumpToIn?: () => void;
    onJumpToOut?: () => void;
    onPlaySelection?: () => void;
    onInOutChange?: (inPoint: number, outPoint: number) => void;
  }

  let {
    duration,
    currentTime = $bindable(),
    inPoint = $bindable(),
    outPoint = $bindable(),
    onSeek,
    onSetIn,
    onSetOut,
    onJumpToIn,
    onJumpToOut,
    onPlaySelection,
    onInOutChange,
  }: Props = $props();

  let trackEl = $state<HTMLDivElement | null>(null);
  let dragMode = $state<DragMode | null>(null);
  let hoverMode = $state<DragMode | null>(null);

  const ICON = 16;
  /**
   * Grab half-width in CSS pixels. Thin markers stay readable on long videos;
   * hit testing uses this larger radius (standard scrubber pattern).
   */
  const HIT_HALF_PX = 8;

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

  /**
   * Pick what to drag when markers overlap.
   * Dual-band priority (common in NLEs):
   * - Upper band favors playhead
   * - Lower band favors in/out
   * Within a band, nearest marker in X wins if inside HIT_HALF_PX.
   * Empty track click → scrub playhead.
   */
  function pickDragMode(clientX: number, clientY: number): DragMode {
    if (!trackEl || !(duration > 0)) return "playhead";

    const rect = trackEl.getBoundingClientRect();
    const x = clientX - rect.left;
    const y = clientY - rect.top;
    const w = Math.max(1, rect.width);
    const h = Math.max(1, rect.height);
    const xOf = (t: number) => (t / duration) * w;
    const distX = (t: number) => Math.abs(x - xOf(t));

    const preferPlayhead = y < h * 0.45;

    type Cand = { mode: DragMode; dist: number; bias: number };
    const cands: Cand[] = [
      { mode: "in", dist: distX(inPoint), bias: preferPlayhead ? 4 : 0 },
      { mode: "out", dist: distX(outPoint), bias: preferPlayhead ? 4 : 0 },
      { mode: "playhead", dist: distX(currentTime), bias: preferPlayhead ? 0 : 4 },
    ];

    let best: Cand | null = null;
    let bestScore = Infinity;
    for (const c of cands) {
      if (c.dist > HIT_HALF_PX) continue;
      const score = c.dist + c.bias;
      if (score < bestScore) {
        bestScore = score;
        best = c;
      }
    }
    return best?.mode ?? "playhead";
  }

  function onPointerDown(event: PointerEvent) {
    if (!(duration > 0) || !trackEl) return;
    dragMode = pickDragMode(event.clientX, event.clientY);
    hoverMode = dragMode;
    trackEl.setPointerCapture(event.pointerId);
    handleDrag(event.clientX);
    event.preventDefault();
  }

  function onPointerMove(event: PointerEvent) {
    if (dragMode) {
      handleDrag(event.clientX);
      return;
    }
    if (!(duration > 0) || !trackEl) {
      hoverMode = null;
      return;
    }
    // Highlight nearest grabbable marker while hovering.
    const mode = pickDragMode(event.clientX, event.clientY);
    const rect = trackEl.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const w = Math.max(1, rect.width);
    const xOf = (t: number) => (t / duration) * w;
    const t =
      mode === "in" ? inPoint : mode === "out" ? outPoint : currentTime;
    hoverMode = Math.abs(x - xOf(t)) <= HIT_HALF_PX ? mode : null;
  }

  function onPointerLeave() {
    if (!dragMode) hoverMode = null;
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

  function jumpToIn() {
    if (!(duration > 0)) return;
    if (onJumpToIn) onJumpToIn();
    else applySeek(inPoint);
  }

  function jumpToOut() {
    if (!(duration > 0)) return;
    if (onJumpToOut) onJumpToOut();
    else applySeek(outPoint);
  }

  const inPct = $derived(pct(inPoint));
  const outPct = $derived(pct(outPoint));
  const playPct = $derived(pct(currentTime));
  const rangeWidth = $derived(Math.max(0, outPct - inPct));
  const activeMode = $derived(dragMode ?? hoverMode);
</script>

<footer class="timeline" aria-label="Timeline">
  <div class="times">
    <span>{formatTimestamp(currentTime)}</span>
    <span class="muted">In {formatTimestamp(inPoint)} · Out {formatTimestamp(outPoint)}</span>
    <span>{formatTimestamp(duration)}</span>
  </div>

  <div
    class="track"
    class:grabbing={!!dragMode}
    bind:this={trackEl}
    role="slider"
    tabindex="0"
    aria-valuemin={0}
    aria-valuemax={duration}
    aria-valuenow={currentTime}
    aria-label="Seek and selection. Top of track: playhead. Bottom: in and out."
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    onpointercancel={onPointerUp}
    onpointerleave={onPointerLeave}
  >
    <div class="rail"></div>
    <div class="range" style="left: {inPct}%; width: {rangeWidth}%;"></div>

    <!-- Visual markers only; all hit-testing is geometric on the track. -->
    <div
      class="marker in"
      class:active={activeMode === "in"}
      style="left: {inPct}%;"
      title="In point"
      aria-hidden="true"
    >
      <span class="stem"></span>
      <span class="flag"></span>
    </div>
    <div
      class="marker out"
      class:active={activeMode === "out"}
      style="left: {outPct}%;"
      title="Out point"
      aria-hidden="true"
    >
      <span class="stem"></span>
      <span class="flag"></span>
    </div>
    <div
      class="marker playhead"
      class:active={activeMode === "playhead"}
      style="left: {playPct}%;"
      title="Playhead"
      aria-hidden="true"
    >
      <span class="stem"></span>
      <span class="cap"></span>
    </div>
  </div>

  <div class="actions">
    <div class="selection">
      <button
        type="button"
        class="secondary"
        disabled={!(duration > 0)}
        title="Jump playhead to in point"
        onclick={jumpToIn}
      >
        <SkipBack size={ICON} strokeWidth={2} aria-hidden="true" />
        <span>To In</span>
      </button>
      <button
        type="button"
        class="secondary"
        disabled={!(duration > 0)}
        title="Jump playhead to out point"
        onclick={jumpToOut}
      >
        <SkipForward size={ICON} strokeWidth={2} aria-hidden="true" />
        <span>To Out</span>
      </button>
      <button
        type="button"
        class="secondary"
        disabled={!(outPoint > inPoint)}
        title="Loop play between in and out"
        onclick={() => onPlaySelection?.()}
      >
        <Repeat size={ICON} strokeWidth={2} aria-hidden="true" />
        <span>Play selection</span>
      </button>
    </div>
    <div class="markers">
      <button type="button" class="secondary" onclick={() => onSetIn?.()}>
        <ArrowLeftToLine size={ICON} strokeWidth={2} aria-hidden="true" />
        <span>Set In (I)</span>
      </button>
      <button type="button" class="secondary" onclick={() => onSetOut?.()}>
        <ArrowRightToLine size={ICON} strokeWidth={2} aria-hidden="true" />
        <span>Set Out (O)</span>
      </button>
    </div>
  </div>
</footer>

<style>
  .timeline {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
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
    height: 36px;
    cursor: pointer;
    touch-action: none;
    user-select: none;
  }

  .track.grabbing {
    cursor: ew-resize;
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

  /* Thin stems + small flags; grab radius is HIT_HALF_PX in script. */
  .marker {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 0;
    margin-left: 0;
    pointer-events: none;
    z-index: 2;
  }

  .marker .stem {
    position: absolute;
    left: 0;
    width: 2px;
    margin-left: -1px;
    border-radius: 1px;
  }

  .marker.in .stem {
    top: 40%;
    bottom: 2px;
    background: var(--accent);
  }

  .marker.out .stem {
    top: 40%;
    bottom: 2px;
    background: var(--accent-hover);
  }

  .marker.playhead .stem {
    top: 2px;
    bottom: 2px;
    background: #fff;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.35);
    z-index: 3;
  }

  /* In: bottom-left caret */
  .marker.in .flag {
    position: absolute;
    left: -1px;
    bottom: 1px;
    width: 7px;
    height: 8px;
    background: var(--accent);
    clip-path: polygon(0 0, 100% 50%, 0 100%);
  }

  /* Out: bottom-right caret */
  .marker.out .flag {
    position: absolute;
    left: -6px;
    bottom: 1px;
    width: 7px;
    height: 8px;
    background: var(--accent-hover);
    clip-path: polygon(0 50%, 100% 0, 100% 100%);
  }

  /* Playhead: top cap (diamond) */
  .marker.playhead .cap {
    position: absolute;
    top: 0;
    left: 50%;
    width: 8px;
    height: 8px;
    margin-left: -4px;
    background: #fff;
    border-radius: 1px;
    transform: rotate(45deg);
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.25);
  }

  .marker.playhead {
    z-index: 4;
  }

  .marker.in,
  .marker.out {
    z-index: 3;
  }

  .marker.active .stem {
    outline: 1px solid color-mix(in srgb, #fff 55%, transparent);
  }

  .marker.active.playhead .cap,
  .marker.active.in .flag,
  .marker.active.out .flag {
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 60%, transparent);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .selection,
  .markers {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  button.secondary {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text);
    font-weight: 500;
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
