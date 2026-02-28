// RecordingChatSource — Recording Conforms to Chatable
//
// This module makes recordings chatbot-compatible by implementing the Chatable trait.
// It handles transcript loading from disk (fast path) or generation via IntelQueue
// (slow path), and persists generated transcripts for reuse.

use async_trait::async_trait;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

use super::chatable::Chatable;
use crate::intelligence::queue::{IntelCommand, IntelQueue, IntelResponse};
use crate::wav::WavConverter;

/// A recording that can be chatted with
pub struct RecordingChatSource {
    app_handle: AppHandle,
    filename: String,
    recordings_dir: PathBuf,
}

impl RecordingChatSource {
    /// Create a new RecordingChatSource
    /// 
    /// # Arguments
    /// 
    /// * `app_handle` - Tauri app handle for emitting events
    /// * `filename` - Recording filename (e.g. "20260228_143022.pcm")
    /// 
    /// # Returns
    /// 
    /// A new RecordingChatSource instance
    /// 
    /// # Errors
    /// 
    /// Returns an error if the recordings directory cannot be determined
    pub fn new(app_handle: AppHandle, filename: String) -> Result<Self, String> {
        let recordings_dir = get_recordings_dir(&app_handle)?;
        Ok(Self {
            app_handle,
            filename,
            recordings_dir,
        })
    }

    /// Get the recording stem (filename without .pcm extension)
    fn stem(&self) -> String {
        self.filename.trim_end_matches(".pcm").to_string()
    }

    /// Per-recording folder: recordings/{stem}/
    fn recording_dir(&self) -> PathBuf {
        self.recordings_dir.join(self.stem())
    }

    /// Transcript lives inside the per-recording folder
    pub fn transcript_path(&self) -> PathBuf {
        self.recording_dir().join("transcript.md")
    }
}

#[async_trait]
impl Chatable for RecordingChatSource {
    async fn get_context(&self, intel_queue: &IntelQueue) -> Result<String, String> {
        let transcript_path = self.transcript_path();

        // Fast path: transcript exists on disk
        if transcript_path.exists() {
            return tokio::fs::read_to_string(&transcript_path).await
                .map_err(|e| format!("Failed to read transcript: {}", e));
        }

        // Slow path: generate transcript
        self.on_preparation_status("preparing", "Generating transcript...");

        // Convert PCM to WAV
        let pcm_path = self.recordings_dir.join(&self.filename);
        let wav_data = WavConverter::pcm_to_wav(&pcm_path)?;

        // Write WAV to temporary file for transcription
        let temp_dir = std::env::temp_dir();
        let wav_filename = format!("{}.wav", self.stem());
        let wav_path = temp_dir.join(wav_filename);
        
        tokio::fs::write(&wav_path, wav_data).await
            .map_err(|e| format!("Failed to write temporary WAV file: {}", e))?;

        // Submit transcript generation request
        let response = intel_queue.submit(IntelCommand::GenerateTranscript {
            audio_path: wav_path.clone(),
        }).await?;

        // Clean up temporary WAV file
        let _ = tokio::fs::remove_file(&wav_path).await;

        // Extract transcript from response
        let transcript = match response {
            IntelResponse::Transcript(result) => result.transcript,
            _ => return Err("Unexpected response type from transcript generation".into()),
        };

        // Persist transcript for reuse
        let transcript_md = format!(
            "# Transcript — {}\n\n**Generated:** {}\n\n---\n\n{}",
            self.stem(),
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            transcript,
        );

        // Ensure per-recording directory exists before writing
        let _ = tokio::fs::create_dir_all(self.recording_dir()).await;

        // Write transcript file (ignore errors — we still have the transcript in memory)
        let _ = tokio::fs::write(&transcript_path, &transcript_md).await;

        self.on_preparation_status("ready", "Ready to chat");
        Ok(transcript)
    }

    fn label(&self) -> String {
        format!("Recording {}", self.stem())
    }

    fn session_dir(&self) -> PathBuf {
        self.recording_dir()
    }

    async fn needs_preparation(&self) -> bool {
        !self.transcript_path().exists()
    }

    fn on_preparation_status(&self, status: &str, message: &str) {
        let _ = self.app_handle.emit("chat-status", serde_json::json!({
            "status": status,
            "message": message,
        }));
    }
}

/// Get the recordings directory path
/// 
/// Uses the same logic as FileManager to determine the platform-specific
/// recordings directory.
/// 
/// # Arguments
/// 
/// * `app_handle` - Tauri app handle (unused, but kept for consistency)
/// 
/// # Returns
/// 
/// The path to the recordings directory
/// 
/// # Errors
/// 
/// Returns an error if the app data directory cannot be determined
fn get_recordings_dir(_app_handle: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = dirs::data_dir()
        .ok_or_else(|| "Failed to determine app data directory".to_string())?;
    
    let recordings_dir = app_data_dir.join("com.jarvis.app").join("recordings");
    
    Ok(recordings_dir)
}
