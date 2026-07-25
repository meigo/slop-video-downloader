# Slop Video Downloader — design

**Date:** 2026-07-26  
**Status:** Design (approved for planning)  
**Product:** Desktop app to download/record short YouTube clips as reference media for slop-animator (`/Users/meigo/Projects/slop/slop-animator`)

## Motivation

slop-animator loads local video files as reference layers (`File` → blob URL → `<video>`). Grabbing a useful motion reference today means browser tools, full downloads, and manual trimming. This app makes that a single flow: paste a YouTube URL, preview, set in/out, export an animator-friendly MP4 clip.

## Goals

1. Paste a public YouTube URL and preview the video in-app.
2. Set precise in/out points on a simple timeline.
3. Export a browser-playable **H.264 + AAC `.mp4`** suitable as a slop-animator video reference.
4. Work well on **macOS first**, without hard-blocking Windows/Linux later.
5. Fail clearly when `yt-dlp` / `ffmpeg` are missing or a video cannot be accessed.

## Non-goals (v1)

- Non-YouTube sources (yt-dlp can wait until later)
- Playlist / batch download UI
- Account login, cookies UI, or age-gate workarounds beyond what plain yt-dlp does
- Deep integration with slop-animator (no auto-import IPC)
- Subtitles, chapters browser, or multi-clip projects
- Bundled/sideloaded yt-dlp+ffmpeg binaries in the installer (PATH check is enough for v1)
- Hardware-encode tuning beyond sane ffmpeg defaults

## Decisions (locked during brainstorming)

| # | Decision | Choice |
|---|----------|--------|
| D1 | Form factor | Desktop GUI |
| D2 | Clip workflow | Metadata + hybrid preview, timeline in/out, then export |
| D3 | Platform | macOS first; keep portable path open |
| D4 | Preview strategy | Hybrid: stream first, fall back to local download for preview |
| D5 | Export target | Configurable; default animator-ready (H.264 MP4 + AAC, max 1080p, audio on) |
| D6 | Sources (v1) | YouTube only |
| D7 | Stack | Tauri 2 + Svelte UI + yt-dlp + ffmpeg (external tools on PATH) |

## Architecture

```
┌─────────────────────────────────────────────┐
│  Svelte UI (Tauri webview)                  │
│  URL bar · player · timeline · export form  │
└─────────────────┬───────────────────────────┘
                  │ invoke + events
┌─────────────────▼───────────────────────────┐
│  Tauri / Rust commands                      │
│  metadata · preview resolve · export · deps │
└─────────────────┬───────────────────────────┘
                  │ spawn processes
        ┌─────────┴─────────┐
        ▼                   ▼
     yt-dlp              ffmpeg
```

### Responsibilities

| Layer | Owns |
|-------|------|
| **Svelte** | Layout, HTML5 video player, timeline/in-out UX, settings form, progress, user-facing errors |
| **Rust (Tauri)** | Validate YouTube URLs, spawn yt-dlp/ffmpeg, temp paths, progress events, open-in-Finder helper, settings persistence bridge |
| **yt-dlp** | Metadata, stream URL discovery, range/full downloads |
| **ffmpeg** | Final trim/mux/transcode to clean H.264+AAC MP4 |

### Tauri commands (v1)

- `check_deps() -> { ytdlp: bool, ffmpeg: bool, ytdlp_path?, ffmpeg_path? }`
- `fetch_metadata(url) -> { id, title, duration_secs, thumbnail_url }`
- `resolve_preview(url) -> { mode: "stream" \| "file", url_or_path, note? }`
- `export_clip(opts) -> { output_path }` with progress events  
  `opts`: `{ url, start_secs, end_secs, max_height, include_audio, out_dir }`

### Settings (persisted locally)

- Last save directory
- Max height (default `1080`)
- Include audio (default `true`)
- Window size (optional, nice-to-have)

## UI workflow

Single main window:

```
┌─────────────────────────────────────────────────────┐
│  [YouTube URL........................] [Fetch]      │
│  Title · duration · status                          │
├──────────────────────────┬──────────────────────────┤
│                          │  In / Out / Duration     │
│     Video preview        │  Max height              │
│                          │  Include audio           │
│                          │  Save to…                │
├──────────────────────────┴──────────────────────────┤
│  ▶ timeline with playhead + in/out handles          │
│  [Set In] [Set Out] [Play selection] [Export clip]  │
└─────────────────────────────────────────────────────┘
```

### Behavior

1. **Fetch** loads metadata and starts hybrid preview resolution.
2. Status line distinguishes: fetching metadata → streaming preview → downloading preview → ready → error.
3. Timeline supports scrubbing, dragging in/out, and **Set In / Set Out** from the current playhead.
4. Keyboard (v1): `I` / `O` set in/out; Space play/pause.
5. **Export** runs the clip pipeline with a progress bar; on success, reveal the file in Finder (macOS).
6. If stream preview fails, automatically fall back to a local preview file and show a non-fatal status message.

### Defaults

| Setting | Default |
|---------|---------|
| Max height | 1080p (options: 480, 720, 1080, source) |
| Audio | On |
| Save folder | Last used, else `~/Movies/Slop Refs` |

## Clip pipeline

### Preview (hybrid)

1. Run yt-dlp metadata for the URL (reject non-YouTube hosts in v1).
2. Prefer a **progressive** format that includes video+audio for stream preview.
3. If no suitable progressive stream exists, or the player errors, **download a local preview file** (quality capped, e.g. ≤720p) into a temp directory and play that file.
4. The UI always uses one player control surface; only the media source changes.

### Export

1. Read in/out in seconds from the timeline (`end > start`, both within `[0, duration]`).
2. Prefer downloading only the needed section when yt-dlp supports it reliably for the chosen format (`--download-sections` or equivalent).
3. Always finish with **ffmpeg** producing **H.264 + AAC in `.mp4`**:
   - Remux when streams are already compatible.
   - Transcode when needed for browser/animator compatibility.
4. Apply max-height scale (no upscale).
5. If audio is off, emit video-only MP4.
6. Filename: sanitized title + in/out timestamps, e.g. `some-video_01m12s-01m45s.mp4`.
7. Delete temp intermediates on success. On failure, do not present a partial file as success.

### Dependency strategy (v1)

Require `yt-dlp` and `ffmpeg` on `PATH` (e.g. Homebrew on macOS). On launch (or first export), `check_deps` drives a clear **missing tools** empty state with install hints. Bundling sidecars is deferred.

## Project layout

```
slop-video-downloader/
  src-tauri/                 # Rust: commands, process spawn, paths
  src/                       # Svelte UI
  docs/superpowers/
    specs/                   # design docs
    plans/                   # implementation plans
  README.md
```

## Error handling

| Case | Behavior |
|------|----------|
| Empty / invalid URL | Client-side validation before invoke |
| Not YouTube (v1) | Reject with “YouTube only in v1” |
| Missing yt-dlp or ffmpeg | Blocking empty state with install guidance |
| Stream preview failure | Auto-fallback to local preview; status note |
| Private / unavailable video | Surface yt-dlp error summary |
| Export failure | Keep editor state; show error; no “success” toast |

Errors shown to the user should be short plain English, with optional expandable detail (stderr) for debugging.

## Testing

### Automated (no live network)

- Time formatting and parse helpers
- Filename sanitization
- YouTube URL validation
- yt-dlp / ffmpeg argument builders (snapshot or equality tests)

### Manual checklist

1. Public short video: stream preview works.
2. Force or hit fallback path: local preview still scrubbable.
3. Export a 10–30s clip; plays in Safari/Chrome.
4. Same file loads as a video reference in slop-animator.
5. Missing-deps screen when tools are not on PATH.
6. Audio on/off and max-height options produce expected outputs.

Do **not** put live YouTube downloads in CI (flaky and environment-dependent).

## Success criteria

1. Paste a normal public YouTube URL → preview within a short wait (stream or fallback).
2. Set in/out and export an MP4 that plays in a browser and loads as a slop-animator video ref.
3. Missing yt-dlp/ffmpeg is obvious and recoverable via install instructions.
4. macOS is the polished target; code avoids unnecessary Mac-only APIs beyond Finder reveal.

## Future (explicitly later)

- Other yt-dlp sites with light source-specific polish
- Bundled yt-dlp/ffmpeg for zero-setup installs
- Cookie / browser-cookie import for restricted videos
- “Open in slop-animator” or shared refs folder convention
- Selection loop playback, frame step, and denser keyboard editing
- Windows / Linux packaging
)
