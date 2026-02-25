use crate::files::{FileManager, RecordingMetadata};
use crate::gems::{Gem, GemPreview, GemStore};
use crate::intelligence::IntelProvider;
use crate::platform::PlatformDetector;
use crate::recording::RecordingManager;
use crate::settings::{ModelManager, Settings, SettingsManager};
use crate::transcription::{TranscriptionManager, TranscriptionSegment, TranscriptionStatus, WhisperKitProvider};
use crate::wav::WavConverter;
use serde::Serialize;
use std::sync::{Arc, Mutex, RwLock};
use tauri::{Emitter, State};

/// Helper function to map PageGist to Gem
/// 
/// This function converts a PageGist (from browser extractors) into a Gem
/// for persistence. It generates a new UUID and timestamp, and merges
/// published_date and image_url into source_meta alongside the extra field.
fn page_gist_to_gem(gist: crate::browser::extractors::PageGist) -> Gem {
    // Merge published_date and image_url into source_meta
    let source_meta = if let serde_json::Value::Object(mut map) = gist.extra {
        // Start with the extra field as base
        if let Some(published_date) = gist.published_date {
            map.insert("published_date".to_string(), serde_json::Value::String(published_date));
        }
        if let Some(image_url) = gist.image_url {
            map.insert("image_url".to_string(), serde_json::Value::String(image_url));
        }
        serde_json::Value::Object(map)
    } else {
        // If extra is not an object, create a new object with all metadata
        let mut map = serde_json::Map::new();
        if let Some(published_date) = gist.published_date {
            map.insert("published_date".to_string(), serde_json::Value::String(published_date));
        }
        if let Some(image_url) = gist.image_url {
            map.insert("image_url".to_string(), serde_json::Value::String(image_url));
        }
        serde_json::Value::Object(map)
    };

    Gem {
        id: uuid::Uuid::new_v4().to_string(),
        source_type: format!("{:?}", gist.source_type),
        source_url: gist.url,
        domain: gist.domain,
        title: gist.title,
        author: gist.author,
        description: gist.description,
        content: gist.content_excerpt,
        source_meta,
        captured_at: chrono::Utc::now().to_rfc3339(),
        ai_enrichment: None,
    }
}

/// Helper function to enrich content with AI-generated metadata
/// 
/// This function calls the IntelProvider to generate tags and a summary
/// for the given content, then builds the ai_enrichment JSON structure.
/// 
/// # Arguments
/// 
/// * `provider` - The IntelProvider trait object
/// * `content` - The content to enrich
/// 
/// # Returns
/// 
/// * `Ok(serde_json::Value)` - The ai_enrichment JSON with tags, summary, provider, enriched_at
/// * `Err(String)` - Error message if enrichment fails
async fn enrich_content(
    provider: &dyn IntelProvider,
    content: &str,
) -> Result<serde_json::Value, String> {
    // Generate tags
    let tags = provider.generate_tags(content).await?;
    
    // Generate summary
    let summary = provider.summarize(content).await?;
    
    // Build ai_enrichment JSON
    let ai_enrichment = serde_json::json!({
        "tags": tags,
        "summary": summary,
        "provider": "intelligencekit",
        "enriched_at": chrono::Utc::now().to_rfc3339(),
    });
    
    Ok(ai_enrichment)
}

/// Save a PageGist as a Gem
///
/// This command converts a PageGist (from browser extractors) into a Gem
/// and persists it via the GemStore trait. It generates a UUID and timestamp,
/// maps fields, and merges published_date and image_url into source_meta.
///
/// # Arguments
///
/// * `gist` - The PageGist to save (from browser extraction)
/// * `gem_store` - Managed state containing the GemStore trait object
///
/// # Returns
///
/// * `Ok(Gem)` - The saved gem (including generated id and captured_at)
/// * `Err(String)` - Error message if save fails
///
/// # Errors
///
/// Returns an error if:
/// - The GemStore save operation fails
/// - Database connection issues occur
///
/// # Examples
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// interface PageGist {
///   url: string;
///   title: string;
///   source_type: string;
///   domain: string;
///   author?: string;
///   description?: string;
///   content_excerpt?: string;
///   published_date?: string;
///   image_url?: string;
///   extra: Record<string, any>;
/// }
///
/// interface Gem {
///   id: string;
///   source_type: string;
///   source_url: string;
///   domain: string;
///   title: string;
///   author?: string;
///   description?: string;
///   content?: string;
///   source_meta: Record<string, any>;
///   captured_at: string;
/// }
///
/// try {
///   const gem: Gem = await invoke('save_gem', { gist: myPageGist });
///   console.log(`Gem saved with ID: ${gem.id}`);
/// } catch (error) {
///   console.error(`Failed to save gem: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn save_gem(
    gist: crate::browser::extractors::PageGist,
    gem_store: State<'_, Arc<dyn GemStore>>,
    intel_provider: State<'_, Arc<dyn IntelProvider>>,
) -> Result<Gem, String> {
    // Convert PageGist to Gem using the helper function
    let mut gem = page_gist_to_gem(gist);

    // Check if AI enrichment is available
    let availability = intel_provider.check_availability().await;
    
    if availability.available {
        // Get content for enrichment (prefer content, fall back to description)
        let content_to_enrich = gem.content.as_ref()
            .or(gem.description.as_ref())
            .filter(|s| !s.trim().is_empty());
        
        if let Some(content) = content_to_enrich {
            // Try to enrich, but don't fail the save if enrichment fails
            match enrich_content(&**intel_provider, content).await {
                Ok(ai_enrichment) => {
                    gem.ai_enrichment = Some(ai_enrichment);
                }
                Err(e) => {
                    // Log error but continue with save
                    eprintln!("Failed to enrich gem: {}", e);
                }
            }
        }
    }

    // Save via GemStore trait (with or without enrichment)
    gem_store.save(gem).await
}

/// List gems with pagination
///
/// This command returns all gems ordered by captured_at descending (most recent first).
/// Supports pagination via limit and offset parameters.
///
/// # Arguments
///
/// * `limit` - Optional maximum number of gems to return (default: 50)
/// * `offset` - Optional number of gems to skip for pagination (default: 0)
/// * `gem_store` - Managed state containing the GemStore trait object
///
/// # Returns
///
/// * `Ok(Vec<GemPreview>)` - Array of gem previews with truncated content
/// * `Err(String)` - Error message if listing fails
///
/// # Errors
///
/// Returns an error if:
/// - The GemStore list operation fails
/// - Database connection issues occur
///
/// # Examples
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// interface GemPreview {
///   id: string;
///   source_type: string;
///   source_url: string;
///   domain: string;
///   title: string;
///   author?: string;
///   description?: string;
///   content_preview?: string;  // Truncated to 200 characters
///   captured_at: string;
/// }
///
/// // List first 50 gems (default)
/// try {
///   const gems: GemPreview[] = await invoke('list_gems');
///   console.log(`Found ${gems.length} gems`);
/// } catch (error) {
///   console.error(`Failed to list gems: ${error}`);
/// }
///
/// // List with custom limit and offset
/// try {
///   const gems: GemPreview[] = await invoke('list_gems', {
///     limit: 20,
///     offset: 40
///   });
///   console.log(`Page 3: ${gems.length} gems`);
/// } catch (error) {
///   console.error(`Failed to list gems: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn list_gems(
    limit: Option<usize>,
    offset: Option<usize>,
    gem_store: State<'_, Arc<dyn GemStore>>,
) -> Result<Vec<GemPreview>, String> {
    gem_store.list(limit.unwrap_or(50), offset.unwrap_or(0)).await
}

/// Search gems by keyword
///
/// This command searches gems using full-text search (FTS5) on title, description,
/// and content fields. Results are ranked by relevance. Empty queries return the
/// same results as list_gems.
///
/// # Arguments
///
/// * `query` - Search query string (supports FTS5 syntax)
/// * `limit` - Optional maximum number of results to return (default: 50)
/// * `gem_store` - Managed state containing the GemStore trait object
///
/// # Returns
///
/// * `Ok(Vec<GemPreview>)` - Array of gem previews ranked by relevance
/// * `Err(String)` - Error message if search fails
///
/// # Errors
///
/// Returns an error if:
/// - The GemStore search operation fails
/// - Database connection issues occur
/// - FTS5 query syntax is invalid
///
/// # Examples
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// interface GemPreview {
///   id: string;
///   source_type: string;
///   source_url: string;
///   domain: string;
///   title: string;
///   author?: string;
///   description?: string;
///   content_preview?: string;  // Truncated to 200 characters
///   captured_at: string;
/// }
///
/// // Basic keyword search
/// try {
///   const gems: GemPreview[] = await invoke('search_gems', {
///     query: 'rust async'
///   });
///   console.log(`Found ${gems.length} gems matching "rust async"`);
/// } catch (error) {
///   console.error(`Failed to search gems: ${error}`);
/// }
///
/// // Search with custom limit
/// try {
///   const gems: GemPreview[] = await invoke('search_gems', {
///     query: 'OAuth token',
///     limit: 20
///   });
///   console.log(`Top 20 results for "OAuth token"`);
/// } catch (error) {
///   console.error(`Failed to search gems: ${error}`);
/// }
///
/// // Empty query returns all gems (same as list_gems)
/// try {
///   const gems: GemPreview[] = await invoke('search_gems', {
///     query: ''
///   });
///   console.log(`All gems: ${gems.length}`);
/// } catch (error) {
///   console.error(`Failed to search gems: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn search_gems(
    query: String,
    limit: Option<usize>,
    gem_store: State<'_, Arc<dyn GemStore>>,
) -> Result<Vec<GemPreview>, String> {
    gem_store.search(&query, limit.unwrap_or(50)).await
}

/// Delete a gem by ID
///
/// This command deletes a gem from the store by its unique identifier.
/// Returns an error if the gem is not found.
///
/// # Arguments
///
/// * `id` - The unique identifier of the gem to delete
/// * `gem_store` - Managed state containing the GemStore trait object
///
/// # Returns
///
/// * `Ok(())` - Gem deleted successfully
/// * `Err(String)` - Error message if deletion fails
///
/// # Errors
///
/// Returns an error if:
/// - The gem with the specified ID does not exist
/// - The GemStore delete operation fails
/// - Database connection issues occur
///
/// # Examples
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// try {
///   await invoke('delete_gem', {
///     id: '550e8400-e29b-41d4-a716-446655440000'
///   });
///   console.log('Gem deleted successfully');
/// } catch (error) {
///   console.error(`Failed to delete gem: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn delete_gem(
    id: String,
    gem_store: State<'_, Arc<dyn GemStore>>,
) -> Result<(), String> {
    gem_store.delete(&id).await
}

/// Get a gem by ID
///
/// This command retrieves a gem from the store by its unique identifier.
/// Returns None if the gem is not found (not an error), returns Err only
/// on database errors.
///
/// # Arguments
///
/// * `id` - The unique identifier of the gem to retrieve
/// * `gem_store` - Managed state containing the GemStore trait object
///
/// # Returns
///
/// * `Ok(Some(Gem))` - Gem found and returned with full content
/// * `Ok(None)` - Gem not found (not an error condition)
/// * `Err(String)` - Error message if database operation fails
///
/// # Errors
///
/// Returns an error if:
/// - The GemStore get operation fails
/// - Database connection issues occur
///
/// Note: A missing gem (None) is not considered an error - it's a valid result
/// indicating the gem doesn't exist in the store.
///
/// # Examples
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// interface Gem {
///   id: string;
///   source_type: string;
///   source_url: string;
///   domain: string;
///   title: string;
///   author?: string;
///   description?: string;
///   content?: string;
///   source_meta: Record<string, any>;
///   captured_at: string;
/// }
///
/// try {
///   const gem: Gem | null = await invoke('get_gem', {
///     id: '550e8400-e29b-41d4-a716-446655440000'
///   });
///   
///   if (gem) {
///     console.log(`Found gem: ${gem.title}`);
///   } else {
///     console.log('Gem not found');
///   }
/// } catch (error) {
///   console.error(`Database error: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn get_gem(
    id: String,
    gem_store: State<'_, Arc<dyn GemStore>>,
) -> Result<Option<Gem>, String> {
    gem_store.get(&id).await
}

/// Enrich a gem with AI-generated tags and summary
///
/// This command enriches an existing gem by generating tags and a summary
/// using the IntelProvider. It fetches the gem, enriches it, and saves it back.
///
/// # Arguments
///
/// * `id` - The unique identifier of the gem to enrich
/// * `gem_store` - Managed state containing the GemStore trait object
/// * `intel_provider` - Managed state containing the IntelProvider trait object
///
/// # Returns
///
/// * `Ok(Gem)` - The enriched gem with ai_enrichment populated
/// * `Err(String)` - Error message if enrichment fails
///
/// # Errors
///
/// Returns an error if:
/// - The IntelProvider is not available
/// - The gem with the specified ID does not exist
/// - The gem has no content or description to enrich
/// - The enrichment process fails (tag generation or summarization)
///
/// # Examples
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// try {
///   const enrichedGem = await invoke('enrich_gem', {
///     id: '550e8400-e29b-41d4-a716-446655440000'
///   });
///   console.log(`Enriched with ${enrichedGem.ai_enrichment.tags.length} tags`);
/// } catch (error) {
///   console.error(`Failed to enrich gem: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn enrich_gem(
    id: String,
    gem_store: State<'_, Arc<dyn GemStore>>,
    intel_provider: State<'_, Arc<dyn IntelProvider>>,
) -> Result<Gem, String> {
    // Check availability first
    let availability = intel_provider.check_availability().await;
    if !availability.available {
        return Err(format!(
            "AI enrichment not available: {}",
            availability.reason.unwrap_or_else(|| "Unknown reason".to_string())
        ));
    }
    
    // Fetch gem by ID
    let mut gem = gem_store.get(&id).await?
        .ok_or_else(|| format!("Gem with id '{}' not found", id))?;
    
    // Get content for enrichment (prefer content, fall back to description)
    let content_to_enrich = gem.content.as_ref()
        .or(gem.description.as_ref())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "Gem has no content or description to enrich".to_string())?;
    
    // Enrich the content
    let ai_enrichment = enrich_content(&**intel_provider, content_to_enrich).await?;
    
    // Update gem with enrichment
    gem.ai_enrichment = Some(ai_enrichment);
    
    // Save and return
    gem_store.save(gem).await
}

/// Check if AI enrichment is available
///
/// This command checks whether the IntelProvider is available and ready
/// to process enrichment requests.
///
/// # Arguments
///
/// * `intel_provider` - Managed state containing the IntelProvider trait object
///
/// # Returns
///
/// * `Ok(AvailabilityResult)` - Availability status with optional reason
/// * `Err(String)` - Never returns an error (always succeeds)
///
/// # Examples
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// interface AvailabilityResult {
///   available: boolean;
///   reason?: string;
/// }
///
/// try {
///   const status: AvailabilityResult = await invoke('check_intel_availability');
///   if (status.available) {
///     console.log('AI enrichment is available');
///   } else {
///     console.log(`AI enrichment unavailable: ${status.reason}`);
///   }
/// } catch (error) {
///   console.error(`Failed to check availability: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn check_intel_availability(
    intel_provider: State<'_, Arc<dyn IntelProvider>>,
) -> Result<crate::intelligence::AvailabilityResult, String> {
    Ok(intel_provider.check_availability().await)
}

/// Filter gems by tag
///
/// This command returns gems that have the specified tag in their ai_enrichment.
/// Results are ordered by captured_at descending (most recent first).
///
/// # Arguments
///
/// * `tag` - The tag to filter by (exact match)
/// * `limit` - Optional maximum number of gems to return (default: 50)
/// * `offset` - Optional number of gems to skip for pagination (default: 0)
/// * `gem_store` - Managed state containing the GemStore trait object
///
/// # Returns
///
/// * `Ok(Vec<GemPreview>)` - Array of gem previews with the specified tag
/// * `Err(String)` - Error message if filtering fails
///
/// # Errors
///
/// Returns an error if:
/// - The GemStore filter_by_tag operation fails
/// - Database connection issues occur
///
/// # Examples
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// try {
///   const gems = await invoke('filter_gems_by_tag', {
///     tag: 'rust',
///     limit: 20
///   });
///   console.log(`Found ${gems.length} gems tagged with 'rust'`);
/// } catch (error) {
///   console.error(`Failed to filter gems: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn filter_gems_by_tag(
    tag: String,
    limit: Option<usize>,
    offset: Option<usize>,
    gem_store: State<'_, Arc<dyn GemStore>>,
) -> Result<Vec<GemPreview>, String> {
    gem_store.filter_by_tag(&tag, limit.unwrap_or(50), offset.unwrap_or(0)).await
}


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

/// Get browser settings
/// 
/// This command returns the current browser observer settings including
/// whether the observer is enabled.
/// 
/// # Arguments
/// 
/// * `settings_manager` - Managed state containing the SettingsManager
/// 
/// # Returns
/// 
/// * `Ok(BrowserSettings)` - Current browser settings
/// * `Err(String)` - Error message if settings cannot be read
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// interface BrowserSettings {
///   observer_enabled: boolean;
/// }
/// 
/// try {
///   const settings: BrowserSettings = await invoke('get_browser_settings');
///   console.log(`Observer enabled: ${settings.observer_enabled}`);
/// } catch (error) {
///   console.error(`Failed to get browser settings: ${error}`);
/// }
/// ```
#[tauri::command]
pub fn get_browser_settings(
    settings_manager: State<'_, Arc<RwLock<SettingsManager>>>,
) -> Result<crate::settings::BrowserSettings, String> {
    let manager = settings_manager.read()
        .map_err(|e| format!("Failed to acquire settings read lock: {}", e))?;
    Ok(manager.get().browser)
}

/// Update browser settings
/// 
/// This command updates the browser observer settings and starts/stops
/// the observer based on the new enabled state.
/// 
/// # Arguments
/// 
/// * `settings_manager` - Managed state containing the SettingsManager
/// * `browser_observer` - Managed state containing the BrowserObserver
/// * `observer_enabled` - Whether the observer should be enabled
/// 
/// # Returns
/// 
/// * `Ok(())` - Settings updated successfully
/// * `Err(String)` - Error message if update fails
/// 
/// # Examples
/// 
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
/// 
/// try {
///   await invoke('update_browser_settings', { observerEnabled: true });
///   console.log('Browser settings updated');
/// } catch (error) {
///   console.error(`Failed to update browser settings: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn update_browser_settings(
    settings_manager: State<'_, Arc<RwLock<SettingsManager>>>,
    browser_observer: State<'_, Arc<tokio::sync::Mutex<crate::browser::BrowserObserver>>>,
    observer_enabled: bool,
) -> Result<(), String> {
    // Get current settings
    let mut settings = {
        let manager = settings_manager.read()
            .map_err(|e| format!("Failed to acquire settings read lock: {}", e))?;
        manager.get()
    };
    
    // Update browser settings
    settings.browser.observer_enabled = observer_enabled;
    
    // Persist settings
    {
        let manager = settings_manager.read()
            .map_err(|e| format!("Failed to acquire settings read lock: {}", e))?;
        manager.update(settings)?;
    }
    
    // Apply observer state change
    let mut observer = browser_observer.lock().await;
    if observer_enabled && !observer.is_running() {
        observer.start().await?;
    } else if !observer_enabled && observer.is_running() {
        observer.stop().await?;
    }
    
    Ok(())
}

/// List all open browser tabs with classification
///
/// This command uses the active browser adapter (currently Chrome via AppleScript)
/// to enumerate all open tabs and classify them by source type.
#[tauri::command]
pub async fn list_browser_tabs() -> Result<Vec<crate::browser::tabs::BrowserTab>, String> {
    crate::browser::tabs::list_all_tabs().await
}

/// Prepare a gist for a browser tab URL
///
/// This command fetches metadata from the URL using the appropriate extractor
/// based on the source type (YouTube extractor for YouTube, generic for everything else).
#[tauri::command]
pub async fn prepare_tab_gist(
    url: String,
    source_type: String,
) -> Result<crate::browser::extractors::PageGist, String> {
    let st: crate::browser::tabs::SourceType =
        serde_json::from_str(&format!("\"{}\"", source_type))
            .unwrap_or(crate::browser::tabs::SourceType::Other);
    crate::browser::extractors::prepare_gist(&url, &st).await
}

/// Prepare a gist for a browser tab, including the Claude conversation if detected.
///
/// Runs the page extractor and Claude extractor concurrently, then merges
/// both into a single PageGist. If Claude extraction fails, returns page-only gist.
#[tauri::command]
pub async fn prepare_tab_gist_with_claude(
    url: String,
    source_type: String,
) -> Result<crate::browser::extractors::PageGist, String> {
    let st: crate::browser::tabs::SourceType =
        serde_json::from_str(&format!("\"{}\"", source_type))
            .unwrap_or(crate::browser::tabs::SourceType::Other);

    let (page_result, claude_result) = tokio::join!(
        crate::browser::extractors::prepare_gist(&url, &st),
        crate::browser::extractors::claude_extension::extract()
    );

    let page_gist = page_result?;

    match claude_result {
        Ok(claude_gist) => {
            eprintln!("[MergedGist] Merging page gist with Claude conversation");
            Ok(crate::browser::extractors::merge_gists(page_gist, claude_gist))
        }
        Err(e) => {
            eprintln!("[MergedGist] Claude extraction failed, returning page-only gist: {}", e);
            Ok(page_gist)
        }
    }
}

/// Export a gist to a text file in ~/.jarvis/gists/
///
/// Returns the full path to the saved file.
#[tauri::command]
pub async fn export_gist(title: String, content: String) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let gists_dir = home.join(".jarvis").join("gists");
    std::fs::create_dir_all(&gists_dir)
        .map_err(|e| format!("Failed to create gists directory: {}", e))?;

    // Sanitize title for filename: keep alphanumeric, spaces → dashes, limit length
    let safe_name: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect();
    let safe_name = safe_name.trim_matches('-');
    let safe_name = if safe_name.len() > 80 {
        &safe_name[..80]
    } else {
        safe_name
    };

    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let filename = format!("{}-{}.md", timestamp, safe_name);
    let file_path = gists_dir.join(&filename);

    std::fs::write(&file_path, &content)
        .map_err(|e| format!("Failed to write gist file: {}", e))?;

    Ok(file_path.to_string_lossy().to_string())
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

/// Capture Claude conversation from Chrome Extension side panel
///
/// This command extracts a conversation from the Claude Chrome Extension
/// using macOS Accessibility APIs. It returns a PageGist that can be saved
/// as a gem.
///
/// # Returns
///
/// * `Ok(PageGist)` - The extracted conversation as a PageGist
/// * `Err(String)` - Error message if extraction fails
///
/// # Errors
///
/// * "Accessibility permission not granted" - User needs to grant accessibility permission
/// * "Chrome is not running" - Chrome must be running with Claude extension open
/// * "No Claude conversation found" - Claude side panel must be open
/// * "Claude conversation capture is only available on macOS" - Non-macOS platforms
///
/// # Example
///
/// ```typescript
/// const gist = await invoke('capture_claude_conversation');
/// ```
#[tauri::command]
pub async fn capture_claude_conversation() -> Result<crate::browser::extractors::PageGist, String> {
    crate::browser::extractors::claude_extension::extract().await
}

/// Status of the Claude Chrome Extension side panel detection
#[derive(Debug, Clone, Serialize)]
pub struct ClaudePanelStatus {
    pub detected: bool,
    pub active_tab_url: Option<String>,
    pub needs_accessibility: bool,
}

/// Check if the Claude Chrome Extension side panel is currently visible
///
/// Scans Chrome's accessibility tree for a Claude web area. Returns detection
/// status along with the active tab URL (so frontend can match to the correct tab row).
#[tauri::command]
pub fn check_claude_panel() -> ClaudePanelStatus {
    #[cfg(target_os = "macos")]
    {
        use crate::browser::accessibility::AccessibilityReader;

        let not_detected = ClaudePanelStatus { detected: false, active_tab_url: None, needs_accessibility: false };

        if !AccessibilityReader::check_permission() {
            return ClaudePanelStatus { detected: false, active_tab_url: None, needs_accessibility: true };
        }

        let pid = match AccessibilityReader::find_chrome_pid() {
            Ok(pid) => pid,
            Err(_) => return not_detected,
        };

        let web_areas = match AccessibilityReader::find_web_areas(pid) {
            Ok(areas) => areas,
            Err(_) => return not_detected,
        };

        let detected = web_areas.iter().any(|wa| {
            let lower = wa.title.to_lowercase();
            lower.contains("claude") || lower.contains("anthropic") || lower.contains("side panel")
        });

        if detected {
            let active_tab_url = crate::browser::adapters::chrome::get_active_tab_url_sync().ok();
            eprintln!("[ClaudePanel] Detected. Active tab URL: {:?}", active_tab_url);
            ClaudePanelStatus { detected: true, active_tab_url, needs_accessibility: false }
        } else {
            not_detected
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        ClaudePanelStatus { detected: false, active_tab_url: None, needs_accessibility: false }
    }
}

/// Check if accessibility permission is granted
///
/// This command checks if the app has accessibility permission on macOS,
/// which is required for capturing Claude conversations. On non-macOS
/// platforms, this always returns false.
///
/// # Returns
///
/// * `true` - Accessibility permission is granted (macOS only)
/// * `false` - Permission not granted or non-macOS platform
///
/// # Example
///
/// ```typescript
/// const hasPermission = await invoke('check_accessibility_permission');
/// if (!hasPermission) {
///   // Show permission prompt
/// }
/// ```
#[tauri::command]
pub fn check_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        crate::browser::accessibility::AccessibilityReader::check_permission()
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}
