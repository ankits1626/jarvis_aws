// Intelligence module - AI enrichment for gems using IntelligenceKit sidecar

pub mod provider;
pub mod intelligencekit_provider;
pub mod noop_provider;

pub use provider::{AvailabilityResult, IntelProvider};
pub use intelligencekit_provider::IntelligenceKitProvider;
pub use noop_provider::NoOpProvider;

use std::sync::Arc;

/// Create an IntelProvider with graceful fallback to NoOpProvider
/// 
/// Attempts to create IntelligenceKitProvider. If it fails (binary missing,
/// spawn error, etc.), returns NoOpProvider instead. This ensures the app
/// continues to work without AI enrichment.
/// 
/// # Returns
/// 
/// Always returns a valid IntelProvider - either IntelligenceKitProvider or NoOpProvider.
pub async fn create_provider(app_handle: tauri::AppHandle) -> Arc<dyn IntelProvider> {
    match IntelligenceKitProvider::new(app_handle).await {
        Ok(provider) => {
            eprintln!("IntelligenceKit: Provider initialized successfully");
            Arc::new(provider)
        }
        Err(e) => {
            eprintln!("IntelligenceKit: Failed to initialize provider: {}", e);
            eprintln!("IntelligenceKit: Using NoOpProvider (AI enrichment disabled)");
            Arc::new(NoOpProvider::new(e))
        }
    }
}
