// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod deps;
mod export;
pub mod filename;
mod settings;
pub mod source;
pub mod youtube;
mod ytdlp;

use deps::check_deps;
use export::export_clip;
use settings::{default_save_dir, load_settings, reveal_in_folder, save_settings};
use ytdlp::{fetch_metadata, resolve_preview};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    deps::ensure_tool_path();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            check_deps,
            fetch_metadata,
            resolve_preview,
            export_clip,
            load_settings,
            save_settings,
            default_save_dir,
            reveal_in_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
