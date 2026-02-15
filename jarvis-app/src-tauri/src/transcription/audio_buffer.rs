/// AudioBuffer manages audio accumulation and windowing for batch transcription.
/// 
/// It accumulates PCM bytes into fixed-duration windows with sliding overlap,
/// converting s16le bytes → i16 samples → f32 samples for transcription engines.
/// 
/// # Audio Format Expectations (Property 30)
/// - Sample rate: 16kHz
/// - Format: s16le (16-bit signed integer, little-endian)
/// - Channels: mono (single channel)
/// - Bytes per second: 32,000 (16000 samples/sec × 2 bytes/sample)

pub struct AudioBuffer {
    /// Accumulated PCM bytes (s16le format)
    buffer: Vec<u8>,
    
    /// Window size in bytes
    window_size_bytes: usize,
    
    /// Advance size in bytes (window_size - overlap)
    advance_size_bytes: usize,
    
    /// Sample rate in Hz
    sample_rate: usize,
}

impl AudioBuffer {
    /// Creates a new AudioBuffer with the specified window and overlap durations.
    /// 
    /// # Arguments
    /// * `window_duration_secs` - Duration of each window in seconds
    /// * `overlap_duration_secs` - Duration of overlap between windows in seconds
    /// * `sample_rate` - Sample rate in Hz (typically 16000)
    /// 
    /// # Audio Format
    /// Expects 16kHz s16le mono audio:
    /// - Bytes per second = sample_rate × 2 (s16le = 2 bytes per sample)
    /// - Window size = window_duration × bytes_per_second
    /// - Advance size = (window_duration - overlap_duration) × bytes_per_second
    pub fn new(window_duration_secs: f32, overlap_duration_secs: f32, sample_rate: usize) -> Self {
        let bytes_per_second = sample_rate * 2; // 2 bytes per sample (s16le)
        let window_size_bytes = (window_duration_secs * bytes_per_second as f32) as usize;
        let advance_duration_secs = window_duration_secs - overlap_duration_secs;
        let advance_size_bytes = (advance_duration_secs * bytes_per_second as f32) as usize;
        
        Self {
            buffer: Vec::new(),
            window_size_bytes,
            advance_size_bytes,
            sample_rate,
        }
    }
    
    /// Pushes a chunk of PCM bytes into the buffer.
    /// 
    /// # Arguments
    /// * `chunk` - PCM bytes in s16le format
    pub fn push(&mut self, chunk: &[u8]) {
        self.buffer.extend_from_slice(chunk);
    }
    
    /// Pushes f32 audio samples into the buffer.
    /// 
    /// Converts f32 samples to s16le bytes before storing.
    /// This is used by HybridProvider which receives f32 audio.
    /// 
    /// # Arguments
    /// * `samples` - f32 audio samples in range [-1.0, 1.0]
    pub fn push_f32(&mut self, samples: &[f32]) {
        // Convert f32 to i16
        let i16_samples: Vec<i16> = samples
            .iter()
            .map(|&sample| (sample * 32768.0).clamp(-32768.0, 32767.0) as i16)
            .collect();
        
        // Convert i16 to s16le bytes
        for sample in i16_samples {
            self.buffer.extend_from_slice(&sample.to_le_bytes());
        }
    }
    
    /// Extracts a window of audio if enough data is available.
    /// 
    /// Returns `Some(Vec<f32>)` if the buffer contains at least `window_size_bytes`,
    /// otherwise returns `None`.
    /// 
    /// After extraction, the buffer is advanced by `advance_size_bytes`, keeping
    /// the overlap portion for the next window.
    /// 
    /// # Conversion Pipeline
    /// s16le bytes → i16 samples → f32 samples
    pub fn extract_window(&mut self) -> Option<Vec<f32>> {
        if self.buffer.len() < self.window_size_bytes {
            return None;
        }
        
        // Extract window bytes
        let window_bytes = &self.buffer[..self.window_size_bytes];
        
        // Convert s16le bytes to i16 samples
        let samples_i16: Vec<i16> = window_bytes
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        
        // Convert i16 to f32 (normalize to [-1.0, 1.0])
        let samples_f32: Vec<f32> = samples_i16
            .iter()
            .map(|&sample| sample as f32 / 32768.0)
            .collect();
        
        // Advance buffer (remove processed bytes, keep overlap)
        self.buffer.drain(..self.advance_size_bytes);
        
        Some(samples_f32)
    }
    
    /// Drains remaining audio from the buffer if it meets the minimum duration.
    /// 
    /// # Arguments
    /// * `min_duration_secs` - Minimum duration in seconds (typically 1.0)
    /// 
    /// Returns `Some(Vec<f32>)` if the buffer contains at least `min_duration_secs`
    /// of audio, otherwise returns `None` and clears the buffer.
    /// 
    /// This is used during recording stop to transcribe the final partial window.
    pub fn drain_remaining(&mut self, min_duration_secs: f32) -> Option<Vec<f32>> {
        let min_bytes = (min_duration_secs * (self.sample_rate * 2) as f32) as usize;
        
        if self.buffer.len() < min_bytes {
            self.buffer.clear();
            return None;
        }
        
        // Convert remaining bytes to f32
        let samples_i16: Vec<i16> = self.buffer
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        
        let samples_f32: Vec<f32> = samples_i16
            .iter()
            .map(|&sample| sample as f32 / 32768.0)
            .collect();
        
        self.buffer.clear();
        Some(samples_f32)
    }
    
    /// Returns the current buffer size in bytes
    pub fn len(&self) -> usize {
        self.buffer.len()
    }
    
    /// Returns true if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_audio_buffer_creation() {
        let buffer = AudioBuffer::new(3.0, 0.5, 16000);
        
        // Property 31: Bytes Per Second Calculation
        // 16kHz s16le mono = 16000 samples/sec × 2 bytes/sample = 32000 bytes/sec
        let expected_bytes_per_second = 16000 * 2;
        
        // Property 32: Bytes Per Window Calculation
        // 3 seconds × 32000 bytes/sec = 96000 bytes
        let expected_window_size = (3.0 * expected_bytes_per_second as f32) as usize;
        assert_eq!(buffer.window_size_bytes, expected_window_size);
        assert_eq!(buffer.window_size_bytes, 96000);
        
        // Advance size = (3.0 - 0.5) × 32000 = 80000 bytes
        let expected_advance_size = (2.5 * expected_bytes_per_second as f32) as usize;
        assert_eq!(buffer.advance_size_bytes, expected_advance_size);
        assert_eq!(buffer.advance_size_bytes, 80000);
    }
    
    #[test]
    fn test_audio_format_expectations() {
        // Property 30: Audio Format Expectations
        // Verify AudioBuffer expects 16kHz s16le mono
        let buffer = AudioBuffer::new(1.0, 0.0, 16000);
        
        // 1 second at 16kHz s16le mono = 32000 bytes
        assert_eq!(buffer.window_size_bytes, 32000);
        
        // Sample rate stored correctly
        assert_eq!(buffer.sample_rate, 16000);
    }
    
    #[test]
    fn test_push_and_extract_window() {
        let mut buffer = AudioBuffer::new(3.0, 0.5, 16000);
        
        // Create 96000 bytes of test data (3 seconds)
        let test_data = vec![0u8; 96000];
        buffer.push(&test_data);
        
        assert_eq!(buffer.len(), 96000);
        
        // Extract window should succeed
        let window = buffer.extract_window();
        assert!(window.is_some());
        
        // Buffer should be advanced by 80000 bytes (2.5 seconds)
        // Remaining: 96000 - 80000 = 16000 bytes (0.5 seconds overlap)
        assert_eq!(buffer.len(), 16000);
    }
    
    #[test]
    fn test_extract_window_underflow() {
        let mut buffer = AudioBuffer::new(3.0, 0.5, 16000);
        
        // Push less than window size
        let test_data = vec![0u8; 50000]; // Less than 96000
        buffer.push(&test_data);
        
        // Extract should return None
        let window = buffer.extract_window();
        assert!(window.is_none());
        
        // Buffer should remain unchanged
        assert_eq!(buffer.len(), 50000);
    }
    
    #[test]
    fn test_drain_remaining_above_threshold() {
        let mut buffer = AudioBuffer::new(3.0, 0.5, 16000);
        
        // Push 1.5 seconds of data (48000 bytes)
        let test_data = vec![0u8; 48000];
        buffer.push(&test_data);
        
        // Drain with 1.0 second threshold should succeed
        let remaining = buffer.drain_remaining(1.0);
        assert!(remaining.is_some());
        
        // Buffer should be empty
        assert!(buffer.is_empty());
    }
    
    #[test]
    fn test_drain_remaining_below_threshold() {
        let mut buffer = AudioBuffer::new(3.0, 0.5, 16000);
        
        // Push 0.5 seconds of data (16000 bytes)
        let test_data = vec![0u8; 16000];
        buffer.push(&test_data);
        
        // Drain with 1.0 second threshold should fail
        let remaining = buffer.drain_remaining(1.0);
        assert!(remaining.is_none());
        
        // Buffer should be cleared
        assert!(buffer.is_empty());
    }
    
    #[test]
    fn test_s16le_to_f32_conversion() {
        let mut buffer = AudioBuffer::new(0.1, 0.0, 16000);
        
        // Create test data: max positive, zero, max negative
        let mut test_data = Vec::new();
        test_data.extend_from_slice(&32767i16.to_le_bytes()); // Max positive
        test_data.extend_from_slice(&0i16.to_le_bytes());     // Zero
        test_data.extend_from_slice(&(-32768i16).to_le_bytes()); // Max negative
        
        // Pad to window size (0.1 sec = 3200 bytes)
        test_data.resize(3200, 0);
        
        buffer.push(&test_data);
        let window = buffer.extract_window().unwrap();
        
        // Check first three samples
        assert!((window[0] - 1.0).abs() < 0.001);  // 32767/32768 ≈ 1.0
        assert!((window[1] - 0.0).abs() < 0.001);  // 0/32768 = 0.0
        assert!((window[2] - (-1.0)).abs() < 0.001); // -32768/32768 = -1.0
    }
}
