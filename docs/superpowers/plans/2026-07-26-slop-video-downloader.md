# Slop Video Downloader Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a macOS-first Tauri 2 + Svelte desktop app that previews a YouTube URL (stream, with local-download fallback), lets the user set in/out on a timeline, and exports an animator-ready H.264+AAC MP4 via yt-dlp + ffmpeg.

**Architecture:** Svelte UI in a Tauri webview invokes Rust commands that shell out to `yt-dlp` and `ffmpeg` on PATH. Pure helpers (URL validation, time formatting, filenames, CLI arg builders) are unit-tested without network. Hybrid preview tries a progressive stream URL first, then downloads a ≤720p local file. Export always finishes through ffmpeg into a clean MP4.

**Tech Stack:** Tauri 2, Svelte 5 (or scaffold default), TypeScript, Vite, Vitest (frontend pure helpers), Rust + cargo test (CLI builders / validation), yt-dlp, ffmpeg.

## Global Constraints

- YouTube only in v1 — reject other hosts with “YouTube only in v1”.
- External tools on PATH only — do not bundle yt-dlp/ffmpeg binaries.
- Default export: H.264 + AAC `.mp4`, max height 1080, audio on.
- Default save folder: last used, else `~/Movies/Slop Refs`.
- No live YouTube calls in CI/unit tests.
- macOS first; Finder reveal is the only Mac-specific convenience.
- Spec: `docs/superpowers/specs/2026-07-26-slop-video-downloader-design.md`.
- Preserve existing `docs/` when scaffolding; never delete design/plan docs.

## File structure

```
slop-video-downloader/
  package.json
  vite.config.ts
  vitest.config.ts
  index.html
  src/
    main.ts
    App.svelte
    app.css
    lib/
      youtube.ts          # isYouTubeUrl, normalizeYouTubeUrl
      time.ts             # formatTimestamp, parseTimestamp, clampRange
      filename.ts         # sanitizeTitle, buildClipFilename
      types.ts            # shared TS types mirroring Rust serde
      tauri.ts            # thin invoke wrappers
    components/
      UrlBar.svelte
      VideoPlayer.svelte
      Timeline.svelte
      ExportPanel.svelte
      MissingDeps.svelte
      StatusLine.svelte
  src-tauri/
    Cargo.toml
    tauri.conf.json
    capabilities/default.json
    src/
      lib.rs              # command registration
      main.rs
      deps.rs             # check_deps, which()
      youtube.rs          # URL validation (Rust side)
      ytdlp.rs            # metadata, preview resolve, download helpers
      export.rs           # export_clip pipeline + arg builders
      settings.rs         # load/save app settings
      filename.rs         # sanitize + clip filename (mirror TS)
  src-tauri/tests/        # or #[cfg(test)] modules beside sources
  README.md
  docs/superpowers/...    # already exists — keep
```

---

### Task 1: Scaffold Tauri 2 + Svelte app around existing docs

**Files:**
- Create: full Tauri/Svelte project files at repo root (via create-tauri-app or manual equivalent)
- Keep: `docs/superpowers/**`
- Create: `README.md` (minimal stub; expand in Task 10)

**Interfaces:**
- Produces: `npm run tauri dev` boots an empty window; `npm test` script placeholder OK until Task 2

- [ ] **Step 1: Scaffold into the repo without wiping docs**

The repo already has `docs/` and a git history. Scaffold carefully:

```bash
cd /Users/meigo/Projects/slop/slop-video-downloader

# If create-tauri-app refuses non-empty dirs, scaffold to a temp dir and merge:
npm create tauri-app@latest /tmp/slop-vdl-scaffold -- --template svelte-ts --manager npm --yes
# Copy scaffold files into this repo, DO NOT overwrite docs/
rsync -a --exclude docs --exclude .git /tmp/slop-vdl-scaffold/ ./
```

If flags differ on the installed create-tauri-app version, run interactively with:
- App name: `slop-video-downloader` / identifier `com.slop.video-downloader`
- Template: Svelte + TypeScript
- Package manager: npm

Set window title to `Slop Video Downloader` in `src-tauri/tauri.conf.json`.

- [ ] **Step 2: Install JS + ensure Rust project builds**

```bash
npm install
cd src-tauri && cargo check
```

Expected: cargo check succeeds (may download crates first).

- [ ] **Step 3: Smoke-run the empty app once**

```bash
npm run tauri dev
```

Expected: window opens with scaffold UI. Quit after confirming.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore: scaffold Tauri 2 + Svelte app"
```

---

### Task 2: Frontend pure helpers (URL, time, filename) + Vitest

**Files:**
- Create: `src/lib/youtube.ts`
- Create: `src/lib/time.ts`
- Create: `src/lib/filename.ts`
- Create: `src/lib/youtube.test.ts`
- Create: `src/lib/time.test.ts`
- Create: `src/lib/filename.test.ts`
- Create: `vitest.config.ts`
- Modify: `package.json` (add `vitest`, script `"test": "vitest run"`)

**Interfaces:**
- Produces:
  - `isYouTubeUrl(url: string): boolean`
  - `normalizeYouTubeUrl(url: string): string | null` — returns canonical `https://www.youtube.com/watch?v=ID` or null
  - `formatTimestamp(secs: number): string` — `MM:SS` or `HH:MM:SS` with optional tenths when needed; for filenames use separate helper
  - `formatFilenameTime(secs: number): string` — e.g. `01m12s` (whole seconds)
  - `clampRange(start: number, end: number, duration: number): { start: number; end: number }`
  - `sanitizeTitle(title: string): string`
  - `buildClipFilename(title: string, startSecs: number, endSecs: number): string` — `{sanitized}_{start}-{end}.mp4`

- [ ] **Step 1: Add Vitest**

```bash
npm install -D vitest
```

`vitest.config.ts`:

```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
```

In `package.json` scripts: `"test": "vitest run"`.

- [ ] **Step 2: Write failing tests**

`src/lib/youtube.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { isYouTubeUrl, normalizeYouTubeUrl } from "./youtube";

describe("isYouTubeUrl", () => {
  it("accepts watch, youtu.be, shorts", () => {
    expect(isYouTubeUrl("https://www.youtube.com/watch?v=dQw4w9WgXcQ")).toBe(true);
    expect(isYouTubeUrl("https://youtu.be/dQw4w9WgXcQ")).toBe(true);
    expect(isYouTubeUrl("https://www.youtube.com/shorts/dQw4w9WgXcQ")).toBe(true);
  });
  it("rejects non-YouTube", () => {
    expect(isYouTubeUrl("https://vimeo.com/123")).toBe(false);
    expect(isYouTubeUrl("not a url")).toBe(false);
  });
});

describe("normalizeYouTubeUrl", () => {
  it("normalizes youtu.be to watch URL", () => {
    expect(normalizeYouTubeUrl("https://youtu.be/dQw4w9WgXcQ")).toBe(
      "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
    );
  });
  it("returns null for non-YouTube", () => {
    expect(normalizeYouTubeUrl("https://example.com")).toBeNull();
  });
});
```

`src/lib/time.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { clampRange, formatFilenameTime, formatTimestamp } from "./time";

describe("formatTimestamp", () => {
  it("formats under an hour as MM:SS", () => {
    expect(formatTimestamp(72)).toBe("01:12");
  });
  it("formats hours", () => {
    expect(formatTimestamp(3661)).toBe("1:01:01");
  });
});

describe("formatFilenameTime", () => {
  it("uses m/s tokens", () => {
    expect(formatFilenameTime(72)).toBe("01m12s");
    expect(formatFilenameTime(3661)).toBe("01h01m01s");
  });
});

describe("clampRange", () => {
  it("ensures end > start within duration", () => {
    expect(clampRange(-1, 500, 100)).toEqual({ start: 0, end: 100 });
    expect(clampRange(50, 40, 100)).toEqual({ start: 40, end: 50 });
  });
});
```

`src/lib/filename.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { buildClipFilename, sanitizeTitle } from "./filename";

describe("sanitizeTitle", () => {
  it("strips unsafe characters", () => {
    expect(sanitizeTitle('My Video: "Cool"/Thing?')).toBe("My Video Cool Thing");
  });
  it("collapses whitespace and trims", () => {
    expect(sanitizeTitle("  a   b  ")).toBe("a b");
  });
});

describe("buildClipFilename", () => {
  it("builds title_start-end.mp4", () => {
    expect(buildClipFilename("Hello World", 72, 105)).toBe(
      "Hello World_01m12s-01m45s.mp4",
    );
  });
});
```

- [ ] **Step 3: Run tests — expect fail**

```bash
npm test
```

Expected: FAIL (modules missing).

- [ ] **Step 4: Implement helpers**

`src/lib/youtube.ts` — parse with `URL`, allow hosts `youtube.com`, `www.youtube.com`, `m.youtube.com`, `youtu.be`, `www.youtu.be`; extract 11-char video id from `v=`, path for youtu.be, or `/shorts/ID`.

`src/lib/time.ts` — floor seconds for display/filename; `clampRange` swaps if start>end, clamps to `[0, duration]`, if equal bump end by `min(0.1, duration-start)` or if at end shrink start.

`src/lib/filename.ts` — replace `/\\?%*:|"<>` and control chars with space; collapse spaces; fallback title `clip` if empty after sanitize.

- [ ] **Step 5: Run tests — expect pass**

```bash
npm test
```

Expected: all PASS.

- [ ] **Step 6: Commit**

```bash
git add package.json package-lock.json vitest.config.ts src/lib
git commit -m "feat: add URL, time, and filename helpers with tests"
```

---

### Task 3: Rust dependency check + YouTube URL validation

**Files:**
- Create: `src-tauri/src/deps.rs`
- Create: `src-tauri/src/youtube.rs`
- Create: `src-tauri/src/filename.rs`
- Modify: `src-tauri/src/lib.rs` (register modules + `check_deps` command)
- Modify: `src-tauri/Cargo.toml` if serde types need extra features (usually already present via tauri)

**Interfaces:**
- Produces:
  - `#[tauri::command] fn check_deps() -> DepsStatus`
  - `pub struct DepsStatus { pub ytdlp: bool, pub ffmpeg: bool, pub ytdlp_path: Option<String>, pub ffmpeg_path: Option<String> }`
  - `pub fn is_youtube_url(url: &str) -> bool`
  - `pub fn normalize_youtube_url(url: &str) -> Option<String>`
  - `pub fn sanitize_title(title: &str) -> String`
  - `pub fn build_clip_filename(title: &str, start_secs: f64, end_secs: f64) -> String`
  - `pub fn format_filename_time(secs: f64) -> String`

- [ ] **Step 1: Write Rust unit tests (fail first)**

In `youtube.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_watch_and_short() {
        assert!(is_youtube_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(is_youtube_url("https://youtu.be/dQw4w9WgXcQ"));
    }

    #[test]
    fn rejects_vimeo() {
        assert!(!is_youtube_url("https://vimeo.com/123"));
    }

    #[test]
    fn normalizes_youtu_be() {
        assert_eq!(
            normalize_youtube_url("https://youtu.be/dQw4w9WgXcQ").as_deref(),
            Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
        );
    }
}
```

In `filename.rs` tests: same expectations as TS (`Hello World_01m12s-01m45s.mp4`).

- [ ] **Step 2: Run tests — expect fail**

```bash
cd src-tauri && cargo test
```

Expected: compile fail or assertion fail until implemented.

- [ ] **Step 3: Implement `deps.rs`**

```rust
use serde::Serialize;
use std::process::Command;

#[derive(Debug, Serialize)]
pub struct DepsStatus {
    pub ytdlp: bool,
    pub ffmpeg: bool,
    pub ytdlp_path: Option<String>,
    pub ffmpeg_path: Option<String>,
}

fn which(bin: &str) -> Option<String> {
    // Prefer `which` on macOS/Linux
    let output = Command::new("which").arg(bin).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() { None } else { Some(path) }
}

#[tauri::command]
pub fn check_deps() -> DepsStatus {
    let ytdlp_path = which("yt-dlp");
    let ffmpeg_path = which("ffmpeg");
    DepsStatus {
        ytdlp: ytdlp_path.is_some(),
        ffmpeg: ffmpeg_path.is_some(),
        ytdlp_path,
        ffmpeg_path,
    }
}
```

Implement youtube/filename pure functions to match TS behavior.

Wire in `lib.rs`:

```rust
mod deps;
mod youtube;
mod filename;

use deps::check_deps;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init()) // only if scaffold includes it; else skip
        .invoke_handler(tauri::generate_handler![check_deps])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Adjust to match scaffold’s actual `lib.rs` / `main.rs` pattern (Tauri 2 often uses `lib.rs` `run()`).

- [ ] **Step 4: Run tests — expect pass**

```bash
cd src-tauri && cargo test
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri
git commit -m "feat: add check_deps and Rust URL/filename helpers"
```

---

### Task 4: `fetch_metadata` via yt-dlp

**Files:**
- Create: `src-tauri/src/ytdlp.rs`
- Modify: `src-tauri/src/lib.rs` (register `fetch_metadata`)
- Create: `src/lib/types.ts`
- Create: `src/lib/tauri.ts`

**Interfaces:**
- Consumes: `youtube::normalize_youtube_url`
- Produces:
  - `#[tauri::command] async fn fetch_metadata(url: String) -> Result<VideoMeta, String>`
  - `pub struct VideoMeta { pub id: String, pub title: String, pub duration_secs: f64, pub thumbnail_url: Option<String> }`
  - TS: `fetchMetadata(url: string): Promise<VideoMeta>`

**Notes:** Implementation shells out:

```bash
yt-dlp -J --no-playlist -- "<url>"
```

Parse JSON for `id`, `title`, `duration`, `thumbnail`. On non-YouTube URL return `Err("YouTube only in v1".into())`. On process failure return truncated stderr.

- [ ] **Step 1: Write unit test for arg builder only (no network)**

In `ytdlp.rs`:

```rust
pub fn metadata_args(url: &str) -> Vec<String> {
    vec![
        "-J".into(),
        "--no-playlist".into(),
        "--".into(),
        url.into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_args_shape() {
        let a = metadata_args("https://www.youtube.com/watch?v=dQw4w9WgXcQ");
        assert_eq!(a[0], "-J");
        assert!(a.contains(&"--no-playlist".to_string()));
    }
}
```

- [ ] **Step 2: Implement `fetch_metadata`**

```rust
#[tauri::command]
pub async fn fetch_metadata(url: String) -> Result<VideoMeta, String> {
    let url = crate::youtube::normalize_youtube_url(&url)
        .ok_or_else(|| "YouTube only in v1".to_string())?;
    let output = tokio::process::Command::new("yt-dlp")
        .args(metadata_args(&url))
        .output()
        .await
        .map_err(|e| format!("Failed to run yt-dlp: {e}"))?;
    if !output.status.success() {
        return Err(truncate_err(&String::from_utf8_lossy(&output.stderr)));
    }
    parse_metadata_json(&output.stdout)
}
```

Use `serde_json` to parse. If `tokio` isn’t already a dependency via Tauri, use `std::process::Command` in `spawn_blocking` instead — prefer whatever the scaffold already enables. Tauri 2 async commands work with async runtimes Tauri provides; `std::process::Command` inside `tauri::async_runtime::spawn_blocking` is the safest portable choice:

```rust
tauri::async_runtime::spawn_blocking(move || {
    std::process::Command::new("yt-dlp").args(metadata_args(&url)).output()
})
.await
.map_err(|e| e.to_string())?
.map_err(|e| format!("Failed to run yt-dlp: {e}"))?;
```

- [ ] **Step 3: Add TS wrappers**

`src/lib/types.ts`:

```ts
export type DepsStatus = {
  ytdlp: boolean;
  ffmpeg: boolean;
  ytdlp_path: string | null;
  ffmpeg_path: string | null;
};

export type VideoMeta = {
  id: string;
  title: string;
  duration_secs: number;
  thumbnail_url: string | null;
};

export type PreviewResult = {
  mode: "stream" | "file";
  url_or_path: string;
  note?: string | null;
};

export type ExportOpts = {
  url: string;
  start_secs: number;
  end_secs: number;
  max_height: number | null; // null = source
  include_audio: boolean;
  out_dir: string;
};

export type ExportResult = { output_path: string };
```

`src/lib/tauri.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";
import type { DepsStatus, ExportOpts, ExportResult, PreviewResult, VideoMeta } from "./types";

export const checkDeps = () => invoke<DepsStatus>("check_deps");
export const fetchMetadata = (url: string) => invoke<VideoMeta>("fetch_metadata", { url });
export const resolvePreview = (url: string) => invoke<PreviewResult>("resolve_preview", { url });
export const exportClip = (opts: ExportOpts) => invoke<ExportResult>("export_clip", { opts });
```

Register `fetch_metadata` in the invoke handler.

- [ ] **Step 4: cargo test**

```bash
cd src-tauri && cargo test
```

Expected: PASS (arg builder + existing tests). Manual metadata fetch is deferred to Task 8.

- [ ] **Step 5: Commit**

```bash
git add src-tauri src/lib/types.ts src/lib/tauri.ts
git commit -m "feat: fetch YouTube metadata via yt-dlp"
```

---

### Task 5: Hybrid `resolve_preview`

**Files:**
- Modify: `src-tauri/src/ytdlp.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Produces: `#[tauri::command] async fn resolve_preview(url: String) -> Result<PreviewResult, String>`
- `PreviewResult { mode: "stream" | "file", url_or_path: String, note: Option<String> }`

**Algorithm:**

1. Normalize YouTube URL (else “YouTube only in v1”).
2. Try progressive stream:
   ```bash
   yt-dlp -g -f "b[ext=mp4]/best[ext=mp4]/best" --no-playlist -- "<url>"
   ```
   If stdout has a single `http` URL, return `{ mode: "stream", url_or_path: url, note: null }`.
   If multiple lines (separate A/V), treat as failure for stream mode.
3. Fallback download to temp:
   ```bash
   yt-dlp -f "bv*[height<=720]+ba/b[height<=720]/best[height<=720]/best" \
     --no-playlist -o "<tempdir>/%(id)s_preview.%(ext)s" --merge-output-format mp4 -- "<url>"
   ```
   Return `{ mode: "file", url_or_path: path, note: Some("Stream preview unavailable — using local preview file.") }`.

Use `std::env::temp_dir().join("slop-video-downloader")` and create it if needed.

- [ ] **Step 1: Unit-test arg builders**

```rust
pub fn stream_url_args(url: &str) -> Vec<String> { /* -g -f ... */ }
pub fn preview_download_args(url: &str, out_template: &str) -> Vec<String> { /* ... */ }

#[test]
fn stream_args_include_g() {
    assert!(stream_url_args("https://www.youtube.com/watch?v=x").contains(&"-g".into()));
}
```

- [ ] **Step 2: Implement `resolve_preview` with stream-then-file**

Convert local file paths for the webview: Tauri 2 needs `convertFileSrc` on the frontend for `mode: "file"`. Backend returns a filesystem path; UI will convert. Document this in a code comment on `PreviewResult`.

- [ ] **Step 3: cargo test + commit**

```bash
cd src-tauri && cargo test
git add src-tauri
git commit -m "feat: hybrid stream/file preview resolution"
```

---

### Task 6: `export_clip` pipeline + ffmpeg/yt-dlp arg builders

**Files:**
- Create: `src-tauri/src/export.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/capabilities/default.json` if needed for fs/dialog later (Task 9)

**Interfaces:**
- Produces:
  - `#[tauri::command] async fn export_clip(app: AppHandle, opts: ExportOpts) -> Result<ExportResult, String>`
  - Progress: emit event `"export-progress"` with payload `{ phase: string, message: string, pct: Option<f64> }`
  - `ExportOpts { url, start_secs, end_secs, max_height: Option<u32>, include_audio: bool, out_dir: String }`
  - `ExportResult { output_path: String }`

**Pipeline:**

1. Validate URL, `end_secs > start_secs`, `out_dir` exists or create it.
2. Build output path: `out_dir / build_clip_filename(title, start, end)` — title from a quick `-J` or require title in opts. **Add optional `title: String` to ExportOpts** so the UI can pass metadata title without a second fetch. Spec opts listed without title; extend as:

```rust
pub struct ExportOpts {
    pub url: String,
    pub title: String,
    pub start_secs: f64,
    pub end_secs: f64,
    pub max_height: Option<u32>, // None = source
    pub include_audio: bool,
    pub out_dir: String,
}
```

Update `src/lib/types.ts` to match.

3. Temp work dir under `temp/slop-video-downloader/export-<uuid>/`.
4. Download section with yt-dlp:
   ```bash
   yt-dlp --no-playlist \
     --download-sections "*START-END" \
     -f "bv*+ba/b" \
     --merge-output-format mp4 \
     -o "TEMP/raw.%(ext)s" \
     -- "<url>"
   ```
   Format `START`/`END` as seconds with `*72.0-105.0` style (`--download-sections "*72-105"`).

5. Run ffmpeg to final MP4:
   - Always re-encode video to H.264 (`libx264 -preset veryfast -crf 20`) and audio to AAC when `include_audio`, for reliable browser playback.
   - Scale: if `max_height` is `Some(h)`, use  
     `scale=-2:'min(h,ih)'` (no upscale).
   - If `!include_audio`: `-an`.
   - `-movflags +faststart`.
   - Write to final path; if final path exists, append ` (1)` before extension.

6. Emit progress phases: `download`, `transcode`, `done`.
7. Delete temp dir on success. On failure, leave temp and return error string.

- [ ] **Step 1: Failing tests for arg builders**

```rust
#[test]
fn download_sections_format() {
    let args = section_download_args(
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
        72.0,
        105.0,
        "/tmp/raw.%(ext)s",
    );
    assert!(args.iter().any(|a| a.contains("--download-sections")));
    assert!(args.iter().any(|a| a == "*72-105" || a.contains("*72")));
}

#[test]
fn ffmpeg_scales_and_strips_audio() {
    let args = ffmpeg_transcode_args(
        "/tmp/in.mp4",
        "/tmp/out.mp4",
        Some(720),
        false,
    );
    assert!(args.iter().any(|a| a.contains("scale=")));
    assert!(args.iter().any(|a| a == "-an"));
}
```

- [ ] **Step 2: Implement builders + `export_clip`**

- [ ] **Step 3: cargo test**

```bash
cd src-tauri && cargo test
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri src/lib/types.ts src/lib/tauri.ts
git commit -m "feat: export clip via yt-dlp section download and ffmpeg"
```

---

### Task 7: Settings persistence

**Files:**
- Create: `src-tauri/src/settings.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/types.ts`, `src/lib/tauri.ts`

**Interfaces:**
- Produces:
  - `pub struct AppSettings { pub last_save_dir: Option<String>, pub max_height: Option<u32>, pub include_audio: bool }`
  - Defaults: `max_height: Some(1080)`, `include_audio: true`, `last_save_dir: None`
  - `#[tauri::command] fn load_settings(app: AppHandle) -> AppSettings`
  - `#[tauri::command] fn save_settings(app: AppHandle, settings: AppSettings) -> Result<(), String>`
  - Store JSON in app config dir: `app.path().app_config_dir()/settings.json` (Tauri `path` plugin / `AppHandle` path resolver).

Enable `tauri-plugin-store` **or** plain `std::fs` + serde_json — prefer plain fs to avoid extra plugins:

```rust
fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}
```

Requires `tauri` features for path protocol — use whatever scaffold enables (`app.path()` needs `tauri::Manager` + path feature; Tauri 2 includes this on `AppHandle` with `tauri::path::PathResolver`).

Also add helper command:

```rust
#[tauri::command]
fn default_save_dir() -> String {
    // ~/Movies/Slop Refs
    let home = dirs::home_dir().unwrap_or_default();
    home.join("Movies").join("Slop Refs").to_string_lossy().into()
}
```

Add `dirs = "5"` to `Cargo.toml` if used.

- [ ] **Step 1: Implement load/save + defaults unit test (temp path injection optional)**

Test pure default merge:

```rust
#[test]
fn defaults_audio_on_1080() {
    let s = AppSettings::default();
    assert_eq!(s.max_height, Some(1080));
    assert!(s.include_audio);
}
```

- [ ] **Step 2: Register commands, update TS types**

- [ ] **Step 3: cargo test + commit**

```bash
cd src-tauri && cargo test
git add src-tauri src/lib
git commit -m "feat: persist export settings"
```

---

### Task 8: UI shell — deps gate, URL fetch, status, layout

**Files:**
- Modify: `src/App.svelte`
- Create: `src/components/MissingDeps.svelte`
- Create: `src/components/UrlBar.svelte`
- Create: `src/components/StatusLine.svelte`
- Create: `src/components/ExportPanel.svelte` (controls only; wire export later)
- Modify: `src/app.css` (simple dark UI, readable, no heavy design system)

**Interfaces:**
- Consumes: `checkDeps`, `fetchMetadata`, `resolvePreview`, `loadSettings`
- Produces: App state holding `deps`, `url`, `meta`, `preview`, `status`, `error`, in/out, settings

- [ ] **Step 1: On mount, `checkDeps()`**

If `!ytdlp || !ffmpeg`, render `MissingDeps` full-screen:

```
Missing tools
This app needs yt-dlp and ffmpeg on your PATH.

  brew install yt-dlp ffmpeg

yt-dlp: not found | found at …
ffmpeg: …
[Recheck]
```

- [ ] **Step 2: Main layout when deps OK**

Wire UrlBar: text input + Fetch button. On Fetch:

1. Client-validate with `isYouTubeUrl` — else set error “Enter a valid YouTube URL”.
2. Status: “Fetching metadata…”
3. `fetchMetadata(url)` → store meta
4. Status: “Resolving preview…”
5. `resolvePreview(url)` → store preview; if note, show it on StatusLine
6. Status: “Ready”
7. Reset in=0, out=duration

- [ ] **Step 3: ExportPanel fields**

- Show In/Out/Duration text (formatted)
- Max height select: 480, 720, 1080, Source
- Include audio checkbox
- Save to: path text + note “chosen folder” (folder dialog in Task 9; for now editable text defaulting to settings/default)
- Export button disabled until meta+preview ready and end>start

- [ ] **Step 4: Manual smoke**

```bash
# Install yt-dlp if missing:
brew install yt-dlp

npm run tauri dev
```

Expected: missing-deps appears if yt-dlp absent; after install, fetch a short public video shows title/duration/status. Preview player can be a placeholder `<video>` tag not fully wired until Task 9.

- [ ] **Step 5: Commit**

```bash
git add src
git commit -m "feat: UI shell with dependency gate and metadata fetch"
```

---

### Task 9: Video player, timeline, keyboard, export UX

**Files:**
- Create: `src/components/VideoPlayer.svelte`
- Create: `src/components/Timeline.svelte`
- Modify: `src/App.svelte`
- Modify: `src-tauri` for dialog + reveal if needed
- Modify: capabilities for `dialog`, `fs` as required by Tauri 2 plugins

**Interfaces:**
- VideoPlayer: props `src: string`, binds `currentTime`, exposes play/pause
- Timeline: props `duration`, `currentTime`, `inPoint`, `outPoint`; events for change
- Export: calls `exportClip`, listens `export-progress`, reveals file

- [ ] **Step 1: VideoPlayer**

```svelte
<video
  bind:this={el}
  src={displaySrc}
  on:timeupdate
  on:error={onError}
  controls={false}
/>
```

For `preview.mode === "file"`, use:

```ts
import { convertFileSrc } from "@tauri-apps/api/core";
displaySrc = convertFileSrc(preview.url_or_path);
```

On `error` event when mode was `stream`, App should call `resolve_preview` again is wrong (already hybrid on backend). Instead: backend already fell back when stream URL discovery failed. If player errors on stream URL, App invokes a new command or re-calls with force file — **add optional** `resolve_preview(url, force_file: bool)` default false; on video error with mode stream, call `resolvePreview(url, true)`.

Minimal approach without API change: on video error + stream mode, invoke `export` is wrong. Implement:

```rust
#[tauri::command]
async fn resolve_preview(url: String, force_file: Option<bool>) -> Result<PreviewResult, String>
```

When `force_file == Some(true)`, skip stream attempt.

- [ ] **Step 2: Timeline**

Horizontal bar 0…duration:

- Click/drag playhead → seek video
- Draggable in/out handles (or click + Set In/Out buttons)
- Highlight selected range
- Buttons: Set In, Set Out, Play selection (loop playhead between in/out until pause)

- [ ] **Step 3: Keyboard**

On `window` keydown when not typing in an input:

- `i` / `I` → in = currentTime
- `o` / `O` → out = currentTime  
- `Space` → toggle play/pause (preventDefault)

- [ ] **Step 4: Export + progress + reveal**

```ts
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog"; // if using plugin
```

Install plugins as needed:

```bash
cd src-tauri
cargo add tauri-plugin-dialog
# frontend: npm install @tauri-apps/plugin-dialog
```

Folder picker sets `out_dir`. On Export:

1. Disable button, clear error
2. Listen `export-progress`
3. `exportClip({ url, title, start_secs, end_secs, max_height, include_audio, out_dir })`
4. On success: status “Saved: …”, call reveal:

```rust
#[tauri::command]
fn reveal_in_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &path])
            .status()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(not(target_os = "macos"))]
    {
        // open parent directory with system default
        let parent = std::path::Path::new(&path).parent().unwrap_or(Path::new("."));
        open::that(parent).map_err(|e| e.to_string())
    }
}
```

(For non-macOS, either use `open` crate or skip; macOS path is required.)

5. Save settings (`last_save_dir`, max_height, include_audio).

- [ ] **Step 5: Manual end-to-end**

```bash
npm run tauri dev
```

Checklist from spec:

1. Public short video stream or fallback preview works  
2. Set in/out, export 10–30s clip  
3. Open MP4 in QuickTime/Safari  
4. Load file as video reference in slop-animator  
5. Toggle audio off / max height 720 and re-export once  

- [ ] **Step 6: Commit**

```bash
git add src src-tauri package.json package-lock.json
git commit -m "feat: player, timeline, and export UX"
```

---

### Task 10: README, polish, final verification

**Files:**
- Create/Modify: `README.md`
- Modify: minor UI copy/CSS if needed
- Modify: `src-tauri/tauri.conf.json` product name if still generic

- [ ] **Step 1: Write README**

Include:

- What it is (YouTube → clip MP4 for slop-animator)
- Requirements: Node, Rust, yt-dlp, ffmpeg (`brew install yt-dlp ffmpeg`)
- Dev: `npm install`, `npm run tauri dev`
- Test: `npm test`, `cd src-tauri && cargo test`
- Usage: paste URL → Fetch → set in/out → Export
- Note: YouTube only v1; tools must be on PATH

- [ ] **Step 2: Run full automated suite**

```bash
npm test
cd src-tauri && cargo test
```

Expected: all PASS.

- [ ] **Step 3: Final manual pass on success criteria**

Confirm all four success criteria from the design spec.

- [ ] **Step 4: Commit**

```bash
git add README.md src src-tauri
git commit -m "docs: README and v1 polish"
```

---

## Self-review (plan vs spec)

| Spec requirement | Task |
|------------------|------|
| Desktop GUI Tauri + Svelte | 1, 8–9 |
| Hybrid stream → file preview | 5, 9 (force_file on player error) |
| Timeline in/out + keyboard I/O/Space | 9 |
| Export H.264+AAC MP4, max height, audio toggle | 6, 9 |
| Defaults 1080 / audio on / ~/Movies/Slop Refs | 7, 8 |
| YouTube only | 2, 3, 4 |
| check_deps + missing tools UI | 3, 8 |
| Settings persistence | 7 |
| Progress + Finder reveal | 6, 9 |
| Unit tests without live YouTube | 2, 3, 4, 5, 6 |
| README | 10 |
| Portable path (no Mac-only core logic) | 3–6; reveal cfg-gated |

**Type consistency notes:**

- `ExportOpts` includes `title: String` (UI-supplied) — extension beyond the one-line spec list; required to avoid a redundant metadata fetch at export.
- `resolve_preview(url, force_file?: bool)` — small extension for player-level stream failure.
- Progress event name: `export-progress`.

**Placeholder scan:** none intentional; scaffold flag differences handled with fallback instructions in Task 1.
)
