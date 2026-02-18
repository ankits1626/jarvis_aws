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

/// Static metadata for a WhisperKit model in the catalog
pub(crate) struct WhisperKitModelEntry {
    pub(crate) name: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) size_estimate: &'static str,
    pub(crate) quality_tier: &'static str,
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
    whisperkit_models_dir: PathBuf,
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

    /// WhisperKit model catalog with metadata for all supported models.
    /// Models are ordered by quality tier (basic → best) for UI display.
    pub(crate) const WHISPERKIT_MODEL_CATALOG: &'static [WhisperKitModelEntry] = &[
        WhisperKitModelEntry {
            name: "openai_whisper-base.en",
            display_name: "Base (English)",
            description: "Fast with reasonable accuracy. Good for testing.",
            size_estimate: "~150 MB",
            quality_tier: "basic",
        },
        WhisperKitModelEntry {
            name: "openai_whisper-large-v3_turbo",
            display_name: "Large V3 Turbo",
            description: "Best value: fast inference with great accuracy. Recommended.",
            size_estimate: "~800 MB",
            quality_tier: "great",
        },
        WhisperKitModelEntry {
            name: "openai_whisper-large-v3_turbo_632MB",
            display_name: "Large V3 Turbo (Compressed)",
            description: "Compressed turbo model. Slightly smaller, similar performance.",
            size_estimate: "~632 MB",
            quality_tier: "great",
        },
        WhisperKitModelEntry {
            name: "openai_whisper-large-v3",
            display_name: "Large V3",
            description: "Highest accuracy. Slower inference.",
            size_estimate: "~1.5 GB",
            quality_tier: "best",
        },
        WhisperKitModelEntry {
            name: "openai_whisper-large-v3_947MB",
            display_name: "Large V3 (Compressed)",
            description: "Compressed Large V3. Smaller size, similar accuracy.",
            size_estimate: "~947 MB",
            quality_tier: "best",
        },
    ];

    /// Subpath that whisperkit-cli creates inside --download-model-path
    /// The CLI always nests models under models/argmaxinc/whisperkit-coreml/
    const WHISPERKIT_HF_SUBPATH: &'static str = "models/argmaxinc/whisperkit-coreml";

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
        let whisperkit_models_dir = models_dir.join("whisperkit");
        
        // Create models directory if it doesn't exist
        if !models_dir.exists() {
            std::fs::create_dir_all(&models_dir)
                .map_err(|e| format!("Failed to create models directory: {}", e))?;
        }
        
        // Create whisperkit models directory if it doesn't exist
        if !whisperkit_models_dir.exists() {
            std::fs::create_dir_all(&whisperkit_models_dir)
                .map_err(|e| format!("Failed to create whisperkit models directory: {}", e))?;
        }
        
        Ok(Self {
            models_dir,
            whisperkit_models_dir,
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
    
    /// Lists all supported WhisperKit models with their status and metadata
    ///
    /// Returns a ModelInfo for each catalog entry with status:
    /// - Downloaded: Model directory exists on disk
    /// - NotDownloaded: Model is not available locally
    ///
    /// Note: WhisperKit models are managed by whisperkit-cli and stored as
    /// directories containing .mlmodelc files, not single binary files.
    pub async fn list_whisperkit_models(&self) -> Result<Vec<ModelInfo>, String> {
        let mut models = Vec::new();

        for entry in Self::WHISPERKIT_MODEL_CATALOG {
            let model_dir = self.whisperkit_model_dir(entry.name);

            let status = if self.whisperkit_model_exists(entry.name) {
                // Try to calculate directory size
                match Self::calculate_dir_size(&model_dir) {
                    Ok(size) => ModelStatus::Downloaded { size_bytes: size },
                    Err(_) => ModelStatus::Downloaded { size_bytes: 0 },
                }
            } else {
                ModelStatus::NotDownloaded
            };

            models.push(ModelInfo {
                filename: entry.name.to_string(),
                display_name: entry.display_name.to_string(),
                description: entry.description.to_string(),
                size_estimate: entry.size_estimate.to_string(),
                quality_tier: entry.quality_tier.to_string(),
                status,
            });
        }

        Ok(models)
    }
    
    /// Returns the resolved path for a WhisperKit model directory.
    ///
    /// whisperkit-cli stores models under a nested HuggingFace cache structure:
    /// `<whisperkit_dir>/models/argmaxinc/whisperkit-coreml/<model_name>/`
    pub fn whisperkit_model_dir(&self, name: &str) -> PathBuf {
        self.whisperkit_models_dir
            .join(Self::WHISPERKIT_HF_SUBPATH)
            .join(name)
    }

    /// Checks if a WhisperKit model exists on disk
    ///
    /// A model is considered to exist if:
    /// - The model directory exists in the HuggingFace cache structure
    /// - The directory contains at least one .mlmodelc file
    ///
    /// # Arguments
    ///
    /// * `name` - The model name (e.g., "openai_whisper-large-v3_turbo")
    pub fn whisperkit_model_exists(&self, name: &str) -> bool {
        let model_dir = self.whisperkit_model_dir(name);

        if !model_dir.exists() || !model_dir.is_dir() {
            return false;
        }

        // Check if directory contains .mlmodelc files
        if let Ok(entries) = std::fs::read_dir(&model_dir) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "mlmodelc" {
                        return true;
                    }
                }
            }
        }

        false
    }
    
    /// Calculates the total size of a directory recursively
    fn calculate_dir_size(path: &PathBuf) -> Result<u64, std::io::Error> {
        let mut total_size = 0u64;
        
        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let metadata = entry.metadata()?;
                
                if metadata.is_dir() {
                    total_size += Self::calculate_dir_size(&entry.path())?;
                } else {
                    total_size += metadata.len();
                }
            }
        }
        
        Ok(total_size)
    }
    
    /// Downloads a WhisperKit model using whisperkit-cli
    /// 
    /// This method spawns an async task and returns immediately.
    /// Progress is reported via "model-download-progress" events.
    /// Completion is reported via "model-download-complete" event.
    /// Errors are reported via "model-download-error" event.
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - Model name is not in WHISPERKIT_MODEL_CATALOG
    /// - whisperkit-cli is not found
    pub async fn download_whisperkit_model(&self, model_name: String) -> Result<(), String> {
        // Validate model name against catalog and get size estimate
        let catalog_entry = Self::WHISPERKIT_MODEL_CATALOG
            .iter()
            .find(|e| e.name == model_name)
            .ok_or_else(|| format!("Unsupported WhisperKit model: {}", model_name))?;

        let estimated_bytes = Self::parse_size_estimate(catalog_entry.size_estimate);

        // Find whisperkit-cli
        let cli_path = Self::find_whisperkit_cli()
            .ok_or_else(|| "whisperkit-cli not found. Install with: brew install whisperkit-cli".to_string())?;

        // Clone necessary data for the spawned task
        let output_dir = self.whisperkit_models_dir.clone();
        let app_handle = self.app_handle.clone();

        // Spawn async task for download (returns immediately)
        tokio::spawn(async move {
            let result = Self::download_whisperkit_task(
                model_name.clone(),
                cli_path,
                output_dir,
                estimated_bytes,
                app_handle.clone(),
            )
            .await;
            
            match result {
                Ok(()) => {
                    // Emit completion event
                    let _ = app_handle.emit(
                        "model-download-complete",
                        serde_json::json!({ "model_name": model_name }),
                    );
                }
                Err(e) => {
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
    
    /// Finds whisperkit-cli binary
    fn find_whisperkit_cli() -> Option<PathBuf> {
        // Check common Homebrew locations
        let homebrew_paths = [
            PathBuf::from("/opt/homebrew/bin/whisperkit-cli"),
            PathBuf::from("/usr/local/bin/whisperkit-cli"),
        ];
        
        for path in &homebrew_paths {
            if path.exists() {
                return Some(path.clone());
            }
        }
        
        // Try PATH via which command
        if let Ok(output) = std::process::Command::new("which")
            .arg("whisperkit-cli")
            .output()
        {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    return Some(PathBuf::from(path_str));
                }
            }
        }
        
        None
    }
    
    /// Strips the `openai_` or `distil-whisper_` prefix from a model name
    /// to get the short name that whisperkit-cli --model expects.
    ///
    /// whisperkit-cli has a --model-prefix flag (default: "openai") that it
    /// prepends automatically, so we must pass just the suffix.
    /// For example: "openai_whisper-large-v3_turbo" → "whisper-large-v3_turbo"
    fn whisperkit_model_short_name(full_name: &str) -> String {
        if let Some(suffix) = full_name.strip_prefix("openai_") {
            suffix.to_string()
        } else if let Some(suffix) = full_name.strip_prefix("distil-whisper_") {
            // distil-whisper models need --model-prefix "distil-whisper"
            suffix.to_string()
        } else {
            full_name.to_string()
        }
    }

    /// Returns the --model-prefix arg for whisperkit-cli based on model name.
    fn whisperkit_model_prefix(full_name: &str) -> &'static str {
        if full_name.starts_with("distil-whisper_") {
            "distil-whisper"
        } else {
            "openai"
        }
    }

    /// Parses a size estimate string like "~800 MB" or "~1.5 GB" to bytes.
    fn parse_size_estimate(estimate: &str) -> u64 {
        let cleaned = estimate.trim().trim_start_matches('~').trim();
        let parts: Vec<&str> = cleaned.split_whitespace().collect();
        if parts.len() != 2 {
            return 0;
        }
        let number: f64 = parts[0].parse().unwrap_or(0.0);
        match parts[1].to_uppercase().as_str() {
            "MB" => (number * 1024.0 * 1024.0) as u64,
            "GB" => (number * 1024.0 * 1024.0 * 1024.0) as u64,
            _ => 0,
        }
    }

    /// Creates a minimal 1-second silent WAV file for triggering model download.
    /// whisperkit-cli has no standalone download command, so we use `transcribe`
    /// with a silent audio file to trigger the model download side effect.
    fn create_silent_wav(path: &PathBuf) -> Result<(), String> {
        use std::io::Write;

        let sample_rate: u32 = 16000;
        let num_samples: u32 = sample_rate; // 1 second
        let bits_per_sample: u16 = 16;
        let num_channels: u16 = 1;
        let byte_rate = sample_rate * (bits_per_sample as u32 / 8) * num_channels as u32;
        let block_align = num_channels * (bits_per_sample / 8);
        let data_size = num_samples * (bits_per_sample as u32 / 8) * num_channels as u32;

        let mut file = std::fs::File::create(path)
            .map_err(|e| format!("Failed to create silent WAV: {}", e))?;

        // WAV header
        file.write_all(b"RIFF").map_err(|e| e.to_string())?;
        file.write_all(&(36 + data_size).to_le_bytes()).map_err(|e| e.to_string())?;
        file.write_all(b"WAVE").map_err(|e| e.to_string())?;
        file.write_all(b"fmt ").map_err(|e| e.to_string())?;
        file.write_all(&16u32.to_le_bytes()).map_err(|e| e.to_string())?; // chunk size
        file.write_all(&1u16.to_le_bytes()).map_err(|e| e.to_string())?;  // PCM
        file.write_all(&num_channels.to_le_bytes()).map_err(|e| e.to_string())?;
        file.write_all(&sample_rate.to_le_bytes()).map_err(|e| e.to_string())?;
        file.write_all(&byte_rate.to_le_bytes()).map_err(|e| e.to_string())?;
        file.write_all(&block_align.to_le_bytes()).map_err(|e| e.to_string())?;
        file.write_all(&bits_per_sample.to_le_bytes()).map_err(|e| e.to_string())?;
        file.write_all(b"data").map_err(|e| e.to_string())?;
        file.write_all(&data_size.to_le_bytes()).map_err(|e| e.to_string())?;

        // Silent PCM data (all zeros)
        let silence = vec![0u8; data_size as usize];
        file.write_all(&silence).map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Internal download task for WhisperKit models (runs in spawned tokio task)
    ///
    /// Uses `whisperkit-cli transcribe` with a silent audio file to trigger model download.
    /// The CLI has no standalone download command — model download is a side effect of
    /// transcribe/serve when the model isn't cached locally.
    ///
    /// Progress is estimated by polling the download directory size.
    async fn download_whisperkit_task(
        model_name: String,
        cli_path: PathBuf,
        output_dir: PathBuf,
        estimated_bytes: u64,
        app_handle: AppHandle,
    ) -> Result<(), String> {
        use tokio::process::Command;

        // Create a silent WAV file for triggering the download
        let silent_wav = output_dir.join(".silent_trigger.wav");
        Self::create_silent_wav(&silent_wav)?;

        // Derive short model name and prefix for the CLI
        let short_name = Self::whisperkit_model_short_name(&model_name);
        let prefix = Self::whisperkit_model_prefix(&model_name);

        eprintln!(
            "whisperkit download: model={}, short_name={}, prefix={}, output={}",
            model_name, short_name, prefix, output_dir.display()
        );

        // Spawn whisperkit-cli transcribe to trigger model download
        let child = Command::new(&cli_path)
            .arg("transcribe")
            .arg("--model")
            .arg(&short_name)
            .arg("--model-prefix")
            .arg(prefix)
            .arg("--download-model-path")
            .arg(&output_dir)
            .arg("--audio-path")
            .arg(&silent_wav)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn whisperkit-cli: {}", e))?;

        // The model directory where files will appear
        let model_dir = output_dir
            .join(Self::WHISPERKIT_HF_SUBPATH)
            .join(&model_name);

        // Poll directory size for progress while the process runs
        let poll_model_dir = model_dir.clone();
        let poll_model_name = model_name.clone();
        let poll_app_handle = app_handle.clone();

        let poll_handle = tokio::spawn(async move {
            let mut last_size: u64 = 0;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                let current_size = Self::calculate_dir_size(&poll_model_dir).unwrap_or(0);
                if current_size > last_size {
                    last_size = current_size;
                    // Calculate percentage from estimated total size
                    let progress = if estimated_bytes > 0 {
                        ((current_size as f64 / estimated_bytes as f64) * 100.0).min(99.0)
                    } else {
                        // Fallback: report MB downloaded as pseudo-progress
                        (current_size as f64 / (1024.0 * 1024.0)).min(99.0)
                    };
                    let size_mb = current_size as f64 / (1024.0 * 1024.0);
                    eprintln!(
                        "whisperkit download progress: {} — {:.1} MB / {:.1} MB ({:.1}%)",
                        poll_model_name,
                        size_mb,
                        estimated_bytes as f64 / (1024.0 * 1024.0),
                        progress,
                    );
                    let _ = poll_app_handle.emit(
                        "model-download-progress",
                        serde_json::json!({
                            "model_name": poll_model_name,
                            "progress": progress,
                            "message": format!("{:.1} MB downloaded", size_mb),
                        }),
                    );
                }
            }
        });

        // Wait for process to complete
        let output = child.wait_with_output().await
            .map_err(|e| format!("Failed to wait for whisperkit-cli: {}", e))?;

        // Stop the polling task
        poll_handle.abort();

        // Clean up silent WAV
        let _ = tokio::fs::remove_file(&silent_wav).await;

        // Log output for debugging
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        if !stdout_str.is_empty() {
            eprintln!("whisperkit-cli stdout: {}", stdout_str);
        }
        if !stderr_str.is_empty() {
            eprintln!("whisperkit-cli stderr: {}", stderr_str);
        }

        // The transcribe command may return non-zero (e.g., if audio is too short)
        // but the model download still succeeds. Check if the model directory exists.
        if !model_dir.exists() {
            return Err(format!(
                "whisperkit-cli failed to download model. Exit code: {}. stderr: {}",
                output.status.code().unwrap_or(-1),
                stderr_str.chars().take(500).collect::<String>(),
            ));
        }

        Ok(())
    }
}
