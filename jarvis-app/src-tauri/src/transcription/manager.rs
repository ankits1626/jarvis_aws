// TranscriptionManager - Orchestrates the transcription lifecycle
// Receives PCM chunks from AudioRouter, feeds to provider, emits Tauri events

use std::sync::Arc;
use tokio::sync::{mpsc, watch, Mutex as TokioMutex};
use tauri::{AppHandle, Emitter};
use serde_json::json;

use crate::transcription::provider::{TranscriptionProvider, TranscriptionSegment, TranscriptionStatus};
use crate::transcription::audio_buffer::AudioBuffer;

/// TranscriptionManager orchestrates the transcription lifecycle.
/// 
/// Responsibilities:
/// - Receive PCM chunks from mpsc channel (sent by AudioRouter)
/// - Own the TranscriptionProvider (Box<dyn TranscriptionProvider>)
/// - Spawn tokio background task for transcription loop
/// - Accumulate transcript in Vec<TranscriptionSegment>
/// - Emit Tauri events for each segment
/// - Handle stop signal and drain remaining audio
/// - Manage graceful shutdown
pub struct TranscriptionManager {
    provider: Arc<TokioMutex<Box<dyn TranscriptionProvider>>>,
    transcript: Arc<TokioMutex<Vec<TranscriptionSegment>>>,
    status: Arc<TokioMutex<TranscriptionStatus>>,
    stop_tx: Option<watch::Sender<bool>>,
    app_handle: AppHandle,
}

impl TranscriptionManager {
    /// Create a new TranscriptionManager
    /// 
    /// # Arguments
    /// * `provider` - The transcription provider (HybridProvider, etc.)
    /// * `app_handle` - Tauri app handle for emitting events
    pub fn new(
        provider: Box<dyn TranscriptionProvider>,
        app_handle: AppHandle,
    ) -> Self {
        Self {
            provider: Arc::new(TokioMutex::new(provider)),
            transcript: Arc::new(TokioMutex::new(Vec::new())),
            status: Arc::new(TokioMutex::new(TranscriptionStatus::Idle)),
            stop_tx: None,
            app_handle,
        }
    }
    
    /// Start transcription
    /// 
    /// Spawns a background task that:
    /// 1. Receives PCM chunks from mpsc channel
    /// 2. Accumulates chunks into audio windows
    /// 3. Transcribes windows with the provider
    /// 4. Emits transcription-update events for each segment
    /// 5. Handles stop signal and drains remaining audio
    /// 
    /// # Arguments
    /// * `rx` - mpsc receiver for PCM chunks from AudioRouter
    /// 
    /// # Returns
    /// * `Ok(())` - Transcription started successfully
    /// * `Err(String)` - Failed to start transcription
    pub async fn start(&mut self, mut rx: mpsc::Receiver<Vec<u8>>) -> Result<(), String> {
        // Set status to active
        *self.status.lock().await = TranscriptionStatus::Active;
        
        // Emit transcription-started event
        self.app_handle.emit("transcription-started", ())
            .map_err(|e| format!("Failed to emit transcription-started: {}", e))?;
        
        eprintln!("TranscriptionManager: Starting transcription");
        
        // Create stop signal channel
        let (stop_tx, mut stop_rx) = watch::channel(false);
        self.stop_tx = Some(stop_tx);
        
        // Clone Arc references for background task
        let provider = self.provider.clone();
        let transcript = self.transcript.clone();
        let status = self.status.clone();
        let app_handle = self.app_handle.clone();
        
        // Spawn background transcription task
        tokio::spawn(async move {
            let mut audio_buffer = AudioBuffer::new(3.0, 0.5, 16000);
            let mut total_chunks = 0usize;
            let mut total_segments = 0usize;
            let mut total_windows = 0usize;

            loop {
                tokio::select! {
                    biased; // Prioritize stop signal over receiving more chunks

                    // Check for stop signal
                    _ = stop_rx.changed() => {
                        if *stop_rx.borrow() {
                            eprintln!("TranscriptionManager: Stop signal received, draining remaining audio ({} bytes buffered)",
                                      audio_buffer.len());
                            break;
                        }
                    }

                    // Receive audio chunks
                    chunk_opt = rx.recv() => {
                        match chunk_opt {
                            Some(chunk) => {
                                total_chunks += 1;

                                // Push chunk to audio buffer
                                audio_buffer.push(&chunk);

                                // Extract windows and transcribe
                                while let Some(audio) = audio_buffer.extract_window() {
                                    total_windows += 1;
                                    eprintln!("TranscriptionManager: Window #{} extracted ({} f32 samples, {:.1}s)",
                                              total_windows, audio.len(), audio.len() as f32 / 16000.0);

                                    // Use block_in_place to run synchronous transcription
                                    // without blocking the tokio executor for other tasks
                                    let transcribe_result = tokio::task::block_in_place(|| {
                                        tokio::runtime::Handle::current().block_on(async {
                                            provider.lock().await.transcribe(&audio)
                                        })
                                    });

                                    let segments_or_err: Result<Vec<TranscriptionSegment>, String> =
                                        transcribe_result.map_err(|e| e.to_string());

                                    match segments_or_err {
                                        Ok(segments) => {
                                            for segment in segments {
                                                transcript.lock().await.push(segment.clone());
                                                total_segments += 1;

                                                // Emit transcription-update event
                                                if let Err(e) = app_handle.emit("transcription-update", &segment) {
                                                    eprintln!("TranscriptionManager: Warning - Failed to emit transcription-update: {}", e);
                                                }
                                            }
                                        }
                                        Err(err_msg) => {
                                            // Non-fatal error: log and emit error event, but continue processing
                                            eprintln!("TranscriptionManager: Transcription error: {}", err_msg);
                                            let _ = app_handle.emit("transcription-error", json!({ "message": err_msg }));
                                        }
                                    }
                                }
                            }
                            None => {
                                // Channel closed - AudioRouter finished or crashed
                                eprintln!("TranscriptionManager: Audio channel closed ({} bytes buffered)",
                                          audio_buffer.len());
                                break;
                            }
                        }
                    }
                }
            }

            // Drain remaining audio after loop exit
            if let Some(audio) = audio_buffer.drain_remaining(1.0) {
                eprintln!("TranscriptionManager: Draining final audio ({} f32 samples, {:.1}s)",
                          audio.len(), audio.len() as f32 / 16000.0);

                let transcribe_result = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        provider.lock().await.transcribe(&audio)
                    })
                });

                let segments_or_err: Result<Vec<TranscriptionSegment>, String> =
                    transcribe_result.map_err(|e| e.to_string());

                match segments_or_err {
                    Ok(segments) => {
                        for segment in segments {
                            transcript.lock().await.push(segment.clone());
                            total_segments += 1;
                            let _ = app_handle.emit("transcription-update", &segment);
                        }
                    }
                    Err(err_msg) => {
                        eprintln!("TranscriptionManager: Error transcribing final audio: {}", err_msg);
                    }
                }
            } else {
                eprintln!("TranscriptionManager: No remaining audio to drain (below 1s threshold)");
            }

            eprintln!("TranscriptionManager: Transcription completed. Processed {} chunks, {} windows, emitted {} segments",
                      total_chunks, total_windows, total_segments);

            // Emit transcription-stopped event with full transcript
            let final_transcript = transcript.lock().await.clone();
            if let Err(e) = app_handle.emit("transcription-stopped", json!({ "transcript": final_transcript })) {
                eprintln!("TranscriptionManager: Warning - Failed to emit transcription-stopped: {}", e);
            }

            // Set status to idle
            *status.lock().await = TranscriptionStatus::Idle;
        });
        
        Ok(())
    }
    
    /// Stop transcription
    /// 
    /// Signals the background task to stop processing and drain remaining audio.
    /// The task will emit a transcription-stopped event when complete.
    /// 
    /// # Returns
    /// * `Ok(())` - Stop signal sent successfully
    /// * `Err(String)` - Failed to send stop signal
    pub async fn stop(&mut self) -> Result<(), String> {
        if let Some(stop_tx) = &self.stop_tx {
            eprintln!("TranscriptionManager: Sending stop signal");
            stop_tx.send(true)
                .map_err(|e| format!("Failed to send stop signal: {}", e))?;
        } else {
            eprintln!("TranscriptionManager: Warning - stop() called but transcription not started");
        }
        Ok(())
    }
    
    /// Get the accumulated transcript
    /// 
    /// Returns a clone of all transcription segments accumulated so far.
    /// 
    /// # Returns
    /// * `Vec<TranscriptionSegment>` - All segments transcribed so far
    pub async fn get_transcript(&self) -> Vec<TranscriptionSegment> {
        self.transcript.lock().await.clone()
    }
    
    /// Get the current transcription status
    /// 
    /// # Returns
    /// * `TranscriptionStatus` - Current status (Idle, Active, Error, Disabled)
    pub async fn get_status(&self) -> TranscriptionStatus {
        *self.status.lock().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::provider::TranscriptionConfig;
    use std::error::Error;
    
    // Mock provider for testing
    struct MockProvider {
        segments: Vec<TranscriptionSegment>,
    }
    
    impl MockProvider {
        fn new() -> Self {
            Self {
                segments: vec![
                    TranscriptionSegment {
                        text: "test segment".to_string(),
                        start_ms: 0,
                        end_ms: 1000,
                        is_final: true,
                    }
                ],
            }
        }
    }
    
    impl TranscriptionProvider for MockProvider {
        fn name(&self) -> &str {
            "mock-provider"
        }
        
        fn initialize(&mut self, _config: &TranscriptionConfig) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
        
        fn transcribe(&mut self, _audio: &[f32]) -> Result<Vec<TranscriptionSegment>, Box<dyn Error>> {
            Ok(self.segments.clone())
        }
    }
    
    #[tokio::test]
    async fn test_transcription_manager_creation() {
        // Note: This test requires a Tauri AppHandle which is not available in unit tests
        // In practice, TranscriptionManager is tested via integration tests with the full Tauri app
        // This test is a placeholder to demonstrate the structure
    }
    
    #[tokio::test]
    async fn test_transcription_manager_status() {
        // Placeholder test - requires Tauri AppHandle
    }
}
