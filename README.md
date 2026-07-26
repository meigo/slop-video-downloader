# Slop Video Downloader

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Desktop app that turns a public YouTube URL into a short, browser-friendly **H.264 + AAC MP4** clip (or AAC **M4A** audio).

Paste a URL → preview in-app → set in/out on a timeline → export a trimmed clip (default max height 1080p, audio on, save under `~/Movies/Slop Refs`).

**Stack:** Tauri 2 + SvelteKit + TypeScript, with **yt-dlp** and **ffmpeg** as external tools on your `PATH`.

## Features

- YouTube URL paste + metadata fetch
- Hybrid preview (stream first, local file fallback)
- Timeline in/out with keyboard shortcuts
- Full-video play or selection loop
- Export **video** (H.264 MP4) or **audio only** (AAC M4A)
- Dependency check for `yt-dlp` / `ffmpeg` with install hints
- macOS-first (Finder reveal on export)

## Requirements

| Tool | Notes |
|------|--------|
| **Node.js** | For frontend tooling (`npm`) |
| **Rust** | Stable toolchain (`rustc` / `cargo`) for the Tauri backend |
| **yt-dlp** | Metadata, stream URLs, section downloads |
| **ffmpeg** | Trim / mux / transcode to clean H.264+AAC MP4 |

On macOS with Homebrew:

```bash
brew install yt-dlp ffmpeg
```

Also install a recent Node LTS and the [Rust toolchain](https://rustup.rs/). Tauri 2 may need additional platform system libraries — see the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

**Supported sources**

- **YouTube** and **Vimeo** (public videos only).
- Other sites are rejected with a clear message until allowlisted.
- No login / cookies UI — private or age-gated pages will fail via yt-dlp.
- If section download isn’t supported for a site, export may **download more of the media** before trimming.

**Notes**

- **yt-dlp and ffmpeg must be on `PATH`.** The app checks at startup and shows install guidance if either is missing. Binaries are not bundled in the installer.
- Keep yt-dlp updated (`brew upgrade yt-dlp`) when a site stops working.

## Development

```bash
npm install
npm run tauri dev
```

Other useful scripts:

| Command | Purpose |
|---------|---------|
| `npm run dev` | Vite/SvelteKit only (no Tauri shell) |
| `npm run build` | Production frontend build |
| `npm run check` | `svelte-check` / TypeScript |
| `npm run tauri build` | Package the desktop app |

## Tests

Frontend (Vitest):

```bash
npm test
```

Rust backend:

```bash
cd src-tauri && cargo test
```

Unit tests do **not** hit live YouTube; they cover URL validation, deps parsing, export naming, and related pure logic.

## Usage

1. Launch the app (`npm run tauri dev` or a built binary).
2. Confirm yt-dlp and ffmpeg are detected (or follow the install guidance if not).
3. Paste a **public YouTube or Vimeo URL** and click **Fetch**.
4. Wait for metadata and preview (stream first; falls back to a local file if streaming fails).
5. Set **in** and **out** on the timeline (drag handles, **Set In** / **Set Out**, or keys **I** / **O**).
6. Optionally choose **export type** (video MP4 or audio-only M4A), max height / include audio (video only), and output folder (defaults: 1080, audio on, `~/Movies/Slop Refs`).
7. Click **Export clip** / **Export audio**. Watch progress; on success the file is revealed in Finder (macOS) / equivalent opener elsewhere.

**Export types**

- **Video (MP4)** — H.264 + optional AAC.
- **Audio only (M4A)** — AAC soundtrack for the same in/out range.

Keyboard (when focus is not in an input):

- **Space** — play / pause  
- **←** / **→** — skip back / forward **5s** (same as the transport buttons)  
- **Shift+←** / **Shift+→** — skip **10s**  
- **I** — set in point to playhead  
- **O** — set out point to playhead  

Skip steps are fixed seconds (not a fraction of duration) so short reference clips and long videos both feel predictable — the usual web-player pattern (e.g. YouTube).

## Project layout

```
src/                 Svelte UI (URL bar, player, timeline, export panel)
src-tauri/           Rust commands (deps, metadata, preview, export, settings)
docs/superpowers/    Design spec and implementation plan
```

## License

[MIT](LICENSE)
