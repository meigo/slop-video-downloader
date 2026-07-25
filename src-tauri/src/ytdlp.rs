//! yt-dlp integration: metadata fetch and hybrid preview resolve.

use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

const STDERR_TRUNCATE: usize = 500;
const PREVIEW_NOTE_FILE: &str = "Stream preview unavailable — using local preview file.";
const STREAM_FORMAT: &str = "b[ext=mp4]/best[ext=mp4]/best";
const PREVIEW_DL_FORMAT: &str = "bv*[height<=720]+ba/b[height<=720]/best[height<=720]/best";

#[derive(Debug, Clone, Serialize)]
pub struct VideoMeta {
    pub id: String,
    pub title: String,
    pub duration_secs: f64,
    pub thumbnail_url: Option<String>,
}

/// Hybrid preview result: direct progressive URL or a local temp file.
///
/// For `mode == "file"`, `url_or_path` is a filesystem path. The frontend must
/// pass it through Tauri 2 `convertFileSrc` before using it as a video `src`.
#[derive(Debug, Clone, Serialize)]
pub struct PreviewResult {
    /// `"stream"` (progressive HTTP URL) or `"file"` (local path).
    pub mode: String,
    pub url_or_path: String,
    pub note: Option<String>,
}

/// CLI args for `yt-dlp` metadata dump (`-J` = dump single JSON).
pub fn metadata_args(url: &str) -> Vec<String> {
    vec![
        "-J".into(),
        "--no-playlist".into(),
        "--".into(),
        url.into(),
    ]
}

/// CLI args for progressive stream URL discovery (`-g` = get URL only).
pub fn stream_url_args(url: &str) -> Vec<String> {
    vec![
        "-g".into(),
        "-f".into(),
        STREAM_FORMAT.into(),
        "--no-playlist".into(),
        "--".into(),
        url.into(),
    ]
}

/// CLI args for ≤720p preview download to an output template.
pub fn preview_download_args(url: &str, out_template: &str) -> Vec<String> {
    vec![
        "-f".into(),
        PREVIEW_DL_FORMAT.into(),
        "--no-playlist".into(),
        "-o".into(),
        out_template.into(),
        "--merge-output-format".into(),
        "mp4".into(),
        "--".into(),
        url.into(),
    ]
}

fn truncate_err(stderr: &str) -> String {
    let trimmed = stderr.trim();
    if trimmed.is_empty() {
        return "yt-dlp failed with no error output".to_string();
    }
    if trimmed.chars().count() <= STDERR_TRUNCATE {
        trimmed.to_string()
    } else {
        let truncated: String = trimmed.chars().take(STDERR_TRUNCATE).collect();
        format!("{truncated}…")
    }
}

/// Accept stdout from `yt-dlp -g` only when it is a single http(s) URL.
/// Multiple lines (separate A/V) are treated as stream-mode failure.
fn single_http_url(stdout: &str) -> Option<String> {
    let lines: Vec<&str> = stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.len() != 1 {
        return None;
    }
    let line = lines[0];
    if line.starts_with("http://") || line.starts_with("https://") {
        Some(line.to_string())
    } else {
        None
    }
}

fn preview_temp_dir() -> PathBuf {
    std::env::temp_dir().join("slop-video-downloader")
}

fn video_id_from_normalized(url: &str) -> Option<&str> {
    url.strip_prefix("https://www.youtube.com/watch?v=")
}

fn find_preview_file(temp_dir: &Path, video_id: &str) -> Result<String, String> {
    let preferred = temp_dir.join(format!("{video_id}_preview.mp4"));
    if preferred.is_file() {
        return Ok(preferred.to_string_lossy().into_owned());
    }

    let prefix = format!("{video_id}_preview.");
    let entries = std::fs::read_dir(temp_dir)
        .map_err(|e| format!("Failed to read temp dir: {e}"))?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(&prefix) && entry.path().is_file() {
            return Ok(entry.path().to_string_lossy().into_owned());
        }
    }
    Err(format!(
        "Preview download succeeded but file not found for id {video_id}"
    ))
}

/// Parse yt-dlp `-J` JSON stdout into `VideoMeta`.
pub fn parse_metadata_json(stdout: &[u8]) -> Result<VideoMeta, String> {
    let v: Value = serde_json::from_slice(stdout)
        .map_err(|e| format!("Failed to parse yt-dlp JSON: {e}"))?;

    let id = v
        .get("id")
        .and_then(|x| x.as_str())
        .ok_or_else(|| "yt-dlp JSON missing id".to_string())?
        .to_string();

    let title = v
        .get("title")
        .and_then(|x| x.as_str())
        .unwrap_or("Untitled")
        .to_string();

    let duration_secs = v
        .get("duration")
        .and_then(|x| x.as_f64())
        .ok_or_else(|| "yt-dlp JSON missing duration".to_string())?;

    let thumbnail_url = v
        .get("thumbnail")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());

    Ok(VideoMeta {
        id,
        title,
        duration_secs,
        thumbnail_url,
    })
}

#[tauri::command]
pub async fn fetch_metadata(url: String) -> Result<VideoMeta, String> {
    let url = crate::youtube::normalize_youtube_url(&url)
        .ok_or_else(|| "YouTube only in v1".to_string())?;

    let output = tauri::async_runtime::spawn_blocking(move || {
        Command::new("yt-dlp")
            .args(metadata_args(&url))
            .output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("Failed to run yt-dlp: {e}"))?;

    if !output.status.success() {
        return Err(truncate_err(&String::from_utf8_lossy(&output.stderr)));
    }

    parse_metadata_json(&output.stdout)
}

#[tauri::command]
pub async fn resolve_preview(url: String) -> Result<PreviewResult, String> {
    let url = crate::youtube::normalize_youtube_url(&url)
        .ok_or_else(|| "YouTube only in v1".to_string())?;

    // 1) Try progressive single-URL stream via yt-dlp -g
    let url_for_stream = url.clone();
    let stream_output = tauri::async_runtime::spawn_blocking(move || {
        Command::new("yt-dlp")
            .args(stream_url_args(&url_for_stream))
            .output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("Failed to run yt-dlp: {e}"))?;

    if stream_output.status.success() {
        if let Some(stream_url) =
            single_http_url(&String::from_utf8_lossy(&stream_output.stdout))
        {
            return Ok(PreviewResult {
                mode: "stream".into(),
                url_or_path: stream_url,
                note: None,
            });
        }
    }

    // 2) Fallback: download ≤720p preview into temp dir
    let temp_dir = preview_temp_dir();
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp dir: {e}"))?;

    let out_template = temp_dir
        .join("%(id)s_preview.%(ext)s")
        .to_string_lossy()
        .into_owned();

    let url_for_dl = url.clone();
    let template = out_template.clone();
    let dl_output = tauri::async_runtime::spawn_blocking(move || {
        Command::new("yt-dlp")
            .args(preview_download_args(&url_for_dl, &template))
            .output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("Failed to run yt-dlp: {e}"))?;

    if !dl_output.status.success() {
        return Err(truncate_err(&String::from_utf8_lossy(&dl_output.stderr)));
    }

    let video_id = video_id_from_normalized(&url)
        .ok_or_else(|| "Internal error: expected normalized YouTube URL".to_string())?;
    let path = find_preview_file(&temp_dir, video_id)?;

    Ok(PreviewResult {
        mode: "file".into(),
        url_or_path: path,
        note: Some(PREVIEW_NOTE_FILE.into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_args_shape() {
        let a = metadata_args("https://www.youtube.com/watch?v=dQw4w9WgXcQ");
        assert_eq!(a[0], "-J");
        assert!(a.contains(&"--no-playlist".to_string()));
        assert!(a.contains(&"--".to_string()));
        assert_eq!(
            a.last().map(String::as_str),
            Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
        );
    }

    #[test]
    fn stream_args_include_g() {
        assert!(stream_url_args("https://www.youtube.com/watch?v=x").contains(&"-g".into()));
    }

    #[test]
    fn stream_args_shape() {
        let url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
        let a = stream_url_args(url);
        assert_eq!(a[0], "-g");
        assert!(a.contains(&"-f".to_string()));
        assert!(a.contains(&STREAM_FORMAT.to_string()));
        assert!(a.contains(&"--no-playlist".to_string()));
        assert!(a.contains(&"--".to_string()));
        assert_eq!(a.last().map(String::as_str), Some(url));
    }

    #[test]
    fn preview_download_args_shape() {
        let url = "https://www.youtube.com/watch?v=x";
        let tmpl = "/tmp/slop/%(id)s_preview.%(ext)s";
        let a = preview_download_args(url, tmpl);
        assert!(a.contains(&"-f".to_string()));
        assert!(a.contains(&PREVIEW_DL_FORMAT.to_string()));
        assert!(a.contains(&"--no-playlist".to_string()));
        assert!(a.contains(&"-o".to_string()));
        assert!(a.contains(&tmpl.to_string()));
        assert!(a.contains(&"--merge-output-format".to_string()));
        assert!(a.contains(&"mp4".to_string()));
        assert!(a.contains(&"--".to_string()));
        assert_eq!(a.last().map(String::as_str), Some(url));
    }

    #[test]
    fn single_http_url_one_line() {
        assert_eq!(
            single_http_url("https://cdn.example/v.mp4\n").as_deref(),
            Some("https://cdn.example/v.mp4")
        );
    }

    #[test]
    fn single_http_url_rejects_multiple() {
        assert_eq!(
            single_http_url("https://cdn.example/v.mp4\nhttps://cdn.example/a.m4a\n"),
            None
        );
    }

    #[test]
    fn parse_metadata_json_basic() {
        let json = br#"{
            "id": "dQw4w9WgXcQ",
            "title": "Never Gonna Give You Up",
            "duration": 212.0,
            "thumbnail": "https://i.ytimg.com/vi/dQw4w9WgXcQ/maxresdefault.jpg"
        }"#;
        let meta = parse_metadata_json(json).unwrap();
        assert_eq!(meta.id, "dQw4w9WgXcQ");
        assert_eq!(meta.title, "Never Gonna Give You Up");
        assert!((meta.duration_secs - 212.0).abs() < f64::EPSILON);
        assert_eq!(
            meta.thumbnail_url.as_deref(),
            Some("https://i.ytimg.com/vi/dQw4w9WgXcQ/maxresdefault.jpg")
        );
    }

    #[test]
    fn truncate_err_short() {
        assert_eq!(truncate_err("  boom  "), "boom");
    }
}
