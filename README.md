# Slop Video Downloader

Desktop app that turns a public YouTube URL into a short, animator-friendly **H.264 + AAC MP4** clip for **slop-animator** reference layers.

Paste a URL → preview in-app → set in/out on a timeline → export a trimmed clip (default max height 1080p, audio on, save under `~/Movies/Slop Refs`).

**Stack:** Tauri 2 + SvelteKit + TypeScript, with **yt-dlp** and **ffmpeg** as external tools on your `PATH`.

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

**v1 notes**

- **YouTube only.** Non-YouTube sources are rejected.
- **yt-dlp and ffmpeg must be on `PATH`.** The app checks at startup and shows install guidance if either is missing. Binaries are not bundled in the installer.

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
3. Paste a **public YouTube URL** and click **Fetch**.
4. Wait for metadata and preview (stream first; falls back to a local file if streaming fails).
5. Set **in** and **out** on the timeline (drag handles, **Set In** / **Set Out**, or keys **I** / **O**).
6. Optionally adjust max height, include audio, and output folder (defaults: 1080, audio on, `~/Movies/Slop Refs`).
7. Click **Export clip**. Watch progress; on success the file is revealed in Finder (macOS) / equivalent opener elsewhere.

Keyboard (when focus is not in an input):

- **I** — set in point to playhead  
- **O** — set out point to playhead  
- **Space** — play / pause  

## Project layout

```
src/                 Svelte UI (URL bar, player, timeline, export panel)
src-tauri/           Rust commands (deps, metadata, preview, export, settings)
docs/superpowers/    Design spec and implementation plan
```

## License

MIT
