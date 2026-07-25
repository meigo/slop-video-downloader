//! yt-dlp integration: metadata fetch (preview/export helpers land in later tasks).

use serde::Serialize;
use serde_json::Value;
use std::process::Command;

const STDERR_TRUNCATE: usize = 500;

#[derive(Debug, Clone, Serialize)]
pub struct VideoMeta {
    pub id: String,
    pub title: String,
    pub duration_secs: f64,
    pub thumbnail_url: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_args_shape() {
        let a = metadata_args("https://www.youtube.com/watch?v=dQw4w9WgXcQ");
        assert_eq!(a[0], "-J");
        assert!(a.contains(&"--no-playlist".to_string()));
        assert!(a.contains(&"--".to_string()));
        assert_eq!(a.last().map(String::as_str), Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
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
