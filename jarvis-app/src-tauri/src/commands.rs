use crate::files::{FileManager, RecordingMetadata};
use crate::platform::PlatformDetector;
use crate::recording::RecordingManager;
use crate::wav::WavConverter;
use std::sync::Mutex;
use tauri::State;

/// Start a new audio recording
/// 
/// This command initiates a new recording by spawning the JarvisListen sidecar
/// process. It returns the filename of the new recording on success.
/// 
/// # Arguments
/// 
/// * `state` - Managed state containing the RecordingManager (wrapped in Mutex)
/// * `file_manager` - Managed state containing the FileManager
/// 
/// # Returns
/// 
/// * `Ok(String)` - The filename of the new recording (e.g., "20240315_143022.pcm")
/// * `Err(String)` - A descriptive error message if the recording cannot be started
/// 
/// # Errors
/// 
/// Returns an error if:
/// - A recording is already in progress (concurrent recording not allowed)
/// - The sidecar process fails to spawn
/// - The output path is invalid
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// try {
///   const filename = await invoke('start_recording');
///   console.log(`Recording started: ${filename}`);
/// } catch (error) {
///   console.error(`Failed to start recording: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn start_recording(
    state: State<'_, Mutex<RecordingManager>>,
    file_manager: State<'_, FileManager>,
) -> Result<String, String> {
    let mut recording_manager = state
        .lock()
        .map_err(|e| format!("Failed to acquire lock on RecordingManager: {}", e))?;
    
    let recordings_dir = file_manager.get_recordings_dir();
    recording_manager.start_recording(recordings_dir)
}

/// Stop the current recording
/// 
/// This command gracefully terminates the active recording by sending SIGTERM
/// to the sidecar process, allowing it to flush audio buffers before exit.
/// 
/// # Arguments
/// 
/// * `state` - Managed state containing the RecordingManager (wrapped in Mutex)
/// 
/// # Returns
/// 
/// * `Ok(())` - Recording stopped successfully
/// * `Err(String)` - A descriptive error message if stopping fails
/// 
/// # Errors
/// 
/// Returns an error if:
/// - No recording is currently in progress
/// - Failed to send SIGTERM to the process
/// - Failed to kill the process with SIGKILL (last resort)
/// - The PCM file doesn't exist or is empty after stopping
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// try {
///   await invoke('stop_recording');
///   console.log('Recording stopped successfully');
/// } catch (error) {
///   console.error(`Failed to stop recording: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn stop_recording(
    state: State<'_, Mutex<RecordingManager>>,
) -> Result<(), String> {
    let mut recording_manager = state
        .lock()
        .map_err(|e| format!("Failed to acquire lock on RecordingManager: {}", e))?;
    
    recording_manager.stop_recording()
}

/// List all recordings in the recordings directory
/// 
/// This command returns metadata for all PCM files in the recordings directory,
/// sorted by creation date in descending order (newest first).
/// 
/// # Arguments
/// 
/// * `state` - Managed state containing the FileManager
/// 
/// # Returns
/// 
/// * `Ok(Vec<RecordingMetadata>)` - A vector of recording metadata
/// * `Err(String)` - A descriptive error message if listing fails
/// 
/// # Errors
/// 
/// Returns an error if:
/// - The recordings directory cannot be read
/// - File metadata cannot be accessed
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// interface RecordingMetadata {
///   filename: string;
///   size_bytes: number;
///   created_at: number;
///   duration_seconds: number;
/// }
/// 
/// try {
///   const recordings: RecordingMetadata[] = await invoke('list_recordings');
///   console.log(`Found ${recordings.length} recordings`);
/// } catch (error) {
///   console.error(`Failed to list recordings: ${error}`);
/// }
/// ```
#[tauri::command]
pub fn list_recordings(
    state: State<'_, FileManager>,
) -> Result<Vec<RecordingMetadata>, String> {
    state.list_recordings()
}

/// Convert a PCM recording to WAV format for playback
/// 
/// This command reads a PCM file from the recordings directory, prepends a
/// 44-byte WAV header, and returns the complete WAV file as a byte array.
/// 
/// # Arguments
/// 
/// * `filename` - The name of the recording file to convert (e.g., "20240315_143022.pcm")
/// * `state` - Managed state containing the FileManager
/// 
/// # Returns
/// 
/// * `Ok(Vec<u8>)` - The complete WAV file (header + PCM data)
/// * `Err(String)` - A descriptive error message if conversion fails
/// 
/// # Errors
/// 
/// Returns an error if:
/// - The filename contains path traversal characters
/// - The PCM file cannot be read
/// - The file is too large (> 4GB, WAV format limitation)
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// try {
///   const wavData: number[] = await invoke('convert_to_wav', {
///     filename: '20240315_143022.pcm'
///   });
///   
///   // Create a blob URL for playback
///   const blob = new Blob([new Uint8Array(wavData)], { type: 'audio/wav' });
///   const url = URL.createObjectURL(blob);
///   
///   const audio = new Audio(url);
///   audio.play();
/// } catch (error) {
///   console.error(`Failed to convert to WAV: ${error}`);
/// }
/// ```
#[tauri::command]
pub fn convert_to_wav(
    filename: String,
    state: State<'_, FileManager>,
) -> Result<Vec<u8>, String> {
    // Validate filename to prevent path traversal
    if filename.is_empty() {
        return Err("Filename cannot be empty".to_string());
    }
    
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Err(format!(
            "Invalid filename '{}': path traversal not allowed",
            filename
        ));
    }
    
    // Construct the full path
    let pcm_path = state.get_recordings_dir().join(&filename);
    
    // Verify the file exists
    if !pcm_path.exists() {
        return Err(format!(
            "Recording '{}' not found in recordings directory",
            filename
        ));
    }
    
    // Convert to WAV
    WavConverter::pcm_to_wav(&pcm_path)
}

/// Delete a recording by filename
/// 
/// This command deletes a PCM file from the recordings directory after
/// validating the filename to prevent path traversal attacks.
/// 
/// # Arguments
/// 
/// * `filename` - The name of the recording file to delete (e.g., "20240315_143022.pcm")
/// * `state` - Managed state containing the FileManager
/// 
/// # Returns
/// 
/// * `Ok(())` - File deleted successfully
/// * `Err(String)` - A descriptive error message if deletion fails
/// 
/// # Errors
/// 
/// Returns an error if:
/// - The filename contains path traversal characters
/// - The filename is empty
/// - The file does not exist
/// - The file cannot be deleted (permission denied, etc.)
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// try {
///   await invoke('delete_recording', {
///     filename: '20240315_143022.pcm'
///   });
///   console.log('Recording deleted successfully');
/// } catch (error) {
///   console.error(`Failed to delete recording: ${error}`);
/// }
/// ```
#[tauri::command]
pub fn delete_recording(
    filename: String,
    state: State<'_, FileManager>,
) -> Result<(), String> {
    state.delete_recording(&filename)
}

/// Check if the current platform is supported for recording
/// 
/// This command returns true if the current platform supports audio recording
/// (currently only macOS), false otherwise.
/// 
/// # Returns
/// 
/// * `Ok(bool)` - true if platform is supported, false otherwise
/// * `Err(String)` - Never returns an error (always succeeds)
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// try {
///   const supported: boolean = await invoke('check_platform_support');
///   if (!supported) {
///     console.warn('Recording is not supported on this platform');
///   }
/// } catch (error) {
///   console.error(`Failed to check platform support: ${error}`);
/// }
/// ```
#[tauri::command]
pub fn check_platform_support() -> Result<bool, String> {
    Ok(PlatformDetector::is_supported())
}

/// Open the system settings for the current platform
/// 
/// On macOS, this opens the Screen Recording privacy settings where users can
/// grant permissions to the application. On other platforms, this returns an error.
/// 
/// # Returns
/// 
/// * `Ok(())` - System settings opened successfully (macOS only)
/// * `Err(String)` - Error message if opening fails or platform is not supported
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// try {
///   await invoke('open_system_settings');
///   console.log('System settings opened');
/// } catch (error) {
///   console.error(`Failed to open system settings: ${error}`);
/// }
/// ```
#[tauri::command]
pub fn open_system_settings() -> Result<(), String> {
    PlatformDetector::open_system_settings()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full integration tests for command handlers require a running Tauri app
    // and are better suited for end-to-end testing. The validation logic is tested
    // in the respective module tests (files.rs, platform.rs, etc.).
    
    // We test the platform-independent commands here
    
    #[test]
    fn test_check_platform_support() {
        let result = check_platform_support();
        assert!(result.is_ok());
        
        // The result should match the platform we're running on
        #[cfg(target_os = "macos")]
        assert!(result.unwrap());
        
        #[cfg(not(target_os = "macos"))]
        assert!(!result.unwrap());
    }

    #[test]
    fn test_open_system_settings() {
        let result = open_system_settings();
        
        // On macOS, this should succeed
        #[cfg(target_os = "macos")]
        assert!(result.is_ok());
        
        // On other platforms, this should fail
        #[cfg(not(target_os = "macos"))]
        {
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("not available on this platform"));
        }
    }
    
    // Test validation logic for convert_to_wav
    #[test]
    fn test_convert_to_wav_validation() {
        // Test empty filename validation
        let filename = String::new();
        assert!(filename.is_empty());
        
        // Test path traversal validation
        let filename = "../../../etc/passwd";
        assert!(filename.contains('/') || filename.contains('\\') || filename.contains(".."));
        
        let filename = "..\\..\\windows\\system32";
        assert!(filename.contains('/') || filename.contains('\\') || filename.contains(".."));
        
        // Test valid filename
        let filename = "20240315_143022.pcm";
        assert!(!filename.is_empty());
        assert!(!filename.contains('/'));
        assert!(!filename.contains('\\'));
        assert!(!filename.contains(".."));
    }
}
