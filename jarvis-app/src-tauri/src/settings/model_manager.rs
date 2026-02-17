use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex as TokioMutex;
use tokio_util::sync::CancellationToken;

/// Static metadata for a model in the catalog
struct ModelEntry {
    filename: &'static str,
    display_name: &'static str,
    description: &'static str,
    size_estimate: &'static str,
    quality_tier: &'static str,
    download_url: &'static str,
}

/// Information about a Whisper model (returned to frontend)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub filename: String,
    pub display_name: String,
    pub description: String,
    pub size_estimate: String,
    pub quality_tier: String,
    pub status: ModelStatus,
}

/// Status of a Whisper model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ModelStatus {
    Downloaded { size_bytes: u64 },
    Downloading { progress: f32 },
    Error { message: String },
    #[serde(rename = "not_downloaded")]
    NotDownloaded,
}

/// Internal state for an in-progress download
struct DownloadState {
    progress: f32,
    cancel_token: CancellationToken,
}

/// Manages Whisper model discovery, download, and status tracking
pub struct ModelManager {
    models_dir: PathBuf,
    app_handle: AppHandle,
    download_queue: std::sync::Arc<TokioMutex<HashMap<String, DownloadState>>>,
    error_states: std::sync::Arc<TokioMutex<HashMap<String, String>>>,
}

impl ModelManager {
    /// Model catalog with metadata for all supported Whisper models.
    /// Models are ordered by quality tier (basic → best) for UI display.
    const MODEL_CATALOG: &'static [ModelEntry] = &[
        ModelEntry {
            filename: "ggml-tiny.en.bin",
            display_name: "Tiny (English)",
            description: "Fastest, lowest accuracy. Good for testing.",
            size_estimate: "75 MB",
            quality_tier: "basic",
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
        },
        ModelEntry {
            filename: "ggml-base.en.bin",
            display_name: "Base (English)",
            description: "Fast with reasonable accuracy. ~10% WER.",
            size_estimate: "142 MB",
            quality_tier: "basic",
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
        },
        ModelEntry {
            filename: "ggml-small.en.bin",
            display_name: "Small (English)",
            description: "Balanced speed and accuracy.",
            size_estimate: "466 MB",
            quality_tier: "good",
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",
        },
        ModelEntry {
            filename: "ggml-medium.en.bin",
            display_name: "Medium (English)",
            description: "Good accuracy, moderate speed. ~7.5% WER.",
            size_estimate: "1.5 GB",
            quality_tier: "good",
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin",
        },
        ModelEntry {
            filename: "ggml-medium.en-q5_0.bin",
            display_name: "Medium Q5 (English)",
            description: "Quantized medium — 3x smaller, similar accuracy.",
            size_estimate: "514 MB",
            quality_tier: "good",
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en-q5_0.bin",
        },
        ModelEntry {
            filename: "ggml-large-v3-turbo-q5_0.bin",
            display_name: "Large V3 Turbo Q5",
            description: "Best value: near-large accuracy, fast inference. ~7.75% WER. Recommended.",
            size_estimate: "547 MB",
            quality_tier: "great",
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin",
        },
        ModelEntry {
            filename: "ggml-large-v3-turbo-q8_0.bin",
            display_name: "Large V3 Turbo Q8",
            description: "Higher precision quantized turbo. Slightly better than Q5.",
            size_estimate: "834 MB",
            quality_tier: "great",
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q8_0.bin",
        },
        ModelEntry {
            filename: "ggml-large-v3-turbo.bin",
            display_name: "Large V3 Turbo",
            description: "Full precision turbo. 6x faster than Large V3.",
            size_estimate: "1.5 GB",
            quality_tier: "great",
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin",
        },
        ModelEntry {
            filename: "ggml-distil-large-v3.bin",
            display_name: "Distil Large V3",
            description: "Distilled from Large V3. 5x faster, within 0.8% WER.",
            size_estimate: "1.5 GB",
            quality_tier: "great",
            download_url: "https://huggingface.co/distil-whisper/distil-large-v3-ggml/resolve/main/ggml-distil-large-v3.bin",
        },
        ModelEntry {
            filename: "ggml-large-v3-q5_0.bin",
            display_name: "Large V3 Q5",
            description: "Quantized Large V3. Best accuracy in compact form.",
            size_estimate: "1.1 GB",
            quality_tier: "best",
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin",
        },
        ModelEntry {
            filename: "ggml-large-v3.bin",
            display_name: "Large V3",
            description: "Highest accuracy, slowest. ~7.4% WER. Needs 16GB+ RAM.",
            size_estimate: "2.9 GB",
            quality_tier: "best",
            download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
        },
    ];

    /// GGML magic number (0x67676d6c = "ggml" in ASCII)
    /// All whisper.cpp models use GGML format (not GGUF which is used by llama.cpp).
    const GGML_MAGIC: u32 = 0x67676d6c;

    /// Minimum valid GGML file size (1MB)
    const MIN_GGML_SIZE: u64 = 1_048_576;
    
    /// Creates a new ModelManager
    /// 
    /// Creates the models directory if it doesn't exist.
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - The home directory cannot be determined
    /// - The models directory cannot be created
    pub fn new(app_handle: AppHandle) -> Result<Self, String> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| "Failed to get home directory".to_string())?;
        
        let models_dir = home_dir.join(".jarvis").join("models");
        
        // Create models directory if it doesn't exist
        if !models_dir.exists() {
            std::fs::create_dir_all(&models_dir)
                .map_err(|e| format!("Failed to create models directory: {}", e))?;
        }
        
        Ok(Self {
            models_dir,
            app_handle,
            download_queue: std::sync::Arc::new(TokioMutex::new(HashMap::new())),
            error_states: std::sync::Arc::new(TokioMutex::new(HashMap::new())),
        })
    }
    
    /// Returns the list of supported model filenames
    pub fn supported_models() -> Vec<&'static str> {
        Self::MODEL_CATALOG.iter().map(|e| e.filename).collect()
    }

    /// Look up a catalog entry by filename
    fn catalog_entry(filename: &str) -> Option<&'static ModelEntry> {
        Self::MODEL_CATALOG.iter().find(|e| e.filename == filename)
    }

    /// Lists all supported models with their status and metadata
    ///
    /// Returns a ModelInfo for each catalog entry with status:
    /// - Downloaded: Model file exists on disk (includes file size)
    /// - Downloading: Model is currently being downloaded (includes progress)
    /// - Error: Previous download failed (includes error message)
    /// - NotDownloaded: Model is not available locally
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, String> {
        let mut models = Vec::new();

        let download_queue = self.download_queue.lock().await;
        let error_states = self.error_states.lock().await;

        for entry in Self::MODEL_CATALOG {
            let model_path = self.models_dir.join(entry.filename);

            let status = if let Some(download_state) = download_queue.get(entry.filename) {
                ModelStatus::Downloading {
                    progress: download_state.progress,
                }
            } else if let Some(error_msg) = error_states.get(entry.filename) {
                ModelStatus::Error {
                    message: error_msg.clone(),
                }
            } else if model_path.exists() {
                match std::fs::metadata(&model_path) {
                    Ok(metadata) => ModelStatus::Downloaded {
                        size_bytes: metadata.len(),
                    },
                    Err(e) => ModelStatus::Error {
                        message: format!("Failed to read file metadata: {}", e),
                    },
                }
            } else {
                ModelStatus::NotDownloaded
            };

            models.push(ModelInfo {
                filename: entry.filename.to_string(),
                display_name: entry.display_name.to_string(),
                description: entry.description.to_string(),
                size_estimate: entry.size_estimate.to_string(),
                quality_tier: entry.quality_tier.to_string(),
                status,
            });
        }

        Ok(models)
    }
    
    /// Validates that a file is a valid GGML model
    /// 
    /// Checks:
    /// - File size is at least 1MB
    /// - First 4 bytes match GGML magic number (0x67676d6c)
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - File cannot be read
    /// - File is too small
    /// - Magic number doesn't match
    fn validate_ggml_file(path: &PathBuf) -> Result<(), String> {
        // Check file size
        let metadata = std::fs::metadata(path)
            .map_err(|e| format!("Failed to read file metadata: {}", e))?;
        
        if metadata.len() < Self::MIN_GGML_SIZE {
            return Err(format!(
                "File too small: {} bytes (minimum {} bytes)",
                metadata.len(),
                Self::MIN_GGML_SIZE
            ));
        }
        
        // Read and verify magic number
        let mut file = std::fs::File::open(path)
            .map_err(|e| format!("Failed to open file: {}", e))?;
        
        let mut magic_bytes = [0u8; 4];
        std::io::Read::read_exact(&mut file, &mut magic_bytes)
            .map_err(|e| format!("Failed to read magic number: {}", e))?;
        
        let magic = u32::from_le_bytes(magic_bytes);
        if magic != Self::GGML_MAGIC {
            return Err(format!(
                "Invalid GGML magic number: 0x{:08x} (expected 0x{:08x})",
                magic,
                Self::GGML_MAGIC
            ));
        }
        
        Ok(())
    }
    
    /// Returns the download URL for a model from the catalog
    fn download_url(model_name: &str) -> Result<String, String> {
        Self::catalog_entry(model_name)
            .map(|e| e.download_url.to_string())
            .ok_or_else(|| format!("Model '{}' not found in catalog", model_name))
    }
    
    /// Downloads a model from Hugging Face
    /// 
    /// This method spawns an async task and returns immediately.
    /// Progress is reported via "model-download-progress" events.
    /// Completion is reported via "model-download-complete" event.
    /// Errors are reported via "model-download-error" event.
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - Model name is not in supported list
    /// - Model is already being downloaded
    pub async fn download_model(&self, model_name: String) -> Result<(), String> {
        // Validate model name against catalog
        if Self::catalog_entry(&model_name).is_none() {
            return Err(format!("Unsupported model: {}", model_name));
        }
        
        // Check if already downloading
        let mut download_queue = self.download_queue.lock().await;
        if download_queue.contains_key(&model_name) {
            return Err(format!("Model {} is already being downloaded", model_name));
        }
        
        // Add to download queue with initial progress
        let cancel_token = CancellationToken::new();
        download_queue.insert(
            model_name.clone(),
            DownloadState {
                progress: 0.0,
                cancel_token: cancel_token.clone(),
            },
        );
        drop(download_queue); // Release lock before spawning
        
        // Clone necessary data for the spawned task
        let models_dir = self.models_dir.clone();
        let app_handle = self.app_handle.clone();
        let download_queue_clone = self.download_queue.clone();
        let error_states_clone = self.error_states.clone();
        
        // Spawn async task for download (returns immediately)
        tokio::spawn(async move {
            let result = Self::download_task(
                model_name.clone(),
                models_dir,
                app_handle.clone(),
                download_queue_clone.clone(),
                cancel_token,
            )
            .await;
            
            // Remove from download queue
            download_queue_clone.lock().await.remove(&model_name);
            
            match result {
                Ok(()) => {
                    // Clear any previous error state
                    error_states_clone.lock().await.remove(&model_name);
                    
                    // Emit completion event
                    let _ = app_handle.emit(
                        "model-download-complete",
                        serde_json::json!({ "model_name": model_name }),
                    );
                }
                Err(e) => {
                    // Store error state
                    error_states_clone.lock().await.insert(model_name.clone(), e.clone());
                    
                    // Emit error event
                    let _ = app_handle.emit(
                        "model-download-error",
                        serde_json::json!({ "model_name": model_name, "error": e }),
                    );
                }
            }
        });
        
        Ok(())
    }
    
    /// Internal download task (runs in spawned tokio task)
    async fn download_task(
        model_name: String,
        models_dir: PathBuf,
        app_handle: AppHandle,
        download_queue: std::sync::Arc<TokioMutex<HashMap<String, DownloadState>>>,
        cancel_token: CancellationToken,
    ) -> Result<(), String> {
        use tokio::io::AsyncWriteExt;
        
        let url = Self::download_url(&model_name)?;
        let temp_path = models_dir.join(format!("{}.tmp", model_name));
        let final_path = models_dir.join(&model_name);
        
        // Create HTTP client
        let client = reqwest::Client::new();
        
        // Start download
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to start download: {}", e))?;
        
        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }
        
        let total_size = response.content_length().unwrap_or(0);
        
        // Create temp file
        let mut file = tokio::fs::File::create(&temp_path)
            .await
            .map_err(|e| format!("Failed to create temp file: {}", e))?;
        
        // Stream download with progress reporting
        let mut downloaded: u64 = 0;
        let mut last_progress: f32 = 0.0;
        let mut stream = response.bytes_stream();
        
        use futures_util::StreamExt;
        
        while let Some(chunk_result) = stream.next().await {
            // Check for cancellation
            if cancel_token.is_cancelled() {
                let _ = tokio::fs::remove_file(&temp_path).await;
                return Err("Download cancelled".to_string());
            }
            
            let chunk = chunk_result
                .map_err(|e| format!("Failed to read chunk: {}", e))?;
            
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Failed to write chunk: {}", e))?;
            
            downloaded += chunk.len() as u64;
            
            // Calculate progress
            let progress = if total_size > 0 {
                (downloaded as f32 / total_size as f32) * 100.0
            } else {
                0.0
            };
            
            // Emit progress event every 1%
            if progress - last_progress >= 1.0 {
                last_progress = progress;
                
                // Update download queue
                if let Some(state) = download_queue.lock().await.get_mut(&model_name) {
                    state.progress = progress;
                }
                
                // Emit progress event
                let _ = app_handle.emit(
                    "model-download-progress",
                    serde_json::json!({
                        "model_name": model_name,
                        "progress": progress,
                        "downloaded": downloaded,
                        "total": total_size,
                    }),
                );
            }
        }
        
        // Flush file
        file.flush()
            .await
            .map_err(|e| format!("Failed to flush file: {}", e))?;
        drop(file);
        
        // Validate GGML format
        Self::validate_ggml_file(&temp_path).map_err(|e| {
            // Clean up invalid file
            let _ = std::fs::remove_file(&temp_path);
            format!("Invalid GGML file: {}", e)
        })?;
        
        // Atomic rename to final location
        tokio::fs::rename(&temp_path, &final_path)
            .await
            .map_err(|e| {
                // Clean up temp file on rename failure
                let _ = std::fs::remove_file(&temp_path);
                format!("Failed to move file to final location: {}", e)
            })?;
        
        Ok(())
    }
    
    /// Cancels an in-progress download
    /// 
    /// # Errors
    /// 
    /// Returns an error if the model is not currently being downloaded
    pub async fn cancel_download(&self, model_name: String) -> Result<(), String> {
        let mut download_queue = self.download_queue.lock().await;
        
        let state = download_queue.remove(&model_name)
            .ok_or_else(|| format!("Model {} is not being downloaded", model_name))?;
        
        // Release lock before I/O
        drop(download_queue);
        
        // Trigger cancellation
        state.cancel_token.cancel();
        
        // Clean up temp file (best effort)
        let temp_path = self.models_dir.join(format!("{}.tmp", model_name));
        let _ = tokio::fs::remove_file(&temp_path).await;
        
        Ok(())
    }
    
    /// Deletes a downloaded model
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - Model file doesn't exist
    /// - File deletion fails
    pub async fn delete_model(&self, model_name: String) -> Result<(), String> {
        let model_path = self.models_dir.join(&model_name);
        
        if !model_path.exists() {
            return Err(format!("Model {} is not downloaded", model_name));
        }
        
        tokio::fs::remove_file(&model_path)
            .await
            .map_err(|e| format!("Failed to delete model: {}", e))?;
        
        // Clear any error state for this model
        self.error_states.lock().await.remove(&model_name);
        
        Ok(())
    }
}
