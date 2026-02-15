use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::PathBuf;

// MARK: - TranscriptionSegment

/// Represents a piece of transcribed text with timing and finality information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    /// Transcribed text
    pub text: String,
    
    /// Start time in milliseconds
    pub start_ms: i64,
    
    /// End time in milliseconds
    pub end_ms: i64,
    
    /// false = Vosk partial (gray text), true = Whisper final (normal text)
    pub is_final: bool,
}

impl TranscriptionSegment {
    /// Creates a new transcription segment
    pub fn new(text: String, start_ms: i64, end_ms: i64, is_final: bool) -> Self {
        Self {
            text,
            start_ms,
            end_ms,
            is_final,
        }
    }
}

/// Helper function to check if two segments overlap in time
pub fn segments_overlap(seg1: &TranscriptionSegment, seg2: &TranscriptionSegment) -> bool {
    (seg1.start_ms < seg2.end_ms) && (seg2.start_ms < seg1.end_ms)
}

// MARK: - TranscriptionConfig

/// Holds runtime configuration for transcription.
#[derive(Debug, Clone)]
pub struct TranscriptionConfig {
    /// Window duration in seconds (default: 3.0, range: 2.0-30.0)
    pub window_duration_secs: f32,
    
    /// Overlap duration in seconds (default: 0.5, must be < window_duration)
    pub overlap_duration_secs: f32,
    
    /// Path to Whisper GGML model
    pub whisper_model_path: PathBuf,
    
    /// Path to Silero VAD ONNX model
    pub vad_model_path: PathBuf,
    
    /// Path to Vosk model directory
    pub vosk_model_path: PathBuf,
    
    /// Number of threads for Whisper (None = auto-detect)
    pub whisper_threads: Option<usize>,
}

impl TranscriptionConfig {
    /// Creates a new configuration from environment variables
    pub fn from_env() -> Self {
        let home = dirs::home_dir().expect("Failed to get home directory");
        let models_dir = home.join(".jarvis/models");
        
        Self {
            window_duration_secs: 3.0,
            overlap_duration_secs: 0.5,
            whisper_model_path: std::env::var("JARVIS_WHISPER_MODEL")
                .map(PathBuf::from)
                .unwrap_or_else(|_| models_dir.join("ggml-base.en.bin")),
            vad_model_path: std::env::var("JARVIS_VAD_MODEL")
                .map(PathBuf::from)
                .unwrap_or_else(|_| models_dir.join("silero_vad.onnx")),
            vosk_model_path: std::env::var("JARVIS_VOSK_MODEL")
                .map(PathBuf::from)
                .unwrap_or_else(|_| models_dir.join("vosk-model-small-en-us-0.15")),
            whisper_threads: std::env::var("JARVIS_WHISPER_THREADS")
                .ok()
                .and_then(|s| s.parse().ok()),
        }
    }
    
    /// Validates the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.window_duration_secs < 2.0 || self.window_duration_secs > 30.0 {
            return Err(format!(
                "Window duration must be between 2 and 30 seconds, got {}",
                self.window_duration_secs
            ));
        }
        
        if self.overlap_duration_secs >= self.window_duration_secs {
            return Err(format!(
                "Overlap duration ({}) must be less than window duration ({})",
                self.overlap_duration_secs, self.window_duration_secs
            ));
        }
        
        Ok(())
    }
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

// MARK: - TranscriptionStatus

/// Represents the current state of the transcription system.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptionStatus {
    /// Not transcribing
    Idle,
    
    /// Currently transcribing
    Active,
    
    /// Error occurred
    Error,
    
    /// Whisper model missing, transcription unavailable
    Disabled,
}

// MARK: - TranscriptionProvider Trait

/// Trait-based abstraction for speech-to-text engines.
/// 
/// This trait enables provider swapping (HybridProvider, AWS Transcribe, etc.)
/// without changing the orchestration logic in TranscriptionManager.
/// 
/// # Object Safety
/// This trait is object-safe and can be used as `Box<dyn TranscriptionProvider>`.
/// 
/// # Thread Safety
/// Implementations must be Send + Sync for use across async tasks.
pub trait TranscriptionProvider: Send + Sync {
    /// Returns the name of this provider (e.g., "hybrid-vad-vosk-whisper")
    fn name(&self) -> &str;
    
    /// Initializes the provider with the given configuration.
    /// 
    /// This method prepares resources such as loading models or establishing connections.
    /// 
    /// # Errors
    /// Returns an error if initialization fails (e.g., model file not found).
    fn initialize(&mut self, config: &TranscriptionConfig) -> Result<(), Box<dyn Error>>;
    
    /// Transcribes the given audio samples.
    /// 
    /// # Arguments
    /// * `audio` - f32 audio samples at 16kHz mono (universal format)
    /// 
    /// # Returns
    /// A vector of transcription segments with text, timestamps, and is_final flag.
    /// 
    /// # Errors
    /// Returns an error if transcription fails.
    fn transcribe(&mut self, audio: &[f32]) -> Result<Vec<TranscriptionSegment>, Box<dyn Error>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_segment_creation() {
        let segment = TranscriptionSegment::new(
            "Hello world".to_string(),
            0,
            1000,
            true
        );
        
        assert_eq!(segment.text, "Hello world");
        assert_eq!(segment.start_ms, 0);
        assert_eq!(segment.end_ms, 1000);
        assert!(segment.is_final);
    }
    
    #[test]
    fn test_segments_overlap_true() {
        let seg1 = TranscriptionSegment::new("A".to_string(), 0, 1000, false);
        let seg2 = TranscriptionSegment::new("B".to_string(), 500, 1500, true);
        
        assert!(segments_overlap(&seg1, &seg2));
        assert!(segments_overlap(&seg2, &seg1)); // Commutative
    }
    
    #[test]
    fn test_segments_overlap_false() {
        let seg1 = TranscriptionSegment::new("A".to_string(), 0, 1000, false);
        let seg2 = TranscriptionSegment::new("B".to_string(), 1000, 2000, true);
        
        assert!(!segments_overlap(&seg1, &seg2));
        assert!(!segments_overlap(&seg2, &seg1));
    }
    
    #[test]
    fn test_config_validation_window_too_small() {
        let mut config = TranscriptionConfig::default();
        config.window_duration_secs = 1.0;
        
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_config_validation_window_too_large() {
        let mut config = TranscriptionConfig::default();
        config.window_duration_secs = 31.0;
        
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_config_validation_overlap_too_large() {
        let mut config = TranscriptionConfig::default();
        config.window_duration_secs = 3.0;
        config.overlap_duration_secs = 3.0;
        
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_config_validation_valid() {
        let config = TranscriptionConfig::default();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_config_default_values() {
        let config = TranscriptionConfig::default();
        
        // Property 15: Default Window Duration = 3.0
        assert_eq!(config.window_duration_secs, 3.0);
        
        // Property 16: Default Overlap Duration = 0.5
        assert_eq!(config.overlap_duration_secs, 0.5);
    }
}
