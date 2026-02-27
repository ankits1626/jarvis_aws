// IntelProvider trait - backend-agnostic intelligence provider interface

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Result of an availability check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityResult {
    pub available: bool,
    pub reason: Option<String>,
}

/// Result of transcript generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptResult {
    pub language: String,
    pub transcript: String,
}

/// Backend-agnostic intelligence provider interface
/// 
/// This trait abstracts the intelligence backend, enabling swappable implementations
/// without modifying commands or frontend code. The default implementation is
/// IntelligenceKitProvider, which uses Apple's on-device Foundation Models.
#[async_trait]
pub trait IntelProvider: Send + Sync {
    /// Check if the provider is available and ready to process requests
    async fn check_availability(&self) -> AvailabilityResult;
    
    /// Generate topic tags from content
    /// 
    /// Returns a vector of short topic strings (1-3 words each).
    /// Implementation should request 3-5 tags but accept 1-10, trimming to max 5.
    /// Returns error if model returns empty array.
    async fn generate_tags(&self, content: &str) -> Result<Vec<String>, String>;
    
    /// Generate a one-sentence summary from content
    /// 
    /// Returns a single sentence capturing the key idea.
    async fn summarize(&self, content: &str) -> Result<String, String>;
    
    /// Generate transcript from audio file
    /// 
    /// Processes an audio file and returns a transcript with detected language.
    /// Default implementation returns error for providers that don't support transcription.
    /// 
    /// # Arguments
    /// 
    /// * `audio_path` - Path to the audio file (.wav or .pcm format)
    /// 
    /// # Returns
    /// 
    /// * `Ok(TranscriptResult)` - Transcript with detected language
    /// * `Err(String)` - Error message if transcription fails or is not supported
    async fn generate_transcript(&self, _audio_path: &std::path::Path) -> Result<TranscriptResult, String> {
        Err("Transcript generation not supported by this provider".to_string())
    }
}
