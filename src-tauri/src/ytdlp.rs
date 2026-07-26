//! yt-dlp integration: metadata fetch and hybrid preview resolve.

use crate::source::SourceFamily;
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

/// Extra yt-dlp flags some hosts need.
/// Vimeo anonymous OAuth is broken; X often needs a logged-in session.
/// Chrome cookies work for both in practice.
pub fn ytdlp_site_args(url: &str) -> Vec<String> {
    match crate::source::source_family(url) {
        Some(SourceFamily::Vimeo) | Some(SourceFamily::X) => vec![
            "--cookies-from-browser".into(),
            "chrome".into(),
        ],
        _ => Vec::new(),
    }
}

/// Insert site-specific args immediately before the trailing `--` URL separator.
pub fn with_site_args(url: &str, mut args: Vec<String>) -> Vec<String> {
    let site = ytdlp_site_args(url);
    if site.is_empty() {
        return args;
    }
    if let Some(pos) = args.iter().position(|a| a == "--") {
        for (i, a) in site.into_iter().enumerate() {
            args.insert(pos + i, a);
        }
    } else {
        args.extend(site);
    }
    args
}

/// CLI args for `yt-dlp` metadata dump (`-J` = dump single JSON).
pub fn metadata_args(url: &str) -> Vec<String> {
    with_site_args(
        url,
        vec![
            "-J".into(),
            "--no-playlist".into(),
            "--".into(),
            url.into(),
        ],
    )
}

/// CLI args for progressive stream URL discovery (`-g` = get URL only).
pub fn stream_url_args(url: &str) -> Vec<String> {
    with_site_args(
        url,
        vec![
            "-g".into(),
            "-f".into(),
            STREAM_FORMAT.into(),
            "--no-playlist".into(),
            "--".into(),
            url.into(),
        ],
    )
}

/// CLI args for ≤720p preview download to an output template.
pub fn preview_download_args(url: &str, out_template: &str) -> Vec<String> {
    with_site_args(
        url,
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
        ],
    )
}

fn truncate_err(stderr: &str) -> String {
    format_ytdlp_error(stderr)
}

/// Map common yt-dlp failures to short user-facing guidance.
pub fn format_ytdlp_error(stderr: &str) -> String {
    let trimmed = stderr.trim();
    if trimmed.is_empty() {
        return "yt-dlp failed with no error output".to_string();
    }

    let lower = trimmed.to_ascii_lowercase();

    // Vimeo: anonymous OAuth is broken; Chrome cookies usually work.
    // https://github.com/yt-dlp/yt-dlp/issues/17271
    if lower.contains("vimeo")
        && (lower.contains("oauth")
            || lower.contains("401")
            || lower.contains("unauthorized")
            || lower.contains("impersonat")
            || lower.contains("could not copy"))
    {
        return "Vimeo needs a logged-in browser session. \
Open Chrome, log into vimeo.com, then try again.\n\n\
This app reads cookies from Chrome (`yt-dlp --cookies-from-browser chrome`). \
macOS may ask to unlock the Keychain — choose Allow.".to_string();
    }

    // X / Twitter: guest token / auth failures
    if (lower.contains("[twitter]") || lower.contains("x.com") || lower.contains("twitter"))
        && (lower.contains("401")
            || lower.contains("403")
            || lower.contains("unauthorized")
            || lower.contains("cookie")
            || lower.contains("login")
            || lower.contains("authenticated")
            || lower.contains("no video could be found"))
    {
        return "X (Twitter) needs a logged-in Chrome session and a post that actually contains video.\n\n\
1. Open Chrome and log into x.com\n\
2. Open the post and confirm the video plays\n\
3. Retry Fetch (allow Keychain access if prompted)\n\n\
Text-only or image-only posts will fail (no video stream).".to_string();
    }

    if lower.contains("could not find") && lower.contains("cookie")
        || lower.contains("failed to load cookies")
        || (lower.contains("cookies-from-browser") && lower.contains("error"))
    {
        return "Could not read browser cookies. Install Chrome, log into the site there, \
then allow Keychain access if macOS prompts.".to_string();
    }

    if lower.contains("impersonat")
        && (lower.contains("unavailable") || lower.contains("no impersonate"))
    {
        return "yt-dlp needs browser impersonation for this site. \
Install curl_cffi support, e.g. pipx install \"yt-dlp[default,curl-cffi]\" \
or use the official yt-dlp_macos binary.".to_string();
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

/// Find `{job_id}_preview.*` written by yt-dlp (job_id is a uuid, not site id).
fn find_preview_file(temp_dir: &Path, job_id: &str) -> Result<String, String> {
    let preferred = temp_dir.join(format!("{job_id}_preview.mp4"));
    if preferred.is_file() {
        return Ok(preferred.to_string_lossy().into_owned());
    }

    let prefix = format!("{job_id}_preview.");
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
        "Preview download succeeded but file not found for job {job_id}"
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
    let url = crate::source::normalize_source_url(&url)
        .ok_or_else(|| crate::source::UNSUPPORTED_SITE_MESSAGE.to_string())?;

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
pub async fn resolve_preview(
    url: String,
    force_file: Option<bool>,
) -> Result<PreviewResult, String> {
    let url = crate::source::normalize_source_url(&url)
        .ok_or_else(|| crate::source::UNSUPPORTED_SITE_MESSAGE.to_string())?;

    let force_file = force_file.unwrap_or(false);

    // 1) Try progressive single-URL stream via yt-dlp -g (unless force_file)
    if !force_file {
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
    }

    // 2) Fallback (or forced): download ≤720p preview into temp dir
    // Unique job id so we find the file without assuming YouTube-style ids.
    let job_id = uuid::Uuid::new_v4().to_string();
    let temp_dir = preview_temp_dir();
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp dir: {e}"))?;

    let out_template = temp_dir
        .join(format!("{job_id}_preview.%(ext)s"))
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

    let path = find_preview_file(&temp_dir, &job_id)?;

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

    #[test]
    fn format_vimeo_oauth_error() {
        let msg = format_ytdlp_error(
            "ERROR: [vimeo] 184782959: Failed to fetch macos OAuth token: HTTP Error 401: Unauthorized",
        );
        assert!(msg.contains("Vimeo"));
        assert!(msg.contains("Chrome") || msg.contains("cookies"));
    }

    #[test]
    fn vimeo_metadata_args_include_chrome_cookies() {
        let a = metadata_args("https://vimeo.com/184782959");
        assert!(a.contains(&"--cookies-from-browser".to_string()));
        assert!(a.contains(&"chrome".to_string()));
        assert_eq!(a.last().map(String::as_str), Some("https://vimeo.com/184782959"));
    }

    #[test]
    fn youtube_metadata_args_skip_cookies() {
        let a = metadata_args("https://www.youtube.com/watch?v=dQw4w9WgXcQ");
        assert!(!a.contains(&"--cookies-from-browser".to_string()));
    }
}
