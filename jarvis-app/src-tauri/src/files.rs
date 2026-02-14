use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Audio format constants for PCM recordings
pub const SAMPLE_RATE: u32 = 16000;
pub const BYTES_PER_SAMPLE: u32 = 2; // s16le (16-bit signed integer)
pub const CHANNELS: u32 = 1; // mono

/// Metadata for a single recording
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecordingMetadata {
    /// Filename of the recording (e.g., "20240315_143022.pcm")
    pub filename: String,
    
    /// Size of the recording file in bytes
    pub size_bytes: u64,
    
    /// Unix timestamp (seconds since epoch) when the recording was created
    pub created_at: u64,
    
    /// Duration of the recording in seconds, calculated from file size
    pub duration_seconds: f64,
}

/// Application configuration
#[derive(Serialize, Deserialize, Debug)]
pub struct AppConfig {
    /// Directory where recordings are stored
    pub recordings_dir: PathBuf,
    
    /// Name of the sidecar binary for the current platform
    pub sidecar_name: String,
    
    /// Sample rate for audio capture (Hz)
    pub sample_rate: u32,
    
    /// Bytes per audio sample
    pub bytes_per_sample: u32,
    
    /// Number of audio channels
    pub channels: u32,
}

/// Manages recording file storage and operations
pub struct FileManager {
    recordings_dir: PathBuf,
}

impl FileManager {
    /// Create a new FileManager instance
    /// 
    /// Gets the platform-specific app data directory and creates a "recordings"
    /// subdirectory if it doesn't exist.
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - The app data directory cannot be determined
    /// - The recordings directory cannot be created
    pub fn new() -> Result<Self, String> {
        // Get platform-specific app data directory
        let app_data_dir = dirs::data_dir()
            .ok_or_else(|| "Failed to determine app data directory".to_string())?;
        
        // Create recordings subdirectory path
        let recordings_dir = app_data_dir.join("com.jarvis.app").join("recordings");
        
        // Create the directory if it doesn't exist
        std::fs::create_dir_all(&recordings_dir)
            .map_err(|e| format!("Failed to create recordings directory: {}", e))?;
        
        Ok(Self { recordings_dir })
    }
    
    /// Get the path to the recordings directory
    pub fn get_recordings_dir(&self) -> &std::path::Path {
        &self.recordings_dir
    }
    
    /// Calculate the duration of a recording from its file size
    /// 
    /// Uses the formula: duration = size_bytes / (sample_rate * bytes_per_sample * channels)
    /// 
    /// # Arguments
    /// 
    /// * `size_bytes` - The size of the PCM file in bytes
    /// 
    /// # Returns
    /// 
    /// The duration in seconds as a floating-point number
    /// 
    /// # Examples
    /// 
    /// ```
    /// use jarvis_app_lib::files::FileManager;
    /// 
    /// // A 32,000 byte file at 16kHz, 16-bit, mono = 1 second
    /// let duration = FileManager::calculate_duration(32000);
    /// assert_eq!(duration, 1.0);
    /// ```
    pub fn calculate_duration(size_bytes: u64) -> f64 {
        size_bytes as f64 / (SAMPLE_RATE * BYTES_PER_SAMPLE * CHANNELS) as f64
    }
    
    /// List all recordings in the recordings directory
    /// 
    /// Reads all .pcm files from the recordings directory, extracts metadata
    /// (filename, size, creation timestamp, calculated duration), and returns
    /// them sorted by creation date in descending order (newest first).
    /// 
    /// # Returns
    /// 
    /// A vector of `RecordingMetadata` sorted by `created_at` descending
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - The recordings directory cannot be read
    /// - File metadata cannot be accessed
    pub fn list_recordings(&self) -> Result<Vec<RecordingMetadata>, String> {
        let mut recordings = Vec::new();
        
        // Read directory entries
        let entries = std::fs::read_dir(&self.recordings_dir)
            .map_err(|e| format!("Failed to read recordings directory: {}", e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            
            // Only process .pcm files
            if path.extension().and_then(|s| s.to_str()) != Some("pcm") {
                continue;
            }
            
            // Get file metadata
            let metadata = std::fs::metadata(&path)
                .map_err(|e| format!("Failed to read metadata for {:?}: {}", path, e))?;
            
            // Get filename
            let filename = path
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| format!("Invalid filename: {:?}", path))?
                .to_string();
            
            // Get file size
            let size_bytes = metadata.len();
            
            // Get creation timestamp (Unix timestamp in seconds)
            let created_at = metadata
                .created()
                .map_err(|e| format!("Failed to get creation time for {:?}: {}", path, e))?
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| format!("Invalid creation time for {:?}: {}", path, e))?
                .as_secs();
            
            // Calculate duration
            let duration_seconds = Self::calculate_duration(size_bytes);
            
            recordings.push(RecordingMetadata {
                filename,
                size_bytes,
                created_at,
                duration_seconds,
            });
        }
        
        // Sort by created_at descending (newest first)
        recordings.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        Ok(recordings)
    }
    
    /// Delete a recording by filename
    /// 
    /// Validates the filename to prevent path traversal attacks, then deletes
    /// the PCM file from the recordings directory.
    /// 
    /// # Arguments
    /// 
    /// * `filename` - The name of the recording file to delete (e.g., "20240315_143022.pcm")
    /// 
    /// # Returns
    /// 
    /// `Ok(())` if the file was successfully deleted
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - The filename contains path traversal characters (e.g., "..", "/")
    /// - The filename is empty
    /// - The file does not exist
    /// - The file cannot be deleted (permission denied, etc.)
    /// 
    /// # Security
    /// 
    /// This method validates the filename to ensure it doesn't contain path
    /// separators or parent directory references, preventing path traversal attacks.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use jarvis_app_lib::files::FileManager;
    /// 
    /// let file_manager = FileManager::new().unwrap();
    /// 
    /// // Valid filename
    /// file_manager.delete_recording("20240315_143022.pcm")?;
    /// 
    /// // Invalid filename (path traversal attempt)
    /// assert!(file_manager.delete_recording("../../../etc/passwd").is_err());
    /// # Ok::<(), String>(())
    /// ```
    pub fn delete_recording(&self, filename: &str) -> Result<(), String> {
        // Validate filename is not empty
        if filename.is_empty() {
            return Err("Filename cannot be empty".to_string());
        }
        
        // Validate filename doesn't contain path separators or parent directory references
        // This prevents path traversal attacks like "../../../etc/passwd"
        if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
            return Err(format!(
                "Invalid filename '{}': path traversal not allowed",
                filename
            ));
        }
        
        // Construct the full path
        let file_path = self.recordings_dir.join(filename);
        
        // Verify the file exists before attempting deletion
        if !file_path.exists() {
            return Err(format!(
                "Recording '{}' not found in recordings directory",
                filename
            ));
        }
        
        // Verify it's actually a file (not a directory)
        if !file_path.is_file() {
            return Err(format!(
                "Path '{}' is not a file",
                filename
            ));
        }
        
        // Delete the file
        std::fs::remove_file(&file_path).map_err(|e| {
            format!(
                "Failed to delete recording '{}' at {:?}: {}",
                filename, file_path, e
            )
        })?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_duration_one_second() {
        // 16000 Hz * 2 bytes * 1 channel = 32000 bytes per second
        let size_bytes = 32000;
        let duration = FileManager::calculate_duration(size_bytes);
        assert_eq!(duration, 1.0);
    }

    #[test]
    fn test_calculate_duration_half_second() {
        // Half a second = 16000 bytes
        let size_bytes = 16000;
        let duration = FileManager::calculate_duration(size_bytes);
        assert_eq!(duration, 0.5);
    }

    #[test]
    fn test_calculate_duration_zero() {
        // Empty file = 0 duration
        let size_bytes = 0;
        let duration = FileManager::calculate_duration(size_bytes);
        assert_eq!(duration, 0.0);
    }

    #[test]
    fn test_calculate_duration_fractional() {
        // 48000 bytes = 1.5 seconds
        let size_bytes = 48000;
        let duration = FileManager::calculate_duration(size_bytes);
        assert_eq!(duration, 1.5);
    }

    #[test]
    fn test_list_recordings_empty_directory() {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        // Create a unique temporary directory for testing
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("jarvis_test_empty_{}", timestamp));
        std::fs::create_dir_all(&temp_dir).unwrap();
        
        // Create a FileManager with the temp directory
        let file_manager = FileManager {
            recordings_dir: temp_dir.clone(),
        };
        
        // List recordings should return empty vector
        let recordings = file_manager.list_recordings().unwrap();
        assert_eq!(recordings.len(), 0);
        
        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_list_recordings_with_files() {
        use std::io::Write;
        use std::time::{SystemTime, UNIX_EPOCH};
        
        // Create a unique temporary directory for testing
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("jarvis_test_files_{}", timestamp));
        std::fs::create_dir_all(&temp_dir).unwrap();
        
        // Create test PCM files with different sizes
        let file1_path = temp_dir.join("20240315_143022.pcm");
        let file2_path = temp_dir.join("20240315_143023.pcm");
        let file3_path = temp_dir.join("20240315_143024.pcm");
        
        // Write some data to the files with delays to ensure different timestamps
        std::fs::File::create(&file1_path).unwrap().write_all(&vec![0u8; 32000]).unwrap(); // 1 second
        std::thread::sleep(std::time::Duration::from_millis(100)); // Longer delay for filesystem timestamp resolution
        std::fs::File::create(&file2_path).unwrap().write_all(&vec![0u8; 16000]).unwrap(); // 0.5 seconds
        std::thread::sleep(std::time::Duration::from_millis(100));
        std::fs::File::create(&file3_path).unwrap().write_all(&vec![0u8; 48000]).unwrap(); // 1.5 seconds
        
        // Create a FileManager with the temp directory
        let file_manager = FileManager {
            recordings_dir: temp_dir.clone(),
        };
        
        // List recordings
        let recordings = file_manager.list_recordings().unwrap();
        
        // Should have 3 recordings
        assert_eq!(recordings.len(), 3);
        
        // Verify they are sorted by created_at descending (newest first)
        // The newest file should be first
        assert!(recordings[0].created_at >= recordings[1].created_at);
        assert!(recordings[1].created_at >= recordings[2].created_at);
        
        // Verify all files are present (order may vary based on filesystem)
        let filenames: Vec<&str> = recordings.iter().map(|r| r.filename.as_str()).collect();
        assert!(filenames.contains(&"20240315_143022.pcm"));
        assert!(filenames.contains(&"20240315_143023.pcm"));
        assert!(filenames.contains(&"20240315_143024.pcm"));
        
        // Verify metadata for each file
        for recording in &recordings {
            match recording.filename.as_str() {
                "20240315_143022.pcm" => {
                    assert_eq!(recording.size_bytes, 32000);
                    assert_eq!(recording.duration_seconds, 1.0);
                }
                "20240315_143023.pcm" => {
                    assert_eq!(recording.size_bytes, 16000);
                    assert_eq!(recording.duration_seconds, 0.5);
                }
                "20240315_143024.pcm" => {
                    assert_eq!(recording.size_bytes, 48000);
                    assert_eq!(recording.duration_seconds, 1.5);
                }
                _ => panic!("Unexpected filename: {}", recording.filename),
            }
        }
        
        // Verify all have valid timestamps
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        for recording in &recordings {
            assert!(recording.created_at > 0);
            assert!(recording.created_at <= now);
        }
        
        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_list_recordings_ignores_non_pcm_files() {
        use std::io::Write;
        use std::time::{SystemTime, UNIX_EPOCH};
        
        // Create a unique temporary directory for testing
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("jarvis_test_ignore_{}", timestamp));
        std::fs::create_dir_all(&temp_dir).unwrap();
        
        // Create a PCM file and some non-PCM files
        let pcm_file = temp_dir.join("recording.pcm");
        let txt_file = temp_dir.join("readme.txt");
        let wav_file = temp_dir.join("audio.wav");
        
        std::fs::File::create(&pcm_file).unwrap().write_all(&vec![0u8; 32000]).unwrap();
        std::fs::File::create(&txt_file).unwrap().write_all(b"test").unwrap();
        std::fs::File::create(&wav_file).unwrap().write_all(&vec![0u8; 1000]).unwrap();
        
        // Create a FileManager with the temp directory
        let file_manager = FileManager {
            recordings_dir: temp_dir.clone(),
        };
        
        // List recordings should only return the PCM file
        let recordings = file_manager.list_recordings().unwrap();
        assert_eq!(recordings.len(), 1);
        assert_eq!(recordings[0].filename, "recording.pcm");
        
        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_delete_recording_success() {
        use std::io::Write;
        use std::time::{SystemTime, UNIX_EPOCH};
        
        // Create a unique temporary directory for testing
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("jarvis_test_delete_{}", timestamp));
        std::fs::create_dir_all(&temp_dir).unwrap();
        
        // Create a test PCM file
        let file_path = temp_dir.join("test_recording.pcm");
        std::fs::File::create(&file_path).unwrap().write_all(&vec![0u8; 32000]).unwrap();
        
        // Verify file exists
        assert!(file_path.exists());
        
        // Create a FileManager with the temp directory
        let file_manager = FileManager {
            recordings_dir: temp_dir.clone(),
        };
        
        // Delete the recording
        let result = file_manager.delete_recording("test_recording.pcm");
        assert!(result.is_ok());
        
        // Verify file no longer exists
        assert!(!file_path.exists());
        
        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_delete_recording_file_not_found() {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        // Create a unique temporary directory for testing
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("jarvis_test_delete_notfound_{}", timestamp));
        std::fs::create_dir_all(&temp_dir).unwrap();
        
        // Create a FileManager with the temp directory
        let file_manager = FileManager {
            recordings_dir: temp_dir.clone(),
        };
        
        // Try to delete a non-existent file
        let result = file_manager.delete_recording("nonexistent.pcm");
        assert!(result.is_err());
        
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("not found"));
        
        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_delete_recording_path_traversal_slash() {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        // Create a unique temporary directory for testing
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("jarvis_test_delete_traversal_{}", timestamp));
        std::fs::create_dir_all(&temp_dir).unwrap();
        
        // Create a FileManager with the temp directory
        let file_manager = FileManager {
            recordings_dir: temp_dir.clone(),
        };
        
        // Try to delete with path traversal using forward slash
        let result = file_manager.delete_recording("../../../etc/passwd");
        assert!(result.is_err());
        
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("path traversal not allowed"));
        
        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_delete_recording_path_traversal_backslash() {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        // Create a unique temporary directory for testing
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("jarvis_test_delete_backslash_{}", timestamp));
        std::fs::create_dir_all(&temp_dir).unwrap();
        
        // Create a FileManager with the temp directory
        let file_manager = FileManager {
            recordings_dir: temp_dir.clone(),
        };
        
        // Try to delete with path traversal using backslash (Windows-style)
        let result = file_manager.delete_recording("..\\..\\..\\windows\\system32\\config\\sam");
        assert!(result.is_err());
        
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("path traversal not allowed"));
        
        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_delete_recording_path_traversal_parent_dir() {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        // Create a unique temporary directory for testing
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("jarvis_test_delete_parent_{}", timestamp));
        std::fs::create_dir_all(&temp_dir).unwrap();
        
        // Create a FileManager with the temp directory
        let file_manager = FileManager {
            recordings_dir: temp_dir.clone(),
        };
        
        // Try to delete with parent directory reference
        let result = file_manager.delete_recording("..file.pcm");
        assert!(result.is_err());
        
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("path traversal not allowed"));
        
        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_delete_recording_empty_filename() {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        // Create a unique temporary directory for testing
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("jarvis_test_delete_empty_{}", timestamp));
        std::fs::create_dir_all(&temp_dir).unwrap();
        
        // Create a FileManager with the temp directory
        let file_manager = FileManager {
            recordings_dir: temp_dir.clone(),
        };
        
        // Try to delete with empty filename
        let result = file_manager.delete_recording("");
        assert!(result.is_err());
        
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("cannot be empty"));
        
        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_delete_recording_directory_not_file() {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        // Create a unique temporary directory for testing
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("jarvis_test_delete_dir_{}", timestamp));
        std::fs::create_dir_all(&temp_dir).unwrap();
        
        // Create a subdirectory (not a file)
        let subdir = temp_dir.join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        
        // Create a FileManager with the temp directory
        let file_manager = FileManager {
            recordings_dir: temp_dir.clone(),
        };
        
        // Try to delete a directory
        let result = file_manager.delete_recording("subdir");
        assert!(result.is_err());
        
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("not a file"));
        
        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
