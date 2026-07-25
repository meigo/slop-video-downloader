//! External tool detection (`yt-dlp`, `ffmpeg`) via PATH `which`.

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
    if path.is_empty() {
        None
    } else {
        Some(path)
    }
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
