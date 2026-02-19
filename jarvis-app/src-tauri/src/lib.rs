// Module declarations
pub mod browser;
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
use transcription::{TranscriptionConfig, TranscriptionManager, HybridProvider, WhisperKitProvider, TranscriptionProvider};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
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
            
            // Initialize TranscriptionManager with selected provider
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
                // Select provider based on transcription_engine setting
                let engine = settings.transcription.transcription_engine.as_str();
                eprintln!("TranscriptionManager: Selected engine: {}", engine);
                
                // Helper closure to create and initialize HybridProvider fallback
                let create_hybrid_fallback = || -> Option<Box<dyn TranscriptionProvider>> {
                    let mut hybrid = HybridProvider::new(&settings.transcription, app.handle().clone());
                    match hybrid.initialize(&transcription_config) {
                        Ok(()) => {
                            eprintln!("TranscriptionManager: Initialized fallback provider '{}'", hybrid.name());
                            Some(Box::new(hybrid))
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to initialize fallback HybridProvider: {}", e);
                            None
                        }
                    }
                };
                
                let provider: Option<Box<dyn TranscriptionProvider>> = match engine {
                    "whisperkit" => {
                        // Try to create WhisperKitProvider
                        let model_name = &settings.transcription.whisperkit_model;
                        let mut whisperkit_provider = WhisperKitProvider::new(model_name);
                        
                        if !whisperkit_provider.is_available() {
                            let reason = whisperkit_provider.unavailable_reason()
                                .unwrap_or("Unknown reason");
                            eprintln!("Warning: WhisperKit unavailable: {}", reason);
                            eprintln!("Falling back to whisper-rs (HybridProvider)");
                            create_hybrid_fallback()
                        } else {
                            // WhisperKit is available, try to initialize it
                            match whisperkit_provider.initialize(&transcription_config) {
                                Ok(()) => {
                                    eprintln!("TranscriptionManager: Initialized with provider '{}'", whisperkit_provider.name());
                                    Some(Box::new(whisperkit_provider))
                                }
                                Err(e) => {
                                    eprintln!("Warning: Failed to initialize WhisperKitProvider: {}", e);
                                    eprintln!("Falling back to whisper-rs (HybridProvider)");
                                    create_hybrid_fallback()
                                }
                            }
                        }
                    }
                    _ => {
                        // Default to HybridProvider (whisper-rs)
                        let mut hybrid = HybridProvider::new(&settings.transcription, app.handle().clone());
                        match hybrid.initialize(&transcription_config) {
                            Ok(()) => {
                                eprintln!("TranscriptionManager: Initialized with provider '{}'", hybrid.name());
                                Some(Box::new(hybrid))
                            }
                            Err(e) => {
                                eprintln!("Warning: Failed to initialize HybridProvider: {}", e);
                                None
                            }
                        }
                    }
                };
                
                // If we successfully created a provider, add TranscriptionManager to state
                if let Some(provider) = provider {
                    let transcription_manager = TranscriptionManager::new(
                        provider,
                        app.handle().clone(),
                        settings.transcription.window_duration,
                    );
                    app.manage(tokio::sync::Mutex::new(transcription_manager));
                } else {
                    eprintln!("Transcription will be disabled. Recording will continue to work.");
                }
            }
            
            // Initialize ShortcutManager and register shortcuts
            let shortcut_manager = ShortcutManager::new(app.handle().clone());
            shortcut_manager.register_shortcuts()
                .map_err(|e| format!("Failed to register shortcuts: {}", e))?;
            
            // Request notification permission (required for macOS to show notifications)
            {
                use tauri_plugin_notification::NotificationExt;
                match app.handle().notification().request_permission() {
                    Ok(state) => eprintln!("Notification permission: {:?}", state),
                    Err(e) => eprintln!("Warning: Failed to request notification permission: {}", e),
                }
            }

            // Initialize BrowserObserver and add to managed state (wrapped in Arc<tokio::sync::Mutex>)
            let browser_observer = browser::BrowserObserver::new(app.handle().clone());
            let browser_observer_arc = Arc::new(tokio::sync::Mutex::new(browser_observer));
            app.manage(browser_observer_arc.clone());

            // Auto-start browser observer if enabled in settings
            if settings.browser.observer_enabled {
                eprintln!("BrowserObserver: Auto-starting observer (enabled in settings)");
                let observer_clone = browser_observer_arc.clone();
                tauri::async_runtime::spawn(async move {
                    let mut observer = observer_clone.lock().await;
                    if let Err(e) = observer.start().await {
                        eprintln!("Warning: Failed to auto-start browser observer: {}", e);
                        eprintln!("Observer can be started manually from settings.");
                    }
                });
            } else {
                eprintln!("BrowserObserver: Auto-start disabled in settings");
            }
            
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
            commands::check_whisperkit_status,
            commands::list_whisperkit_models,
            commands::download_whisperkit_model,
            commands::start_browser_observer,
            commands::stop_browser_observer,
            commands::fetch_youtube_gist,
            commands::get_observer_status,
            commands::get_browser_settings,
            commands::update_browser_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
