// Co-Pilot Agent — Live Recording Intelligence
//
// This module implements the Co-Pilot agent that runs alongside audio recording
// and produces real-time actionable insights by analyzing audio chunks with Qwen Omni.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex as TokioMutex, watch};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::files::{SAMPLE_RATE, BYTES_PER_SAMPLE, CHANNELS};
use crate::wav::WavConverter;
use crate::intelligence::provider::IntelProvider;

/// Co-Pilot agent state containing all analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoPilotState {
    /// Rolling summary of the entire conversation
    pub running_summary: String,
    
    /// Key points mentioned in the conversation
    pub key_points: Vec<String>,
    
    /// Decisions made during the conversation
    pub decisions: Vec<String>,
    
    /// Action items identified
    pub action_items: Vec<String>,
    
    /// Open questions raised
    pub open_questions: Vec<String>,
    
    /// Suggested questions to ask next (max 5)
    pub suggested_questions: Vec<SuggestedQuestion>,
    
    /// Key concepts (technical terms, names, topics)
    pub key_concepts: Vec<KeyConcept>,
    
    /// Metadata about cycle execution
    pub cycle_metadata: CycleMetadata,
}

/// A suggested question with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedQuestion {
    /// The question text
    pub question: String,
    
    /// Reason why this question is suggested
    pub reason: String,
    
    /// Cycle number when this question was added
    pub cycle_added: u32,
    
    /// Whether the user has dismissed this question
    pub dismissed: bool,
}

/// A key concept mentioned in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyConcept {
    /// The term or concept
    pub term: String,
    
    /// Brief context or explanation
    pub context: String,
    
    /// Cycle number when this concept was first mentioned
    pub cycle_added: u32,
    
    /// Number of times this concept has been mentioned
    pub mention_count: u32,
}

/// Metadata about cycle execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleMetadata {
    /// Current cycle number (0 = not started)
    pub cycle_number: u32,
    
    /// ISO 8601 timestamp of last update
    pub last_updated_at: String,
    
    /// Whether a cycle is currently processing
    pub processing: bool,
    
    /// Number of cycles that failed
    pub failed_cycles: u32,
    
    /// Total seconds of audio analyzed
    pub total_audio_seconds: u64,
}

impl Default for CoPilotState {
    fn default() -> Self {
        Self {
            running_summary: String::new(),
            key_points: Vec::new(),
            decisions: Vec::new(),
            action_items: Vec::new(),
            open_questions: Vec::new(),
            suggested_questions: Vec::new(),
            key_concepts: Vec::new(),
            cycle_metadata: CycleMetadata {
                cycle_number: 0,
                last_updated_at: String::new(),
                processing: false,
                failed_cycles: 0,
                total_audio_seconds: 0,
            },
        }
    }
}

/// Result of a single cycle execution (for logging purposes)
struct CycleExecutionResult {
    audio_duration: u64,
    response_summary: String, // Brief summary of response for logging
}

/// Run the Co-Pilot agent cycle loop
/// 
/// This function runs in a background tokio task and executes analysis cycles
/// at the configured interval until a stop signal is received.
/// 
/// # Arguments
/// 
/// * `provider` - The intelligence provider for analysis
/// * `recording_filepath` - Path to the active PCM recording file
/// * `settings` - Co-Pilot settings
/// * `state` - Shared agent state
/// * `app_handle` - Tauri app handle for emitting events
/// * `stop_rx` - Receiver for stop signal
async fn run_cycle_loop(
    provider: Arc<dyn IntelProvider>,
    recording_filepath: PathBuf,
    settings: crate::settings::CoPilotSettings,
    state: Arc<TokioMutex<CoPilotState>>,
    app_handle: AppHandle,
    mut stop_rx: watch::Receiver<bool>,
) {
    // Initialize cycle tracking
    let mut consecutive_failures = 0u32;
    let cycle_interval = std::time::Duration::from_secs(settings.cycle_interval);
    
    // Create log file if logging is enabled
    let recording_filename = recording_filepath
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.pcm");
    
    let log_path = CoPilotAgent::create_log_file(
        recording_filename,
        &settings,
        "Qwen 2.5 Omni 3B", // TODO: Get actual model name from provider
    ).await;
    
    // Main cycle loop
    loop {
        // Check for stop signal (non-blocking)
        if *stop_rx.borrow() {
            break;
        }
        
        // Mark as processing
        {
            let mut state_guard = state.lock().await;
            state_guard.cycle_metadata.processing = true;
        }
        
        // Emit status event
        let _ = app_handle.emit("copilot-status", serde_json::json!({
            "status": "processing",
        }));
        
        // Run single cycle
        let cycle_start = std::time::Instant::now();
        let cycle_result = run_single_cycle(
            &provider,
            &recording_filepath,
            &settings,
            &state,
        ).await;
        let cycle_duration = cycle_start.elapsed();
        
        // Handle cycle result
        match cycle_result {
            Ok(exec_result) => {
                // Successful cycle - reset failure counter
                consecutive_failures = 0;
                
                // Emit updated event
                let state_guard = state.lock().await;
                let _ = app_handle.emit("copilot-updated", state_guard.clone());
                
                // Log cycle if enabled
                if let Some(ref log_path) = log_path {
                    // Calculate audio timestamps (approximate)
                    let total_audio_secs = state_guard.cycle_metadata.total_audio_seconds;
                    let audio_start_secs = total_audio_secs.saturating_sub(exec_result.audio_duration);
                    let audio_start = format!("{}:{:02}", audio_start_secs / 60, audio_start_secs % 60);
                    let audio_end = format!("{}:{:02}", total_audio_secs / 60, total_audio_secs % 60);
                    
                    let _ = CoPilotAgent::log_cycle(
                        log_path,
                        state_guard.cycle_metadata.cycle_number,
                        &audio_start,
                        &audio_end,
                        exec_result.audio_duration,
                        settings.audio_overlap,
                        "[Prompt generated in Python sidecar]",
                        &exec_result.response_summary,
                        cycle_duration.as_secs_f64(),
                        "success",
                    ).await;
                }
            }
            Err(error) => {
                // Failed cycle - increment failure counters
                consecutive_failures += 1;
                
                // Increment failed_cycles in state metadata
                {
                    let mut state_guard = state.lock().await;
                    state_guard.cycle_metadata.failed_cycles += 1;
                }
                
                // Emit error event
                let cycle_number = {
                    let state_guard = state.lock().await;
                    state_guard.cycle_metadata.cycle_number
                };
                
                let _ = app_handle.emit("copilot-error", serde_json::json!({
                    "cycle": cycle_number,
                    "error": error,
                }));
                
                // Log cycle failure if enabled
                if let Some(ref log_path) = log_path {
                    let _ = CoPilotAgent::log_cycle(
                        log_path,
                        cycle_number,
                        "00:00",
                        "00:00",
                        0,
                        settings.audio_overlap,
                        "[N/A — error occurred before prompt]",
                        &error,
                        cycle_duration.as_secs_f64(),
                        "error",
                    ).await;
                }
                
                // Stop after 3 consecutive failures
                if consecutive_failures >= 3 {
                    let _ = app_handle.emit("copilot-status", serde_json::json!({
                        "status": "error",
                        "message": "Agent paused after 3 consecutive failures",
                    }));
                    break;
                }
            }
        }
        
        // Mark as not processing
        {
            let mut state_guard = state.lock().await;
            state_guard.cycle_metadata.processing = false;
        }
        
        // Sleep until next cycle (or stop signal)
        tokio::select! {
            _ = tokio::time::sleep(cycle_interval) => {},
            _ = stop_rx.changed() => {
                if *stop_rx.borrow() {
                    break;
                }
            }
        }
    }
    
    // Write log summary if logging was enabled
    if let Some(log_path) = log_path {
        let state_guard = state.lock().await;
        let total_cycles = state_guard.cycle_metadata.cycle_number;
        let failed_cycles = state_guard.cycle_metadata.failed_cycles;
        let successful_cycles = total_cycles.saturating_sub(failed_cycles);
        
        let _ = CoPilotAgent::write_log_summary(
            &log_path,
            total_cycles,
            successful_cycles,
            0, // skipped cycles (not tracked separately)
            failed_cycles,
            0.0, // avg inference time (TODO: track this)
            "0m 0s", // total duration (TODO: calculate this)
        ).await;
    }
    
    // Emit final status
    let _ = app_handle.emit("copilot-status", serde_json::json!({
        "status": "stopped",
    }));
}

/// Run a single analysis cycle
/// 
/// Extracts audio chunk, calls provider for analysis, updates state.
/// 
/// # Arguments
/// 
/// * `provider` - The intelligence provider
/// * `recording_filepath` - Path to the PCM recording file
/// * `settings` - Co-Pilot settings
/// * `state` - Shared agent state
/// 
/// # Returns
/// 
/// `Ok(CycleExecutionResult)` on success, `Err(error_message)` on failure
async fn run_single_cycle(
    provider: &Arc<dyn IntelProvider>,
    recording_filepath: &Path,
    settings: &crate::settings::CoPilotSettings,
    state: &Arc<TokioMutex<CoPilotState>>,
) -> Result<CycleExecutionResult, String> {
    // Extract audio chunk
    let temp_path = CoPilotAgent::extract_audio_chunk(
        recording_filepath,
        settings.cycle_interval,
        settings.audio_overlap,
    ).await?;
    
    // Get running context from state
    let context = {
        let state_guard = state.lock().await;
        if state_guard.cycle_metadata.cycle_number == 0 {
            String::new() // First cycle - empty context
        } else {
            state_guard.running_summary.clone()
        }
    };
    
    // Call provider with timeout
    let analysis_result = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        provider.copilot_analyze(&temp_path, &context),
    ).await;
    
    // Clean up temp file
    CoPilotAgent::cleanup_temp_file(&temp_path).await;
    
    // Handle timeout
    let cycle_result = match analysis_result {
        Ok(result) => result,
        Err(_) => return Err("Analysis timed out after 120 seconds".to_string()),
    }?;
    
    // Calculate audio duration
    let audio_duration = settings.cycle_interval + settings.audio_overlap;
    
    // Create response summary for logging (truncate if too long)
    let response_summary = if cycle_result.updated_summary.len() > 200 {
        format!("{}...", &cycle_result.updated_summary[..200])
    } else {
        cycle_result.updated_summary.clone()
    };
    
    // Update state using the update_state_internal helper
    update_state_internal(state, cycle_result, audio_duration).await;
    
    Ok(CycleExecutionResult {
        audio_duration,
        response_summary,
    })
}

/// Internal helper to update state (used by run_single_cycle)
async fn update_state_internal(
    state: &Arc<TokioMutex<CoPilotState>>,
    result: crate::intelligence::provider::CoPilotCycleResult,
    audio_duration_seconds: u64,
) {
    let mut state_guard = state.lock().await;
    
    // Replace summary with latest
    state_guard.running_summary = result.updated_summary;
    
    // Append new items, deduplicate
    for point in result.key_points {
        if !state_guard.key_points.contains(&point) {
            state_guard.key_points.push(point);
        }
    }
    
    for decision in result.decisions {
        if !state_guard.decisions.contains(&decision) {
            state_guard.decisions.push(decision);
        }
    }
    
    for item in result.action_items {
        if !state_guard.action_items.contains(&item) {
            state_guard.action_items.push(item);
        }
    }
    
    for question in result.open_questions {
        if !state_guard.open_questions.contains(&question) {
            state_guard.open_questions.push(question);
        }
    }
    
    // Replace suggested questions (keep max 5, preserve dismissed state)
    let next_cycle_number = state_guard.cycle_metadata.cycle_number + 1;
    let mut new_questions = Vec::new();
    for new_q in result.suggested_questions {
        // Check if this question was previously dismissed
        let was_dismissed = state_guard.suggested_questions.iter()
            .any(|old_q| old_q.question == new_q.question && old_q.dismissed);
        
        new_questions.push(SuggestedQuestion {
            question: new_q.question,
            reason: new_q.reason,
            cycle_added: next_cycle_number,
            dismissed: was_dismissed,
        });
    }
    state_guard.suggested_questions = new_questions.into_iter().take(5).collect();
    
    // Merge key concepts (increment mention_count for existing)
    for new_concept in result.key_concepts {
        if let Some(existing) = state_guard.key_concepts.iter_mut()
            .find(|c| c.term.eq_ignore_ascii_case(&new_concept.term)) {
            existing.mention_count += 1;
            // Update context with latest
            existing.context = new_concept.context;
        } else {
            state_guard.key_concepts.push(KeyConcept {
                term: new_concept.term,
                context: new_concept.context,
                cycle_added: next_cycle_number,
                mention_count: 1,
            });
        }
    }
    
    // Update metadata
    state_guard.cycle_metadata.cycle_number = next_cycle_number;
    state_guard.cycle_metadata.last_updated_at = chrono::Utc::now().to_rfc3339();
    state_guard.cycle_metadata.processing = false;
    state_guard.cycle_metadata.total_audio_seconds += audio_duration_seconds;
}

/// Co-Pilot agent that analyzes audio during live recording
pub struct CoPilotAgent {
    app_handle: AppHandle,
    state: Arc<TokioMutex<CoPilotState>>,
    /// Handle to the background cycle loop task
    cycle_task: Option<tokio::task::JoinHandle<()>>,
    /// Sender for stop signal to cycle loop
    stop_tx: Option<watch::Sender<bool>>,
}

impl CoPilotAgent {
    /// Create a new Co-Pilot agent instance
    /// 
    /// Creates an agent in the stopped state. Call `start()` to begin the cycle loop.
    /// 
    /// # Arguments
    /// 
    /// * `app_handle` - Handle to the Tauri application for emitting events
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use tauri::AppHandle;
    /// use jarvis_app_lib::agents::copilot::CoPilotAgent;
    /// 
    /// fn create_agent(app_handle: AppHandle) {
    ///     let agent = CoPilotAgent::new(app_handle);
    /// }
    /// ```
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            state: Arc::new(TokioMutex::new(CoPilotState::default())),
            cycle_task: None,
            stop_tx: None,
        }
    }
    
    /// Create log file and write header
    /// 
    /// Creates the agent_logs directory if it doesn't exist, generates a timestamped
    /// log filename, and writes the log header with recording info, settings, and model name.
    /// 
    /// # Arguments
    /// 
    /// * `recording_filename` - Name of the recording file (e.g., "20240315_143022.pcm")
    /// * `settings` - Co-Pilot settings
    /// * `model_name` - Name of the model being used
    /// 
    /// # Returns
    /// 
    /// Path to the created log file, or None if logging is disabled or creation fails
    async fn create_log_file(
        recording_filename: &str,
        settings: &crate::settings::CoPilotSettings,
        model_name: &str,
    ) -> Option<PathBuf> {
        // Only create log if logging is enabled
        if !settings.agent_logging {
            return None;
        }
        
        // Get log directory path
        let log_dir = dirs::home_dir()?
            .join("Library/Application Support/com.jarvis.app/agent_logs");
        
        // Create directory if it doesn't exist
        if let Err(e) = tokio::fs::create_dir_all(&log_dir).await {
            eprintln!("Warning: Failed to create agent_logs directory: {}", e);
            return None;
        }
        
        // Generate log filename with timestamp matching recording
        // Extract timestamp from recording filename (YYYYMMDD_HHMMSS.pcm)
        let timestamp_str = recording_filename
            .strip_suffix(".pcm")
            .unwrap_or(recording_filename);
        let log_filename = format!("{}_copilot.md", timestamp_str);
        let log_path = log_dir.join(log_filename);
        
        // Write log header
        if let Err(e) = Self::write_log_header(&log_path, recording_filename, settings, model_name).await {
            eprintln!("Warning: Failed to write log header: {}", e);
            return None;
        }
        
        Some(log_path)
    }
    
    /// Write log header to file
    /// 
    /// # Arguments
    /// 
    /// * `log_path` - Path to the log file
    /// * `recording_filename` - Name of the recording file
    /// * `settings` - Co-Pilot settings
    /// * `model_name` - Name of the model being used
    async fn write_log_header(
        log_path: &Path,
        recording_filename: &str,
        settings: &crate::settings::CoPilotSettings,
        model_name: &str,
    ) -> Result<(), String> {
        use tokio::io::AsyncWriteExt;
        
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        
        let header = format!(
            r#"# Co-Pilot Agent Log — {}

**Recording:** {}
**Settings:** cycle_interval={}s, audio_overlap={}s
**Model:** {}

"#,
            timestamp,
            recording_filename,
            settings.cycle_interval,
            settings.audio_overlap,
            model_name
        );
        
        let mut file = File::create(log_path)
            .await
            .map_err(|e| format!("Failed to create log file: {}", e))?;
        
        file.write_all(header.as_bytes())
            .await
            .map_err(|e| format!("Failed to write log header: {}", e))?;
        
        Ok(())
    }
    
    /// Append cycle entry to log file
    /// 
    /// Writes a cycle entry with audio chunk info, inference time, status, prompt, and response.
    /// 
    /// # Arguments
    /// 
    /// * `log_path` - Path to the log file
    /// * `cycle_number` - Current cycle number
    /// * `audio_start` - Start time of audio chunk (e.g., "00:00")
    /// * `audio_end` - End time of audio chunk (e.g., "01:05")
    /// * `audio_duration` - Duration of audio chunk in seconds
    /// * `audio_overlap` - Overlap duration in seconds
    /// * `prompt` - The prompt sent to the model
    /// * `response` - The JSON response from the model
    /// * `inference_time` - Time taken for inference in seconds
    /// * `status` - Status of the cycle ("success", "error", "skipped")
    async fn log_cycle(
        log_path: &Path,
        cycle_number: u32,
        audio_start: &str,
        audio_end: &str,
        audio_duration: u64,
        audio_overlap: u64,
        prompt: &str,
        response: &str,
        inference_time: f64,
        status: &str,
    ) -> Result<(), String> {
        use tokio::fs::OpenOptions;
        use tokio::io::AsyncWriteExt;
        
        let entry = format!(
            r#"---

## Cycle {} — {} → {}

**Audio chunk:** {}–{} ({}s, includes {}s overlap)
**Inference time:** {:.1}s
**Status:** {}

### Prompt
{}

### Response
```json
{}
```

"#,
            cycle_number,
            audio_start,
            audio_end,
            audio_start,
            audio_end,
            audio_duration,
            audio_overlap,
            inference_time,
            status,
            prompt,
            response
        );
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .await
            .map_err(|e| format!("Failed to open log file: {}", e))?;
        
        file.write_all(entry.as_bytes())
            .await
            .map_err(|e| format!("Failed to write to log file: {}", e))?;
        
        Ok(())
    }
    
    /// Write summary section to log file
    /// 
    /// Appends a summary table with cycle statistics when the agent stops.
    /// 
    /// # Arguments
    /// 
    /// * `log_path` - Path to the log file
    /// * `total_cycles` - Total number of cycles attempted
    /// * `successful_cycles` - Number of successful cycles
    /// * `skipped_cycles` - Number of skipped cycles
    /// * `error_cycles` - Number of cycles that errored
    /// * `avg_inference_time` - Average inference time in seconds
    /// * `total_duration` - Total recording duration (e.g., "5m 22s")
    async fn write_log_summary(
        log_path: &Path,
        total_cycles: u32,
        successful_cycles: u32,
        skipped_cycles: u32,
        error_cycles: u32,
        avg_inference_time: f64,
        total_duration: &str,
    ) -> Result<(), String> {
        use tokio::fs::OpenOptions;
        use tokio::io::AsyncWriteExt;
        
        let summary = format!(
            r#"---

## Summary

| Metric | Value |
|---|---|
| Total cycles | {} |
| Successful | {} |
| Skipped | {} |
| Errors | {} |
| Avg inference time | {:.1}s |
| Total recording duration | {} |
"#,
            total_cycles,
            successful_cycles,
            skipped_cycles,
            error_cycles,
            avg_inference_time,
            total_duration
        );
        
        let mut file = OpenOptions::new()
            .append(true)
            .open(log_path)
            .await
            .map_err(|e| format!("Failed to open log file: {}", e))?;
        
        file.write_all(summary.as_bytes())
            .await
            .map_err(|e| format!("Failed to write log summary: {}", e))?;
        
        Ok(())
    }
    
    /// Clean up a temporary audio file
    /// 
    /// Attempts to delete the temporary file. Logs a warning if cleanup fails
    /// but does not return an error (cleanup failure should not block the agent).
    /// 
    /// # Arguments
    /// 
    /// * `temp_path` - Path to the temporary file to delete
    async fn cleanup_temp_file(temp_path: &Path) {
        if let Err(e) = tokio::fs::remove_file(temp_path).await {
            eprintln!(
                "Warning: Failed to clean up temp file {:?}: {}",
                temp_path, e
            );
        }
    }
    
    /// Extract audio chunk from recording file
    /// 
    /// Reads the last (cycle_interval + audio_overlap) seconds of audio from the
    /// recording file, converts it to WAV format, and writes it to a temporary file.
    /// 
    /// The caller is responsible for cleaning up the temporary file using
    /// `cleanup_temp_file()` after use.
    /// 
    /// # Arguments
    /// 
    /// * `recording_filepath` - Path to the PCM recording file
    /// * `cycle_interval` - Duration of the cycle in seconds
    /// * `audio_overlap` - Duration of overlap with previous cycle in seconds
    /// 
    /// # Returns
    /// 
    /// Path to the temporary WAV file containing the audio chunk
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - The recording file cannot be opened or read
    /// - WAV conversion fails
    /// - The temporary file cannot be written
    async fn extract_audio_chunk(
        recording_filepath: &Path,
        cycle_interval: u64,
        audio_overlap: u64,
    ) -> Result<PathBuf, String> {
        // Calculate chunk duration in seconds
        let chunk_duration = cycle_interval + audio_overlap;
        
        // Calculate byte size for chunk
        // PCM format: 16kHz, 16-bit (2 bytes), mono
        let bytes_per_second = (SAMPLE_RATE * BYTES_PER_SAMPLE * CHANNELS) as u64;
        let chunk_size_bytes = chunk_duration * bytes_per_second;
        
        // Open recording file
        let mut file = File::open(recording_filepath)
            .await
            .map_err(|e| format!("Failed to open recording file: {}", e))?;
        
        let file_size = file
            .metadata()
            .await
            .map_err(|e| format!("Failed to get file metadata: {}", e))?
            .len();
        
        // Handle case where file is shorter than chunk size (first cycle)
        let read_size = std::cmp::min(chunk_size_bytes, file_size);
        let start_offset = file_size.saturating_sub(read_size);
        
        // Read chunk from end of file
        file.seek(SeekFrom::Start(start_offset))
            .await
            .map_err(|e| format!("Failed to seek in recording file: {}", e))?;
        
        let mut chunk_data = vec![0u8; read_size as usize];
        file.read_exact(&mut chunk_data)
            .await
            .map_err(|e| format!("Failed to read audio chunk: {}", e))?;
        
        // Generate unique temp filename
        let temp_path = std::env::temp_dir()
            .join(format!("jarvis_copilot_chunk_{}.wav", Uuid::new_v4()));
        
        // Convert PCM to WAV using existing WavConverter from wav module
        let wav_data = WavConverter::from_pcm_bytes(&chunk_data)?;
        
        // Write to temp file
        let mut temp_file = File::create(&temp_path)
            .await
            .map_err(|e| format!("Failed to create temp file: {}", e))?;
        
        temp_file
            .write_all(&wav_data)
            .await
            .map_err(|e| format!("Failed to write temp file: {}", e))?;
        
        Ok(temp_path)
    }
    
    /// Start the Co-Pilot agent cycle loop
    /// 
    /// Spawns a background tokio task that runs the agent cycle loop. The loop will
    /// continue until `stop()` is called or the recording ends.
    /// 
    /// # Arguments
    /// 
    /// * `provider` - The intelligence provider to use for analysis
    /// * `recording_filepath` - Path to the active PCM recording file
    /// * `settings` - Co-Pilot settings (cycle interval, audio overlap, logging)
    /// 
    /// # Returns
    /// 
    /// `Ok(())` if the agent started successfully, or `Err(String)` if:
    /// - The agent is already running
    /// - The recording file doesn't exist
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use std::sync::Arc;
    /// use std::path::PathBuf;
    /// use jarvis_app_lib::agents::copilot::CoPilotAgent;
    /// use jarvis_app_lib::intelligence::provider::IntelProvider;
    /// use jarvis_app_lib::settings::CoPilotSettings;
    /// 
    /// async fn start_agent(
    ///     agent: &mut CoPilotAgent,
    ///     provider: Arc<dyn IntelProvider>,
    ///     recording_path: PathBuf,
    ///     settings: CoPilotSettings,
    /// ) -> Result<(), String> {
    ///     agent.start(provider, recording_path, settings).await
    /// }
    /// ```
    pub async fn start(
        &mut self,
        provider: Arc<dyn IntelProvider>,
        recording_filepath: PathBuf,
        settings: crate::settings::CoPilotSettings,
    ) -> Result<(), String> {
        // Check if agent is already running
        if self.cycle_task.is_some() {
            return Err("Co-Pilot agent is already running".to_string());
        }
        
        // Verify recording file exists
        if !tokio::fs::try_exists(&recording_filepath).await.unwrap_or(false) {
            return Err("Recording file does not exist".to_string());
        }
        
        // Emit starting status event (Requirement 6.2)
        let _ = self.app_handle.emit("copilot-status", serde_json::json!({
            "status": "starting",
        }));
        
        // Create stop signal channel
        let (stop_tx, stop_rx) = watch::channel(false);
        
        // Clone necessary data for the background task
        let state = Arc::clone(&self.state);
        let app_handle = self.app_handle.clone();
        
        // Spawn background task for cycle loop
        let cycle_task = tokio::spawn(async move {
            run_cycle_loop(
                provider,
                recording_filepath,
                settings,
                state,
                app_handle,
                stop_rx,
            ).await;
        });
        
        // Store task handle and stop sender
        self.cycle_task = Some(cycle_task);
        self.stop_tx = Some(stop_tx);
        
        Ok(())
    }
    
    /// Stop the Co-Pilot agent cycle loop
    /// 
    /// Sends a stop signal to the background task and waits for it to complete.
    /// If an inference is in progress, it will complete before stopping (up to 120s timeout).
    /// 
    /// # Returns
    /// 
    /// The final agent state after stopping
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use jarvis_app_lib::agents::copilot::CoPilotAgent;
    /// 
    /// async fn stop_agent(agent: &mut CoPilotAgent) {
    ///     let final_state = agent.stop().await;
    ///     println!("Agent stopped. Final cycle: {}", final_state.cycle_metadata.cycle_number);
    /// }
    /// ```
    pub async fn stop(&mut self) -> CoPilotState {
        // Send stop signal if we have a sender
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(true);
        }
        
        // Wait for cycle task to complete (with timeout)
        if let Some(cycle_task) = self.cycle_task.take() {
            // Wait up to 150 seconds (120s inference timeout + 30s buffer)
            let timeout_duration = std::time::Duration::from_secs(150);
            let _ = tokio::time::timeout(timeout_duration, cycle_task).await;
        }
        
        // Return final state
        let state = self.state.lock().await;
        state.clone()
    }
    
    /// Get the current agent state
    /// 
    /// Returns a clone of the current state without stopping the agent.
    /// 
    /// # Returns
    /// 
    /// The current agent state
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use jarvis_app_lib::agents::copilot::CoPilotAgent;
    /// 
    /// async fn check_state(agent: &CoPilotAgent) {
    ///     let state = agent.get_state().await;
    ///     println!("Current cycle: {}", state.cycle_metadata.cycle_number);
    /// }
    /// ```
    pub async fn get_state(&self) -> CoPilotState {
        let state = self.state.lock().await;
        state.clone()
    }
    
    /// Dismiss a suggested question by index
    /// 
    /// Marks the question at the specified index as dismissed. Dismissed questions
    /// will not be shown in the UI but will be preserved if the same question is
    /// suggested again in a future cycle.
    /// 
    /// # Arguments
    /// 
    /// * `index` - The index of the question to dismiss (0-based)
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use jarvis_app_lib::agents::copilot::CoPilotAgent;
    /// 
    /// async fn dismiss_first_question(agent: &CoPilotAgent) {
    ///     agent.dismiss_question(0).await;
    /// }
    /// ```
    pub async fn dismiss_question(&self, index: usize) {
        let mut state = self.state.lock().await;
        if let Some(question) = state.suggested_questions.get_mut(index) {
            question.dismissed = true;
        }
    }

}
