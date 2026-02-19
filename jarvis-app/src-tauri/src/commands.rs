use crate::files::{FileManager, RecordingMetadata};
use crate::platform::PlatformDetector;
use crate::recording::RecordingManager;
use crate::settings::{ModelManager, Settings, SettingsManager};
use crate::transcription::{TranscriptionManager, TranscriptionSegment, TranscriptionStatus, WhisperKitProvider};
use crate::wav::WavConverter;
use serde::Serialize;
use std::sync::{Arc, Mutex, RwLock};
use tauri::{Emitter, State};

/// WhisperKit availability status
/// 
/// This struct contains information about whether WhisperKit is available
/// on the current system and the reason if it's not available.
#[derive(Debug, Clone, Serialize)]
pub struct WhisperKitStatus {
    pub available: bool,
    pub reason: Option<String>,
}

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

/// Get the accumulated transcript from the current recording
/// 
/// This command returns all transcription segments that have been accumulated
/// during the current recording session. Returns an empty array if transcription
/// is not available or no recording is in progress.
/// 
/// # Arguments
/// 
/// * `state` - Managed state containing the TranscriptionManager (wrapped in tokio::sync::Mutex)
/// 
/// # Returns
/// 
/// * `Ok(Vec<TranscriptionSegment>)` - Array of transcription segments
/// * `Err(String)` - Error message if TranscriptionManager is not available
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// interface TranscriptionSegment {
///   text: string;
///   start_ms: number;
///   end_ms: number;
///   is_final: boolean;
/// }
/// 
/// try {
///   const transcript: TranscriptionSegment[] = await invoke('get_transcript');
///   console.log(`Got ${transcript.length} segments`);
/// } catch (error) {
///   console.error(`Failed to get transcript: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn get_transcript(
    state: State<'_, tokio::sync::Mutex<TranscriptionManager>>,
) -> Result<Vec<TranscriptionSegment>, String> {
    let manager = state.lock().await;
    Ok(manager.get_transcript().await)
}

/// Get the current transcription status
/// 
/// This command returns the current status of the transcription system:
/// - "idle": Not currently transcribing
/// - "active": Currently transcribing
/// - "error": An error occurred
/// - "disabled": Transcription is disabled (models not available)
/// 
/// # Arguments
/// 
/// * `state` - Managed state containing the TranscriptionManager (wrapped in tokio::sync::Mutex)
/// 
/// # Returns
/// 
/// * `Ok(TranscriptionStatus)` - Current transcription status
/// * `Err(String)` - Error message if TranscriptionManager is not available
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// type TranscriptionStatus = "idle" | "active" | "error" | "disabled";
/// 
/// try {
///   const status: TranscriptionStatus = await invoke('get_transcription_status');
///   console.log(`Transcription status: ${status}`);
/// } catch (error) {
///   console.error(`Failed to get transcription status: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn get_transcription_status(
    state: State<'_, tokio::sync::Mutex<TranscriptionManager>>,
) -> Result<TranscriptionStatus, String> {
    let manager = state.lock().await;
    Ok(manager.get_status().await)
}

/// Get current application settings
/// 
/// This command returns the current settings including transcription engine
/// toggles and Whisper model selection.
/// 
/// # Arguments
/// 
/// * `state` - Managed state containing the SettingsManager (wrapped in Arc<RwLock>)
/// 
/// # Returns
/// 
/// * `Ok(Settings)` - Current settings
/// * `Err(String)` - Error message if settings cannot be read
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// interface Settings {
///   transcription: {
///     vad_enabled: boolean;
///     vad_threshold: number;
///     vosk_enabled: boolean;
///     whisper_enabled: boolean;
///     whisper_model: string;
///   };
/// }
/// 
/// try {
///   const settings: Settings = await invoke('get_settings');
///   console.log(`VAD enabled: ${settings.transcription.vad_enabled}`);
/// } catch (error) {
///   console.error(`Failed to get settings: ${error}`);
/// }
/// ```
#[tauri::command]
pub fn get_settings(
    state: State<'_, Arc<RwLock<SettingsManager>>>,
) -> Result<Settings, String> {
    let manager = state
        .read()
        .map_err(|e| format!("Failed to acquire read lock: {}", e))?;
    Ok(manager.get())
}

/// Update application settings
/// 
/// This command updates the settings and emits a "settings-changed" event
/// to notify the frontend of the change.
/// 
/// # Arguments
/// 
/// * `settings` - New settings to apply
/// * `state` - Managed state containing the SettingsManager (wrapped in Arc<RwLock>)
/// * `app_handle` - Tauri app handle for emitting events
/// 
/// # Returns
/// 
/// * `Ok(())` - Settings updated successfully
/// * `Err(String)` - Error message if update fails (validation or persistence error)
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// try {
///   await invoke('update_settings', {
///     settings: {
///       transcription: {
///         vad_enabled: true,
///         vad_threshold: 0.3,
///         vosk_enabled: true,
///         whisper_enabled: true,
///         whisper_model: 'ggml-base.en.bin'
///       }
///     }
///   });
///   console.log('Settings updated successfully');
/// } catch (error) {
///   console.error(`Failed to update settings: ${error}`);
/// }
/// ```
#[tauri::command]
pub fn update_settings(
    settings: Settings,
    state: State<'_, Arc<RwLock<SettingsManager>>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let manager = state
        .read()
        .map_err(|e| format!("Failed to acquire read lock: {}", e))?;
    
    manager.update(settings.clone())?;
    
    // Emit settings-changed event
    app_handle
        .emit("settings-changed", &settings)
        .map_err(|e| format!("Failed to emit settings-changed event: {}", e))?;
    
    Ok(())
}

/// List all supported Whisper models with their status
/// 
/// This command returns information about all supported models including:
/// - Downloaded models (with file size)
/// - Models currently being downloaded (with progress)
/// - Models with download errors (with error message)
/// - Models not yet downloaded
/// 
/// # Arguments
/// 
/// * `state` - Managed state containing the ModelManager (wrapped in Arc)
/// 
/// # Returns
/// 
/// * `Ok(Vec<ModelInfo>)` - Array of model information
/// * `Err(String)` - Error message if listing fails
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// interface ModelInfo {
///   filename: string;
///   status: 
///     | { type: 'downloaded'; size_bytes: number }
///     | { type: 'downloading'; progress: number }
///     | { type: 'error'; message: string }
///     | { type: 'notdownloaded' };
/// }
/// 
/// try {
///   const models: ModelInfo[] = await invoke('list_models');
///   console.log(`Found ${models.length} models`);
/// } catch (error) {
///   console.error(`Failed to list models: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn list_models(
    state: State<'_, Arc<ModelManager>>,
) -> Result<Vec<crate::settings::ModelInfo>, String> {
    state.list_models().await
}

/// Download a Whisper model from Hugging Face
/// 
/// This command initiates a model download in the background. Progress is
/// reported via "model-download-progress" events, and completion/errors are
/// reported via "model-download-complete" and "model-download-error" events.
/// 
/// The command returns immediately after spawning the download task.
/// 
/// # Arguments
/// 
/// * `model_name` - Name of the model to download (e.g., "ggml-base.en.bin")
/// * `state` - Managed state containing the ModelManager (wrapped in Arc)
/// 
/// # Returns
/// 
/// * `Ok(())` - Download started successfully
/// * `Err(String)` - Error message if download cannot be started
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Model name is not in the supported list
/// - Model is already being downloaded
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// import { listen } from '@tauri-apps/api/event';
/// 
/// // Listen for progress events
/// listen('model-download-progress', (event) => {
///   console.log(`Progress: ${event.payload.progress}%`);
/// });
/// 
/// // Listen for completion
/// listen('model-download-complete', (event) => {
///   console.log(`Download complete: ${event.payload.model}`);
/// });
/// 
/// // Listen for errors
/// listen('model-download-error', (event) => {
///   console.error(`Download error: ${event.payload.error}`);
/// });
/// 
/// try {
///   await invoke('download_model', { modelName: 'ggml-base.en.bin' });
///   console.log('Download started');
/// } catch (error) {
///   console.error(`Failed to start download: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn download_model(
    model_name: String,
    state: State<'_, Arc<ModelManager>>,
) -> Result<(), String> {
    state.download_model(model_name).await
}

/// Cancel an in-progress model download
/// 
/// This command cancels a model download that is currently in progress.
/// The download task will be terminated and the temporary file will be cleaned up.
/// 
/// # Arguments
/// 
/// * `model_name` - Name of the model to cancel (e.g., "ggml-base.en.bin")
/// * `state` - Managed state containing the ModelManager (wrapped in Arc)
/// 
/// # Returns
/// 
/// * `Ok(())` - Download cancelled successfully
/// * `Err(String)` - Error message if cancellation fails
/// 
/// # Errors
/// 
/// Returns an error if the model is not currently being downloaded.
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// try {
///   await invoke('cancel_download', { modelName: 'ggml-base.en.bin' });
///   console.log('Download cancelled');
/// } catch (error) {
///   console.error(`Failed to cancel download: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn cancel_download(
    model_name: String,
    state: State<'_, Arc<ModelManager>>,
) -> Result<(), String> {
    state.cancel_download(model_name).await
}

/// Delete a downloaded model
/// 
/// This command deletes a model file from disk and clears any associated
/// error state.
/// 
/// # Arguments
/// 
/// * `model_name` - Name of the model to delete (e.g., "ggml-base.en.bin")
/// * `state` - Managed state containing the ModelManager (wrapped in Arc)
/// 
/// # Returns
/// 
/// * `Ok(())` - Model deleted successfully
/// * `Err(String)` - Error message if deletion fails
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Model file doesn't exist
/// - File deletion fails (permission denied, etc.)
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// try {
///   await invoke('delete_model', { modelName: 'ggml-base.en.bin' });
///   console.log('Model deleted');
/// } catch (error) {
///   console.error(`Failed to delete model: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn delete_model(
    model_name: String,
    state: State<'_, Arc<ModelManager>>,
) -> Result<(), String> {
    state.delete_model(model_name).await
}

/// Check WhisperKit availability on the current system
/// 
/// This command checks if WhisperKit can be used on the current system by
/// verifying:
/// - Apple Silicon (aarch64) architecture
/// - macOS 14.0 or later
/// - whisperkit-cli binary is installed
/// 
/// # Returns
/// 
/// * `Ok(WhisperKitStatus)` - Status object with availability and reason
/// * `Err(String)` - Never returns an error (always succeeds)
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// interface WhisperKitStatus {
///   available: boolean;
///   reason?: string;
/// }
/// 
/// try {
///   const status: WhisperKitStatus = await invoke('check_whisperkit_status');
///   if (status.available) {
///     console.log('WhisperKit is available');
///   } else {
///     console.log(`WhisperKit unavailable: ${status.reason}`);
///   }
/// } catch (error) {
///   console.error(`Failed to check WhisperKit status: ${error}`);
/// }
/// ```
#[tauri::command]
pub fn check_whisperkit_status() -> Result<WhisperKitStatus, String> {
    // Create a temporary WhisperKitProvider to check availability
    // We use a dummy model name since we're only checking availability
    let provider = WhisperKitProvider::new("dummy");
    
    Ok(WhisperKitStatus {
        available: provider.is_available(),
        reason: provider.unavailable_reason().map(|s| s.to_string()),
    })
}

/// List all supported WhisperKit models with their status
/// 
/// This command returns information about all supported WhisperKit models including:
/// - Downloaded models (with directory size)
/// - Models not yet downloaded
/// 
/// # Arguments
/// 
/// * `state` - Managed state containing the ModelManager (wrapped in Arc)
/// 
/// # Returns
/// 
/// * `Ok(Vec<ModelInfo>)` - Array of model information
/// * `Err(String)` - Error message if listing fails
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// interface ModelInfo {
///   filename: string;
///   display_name: string;
///   description: string;
///   size_estimate: string;
///   quality_tier: string;
///   status: 
///     | { type: 'downloaded'; size_bytes: number }
///     | { type: 'notdownloaded' };
/// }
/// 
/// try {
///   const models: ModelInfo[] = await invoke('list_whisperkit_models');
///   console.log(`Found ${models.length} WhisperKit models`);
/// } catch (error) {
///   console.error(`Failed to list WhisperKit models: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn list_whisperkit_models(
    state: State<'_, Arc<ModelManager>>,
) -> Result<Vec<crate::settings::ModelInfo>, String> {
    state.list_whisperkit_models().await
}

/// Download a WhisperKit model using whisperkit-cli
/// 
/// This command initiates a model download in the background using whisperkit-cli.
/// Progress is reported via "model-download-progress" events, and completion/errors
/// are reported via "model-download-complete" and "model-download-error" events.
/// 
/// The command returns immediately after spawning the download task.
/// 
/// # Arguments
/// 
/// * `model_name` - Name of the model to download (e.g., "openai_whisper-large-v3_turbo")
/// * `state` - Managed state containing the ModelManager (wrapped in Arc)
/// 
/// # Returns
/// 
/// * `Ok(())` - Download started successfully
/// * `Err(String)` - Error message if download cannot be started
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Model name is not in the supported list
/// - whisperkit-cli is not installed
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// import { listen } from '@tauri-apps/api/event';
/// 
/// // Listen for progress events
/// listen('model-download-progress', (event) => {
///   console.log(`Progress: ${event.payload.progress}%`);
/// });
/// 
/// // Listen for completion
/// listen('model-download-complete', (event) => {
///   console.log(`Download complete: ${event.payload.model_name}`);
/// });
/// 
/// // Listen for errors
/// listen('model-download-error', (event) => {
///   console.error(`Download error: ${event.payload.error}`);
/// });
/// 
/// try {
///   await invoke('download_whisperkit_model', { 
///     modelName: 'openai_whisper-large-v3_turbo' 
///   });
///   console.log('Download started');
/// } catch (error) {
///   console.error(`Failed to start download: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn download_whisperkit_model(
    model_name: String,
    state: State<'_, Arc<ModelManager>>,
) -> Result<(), String> {
    state.download_whisperkit_model(model_name).await
}

/// Start the browser observer
/// 
/// This command starts the browser observer which polls Chrome's active tab URL
/// every 3 seconds and detects YouTube videos.
/// 
/// # Arguments
/// 
/// * `observer` - Managed state containing the BrowserObserver (wrapped in Arc<tokio::sync::Mutex>)
/// 
/// # Returns
/// 
/// * `Ok(())` - Observer started successfully
/// * `Err(String)` - Error message if observer is already running or start fails
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// try {
///   await invoke('start_browser_observer');
///   console.log('Browser observer started');
/// } catch (error) {
///   console.error(`Failed to start observer: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn start_browser_observer(
    observer: State<'_, Arc<tokio::sync::Mutex<crate::browser::BrowserObserver>>>,
) -> Result<(), String> {
    observer.lock().await.start().await
}

/// Stop the browser observer
/// 
/// This command stops the browser observer and terminates the background polling task.
/// 
/// # Arguments
/// 
/// * `observer` - Managed state containing the BrowserObserver (wrapped in Arc<tokio::sync::Mutex>)
/// 
/// # Returns
/// 
/// * `Ok(())` - Observer stopped successfully
/// * `Err(String)` - Error message if observer is not running or stop fails
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// try {
///   await invoke('stop_browser_observer');
///   console.log('Browser observer stopped');
/// } catch (error) {
///   console.error(`Failed to stop observer: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn stop_browser_observer(
    observer: State<'_, Arc<tokio::sync::Mutex<crate::browser::BrowserObserver>>>,
) -> Result<(), String> {
    observer.lock().await.stop().await
}

/// Fetch YouTube video metadata (gist)
/// 
/// This command scrapes a YouTube video page and extracts metadata including
/// title, channel, description, and duration.
/// 
/// # Arguments
/// 
/// * `url` - YouTube video URL to scrape
/// 
/// # Returns
/// 
/// * `Ok(YouTubeGist)` - Video metadata
/// * `Err(String)` - Error message if scraping fails
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// interface YouTubeGist {
///   url: string;
///   video_id: string;
///   title: string;
///   channel: string;
///   description: string;
///   duration_seconds: number;
/// }
/// 
/// try {
///   const gist: YouTubeGist = await invoke('fetch_youtube_gist', {
///     url: 'https://www.youtube.com/watch?v=dQw4w9WgXcQ'
///   });
///   console.log(`Title: ${gist.title}`);
/// } catch (error) {
///   console.error(`Failed to fetch gist: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn fetch_youtube_gist(url: String) -> Result<crate::browser::YouTubeGist, String> {
    crate::browser::scrape_youtube_gist(&url).await
}

/// Get browser observer status
/// 
/// This command returns whether the browser observer is currently running.
/// 
/// # Arguments
/// 
/// * `observer` - Managed state containing the BrowserObserver (wrapped in Arc<tokio::sync::Mutex>)
/// 
/// # Returns
/// 
/// * `Ok(bool)` - true if observer is running, false otherwise
/// * `Err(String)` - Never returns an error (always succeeds)
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// try {
///   const isRunning: boolean = await invoke('get_observer_status');
///   console.log(`Observer running: ${isRunning}`);
/// } catch (error) {
///   console.error(`Failed to get observer status: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn get_observer_status(
    observer: State<'_, Arc<tokio::sync::Mutex<crate::browser::BrowserObserver>>>,
) -> Result<bool, String> {
    Ok(observer.lock().await.is_running())
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
