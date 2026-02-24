// NoOpProvider - fallback provider when IntelligenceKit is unavailable

use async_trait::async_trait;

use super::provider::{AvailabilityResult, IntelProvider};

/// No-op provider that always returns unavailable
/// 
/// Used as a fallback when IntelligenceKit binary is missing or fails to spawn.
/// This enables graceful degradation - the app continues to work without AI enrichment.
pub struct NoOpProvider {
    reason: String,
}

impl NoOpProvider {
    /// Create a new NoOpProvider with the given unavailability reason
    pub fn new(reason: String) -> Self {
        Self { reason }
    }
}

#[async_trait]
impl IntelProvider for NoOpProvider {
    async fn check_availability(&self) -> AvailabilityResult {
        AvailabilityResult {
            available: false,
            reason: Some(self.reason.clone()),
        }
    }

    async fn generate_tags(&self, _content: &str) -> Result<Vec<String>, String> {
        Err("IntelligenceKit unavailable".to_string())
    }

    async fn summarize(&self, _content: &str) -> Result<String, String> {
        Err("IntelligenceKit unavailable".to_string())
    }
}
