// Hybrid provider composing VAD, Vosk, and Whisper
// Implements the complete transcription pipeline with graceful degradation

use std::error::Error;
use tauri::{AppHandle, Emitter};
use crate::transcription::provider::{TranscriptionProvider, TranscriptionSegment, TranscriptionConfig};
use crate::transcription::vad::SileroVad;
use crate::transcription::vosk_provider::VoskProvider;
use crate::transcription::whisper_provider::WhisperProvider;
use crate::settings::manager::TranscriptionSettings;

/// Hybrid provider orchestrating VAD → Vosk → Whisper pipeline
/// 
/// Combines three engines for optimal transcription:
/// - Silero VAD: Gates pipeline to skip silence (optional)
/// - Vosk: Provides instant partials <100ms (optional)
/// - Whisper: Provides accurate finals 1-2s (required)
/// 
/// Note: This provider expects to receive pre-windowed audio (3s chunks)
/// from TranscriptionManager's AudioBuffer. It processes each chunk directly
/// without additional buffering.
pub struct HybridProvider {
    vad: Option<SileroVad>,
    vosk: Option<VoskProvider>,
    whisper: WhisperProvider,
    app_handle: Option<AppHandle>,
}

impl HybridProvider {
    /// Create a new HybridProvider instance with settings
    /// 
    /// Conditionally initializes engines based on settings:
    /// - VAD: Initialized if vad_enabled is true, with configured threshold
    /// - Vosk: Initialized if vosk_enabled is true
    /// - Whisper: Always initialized (required)
    /// 
    /// # Arguments
    ///
    /// * `settings` - Transcription settings controlling which engines to enable
    /// * `app_handle` - Tauri app handle for emitting Vosk partial events directly
    pub fn new(settings: &TranscriptionSettings, app_handle: AppHandle) -> Self {
        // Conditionally initialize VAD based on settings
        let vad = if settings.vad_enabled {
            eprintln!("VAD: enabled (threshold={})", settings.vad_threshold);
            Some(SileroVad::new(None, settings.vad_threshold))
        } else {
            eprintln!("VAD: disabled (bypassed per settings)");
            None
        };
        
        // Conditionally initialize Vosk based on settings
        let vosk = if settings.vosk_enabled {
            let vosk_provider = VoskProvider::new(None);
            if vosk_provider.is_available() {
                eprintln!("Vosk: enabled");
                Some(vosk_provider)
            } else {
                eprintln!("Warning: Vosk enabled in settings but unavailable - will skip instant partials");
                None
            }
        } else {
            eprintln!("Vosk: disabled (bypassed per settings)");
            None
        };
        
        // Whisper is always initialized (required)
        eprintln!("Whisper: enabled (model={})", settings.whisper_model);
        let whisper = WhisperProvider::new();
        
        Self {
            vad,
            vosk,
            whisper,
            app_handle: Some(app_handle),
        }
    }
    
    /// Update settings at runtime
    /// 
    /// Reconfigures the provider based on new settings:
    /// - Reinitializes VAD if enabled state changed
    /// - Updates VAD threshold if VAD is enabled
    /// - Reinitializes Vosk if enabled state changed
    /// - Note: Whisper model changes require reinitialization via initialize()
    /// 
    /// # Arguments
    /// 
    /// * `settings` - New transcription settings to apply
    pub fn update_settings(&mut self, settings: &TranscriptionSettings) {
        // Handle VAD updates
        match (&mut self.vad, settings.vad_enabled) {
            (Some(vad), true) => {
                // VAD enabled and still enabled - just update threshold
                eprintln!("VAD: updating threshold to {}", settings.vad_threshold);
                vad.set_threshold(settings.vad_threshold);
            }
            (Some(_), false) => {
                // VAD was enabled, now disabled - remove it
                eprintln!("VAD: disabling");
                self.vad = None;
            }
            (None, true) => {
                // VAD was disabled, now enabled - initialize it
                eprintln!("VAD: enabling (threshold={})", settings.vad_threshold);
                self.vad = Some(SileroVad::new(None, settings.vad_threshold));
            }
            (None, false) => {
                // VAD disabled and still disabled - no change
            }
        }
        
        // Handle Vosk updates
        match (&self.vosk, settings.vosk_enabled) {
            (Some(_), false) => {
                // Vosk was enabled, now disabled - remove it
                eprintln!("Vosk: disabling");
                self.vosk = None;
            }
            (None, true) => {
                // Vosk was disabled, now enabled - initialize it
                let vosk_provider = VoskProvider::new(None);
                if vosk_provider.is_available() {
                    eprintln!("Vosk: enabling");
                    self.vosk = Some(vosk_provider);
                } else {
                    eprintln!("Warning: Vosk enabled in settings but unavailable");
                    self.vosk = None;
                }
            }
            _ => {
                // No change needed
            }
        }
        
        // Note: Whisper model path changes require calling initialize() with new config
        // We don't reload Whisper here because it requires TranscriptionConfig
    }
    
    /// Check if VAD detected speech in the audio
    /// 
    /// Returns:
    /// - Some(true) if speech detected
    /// - Some(false) if silence detected
    /// - None if VAD unavailable (assume speech present)
    fn check_vad(&mut self, audio: &[f32]) -> Option<bool> {
        if let Some(vad) = &mut self.vad {
            // Split audio into 512-sample chunks for VAD
            let chunk_size = vad.chunk_size();
            let mut speech_detected = false;
            
            for chunk in audio.chunks(chunk_size) {
                if chunk.len() < chunk_size {
                    // Skip incomplete chunks
                    continue;
                }
                
                match vad.contains_speech(chunk) {
                    Some(true) => {
                        speech_detected = true;
                        break; // Found speech, no need to check more chunks
                    }
                    Some(false) => {
                        // Continue checking other chunks
                    }
                    None => {
                        // VAD unavailable, assume speech
                        return None;
                    }
                }
            }
            
            Some(speech_detected)
        } else {
            // VAD not available, assume speech present
            None
        }
    }
    
    /// Process audio with Vosk for instant partials
    /// 
    /// Returns partial transcription segment if available.
    fn process_vosk(&mut self, audio: &[f32]) -> Option<TranscriptionSegment> {
        if let Some(vosk) = &mut self.vosk {
            // Convert f32 to i16 for Vosk
            let i16_samples: Vec<i16> = audio.iter()
                .map(|&sample| (sample * 32767.0).clamp(-32768.0, 32767.0) as i16)
                .collect();

            // Feed audio to Vosk
            vosk.accept_waveform(&i16_samples);

            // Get partial result for this window
            let partial = vosk.partial_result();

            // Reset Vosk by calling final_result() so next window starts fresh
            // Without this, Vosk accumulates all audio across windows
            let _ = vosk.final_result();

            if let Some(text) = partial {
                return Some(TranscriptionSegment {
                    text,
                    start_ms: 0,
                    end_ms: 0,
                    is_final: false,
                });
            }
        }

        None
    }
}

impl TranscriptionProvider for HybridProvider {
    fn name(&self) -> &str {
        "hybrid-vad-vosk-whisper"
    }
    
    fn initialize(&mut self, config: &TranscriptionConfig) -> Result<(), Box<dyn Error>> {
        // Initialize Whisper (required)
        self.whisper.initialize(config)?;
        
        eprintln!("HybridProvider initialized successfully");
        eprintln!("  - VAD: {}", if self.vad.is_some() { "enabled" } else { "disabled" });
        eprintln!("  - Vosk: {}", if self.vosk.is_some() { "enabled" } else { "disabled" });
        eprintln!("  - Whisper: enabled");
        
        Ok(())
    }
    
    fn transcribe(&mut self, audio: &[f32]) -> Result<Vec<TranscriptionSegment>, Box<dyn Error>> {
        let mut segments = Vec::new();

        // Diagnostic: compute audio RMS and peak levels
        let rms = (audio.iter().map(|&s| s * s).sum::<f32>() / audio.len() as f32).sqrt();
        let peak = audio.iter().map(|&s| s.abs()).fold(0.0f32, f32::max);
        eprintln!("HybridProvider: Audio stats - {} samples, RMS={:.6}, peak={:.6}", audio.len(), rms, peak);

        // Step 1: Check VAD if enabled
        let vad_result = self.check_vad(audio);
        match vad_result {
            Some(true) => {
                eprintln!("HybridProvider: VAD detected speech, transcribing");
            }
            Some(false) => {
                eprintln!("HybridProvider: VAD detected silence, skipping transcription");
                return Ok(segments); // Return empty segments for silence
            }
            None => {
                eprintln!("HybridProvider: VAD disabled, transcribing all audio");
            }
        }

        // Step 2: Process with Vosk for instant partials
        // Emit directly via AppHandle so the partial reaches the frontend
        // BEFORE Whisper blocks for 1-2 seconds
        if let Some(partial_segment) = self.process_vosk(audio) {
            eprintln!("HybridProvider: Vosk partial: \"{}\" (emitting directly)", partial_segment.text);
            if let Some(handle) = &self.app_handle {
                let _ = handle.emit("transcription-update", &partial_segment);
            }
        }

        // Step 3: Process with Whisper for accurate finals (blocks 1-2s)
        let whisper_start = std::time::Instant::now();
        let whisper_segments = self.whisper.transcribe(audio)?;
        let whisper_ms = whisper_start.elapsed().as_millis();
        eprintln!("HybridProvider: Whisper returned {} segments in {}ms", whisper_segments.len(), whisper_ms);
        for seg in &whisper_segments {
            eprintln!("  Whisper: \"{}\" ({}ms-{}ms)", seg.text, seg.start_ms, seg.end_ms);
        }
        segments.extend(whisper_segments);

        Ok(segments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn test_settings() -> TranscriptionSettings {
        TranscriptionSettings {
            vad_enabled: false,
            vosk_enabled: false,
            ..Default::default()
        }
    }

    fn test_provider(_settings: &TranscriptionSettings) -> HybridProvider {
        HybridProvider {
            vad: None,
            vosk: None,
            whisper: WhisperProvider::new(),
            app_handle: None,
        }
    }

    #[test]
    fn test_hybrid_provider_creation() {
        let settings = test_settings();
        let provider = test_provider(&settings);
        assert_eq!(provider.name(), "hybrid-vad-vosk-whisper");
    }

    #[test]
    fn test_hybrid_provider_transcribe_without_initialize() {
        let settings = test_settings();
        let mut provider = test_provider(&settings);

        // If VAD is available, silent audio gets gated → returns Ok(empty)
        let silence = vec![0.0f32; 48000]; // 3 seconds at 16kHz
        let result = provider.transcribe(&silence);
        if provider.vad.is_some() {
            // VAD detects silence and short-circuits → Ok(vec![])
            assert!(result.is_ok());
            assert!(result.unwrap().is_empty());
        } else {
            // Without VAD, silence reaches uninitialized Whisper → Err
            assert!(result.is_err());
        }
    }
    
    #[test]
    fn test_f32_to_i16_conversion() {
        let f32_samples = vec![0.0_f32, 0.5_f32, -0.5_f32, 1.0_f32, -1.0_f32];
        let i16_samples: Vec<i16> = f32_samples.iter()
            .map(|&sample| (sample * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();
        
        assert_eq!(i16_samples[0], 0);
        assert_eq!(i16_samples[1], 16383);
        assert_eq!(i16_samples[2], -16383);
        assert_eq!(i16_samples[3], 32767);
        assert_eq!(i16_samples[4], -32767);
    }
}
