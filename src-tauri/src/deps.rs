//! External tool detection (`yt-dlp`, `ffmpeg`) via PATH `which`.

use serde::Serialize;
use std::process::Command;

#[derive(Debug, Serialize)]
pub struct DepsStatus {
    pub ytdlp: bool,
    pub ffmpeg: bool,
    pub ytdlp_path: Option<String>,
    pub ffmpeg_path: Option<String>,
    /// `yt-dlp --version`, e.g. `2026.07.04`. `None` when it could not be read.
    pub ytdlp_version: Option<String>,
    /// Old enough that YouTube has likely outgrown it — see `YTDLP_STALE_DAYS`.
    pub ytdlp_stale: bool,
}

/// yt-dlp ships roughly monthly and YouTube breaks older builds regularly
/// (see the 2026-08-17 `android_vr` block), so two releases behind is the point
/// where a nudge is worth the noise.
const YTDLP_STALE_DAYS: i64 = 60;

/// Days since 1970-01-01 for a civil date — Howard Hinnant's `days_from_civil`.
/// Avoids pulling in a date crate for one subtraction.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Today as days since the epoch.
fn today_days() -> Option<i64> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some((secs / 86_400) as i64)
}

/// Whether a yt-dlp build is old enough to warn about.
///
/// A version we cannot read never warns. This is deliberately the opposite of
/// [`crate::ytdlp::needs_client_pin`], which pins when the version is unknown:
/// guessing wrong there costs a 403, guessing wrong here only nags.
pub fn ytdlp_is_stale(version: Option<&str>, today: i64) -> bool {
    match version.and_then(crate::ytdlp::parse_ytdlp_version) {
        Some((y, m, d)) => today - days_from_civil(y as i64, m as i64, d as i64) > YTDLP_STALE_DAYS,
        None => false,
    }
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
    // Already resolved once per process for the client pin — no extra subprocess.
    let ytdlp_version = crate::ytdlp::installed_ytdlp_version();
    let ytdlp_stale = today_days()
        .map(|today| ytdlp_is_stale(ytdlp_version, today))
        .unwrap_or(false);
    DepsStatus {
        ytdlp: ytdlp_path.is_some(),
        ffmpeg: ffmpeg_path.is_some(),
        ytdlp_path,
        ffmpeg_path,
        ytdlp_version: ytdlp_version.map(str::to_string),
        ytdlp_stale,
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

    /// 2026-08-22, the day the android_vr breakage was diagnosed.
    const TODAY: i64 = 20_687;

    #[test]
    fn days_from_civil_matches_known_dates() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(1970, 1, 2), 1);
        assert_eq!(days_from_civil(1969, 12, 31), -1);
        // Leap day handling.
        assert_eq!(days_from_civil(2024, 3, 1) - days_from_civil(2024, 2, 28), 2);
        assert_eq!(days_from_civil(2026, 8, 22), TODAY);
    }

    #[test]
    fn stale_past_the_threshold_only() {
        let day = |y, m, d| days_from_civil(y, m, d);
        // Exactly 60 days old is not yet stale; 61 is.
        let sixty = day(2026, 8, 22) - 60;
        let sixty_one = day(2026, 8, 22) - 61;
        assert!(!ytdlp_is_stale(Some("2026.08.22"), TODAY));
        assert!(!ytdlp_is_stale(Some(&fmt_day(sixty)), TODAY));
        assert!(ytdlp_is_stale(Some(&fmt_day(sixty_one)), TODAY));
    }

    #[test]
    fn homebrew_stable_at_time_of_breakage_is_not_yet_stale() {
        // 2026.07.04 is 49 days before 2026-08-22 — broken, but not old.
        // The client pin covers that case; staleness is about the *next* break.
        assert!(!ytdlp_is_stale(Some("2026.07.04"), TODAY));
        // Four months on, the same build should be flagged.
        assert!(ytdlp_is_stale(Some("2026.07.04"), TODAY + 90));
    }

    #[test]
    fn unreadable_version_never_warns() {
        // Opposite of the client pin: a wrong guess here only nags.
        assert!(!ytdlp_is_stale(None, TODAY));
        assert!(!ytdlp_is_stale(Some("garbage"), TODAY));
        assert!(!ytdlp_is_stale(Some(""), TODAY));
    }

    #[test]
    fn nightly_build_strings_are_understood() {
        assert!(!ytdlp_is_stale(Some("2026.08.20.234504"), TODAY));
        assert!(ytdlp_is_stale(Some("2026.01.01.234504"), TODAY));
    }

    /// Render a day number back to a `YYYY.MM.DD` version string for tests.
    fn fmt_day(days: i64) -> String {
        for y in 2020..2032 {
            for m in 1..=12 {
                for d in 1..=31 {
                    if days_from_civil(y, m, d) == days {
                        return format!("{y}.{m:02}.{d:02}");
                    }
                }
            }
        }
        panic!("day {days} outside test range")
    }

}
