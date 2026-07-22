use std::fs;
use std::path::PathBuf;
use tauri::Manager;

fn settings_path(app: &tauri::AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_data_dir()
        .expect("failed to resolve app data dir");
    fs::create_dir_all(&dir).ok();
    dir.join("settings.json")
}

#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))
}

#[tauri::command]
fn write_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, &content).map_err(|e| format!("Failed to write file: {}", e))
}

#[tauri::command]
fn read_settings(app: tauri::AppHandle) -> Result<String, String> {
    let path = settings_path(&app);
    if path.exists() {
        fs::read_to_string(&path).map_err(|e| format!("Failed to read settings: {}", e))
    } else {
        Ok("{}".to_string())
    }
}

#[tauri::command]
fn write_settings(app: tauri::AppHandle, json: String) -> Result<(), String> {
    let path = settings_path(&app);
    fs::write(&path, &json).map_err(|e| format!("Failed to write settings: {}", e))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            read_file,
            write_file,
            read_settings,
            write_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}