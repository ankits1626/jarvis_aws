// Module declarations
pub mod commands;
pub mod error;
pub mod files;
pub mod platform;
pub mod recording;
pub mod shortcuts;
pub mod wav;

use std::sync::Mutex;
use tauri::Manager;
use files::FileManager;
use recording::RecordingManager;
use shortcuts::ShortcutManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // Initialize FileManager and add to managed state
            let file_manager = FileManager::new()
                .map_err(|e| format!("Failed to initialize FileManager: {}", e))?;
            app.manage(file_manager);
            
            // Initialize RecordingManager with AppHandle and add to managed state (wrapped in Mutex)
            let recording_manager = RecordingManager::new(app.handle().clone());
            app.manage(Mutex::new(recording_manager));
            
            // Initialize ShortcutManager and register shortcuts
            let shortcut_manager = ShortcutManager::new(app.handle().clone());
            shortcut_manager.register_shortcuts()
                .map_err(|e| format!("Failed to register shortcuts: {}", e))?;
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_recording,
            commands::stop_recording,
            commands::list_recordings,
            commands::convert_to_wav,
            commands::delete_recording,
            commands::check_platform_support,
            commands::open_system_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
