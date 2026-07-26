//! Export pipeline: yt-dlp section download → ffmpeg video or audio-only.

use crate::filename::build_clip_filename;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Emitter};

const STDERR_TRUNCATE: usize = 500;
/// Prefer progressive A/V when possible so the section lands as a real local file.
/// (Audio-only format + sections often leaves stream URLs that ffmpeg 403s on.)
const SECTION_FORMAT: &str = "bv*+ba/b";

/// What to write to disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportKind {
    Video,
    Audio,
}

impl Default for ExportKind {
    fn default() -> Self {
        Self::Video
    }
}

#[derive(Debug, Deserialize)]
pub struct ExportOpts {
    pub url: String,
    pub title: String,
    pub start_secs: f64,
    pub end_secs: f64,
    /// `None` = source height (no scale). Ignored for audio-only.
    pub max_height: Option<u32>,
    /// Video mux: include AAC audio track. Ignored for audio-only (always extracts audio).
    pub include_audio: bool,
    pub out_dir: String,
    /// `"video"` (default) or `"audio"`.
    #[serde(default)]
    pub export_kind: ExportKind,
}

#[derive(Debug, Serialize)]
pub struct ExportResult {
    pub output_path: String,
}

#[derive(Clone, Serialize)]
struct ExportProgress {
    phase: String,
    message: String,
    pct: Option<f64>,
}

/// yt-dlp args for section download to a **local** file (`raw.%(ext)s`).
///
/// Always fetches a merged A/V section (not bare `ba` URLs). Audio-only export
/// reuses this file and strips video in ffmpeg — avoids HTTP 403 when ffmpeg
/// opens googlevideo stream URLs without yt-dlp’s cookies/headers.
pub fn section_download_args(
    url: &str,
    start_secs: f64,
    end_secs: f64,
    out_template: &str,
) -> Vec<String> {
    let section = format!("*{start_secs}-{end_secs}");
    vec![
        "--no-playlist".into(),
        // Force a real download into -o (never print URLs / invoke external players).
        "--newline".into(),
        "--download-sections".into(),
        section,
        "-f".into(),
        SECTION_FORMAT.into(),
        "--merge-output-format".into(),
        "mp4".into(),
        // Prefer remux of section into a single local file.
        "--force-keyframes-at-cuts".into(),
        "-o".into(),
        out_template.into(),
        "--".into(),
        url.into(),
    ]
}

/// Full media download (no section) when `--download-sections` fails for a site.
pub fn full_download_args(url: &str, out_template: &str) -> Vec<String> {
    vec![
        "--no-playlist".into(),
        "--newline".into(),
        "-f".into(),
        SECTION_FORMAT.into(),
        "--merge-output-format".into(),
        "mp4".into(),
        "-o".into(),
        out_template.into(),
        "--".into(),
        url.into(),
    ]
}

/// ffmpeg args: H.264 re-encode, optional scale/audio strip, faststart.
/// When `trim` is Some((start, end)), applies output seeking after `-i` (accurate).
pub fn ffmpeg_transcode_args(
    input: &str,
    output: &str,
    max_height: Option<u32>,
    include_audio: bool,
    trim: Option<(f64, f64)>,
) -> Vec<String> {
    let mut args = vec!["-y".into(), "-i".into(), input.into()];

    if let Some((start, end)) = trim {
        args.push("-ss".into());
        args.push(format!("{start}"));
        args.push("-to".into());
        args.push(format!("{end}"));
    }

    args.push("-c:v".into());
    args.push("libx264".into());
    args.push("-preset".into());
    args.push("veryfast".into());
    args.push("-crf".into());
    args.push("20".into());

    if let Some(h) = max_height {
        args.push("-vf".into());
        args.push(format!("scale=-2:'min({h},ih)'"));
    }

    if include_audio {
        args.push("-c:a".into());
        args.push("aac".into());
    } else {
        args.push("-an".into());
    }

    args.push("-movflags".into());
    args.push("+faststart".into());
    args.push(output.into());
    args
}

/// ffmpeg args: audio-only AAC in `.m4a` from a **local** media file.
/// When `trim` is Some((start, end)), applies output seeking after `-i`.
pub fn ffmpeg_audio_extract_args(
    input: &str,
    output: &str,
    trim: Option<(f64, f64)>,
) -> Vec<String> {
    let mut args = vec!["-y".into(), "-i".into(), input.into()];
    if let Some((start, end)) = trim {
        args.push("-ss".into());
        args.push(format!("{start}"));
        args.push("-to".into());
        args.push(format!("{end}"));
    }
    args.push("-vn".into());
    args.push("-map".into());
    args.push("0:a:0?".into());
    args.push("-c:a".into());
    args.push("aac".into());
    args.push("-b:a".into());
    args.push("192k".into());
    args.push(output.into());
    args
}

fn assert_local_media(path: &Path) -> Result<(), String> {
    let s = path.to_string_lossy();
    if s.starts_with("http://") || s.starts_with("https://") {
        return Err(
            "Internal error: export expected a local file but got a URL (refusing ffmpeg open)"
                .into(),
        );
    }
    if !path.is_file() {
        return Err(format!("Downloaded media missing: {}", path.display()));
    }
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if meta.len() < 64 {
        return Err(format!(
            "Downloaded media looks empty or invalid ({} bytes): {}",
            meta.len(),
            path.display()
        ));
    }
    Ok(())
}

fn truncate_err(stderr: &str) -> String {
    // Prefer yt-dlp-aware mapping when the message looks like extractor output.
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("yt-dlp")
        || lower.contains("[vimeo]")
        || lower.contains("[youtube]")
        || lower.contains("oauth")
        || lower.contains("impersonat")
    {
        return crate::ytdlp::format_ytdlp_error(stderr);
    }
    let trimmed = stderr.trim();
    if trimmed.is_empty() {
        return "command failed with no error output".to_string();
    }
    if trimmed.chars().count() <= STDERR_TRUNCATE {
        trimmed.to_string()
    } else {
        let truncated: String = trimmed.chars().take(STDERR_TRUNCATE).collect();
        format!("{truncated}…")
    }
}

fn emit_progress(app: &AppHandle, phase: &str, message: &str, pct: Option<f64>) {
    let _ = app.emit(
        "export-progress",
        ExportProgress {
            phase: phase.into(),
            message: message.into(),
            pct,
        },
    );
}

/// If `path` exists, append ` (1)`, ` (2)`, … before the extension.
fn unique_output_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }
    let parent = path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("clip");
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("mp4");

    let mut n = 1u32;
    loop {
        let candidate = parent.join(format!("{stem} ({n}).{ext}"));
        if !candidate.exists() {
            return candidate;
        }
        n = n.saturating_add(1);
        if n == 0 {
            return path;
        }
    }
}

fn find_media_file(work_dir: &Path, stem_prefix: &str) -> Result<PathBuf, String> {
    let preferred_mp4 = work_dir.join(format!("{stem_prefix}.mp4"));
    if preferred_mp4.is_file() {
        assert_local_media(&preferred_mp4)?;
        return Ok(preferred_mp4);
    }

    let entries = std::fs::read_dir(work_dir)
        .map_err(|e| format!("Failed to read export temp dir: {e}"))?;
    let mut candidates: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // Skip yt-dlp bookkeeping / incomplete parts.
        if name.ends_with(".part")
            || name.ends_with(".ytdl")
            || name.ends_with(".temp")
            || name.ends_with(".info.json")
        {
            continue;
        }
        if name.starts_with(&format!("{stem_prefix}.")) && path.is_file() {
            candidates.push(path);
        }
    }
    // Prefer largest file (real media over tiny stubs).
    candidates.sort_by_key(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0));
    if let Some(path) = candidates.pop() {
        assert_local_media(&path)?;
        return Ok(path);
    }
    Err(format!(
        "Download succeeded but media file not found ({stem_prefix}.*)"
    ))
}

fn run_ytdlp(args: Vec<String>) -> Result<std::process::Output, String> {
    Command::new("yt-dlp")
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run yt-dlp: {e}"))
}

#[tauri::command]
pub async fn export_clip(app: AppHandle, opts: ExportOpts) -> Result<ExportResult, String> {
    let url = crate::source::normalize_source_url(&opts.url)
        .ok_or_else(|| crate::source::UNSUPPORTED_SITE_MESSAGE.to_string())?;

    if opts.end_secs <= opts.start_secs {
        return Err("end_secs must be greater than start_secs".to_string());
    }

    let out_dir = PathBuf::from(&opts.out_dir);
    std::fs::create_dir_all(&out_dir)
        .map_err(|e| format!("Failed to create out_dir: {e}"))?;

    let audio_only = opts.export_kind == ExportKind::Audio;
    let ext = if audio_only { "m4a" } else { "mp4" };
    let filename = build_clip_filename(&opts.title, opts.start_secs, opts.end_secs, ext);
    let final_path = unique_output_path(out_dir.join(&filename));

    let work_dir = std::env::temp_dir()
        .join("slop-video-downloader")
        .join(format!("export-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&work_dir)
        .map_err(|e| format!("Failed to create temp dir: {e}"))?;

    let start = opts.start_secs;
    let end = opts.end_secs;

    // --- 1) Try section download ---
    emit_progress(
        &app,
        "download",
        if audio_only {
            "Downloading section for audio extract…"
        } else {
            "Downloading clip section…"
        },
        Some(0.0),
    );

    let section_template = work_dir
        .join("raw.%(ext)s")
        .to_string_lossy()
        .into_owned();
    let url_section = url.clone();
    let section_args = section_download_args(&url_section, start, end, &section_template);
    let section_output = tauri::async_runtime::spawn_blocking(move || run_ytdlp(section_args))
        .await
        .map_err(|e| e.to_string())??;

    let mut needs_trim = true;
    let mut raw_path: Option<PathBuf> = None;
    let mut section_err: Option<String> = None;

    if section_output.status.success() {
        match find_media_file(&work_dir, "raw") {
            Ok(p) => {
                raw_path = Some(p);
                needs_trim = false;
            }
            Err(e) => {
                section_err = Some(e);
            }
        }
    } else {
        section_err = Some(truncate_err(&String::from_utf8_lossy(
            &section_output.stderr,
        )));
    }

    // --- 2) Full download + ffmpeg trim when section path fails ---
    if needs_trim {
        emit_progress(
            &app,
            "download",
            "Section download unavailable — downloading full media (then trim)…",
            Some(15.0),
        );
        let full_template = work_dir
            .join("raw_full.%(ext)s")
            .to_string_lossy()
            .into_owned();
        let url_full = url.clone();
        let full_args = full_download_args(&url_full, &full_template);
        let full_output = tauri::async_runtime::spawn_blocking(move || run_ytdlp(full_args))
            .await
            .map_err(|e| e.to_string())??;

        if !full_output.status.success() {
            let _ = std::fs::remove_dir_all(&work_dir);
            let mut msg = truncate_err(&String::from_utf8_lossy(&full_output.stderr));
            if let Some(se) = section_err {
                if !se.is_empty() && se != "command failed with no error output" {
                    msg = format!("{se} | full download: {msg}");
                }
            }
            return Err(msg);
        }

        match find_media_file(&work_dir, "raw_full") {
            Ok(p) => {
                raw_path = Some(p);
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&work_dir);
                return Err(e);
            }
        }
    }

    let raw_path = raw_path.ok_or_else(|| {
        let _ = std::fs::remove_dir_all(&work_dir);
        "Download produced no media file".to_string()
    })?;

    // --- encode (never pass http(s) URLs into ffmpeg) ---
    emit_progress(
        &app,
        "transcode",
        if needs_trim {
            if audio_only {
                "Trimming and extracting AAC audio…"
            } else {
                "Trimming and transcoding to H.264/AAC…"
            }
        } else if audio_only {
            "Extracting AAC audio…"
        } else {
            "Transcoding to H.264/AAC…"
        },
        Some(50.0),
    );

    let input = raw_path.to_string_lossy().into_owned();
    if input.starts_with("http://") || input.starts_with("https://") {
        let _ = std::fs::remove_dir_all(&work_dir);
        return Err(
            "Download produced a stream URL instead of a local file; try updating yt-dlp (`brew upgrade yt-dlp`) and retry."
                .into(),
        );
    }
    let output = final_path.to_string_lossy().into_owned();
    let max_height = opts.max_height;
    let include_audio = opts.include_audio;
    let trim = if needs_trim {
        Some((start, end))
    } else {
        None
    };

    let ff_output = tauri::async_runtime::spawn_blocking(move || {
        let args = if audio_only {
            ffmpeg_audio_extract_args(&input, &output, trim)
        } else {
            ffmpeg_transcode_args(&input, &output, max_height, include_audio, trim)
        };
        Command::new("ffmpeg").args(args).output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("Failed to run ffmpeg: {e}"))?;

    if !ff_output.status.success() {
        let _ = std::fs::remove_file(&final_path);
        let _ = std::fs::remove_dir_all(&work_dir);
        let mut msg = truncate_err(&String::from_utf8_lossy(&ff_output.stderr));
        if msg.contains("403") || msg.contains("Forbidden") {
            msg.push_str(
                " — tip: update yt-dlp (`brew upgrade yt-dlp`) if the site blocks the download.",
            );
        }
        return Err(msg);
    }

    if !final_path.is_file() {
        let _ = std::fs::remove_dir_all(&work_dir);
        return Err("ffmpeg completed but output file not found".to_string());
    }

    let _ = std::fs::remove_dir_all(&work_dir);

    let output_path = final_path.to_string_lossy().into_owned();
    emit_progress(&app, "done", "Export complete", Some(100.0));

    Ok(ExportResult { output_path })
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(args.contains(&"--no-playlist".to_string()));
        assert!(args.contains(&SECTION_FORMAT.to_string()));
        assert!(args.contains(&"--merge-output-format".to_string()));
        assert!(args.contains(&"mp4".to_string()));
        assert!(args.contains(&"--force-keyframes-at-cuts".to_string()));
    }

    #[test]
    fn full_download_args_no_sections() {
        let args = full_download_args(
            "https://vimeo.com/123456789",
            "/tmp/raw_full.%(ext)s",
        );
        assert!(!args.iter().any(|a| a.contains("download-sections")));
        assert!(args.contains(&SECTION_FORMAT.to_string()));
        assert!(args.contains(&"--merge-output-format".to_string()));
        assert_eq!(
            args.last().map(String::as_str),
            Some("https://vimeo.com/123456789")
        );
    }

    #[test]
    fn ffmpeg_scales_and_strips_audio() {
        let args = ffmpeg_transcode_args("/tmp/in.mp4", "/tmp/out.mp4", Some(720), false, None);
        assert!(args.iter().any(|a| a.contains("scale=")));
        assert!(args.iter().any(|a| a == "-an"));
        assert!(args.contains(&"libx264".to_string()));
        assert!(args.contains(&"+faststart".to_string()));
        assert!(!args.iter().any(|a| a == "aac"));
    }

    #[test]
    fn ffmpeg_with_audio_no_scale() {
        let args = ffmpeg_transcode_args("/tmp/in.mp4", "/tmp/out.mp4", None, true, None);
        assert!(!args.iter().any(|a| a.contains("scale=")));
        assert!(args.contains(&"-c:a".to_string()));
        assert!(args.contains(&"aac".to_string()));
        assert!(!args.iter().any(|a| a == "-an"));
    }

    #[test]
    fn ffmpeg_trim_includes_ss_to() {
        let args =
            ffmpeg_transcode_args("/tmp/in.mp4", "/tmp/out.mp4", None, true, Some((10.0, 25.0)));
        assert!(args.contains(&"-ss".to_string()));
        assert!(args.contains(&"10".to_string()) || args.iter().any(|a| a.starts_with("10")));
        assert!(args.contains(&"-to".to_string()));
    }

    #[test]
    fn ffmpeg_audio_only_extract() {
        let args = ffmpeg_audio_extract_args("/tmp/in.mp4", "/tmp/out.m4a", None);
        assert!(args.contains(&"-vn".to_string()));
        assert!(args.contains(&"aac".to_string()));
        assert!(args.contains(&"192k".to_string()));
        assert!(args.contains(&"-map".to_string()));
        assert!(!args.iter().any(|a| a == "libx264"));
        assert_eq!(args.last().map(String::as_str), Some("/tmp/out.m4a"));
    }
}

