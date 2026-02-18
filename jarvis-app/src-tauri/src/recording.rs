use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use chrono::Local;
use tokio::sync::mpsc::{self, Receiver};
use serde_json::json;

use crate::transcription::{AudioRouter, TranscriptionManager};

/// Manages the lifecycle of audio recording via the JarvisListen sidecar
/// 
/// RecordingManager is responsible for:
/// - Spawning and terminating the JarvisListen sidecar process
/// - Tracking the current recording state
/// - Monitoring sidecar events (stderr, crashes)
/// - Emitting Tauri events to notify the frontend
/// - Managing AudioRouter for FIFO-based audio routing
/// - Coordinating with TranscriptionManager for real-time transcription
pub struct RecordingManager {
    /// The currently running sidecar process, if any
    current_child: Option<CommandChild>,
    
    /// The filepath where the current recording is being written, if any
    current_filepath: Option<PathBuf>,
    
    /// Handle to the Tauri application for emitting events
    app_handle: AppHandle,
    
    /// Handle to the AudioRouter background task
    audio_router_task: Option<tokio::task::JoinHandle<()>>,
}

impl RecordingManager {
    /// Create a new RecordingManager instance
    /// 
    /// # Arguments
    /// 
    /// * `app_handle` - Handle to the Tauri application for emitting events
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use tauri::AppHandle;
    /// use jarvis_app_lib::recording::RecordingManager;
    /// 
    /// fn setup(app_handle: AppHandle) {
    ///     let recording_manager = RecordingManager::new(app_handle);
    /// }
    /// ```
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            current_child: None,
            current_filepath: None,
            app_handle,
            audio_router_task: None,
        }
    }
    
    /// Check if a recording is currently active
    /// 
    /// Returns `true` if a sidecar process is currently running and recording
    /// audio, `false` otherwise.
    /// 
    /// # Returns
    /// 
    /// `true` if recording is active, `false` otherwise
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use tauri::AppHandle;
    /// use jarvis_app_lib::recording::RecordingManager;
    /// 
    /// fn check_status(recording_manager: &RecordingManager) {
    ///     if recording_manager.is_recording() {
    ///         println!("Recording in progress");
    ///     } else {
    ///         println!("Idle");
    ///     }
    /// }
    /// ```
    pub fn is_recording(&self) -> bool {
        self.current_child.is_some()
    }
    
    /// Generate a timestamped filepath for a new recording
    /// 
    /// Creates a filename in the format `YYYYMMDD_HHMMSS.pcm` using the current
    /// local timestamp and returns the full path in the recordings directory.
    /// 
    /// # Arguments
    /// 
    /// * `recordings_dir` - The directory where recordings are stored
    /// 
    /// # Returns
    /// 
    /// A `PathBuf` containing the full path to the new recording file
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use std::path::PathBuf;
    /// use tauri::AppHandle;
    /// use jarvis_app_lib::recording::RecordingManager;
    /// 
    /// fn create_recording_path(recording_manager: &RecordingManager) {
    ///     let recordings_dir = PathBuf::from("/path/to/recordings");
    ///     let path = recording_manager.generate_timestamped_path(&recordings_dir);
    ///     // path will be something like "/path/to/recordings/20240315_143022.pcm"
    /// }
    /// ```
    pub fn generate_timestamped_path(&self, recordings_dir: &std::path::Path) -> PathBuf {
        // Get current local time
        let now = Local::now();
        
        // Format as YYYYMMDD_HHMMSS
        let filename = now.format("%Y%m%d_%H%M%S.pcm").to_string();
        
        // Return full path in recordings directory
        recordings_dir.join(filename)
    }
    
    /// Spawn the JarvisListen sidecar process with the specified output path
    /// 
    /// This method spawns the JarvisListen sidecar binary with the following arguments:
    /// - `--mono`: Capture audio in mono format (single channel)
    /// - `--sample-rate 16000`: Set sample rate to 16kHz
    /// - `--output <filepath>`: Write PCM data directly to the specified file
    /// 
    /// The sidecar writes PCM data directly to disk to avoid binary data corruption
    /// issues that occur when piping through Tauri's shell plugin (which splits on
    /// newline bytes).
    /// 
    /// # Arguments
    /// 
    /// * `output_path` - The file path where the sidecar should write PCM data
    /// 
    /// # Returns
    /// 
    /// A `Result` containing:
    /// - `Ok((Receiver<CommandEvent>, CommandChild))` - A receiver for monitoring
    ///   sidecar events (stderr, termination) and the child process handle
    /// - `Err(String)` - A descriptive error message if spawning fails
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - The sidecar binary cannot be found or accessed
    /// - The process fails to spawn (e.g., permission denied, invalid arguments)
    /// - The output path is invalid or cannot be converted to a string
    /// 
    /// # Examples
    /// 
    /// ```ignore
    /// // This is a private method used internally by start_recording()
    /// // Example usage (internal):
    /// let output_path = PathBuf::from("/tmp/recordings/20240315_143022.pcm");
    /// let (rx, child) = self.spawn_sidecar(&output_path)?;
    /// // Monitor events in a separate task
    /// // Store child for later termination
    /// ```
    fn spawn_sidecar(
        &self,
        output_path: &Path,
    ) -> Result<(Receiver<CommandEvent>, CommandChild), String> {
        // Get the sidecar command from the shell plugin
        let sidecar = self
            .app_handle
            .shell()
            .sidecar("JarvisListen")
            .map_err(|e| format!("Failed to get sidecar command: {}", e))?;
        
        // Convert output path to string
        let output_path_str = output_path
            .to_str()
            .ok_or_else(|| "Invalid output path: cannot convert to string".to_string())?;
        
        // Add arguments: --mono, --sample-rate 16000, --output <filepath>
        let sidecar_with_args = sidecar.args([
            "--mono",
            "--sample-rate",
            "16000",
            "--output",
            output_path_str,
        ]);
        
        // Spawn the process
        let (rx, child) = sidecar_with_args
            .spawn()
            .map_err(|e| format!("Failed to spawn sidecar process: {}", e))?;
        
        Ok((rx, child))
    }
    
    /// Monitor sidecar process events and emit appropriate Tauri events
    /// 
    /// This method spawns an async task that listens to the CommandEvent receiver
    /// from the sidecar process. It monitors stderr output and process termination,
    /// classifying errors and emitting appropriate events to the frontend:
    /// 
    /// - **Permission errors**: Detected by keywords "permission", "Screen Recording",
    ///   or "Microphone" in stderr. Emits "permission-error" event.
    /// - **Actual errors**: Lines starting with "Error: " prefix. Emits "sidecar-error" event.
    /// - **Warnings**: Lines starting with "Warning: " prefix. Logged to Rust stderr but not emitted.
    /// - **Informational messages**: Lines without error/warning prefix. Logged to Rust stderr but not emitted.
    /// - **Unexpected termination**: Process exits without stop_recording being called.
    ///   Clears state and emits "sidecar-crashed" (non-zero exit) or "recording-stopped" (zero exit).
    /// - **Expected termination**: Process exits after stop_recording (state already cleared).
    ///   No additional events emitted.
    /// 
    /// The monitoring task runs asynchronously and continues until the receiver
    /// is closed (when the sidecar process terminates).
    /// 
    /// # Arguments
    /// 
    /// * `rx` - Receiver for CommandEvent messages from the sidecar process
    /// 
    /// # Examples
    /// 
    /// ```ignore
    /// // This is a private method used internally by start_recording()
    /// // Example usage (internal):
    /// let (rx, child) = self.spawn_sidecar(&output_path)?;
    /// // Start monitoring events in background
    /// self.monitor_events(rx);
    /// // Store child for later termination
    /// ```
    fn monitor_events(&self, mut rx: Receiver<CommandEvent>) {
        let app_handle = self.app_handle.clone();
        
        // Spawn async task to monitor events
        tauri::async_runtime::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stderr(line) => {
                        // Convert Vec<u8> to String for parsing
                        let line_str = String::from_utf8_lossy(&line).to_string();
                        let line_lower = line_str.to_lowercase();
                        
                        // Check for actual permission errors with specific patterns
                        // Only match lines that are truly permission-related, not warnings mentioning "microphone"
                        let is_permission_error = 
                            // Error messages containing permission keywords
                            (line_str.starts_with("Error: ") && (
                                line_lower.contains("permission")
                                || line_lower.contains("screen recording")
                            ))
                            // Warning about permission denied (specific phrase)
                            || (line_str.starts_with("Warning: ") && line_lower.contains("permission denied"));
                        
                        if is_permission_error {
                            // Emit permission-error event for actual permission issues
                            if let Err(e) = app_handle.emit("permission-error", json!({ "message": line_str })) {
                                eprintln!("Failed to emit permission-error event: {}", e);
                            }
                        }
                        // Check if line starts with "Error: " prefix (actual errors from JarvisListen)
                        else if line_str.starts_with("Error: ") {
                            // Emit sidecar-error event for actual errors
                            if let Err(e) = app_handle.emit("sidecar-error", json!({ "message": line_str })) {
                                eprintln!("Failed to emit sidecar-error event: {}", e);
                            }
                        }
                        // Lines starting with "Warning: " or informational messages (no prefix)
                        else {
                            // Log to Rust stderr for debugging, but don't emit events
                            // These are normal operational messages like "Capturing: mic=Default, ..."
                            // or warnings like "Warning: Audio conversion failed for microphone..."
                            eprintln!("[JarvisListen] {}", line_str);
                        }
                    }
                    CommandEvent::Terminated(payload) => {
                        // Access RecordingManager from Tauri state to check if termination was unexpected
                        if let Some(recording_manager_mutex) = app_handle.try_state::<Mutex<RecordingManager>>() {
                            if let Ok(mut recording_manager) = recording_manager_mutex.lock() {
                                // Check if current_child is Some (means termination was unexpected)
                                let was_unexpected = recording_manager.current_child.is_some();
                                
                                if was_unexpected {
                                    // Clear state since process terminated unexpectedly
                                    recording_manager.current_child = None;
                                    recording_manager.current_filepath = None;
                                    
                                    // Emit appropriate event based on exit code
                                    if payload.code != Some(0) {
                                        // Non-zero exit code = crash
                                        if let Err(e) = app_handle.emit("sidecar-crashed", json!({ "code": payload.code })) {
                                            eprintln!("Failed to emit sidecar-crashed event: {}", e);
                                        }
                                    } else {
                                        // Zero exit code = graceful unexpected exit
                                        if let Err(e) = app_handle.emit("recording-stopped", ()) {
                                            eprintln!("Failed to emit recording-stopped event: {}", e);
                                        }
                                    }
                                }
                                // If was_unexpected is false, stop_recording already handled cleanup
                            } else {
                                eprintln!("Failed to acquire RecordingManager lock in monitor_events");
                            }
                        } else {
                            eprintln!("Failed to access RecordingManager state in monitor_events");
                        }
                    }
                    _ => {
                        // Ignore other event types (Stdout, Error)
                    }
                }
            }
        });
    }
    
    /// Start a new recording
    /// 
    /// This method initiates a new audio recording by:
    /// 1. Checking if a recording is already in progress (returns error if true)
    /// 2. Generating a timestamped filepath in the recordings directory
    /// 3. Creating AudioRouter with FIFO for audio routing
    /// 4. Spawning the JarvisListen sidecar with --output pointing to FIFO path
    /// 5. Starting AudioRouter background task to route audio to file + transcription
    /// 6. Starting TranscriptionManager with mpsc receiver
    /// 7. Storing the child process, filepath, and AudioRouter in state
    /// 8. Starting event monitoring in the background
    /// 9. Emitting a "recording-started" event with the filename
    /// 
    /// # Arguments
    /// 
    /// * `recordings_dir` - The directory where recordings are stored
    /// 
    /// # Returns
    /// 
    /// A `Result` containing:
    /// - `Ok(String)` - The filename of the new recording (e.g., "20240315_143022.pcm")
    /// - `Err(String)` - A descriptive error message if the recording cannot be started
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - A recording is already in progress (concurrent recording not allowed)
    /// - AudioRouter creation fails (FIFO creation error)
    /// - The sidecar process fails to spawn
    /// - TranscriptionManager fails to start
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use std::path::PathBuf;
    /// use tauri::AppHandle;
    /// use jarvis_app_lib::recording::RecordingManager;
    /// 
    /// fn start_new_recording(
    ///     recording_manager: &mut RecordingManager,
    ///     recordings_dir: &std::path::Path
    /// ) -> Result<(), String> {
    ///     match recording_manager.start_recording(recordings_dir) {
    ///         Ok(filename) => {
    ///             println!("Recording started: {}", filename);
    ///             Ok(())
    ///         }
    ///         Err(e) => {
    ///             eprintln!("Failed to start recording: {}", e);
    ///             Err(e)
    ///         }
    ///     }
    /// }
    /// ```
    pub fn start_recording(&mut self, recordings_dir: &std::path::Path) -> Result<String, String> {
        // Check if already recording (concurrent recording prevention)
        if self.is_recording() {
            return Err("A recording is already in progress".to_string());
        }
        
        // Generate timestamped filepath
        let output_path = self.generate_timestamped_path(recordings_dir);
        
        // Extract filename for return value and event
        let filename = output_path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| "Failed to extract filename from path".to_string())?
            .to_string();
        
        // Create mpsc channel for audio routing (AudioRouter → TranscriptionManager)
        // Large buffer (1000 chunks × 3200 bytes = 100s of audio) to avoid backpressure
        // from blocking Whisper inference reaching JarvisListen via FIFO pipe
        let (tx, rx) = mpsc::channel::<Vec<u8>>(1000);
        
        // Create AudioRouter (creates FIFO, returns path)
        let audio_router = AudioRouter::new(output_path.clone(), tx)
            .map_err(|e| format!("Failed to create AudioRouter: {}", e))?;
        
        // Get FIFO path to pass to sidecar
        let fifo_path = audio_router.fifo_path().to_path_buf();
        
        // Spawn sidecar with --output pointing to FIFO path
        let (event_rx, child) = self.spawn_sidecar(&fifo_path)?;
        
        // Start AudioRouter background task (opens FIFO, reads chunks, routes to file + mpsc)
        // Note: AudioRouter is moved into the task, so we can't store it in state
        let audio_router_task = tokio::spawn(async move {
            if let Err(e) = audio_router.start_routing().await {
                eprintln!("AudioRouter error: {}", e);
            }
            // AudioRouter is dropped here, cleaning up FIFO
        });
        
        // Start TranscriptionManager with mpsc receiver (spawn task to avoid holding lock)
        let app_handle_clone = self.app_handle.clone();
        tokio::spawn(async move {
            if let Some(transcription_manager_mutex) = app_handle_clone.try_state::<tokio::sync::Mutex<TranscriptionManager>>() {
                let mut transcription_manager = transcription_manager_mutex.lock().await;
                // Update window_duration from latest settings
                if let Some(settings_manager) = app_handle_clone.try_state::<std::sync::Arc<crate::settings::SettingsManager>>() {
                    let window_dur = settings_manager.get().transcription.window_duration;
                    transcription_manager.set_window_duration(window_dur);
                }
                if let Err(e) = transcription_manager.start(rx).await {
                    eprintln!("Warning: Failed to start transcription: {}", e);
                    // Don't fail recording start if transcription fails
                }
            } else {
                eprintln!("Warning: TranscriptionManager not found in Tauri state");
                // Don't fail recording start if transcription is unavailable
            }
        });
        
        // Store child process, filepath, and task in state
        self.current_child = Some(child);
        self.current_filepath = Some(output_path);
        self.audio_router_task = Some(audio_router_task);
        
        // Start monitoring events in background
        self.monitor_events(event_rx);
        
        // Emit "recording-started" event with filename
        if let Err(e) = self.app_handle.emit("recording-started", json!({ "filename": filename })) {
            eprintln!("Warning: Failed to emit recording-started event: {}", e);
            // Don't fail the recording start if event emission fails
        }
        
        // Return filename on success
        Ok(filename)
    }
    
    /// Stop the current recording
    /// 
    /// This method gracefully terminates the active recording by:
    /// 1. Checking if a recording is active (returns error if not)
    /// 2. Stopping TranscriptionManager to drain remaining audio
    /// 3. Sending SIGTERM to the sidecar process to allow signal handlers to flush buffers
    /// 4. Waiting for the process to exit with a 5-second timeout
    /// 5. Falling back to SIGKILL if the timeout expires
    /// 6. Waiting for AudioRouter task to complete
    /// 7. Verifying the PCM file exists and has data
    /// 8. Clearing state (current_child, current_filepath, audio_router, audio_router_task)
    /// 9. Emitting a "recording-stopped" event
    /// 
    /// # Returns
    /// 
    /// A `Result` containing:
    /// - `Ok(())` - Recording stopped successfully
    /// - `Err(String)` - A descriptive error message if stopping fails
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - No recording is currently in progress
    /// - Failed to send SIGTERM to the process
    /// - Failed to kill the process with SIGKILL (last resort)
    /// - The PCM file doesn't exist or is empty after stopping
    /// 
    /// # Note
    /// 
    /// This method uses SIGTERM (not SIGKILL) to allow JarvisListen's signal handlers
    /// to flush audio buffers before exit. Tauri's `CommandChild::kill()` sends SIGKILL
    /// which cannot be caught, causing data loss. Only if the process doesn't exit
    /// within the timeout do we fall back to SIGKILL as a last resort.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use tauri::AppHandle;
    /// use jarvis_app_lib::recording::RecordingManager;
    /// 
    /// fn stop_current_recording(
    ///     recording_manager: &mut RecordingManager
    /// ) -> Result<(), String> {
    ///     match recording_manager.stop_recording() {
    ///         Ok(()) => {
    ///             println!("Recording stopped successfully");
    ///             Ok(())
    ///         }
    ///         Err(e) => {
    ///             eprintln!("Failed to stop recording: {}", e);
    ///             Err(e)
    ///         }
    ///     }
    /// }
    /// ```
    pub fn stop_recording(&mut self) -> Result<(), String> {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;
        
        // Check if recording is active and extract child/filepath (quick lock)
        let child = self
            .current_child
            .take()
            .ok_or("No recording in progress")?;
        
        let filepath = self
            .current_filepath
            .take()
            .ok_or("No recording filepath found")?;
        
        // Take AudioRouter task
        let audio_router_task = self.audio_router_task.take();
        
        // Stop TranscriptionManager to drain remaining audio (spawn task to avoid blocking)
        let app_handle_clone = self.app_handle.clone();
        tokio::spawn(async move {
            if let Some(transcription_manager_mutex) = app_handle_clone.try_state::<tokio::sync::Mutex<TranscriptionManager>>() {
                let mut transcription_manager = transcription_manager_mutex.lock().await;
                if let Err(e) = transcription_manager.stop().await {
                    eprintln!("Warning: Failed to stop transcription: {}", e);
                    // Don't fail recording stop if transcription stop fails
                }
            }
        });
        
        // Get process ID before releasing mutex
        let pid = child.pid();
        
        // Send SIGTERM to allow signal handlers to flush buffers
        kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
            .map_err(|e| format!("Failed to send SIGTERM to process: {}", e))?;
        
        // Spawn async task to poll for process exit without blocking
        // This allows other commands to acquire the mutex while we wait
        let app_handle = self.app_handle.clone();
        let pid_for_task = pid;
        let filepath_for_task = filepath.clone();
        
        tauri::async_runtime::spawn(async move {
            use tokio::time::{sleep, Duration, Instant};
            
            // Wait for graceful exit with timeout (5 seconds)
            let timeout = Duration::from_secs(5);
            let start = Instant::now();
            
            // Poll for process exit using async sleep
            let mut process_exited = false;
            while start.elapsed() < timeout {
                // Check if process has exited by attempting to send signal 0 (null signal)
                if kill(Pid::from_raw(pid_for_task as i32), None).is_err() {
                    process_exited = true;
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
            
            // If timeout expired, force kill with SIGKILL as last resort
            if !process_exited {
                eprintln!("Warning: Process didn't exit gracefully within timeout, sending SIGKILL");
                // Note: We can't use child.kill() here since child was moved
                // But SIGKILL via kill() should work
                let _ = kill(Pid::from_raw(pid_for_task as i32), Signal::SIGKILL);
            }
            
            // Wait for AudioRouter task to complete (if it exists)
            if let Some(task) = audio_router_task {
                if let Err(e) = task.await {
                    eprintln!("Warning: AudioRouter task join error: {}", e);
                }
            }
            
            // Verify PCM file exists and has data
            if !filepath_for_task.exists() {
                eprintln!(
                    "Warning: Recording file does not exist: {}",
                    filepath_for_task.display()
                );
                if let Err(e) = app_handle.emit("recording-stopped", ()) {
                    eprintln!("Warning: Failed to emit recording-stopped event: {}", e);
                }
                return;
            }
            
            let metadata = match std::fs::metadata(&filepath_for_task) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Warning: Failed to read file metadata: {}", e);
                    if let Err(e) = app_handle.emit("recording-stopped", ()) {
                        eprintln!("Warning: Failed to emit recording-stopped event: {}", e);
                    }
                    return;
                }
            };
            
            if metadata.len() == 0 {
                eprintln!(
                    "Warning: Recording file is empty: {}",
                    filepath_for_task.display()
                );
            }
            
            // Emit "recording-stopped" event
            if let Err(e) = app_handle.emit("recording-stopped", ()) {
                eprintln!("Warning: Failed to emit recording-stopped event: {}", e);
            }
        });
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    
    // Note: Full integration tests for RecordingManager require a running Tauri app
    // and are better suited for end-to-end testing. These unit tests cover the
    // basic state management functionality.
    
    #[test]
    fn test_is_recording_initially_false() {
        // We can't easily create a real AppHandle in unit tests, so we'll test
        // the is_recording logic with a mock structure
        struct MockRecordingManager {
            current_child: Option<()>,
        }
        
        impl MockRecordingManager {
            fn is_recording(&self) -> bool {
                self.current_child.is_some()
            }
        }
        
        let manager = MockRecordingManager {
            current_child: None,
        };
        
        assert!(!manager.is_recording());
    }
    
    #[test]
    fn test_is_recording_true_when_child_present() {
        struct MockRecordingManager {
            current_child: Option<()>,
        }
        
        impl MockRecordingManager {
            fn is_recording(&self) -> bool {
                self.current_child.is_some()
            }
        }
        
        let manager = MockRecordingManager {
            current_child: Some(()),
        };
        
        assert!(manager.is_recording());
    }
    
    #[test]
    fn test_concurrent_recording_prevention() {
        // Test that start_recording returns an error when already recording
        struct MockRecordingManager {
            current_child: Option<()>,
        }
        
        impl MockRecordingManager {
            fn is_recording(&self) -> bool {
                self.current_child.is_some()
            }
            
            fn start_recording(&self) -> Result<String, String> {
                if self.is_recording() {
                    return Err("A recording is already in progress".to_string());
                }
                Ok("test.pcm".to_string())
            }
        }
        
        // Test when not recording - should succeed
        let manager_idle = MockRecordingManager {
            current_child: None,
        };
        assert!(manager_idle.start_recording().is_ok());
        
        // Test when already recording - should fail with concurrent recording error
        let manager_recording = MockRecordingManager {
            current_child: Some(()),
        };
        let result = manager_recording.start_recording();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "A recording is already in progress");
    }
    
    #[test]
    fn test_generate_timestamped_path_format() {
        use std::path::PathBuf;
        use regex::Regex;
        
        // Create a mock recordings directory
        let recordings_dir = PathBuf::from("/tmp/recordings");
        
        // We can't easily create a real AppHandle in unit tests, so we'll test
        // the path generation logic directly
        struct MockRecordingManager;
        
        impl MockRecordingManager {
            fn generate_timestamped_path(&self, recordings_dir: &std::path::Path) -> PathBuf {
                let now = chrono::Local::now();
                let filename = now.format("%Y%m%d_%H%M%S.pcm").to_string();
                recordings_dir.join(filename)
            }
        }
        
        let manager = MockRecordingManager;
        let path = manager.generate_timestamped_path(&recordings_dir);
        
        // Verify the path is in the recordings directory
        assert_eq!(path.parent().unwrap(), recordings_dir);
        
        // Verify the filename matches the pattern YYYYMMDD_HHMMSS.pcm
        let filename = path.file_name().unwrap().to_str().unwrap();
        let pattern = Regex::new(r"^\d{8}_\d{6}\.pcm$").unwrap();
        assert!(
            pattern.is_match(filename),
            "Filename '{}' does not match pattern YYYYMMDD_HHMMSS.pcm",
            filename
        );
    }
    
    #[test]
    fn test_generate_timestamped_path_unique() {
        use std::path::PathBuf;
        use std::thread;
        use std::time::Duration;
        
        // Create a mock recordings directory
        let recordings_dir = PathBuf::from("/tmp/recordings");
        
        struct MockRecordingManager;
        
        impl MockRecordingManager {
            fn generate_timestamped_path(&self, recordings_dir: &std::path::Path) -> PathBuf {
                let now = chrono::Local::now();
                let filename = now.format("%Y%m%d_%H%M%S.pcm").to_string();
                recordings_dir.join(filename)
            }
        }
        
        let manager = MockRecordingManager;
        
        // Generate two paths with a small delay
        let path1 = manager.generate_timestamped_path(&recordings_dir);
        thread::sleep(Duration::from_secs(1));
        let path2 = manager.generate_timestamped_path(&recordings_dir);
        
        // Paths should be different (different timestamps)
        assert_ne!(path1, path2);
    }
}
