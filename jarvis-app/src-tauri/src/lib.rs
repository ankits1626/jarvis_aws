// Module declarations
pub mod agents;
pub mod browser;
pub mod commands;
pub mod error;
pub mod files;
pub mod gems;
pub mod intelligence;
pub mod knowledge;
pub mod logging;
pub mod platform;
pub mod projects;
pub mod recording;
pub mod search;
pub mod settings;
pub mod shortcuts;
pub mod transcription;
pub mod wav;

use std::sync::{Arc, Mutex, RwLock};
use tauri::Manager;
use files::FileManager;
use gems::{GemStore, SqliteGemStore};
use intelligence::{LlmModelManager, VenvManager};
use recording::RecordingManager;
use search::{FtsResultProvider, QmdResultProvider, SearchResultProvider};
use settings::{ModelManager, SettingsManager};
use shortcuts::ShortcutManager;
use transcription::{TranscriptionConfig, TranscriptionManager, HybridProvider, WhisperKitProvider, TranscriptionProvider};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize file logging before anything else
    // Logs go to ~/Library/Application Support/com.jarvis.app/logs/
    if let Some(logs_dir) = logging::logs_dir() {
        logging::init(&logs_dir);
    }
    eprintln!("=== Jarvis App Starting ===");

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
            
            // Initialize GemStore (SqliteGemStore as default implementation)
            let gem_store = SqliteGemStore::new()
                .map_err(|e| format!("Failed to initialize gem store: {}", e))?;
            let shared_conn = gem_store.get_conn();  // Get conn BEFORE Arc wrapping
            let gem_store_arc = Arc::new(gem_store) as Arc<dyn GemStore>;
            app.manage(gem_store_arc.clone());
            
            // Initialize ProjectStore with same connection
            let project_store = projects::SqliteProjectStore::new(shared_conn)
                .map_err(|e| format!("Failed to initialize project store: {}", e))?;
            let project_store_arc = Arc::new(project_store) as Arc<dyn projects::ProjectStore>;
            app.manage(project_store_arc);
            
            // Initialize Knowledge Store
            let app_data_dir = app.path().app_data_dir()
                .map_err(|e| format!("Failed to get app data dir: {}", e))?;
            let knowledge_path = app_data_dir.join("knowledge");
            let knowledge_event_emitter = Arc::new(
                knowledge::store::TauriKnowledgeEventEmitter::new(app.handle().clone())
            ) as Arc<dyn knowledge::store::KnowledgeEventEmitter + Send + Sync>;
            let knowledge_store = knowledge::LocalKnowledgeStore::new(
                knowledge_path.clone(),
                knowledge_event_emitter.clone(),
            );
            let knowledge_store_arc = Arc::new(knowledge_store) as Arc<dyn knowledge::KnowledgeStore>;
            app.manage(knowledge_store_arc.clone());
            
            // Initialize SettingsManager and add to managed state (wrapped in Arc<RwLock>)
            let settings_manager = SettingsManager::new()
                .map_err(|e| format!("Failed to initialize SettingsManager: {}", e))?;
            let settings_manager_arc = Arc::new(RwLock::new(settings_manager));
            app.manage(settings_manager_arc.clone());
            
            // Initialize VenvManager for MLX Python environment
            let venv_manager = VenvManager::new()
                .map_err(|e| format!("Failed to initialize VenvManager: {}", e))?;
            let venv_manager_arc = Arc::new(venv_manager);
            app.manage(venv_manager_arc.clone());

            // Load settings for provider initialization
            let settings = settings_manager_arc.read()
                .expect("Failed to acquire settings read lock")
                .get();

            // Resolve Python path: use venv if ready, else base python from settings
            let resolved_python = venv_manager_arc.resolve_python_path(&settings.intelligence.python_path);
            eprintln!("Intelligence: Resolved python path: {}", resolved_python);

            // Initialize LlmModelManager with resolved Python path
            let llm_manager = LlmModelManager::new(app.handle().clone(), resolved_python)
                .map_err(|e| format!("Failed to initialize LlmModelManager: {}", e))?;
            let llm_manager_arc = Arc::new(llm_manager);
            app.manage(llm_manager_arc.clone());

            // Initialize IntelProvider with fallback chain based on settings
            let app_handle = app.handle().clone();
            let llm_manager_for_provider = llm_manager_arc.clone();
            let venv_manager_for_provider = venv_manager_arc.clone();
            let (intel_provider, mlx_provider) = tauri::async_runtime::block_on(async move {
                intelligence::create_provider(app_handle, &settings, &llm_manager_for_provider, &venv_manager_for_provider).await
            });
            
            let availability = tauri::async_runtime::block_on(intel_provider.check_availability());
            if availability.available {
                eprintln!("Intelligence: AI enrichment available");
            } else {
                eprintln!("Intelligence: AI enrichment unavailable - {}", 
                    availability.reason.unwrap_or_else(|| "Unknown reason".to_string()));
            }
            
            // Create IntelQueue for serialized provider access
            // Must run inside block_on so tokio::spawn in IntelQueue::new() has a runtime context
            let intel_queue = tauri::async_runtime::block_on(async {
                intelligence::IntelQueue::new(intel_provider.clone())
            });
            app.manage(intel_queue);
            
            app.manage(intel_provider);
            
            // Wrap MlxProvider in Arc<tokio::sync::Mutex<>> for Phase 5 model switching
            // This allows switch_llm_model command to mutate the provider reference
            let mlx_provider_mutex = Arc::new(tokio::sync::Mutex::new(mlx_provider));
            app.manage(mlx_provider_mutex);
            
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
            
            // Initialize Co-Pilot agent state (None = not running)
            app.manage(Arc::new(tokio::sync::Mutex::new(None::<agents::copilot::CoPilotAgent>)));
            
            // Initialize Chatbot state
            app.manage(tokio::sync::Mutex::new(agents::chatbot::Chatbot::new()));
            
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
            
            // Run knowledge migration in background (non-blocking)
            let ks_clone = knowledge_store_arc.clone();
            let gs_clone = gem_store_arc.clone();
            let ee_clone = knowledge_event_emitter.clone();
            let kp_clone = knowledge_path.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = knowledge::migration::check_and_run_migration(
                    ks_clone.as_ref(),
                    gs_clone.as_ref(),
                    ee_clone.as_ref(),
                    &kp_clone,
                ).await {
                    eprintln!("Knowledge migration error: {}", e);
                }
            });
            
            // Initialize Search Provider
            // Read search settings
            let (search_enabled, search_accuracy) = {
                let manager = app.state::<Arc<RwLock<SettingsManager>>>();
                let settings = manager.read().expect("Failed to acquire settings read lock").get();
                (settings.search.semantic_search_enabled, settings.search.semantic_search_accuracy)
            };

            let search_provider: Arc<dyn SearchResultProvider> = if search_enabled {
                // Try to initialize QMD provider
                match tauri::async_runtime::block_on(QmdResultProvider::find_qmd_binary()) {
                    Some(qmd_path) => {
                        let knowledge_path = app.path().app_data_dir()
                            .expect("Failed to get app data dir")
                            .join("knowledge");
                        let qmd = QmdResultProvider::new(qmd_path.clone(), knowledge_path, search_accuracy);
                        
                        // Check availability before committing
                        let availability = tauri::async_runtime::block_on(qmd.check_availability());
                        if availability.available {
                            eprintln!("Search: Using QMD semantic search provider ({})", qmd_path.display());
                            Arc::new(qmd)
                        } else {
                            eprintln!("Search: QMD unavailable ({}), falling back to FTS5",
                                availability.reason.unwrap_or_else(|| "unknown".to_string()));
                            Arc::new(FtsResultProvider::new(gem_store_arc.clone()))
                        }
                    }
                    None => {
                        eprintln!("Search: QMD binary not found, falling back to FTS5");
                        Arc::new(FtsResultProvider::new(gem_store_arc.clone()))
                    }
                }
            } else {
                eprintln!("Search: Using FTS5 keyword search provider (default)");
                Arc::new(FtsResultProvider::new(gem_store_arc.clone()))
            };
            
            app.manage(search_provider);
            
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
            commands::list_browser_tabs,
            commands::prepare_tab_gist,
            commands::export_gist,
            commands::save_gem,
            commands::list_gems,
            search::commands::search_gems,
            search::commands::check_search_availability,
            search::commands::setup_semantic_search,
            search::commands::rebuild_search_index,
            commands::delete_gem,
            commands::get_gem,
            commands::enrich_gem,
            commands::transcribe_gem,
            commands::transcribe_recording,
            commands::check_recording_gem,
            commands::check_recording_gems_batch,
            commands::save_recording_gem,
            commands::check_intel_availability,
            commands::check_mlx_dependencies,
            commands::filter_gems_by_tag,
            commands::capture_claude_conversation,
            commands::check_claude_panel,
            commands::check_accessibility_permission,
            commands::prepare_tab_gist_with_claude,
            commands::list_llm_models,
            commands::download_llm_model,
            commands::cancel_llm_download,
            commands::delete_llm_model,
            commands::switch_llm_model,
            commands::setup_mlx_venv,
            commands::reset_mlx_venv,
            commands::start_copilot,
            commands::stop_copilot,
            commands::get_copilot_state,
            commands::dismiss_copilot_question,
            commands::chat_with_recording,
            commands::chat_send_message,
            commands::chat_get_history,
            commands::chat_end_session,
            commands::get_saved_transcript,
            knowledge::commands::get_gem_knowledge,
            knowledge::commands::get_gem_knowledge_assembled,
            knowledge::commands::get_gem_knowledge_subfile,
            knowledge::commands::regenerate_gem_knowledge,
            knowledge::commands::check_knowledge_availability,
            projects::commands::create_project,
            projects::commands::list_projects,
            projects::commands::get_project,
            projects::commands::update_project,
            projects::commands::delete_project,
            projects::commands::add_gems_to_project,
            projects::commands::remove_gem_from_project,
            projects::commands::get_project_gems,
            projects::commands::get_gem_projects,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
