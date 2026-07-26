# Multi-source allowlist + export fallback — design

**Date:** 2026-07-26  
**Status:** Design (ready for implementation)  
**Parent:** `docs/superpowers/specs/2026-07-26-slop-video-downloader-design.md` (v1 YouTube-only)

## Motivation

v1 rejects every non-YouTube URL before yt-dlp runs. yt-dlp already supports many sites; the app only needs safer gating, clearer errors, and a download path that works when YouTube-style `--download-sections` does not.

## Goals

1. Accept **public** URLs from a small **allowlist** of hosts (YouTube + Vimeo first).
2. Keep timeline / export UX unchanged (in/out, video MP4 / audio M4A).
3. Prefer **section download** when it works; **fallback** to full download + ffmpeg trim.
4. Fail with short, user-facing errors (not raw stack traces).
5. Stay **no-login / no-cookies UI** in this phase (public videos only).

## Non-goals (this phase)

- Instagram, TikTok, X/Twitter (add later if allowlist expansion is deliberate)
- Cookies, browser import, or age-gate UI
- “Any yt-dlp URL” free-for-all
- Playlist / batch import
- Site-specific format pickers

## Decisions

| # | Decision | Choice |
|---|----------|--------|
| M1 | Gate style | **Host allowlist** (not open-world) |
| M2 | v1.1 hosts | **YouTube** (existing) + **Vimeo** (`vimeo.com`, `player.vimeo.com`) |
| M3 | URL handling | Validate host → pass **normalized URL** to yt-dlp (no YouTube-only id rewrite for other sites) |
| M4 | Export download | **Try section download** → on failure **full download + ffmpeg `-ss`/`-to`** |
| M5 | Preview | Keep hybrid stream → file; on stream failure always local file (existing). No section logic in preview. |
| M6 | Errors | Map common yt-dlp failures to short strings; include “update yt-dlp” when useful |
| M7 | Expansion | New hosts = config table + smoke test note, not new pipeline |

## Allowlist

### Host table (v1.1)

| Family | Allowed hosts (exact / suffix) | Notes |
|--------|--------------------------------|--------|
| YouTube | `youtube.com`, `www.youtube.com`, `m.youtube.com`, `youtu.be`, `www.youtu.be` | Keep existing id normalize for YouTube |
| Vimeo | `vimeo.com`, `www.vimeo.com`, `player.vimeo.com` | Public only; password/private → yt-dlp error |

Matching: parse with `URL` / hand-rolled parse; compare **lowercase host**; reject non-http(s).

### Rejected examples

- `instagram.com`, `tiktok.com`, `x.com`, `twitter.com` (until allowlisted)
- Bare text, `ftp://`, missing host
- Playlists / channels as primary input (still `--no-playlist` on single-item pages when possible)

### User-facing reject copy

```
Unsupported site in this version. Supported: YouTube, Vimeo (public videos).
```

(Replace old: `YouTube only in v1`.)

## URL module redesign

### Today

- TS: `src/lib/youtube.ts` — YouTube only  
- Rust: `src-tauri/src/youtube.rs` — YouTube only  
- Commands call `normalize_youtube_url` or error

### Target

Rename conceptually to **source URL** helpers (files may stay named `youtube` short-term or move to `source.rs` / `source.ts`).

```ts
// src/lib/source.ts (new) or extended youtube.ts
export type SourceFamily = "youtube" | "vimeo";

export function isSupportedUrl(url: string): boolean;
export function normalizeSourceUrl(url: string): string | null;
// YouTube → https://www.youtube.com/watch?v=ID
// Vimeo  → https://vimeo.com/{id} when parseable; else original https URL if host allowed
export function sourceFamily(url: string): SourceFamily | null;
```

```rust
// src-tauri/src/source.rs (preferred) or extend youtube.rs
pub fn is_supported_url(url: &str) -> bool;
pub fn normalize_source_url(url: &str) -> Option<String>;
```

**Commands** (`fetch_metadata`, `resolve_preview`, `export_clip`):

```rust
let url = normalize_source_url(&url)
    .ok_or_else(|| "Unsupported site in this version. Supported: YouTube, Vimeo (public videos).".to_string())?;
```

UI Fetch: use `isSupportedUrl` instead of `isYouTubeUrl`; placeholder text: `Paste a YouTube or Vimeo URL…`.

## Export pipeline (section + fallback)

### Current (YouTube-centric)

```
yt-dlp --download-sections "*START-END" -f bv*+ba/b → raw.mp4
ffmpeg → final mp4/m4a
```

### Target

```
1) Try section download (same as today)
   → success: find local raw.* → ffmpeg as today

2) Else full download to temp (no --download-sections)
   yt-dlp -f "bv*+ba/b" --merge-output-format mp4 -o "raw_full.%(ext)s" -- URL
   → ffmpeg trim:
        video: -ss START -to END (or -ss + -t DURATION) + existing H.264 path
        audio: -ss START -to END -vn -c:a aac
```

### ffmpeg trim args (fallback only)

Prefer **output seeking** for accuracy after full download:

```text
ffmpeg -y -i INPUT -ss START -to END ...encoders... OUTPUT
```

(`-ss` after `-i` is slower but more accurate; fine for short reference clips.)

Emit progress phases:

| Phase | Message |
|-------|---------|
| `download` | Downloading clip section… / Section download failed — downloading full video… |
| `transcode` | Transcoding… / Trimming… |
| `done` | Export complete |

### When to treat section as failed

- yt-dlp non-zero exit  
- No local `raw.*` file, or empty/tiny file  
- Optional: stderr contains known “Unsupported URL” / section errors  

Do **not** pass `http(s)` URLs into ffmpeg (keep existing local-file guard).

### Unit tests (no network)

- Allowlist: accept YT + Vimeo hosts; reject IG/TikTok/random  
- YouTube normalize still canonical  
- Vimeo normalize: `https://vimeo.com/123456789` stays valid https  
- `section_download_args` unchanged shape  
- New `full_download_args(url, out_template)`  
- New `ffmpeg_trim_*_args` include `-ss` / `-to`  

## Preview (light touch)

Keep hybrid:

1. `yt-dlp -g` progressive → stream mode  
2. Else local ≤720p download  

For non-YouTube, stream mode will fail more often → file mode is the normal path. No extra allowlist logic beyond `normalize_source_url` at the command entry.

## Metadata

`yt-dlp -J` already returns `id`, `title`, `duration`, `thumbnail` for Vimeo.  
`parse_metadata_json` stays; ensure missing `duration` errors clearly.

Filename: existing `sanitize_title` + time tokens + ext — works for any title.

## UI copy

| Location | Copy |
|----------|------|
| URL placeholder | Paste a YouTube or Vimeo URL… |
| Reject | Unsupported site… Supported: YouTube, Vimeo (public videos). |
| Missing tools | unchanged |
| README | Supported sources: YouTube, Vimeo (public). More sites may be added later. |

## Expanding the allowlist later

Add a host to a single table (TS + Rust mirror) + one manual smoke:

1. Public sample URL  
2. Fetch metadata  
3. Preview (file OK)  
4. Export 10s video + audio  

Sites that need cookies stay **out** until a cookies design exists.

### Suggested later candidates (not in v1.1)

| Host | Risk |
|------|------|
| `dailymotion.com` | Medium |
| `twitch.tv` (clips/VODs) | Medium–high |
| `x.com` / `twitter.com` | High (login, rate limits) |
| `instagram.com`, `tiktok.com` | High (login, short-lived URLs) |

## Implementation plan (tasks)

### Task 1 — Source allowlist helpers + tests

- Add `source.ts` / `source.rs` (or extend youtube modules)  
- Tests for YT + Vimeo accept; foreign reject  
- Wire UI + three Rust commands to new normalize  

### Task 2 — Export full-download fallback

- `full_download_args`  
- On section failure → full download → ffmpeg trim  
- Progress messages  
- Arg-builder unit tests  

### Task 3 — Copy + README

- Placeholder, errors, supported-sources section  
- Note: public videos only; update yt-dlp if a site breaks  

### Task 4 — Manual smoke

| # | Check |
|---|--------|
| 1 | YouTube short clip: section path still works |
| 2 | Public Vimeo: fetch + preview + 15s video export |
| 3 | Same Vimeo: audio-only M4A |
| 4 | Instagram URL: clear unsupported message |
| 5 | Private/password Vimeo: readable yt-dlp error (no crash) |

## Risks

| Risk | Mitigation |
|------|------------|
| Section fails often on Vimeo | Fallback full download |
| Full download of long VOD is slow | User sets short in/out; still downloads full file in fallback — **document** this |
| yt-dlp breaks a site overnight | README: `brew upgrade yt-dlp` |
| Legal/ToS | Public allowlist only; no login features in this phase |

### Fallback download size (important)

For a 2-hour Vimeo with a 20s selection, fallback downloads **the whole file**. Acceptable for v1.1 with UI note in README:

> If section download isn’t supported for a site, the app may download more of the video before trimming.

Future optimization: yt-dlp format/range tricks per site (out of scope here).

## Success criteria

1. Public YouTube behavior unchanged for happy path.  
2. Public Vimeo: metadata + preview + export video/audio work without code hacks per title.  
3. Unsupported hosts rejected before network.  
4. Section failure does not hard-fail if full download + trim succeeds.  
5. Automated tests cover allowlist + arg builders (no live network in CI).

## Open questions (defaults if unasked)

| Q | Default |
|---|---------|
| Include `player.vimeo.com` embed URLs? | Yes if host allowlisted |
| Auto-detect more hosts from yt-dlp? | No — explicit allowlist only |
| Cookies file path in settings? | Defer to a later design |

## Suggested PR order

1. Allowlist + wire commands (behavior: Vimeo accepted; export still section-only)  
2. Export fallback  
3. README / UX copy + manual smoke notes  
