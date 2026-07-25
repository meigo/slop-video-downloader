<script lang="ts">
  import { formatTimestamp } from "$lib/time";

  interface Props {
    inPoint: number;
    outPoint: number;
    duration: number;
    maxHeight: number | null;
    includeAudio: boolean;
    savePath: string;
    canExport: boolean;
    onExport?: () => void;
  }

  let {
    inPoint,
    outPoint,
    duration,
    maxHeight = $bindable(),
    includeAudio = $bindable(),
    savePath = $bindable(),
    canExport,
    onExport,
  }: Props = $props();

  const clipDuration = $derived(Math.max(0, outPoint - inPoint));

  function onHeightChange(event: Event) {
    const value = (event.currentTarget as HTMLSelectElement).value;
    maxHeight = value === "source" ? null : Number(value);
  }
</script>

<aside class="export-panel">
  <h2>Export</h2>

  <dl class="range">
    <div>
      <dt>In</dt>
      <dd>{formatTimestamp(inPoint)}</dd>
    </div>
    <div>
      <dt>Out</dt>
      <dd>{formatTimestamp(outPoint)}</dd>
    </div>
    <div>
      <dt>Duration</dt>
      <dd>{formatTimestamp(clipDuration)}</dd>
    </div>
    <div>
      <dt>Video</dt>
      <dd>{duration > 0 ? formatTimestamp(duration) : "—"}</dd>
    </div>
  </dl>

  <label class="field">
    <span>Max height</span>
    <select value={maxHeight === null ? "source" : String(maxHeight)} onchange={onHeightChange}>
      <option value="480">480</option>
      <option value="720">720</option>
      <option value="1080">1080</option>
      <option value="source">Source</option>
    </select>
  </label>

  <label class="field checkbox">
    <input type="checkbox" bind:checked={includeAudio} />
    <span>Include audio</span>
  </label>

  <label class="field">
    <span>Save to</span>
    <input type="text" bind:value={savePath} placeholder="Export folder path" />
    <span class="hint">Chosen folder (folder dialog in a later step)</span>
  </label>

  <button type="button" class="export-btn" disabled={!canExport} onclick={() => onExport?.()}>
    Export clip
  </button>
</aside>

<style>
  .export-panel {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
    padding: 1rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    min-width: 0;
  }

  h2 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
  }

  .range {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem 0.75rem;
    margin: 0;
  }

  .range div {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }

  .range dt {
    font-size: 0.75rem;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .range dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.95rem;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-size: 0.9rem;
  }

  .field.checkbox {
    flex-direction: row;
    align-items: center;
    gap: 0.5rem;
  }

  .field.checkbox input {
    width: auto;
    margin: 0;
  }

  .hint {
    font-size: 0.75rem;
    color: var(--muted);
  }

  .export-btn {
    margin-top: 0.25rem;
    width: 100%;
  }
</style>
