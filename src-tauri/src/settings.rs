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
    // ~/Movies/Slop Refs
    let home = dirs::home_dir().unwrap_or_default();
    home.join("Movies")
        .join("Slop Refs")
        .to_string_lossy()
        .into()
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
}
