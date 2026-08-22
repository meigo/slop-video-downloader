# Windows support — design

**Date:** 2026-08-22
**Status:** Design (ready for implementation)
**Parent:** `docs/superpowers/specs/2026-07-26-slop-video-downloader-design.md`
**Reference:** `slop-video-compositor` — already ships Windows builds; its `release.yml` and
`deps.rs` are the template for this work.

## Motivation

The app is macOS-only. `slop-video-compositor` — same stack, same shape of external-tool
dependency — already ships Windows installers, so the pattern is proven in a sibling repo
rather than hypothetical.

Nothing in the app is inherently macOS-bound. It shells out to `yt-dlp` and `ffmpeg`, both
of which run on Windows, and the UI is a webview. What blocks Windows is a handful of
Unix assumptions in four files, plus a CI matrix that only lists Apple targets.

## Goals

1. Produce Windows `.exe` (NSIS) and `.msi` installers from CI on tag.
2. Tool detection, preview, export and reveal-in-folder work on Windows.
3. No console windows visible to the user during normal operation.
4. Install and upgrade hints name the right package manager per platform.
5. macOS behaviour unchanged — including its ad-hoc signature and first-launch flow.

## Non-goals (this phase)

- Linux builds (the code will be closer to it, but it is untested and unclaimed).
- Code-signing the Windows installer. It is unsigned; SmartScreen will warn.
- Bundling `yt-dlp`/`ffmpeg` as sidecars. Considered and rejected — see W10.
- A one-click `winget` install button inside the app. Considered and rejected — see W9.
- Declaring Windows "supported" in the README before a human has run it (see Risks).

## Decisions

| # | Decision | Choice |
|---|----------|--------|
| W1 | CI structure | Adopt the compositor's `release.yml` **wholesale**, not a minimal matrix addition |
| W2 | macOS signing | **Keep** `APPLE_SIGNING_IDENTITY: '-'` — the compositor omits it, and copying that would regress macOS first launch |
| W3 | Tool lookup | `#[cfg(windows)]` → `where`, else `which` (compositor's `which_on_path`) |
| W4 | PATH augmentation | `TOOL_DIRS`/`ensure_tool_path` become Unix-only; no-op on Windows |
| W5 | Console windows | All tool spawns route through one helper setting `CREATE_NO_WINDOW` on Windows |
| W6 | Platform text | Backend supplies hint strings; frontend renders them. No OS plugin in the frontend |
| W7 | Save directory | `dirs::video_dir()` with home fallback, keeping the `Slop Refs` subfolder |
| W8 | Order of work | CI **first**, smoke-tested via `workflow_dispatch`, before any Rust change |
| W9 | In-app `winget` install button | **Rejected** — adds elevation prompts, a progress/error surface and a failure path, all on a platform that cannot be tested here |
| W10 | Bundling tools as sidecars | **Rejected** — ~100MB, ffmpeg licensing/attribution duties, and it freezes yt-dlp at build time, which the staleness warning cannot then fix |

### W8 rationale

The current code *compiles* on Windows — `Command::new("which")` is a runtime failure, not
a build error. So the Windows toolchain, the MSVC target and the NSIS bundler can all be
proven green before a line of Rust is written. If the bundler needs coaxing, that surfaces
in one ~5-minute CI run instead of after the code work is done.

## Components

### 1. CI — `.github/workflows/release.yml`

Replace with the compositor's two-job structure:

- **`create-release`** (ubuntu): verifies `tag == package.json == Cargo.toml ==
  tauri.conf.json`, then creates **one** draft release and outputs its id.
- **`build`** (matrix): `macos-latest`/`aarch64-apple-darwin`,
  `macos-latest`/`x86_64-apple-darwin`, `windows-latest`/`x86_64-pc-windows-msvc`,
  each uploading to `releaseId`.

Two properties this buys beyond Windows:

- The current workflow has every matrix job pass `tagName` to `tauri-action`, so each job
  races to create the release. With two jobs it happens to work; a third makes it worse.
  Splitting creation out removes the race by construction.
- The version check would have caught a partial version bump before artifacts were built.

`workflow_dispatch` builds without creating or uploading anything — the smoke test W8 needs.

**Release body** merges the existing macOS text (including the v0.1.3 "keep yt-dlp current"
section) with Windows equivalents:

- `_x64-setup.exe` — the installer most people want
- `_x64_en-US.msi` — same app, for managed or scripted installs
- Requirements: `winget install yt-dlp.yt-dlp` and `winget install Gyan.FFmpeg`
- First launch: installer is unsigned, SmartScreen → **More info → Run anyway**

### 2. `src-tauri/src/deps.rs`

`which()` gains a `#[cfg(windows)]` arm calling `where` and taking the **first** line
(`where` prints every match). `TOOL_DIRS`, `augmented_path` and `ensure_tool_path` are
gated to Unix — Homebrew/MacPorts paths mean nothing on Windows, and `ensure_tool_path`
becomes an empty call there.

The three existing `augmented_path` tests move under `#[cfg(unix)]`.

### 3. `src-tauri/src/proc.rs` (new)

A GUI-subsystem process spawning a console process causes Windows to allocate a console
window for the child. `main.rs` already sets `windows_subsystem = "windows"` for the app
itself, but that does not cover children, and Rust's `Command` does not set
`CREATE_NO_WINDOW`. The app shells out on every user action — metadata, preview,
`--version`, and export with streaming progress — so the visible result is a black window
blinking repeatedly.

One module, one function:

```rust
/// Spawn helper for external tools. On Windows, suppresses the console
/// window that a GUI process would otherwise allocate for each child.
pub fn command(bin: &str) -> std::process::Command
```

All **seven** tool spawns route through it:

| File | Sites |
|------|-------|
| `ytdlp.rs` | `installed_ytdlp_version`, `fetch_metadata`, stream resolve, preview download |
| `export.rs` | `run_ytdlp`, ffmpeg invocation |
| `deps.rs` | `which`/`where` |

`settings.rs`'s `explorer` / `open` / `xdg-open` calls are **excluded** — those are supposed
to open a window.

### 4. `src-tauri/src/settings.rs`

`default_save_dir` hardcodes `~/Movies/Slop Refs`. Windows has no `Movies`. Use
`dirs::video_dir()`, falling back to the home directory, keeping the `Slop Refs` subfolder.

`reveal_in_folder` already branches to `explorer /select,` — no change needed.

### 5. Platform-specific hint text

`MissingDeps.svelte` hardcodes `brew install yt-dlp ffmpeg`, and the staleness banner in
`+page.svelte` hardcodes `brew upgrade yt-dlp`. Both are wrong on Windows.

The backend already knows the platform at compile time, so it supplies the strings rather
than the frontend detecting the OS. `DepsStatus` gains:

```rust
pub install_hint: String,   // "brew install yt-dlp ffmpeg" | "winget install yt-dlp.yt-dlp Gyan.FFmpeg"
pub upgrade_hint: String,   // "brew upgrade yt-dlp"        | "winget upgrade yt-dlp.yt-dlp"
```

Mirrored in `src/lib/types.ts`; both components render the field instead of a literal. No
new frontend dependency, and one source of truth for platform text.

### 6. `README.md`

Remove the macOS-only statement, add Windows requirements and the SmartScreen note.

## Data flow

Unchanged. The only structural change is that tool spawning gains an indirection
(`proc::command`) and `DepsStatus` carries two more strings. No new commands, no changes to
the preview/export pipeline, no frontend state changes.

## Error handling

No new error paths. Two existing ones change shape on Windows:

- `which()` returning `None` already drives the `MissingDeps` blocker; on Windows it now
  reflects a real `where` lookup instead of failing because `which` does not exist.
- `format_ytdlp_error` maps yt-dlp stderr to guidance. Its Vimeo/X cookie messages name
  Chrome, which is correct on Windows too.

## Testing

**Automated, runs on macOS:**

- Existing 46 Rust + 22 frontend tests stay green.
- `augmented_path` tests gated `#[cfg(unix)]`.
- `default_save_dir` gets a test asserting a non-empty absolute path ending in `Slop Refs`.
- `proc::command` gets a test asserting the returned `Command`'s program name, which
  compiles on both platforms and guards against the helper being bypassed.

**Automated, runs on Windows CI:**

- `workflow_dispatch` proves compile + bundle for `x86_64-pc-windows-msvc`.
- `cargo test` on the Windows runner catches `cfg` mistakes that macOS cannot.

**Manual, requires a Windows machine — not satisfiable from this workstation:**

- yt-dlp and ffmpeg resolve via `where`.
- Preview renders; export produces a correct clip.
- No console windows appear during fetch, preview or export.
- Reveal-in-folder selects the file in Explorer.

## Risks

| Risk | Severity | Notes |
|------|----------|-------|
| Backslash temp paths break `convertFileSrc` → `<video>` | **High** | The most likely first-run Windows bug. Preview writes to `%TEMP%\slop-video-downloader\…`; the asset protocol must round-trip that. Unverifiable from macOS. |
| Console windows still flash | Medium | `CREATE_NO_WINDOW` is the documented fix, but unverified here. The compositor does **not** set it, so it may have this bug too — worth checking there. |
| `--cookies-from-browser chrome` fails on Windows Chrome paths | Medium | Affects Vimeo/X only; YouTube is unaffected. yt-dlp supports Windows Chrome, but the Keychain-equivalent prompt differs. |
| NSIS bundler needs configuration | Low | Surfaces immediately in the W8 smoke test. |
| macOS regression from the workflow rewrite | Low | Mitigated by W2 and by the fact that v0.1.3 just built green — compare artifact names against that run. |

## Definition of done

- `workflow_dispatch` green for all three targets.
- A tagged build produces four macOS artifacts **and** `_x64-setup.exe` + `_x64_en-US.msi`.
- Rust + frontend tests green on macOS and on the Windows runner.
- README and release notes document Windows.
- Release published as a **draft** and exercised by a human on real Windows before the
  README claims support.

## Open question

No Windows hardware is available to the author. Everything above is buildable and testable
in CI except the four manual checks, which decide whether this is "Windows builds" or
"Windows works". The plan deliberately stops at a draft release for that reason.
