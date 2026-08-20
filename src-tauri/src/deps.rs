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

/// Dirs a GUI launch cannot see: launchd hands apps a bare
/// `/usr/bin:/bin:/usr/sbin:/sbin`, so Homebrew, MacPorts and pipx
/// installs are invisible unless we put them back.
const TOOL_DIRS: [&str; 3] = ["/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin"];

fn augmented_path(current: &str, home: Option<&str>) -> String {
    let mut dirs: Vec<String> = TOOL_DIRS.iter().map(|d| d.to_string()).collect();
    if let Some(home) = home {
        dirs.push(format!("{home}/.local/bin"));
    }
    dirs.extend(current.split(':').filter(|d| !d.is_empty()).map(str::to_string));

    let mut seen = std::collections::HashSet::new();
    dirs.retain(|d| seen.insert(d.clone()));
    dirs.join(":")
}

/// Must run before any tool is spawned: `Command` inherits this process's PATH.
pub fn ensure_tool_path() {
    let current = std::env::var("PATH").unwrap_or_default();
    let home = std::env::var("HOME").ok();
    std::env::set_var("PATH", augmented_path(&current, home.as_deref()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepends_tool_dirs_to_launchd_default() {
        let path = augmented_path("/usr/bin:/bin:/usr/sbin:/sbin", None);
        assert!(path.starts_with("/opt/homebrew/bin:/usr/local/bin:/opt/local/bin:"));
        assert!(path.ends_with("/usr/bin:/bin:/usr/sbin:/sbin"));
    }

    #[test]
    fn includes_user_local_bin_when_home_known() {
        let path = augmented_path("/usr/bin", Some("/Users/someone"));
        assert!(path.contains("/Users/someone/.local/bin"));
    }

    #[test]
    fn keeps_existing_entries_once() {
        let path = augmented_path("/opt/homebrew/bin:/usr/bin", None);
        assert_eq!(path.matches("/opt/homebrew/bin").count(), 1);
        assert!(path.contains("/usr/bin"));
    }
}
