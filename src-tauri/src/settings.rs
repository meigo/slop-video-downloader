//! App settings persistence (JSON in app config dir).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub last_save_dir: Option<String>,
    pub max_height: Option<u32>,
    pub include_audio: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            last_save_dir: None,
            max_height: Some(1080),
            include_audio: true,
        }
    }
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}

#[tauri::command]
pub fn load_settings(app: AppHandle) -> AppSettings {
    let path = match settings_path(&app) {
        Ok(p) => p,
        Err(_) => return AppSettings::default(),
    };
    match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => AppSettings::default(),
    }
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let path = settings_path(&app)?;
    let raw = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, raw).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn default_save_dir() -> String {
    // ~/Movies/Slop Refs on macOS, %USERPROFILE%\Videos\Slop Refs on Windows.
    let base = dirs::video_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default());
    base.join("Slop Refs").to_string_lossy().into()
}

/// Reveal `path` in the system file manager (Finder on macOS).
#[tauri::command]
pub fn reveal_in_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &path])
            .status()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", &path])
            .status()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        use std::path::Path;
        let parent = Path::new(&path)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| Path::new(".").to_path_buf());
        std::process::Command::new("xdg-open")
            .arg(&parent)
            .status()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_audio_on_1080() {
        let s = AppSettings::default();
        assert_eq!(s.max_height, Some(1080));
        assert!(s.include_audio);
        assert_eq!(s.last_save_dir, None);
    }

    #[test]
    fn default_save_dir_is_absolute_and_named() {
        let d = default_save_dir();
        assert!(!d.is_empty(), "save dir must not be empty");
        assert!(
            std::path::Path::new(&d).is_absolute(),
            "save dir must be absolute, got {d}"
        );
        assert!(d.ends_with("Slop Refs"), "save dir must end in Slop Refs, got {d}");
        // The macOS-only "Movies" literal must not be hardcoded any more.
        #[cfg(windows)]
        assert!(!d.contains("Movies"), "Windows must not use a Movies folder");
    }
}
