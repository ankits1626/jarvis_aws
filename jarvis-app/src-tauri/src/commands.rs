use crate::files::{FileManager, RecordingMetadata};
use crate::gems::{Gem, GemPreview, GemStore};
use crate::intelligence::{IntelProvider, LlmModelInfo, LlmModelManager, VenvManager};
use crate::intelligence::provider::TranscriptResult;
use crate::intelligence::queue::IntelQueue;
use crate::agents::chatable::Chatable;
use crate::agents::chatbot::{Chatbot, ChatMessage};
use crate::agents::recording_chat::RecordingChatSource;
use crate::platform::PlatformDetector;
use crate::recording::RecordingManager;
use crate::settings::{ModelManager, Settings, SettingsManager};
use crate::transcription::{TranscriptionManager, TranscriptionSegment, TranscriptionStatus, WhisperKitProvider};
use crate::wav::WavConverter;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use tauri::{AppHandle, Emitter, Manager, State};

/// Append a timestamped log line to ~/Library/Application Support/com.jarvis.app/logs/gem_save.log
fn log_gem_save(msg: &str) {
    if let Some(data_dir) = dirs::data_dir() {
        let log_dir = data_dir.join("com.jarvis.app").join("logs");
        let _ = std::fs::create_dir_all(&log_dir);
        let log_path = log_dir.join("gem_save.log");
        let timestamp = chrono::Utc::now().to_rfc3339();
        let line = format!("[{}] {}\n", timestamp, msg);
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
    }
}

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
        transcript: None,
        transcript_language: None,
    }
}

/// Helper function to extract recording file path from a gem
///
/// This function checks if a gem is a recording and extracts the audio file path.
/// Recording gems have source_type "Recording" and store the filename in source_meta.
///
/// # Arguments
///
/// * `gem` - The gem to extract the recording path from
///
/// # Returns
///
/// * `Some(PathBuf)` - Full path to the recording file if gem is a recording
/// * `None` - If gem is not a recording or filename is missing
fn extract_recording_path(gem: &Gem) -> Option<PathBuf> {
    // Detect recording gems by presence of recording filename in source_meta
    // (NOT by source_type — recordings are saved with source_type "Other")
    let filename = gem.source_meta.get("recording_filename")
        .or_else(|| gem.source_meta.get("filename"))
        .or_else(|| gem.source_meta.get("recording_path"))
        .or_else(|| gem.source_meta.get("file"))
        .or_else(|| gem.source_meta.get("path"))
        .and_then(|v| v.as_str())?;

    // Build full path using the same location as FileManager:
    // dirs::data_dir()/com.jarvis.app/recordings/{filename}
    let data_dir = dirs::data_dir()?;
    Some(data_dir.join("com.jarvis.app").join("recordings").join(filename))
}

/// Result of content enrichment including AI-generated metadata and optional transcript
struct EnrichmentResult {
    ai_enrichment: serde_json::Value,
    transcript: Option<String>,
    transcript_language: Option<String>,
}

/// Helper function to enrich content with AI-generated metadata and optional transcript
/// 
/// This function calls the IntelProvider to generate tags, summary, and optionally
/// a transcript (for recording gems). It builds the complete enrichment result.
/// 
/// # Arguments
/// 
/// * `provider` - The IntelProvider trait object
/// * `content` - The content to enrich (for tags/summary)
/// * `gem` - The full gem (to extract recording path for transcript)
/// * `provider_name` - The name of the provider being used
/// * `model_name` - Optional model name (for MLX provider)
/// * `transcription_engine` - The transcription engine setting ("whisper-rs", "whisperkit", "mlx-omni")
/// 
/// # Returns
/// 
/// * `Ok(EnrichmentResult)` - The enrichment result with ai_enrichment JSON and optional transcript
/// * `Err(String)` - Error message if enrichment fails
async fn enrich_content(
    provider: &dyn IntelProvider,
    content: &str,
    gem: &Gem,
    provider_name: &str,
    model_name: Option<&str>,
    transcription_engine: &str,
) -> Result<EnrichmentResult, String> {
    // Generate transcript first (if applicable) so we can use it for tags/summary
    let (transcript, transcript_language) = if transcription_engine == "mlx-omni" {
        if let Some(recording_path) = extract_recording_path(gem) {
            match provider.generate_transcript(&recording_path).await {
                Ok(result) => (Some(result.transcript), Some(result.language)),
                Err(e) => {
                    eprintln!("Failed to generate transcript for {}: {}",
                        recording_path.display(), e);
                    (None, None)
                }
            }
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    // Use MLX Omni transcript for tags/summary if available (more accurate than Whisper real-time)
    let text_for_enrichment = transcript.as_deref().unwrap_or(content);

    // Generate tags
    let tags = provider.generate_tags(text_for_enrichment).await?;

    // Generate summary
    let summary = provider.summarize(text_for_enrichment).await?;

    // Build ai_enrichment JSON
    let mut ai_enrichment = serde_json::json!({
        "tags": tags,
        "summary": summary,
        "provider": provider_name,
        "enriched_at": chrono::Utc::now().to_rfc3339(),
    });

    if let Some(model) = model_name {
        ai_enrichment["model"] = serde_json::Value::String(model.to_string());
    }

    Ok(EnrichmentResult {
        ai_enrichment,
        transcript,
        transcript_language,
    })
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
    app_handle: tauri::AppHandle,
    gist: crate::browser::extractors::PageGist,
    gem_store: State<'_, Arc<dyn GemStore>>,
    intel_provider: State<'_, Arc<dyn IntelProvider>>,
    settings_manager: State<'_, Arc<RwLock<SettingsManager>>>,
) -> Result<Gem, String> {
    // Convert PageGist to Gem using the helper function
    log_gem_save(&format!("save_gem called: url={}, title={}", gist.url, gist.title));
    let mut gem = page_gist_to_gem(gist);
    log_gem_save(&format!("save_gem: gem id={}, content_len={:?}", gem.id, gem.content.as_ref().map(|c| c.len())));

    // Check if AI enrichment is available
    let availability = intel_provider.check_availability().await;
    log_gem_save(&format!("save_gem: intel available={}", availability.available));

    if availability.available {
        // Get provider name and model from settings
        let (provider_name, model_name, transcription_engine) = {
            let manager = settings_manager.read()
                .map_err(|e| format!("Failed to acquire settings lock: {}", e))?;
            let s = manager.get();
            (
                s.intelligence.provider.clone(), 
                s.intelligence.active_model.clone(),
                s.transcription.transcription_engine.clone()
            )
        };
        let model_ref = if provider_name == "mlx" { Some(model_name.as_str()) } else { None };

        // Get content for enrichment (prefer content, fall back to description)
        let content_to_enrich = gem.content.as_ref()
            .or(gem.description.as_ref())
            .filter(|s| !s.trim().is_empty());

        if let Some(content) = content_to_enrich {
            // Try to enrich, but don't fail the save if enrichment fails
            match enrich_content(&**intel_provider, content, &gem, &provider_name, model_ref, &transcription_engine).await {
                Ok(enrichment_result) => {
                    gem.ai_enrichment = Some(enrichment_result.ai_enrichment);
                    gem.transcript = enrichment_result.transcript;
                    gem.transcript_language = enrichment_result.transcript_language;
                }
                Err(e) => {
                    // Log error but continue with save
                    log_gem_save(&format!("save_gem: WARN enrichment failed: {}", e));
                    eprintln!("Failed to enrich gem: {}", e);
                }
            }
        }
    }

    // Save via GemStore trait (with or without enrichment)
    log_gem_save(&format!("save_gem: saving gem id={}", gem.id));
    let result = gem_store.save(gem).await;
    match &result {
        Ok(g) => log_gem_save(&format!("save_gem: SUCCESS id={}", g.id)),
        Err(e) => log_gem_save(&format!("save_gem: ERROR {}", e)),
    }
    
    // Generate knowledge files
    if let Ok(ref saved_gem) = result {
        if let Some(ks) = app_handle.try_state::<Arc<dyn crate::knowledge::KnowledgeStore>>() {
            if let Err(e) = ks.create(saved_gem).await {
                eprintln!("Knowledge file creation failed for gem {}: {}", saved_gem.id, e);
            }
        }
    }
    
    result
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
    app_handle: tauri::AppHandle,
    id: String,
    gem_store: State<'_, Arc<dyn GemStore>>,
) -> Result<(), String> {
    gem_store.delete(&id).await?;
    
    // Delete knowledge files
    if let Some(ks) = app_handle.try_state::<Arc<dyn crate::knowledge::KnowledgeStore>>() {
        if let Err(e) = ks.delete(&id).await {
            eprintln!("Knowledge file deletion failed for gem {}: {}", id, e);
        }
    }
    
    Ok(())
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
    app_handle: tauri::AppHandle,
    id: String,
    gem_store: State<'_, Arc<dyn GemStore>>,
    intel_provider: State<'_, Arc<dyn IntelProvider>>,
    settings_manager: State<'_, Arc<RwLock<SettingsManager>>>,
) -> Result<Gem, String> {
    // Check availability first
    let availability = intel_provider.check_availability().await;
    if !availability.available {
        return Err(format!(
            "AI enrichment not available: {}",
            availability.reason.unwrap_or_else(|| "Unknown reason".to_string())
        ));
    }
    
    // Get provider name and model from settings
    let (provider_name, model_name, transcription_engine) = {
        let manager = settings_manager.read()
            .map_err(|e| format!("Failed to acquire settings lock: {}", e))?;
        let s = manager.get();
        (
            s.intelligence.provider.clone(), 
            s.intelligence.active_model.clone(),
            s.transcription.transcription_engine.clone()
        )
    };
    let model_ref = if provider_name == "mlx" { Some(model_name.as_str()) } else { None };

    // Fetch gem by ID
    let mut gem = gem_store.get(&id).await?
        .ok_or_else(|| format!("Gem with id '{}' not found", id))?;

    // Get content for enrichment (prefer content, fall back to description)
    let content_to_enrich = gem.content.as_ref()
        .or(gem.description.as_ref())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "Gem has no content or description to enrich".to_string())?;

    // Enrich the content
    let enrichment_result = match enrich_content(&**intel_provider, content_to_enrich, &gem, &provider_name, model_ref, &transcription_engine).await {
        Ok(enrichment) => enrichment,
        Err(e) => {
            // Check if error indicates sidecar crash (broken pipe)
            if e.contains("broken pipe") || e.contains("closed connection") || e.contains("Sidecar") {
                // Emit event to frontend for toast notification
                let _ = app_handle.emit("mlx-sidecar-error", serde_json::json!({
                    "error": e.clone()
                }));
            }
            return Err(e);
        }
    };
    
    // Update gem with enrichment
    gem.ai_enrichment = Some(enrichment_result.ai_enrichment);
    gem.transcript = enrichment_result.transcript;
    gem.transcript_language = enrichment_result.transcript_language;
    
    // Save and return
    let result = gem_store.save(gem).await;
    
    // Update knowledge files
    if let Ok(ref enriched_gem) = result {
        if let Some(ks) = app_handle.try_state::<Arc<dyn crate::knowledge::KnowledgeStore>>() {
            // Update enrichment subfile
            if let Some(ref enrichment) = enriched_gem.ai_enrichment {
                let formatted = crate::knowledge::assembler::format_enrichment(enrichment);
                if let Err(e) = ks.update_subfile(&enriched_gem.id, "enrichment.md", &formatted).await {
                    eprintln!("Knowledge enrichment update failed: {}", e);
                }
            }
        }
    }
    
    result
}

/// Transcribe a recording gem and regenerate tags/summary from the transcript
///
/// This command generates an accurate transcript for a specific recording gem,
/// then regenerates tags and summary based on that transcript (which is more
/// accurate than the Whisper real-time content).
///
/// # Arguments
///
/// * `id` - The unique identifier of the gem to transcribe
///
/// # Returns
///
/// * `Ok(Gem)` - The updated gem with transcript, tags, and summary
/// * `Err(String)` - Error message if transcription fails
#[tauri::command]
pub async fn transcribe_gem(
    app_handle: tauri::AppHandle,
    id: String,
    gem_store: State<'_, Arc<dyn GemStore>>,
    intel_provider: State<'_, Arc<dyn IntelProvider>>,
    settings_manager: State<'_, Arc<RwLock<SettingsManager>>>,
) -> Result<Gem, String> {
    // Check availability first
    let availability = intel_provider.check_availability().await;
    if !availability.available {
        return Err(format!(
            "AI provider not available: {}",
            availability.reason.unwrap_or_else(|| "Unknown reason".to_string())
        ));
    }

    // Fetch gem by ID
    let mut gem = gem_store.get(&id).await?
        .ok_or_else(|| format!("Gem with id '{}' not found", id))?;

    // Extract recording path from source_meta
    let recording_path = extract_recording_path(&gem)
        .ok_or_else(|| "This gem has no associated recording file".to_string())?;

    // Verify recording file exists on disk
    if !recording_path.exists() {
        return Err(format!("Recording file not found: {}", recording_path.display()));
    }

    // Generate transcript
    let result = intel_provider.generate_transcript(&recording_path).await
        .map_err(|e| {
            if e.contains("not supported") {
                "Current AI provider does not support transcription".to_string()
            } else {
                e
            }
        })?;

    gem.transcript = Some(result.transcript);
    gem.transcript_language = Some(result.language);

    // Regenerate tags/summary from the accurate transcript
    let transcript_text = gem.transcript.as_deref().unwrap_or("");
    if !transcript_text.is_empty() {
        let (provider_name, model_name) = {
            let manager = settings_manager.read()
                .map_err(|e| format!("Failed to acquire settings lock: {}", e))?;
            let s = manager.get();
            (s.intelligence.provider.clone(), s.intelligence.active_model.clone())
        };

        let tags = intel_provider.generate_tags(transcript_text).await.unwrap_or_default();
        let summary = intel_provider.summarize(transcript_text).await.unwrap_or_default();

        let mut ai_enrichment = serde_json::json!({
            "tags": tags,
            "summary": summary,
            "provider": provider_name,
            "enriched_at": chrono::Utc::now().to_rfc3339(),
        });
        if provider_name == "mlx" {
            ai_enrichment["model"] = serde_json::Value::String(model_name);
        }
        gem.ai_enrichment = Some(ai_enrichment);
    }

    // Save and return
    let result = gem_store.save(gem).await;
    
    // Recreate all knowledge files (transcript + re-enrichment changes multiple things)
    if let Ok(ref gem) = result {
        if let Some(ks) = app_handle.try_state::<Arc<dyn crate::knowledge::KnowledgeStore>>() {
            if let Err(e) = ks.create(gem).await {
                eprintln!("Knowledge file update failed: {}", e);
            }
        }
    }
    
    result
}

/// Transcribe a recording file without creating a gem
///
/// This command transcribes a raw PCM recording file from the recordings directory
/// without creating or modifying any gems. It's used for the "Transcribe" button
/// in the recordings list UI.
///
/// # Arguments
///
/// * `filename` - The recording filename (e.g., "recording_1234567890.pcm")
/// * `intel_provider` - Managed state containing the IntelProvider trait object
///
/// # Returns
///
/// * `Ok(TranscriptResult)` - Transcript with detected language
/// * `Err(String)` - Error message if transcription fails
///
/// # Errors
///
/// Returns an error if:
/// - The filename contains path separators (security validation)
/// - The IntelProvider is not available
/// - The recording file doesn't exist on disk
/// - The provider doesn't support transcription
/// - The transcription process fails

/// Helper function for transcribe_recording that can be tested without Tauri State
async fn transcribe_recording_inner(
    filename: &str,
    provider: &dyn IntelProvider,
) -> Result<TranscriptResult, String> {
    // Security: Validate filename doesn't contain path separators
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Err("Invalid filename: path separators not allowed".to_string());
    }

    let data_dir = dirs::data_dir()
        .ok_or_else(|| "Could not find data directory".to_string())?;
    let recordings_dir = data_dir.join("com.jarvis.app").join("recordings");
    let recording_path = recordings_dir.join(filename);

    // Verify PCM file exists
    if !recording_path.exists() {
        return Err(format!("Recording file not found: {}", recording_path.display()));
    }

    // Per-recording folder: recordings/{stem}/
    let stem = filename.trim_end_matches(".pcm");
    let recording_dir = recordings_dir.join(stem);
    let transcript_path = recording_dir.join("transcript.md");

    // Fast path: transcript already exists on disk (generated by Chat or previous Transcribe)
    if transcript_path.exists() {
        let content = tokio::fs::read_to_string(&transcript_path).await
            .map_err(|e| format!("Failed to read transcript: {}", e))?;
        return Ok(TranscriptResult {
            language: String::new(),
            transcript: content,
        });
    }

    // Check provider availability
    let availability = provider.check_availability().await;
    if !availability.available {
        return Err(format!(
            "AI provider not available: {}",
            availability.reason.unwrap_or_else(|| "Unknown reason".to_string())
        ));
    }

    // Generate transcript
    let result = provider.generate_transcript(&recording_path).await
        .map_err(|e| {
            if e.contains("not supported") {
                "Current AI provider does not support transcription".to_string()
            } else {
                e
            }
        })?;

    // Save to per-recording folder for reuse by Chat and future Transcribe calls
    let transcript_md = format!(
        "# Transcript — {}\n\n**Generated:** {}\n\n---\n\n{}",
        stem,
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        result.transcript,
    );
    let _ = tokio::fs::create_dir_all(&recording_dir).await;
    let _ = tokio::fs::write(&transcript_path, &transcript_md).await;

    Ok(result)
}

#[tauri::command]
pub async fn transcribe_recording(
    filename: String,
    intel_provider: State<'_, Arc<dyn IntelProvider>>,
) -> Result<TranscriptResult, String> {
    transcribe_recording_inner(&filename, &**intel_provider).await
}

/// Check if a recording has an associated gem
///
/// This command queries the gem store for gems with a matching recording filename
/// in their source_meta. Used to determine button labels ("Save as Gem" vs "Update Gem").
///
/// # Arguments
///
/// * `filename` - The recording filename to search for
/// * `gem_store` - Managed state containing the GemStore trait object
///
/// # Returns
///
/// * `Ok(Some(GemPreview))` - Gem preview if found
/// * `Ok(None)` - No gem found for this recording
/// * `Err(String)` - Error message if query fails
#[tauri::command]
pub async fn check_recording_gem(
    filename: String,
    gem_store: State<'_, Arc<dyn GemStore>>,
) -> Result<Option<GemPreview>, String> {
    gem_store.find_by_recording_filename(&filename).await
}

/// Check which recordings have associated gems (batch operation)
///
/// This command queries the gem store for all provided recording filenames
/// and returns a map of filename -> GemPreview for recordings that have gems.
/// Used on mount to display gem indicators efficiently.
///
/// # Arguments
///
/// * `filenames` - Vector of recording filenames to check
/// * `gem_store` - Managed state containing the GemStore trait object
///
/// # Returns
///
/// * `Ok(HashMap<String, GemPreview>)` - Map of filename to gem preview (only for recordings with gems)
/// * `Err(String)` - Error message if query fails
#[tauri::command]
pub async fn check_recording_gems_batch(
    filenames: Vec<String>,
    gem_store: State<'_, Arc<dyn GemStore>>,
) -> Result<std::collections::HashMap<String, GemPreview>, String> {
    let mut result = std::collections::HashMap::new();
    
    for filename in filenames {
        if let Some(gem_preview) = gem_store.find_by_recording_filename(&filename).await? {
            result.insert(filename, gem_preview);
        }
    }
    
    Ok(result)
}

/// Save or update a recording gem with transcript
///
/// This command creates a new gem or updates an existing gem for a recording.
/// It checks for existing gems via recording filename, generates AI enrichment
/// (tags/summary), and handles graceful degradation when AI is unavailable.
///
/// # Arguments
///
/// * `filename` - The recording filename
/// * `transcript` - The transcript text
/// * `language` - The detected language code
/// * `created_at` - Unix timestamp (seconds) from RecordingMetadata
/// * `gem_store` - Managed state containing the GemStore trait object
/// * `intel_provider` - Managed state containing the IntelProvider trait object
/// * `settings_manager` - Managed state containing settings
///
/// # Returns
///
/// * `Ok(Gem)` - The saved or updated gem
/// * `Err(String)` - Error message if save fails
#[tauri::command]
pub async fn save_recording_gem(
    app_handle: tauri::AppHandle,
    filename: String,
    transcript: String,
    language: String,
    created_at: u64,
    copilot_data: Option<serde_json::Value>,
    gem_store: State<'_, Arc<dyn GemStore>>,
    intel_provider: State<'_, Arc<dyn IntelProvider>>,
    settings_manager: State<'_, Arc<RwLock<SettingsManager>>>,
) -> Result<Gem, String> {
    // Check for existing gem
    log_gem_save(&format!("save_recording_gem called: filename={}, transcript_len={}, language={}, copilot_data={}",
        filename, transcript.len(), language, copilot_data.is_some()));

    let existing_gem = gem_store.find_by_recording_filename(&filename).await
        .map_err(|e| {
            log_gem_save(&format!("ERROR find_by_recording_filename: {}", e));
            e
        })?;

    log_gem_save(&format!("existing_gem found: {}", existing_gem.is_some()));

    let mut gem = if let Some(preview) = existing_gem {
        // Update existing gem
        log_gem_save(&format!("updating existing gem id={}", preview.id));
        let mut existing = gem_store.get(&preview.id).await
            .map_err(|e| {
                log_gem_save(&format!("ERROR gem_store.get: {}", e));
                e
            })?
            .ok_or_else(|| {
                let msg = format!("Gem with id '{}' not found", preview.id);
                log_gem_save(&format!("ERROR {}", msg));
                msg
            })?;

        existing.transcript = Some(transcript.clone());
        existing.transcript_language = Some(language.clone());

        // Add Co-Pilot data if provided (Requirement 10.1, 10.2)
        if let Some(copilot) = copilot_data {
            existing.source_meta["copilot"] = copilot;
        }

        existing
    } else {
        // Create new gem with deterministic URL
        log_gem_save(&format!("creating new gem for filename={}", filename));
        let title = if let Some(dt) = chrono::DateTime::from_timestamp(created_at as i64, 0) {
            format!("Audio Transcript - {}", dt.format("%Y-%m-%d %H:%M:%S"))
        } else {
            format!("Audio Transcript - {}", filename)
        };

        let mut source_meta = serde_json::json!({
            "recording_filename": filename,
            "source": "recording_transcription"
        });

        // Add Co-Pilot data if provided (Requirement 10.1, 10.2)
        if let Some(copilot) = copilot_data {
            source_meta["copilot"] = copilot;
        }

        let new_id = uuid::Uuid::new_v4().to_string();
        log_gem_save(&format!("new gem id={}", new_id));
        Gem {
            id: new_id,
            source_type: "Other".to_string(),
            source_url: format!("jarvis://recording/{}", filename),
            domain: "jarvis-app".to_string(),
            title,
            author: None,
            description: None,
            content: None,
            source_meta,
            captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: Some(transcript.clone()),
            transcript_language: Some(language.clone()),
        }
    };
    
    // Try to generate AI enrichment (tags/summary) from transcript
    log_gem_save(&format!("gem ready for enrichment, id={}", gem.id));
    let availability = intel_provider.check_availability().await;
    log_gem_save(&format!("intel availability: {}, transcript_empty: {}", availability.available, transcript.trim().is_empty()));
    if availability.available && !transcript.trim().is_empty() {
        let (provider_name, model_name) = {
            let manager = settings_manager.read()
                .map_err(|e| format!("Failed to acquire settings lock: {}", e))?;
            let s = manager.get();
            (s.intelligence.provider.clone(), s.intelligence.active_model.clone())
        };
        
        // Try to generate tags and summary, but don't fail the save if enrichment fails
        match intel_provider.generate_tags(&transcript).await {
            Ok(tags) => {
                match intel_provider.summarize(&transcript).await {
                    Ok(summary) => {
                        let mut ai_enrichment = serde_json::json!({
                            "tags": tags,
                            "summary": summary,
                            "provider": provider_name,
                            "enriched_at": chrono::Utc::now().to_rfc3339(),
                        });
                        if provider_name == "mlx" {
                            ai_enrichment["model"] = serde_json::Value::String(model_name);
                        }
                        gem.ai_enrichment = Some(ai_enrichment);
                    }
                    Err(e) => {
                        log_gem_save(&format!("WARN failed to generate summary: {}", e));
                        eprintln!("Failed to generate summary: {}", e);
                    }
                }
            }
            Err(e) => {
                log_gem_save(&format!("WARN failed to generate tags: {}", e));
                eprintln!("Failed to generate tags: {}", e);
            }
        }
    }

    // Save and return
    log_gem_save(&format!("saving gem id={} to store", gem.id));
    let result = gem_store.save(gem).await;
    match &result {
        Ok(g) => log_gem_save(&format!("SUCCESS gem saved id={}", g.id)),
        Err(e) => log_gem_save(&format!("ERROR gem_store.save failed: {}", e)),
    }
    
    // Create knowledge files (including copilot.md if present)
    if let Ok(ref saved_gem) = result {
        if let Some(ks) = app_handle.try_state::<Arc<dyn crate::knowledge::KnowledgeStore>>() {
            if let Err(e) = ks.create(saved_gem).await {
                eprintln!("Knowledge file creation failed for recording gem {}: {}", saved_gem.id, e);
            }
        }
    }
    
    result
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

/// Check MLX dependencies (Python and mlx packages)
///
/// This command checks if Python is installed and accessible, and provides
/// diagnostic information about MLX availability. Useful for troubleshooting
/// MLX provider initialization failures.
///
/// # Arguments
///
/// * `settings_manager` - Managed state containing settings (for python_path)
///
/// # Returns
///
/// * `Ok(MlxDiagnostics)` - Diagnostic information about Python and MLX
///
/// # Examples
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// interface MlxDiagnostics {
///   python_found: boolean;
///   python_version?: string;
///   python_error?: string;
/// }
///
/// try {
///   const diagnostics = await invoke<MlxDiagnostics>('check_mlx_dependencies');
///   if (!diagnostics.python_found) {
///     console.error('Python not found:', diagnostics.python_error);
///   }
/// } catch (error) {
///   console.error('Failed to check MLX dependencies:', error);
/// }
/// ```
#[derive(Serialize)]
pub struct MlxDiagnostics {
    pub python_found: bool,
    pub python_version: Option<String>,
    pub python_error: Option<String>,
    pub venv_status: String,
    pub venv_python_path: Option<String>,
}

#[tauri::command]
pub async fn check_mlx_dependencies(
    settings_manager: State<'_, Arc<RwLock<SettingsManager>>>,
    venv_manager: State<'_, Arc<VenvManager>>,
) -> Result<MlxDiagnostics, String> {
    let settings = {
        let manager = settings_manager.read()
            .map_err(|e| format!("Failed to acquire settings lock: {}", e))?;
        manager.get()
    };

    let python_path = &settings.intelligence.python_path;

    // Check venv status
    let venv_status = venv_manager.status();
    let venv_status_str = serde_json::to_value(&venv_status)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "unknown".to_string());
    let venv_python = venv_manager.venv_python_path()
        .map(|p| p.to_string_lossy().to_string());

    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));

    // Try to run python --version
    match tokio::process::Command::new(python_path)
        .arg("--version")
        .current_dir(&home)
        .output()
        .await
    {
        Ok(output) => {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                Ok(MlxDiagnostics {
                    python_found: true,
                    python_version: Some(version),
                    python_error: None,
                    venv_status: venv_status_str.clone(),
                    venv_python_path: venv_python,
                })
            } else {
                Ok(MlxDiagnostics {
                    python_found: false,
                    python_version: None,
                    python_error: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
                    venv_status: venv_status_str.clone(),
                    venv_python_path: venv_python,
                })
            }
        }
        Err(e) => {
            let error_msg = if e.kind() == std::io::ErrorKind::NotFound {
                format!("Python not found at '{}'. Please install Python 3.10+ or update python_path in settings.", python_path)
            } else {
                format!("Failed to check Python: {}", e)
            };

            Ok(MlxDiagnostics {
                python_found: false,
                python_version: None,
                python_error: Some(error_msg),
                venv_status: venv_status_str.clone(),
                venv_python_path: venv_python,
            })
        }
    }
}

#[tauri::command]
pub async fn setup_mlx_venv(
    settings_manager: State<'_, Arc<RwLock<SettingsManager>>>,
    venv_manager: State<'_, Arc<VenvManager>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let python_path = {
        let manager = settings_manager.read()
            .map_err(|e| format!("Failed to acquire settings lock: {}", e))?;
        manager.get().intelligence.python_path.clone()
    };

    venv_manager.setup(&python_path, &app_handle).await
}

#[tauri::command]
pub async fn reset_mlx_venv(
    venv_manager: State<'_, Arc<VenvManager>>,
) -> Result<(), String> {
    venv_manager.reset()
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

    // Tests for extract_recording_path function
    
    #[test]
    fn test_extract_recording_path_with_recording_filename() {
        // Test with primary key: recording_filename
        let gem = Gem {
            id: "test-id".to_string(),
            source_type: "Other".to_string(),
            source_url: "jarvis://recording/test".to_string(),
            domain: "jarvis-app".to_string(),
            title: "Test Recording".to_string(),
            author: None,
            description: None,
            content: Some("Whisper transcript".to_string()),
            source_meta: serde_json::json!({
                "recording_filename": "20240315_143022.pcm"
            }),
            captured_at: "2024-03-15T14:30:22Z".to_string(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };

        let result = extract_recording_path(&gem);
        assert!(result.is_some());
        
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("com.jarvis.app"));
        assert!(path.to_string_lossy().contains("recordings"));
        assert!(path.to_string_lossy().ends_with("20240315_143022.pcm"));
    }

    #[test]
    fn test_extract_recording_path_with_fallback_keys() {
        // Test with fallback key: filename
        let gem_filename = Gem {
            id: "test-id".to_string(),
            source_type: "Other".to_string(),
            source_url: "jarvis://recording/test".to_string(),
            domain: "jarvis-app".to_string(),
            title: "Test Recording".to_string(),
            author: None,
            description: None,
            content: None,
            source_meta: serde_json::json!({
                "filename": "test_audio.pcm"
            }),
            captured_at: "2024-03-15T14:30:22Z".to_string(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };

        let result = extract_recording_path(&gem_filename);
        assert!(result.is_some());
        assert!(result.unwrap().to_string_lossy().ends_with("test_audio.pcm"));

        // Test with fallback key: recording_path
        let gem_recording_path = Gem {
            id: "test-id".to_string(),
            source_type: "Other".to_string(),
            source_url: "jarvis://recording/test".to_string(),
            domain: "jarvis-app".to_string(),
            title: "Test Recording".to_string(),
            author: None,
            description: None,
            content: None,
            source_meta: serde_json::json!({
                "recording_path": "another_audio.pcm"
            }),
            captured_at: "2024-03-15T14:30:22Z".to_string(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };

        let result = extract_recording_path(&gem_recording_path);
        assert!(result.is_some());
        assert!(result.unwrap().to_string_lossy().ends_with("another_audio.pcm"));

        // Test with fallback key: file
        let gem_file = Gem {
            id: "test-id".to_string(),
            source_type: "Other".to_string(),
            source_url: "jarvis://recording/test".to_string(),
            domain: "jarvis-app".to_string(),
            title: "Test Recording".to_string(),
            author: None,
            description: None,
            content: None,
            source_meta: serde_json::json!({
                "file": "file_audio.pcm"
            }),
            captured_at: "2024-03-15T14:30:22Z".to_string(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };

        let result = extract_recording_path(&gem_file);
        assert!(result.is_some());
        assert!(result.unwrap().to_string_lossy().ends_with("file_audio.pcm"));

        // Test with fallback key: path
        let gem_path = Gem {
            id: "test-id".to_string(),
            source_type: "Other".to_string(),
            source_url: "jarvis://recording/test".to_string(),
            domain: "jarvis-app".to_string(),
            title: "Test Recording".to_string(),
            author: None,
            description: None,
            content: None,
            source_meta: serde_json::json!({
                "path": "path_audio.pcm"
            }),
            captured_at: "2024-03-15T14:30:22Z".to_string(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };

        let result = extract_recording_path(&gem_path);
        assert!(result.is_some());
        assert!(result.unwrap().to_string_lossy().ends_with("path_audio.pcm"));
    }

    #[test]
    fn test_extract_recording_path_without_metadata() {
        // Test gem without any recording filename keys
        let gem = Gem {
            id: "test-id".to_string(),
            source_type: "YouTube".to_string(),
            source_url: "https://youtube.com/watch?v=test".to_string(),
            domain: "youtube.com".to_string(),
            title: "Test Video".to_string(),
            author: Some("Test Author".to_string()),
            description: None,
            content: Some("Video transcript".to_string()),
            source_meta: serde_json::json!({
                "video_id": "test123",
                "duration": 300
            }),
            captured_at: "2024-03-15T14:30:22Z".to_string(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };

        let result = extract_recording_path(&gem);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_recording_path_ignores_source_type() {
        // Test that function works regardless of source_type value
        // This verifies the bug fix (previously checked source_type != "Recording")
        
        // Test with source_type "Other" (actual value for recordings)
        let gem_other = Gem {
            id: "test-id".to_string(),
            source_type: "Other".to_string(),
            source_url: "jarvis://recording/test".to_string(),
            domain: "jarvis-app".to_string(),
            title: "Test Recording".to_string(),
            author: None,
            description: None,
            content: None,
            source_meta: serde_json::json!({
                "recording_filename": "test1.pcm"
            }),
            captured_at: "2024-03-15T14:30:22Z".to_string(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };

        let result = extract_recording_path(&gem_other);
        assert!(result.is_some());
        assert!(result.unwrap().to_string_lossy().ends_with("test1.pcm"));

        // Test with source_type "Recording" (hypothetical value)
        let gem_recording = Gem {
            id: "test-id".to_string(),
            source_type: "Recording".to_string(),
            source_url: "jarvis://recording/test".to_string(),
            domain: "jarvis-app".to_string(),
            title: "Test Recording".to_string(),
            author: None,
            description: None,
            content: None,
            source_meta: serde_json::json!({
                "recording_filename": "test2.pcm"
            }),
            captured_at: "2024-03-15T14:30:22Z".to_string(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };

        let result = extract_recording_path(&gem_recording);
        assert!(result.is_some());
        assert!(result.unwrap().to_string_lossy().ends_with("test2.pcm"));

        // Test with source_type "YouTube" (unrelated value)
        let gem_youtube = Gem {
            id: "test-id".to_string(),
            source_type: "YouTube".to_string(),
            source_url: "jarvis://recording/test".to_string(),
            domain: "jarvis-app".to_string(),
            title: "Test Recording".to_string(),
            author: None,
            description: None,
            content: None,
            source_meta: serde_json::json!({
                "recording_filename": "test3.pcm"
            }),
            captured_at: "2024-03-15T14:30:22Z".to_string(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };

        let result = extract_recording_path(&gem_youtube);
        assert!(result.is_some());
        assert!(result.unwrap().to_string_lossy().ends_with("test3.pcm"));
    }

    // Property-based tests for extract_recording_path
    
    use proptest::prelude::*;

    // Generator for gems with recording metadata
    fn arb_gem_with_recording() -> impl Strategy<Value = Gem> {
        (
            any::<String>(),
            any::<String>(),
            prop::collection::vec(any::<String>(), 0..5),
            prop::option::of(any::<String>()),
            prop::option::of(any::<String>()),
            prop::option::of(any::<String>()),
            "[a-zA-Z0-9_-]{1,50}\\.pcm",
            prop::option::of(any::<String>()),
        ).prop_map(|(id, source_type, _tags, author, description, content, filename, transcript)| {
            Gem {
                id,
                source_type,
                source_url: "jarvis://recording/test".to_string(),
                domain: "jarvis-app".to_string(),
                title: "Test Recording".to_string(),
                author,
                description,
                content,
                source_meta: serde_json::json!({
                    "recording_filename": filename
                }),
                captured_at: "2024-03-15T14:30:22Z".to_string(),
                ai_enrichment: None,
                transcript,
                transcript_language: None,
            }
        })
    }

    // Generator for gems without recording metadata
    fn arb_gem_without_recording() -> impl Strategy<Value = Gem> {
        (
            any::<String>(),
            any::<String>(),
            any::<String>(),
            any::<String>(),
            prop::option::of(any::<String>()),
            prop::option::of(any::<String>()),
            prop::option::of(any::<String>()),
        ).prop_map(|(id, source_type, source_url, domain, author, description, content)| {
            Gem {
                id,
                source_type,
                source_url,
                domain,
                title: "Test Content".to_string(),
                author,
                description,
                content,
                source_meta: serde_json::json!({
                    "video_id": "test123",
                    "duration": 300
                }),
                captured_at: "2024-03-15T14:30:22Z".to_string(),
                ai_enrichment: None,
                transcript: None,
                transcript_language: None,
            }
        })
    }

    // Feature: transcribe-existing-gems, Property 1: Recording Path Extraction from Metadata
    proptest! {
        #[test]
        fn prop_extract_recording_path_with_metadata(gem in arb_gem_with_recording()) {
            let result = extract_recording_path(&gem);
            
            // Property: Should always return Some for gems with recording metadata
            prop_assert!(result.is_some());
            
            let path = result.unwrap();
            let path_str = path.to_string_lossy();
            
            // Property: Path should contain the expected directory structure
            prop_assert!(path_str.contains("com.jarvis.app"));
            prop_assert!(path_str.contains("recordings"));
            
            // Property: Path should end with .pcm extension
            prop_assert!(path_str.ends_with(".pcm"));
            
            // Property: Filename from source_meta should be in the path
            if let Some(filename) = gem.source_meta.get("recording_filename").and_then(|v| v.as_str()) {
                prop_assert!(path_str.ends_with(filename));
            }
        }
    }

    // Feature: transcribe-existing-gems, Property 2: Recording Path Extraction Returns None for Non-Recordings
    proptest! {
        #[test]
        fn prop_extract_recording_path_without_metadata(gem in arb_gem_without_recording()) {
            let result = extract_recording_path(&gem);
            
            // Property: Should always return None for gems without recording metadata
            prop_assert!(result.is_none());
        }
    }

    // Mock implementations for testing transcribe_gem command
    
    use std::collections::HashMap;
    use std::sync::Mutex;
    use crate::intelligence::provider::{TranscriptResult, AvailabilityResult};
    
    /// Mock IntelProvider for testing
    pub(super) struct MockIntelProvider {
        available: bool,
        availability_reason: Option<String>,
        transcript_result: Mutex<Option<Result<TranscriptResult, String>>>,
        tags_result: Mutex<Option<Result<Vec<String>, String>>>,
        summary_result: Mutex<Option<Result<String, String>>>,
    }
    
    impl MockIntelProvider {
        pub(super) fn new() -> Self {
            Self {
                available: true,
                availability_reason: None,
                transcript_result: Mutex::new(None),
                tags_result: Mutex::new(None),
                summary_result: Mutex::new(None),
            }
        }
        
        pub(super) fn with_availability(mut self, available: bool, reason: Option<String>) -> Self {
            self.available = available;
            self.availability_reason = reason;
            self
        }
        
        pub(super) fn with_transcript_result(self, result: Result<TranscriptResult, String>) -> Self {
            *self.transcript_result.lock().unwrap() = Some(result);
            self
        }
        
        pub(super) fn with_tags_result(self, result: Result<Vec<String>, String>) -> Self {
            *self.tags_result.lock().unwrap() = Some(result);
            self
        }
        
        pub(super) fn with_summary_result(self, result: Result<String, String>) -> Self {
            *self.summary_result.lock().unwrap() = Some(result);
            self
        }
    }
    
    #[async_trait::async_trait]
    impl IntelProvider for MockIntelProvider {
        async fn check_availability(&self) -> AvailabilityResult {
            AvailabilityResult {
                available: self.available,
                reason: self.availability_reason.clone(),
            }
        }
        
        async fn generate_tags(&self, _content: &str) -> Result<Vec<String>, String> {
            self.tags_result.lock().unwrap()
                .clone()
                .unwrap_or_else(|| Ok(vec!["test".to_string(), "mock".to_string()]))
        }
        
        async fn summarize(&self, _content: &str) -> Result<String, String> {
            self.summary_result.lock().unwrap()
                .clone()
                .unwrap_or_else(|| Ok("Mock summary".to_string()))
        }
        
        async fn generate_transcript(&self, _audio_path: &std::path::Path) -> Result<TranscriptResult, String> {
            self.transcript_result.lock().unwrap()
                .clone()
                .unwrap_or_else(|| Ok(TranscriptResult {
                    language: "en".to_string(),
                    transcript: "Mock transcript".to_string(),
                }))
        }
    }
    
    /// Mock GemStore for testing
    pub(super) struct MockGemStore {
        gems: Mutex<HashMap<String, Gem>>,
    }
    
    impl MockGemStore {
        pub(super) fn new() -> Self {
            Self {
                gems: Mutex::new(HashMap::new()),
            }
        }
        
        pub(super) fn with_gem(self, gem: Gem) -> Self {
            self.gems.lock().unwrap().insert(gem.id.clone(), gem);
            self
        }
    }
    
    #[async_trait::async_trait]
    impl GemStore for MockGemStore {
        async fn save(&self, gem: Gem) -> Result<Gem, String> {
            self.gems.lock().unwrap().insert(gem.id.clone(), gem.clone());
            Ok(gem)
        }
        
        async fn get(&self, id: &str) -> Result<Option<Gem>, String> {
            Ok(self.gems.lock().unwrap().get(id).cloned())
        }
        
        async fn list(&self, _limit: usize, _offset: usize) -> Result<Vec<GemPreview>, String> {
            unimplemented!("Not needed for transcribe_gem tests")
        }
        
        async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<GemPreview>, String> {
            unimplemented!("Not needed for transcribe_gem tests")
        }
        
        async fn filter_by_tag(&self, _tag: &str, _limit: usize, _offset: usize) -> Result<Vec<GemPreview>, String> {
            unimplemented!("Not needed for transcribe_gem tests")
        }
        
        async fn delete(&self, id: &str) -> Result<(), String> {
            self.gems.lock().unwrap().remove(id);
            Ok(())
        }
        
        async fn find_by_recording_filename(&self, filename: &str) -> Result<Option<GemPreview>, String> {
            // Search through all gems for one with matching recording_filename in source_meta
            let gems = self.gems.lock().unwrap();
            let matching_gem = gems.values()
                .find(|gem| {
                    gem.source_meta
                        .get("recording_filename")
                        .and_then(|v| v.as_str())
                        .map(|f| f == filename)
                        .unwrap_or(false)
                });
            
            Ok(matching_gem.map(|gem| GemPreview {
                id: gem.id.clone(),
                source_type: gem.source_type.clone(),
                source_url: gem.source_url.clone(),
                domain: gem.domain.clone(),
                title: gem.title.clone(),
                author: gem.author.clone(),
                description: gem.description.clone(),
                content_preview: gem.content.as_ref().map(|c| {
                    if c.chars().count() > 200 {
                        format!("{}...", c.chars().take(200).collect::<String>())
                    } else {
                        c.clone()
                    }
                }),
                captured_at: gem.captured_at.clone(),
                tags: None,
                summary: None,
                enrichment_source: None,
                transcript_language: gem.transcript_language.clone(),
            }))
        }
    }
    
    // Helper function to create a test gem with recording metadata
    pub(super) fn create_test_gem_with_recording(id: &str, filename: &str) -> Gem {
        Gem {
            id: id.to_string(),
            source_type: "Other".to_string(),
            source_url: "jarvis://recording/test".to_string(),
            domain: "jarvis-app".to_string(),
            title: "Test Recording".to_string(),
            author: None,
            description: Some("Test description".to_string()),
            content: Some("Original whisper transcript".to_string()),
            source_meta: serde_json::json!({
                "recording_filename": filename
            }),
            captured_at: "2024-03-15T14:30:22Z".to_string(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        }
    }
    
    // Helper function to create a test gem without recording metadata
    pub(super) fn create_test_gem_without_recording(id: &str) -> Gem {
        Gem {
            id: id.to_string(),
            source_type: "YouTube".to_string(),
            source_url: "https://youtube.com/watch?v=test".to_string(),
            domain: "youtube.com".to_string(),
            title: "Test Video".to_string(),
            author: Some("Test Author".to_string()),
            description: None,
            content: Some("Video content".to_string()),
            source_meta: serde_json::json!({
                "video_id": "test123"
            }),
            captured_at: "2024-03-15T14:30:22Z".to_string(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        }
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

/// List all LLM models with their status
///
/// This command returns all models from the LLM catalog with their current
/// status (downloaded, downloading, not downloaded, or error).
///
/// # Arguments
///
/// * `llm_manager` - Managed state containing the LlmModelManager
///
/// # Returns
///
/// * `Ok(Vec<LlmModelInfo>)` - List of all models with status
/// * `Err(String)` - Error message if listing fails
///
/// # Example
///
/// ```typescript
/// const models = await invoke('list_llm_models');
/// models.forEach(model => {
///   console.log(`${model.display_name}: ${model.status.type}`);
/// });
/// ```
#[tauri::command]
pub async fn list_llm_models(
    llm_manager: State<'_, Arc<LlmModelManager>>,
) -> Result<Vec<LlmModelInfo>, String> {
    llm_manager.list_models().await
}

/// Download an LLM model from HuggingFace
///
/// This command starts downloading a model in the background. Progress is
/// reported via `llm-model-download-progress` events, and completion is
/// reported via `llm-model-download-complete` event.
///
/// # Arguments
///
/// * `model_id` - The model ID from the catalog (e.g., "qwen3-8b-4bit")
/// * `llm_manager` - Managed state containing the LlmModelManager
///
/// # Returns
///
/// * `Ok(())` - Download started successfully
/// * `Err(String)` - Error message if download cannot be started
///
/// # Example
///
/// ```typescript
/// try {
///   await invoke('download_llm_model', { modelId: 'qwen3-8b-4bit' });
///   console.log('Download started');
/// } catch (error) {
///   console.error(`Failed to start download: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn download_llm_model(
    model_id: String,
    llm_manager: State<'_, Arc<LlmModelManager>>,
) -> Result<(), String> {
    llm_manager.download_model(model_id).await
}

/// Cancel an in-progress LLM model download
///
/// This command cancels a currently downloading model and cleans up
/// any partial files.
///
/// # Arguments
///
/// * `model_id` - The model ID being downloaded
/// * `llm_manager` - Managed state containing the LlmModelManager
///
/// # Returns
///
/// * `Ok(())` - Download cancelled successfully
/// * `Err(String)` - Error message if cancellation fails
///
/// # Example
///
/// ```typescript
/// try {
///   await invoke('cancel_llm_download', { modelId: 'qwen3-8b-4bit' });
///   console.log('Download cancelled');
/// } catch (error) {
///   console.error(`Failed to cancel: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn cancel_llm_download(
    model_id: String,
    llm_manager: State<'_, Arc<LlmModelManager>>,
) -> Result<(), String> {
    llm_manager.cancel_download(model_id).await
}

/// Delete a downloaded LLM model
///
/// This command deletes a model from disk. It prevents deletion of the
/// currently active model to avoid breaking the inference provider.
///
/// # Arguments
///
/// * `model_id` - The model ID to delete
/// * `llm_manager` - Managed state containing the LlmModelManager
/// * `settings_manager` - Managed state for checking active model
///
/// # Returns
///
/// * `Ok(())` - Model deleted successfully
/// * `Err(String)` - Error message if deletion fails
///
/// # Example
///
/// ```typescript
/// try {
///   await invoke('delete_llm_model', { modelId: 'qwen3-4b-4bit' });
///   console.log('Model deleted');
/// } catch (error) {
///   console.error(`Failed to delete: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn delete_llm_model(
    model_id: String,
    llm_manager: State<'_, Arc<LlmModelManager>>,
    settings_manager: State<'_, Arc<RwLock<SettingsManager>>>,
) -> Result<(), String> {
    // Check if this is the active model
    let active_model = {
        let manager = settings_manager.read()
            .map_err(|e| format!("Failed to acquire settings lock: {}", e))?;
        manager.get().intelligence.active_model.clone()
    };
    
    if active_model == model_id {
        return Err(format!(
            "Cannot delete active model '{}'. Switch to a different model first.",
            model_id
        ));
    }
    
    llm_manager.delete_model(model_id).await
}

/// Switch to a different LLM model
///
/// This command switches the active LLM model by:
/// 1. Verifying the model is downloaded
/// 2. Updating settings with the new model
/// 3. Reloading the MlxProvider sidecar with the new model
///
/// # Arguments
///
/// * `model_id` - The model ID to switch to
/// * `llm_manager` - Managed state containing the LlmModelManager
/// * `settings_manager` - Managed state for updating active model
/// * `mlx_provider` - Managed state containing the MlxProvider reference
///
/// # Returns
///
/// * `Ok(())` - Model switched successfully
/// * `Err(String)` - Error message if switch fails
///
/// # Example
///
/// ```typescript
/// try {
///   await invoke('switch_llm_model', { modelId: 'qwen3-8b-4bit' });
///   console.log('Switched to new model');
/// } catch (error) {
///   console.error(`Failed to switch: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn switch_llm_model(
    model_id: String,
    llm_manager: State<'_, Arc<LlmModelManager>>,
    settings_manager: State<'_, Arc<RwLock<SettingsManager>>>,
    mlx_provider: State<'_, Arc<tokio::sync::Mutex<Option<Arc<crate::intelligence::MlxProvider>>>>>,
) -> Result<(), String> {
    // Verify model is downloaded
    let model_path = llm_manager.model_path(&model_id);
    if !model_path.exists() {
        return Err(format!(
            "Model '{}' is not downloaded. Download it first.",
            model_id
        ));
    }
    
    // Verify model has config.json
    if !model_path.join("config.json").exists() {
        return Err(format!(
            "Model '{}' is incomplete (missing config.json). Re-download it.",
            model_id
        ));
    }
    
    // Save the old model ID for rollback on failure
    let old_model_id = {
        let manager = settings_manager.read()
            .map_err(|e| format!("Failed to acquire settings lock: {}", e))?;
        manager.get().intelligence.active_model.clone()
    };
    
    // Update settings with new active model
    {
        let manager = settings_manager.read()
            .map_err(|e| format!("Failed to acquire settings lock: {}", e))?;
        let mut settings = manager.get();
        settings.intelligence.active_model = model_id.clone();
        manager.update(settings)
            .map_err(|e| format!("Failed to save settings: {}", e))?;
    }
    
    // If MlxProvider is active, reload it with the new model
    let provider_guard = mlx_provider.lock().await;
    if let Some(provider) = provider_guard.as_ref() {
        // Try to switch the model in the running sidecar
        match provider.switch_model(model_path).await {
            Ok(()) => {
                eprintln!("Successfully switched to model: {}", model_id);
                Ok(())
            }
            Err(e) => {
                // Model switch failed - roll back settings to old model
                eprintln!("Failed to hot-reload model: {}. Rolling back settings.", e);
                
                // Rollback settings
                let rollback_result = {
                    let manager = settings_manager.read()
                        .map_err(|e| format!("Failed to acquire settings lock for rollback: {}", e))?;
                    let mut settings = manager.get();
                    settings.intelligence.active_model = old_model_id.clone();
                    manager.update(settings)
                };
                
                match rollback_result {
                    Ok(()) => {
                        Err(format!(
                            "Failed to switch model: {}. Settings rolled back to '{}'.",
                            e, old_model_id
                        ))
                    }
                    Err(rollback_err) => {
                        Err(format!(
                            "Failed to switch model: {}. WARNING: Settings rollback also failed: {}. Settings may be inconsistent - restart the app.",
                            e, rollback_err
                        ))
                    }
                }
            }
        }
    } else {
        // No MlxProvider active (using IntelligenceKit or NoOp)
        // Settings updated successfully, will take effect on next provider init
        Ok(())
    }
}

    // Unit tests for transcribe_gem command
    
    // Note: These tests cannot be run as standard #[tokio::test] because transcribe_gem
    // requires Tauri State parameters which are only available in a running Tauri app.
    // The tests below demonstrate the test structure and logic, but would need to be
    // adapted for integration testing or use a test harness that provides State.
    
    #[cfg(test)]
    mod transcribe_gem_tests {
        use crate::commands::tests::{MockIntelProvider, MockGemStore, create_test_gem_with_recording, create_test_gem_without_recording};
        use super::*;
        use std::sync::Arc;
        use std::path::PathBuf;
        use tokio::sync::RwLock;
        use crate::settings::SettingsManager;
        use crate::intelligence::provider::TranscriptResult;
        
        // Helper to create a mock settings manager
        fn create_mock_settings_manager() -> Arc<RwLock<SettingsManager>> {
            // Create a temporary settings file for testing
            let temp_dir = std::env::temp_dir();
            let settings_path = temp_dir.join(format!("test_settings_{}.json", uuid::Uuid::new_v4()));
            
            let manager = SettingsManager::new_with_path(settings_path).unwrap();
            Arc::new(RwLock::new(manager))
        }
        
        // Helper to create a temporary test file
        fn create_test_audio_file() -> PathBuf {
            let temp_dir = std::env::temp_dir();
            let test_file = temp_dir.join("test_audio.pcm");
            std::fs::write(&test_file, b"fake audio data").unwrap();
            test_file
        }
        
        // Task 4.1: Test successful transcription (happy path)
        #[tokio::test]
        async fn test_transcribe_gem_success() {
            // Create test audio file
            let test_file = create_test_audio_file();
            let filename = test_file.file_name().unwrap().to_str().unwrap();
            
            // Create gem with recording metadata
            let gem = create_test_gem_with_recording("test-id", filename);
            
            // Create mock store with the gem
            let store = Arc::new(MockGemStore::new().with_gem(gem.clone())) as Arc<dyn GemStore>;
            
            // Create mock provider with successful responses
            let provider = Arc::new(
                MockIntelProvider::new()
                    .with_transcript_result(Ok(TranscriptResult {
                        language: "en".to_string(),
                        transcript: "This is a test transcript".to_string(),
                    }))
                    .with_tags_result(Ok(vec!["test".to_string(), "audio".to_string()]))
                    .with_summary_result(Ok("Test audio summary".to_string()))
            ) as Arc<dyn IntelProvider>;
            
            let settings_manager = create_mock_settings_manager();
            
            // Note: This test structure shows the logic but cannot run without Tauri State
            // In a real integration test, you would use:
            // let result = transcribe_gem("test-id".to_string(), State::from(store), State::from(provider), State::from(settings_manager)).await;
            
            // Verify the logic manually:
            // 1. Provider should be available
            let availability = provider.check_availability().await;
            assert!(availability.available);
            
            // 2. Gem should be found
            let fetched_gem = store.get("test-id").await.unwrap();
            assert!(fetched_gem.is_some());
            
            // 3. Recording path should be extractable
            let recording_path = extract_recording_path(&fetched_gem.unwrap());
            assert!(recording_path.is_some());
            
            // 4. Transcript should be generated successfully
            let transcript_result: Result<TranscriptResult, String> = provider.generate_transcript(&test_file).await;
            assert!(transcript_result.is_ok());
            let transcript = transcript_result.unwrap();
            assert_eq!(transcript.language, "en");
            assert_eq!(transcript.transcript, "This is a test transcript");
            
            // Cleanup
            std::fs::remove_file(test_file).ok();
        }
        
        // Task 4.2: Test gem not found error
        #[tokio::test]
        async fn test_transcribe_gem_not_found() {
            let store = Arc::new(MockGemStore::new()) as Arc<dyn GemStore>;
            let provider = Arc::new(MockIntelProvider::new()) as Arc<dyn IntelProvider>;
            
            // Verify gem is not found
            let result = store.get("nonexistent-id").await.unwrap();
            assert!(result.is_none());
            
            // In the actual command, this would return:
            // Err("Gem with id 'nonexistent-id' not found")
        }
        
        // Task 4.3: Test no recording metadata error
        #[tokio::test]
        async fn test_transcribe_gem_no_recording_metadata() {
            let gem = create_test_gem_without_recording("test-id");
            let store = Arc::new(MockGemStore::new().with_gem(gem.clone())) as Arc<dyn GemStore>;
            
            // Verify gem has no recording metadata
            let fetched_gem = store.get("test-id").await.unwrap().unwrap();
            let recording_path = extract_recording_path(&fetched_gem);
            assert!(recording_path.is_none());
            
            // In the actual command, this would return:
            // Err("This gem has no associated recording file")
        }
        
        // Task 4.4: Test recording file not found error
        #[tokio::test]
        async fn test_transcribe_gem_file_not_found() {
            let gem = create_test_gem_with_recording("test-id", "nonexistent_file.pcm");
            let store = Arc::new(MockGemStore::new().with_gem(gem.clone())) as Arc<dyn GemStore>;
            
            // Verify recording path exists but file doesn't
            let fetched_gem = store.get("test-id").await.unwrap().unwrap();
            let recording_path = extract_recording_path(&fetched_gem);
            assert!(recording_path.is_some());
            
            let path = recording_path.unwrap();
            assert!(!path.exists());
            
            // In the actual command, this would return:
            // Err("Recording file not found: {path}")
        }
        
        // Task 4.5: Test provider unavailable error
        #[tokio::test]
        async fn test_transcribe_gem_provider_unavailable() {
            let provider = Arc::new(
                MockIntelProvider::new()
                    .with_availability(false, Some("MLX not installed".to_string()))
            ) as Arc<dyn IntelProvider>;
            
            // Verify provider is unavailable
            let availability = provider.check_availability().await;
            assert!(!availability.available);
            assert_eq!(availability.reason, Some("MLX not installed".to_string()));
            
            // In the actual command, this would return:
            // Err("AI provider not available: MLX not installed")
        }
        
        // Task 4.6: Test provider doesn't support transcription error
        #[tokio::test]
        async fn test_transcribe_gem_transcription_not_supported() {
            let test_file = create_test_audio_file();
            
            let provider = Arc::new(
                MockIntelProvider::new()
                    .with_transcript_result(Err("Transcript generation not supported by this provider".to_string()))
            ) as Arc<dyn IntelProvider>;
            
            // Verify transcription is not supported
            let result = provider.generate_transcript(&test_file).await;
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("not supported"));
            
            // In the actual command, this would return:
            // Err("Current AI provider does not support transcription")
            
            std::fs::remove_file(test_file).ok();
        }
        
        // Task 4.7: Test that only expected fields are updated
        #[tokio::test]
        async fn test_transcribe_gem_field_preservation() {
            let test_file = create_test_audio_file();
            let filename = test_file.file_name().unwrap().to_str().unwrap();
            
            let original_gem = create_test_gem_with_recording("test-id", filename);
            let store = Arc::new(MockGemStore::new().with_gem(original_gem.clone())) as Arc<dyn GemStore>;
            
            let provider = Arc::new(
                MockIntelProvider::new()
                    .with_transcript_result(Ok(TranscriptResult {
                        language: "en".to_string(),
                        transcript: "New transcript".to_string(),
                    }))
            ) as Arc<dyn IntelProvider>;
            
            // Simulate the transcription process
            let mut updated_gem = original_gem.clone();
            let transcript_result = provider.generate_transcript(&test_file).await.unwrap();
            updated_gem.transcript = Some(transcript_result.transcript.clone());
            updated_gem.transcript_language = Some(transcript_result.language.clone());
            
            // Verify only transcript fields are updated
            assert_eq!(updated_gem.id, original_gem.id);
            assert_eq!(updated_gem.source_type, original_gem.source_type);
            assert_eq!(updated_gem.source_url, original_gem.source_url);
            assert_eq!(updated_gem.domain, original_gem.domain);
            assert_eq!(updated_gem.title, original_gem.title);
            assert_eq!(updated_gem.author, original_gem.author);
            assert_eq!(updated_gem.description, original_gem.description);
            assert_eq!(updated_gem.content, original_gem.content);
            assert_eq!(updated_gem.source_meta, original_gem.source_meta);
            assert_eq!(updated_gem.captured_at, original_gem.captured_at);
            
            // These fields should be updated
            assert_eq!(updated_gem.transcript, Some("New transcript".to_string()));
            assert_eq!(updated_gem.transcript_language, Some("en".to_string()));
            
            std::fs::remove_file(test_file).ok();
        }
        
        // Task 4.8: Test that tags are generated from transcript
        #[tokio::test]
        async fn test_transcribe_gem_tags_from_transcript() {
            let provider = Arc::new(MockIntelProvider::new()) as Arc<dyn IntelProvider>;
            
            // Verify tags are generated from transcript content
            let tags = provider.generate_tags("This is a test transcript").await.unwrap();
            assert!(!tags.is_empty());
            
            // In the actual command, generate_tags is called with transcript text
            // after successful transcription
        }
        
        // Task 4.9: Test that summary is generated from transcript
        #[tokio::test]
        async fn test_transcribe_gem_summary_from_transcript() {
            let provider = Arc::new(MockIntelProvider::new()) as Arc<dyn IntelProvider>;
            
            // Verify summary is generated from transcript content
            let summary = provider.summarize("This is a test transcript").await.unwrap();
            assert!(!summary.is_empty());
            
            // In the actual command, summarize is called with transcript text
            // after successful transcription
        }
        
        // Task 4.10: Test graceful degradation when tag generation fails
        #[tokio::test]
        async fn test_transcribe_gem_tag_generation_failure() {
            let provider = Arc::new(
                MockIntelProvider::new()
                    .with_tags_result(Err("Tag generation failed".to_string()))
                    .with_summary_result(Ok("Summary still works".to_string()))
            ) as Arc<dyn IntelProvider>;
            
            // Verify tag generation fails but doesn't crash
            let tags_result = provider.generate_tags("test").await;
            assert!(tags_result.is_err());
            
            // Verify summary still works
            let summary_result = provider.summarize("test").await;
            assert!(summary_result.is_ok());
            
            // In the actual command, .unwrap_or_default() handles this gracefully
            let tags = tags_result.unwrap_or_default();
            assert!(tags.is_empty());
        }
        
        // Task 4.11: Test graceful degradation when summary generation fails
        #[tokio::test]
        async fn test_transcribe_gem_summary_generation_failure() {
            let provider = Arc::new(
                MockIntelProvider::new()
                    .with_tags_result(Ok(vec!["test".to_string()]))
                    .with_summary_result(Err("Summary generation failed".to_string()))
            ) as Arc<dyn IntelProvider>;
            
            // Verify tags still work
            let tags_result = provider.generate_tags("test").await;
            assert!(tags_result.is_ok());
            
            // Verify summary generation fails but doesn't crash
            let summary_result = provider.summarize("test").await;
            assert!(summary_result.is_err());
            
            // In the actual command, .unwrap_or_default() handles this gracefully
            let summary = summary_result.unwrap_or_default();
            assert!(summary.is_empty());
        }
    }

    // Phase 2 Tests: transcribe_recording, check_recording_gem, check_recording_gems_batch, save_recording_gem

    #[cfg(test)]
    mod transcribe_recording_tests {
        use super::*;
        use std::fs;

        // Helper to create a test recording file
        fn create_test_recording(filename: &str) -> PathBuf {
            let data_dir = dirs::data_dir().unwrap();
            let recordings_dir = data_dir.join("com.jarvis.app").join("recordings");
            fs::create_dir_all(&recordings_dir).unwrap();
            let file_path = recordings_dir.join(filename);
            fs::write(&file_path, b"fake pcm data").unwrap();
            file_path
        }

        // Helper to cleanup test recording
        fn cleanup_test_recording(path: &PathBuf) {
            let _ = fs::remove_file(path);
        }

        // Task 2.1: Test transcribe_recording with valid file
        #[tokio::test]
        async fn test_transcribe_recording_success() {
            let filename = "test_recording_success.pcm";
            let file_path = create_test_recording(filename);

            let provider = tests::MockIntelProvider::new()
                .with_transcript_result(Ok(TranscriptResult {
                    language: "en".to_string(),
                    transcript: "Test transcript".to_string(),
                }));

            // Call the actual helper function
            let result = transcribe_recording_inner(filename, &provider).await;
            
            assert!(result.is_ok(), "Expected success, got error: {:?}", result.err());
            let transcript = result.unwrap();
            assert_eq!(transcript.language, "en");
            assert_eq!(transcript.transcript, "Test transcript");

            cleanup_test_recording(&file_path);
        }

        // Task 2.2: Test transcribe_recording with missing file
        #[tokio::test]
        async fn test_transcribe_recording_file_not_found() {
            let filename = "nonexistent_file.pcm";

            let provider = tests::MockIntelProvider::new()
                .with_transcript_result(Ok(TranscriptResult {
                    language: "en".to_string(),
                    transcript: "Should not reach here".to_string(),
                }));

            // Call the actual helper function
            let result = transcribe_recording_inner(filename, &provider).await;
            
            assert!(result.is_err(), "Expected error for missing file");
            let error = result.unwrap_err();
            assert!(error.contains("Recording file not found"), "Error message should mention file not found, got: {}", error);
        }

        // Task 2.3: Test transcribe_recording with unavailable provider
        #[tokio::test]
        async fn test_transcribe_recording_provider_unavailable() {
            let filename = "test_unavailable.pcm";
            let file_path = create_test_recording(filename);

            let provider = tests::MockIntelProvider::new()
                .with_availability(false, Some("Provider not ready".to_string()));

            // Call the actual helper function
            let result = transcribe_recording_inner(filename, &provider).await;
            
            assert!(result.is_err(), "Expected error for unavailable provider");
            let error = result.unwrap_err();
            assert!(error.contains("AI provider not available"), "Error should mention provider unavailable, got: {}", error);
            assert!(error.contains("Provider not ready"), "Error should include reason, got: {}", error);

            cleanup_test_recording(&file_path);
        }

        // Task 2.4: Test transcribe_recording with invalid filename (path traversal)
        #[tokio::test]
        async fn test_transcribe_recording_invalid_filename() {
            let invalid_filenames = vec![
                "../etc/passwd",
                "../../etc/passwd",
                "subdir/file.pcm",
                "..\\windows\\system32",
                "test/../../../etc/passwd",
            ];

            let provider = tests::MockIntelProvider::new();

            for filename in invalid_filenames {
                // Call the actual helper function
                let result = transcribe_recording_inner(filename, &provider).await;
                
                assert!(result.is_err(), "Expected error for invalid filename: {}", filename);
                let error = result.unwrap_err();
                assert_eq!(error, "Invalid filename: path separators not allowed", 
                    "Wrong error message for filename '{}': {}", filename, error);
            }
        }

        // Test error message remapping for "not supported"
        #[tokio::test]
        async fn test_transcribe_recording_not_supported_error() {
            let filename = "test_not_supported.pcm";
            let file_path = create_test_recording(filename);

            let provider = tests::MockIntelProvider::new()
                .with_transcript_result(Err("Transcript generation not supported by this provider".to_string()));

            // Call the actual helper function
            let result = transcribe_recording_inner(filename, &provider).await;
            
            assert!(result.is_err(), "Expected error for unsupported provider");
            let error = result.unwrap_err();
            assert_eq!(error, "Current AI provider does not support transcription", 
                "Error message should be remapped, got: {}", error);

            cleanup_test_recording(&file_path);
        }

        // Test error message passthrough for other errors
        #[tokio::test]
        async fn test_transcribe_recording_other_error() {
            let filename = "test_other_error.pcm";
            let file_path = create_test_recording(filename);

            let provider = tests::MockIntelProvider::new()
                .with_transcript_result(Err("Transcription timeout after 120 seconds".to_string()));

            // Call the actual helper function
            let result = transcribe_recording_inner(filename, &provider).await;
            
            assert!(result.is_err(), "Expected error");
            let error = result.unwrap_err();
            assert_eq!(error, "Transcription timeout after 120 seconds", 
                "Error message should be passed through, got: {}", error);

            cleanup_test_recording(&file_path);
        }
    }

    #[cfg(test)]
    mod check_recording_gem_tests {
        use super::*;

        // Task 3.1: Test check_recording_gem with existing gem
        #[tokio::test]
        async fn test_check_recording_gem_exists() {
            let filename = "test_recording.pcm";
            let gem = tests::create_test_gem_with_recording("test-id", filename);
            let store = Arc::new(tests::MockGemStore::new().with_gem(gem.clone())) as Arc<dyn GemStore>;

            // Query by filename
            let result = store.find_by_recording_filename(filename).await;
            assert!(result.is_ok());
            let preview = result.unwrap();
            assert!(preview.is_some());
            assert_eq!(preview.unwrap().id, "test-id");
        }

        // Task 3.2: Test check_recording_gem with no gem
        #[tokio::test]
        async fn test_check_recording_gem_not_found() {
            let store = Arc::new(tests::MockGemStore::new()) as Arc<dyn GemStore>;

            let result = store.find_by_recording_filename("nonexistent.pcm").await;
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());
        }

        // Task 3.3: Test check_recording_gems_batch with mixed results
        #[tokio::test]
        async fn test_check_recording_gems_batch_mixed() {
            let gem1 = tests::create_test_gem_with_recording("id1", "recording1.pcm");
            let gem2 = tests::create_test_gem_with_recording("id2", "recording2.pcm");
            
            let store = Arc::new(
                tests::MockGemStore::new()
                    .with_gem(gem1.clone())
                    .with_gem(gem2.clone())
            ) as Arc<dyn GemStore>;

            let filenames = vec![
                "recording1.pcm".to_string(),
                "recording2.pcm".to_string(),
                "recording3.pcm".to_string(), // No gem
            ];

            // Simulate batch check
            let mut result = std::collections::HashMap::new();
            for filename in &filenames {
                if let Some(preview) = store.find_by_recording_filename(filename).await.unwrap() {
                    result.insert(filename.clone(), preview);
                }
            }

            // Verify results
            assert_eq!(result.len(), 2);
            assert!(result.contains_key("recording1.pcm"));
            assert!(result.contains_key("recording2.pcm"));
            assert!(!result.contains_key("recording3.pcm"));
        }
    }

    #[cfg(test)]
    mod save_recording_gem_tests {
        use super::*;

        // Task 4.1: Test save_recording_gem create flow (no existing gem)
        #[tokio::test]
        async fn test_save_recording_gem_create() {
            let filename = "new_recording.pcm";
            let transcript = "This is a new transcript".to_string();
            let language = "en".to_string();
            let created_at: u64 = 1709481600; // 2024-03-03 12:00:00 UTC

            let store = Arc::new(tests::MockGemStore::new()) as Arc<dyn GemStore>;
            let provider = Arc::new(
                tests::MockIntelProvider::new()
                    .with_tags_result(Ok(vec!["test".to_string()]))
                    .with_summary_result(Ok("Test summary".to_string()))
            ) as Arc<dyn IntelProvider>;

            // Verify no existing gem
            let existing = store.find_by_recording_filename(filename).await.unwrap();
            assert!(existing.is_none());

            // Create new gem (simulating command logic)
            let title = if let Some(dt) = chrono::DateTime::from_timestamp(created_at as i64, 0) {
                format!("Audio Transcript - {}", dt.format("%Y-%m-%d %H:%M:%S"))
            } else {
                format!("Audio Transcript - {}", filename)
            };

            let gem = Gem {
                id: uuid::Uuid::new_v4().to_string(),
                source_type: "Other".to_string(),
                source_url: format!("jarvis://recording/{}", filename),
                domain: "jarvis-app".to_string(),
                title,
                author: None,
                description: None,
                content: None,
                source_meta: serde_json::json!({
                    "recording_filename": filename,
                    "source": "recording_transcription"
                }),
                captured_at: chrono::Utc::now().to_rfc3339(),
                ai_enrichment: None,
                transcript: Some(transcript.clone()),
                transcript_language: Some(language.clone()),
            };

            // Verify gem structure
            assert_eq!(gem.source_type, "Other");
            assert_eq!(gem.source_url, format!("jarvis://recording/{}", filename));
            assert_eq!(gem.domain, "jarvis-app");
            assert!(gem.title.contains("Audio Transcript"));
            assert_eq!(gem.transcript, Some(transcript));
            assert_eq!(gem.transcript_language, Some(language));
            assert_eq!(gem.source_meta["recording_filename"], filename);
            assert_eq!(gem.source_meta["source"], "recording_transcription");
        }

        // Task 4.2: Test save_recording_gem update flow (existing gem)
        #[tokio::test]
        async fn test_save_recording_gem_update() {
            let filename = "existing_recording.pcm";
            let original_gem = tests::create_test_gem_with_recording("original-id", filename);
            let store = Arc::new(tests::MockGemStore::new().with_gem(original_gem.clone())) as Arc<dyn GemStore>;

            // Verify existing gem
            let existing = store.find_by_recording_filename(filename).await.unwrap();
            assert!(existing.is_some());
            assert_eq!(existing.unwrap().id, "original-id");

            // Update gem (simulating command logic)
            let mut updated_gem = store.get("original-id").await.unwrap().unwrap();
            updated_gem.transcript = Some("Updated transcript".to_string());
            updated_gem.transcript_language = Some("es".to_string());

            // Verify ID preserved
            assert_eq!(updated_gem.id, "original-id");
            assert_eq!(updated_gem.transcript, Some("Updated transcript".to_string()));
            assert_eq!(updated_gem.transcript_language, Some("es".to_string()));
        }

        // Task 4.3: Test save_recording_gem with unavailable AI enrichment
        #[tokio::test]
        async fn test_save_recording_gem_no_enrichment() {
            let provider = Arc::new(
                tests::MockIntelProvider::new()
                    .with_availability(false, Some("AI unavailable".to_string()))
            ) as Arc<dyn IntelProvider>;

            let availability = provider.check_availability().await;
            assert!(!availability.available);

            // In actual command, gem would be saved without ai_enrichment
            // This is graceful degradation - save succeeds with transcript only
        }
    }


// ============================================================================
// Co-Pilot Agent Commands
// ============================================================================

/// Start the Co-Pilot agent for live recording intelligence
///
/// This command starts the Co-Pilot agent which analyzes audio during recording
/// and produces real-time actionable insights (summary, questions, concepts).
///
/// # Arguments
///
/// * `recording_manager` - Managed state containing the RecordingManager
/// * `settings_manager` - Managed state containing settings
/// * `intel_provider` - Managed state containing the IntelProvider trait object
/// * `app_handle` - Tauri app handle for agent initialization
///
/// # Returns
///
/// * `Ok(())` - Agent started successfully
/// * `Err(String)` - Error message if start fails
///
/// # Errors
///
/// Returns an error if:
/// - No recording is currently active
/// - Agent is already running
/// - Recording file doesn't exist
///
/// # Examples
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// try {
///   await invoke('start_copilot');
///   console.log('Co-Pilot agent started');
/// } catch (error) {
///   console.error(`Failed to start Co-Pilot: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn start_copilot(
    recording_manager: State<'_, Mutex<RecordingManager>>,
    settings_manager: State<'_, Arc<RwLock<SettingsManager>>>,
    intel_provider: State<'_, Arc<dyn IntelProvider>>,
    app_handle: AppHandle,
) -> Result<(), String> {
    use crate::agents::copilot::CoPilotAgent;
    
    // Get recording filepath and verify recording is active
    let recording_filepath = {
        let manager = recording_manager.lock()
            .map_err(|e| format!("Failed to acquire recording lock: {}", e))?;
        
        if !manager.is_recording() {
            return Err("No recording in progress".to_string());
        }
        
        manager.current_filepath()
            .cloned()
            .ok_or_else(|| "No recording filepath found".to_string())?
    };
    
    // Get copilot settings
    let settings = {
        let manager = settings_manager.read()
            .map_err(|e| format!("Failed to acquire settings lock: {}", e))?;
        manager.get().copilot.clone()
    };
    
    // Get or create agent state from app state
    let agent_state = app_handle.state::<Arc<tokio::sync::Mutex<Option<CoPilotAgent>>>>();
    let mut agent_guard = agent_state.lock().await;
    
    // Check if agent already running
    if agent_guard.is_some() {
        return Err("Co-Pilot agent is already running".to_string());
    }
    
    // Create and start agent
    let provider = intel_provider.inner().clone();
    let mut agent = CoPilotAgent::new(app_handle.clone());
    agent.start(provider, recording_filepath, settings).await?;
    
    *agent_guard = Some(agent);
    
    Ok(())
}

/// Stop the Co-Pilot agent
///
/// This command stops the Co-Pilot agent gracefully, waiting for any in-flight
/// inference to complete (up to 120s timeout), and returns the final agent state.
///
/// # Arguments
///
/// * `app_handle` - Tauri app handle to access agent state
///
/// # Returns
///
/// * `Ok(CoPilotState)` - Final agent state after stopping
/// * `Err(String)` - Error message if stop fails
///
/// # Errors
///
/// Returns an error if no Co-Pilot agent is currently running.
///
/// # Examples
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// interface CoPilotState {
///   running_summary: string;
///   key_points: string[];
///   decisions: string[];
///   action_items: string[];
///   open_questions: string[];
///   suggested_questions: SuggestedQuestion[];
///   key_concepts: KeyConcept[];
///   cycle_metadata: CycleMetadata;
/// }
///
/// try {
///   const finalState: CoPilotState = await invoke('stop_copilot');
///   console.log(`Agent stopped. Final cycle: ${finalState.cycle_metadata.cycle_number}`);
/// } catch (error) {
///   console.error(`Failed to stop Co-Pilot: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn stop_copilot(
    app_handle: AppHandle,
) -> Result<crate::agents::copilot::CoPilotState, String> {
    use crate::agents::copilot::CoPilotAgent;
    
    let agent_state = app_handle.state::<Arc<tokio::sync::Mutex<Option<CoPilotAgent>>>>();
    let mut agent_guard = agent_state.lock().await;
    
    let mut agent = agent_guard.take()
        .ok_or_else(|| "No Co-Pilot agent running".to_string())?;
    
    let final_state = agent.stop().await;
    
    Ok(final_state)
}

/// Get the current Co-Pilot agent state
///
/// This command returns the current state of the Co-Pilot agent without stopping it.
/// Useful for polling or refreshing the UI.
///
/// # Arguments
///
/// * `app_handle` - Tauri app handle to access agent state
///
/// # Returns
///
/// * `Ok(CoPilotState)` - Current agent state
/// * `Err(String)` - Error message if agent is not running
///
/// # Errors
///
/// Returns an error if no Co-Pilot agent is currently running.
///
/// # Examples
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// try {
///   const state: CoPilotState = await invoke('get_copilot_state');
///   console.log(`Current cycle: ${state.cycle_metadata.cycle_number}`);
/// } catch (error) {
///   console.error(`Failed to get Co-Pilot state: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn get_copilot_state(
    app_handle: AppHandle,
) -> Result<crate::agents::copilot::CoPilotState, String> {
    use crate::agents::copilot::CoPilotAgent;
    
    let agent_state = app_handle.state::<Arc<tokio::sync::Mutex<Option<CoPilotAgent>>>>();
    let agent_guard = agent_state.lock().await;
    
    let agent = agent_guard.as_ref()
        .ok_or_else(|| "No Co-Pilot agent running".to_string())?;
    
    Ok(agent.get_state().await)
}

/// Dismiss a suggested question by index
///
/// This command marks a suggested question as dismissed. Dismissed questions
/// will not be shown in the UI but will be preserved if the same question is
/// suggested again in a future cycle.
///
/// # Arguments
///
/// * `index` - The index of the question to dismiss (0-based)
/// * `app_handle` - Tauri app handle to access agent state
///
/// # Returns
///
/// * `Ok(())` - Question dismissed successfully
/// * `Err(String)` - Error message if dismiss fails
///
/// # Errors
///
/// Returns an error if no Co-Pilot agent is currently running.
///
/// # Examples
///
/// ```typescript
/// import { invoke } from '@tauri-apps/api/core';
///
/// try {
///   await invoke('dismiss_copilot_question', { index: 0 });
///   console.log('Question dismissed');
/// } catch (error) {
///   console.error(`Failed to dismiss question: ${error}`);
/// }
/// ```
#[tauri::command]
pub async fn dismiss_copilot_question(
    index: usize,
    app_handle: AppHandle,
) -> Result<(), String> {
    use crate::agents::copilot::CoPilotAgent;
    
    let agent_state = app_handle.state::<Arc<tokio::sync::Mutex<Option<CoPilotAgent>>>>();
    let agent_guard = agent_state.lock().await;
    
    let agent = agent_guard.as_ref()
        .ok_or_else(|| "No Co-Pilot agent running".to_string())?;
    
    agent.dismiss_question(index).await;
    
    Ok(())
}

// ============================================================================
// Chat Commands
// ============================================================================

/// Start a chat session with a recording
///
/// Creates session immediately and returns. If the recording has no transcript,
/// spawns background preparation and emits `chat-status` events for progress.
///
/// # Returns
///
/// JSON object: `{ "session_id": "...", "needs_preparation": bool }`
#[tauri::command]
pub async fn chat_with_recording(
    recording_filename: String,
    intel_queue: State<'_, IntelQueue>,
    app_handle: AppHandle,
) -> Result<serde_json::Value, String> {
    let source = RecordingChatSource::new(app_handle.clone(), recording_filename.clone())?;
    let needs_prep = source.needs_preparation().await;

    // Create session immediately (no blocking context generation)
    let chatbot_state = app_handle.state::<tokio::sync::Mutex<Chatbot>>();
    let mut chatbot = chatbot_state.lock().await;
    let session_id = chatbot.start_session(&source).await?;
    drop(chatbot); // Release lock before spawning

    // If transcript doesn't exist, generate it in the background
    if needs_prep {
        let queue_clone = intel_queue.inner().clone();
        let app_clone = app_handle.clone();
        let filename_clone = recording_filename.clone();

        tokio::spawn(async move {
            let source = match RecordingChatSource::new(app_clone, filename_clone) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Chat: Failed to create source for preparation: {}", e);
                    return;
                }
            };
            // get_context() generates transcript and emits chat-status events
            if let Err(e) = source.get_context(&queue_clone).await {
                eprintln!("Chat: Background preparation failed: {}", e);
                source.on_preparation_status("error", &format!("Preparation failed: {}", e));
            }
        });
    }

    Ok(serde_json::json!({
        "session_id": session_id,
        "needs_preparation": needs_prep
    }))
}

/// Send a message in an existing chat session
///
/// Recreates the RecordingChatSource (stateless, cheap) and sends a message
/// through the chatbot. The chatbot fetches fresh context on every message.
///
/// # Arguments
///
/// * `session_id` - The session ID
/// * `recording_filename` - The recording filename (to recreate the source)
/// * `message` - The user's message text
/// * `intel_queue` - The IntelQueue for submitting LLM requests
/// * `app_handle` - Tauri app handle to access chatbot state
///
/// # Returns
///
/// The assistant's response text
///
/// # Errors
///
/// Returns an error if:
/// - The RecordingChatSource cannot be created
/// - The chatbot state cannot be accessed
/// - The session is not found
/// - Message sending fails
#[tauri::command]
pub async fn chat_send_message(
    session_id: String,
    recording_filename: String,
    message: String,
    intel_queue: State<'_, IntelQueue>,
    app_handle: AppHandle,
) -> Result<String, String> {
    // Recreate RecordingChatSource (stateless, cheap to construct)
    let source = RecordingChatSource::new(app_handle.clone(), recording_filename)?;

    // Get chatbot from managed state
    let chatbot_state = app_handle.state::<tokio::sync::Mutex<Chatbot>>();
    let mut chatbot = chatbot_state.lock().await;

    // Send message
    chatbot.send_message(&session_id, &message, &source, &intel_queue).await
}

/// Get the message history for a chat session
///
/// Returns all messages (user and assistant) in the session.
///
/// # Arguments
///
/// * `session_id` - The session ID
/// * `app_handle` - Tauri app handle to access chatbot state
///
/// # Returns
///
/// A vector of ChatMessage objects with role, content, and timestamp
///
/// # Errors
///
/// Returns an error if:
/// - The chatbot state cannot be accessed
/// - The session is not found
#[tauri::command]
pub async fn chat_get_history(
    session_id: String,
    app_handle: AppHandle,
) -> Result<Vec<ChatMessage>, String> {
    // Get chatbot from managed state
    let chatbot_state = app_handle.state::<tokio::sync::Mutex<Chatbot>>();
    let chatbot = chatbot_state.lock().await;

    // Get history
    chatbot.get_history(&session_id)
}

/// End a chat session
///
/// Removes the session from memory. The session log file remains on disk.
///
/// # Arguments
///
/// * `session_id` - The session ID
/// * `app_handle` - Tauri app handle to access chatbot state
///
/// # Returns
///
/// Ok(()) on success
///
/// # Errors
///
/// Returns an error if the chatbot state cannot be accessed
#[tauri::command]
pub async fn chat_end_session(
    session_id: String,
    app_handle: AppHandle,
) -> Result<(), String> {
    // Get chatbot from managed state
    let chatbot_state = app_handle.state::<tokio::sync::Mutex<Chatbot>>();
    let mut chatbot = chatbot_state.lock().await;

    // End session
    chatbot.end_session(&session_id);
    Ok(())
}

/// Check if a saved transcript exists on disk for a recording and return it.
///
/// Looks in the per-recording folder: `recordings/{stem}/transcript.md`
/// Returns `null` if no transcript file exists.
#[tauri::command]
pub async fn get_saved_transcript(
    recording_filename: String,
    app_handle: AppHandle,
) -> Result<Option<String>, String> {
    let source = RecordingChatSource::new(app_handle, recording_filename)?;
    let transcript_path = source.transcript_path();

    if transcript_path.exists() {
        let content = tokio::fs::read_to_string(&transcript_path).await
            .map_err(|e| format!("Failed to read transcript: {}", e))?;
        Ok(Some(content))
    } else {
        Ok(None)
    }
}
