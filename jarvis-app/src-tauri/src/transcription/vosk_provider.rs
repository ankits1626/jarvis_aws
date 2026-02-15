// Vosk provider for instant partial transcriptions
// Provides <100ms latency partials with is_final=false
// Standalone wrapper - does NOT implement TranscriptionProvider trait
//
// Uses the vosk crate (safe FFI bindings) with libvosk.dylib
// from the Python vosk package (universal binary: x86_64 + arm64)

use std::path::PathBuf;
use vosk::{Model, Recognizer, DecodingState};

/// Vosk provider for fast partial transcriptions
///
/// Wraps the Vosk speech recognition engine for instant partials (<100ms).
/// Gracefully degrades if the model or native library is unavailable.
pub struct VoskProvider {
    #[allow(dead_code)]
    model: Option<Model>,
    recognizer: Option<Recognizer>,
    available: bool,
}

impl VoskProvider {
    /// Create a new VoskProvider instance
    ///
    /// Loads the Vosk model from the given path (or default ~/.jarvis/models/vosk-model-small-en-us-0.15).
    /// If loading fails, gracefully degrades (available=false).
    pub fn new(model_path: Option<PathBuf>) -> Self {
        // Suppress verbose Kaldi logging
        vosk::set_log_level(vosk::LogLevel::Error);

        let path = model_path.unwrap_or_else(|| {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            home.join(".jarvis/models/vosk-model-small-en-us-0.15")
        });

        if !path.exists() {
            eprintln!(
                "Warning: Vosk model not found at {:?}. Vosk partials disabled.",
                path
            );
            return Self::unavailable();
        }

        let path_str = path.to_string_lossy().to_string();
        match Model::new(&path_str) {
            Some(model) => {
                // Create recognizer at 16kHz
                match Recognizer::new(&model, 16000.0) {
                    Some(recognizer) => {
                        eprintln!("Vosk loaded successfully from {:?}", path);
                        Self {
                            model: Some(model),
                            recognizer: Some(recognizer),
                            available: true,
                        }
                    }
                    None => {
                        eprintln!("Warning: Failed to create Vosk recognizer. Vosk partials disabled.");
                        Self::unavailable()
                    }
                }
            }
            None => {
                eprintln!(
                    "Warning: Failed to load Vosk model from {:?}. Vosk partials disabled.",
                    path
                );
                Self::unavailable()
            }
        }
    }

    fn unavailable() -> Self {
        Self {
            model: None,
            recognizer: None,
            available: false,
        }
    }

    /// Feed audio samples to Vosk recognizer
    ///
    /// Accepts i16 PCM audio at 16kHz mono.
    /// Returns true if Vosk detected end of utterance (can call result()).
    pub fn accept_waveform(&mut self, samples: &[i16]) -> bool {
        if let Some(recognizer) = &mut self.recognizer {
            match recognizer.accept_waveform(samples) {
                Ok(DecodingState::Finalized) => true,
                Ok(_) => false,
                Err(e) => {
                    eprintln!("Warning: Vosk accept_waveform error: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    /// Get instant partial result from Vosk
    ///
    /// Returns the current partial transcription text, or None if empty.
    pub fn partial_result(&mut self) -> Option<String> {
        if let Some(recognizer) = &mut self.recognizer {
            let result = recognizer.partial_result();
            let text = result.partial.trim();
            if text.is_empty() {
                None
            } else {
                Some(text.to_string())
            }
        } else {
            None
        }
    }

    /// Get final result from Vosk
    ///
    /// Returns the finalized transcription text, or None if empty.
    pub fn final_result(&mut self) -> Option<String> {
        if let Some(recognizer) = &mut self.recognizer {
            let result = recognizer.result();
            match result.single() {
                Some(single) => {
                    let text = single.text.trim();
                    if text.is_empty() {
                        None
                    } else {
                        Some(text.to_string())
                    }
                }
                None => None,
            }
        } else {
            None
        }
    }

    /// Check if Vosk is available
    pub fn is_available(&self) -> bool {
        self.available
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vosk_creation() {
        let vosk = VoskProvider::new(None);
        // Vosk should be available if model exists at default path
        if vosk.is_available() {
            eprintln!("Vosk is available - model found");
        } else {
            eprintln!("Vosk not available - model not found (expected if model not downloaded)");
        }
    }

    #[test]
    fn test_vosk_graceful_degradation_missing_model() {
        let vosk = VoskProvider::new(Some(PathBuf::from("/nonexistent/path")));
        assert!(!vosk.is_available());
    }

    #[test]
    fn test_vosk_accept_waveform_when_unavailable() {
        let mut vosk = VoskProvider::new(Some(PathBuf::from("/nonexistent/path")));
        let samples = vec![0i16; 1600];
        assert!(!vosk.accept_waveform(&samples));
    }

    #[test]
    fn test_vosk_partial_result_when_unavailable() {
        let mut vosk = VoskProvider::new(Some(PathBuf::from("/nonexistent/path")));
        assert_eq!(vosk.partial_result(), None);
    }

    #[test]
    fn test_vosk_silence_produces_empty_partial() {
        let mut vosk = VoskProvider::new(None);
        if !vosk.is_available() {
            eprintln!("Skipping silence test - Vosk model not available");
            return;
        }

        // Feed silence (all zeros) - should produce empty partial
        let silence = vec![0i16; 16000]; // 1 second at 16kHz
        vosk.accept_waveform(&silence);
        let partial = vosk.partial_result();
        // Silence should produce None (empty text)
        assert!(partial.is_none(), "Silence should produce no partial text");
    }
}
