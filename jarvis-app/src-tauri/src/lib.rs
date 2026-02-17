// Module declarations
pub mod commands;
pub mod error;
pub mod files;
pub mod platform;
pub mod recording;
pub mod settings;
pub mod shortcuts;
pub mod transcription;
pub mod wav;

use std::sync::{Arc, Mutex, RwLock};
use tauri::Manager;
use files::FileManager;
use recording::RecordingManager;
use settings::{ModelManager, SettingsManager};
use shortcuts::ShortcutManager;
use transcription::{TranscriptionConfig, TranscriptionManager, HybridProvider, TranscriptionProvider};

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
            
            // Initialize SettingsManager and add to managed state (wrapped in Arc<RwLock>)
            let settings_manager = SettingsManager::new()
                .map_err(|e| format!("Failed to initialize SettingsManager: {}", e))?;
            app.manage(Arc::new(RwLock::new(settings_manager)));
            
            // Initialize ModelManager and add to managed state (wrapped in Arc)
            let model_manager = ModelManager::new(app.handle().clone())
                .map_err(|e| format!("Failed to initialize ModelManager: {}", e))?;
            app.manage(Arc::new(model_manager));
            
            // Initialize RecordingManager with AppHandle and add to managed state (wrapped in Mutex)
            let recording_manager = RecordingManager::new(app.handle().clone());
            app.manage(Mutex::new(recording_manager));
            
            // Initialize TranscriptionManager with HybridProvider
            // Load settings from SettingsManager
            let settings_manager = app.state::<Arc<RwLock<SettingsManager>>>();
            let settings = settings_manager.read()
                .expect("Failed to acquire settings read lock")
                .get();
            
            // Create TranscriptionConfig from settings with environment variable overrides
            let transcription_config = TranscriptionConfig::from_settings(&settings.transcription);
            
            // Validate configuration - skip initialization if invalid
            if let Err(e) = transcription_config.validate() {
                eprintln!("Warning: Invalid transcription configuration: {}", e);
                eprintln!("Transcription will be disabled. Recording will continue to work.");
                // Don't initialize provider with invalid config
            } else {
                // Initialize HybridProvider with settings
                let mut provider = HybridProvider::new(&settings.transcription, app.handle().clone());
                
                // Initialize the provider with config (loads models)
                match provider.initialize(&transcription_config) {
                    Ok(()) => {
                        eprintln!("TranscriptionManager: Initialized with provider '{}'", provider.name());
                        
                        // Create TranscriptionManager with the provider
                        let transcription_manager = TranscriptionManager::new(
                            Box::new(provider),
                            app.handle().clone()
                        );
                        
                        // Add to managed state wrapped in tokio::sync::Mutex (not std::sync::Mutex)
                        app.manage(tokio::sync::Mutex::new(transcription_manager));
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to initialize HybridProvider: {}", e);
                        eprintln!("Transcription will be disabled. Recording will continue to work.");
                        // Don't add TranscriptionManager to state - RecordingManager will handle gracefully
                    }
                }
            }
            
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
            commands::get_transcript,
            commands::get_transcription_status,
            commands::get_settings,
            commands::update_settings,
            commands::list_models,
            commands::download_model,
            commands::cancel_download,
            commands::delete_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
