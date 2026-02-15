// Silero VAD v5 wrapper for voice activity detection
// Uses ort (ONNX Runtime) directly to load the real Silero VAD model (2.2MB)
// Processes 512-sample chunks (~32ms at 16kHz)
//
// Model architecture (silero_vad.onnx v5):
//   Inputs:  input (float[1, 512]), state (float[2, 1, 128]), sr (int64)
//   Outputs: output (float[1, 1] speech probability), stateN (float[2, 1, 128])

use std::path::PathBuf;
use ort::session::Session;
use ort::value::Tensor;

/// Voice Activity Detection using Silero VAD v5
///
/// Detects speech presence in audio chunks to gate the transcription pipeline.
/// Uses the real Silero VAD ONNX model (~2.2MB) with direct ort inference.
/// Gracefully degrades if model is unavailable (returns None).
pub struct SileroVad {
    session: Option<Session>,
    /// LSTM hidden state: float[2, 1, 128]
    state: Vec<f32>,
    /// Whether VAD is actually available
    available: bool,
    /// Chunk size for VAD processing (512 samples = 32ms at 16kHz)
    chunk_size: usize,
    /// Speech detection threshold (0.0 to 1.0)
    threshold: f32,
}

impl SileroVad {
    /// VAD chunk size (512 samples = 32ms at 16kHz)
    const CHUNK_SIZE: usize = 512;

    /// LSTM state size per direction per batch
    const STATE_DIM: usize = 128;

    /// Default speech detection threshold
    const THRESHOLD: f32 = 0.5;

    /// Create a new SileroVad instance
    ///
    /// Loads the Silero VAD v5 ONNX model from the configured path.
    /// Download from: https://github.com/snakers4/silero-vad/raw/master/src/silero_vad/data/silero_vad.onnx
    /// If loading fails, gracefully degrades (available=false).
    pub fn new(model_path: Option<PathBuf>) -> Self {
        let path = model_path.unwrap_or_else(|| {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            home.join(".jarvis/models/silero_vad.onnx")
        });

        if !path.exists() {
            eprintln!(
                "Warning: Silero VAD model not found at {:?}. VAD disabled - will process all audio.",
                path
            );
            return Self::unavailable();
        }

        match Session::builder()
            .and_then(|b| b.with_intra_threads(1))
            .and_then(|b| b.commit_from_file(&path))
        {
            Ok(session) => {
                eprintln!("Silero VAD loaded successfully from {:?} (2.2MB model)", path);
                Self {
                    session: Some(session),
                    state: vec![0.0f32; 2 * 1 * Self::STATE_DIM], // [2, 1, 128]
                    available: true,
                    chunk_size: Self::CHUNK_SIZE,
                    threshold: Self::THRESHOLD,
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to load Silero VAD model: {}. Will process all audio.",
                    e
                );
                Self::unavailable()
            }
        }
    }

    fn unavailable() -> Self {
        Self {
            session: None,
            state: Vec::new(),
            available: false,
            chunk_size: Self::CHUNK_SIZE,
            threshold: Self::THRESHOLD,
        }
    }

    /// Check if speech is present in the given audio samples
    ///
    /// Processes audio in 512-sample chunks and checks speech probability.
    /// Returns:
    /// - `Some(true)` if any chunk has speech probability >= threshold
    /// - `Some(false)` if all chunks are below threshold (silence)
    /// - `None` if VAD is unavailable (graceful degradation)
    pub fn contains_speech(&mut self, samples: &[f32]) -> Option<bool> {
        if self.session.is_none() {
            return None;
        }

        let mut offset = 0;
        while offset + Self::CHUNK_SIZE <= samples.len() {
            let chunk = &samples[offset..offset + Self::CHUNK_SIZE];

            match self.process_chunk(chunk) {
                Ok(prob) => {
                    if prob >= self.threshold {
                        return Some(true);
                    }
                }
                Err(e) => {
                    eprintln!("Warning: VAD processing failed: {}. Assuming speech present.", e);
                    return Some(true); // Conservative: assume speech on error
                }
            }

            offset += Self::CHUNK_SIZE;
        }

        Some(false)
    }

    /// Process a single 512-sample chunk through the ONNX model
    fn process_chunk(&mut self, chunk: &[f32]) -> Result<f32, String> {
        let session = self.session.as_ref()
            .ok_or_else(|| "VAD session not available".to_string())?;
        // Prepare input tensor: float[1, 512]
        let input_tensor = Tensor::from_array(([1_usize, Self::CHUNK_SIZE], chunk.to_vec()))
            .map_err(|e| format!("Failed to create input tensor: {}", e))?;

        // Prepare state tensor: float[2, 1, 128]
        let state_tensor = Tensor::from_array(([2_usize, 1, Self::STATE_DIM], self.state.clone()))
            .map_err(|e| format!("Failed to create state tensor: {}", e))?;

        // Prepare sample rate tensor: int64 scalar
        let sr_tensor = Tensor::from_array(([1_usize], vec![16000_i64]))
            .map_err(|e| format!("Failed to create sr tensor: {}", e))?;

        // Run inference
        let inputs = vec![
            ("input", input_tensor.into_dyn()),
            ("state", state_tensor.into_dyn()),
            ("sr", sr_tensor.into_dyn()),
        ];

        let outputs = session
            .run(inputs)
            .map_err(|e| format!("VAD inference failed: {}", e))?;

        // Extract speech probability from output[0]: float[1, 1]
        let output = outputs[0]
            .try_extract_raw_tensor::<f32>()
            .map_err(|e| format!("Failed to extract output: {}", e))?;
        let (_shape, data) = output;
        let prob = data.first().copied().unwrap_or(0.0);

        // Update LSTM state from output[1]: float[2, 1, 128]
        let (_state_shape, state_data) = outputs[1]
            .try_extract_raw_tensor::<f32>()
            .map_err(|e| format!("Failed to extract state: {}", e))?;
        self.state = state_data.to_vec();

        Ok(prob)
    }

    /// Get the chunk size used by VAD (512 samples)
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    /// Check if VAD is available
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Reset the VAD state (call between recordings)
    pub fn reset(&mut self) {
        if self.available {
            self.state = vec![0.0f32; 2 * 1 * Self::STATE_DIM];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_creation() {
        let vad = SileroVad::new(None);
        assert_eq!(vad.chunk_size(), 512);
    }

    #[test]
    fn test_vad_graceful_degradation_missing_model() {
        let mut vad = SileroVad::new(Some(PathBuf::from("/nonexistent/path/model.onnx")));
        let samples = vec![0.0f32; 512];

        assert!(!vad.is_available());
        assert_eq!(vad.contains_speech(&samples), None);
    }

    #[test]
    fn test_vad_chunk_size() {
        // Property 12: VAD Chunk Size
        // Validates: Requirements 4.1
        let vad = SileroVad::new(None);
        assert_eq!(vad.chunk_size(), 512, "VAD must process 512-sample chunks");
    }

    #[test]
    fn test_vad_silence_detection() {
        let mut vad = SileroVad::new(None);
        if !vad.is_available() {
            eprintln!("Skipping silence detection test - VAD model not available");
            return;
        }

        // Feed silence (all zeros) - should detect no speech
        let silence = vec![0.0f32; 512];
        let result = vad.contains_speech(&silence);
        assert_eq!(result, Some(false), "VAD should detect silence in zero audio");
    }
}
