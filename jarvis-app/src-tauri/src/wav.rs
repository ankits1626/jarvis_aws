use std::path::Path;
use crate::files::{SAMPLE_RATE, BYTES_PER_SAMPLE, CHANNELS};

/// Converts PCM audio files to WAV format for playback
pub struct WavConverter;

impl WavConverter {
    /// Convert a PCM file to WAV format by prepending a 44-byte WAV header
    /// 
    /// Reads the raw PCM data from the specified file path and prepends a
    /// standard WAV header with the correct audio parameters (16kHz, 16-bit, mono).
    /// 
    /// # Arguments
    /// 
    /// * `pcm_path` - Path to the PCM file to convert
    /// 
    /// # Returns
    /// 
    /// A `Vec<u8>` containing the complete WAV file (header + PCM data)
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - The PCM file cannot be read
    /// - The file is too large (> 4GB, WAV format limitation)
    /// 
    /// # WAV Format
    /// 
    /// The generated WAV file uses the following format:
    /// - Sample rate: 16000 Hz
    /// - Bits per sample: 16 (signed integer, little-endian)
    /// - Channels: 1 (mono)
    /// - Format: PCM (format code 1)
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use jarvis_app_lib::wav::WavConverter;
    /// use std::path::Path;
    /// 
    /// let wav_data = WavConverter::pcm_to_wav(Path::new("recording.pcm"))?;
    /// // wav_data now contains a complete WAV file ready for playback
    /// # Ok::<(), String>(())
    /// ```
    pub fn pcm_to_wav(pcm_path: &Path) -> Result<Vec<u8>, String> {
        // Read the PCM file
        let pcm_data = std::fs::read(pcm_path)
            .map_err(|e| format!("Failed to read PCM file {:?}: {}", pcm_path, e))?;
        
        Self::from_pcm_bytes(&pcm_data)
    }
    
    /// Convert PCM bytes to WAV format by prepending a 44-byte WAV header
    /// 
    /// Takes raw PCM data and prepends a standard WAV header with the correct
    /// audio parameters (16kHz, 16-bit, mono).
    /// 
    /// # Arguments
    /// 
    /// * `pcm_data` - Raw PCM audio data (16kHz, 16-bit, mono)
    /// 
    /// # Returns
    /// 
    /// A `Vec<u8>` containing the complete WAV file (header + PCM data)
    /// 
    /// # Errors
    /// 
    /// Returns an error if the PCM data is too large (> 4GB, WAV format limitation)
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use jarvis_app_lib::wav::WavConverter;
    /// 
    /// let pcm_data = vec![0u8; 32000]; // 1 second of audio
    /// let wav_data = WavConverter::from_pcm_bytes(&pcm_data)?;
    /// # Ok::<(), String>(())
    /// ```
    pub fn from_pcm_bytes(pcm_data: &[u8]) -> Result<Vec<u8>, String> {
        // Check if file size exceeds WAV format limit (4GB - 8 bytes for RIFF header)
        let data_size = pcm_data.len() as u64;
        if data_size > (u32::MAX as u64 - 36) {
            return Err(format!(
                "PCM data too large ({} bytes). WAV format supports maximum {} bytes",
                data_size,
                u32::MAX as u64 - 36
            ));
        }
        
        // Generate WAV header
        let header = Self::create_wav_header(pcm_data.len() as u32);
        
        // Concatenate header + PCM data
        let mut wav_data = Vec::with_capacity(header.len() + pcm_data.len());
        wav_data.extend_from_slice(&header);
        wav_data.extend_from_slice(pcm_data);
        
        Ok(wav_data)
    }
    
    /// Create a 44-byte WAV header for PCM audio data
    /// 
    /// Generates a standard WAV file header with the following structure:
    /// 
    /// ```text
    /// Bytes 0-3:   "RIFF" (chunk ID)
    /// Bytes 4-7:   File size - 8 (little-endian u32)
    /// Bytes 8-11:  "WAVE" (format)
    /// Bytes 12-15: "fmt " (subchunk1 ID)
    /// Bytes 16-19: 16 (subchunk1 size - PCM format)
    /// Bytes 20-21: 1 (audio format - PCM)
    /// Bytes 22-23: 1 (number of channels - mono)
    /// Bytes 24-27: 16000 (sample rate)
    /// Bytes 28-31: 32000 (byte rate = sample_rate * channels * bytes_per_sample)
    /// Bytes 32-33: 2 (block align = channels * bytes_per_sample)
    /// Bytes 34-35: 16 (bits per sample)
    /// Bytes 36-39: "data" (subchunk2 ID)
    /// Bytes 40-43: Data size (little-endian u32)
    /// ```
    /// 
    /// # Arguments
    /// 
    /// * `data_size` - Size of the PCM data in bytes
    /// 
    /// # Returns
    /// 
    /// A 44-byte array containing the WAV header
    fn create_wav_header(data_size: u32) -> [u8; 44] {
        let mut header = [0u8; 44];
        
        // RIFF chunk descriptor
        header[0..4].copy_from_slice(b"RIFF");
        
        // File size - 8 bytes (entire file size minus 8 bytes for RIFF header)
        let file_size = data_size + 36; // 36 = header size (44) - RIFF header (8)
        header[4..8].copy_from_slice(&file_size.to_le_bytes());
        
        // WAVE format
        header[8..12].copy_from_slice(b"WAVE");
        
        // fmt subchunk
        header[12..16].copy_from_slice(b"fmt ");
        
        // Subchunk1 size (16 for PCM)
        header[16..20].copy_from_slice(&16u32.to_le_bytes());
        
        // Audio format (1 = PCM)
        header[20..22].copy_from_slice(&1u16.to_le_bytes());
        
        // Number of channels (1 = mono)
        header[22..24].copy_from_slice(&(CHANNELS as u16).to_le_bytes());
        
        // Sample rate (16000 Hz)
        header[24..28].copy_from_slice(&SAMPLE_RATE.to_le_bytes());
        
        // Byte rate (sample_rate * channels * bytes_per_sample)
        let byte_rate = SAMPLE_RATE * CHANNELS * BYTES_PER_SAMPLE;
        header[28..32].copy_from_slice(&byte_rate.to_le_bytes());
        
        // Block align (channels * bytes_per_sample)
        let block_align = (CHANNELS * BYTES_PER_SAMPLE) as u16;
        header[32..34].copy_from_slice(&block_align.to_le_bytes());
        
        // Bits per sample (16-bit)
        let bits_per_sample = (BYTES_PER_SAMPLE * 8) as u16;
        header[34..36].copy_from_slice(&bits_per_sample.to_le_bytes());
        
        // data subchunk
        header[36..40].copy_from_slice(b"data");
        
        // Data size
        header[40..44].copy_from_slice(&data_size.to_le_bytes());
        
        header
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_create_wav_header_structure() {
        // Test with 32000 bytes of PCM data (1 second at 16kHz, 16-bit, mono)
        let data_size = 32000u32;
        let header = WavConverter::create_wav_header(data_size);
        
        // Verify header is 44 bytes
        assert_eq!(header.len(), 44);
        
        // Verify RIFF chunk ID
        assert_eq!(&header[0..4], b"RIFF");
        
        // Verify file size (data_size + 36)
        let file_size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
        assert_eq!(file_size, data_size + 36);
        
        // Verify WAVE format
        assert_eq!(&header[8..12], b"WAVE");
        
        // Verify fmt chunk ID
        assert_eq!(&header[12..16], b"fmt ");
        
        // Verify subchunk1 size (16 for PCM)
        let subchunk1_size = u32::from_le_bytes([header[16], header[17], header[18], header[19]]);
        assert_eq!(subchunk1_size, 16);
        
        // Verify audio format (1 = PCM)
        let audio_format = u16::from_le_bytes([header[20], header[21]]);
        assert_eq!(audio_format, 1);
        
        // Verify number of channels (1 = mono)
        let num_channels = u16::from_le_bytes([header[22], header[23]]);
        assert_eq!(num_channels, 1);
        
        // Verify sample rate (16000 Hz)
        let sample_rate = u32::from_le_bytes([header[24], header[25], header[26], header[27]]);
        assert_eq!(sample_rate, 16000);
        
        // Verify byte rate (16000 * 1 * 2 = 32000)
        let byte_rate = u32::from_le_bytes([header[28], header[29], header[30], header[31]]);
        assert_eq!(byte_rate, 32000);
        
        // Verify block align (1 * 2 = 2)
        let block_align = u16::from_le_bytes([header[32], header[33]]);
        assert_eq!(block_align, 2);
        
        // Verify bits per sample (16)
        let bits_per_sample = u16::from_le_bytes([header[34], header[35]]);
        assert_eq!(bits_per_sample, 16);
        
        // Verify data chunk ID
        assert_eq!(&header[36..40], b"data");
        
        // Verify data size
        let data_chunk_size = u32::from_le_bytes([header[40], header[41], header[42], header[43]]);
        assert_eq!(data_chunk_size, data_size);
    }

    #[test]
    fn test_pcm_to_wav_success() {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        // Create a unique temporary directory for testing
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("jarvis_test_wav_{}", timestamp));
        std::fs::create_dir_all(&temp_dir).unwrap();
        
        // Create a test PCM file with 32000 bytes (1 second of audio)
        let pcm_path = temp_dir.join("test.pcm");
        let pcm_data = vec![0u8; 32000];
        std::fs::File::create(&pcm_path).unwrap().write_all(&pcm_data).unwrap();
        
        // Convert to WAV
        let wav_data = WavConverter::pcm_to_wav(&pcm_path).unwrap();
        
        // Verify WAV data size (44 bytes header + 32000 bytes data)
        assert_eq!(wav_data.len(), 44 + 32000);
        
        // Verify header
        assert_eq!(&wav_data[0..4], b"RIFF");
        assert_eq!(&wav_data[8..12], b"WAVE");
        assert_eq!(&wav_data[12..16], b"fmt ");
        assert_eq!(&wav_data[36..40], b"data");
        
        // Verify PCM data is intact
        assert_eq!(&wav_data[44..], &pcm_data[..]);
        
        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_pcm_to_wav_file_not_found() {
        use std::path::PathBuf;
        
        // Try to convert a non-existent file
        let result = WavConverter::pcm_to_wav(&PathBuf::from("/nonexistent/file.pcm"));
        assert!(result.is_err());
        
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Failed to read PCM file"));
    }

    #[test]
    fn test_pcm_to_wav_empty_file() {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        // Create a unique temporary directory for testing
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("jarvis_test_wav_empty_{}", timestamp));
        std::fs::create_dir_all(&temp_dir).unwrap();
        
        // Create an empty PCM file
        let pcm_path = temp_dir.join("empty.pcm");
        std::fs::File::create(&pcm_path).unwrap();
        
        // Convert to WAV
        let wav_data = WavConverter::pcm_to_wav(&pcm_path).unwrap();
        
        // Verify WAV data size (44 bytes header + 0 bytes data)
        assert_eq!(wav_data.len(), 44);
        
        // Verify header with 0 data size
        let data_size = u32::from_le_bytes([wav_data[40], wav_data[41], wav_data[42], wav_data[43]]);
        assert_eq!(data_size, 0);
        
        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_pcm_to_wav_various_sizes() {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        // Test with various file sizes to ensure header is correct
        let test_sizes = vec![0, 100, 1000, 16000, 32000, 48000, 100000];
        
        for size in test_sizes {
            // Create a unique temporary directory for testing
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let temp_dir = std::env::temp_dir().join(format!("jarvis_test_wav_size_{}_{}", size, timestamp));
            std::fs::create_dir_all(&temp_dir).unwrap();
            
            // Create a test PCM file
            let pcm_path = temp_dir.join("test.pcm");
            let pcm_data = vec![0u8; size];
            std::fs::File::create(&pcm_path).unwrap().write_all(&pcm_data).unwrap();
            
            // Convert to WAV
            let wav_data = WavConverter::pcm_to_wav(&pcm_path).unwrap();
            
            // Verify WAV data size
            assert_eq!(wav_data.len(), 44 + size);
            
            // Verify data size in header
            let data_size = u32::from_le_bytes([wav_data[40], wav_data[41], wav_data[42], wav_data[43]]);
            assert_eq!(data_size, size as u32);
            
            // Verify file size in header
            let file_size = u32::from_le_bytes([wav_data[4], wav_data[5], wav_data[6], wav_data[7]]);
            assert_eq!(file_size, size as u32 + 36);
            
            // Cleanup
            std::fs::remove_dir_all(&temp_dir).ok();
        }
    }
}
