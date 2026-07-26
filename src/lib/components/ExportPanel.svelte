<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import Download from "@lucide/svelte/icons/download";
  import Film from "@lucide/svelte/icons/film";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import Music from "@lucide/svelte/icons/music";
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import { formatTimestamp } from "$lib/time";
  import type { ExportKind } from "$lib/types";

  interface Props {
    inPoint: number;
    outPoint: number;
    duration: number;
    maxHeight: number | null;
    includeAudio: boolean;
    exportKind: ExportKind;
    savePath: string;
    canExport: boolean;
    exporting?: boolean;
    onExport?: () => void;
  }

  let {
    inPoint,
    outPoint,
    duration,
    maxHeight = $bindable(),
    includeAudio = $bindable(),
    exportKind = $bindable("video" as ExportKind),
    savePath = $bindable(),
    canExport,
    exporting = false,
    onExport,
  }: Props = $props();

  const clipDuration = $derived(Math.max(0, outPoint - inPoint));
  const audioOnly = $derived(exportKind === "audio");
  const ICON = 16;

  function onHeightChange(event: Event) {
    const value = (event.currentTarget as HTMLSelectElement).value;
    maxHeight = value === "source" ? null : Number(value);
  }

  function setKind(kind: ExportKind) {
    exportKind = kind;
  }

  async function pickFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: savePath || undefined,
        title: "Choose export folder",
      });
      if (typeof selected === "string" && selected.length > 0) {
        savePath = selected;
      }
    } catch {
      // User cancel or dialog unavailable in non-Tauri env
    }
  }
</script>

<aside class="export-panel">
  <h2>
    {#if audioOnly}
      <Music size={18} strokeWidth={2} aria-hidden="true" />
    {:else}
      <Film size={18} strokeWidth={2} aria-hidden="true" />
    {/if}
    <span>Export</span>
  </h2>

  <div
    class="kind-toggle"
    role="group"
    aria-label="Export type"
  >
    <button
      type="button"
      class="kind-btn"
      class:active={!audioOnly}
      disabled={exporting}
      aria-pressed={!audioOnly}
      onclick={() => setKind("video")}
    >
      <Film size={ICON} strokeWidth={2} aria-hidden="true" />
      <span>Video</span>
      <span class="kind-ext">MP4</span>
    </button>
    <button
      type="button"
      class="kind-btn"
      class:active={audioOnly}
      disabled={exporting}
      aria-pressed={audioOnly}
      onclick={() => setKind("audio")}
    >
      <Music size={ICON} strokeWidth={2} aria-hidden="true" />
      <span>Audio only</span>
      <span class="kind-ext">M4A</span>
    </button>
  </div>

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
      <dt>Source</dt>
      <dd>{duration > 0 ? formatTimestamp(duration) : "—"}</dd>
    </div>
  </dl>

  {#if !audioOnly}
    <label class="field">
      <span>Max height</span>
      <select
        value={maxHeight === null ? "source" : String(maxHeight)}
        onchange={onHeightChange}
        disabled={exporting}
      >
        <option value="480">480</option>
        <option value="720">720</option>
        <option value="1080">1080</option>
        <option value="source">Source</option>
      </select>
    </label>

    <label class="field checkbox">
      <input type="checkbox" bind:checked={includeAudio} disabled={exporting} />
      <Volume2 size={ICON} strokeWidth={2} aria-hidden="true" />
      <span>Include audio</span>
    </label>
  {:else}
    <p class="note muted">
      AAC audio in <code>.m4a</code>. Max height and video audio toggle don’t apply.
    </p>
  {/if}

  <div class="field">
    <span>Save to</span>
    <div class="path-row">
      <input type="text" bind:value={savePath} placeholder="Export folder path" />
      <button type="button" class="browse" onclick={() => void pickFolder()} disabled={exporting}>
        <FolderOpen size={ICON} strokeWidth={2} aria-hidden="true" />
        <span>Browse</span>
      </button>
    </div>
  </div>

  <button
    type="button"
    class="export-btn"
    class:audio={audioOnly}
    disabled={!canExport || exporting}
    onclick={() => onExport?.()}
  >
    {#if exporting}
      <LoaderCircle class="spin" size={ICON} strokeWidth={2} aria-hidden="true" />
      <span>Exporting…</span>
    {:else if audioOnly}
      <Music size={ICON} strokeWidth={2} aria-hidden="true" />
      <span>Export audio</span>
    {:else}
      <Download size={ICON} strokeWidth={2} aria-hidden="true" />
      <span>Export clip</span>
    {/if}
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
    min-height: 0;
    max-height: 100%;
    overflow-y: auto;
  }

  h2 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
  }

  .kind-toggle {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.4rem;
  }

  .kind-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.2rem;
    padding: 0.55rem 0.4rem;
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--muted);
    font-weight: 500;
    font-size: 0.85rem;
    border-radius: 8px;
    cursor: pointer;
  }

  .kind-btn:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--text);
    background: color-mix(in srgb, var(--accent) 12%, var(--bg));
  }

  .kind-btn.active {
    color: #fff;
    background: var(--accent);
    border-color: transparent;
  }

  .kind-btn.active:hover:not(:disabled) {
    background: var(--accent-hover);
    color: #fff;
  }

  .kind-btn.active:nth-child(2) {
    background: var(--selection);
    color: #1a1400;
  }

  .kind-btn.active:nth-child(2):hover:not(:disabled) {
    background: var(--selection-hover);
    color: #1a1400;
  }

  .kind-ext {
    font-size: 0.7rem;
    opacity: 0.85;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    text-transform: uppercase;
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
    gap: 0.45rem;
  }

  .field.checkbox input {
    width: auto;
    margin: 0;
  }

  .note {
    margin: 0;
    font-size: 0.82rem;
    line-height: 1.4;
  }

  .note code {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.85em;
  }

  .muted {
    color: var(--muted);
  }

  .path-row {
    display: flex;
    gap: 0.4rem;
    min-width: 0;
  }

  .path-row input {
    flex: 1;
    min-width: 0;
  }

  button.browse {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text);
  }

  button.browse:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-color: var(--accent);
  }

  .export-btn {
    margin-top: 0.25rem;
    width: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.45rem;
  }

  .export-btn.audio {
    background: var(--selection);
    color: #1a1400;
  }

  .export-btn.audio:hover:not(:disabled) {
    background: var(--selection-hover);
  }

  .export-btn :global(.spin) {
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
