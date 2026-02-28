// Intelligence module - AI enrichment for gems using IntelligenceKit sidecar

pub mod provider;
pub mod intelligencekit_provider;
pub mod llm_model_manager;
pub mod mlx_provider;
pub mod noop_provider;
pub mod queue;
pub mod utils;
pub mod venv_manager;

pub use provider::{AvailabilityResult, IntelProvider};
pub use intelligencekit_provider::IntelligenceKitProvider;
pub use llm_model_manager::{LlmModelInfo, LlmModelManager};
pub use mlx_provider::MlxProvider;
pub use noop_provider::NoOpProvider;
pub use queue::{IntelCommand, IntelQueue, IntelResponse};
pub use venv_manager::VenvManager;

use crate::settings::Settings;
use std::sync::Arc;

/// Create an IntelProvider with fallback chain based on settings
/// 
/// Attempts to create providers in this order:
/// 1. MLX (if provider = "mlx" and model downloaded)
/// 2. IntelligenceKit (if provider = "intelligencekit" or MLX fails)
/// 3. NoOpProvider (if all else fails)
/// 
/// # Arguments
/// 
/// * `app_handle` - Tauri app handle for resource resolution
/// * `settings` - Application settings containing intelligence configuration
/// * `llm_manager` - LLM model manager for resolving model paths
/// 
/// # Returns
/// 
/// Returns a tuple of:
/// - `Arc<dyn IntelProvider>` - The active provider (trait object)
/// - `Option<Arc<MlxProvider>>` - Direct reference to MlxProvider if active (for model switching)
pub async fn create_provider(
    app_handle: tauri::AppHandle,
    settings: &Settings,
    llm_manager: &LlmModelManager,
    venv_manager: &VenvManager,
) -> (Arc<dyn IntelProvider>, Option<Arc<MlxProvider>>) {
    let provider_name = &settings.intelligence.provider;

    eprintln!("Intelligence: Requested provider: {}", provider_name);

    // Try MLX if requested
    if provider_name == "mlx" {
        let model_id = &settings.intelligence.active_model;
        // Resolve python path: use venv if ready, else base python from settings
        let python_path = venv_manager.resolve_python_path(&settings.intelligence.python_path);
        eprintln!("Intelligence: Resolved python path: {}", python_path);
        
        // Use LlmModelManager to resolve the correct model path
        let model_path = llm_manager.model_path(model_id);
        
        // Check if model exists
        if !model_path.exists() {
            eprintln!("Intelligence: MLX model '{}' not found at {:?}", model_id, model_path);
            eprintln!("Intelligence: Please download a model in Settings before using MLX");
            eprintln!("Intelligence: Falling back to IntelligenceKit");
            return try_intelligencekit_fallback(app_handle).await;
        }
        
        // Try to create MlxProvider
        match MlxProvider::new(app_handle.clone(), model_path, python_path).await {
            Ok(provider) => {
                eprintln!("Intelligence: MlxProvider initialized successfully with model '{}'", model_id);
                let provider_arc = Arc::new(provider);
                return (provider_arc.clone() as Arc<dyn IntelProvider>, Some(provider_arc));
            }
            Err(e) => {
                eprintln!("Intelligence: Failed to initialize MlxProvider: {}", e);
                
                // Provide specific guidance based on error type
                if e.contains("Python not found") {
                    eprintln!("Intelligence: Python is not installed or not in PATH");
                    eprintln!("Intelligence: Install Python 3.10+ or update python_path in Settings");
                } else if e.contains("MLX dependencies not installed") {
                    eprintln!("Intelligence: MLX Python packages are not installed");
                    eprintln!("Intelligence: Use 'Setup MLX Environment' in Settings to auto-install");
                } else if e.contains("model") {
                    eprintln!("Intelligence: Model loading failed - the model may be corrupted");
                    eprintln!("Intelligence: Try deleting and re-downloading the model in Settings");
                }
                
                eprintln!("Intelligence: Falling back to IntelligenceKit");
                return try_intelligencekit_fallback(app_handle).await;
            }
        }
    }
    
    // Try IntelligenceKit if requested or as fallback
    if provider_name == "intelligencekit" {
        return try_intelligencekit_fallback(app_handle).await;
    }
    
    // API provider not implemented yet, use NoOpProvider
    if provider_name == "api" {
        eprintln!("Intelligence: API provider not implemented, using NoOpProvider");
        return (Arc::new(NoOpProvider::new("API provider not implemented".to_string())), None);
    }
    
    // Unknown provider, use NoOpProvider
    eprintln!("Intelligence: Unknown provider '{}', using NoOpProvider", provider_name);
    (Arc::new(NoOpProvider::new(format!("Unknown provider: {}", provider_name))), None)
}

/// Try to create IntelligenceKitProvider, fall back to NoOpProvider on failure
async fn try_intelligencekit_fallback(
    app_handle: tauri::AppHandle,
) -> (Arc<dyn IntelProvider>, Option<Arc<MlxProvider>>) {
    match IntelligenceKitProvider::new(app_handle).await {
        Ok(provider) => {
            eprintln!("Intelligence: IntelligenceKitProvider initialized successfully");
            (Arc::new(provider) as Arc<dyn IntelProvider>, None)
        }
        Err(e) => {
            eprintln!("Intelligence: Failed to initialize IntelligenceKitProvider: {}", e);
            eprintln!("Intelligence: Using NoOpProvider (AI enrichment disabled)");
            (Arc::new(NoOpProvider::new(e)), None)
        }
    }
}
