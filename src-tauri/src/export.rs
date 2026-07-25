//! Export pipeline: yt-dlp section download → ffmpeg H.264/AAC → final MP4.

use crate::filename::build_clip_filename;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Emitter};

const STDERR_TRUNCATE: usize = 500;
const SECTION_FORMAT: &str = "bv*+ba/b";

#[derive(Debug, Deserialize)]
pub struct ExportOpts {
    pub url: String,
    pub title: String,
    pub start_secs: f64,
    pub end_secs: f64,
    /// `None` = source height (no scale).
    pub max_height: Option<u32>,
    pub include_audio: bool,
    pub out_dir: String,
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

/// yt-dlp args for section download to an output template (`raw.%(ext)s`).
pub fn section_download_args(
    url: &str,
    start_secs: f64,
    end_secs: f64,
    out_template: &str,
) -> Vec<String> {
    let section = format!("*{start_secs}-{end_secs}");
    vec![
        "--no-playlist".into(),
        "--download-sections".into(),
        section,
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
pub fn ffmpeg_transcode_args(
    input: &str,
    output: &str,
    max_height: Option<u32>,
    include_audio: bool,
) -> Vec<String> {
    let mut args = vec![
        "-y".into(),
        "-i".into(),
        input.into(),
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "veryfast".into(),
        "-crf".into(),
        "20".into(),
    ];

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

fn truncate_err(stderr: &str) -> String {
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
            // overflow — fall back to original (will fail later if still exists)
            return path;
        }
    }
}

fn find_raw_file(work_dir: &Path) -> Result<PathBuf, String> {
    let preferred = work_dir.join("raw.mp4");
    if preferred.is_file() {
        return Ok(preferred);
    }

    let entries = std::fs::read_dir(work_dir)
        .map_err(|e| format!("Failed to read export temp dir: {e}"))?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("raw.") && entry.path().is_file() {
            return Ok(entry.path());
        }
    }
    Err("Section download succeeded but raw file not found".to_string())
}

#[tauri::command]
pub async fn export_clip(app: AppHandle, opts: ExportOpts) -> Result<ExportResult, String> {
    let url = crate::youtube::normalize_youtube_url(&opts.url)
        .ok_or_else(|| "YouTube only in v1".to_string())?;

    if opts.end_secs <= opts.start_secs {
        return Err("end_secs must be greater than start_secs".to_string());
    }

    let out_dir = PathBuf::from(&opts.out_dir);
    std::fs::create_dir_all(&out_dir)
        .map_err(|e| format!("Failed to create out_dir: {e}"))?;

    let filename = build_clip_filename(&opts.title, opts.start_secs, opts.end_secs);
    let final_path = unique_output_path(out_dir.join(&filename));

    let work_dir = std::env::temp_dir()
        .join("slop-video-downloader")
        .join(format!("export-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&work_dir)
        .map_err(|e| format!("Failed to create temp dir: {e}"))?;

    let out_template = work_dir
        .join("raw.%(ext)s")
        .to_string_lossy()
        .into_owned();

    // --- download ---
    emit_progress(&app, "download", "Downloading clip section…", Some(0.0));

    let url_dl = url.clone();
    let start = opts.start_secs;
    let end = opts.end_secs;
    let template = out_template.clone();
    let dl_output = tauri::async_runtime::spawn_blocking(move || {
        Command::new("yt-dlp")
            .args(section_download_args(&url_dl, start, end, &template))
            .output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("Failed to run yt-dlp: {e}"))?;

    if !dl_output.status.success() {
        return Err(truncate_err(&String::from_utf8_lossy(&dl_output.stderr)));
    }

    let raw_path = find_raw_file(&work_dir)?;

    // --- transcode ---
    emit_progress(&app, "transcode", "Transcoding to H.264/AAC…", Some(50.0));

    let input = raw_path.to_string_lossy().into_owned();
    let output = final_path.to_string_lossy().into_owned();
    let max_height = opts.max_height;
    let include_audio = opts.include_audio;
    let ff_output = tauri::async_runtime::spawn_blocking(move || {
        Command::new("ffmpeg")
            .args(ffmpeg_transcode_args(
                &input,
                &output,
                max_height,
                include_audio,
            ))
            .output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("Failed to run ffmpeg: {e}"))?;

    if !ff_output.status.success() {
        return Err(truncate_err(&String::from_utf8_lossy(&ff_output.stderr)));
    }

    if !final_path.is_file() {
        return Err("ffmpeg completed but output file not found".to_string());
    }

    // Success: clean temp
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
        assert!(args.contains(&"-f".to_string()));
        assert!(args.contains(&SECTION_FORMAT.to_string()));
        assert!(args.contains(&"--merge-output-format".to_string()));
        assert!(args.contains(&"mp4".to_string()));
        assert!(args.contains(&"-o".to_string()));
        assert!(args.contains(&"/tmp/raw.%(ext)s".to_string()));
        assert_eq!(
            args.last().map(String::as_str),
            Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
        );
    }

    #[test]
    fn ffmpeg_scales_and_strips_audio() {
        let args = ffmpeg_transcode_args("/tmp/in.mp4", "/tmp/out.mp4", Some(720), false);
        assert!(args.iter().any(|a| a.contains("scale=")));
        assert!(args.iter().any(|a| a == "-an"));
        assert!(args.contains(&"libx264".to_string()));
        assert!(args.contains(&"veryfast".to_string()));
        assert!(args.contains(&"20".to_string()));
        assert!(args.contains(&"+faststart".to_string()));
        assert!(!args.iter().any(|a| a == "aac"));
    }

    #[test]
    fn ffmpeg_with_audio_no_scale() {
        let args = ffmpeg_transcode_args("/tmp/in.mp4", "/tmp/out.mp4", None, true);
        assert!(!args.iter().any(|a| a.contains("scale=")));
        assert!(args.contains(&"-c:a".to_string()));
        assert!(args.contains(&"aac".to_string()));
        assert!(!args.iter().any(|a| a == "-an"));
    }
}
