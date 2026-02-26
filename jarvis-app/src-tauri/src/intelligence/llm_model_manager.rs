// LlmModelManager - manages LLM model catalog, downloads, and status tracking
//
// Downloads use a separate short-lived Python process (not the inference sidecar)
// to avoid blocking inference during multi-GB downloads.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::Mutex as TokioMutex;
use tokio_util::sync::CancellationToken;

use crate::settings::ModelStatus;

/// Static metadata for an LLM model in the catalog
struct LlmModelEntry {
    id: &'static str,
    repo_id: &'static str,
    display_name: &'static str,
    description: &'static str,
    size_estimate: &'static str,
    quality_tier: &'static str,
}

/// Information about an LLM model (returned to frontend)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmModelInfo {
    pub id: String,
    pub display_name: String,
    pub repo_id: String,
    pub description: String,
    pub size_estimate: String,
    pub quality_tier: String,
    pub status: ModelStatus,
}

/// Internal state for an in-progress download
struct DownloadState {
    progress: f32,
    cancel_token: CancellationToken,
}

/// Manages LLM model discovery, download, and status tracking.
///
/// Models are downloaded from HuggingFace via a short-lived Python sidecar process.
/// The inference sidecar (MlxProvider) is separate and never blocked by downloads.
pub struct LlmModelManager {
    models_dir: PathBuf,
    app_handle: AppHandle,
    python_path: String,
    download_queue: Arc<TokioMutex<HashMap<String, DownloadState>>>,
    error_states: Arc<TokioMutex<HashMap<String, String>>>,
}

impl LlmModelManager {
    /// Model catalog — curated list of MLX-compatible models from mlx-community.
    /// Ordered by quality tier (basic → best) for UI display.
    const LLM_MODEL_CATALOG: &'static [LlmModelEntry] = &[
        LlmModelEntry {
            id: "llama-3.2-3b-4bit",
            repo_id: "mlx-community/Llama-3.2-3B-Instruct-4bit",
            display_name: "Llama 3.2 3B (Q4)",
            description: "Fast and lightweight. Good for quick tasks.",
            size_estimate: "~2 GB",
            quality_tier: "basic",
        },
        LlmModelEntry {
            id: "qwen3-4b-4bit",
            repo_id: "mlx-community/Qwen3-4B-4bit",
            display_name: "Qwen 3 4B (Q4)",
            description: "Compact and efficient. Good balance for smaller machines.",
            size_estimate: "~3 GB",
            quality_tier: "good",
        },
        LlmModelEntry {
            id: "qwen3-8b-4bit",
            repo_id: "mlx-community/Qwen3-8B-4bit",
            display_name: "Qwen 3 8B (Q4)",
            description: "Great quality, balanced performance. Recommended.",
            size_estimate: "~5 GB",
            quality_tier: "great",
        },
        LlmModelEntry {
            id: "qwen3-14b-4bit",
            repo_id: "mlx-community/Qwen3-14B-4bit",
            display_name: "Qwen 3 14B (Q4)",
            description: "Highest quality. Needs 16GB+ RAM.",
            size_estimate: "~9 GB",
            quality_tier: "best",
        },
    ];

    /// Creates a new LlmModelManager.
    ///
    /// Creates ~/.jarvis/models/llm/ directory if it doesn't exist.
    /// `python_path` should be the resolved path (venv Python if available).
    pub fn new(app_handle: AppHandle, python_path: String) -> Result<Self, String> {
        let home_dir =
            dirs::home_dir().ok_or_else(|| "Failed to get home directory".to_string())?;

        let models_dir = home_dir.join(".jarvis").join("models").join("llm");

        if !models_dir.exists() {
            std::fs::create_dir_all(&models_dir)
                .map_err(|e| format!("Failed to create LLM models directory: {}", e))?;
        }

        Ok(Self {
            models_dir,
            app_handle,
            python_path,
            download_queue: Arc::new(TokioMutex::new(HashMap::new())),
            error_states: Arc::new(TokioMutex::new(HashMap::new())),
        })
    }

    /// Returns the path where a model is (or would be) stored on disk.
    ///
    /// Derives directory name from repo_id: "mlx-community/Qwen3-8B-4bit" → "Qwen3-8B-4bit"
    pub fn model_path(&self, model_id: &str) -> PathBuf {
        if let Some(entry) = Self::catalog_entry(model_id) {
            let model_dir = entry.repo_id.split('/').next_back().unwrap_or(model_id);
            self.models_dir.join(model_dir)
        } else {
            self.models_dir.join(model_id)
        }
    }

    /// Look up a catalog entry by ID
    fn catalog_entry(model_id: &str) -> Option<&'static LlmModelEntry> {
        Self::LLM_MODEL_CATALOG.iter().find(|e| e.id == model_id)
    }

    /// Validates that a downloaded model directory contains config.json
    fn validate_model(&self, model_id: &str) -> bool {
        self.model_path(model_id).join("config.json").exists()
    }

    /// Lists all supported LLM models with their current status.
    ///
    /// Returns one LlmModelInfo per catalog entry. Status reflects:
    /// - Downloading: model is currently being downloaded (includes progress)
    /// - Error: previous download failed (includes error message)
    /// - Downloaded: model directory exists with valid config.json
    /// - NotDownloaded: model is not available locally
    pub async fn list_models(&self) -> Result<Vec<LlmModelInfo>, String> {
        let mut models = Vec::new();
        let download_queue = self.download_queue.lock().await;
        let error_states = self.error_states.lock().await;

        for entry in Self::LLM_MODEL_CATALOG {
            let model_path = self.model_path(entry.id);

            let status = if let Some(download_state) = download_queue.get(entry.id) {
                ModelStatus::Downloading {
                    progress: download_state.progress,
                }
            } else if let Some(error_msg) = error_states.get(entry.id) {
                ModelStatus::Error {
                    message: error_msg.clone(),
                }
            } else if model_path.exists() && self.validate_model(entry.id) {
                match Self::calculate_dir_size(&model_path) {
                    Ok(size) => ModelStatus::Downloaded { size_bytes: size },
                    Err(_) => ModelStatus::Downloaded { size_bytes: 0 },
                }
            } else {
                ModelStatus::NotDownloaded
            };

            models.push(LlmModelInfo {
                id: entry.id.to_string(),
                display_name: entry.display_name.to_string(),
                repo_id: entry.repo_id.to_string(),
                description: entry.description.to_string(),
                size_estimate: entry.size_estimate.to_string(),
                quality_tier: entry.quality_tier.to_string(),
                status,
            });
        }

        Ok(models)
    }

    /// Downloads a model from HuggingFace via a short-lived Python sidecar process.
    ///
    /// Spawns an async task and returns immediately.
    /// Progress: emitted as `llm-model-download-progress` Tauri events.
    /// Completion: emitted as `llm-model-download-complete` event.
    /// Errors: emitted as `llm-model-download-error` event.
    ///
    /// Downloads to a `.downloads/` temp directory then atomically renames on success.
    pub async fn download_model(&self, model_id: String) -> Result<(), String> {
        let entry = Self::catalog_entry(&model_id)
            .ok_or_else(|| format!("Unknown model: {}", model_id))?;

        // Prevent concurrent downloads of the same model
        let mut download_queue = self.download_queue.lock().await;
        if download_queue.contains_key(&model_id) {
            return Err(format!("Model {} is already being downloaded", model_id));
        }

        let cancel_token = CancellationToken::new();
        download_queue.insert(
            model_id.clone(),
            DownloadState {
                progress: 0.0,
                cancel_token: cancel_token.clone(),
            },
        );
        drop(download_queue);

        // Resolve paths
        let repo_id = entry.repo_id.to_string();
        let model_dir_name = entry
            .repo_id
            .split('/')
            .last()
            .unwrap_or(&model_id)
            .to_string();
        let estimated_bytes = Self::parse_size_estimate(entry.size_estimate);
        let downloads_dir = self.models_dir.join(".downloads");
        let download_dest = downloads_dir.join(&model_dir_name);
        let final_dest = self.models_dir.join(&model_dir_name);
        let sidecar_path = Self::resolve_sidecar_path()?;
        let python_path = self.python_path.clone();
        let app_handle = self.app_handle.clone();
        let download_queue_clone = self.download_queue.clone();
        let error_states_clone = self.error_states.clone();

        // Create .downloads directory
        if !downloads_dir.exists() {
            std::fs::create_dir_all(&downloads_dir)
                .map_err(|e| format!("Failed to create downloads directory: {}", e))?;
        }

        // Spawn async download task (returns immediately to caller)
        tokio::spawn(async move {
            let result = Self::download_task(
                model_id.clone(),
                repo_id,
                download_dest.clone(),
                final_dest,
                sidecar_path,
                python_path,
                estimated_bytes,
                app_handle.clone(),
                download_queue_clone.clone(),
                cancel_token,
            )
            .await;

            // Remove from download queue
            download_queue_clone.lock().await.remove(&model_id);

            match result {
                Ok(()) => {
                    error_states_clone.lock().await.remove(&model_id);
                    let _ = app_handle.emit(
                        "llm-model-download-complete",
                        serde_json::json!({ "model_id": model_id }),
                    );
                }
                Err(e) => {
                    // Clean up partial download
                    let _ = std::fs::remove_dir_all(&download_dest);

                    if e != "Download cancelled" {
                        error_states_clone
                            .lock()
                            .await
                            .insert(model_id.clone(), e.clone());
                    }
                    let _ = app_handle.emit(
                        "llm-model-download-error",
                        serde_json::json!({ "model_id": model_id, "error": e }),
                    );
                }
            }
        });

        Ok(())
    }

    /// Internal download task — runs inside a spawned tokio task.
    ///
    /// Flow:
    /// 1. Spawn Python sidecar in download mode
    /// 2. Send download-model command via stdin, then close stdin
    /// 3. Poll directory size for progress (every 2s)
    /// 4. Read stdout for completion/error response
    /// 5. On success: validate config.json, rename .downloads/ → final location
    #[allow(clippy::too_many_arguments)]
    async fn download_task(
        model_id: String,
        repo_id: String,
        download_dest: PathBuf,
        final_dest: PathBuf,
        sidecar_path: PathBuf,
        python_path: String,
        estimated_bytes: u64,
        app_handle: AppHandle,
        download_queue: Arc<TokioMutex<HashMap<String, DownloadState>>>,
        cancel_token: CancellationToken,
    ) -> Result<(), String> {
        eprintln!(
            "LLM download: starting {} (repo: {}, dest: {})",
            model_id,
            repo_id,
            download_dest.display()
        );

        // Spawn Python process in download mode
        // Set current_dir to home to avoid inheriting a stale/deleted cwd from the parent process
        let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
        let mut child = tokio::process::Command::new(&python_path)
            .arg(&sidecar_path)
            .current_dir(&home)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn MLX download process: {}", e))?;

        let stdin = child.stdin.take().ok_or("Failed to get stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to get stdout")?;
        let stderr = child.stderr.take().ok_or("Failed to get stderr")?;

        // Monitor stderr in background
        tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stderr);
            let mut line = String::new();
            while let Ok(n) = reader.read_line(&mut line).await {
                if n == 0 {
                    break;
                }
                eprint!("[MLX Download] {}", line);
                line.clear();
            }
        });

        // Send download-model command and close stdin
        let command = serde_json::json!({
            "command": "download-model",
            "repo_id": repo_id,
            "destination": download_dest.to_string_lossy(),
        });

        let mut writer = tokio::io::BufWriter::new(stdin);
        writer
            .write_all(command.to_string().as_bytes())
            .await
            .map_err(|e| format!("Failed to write download command: {}", e))?;
        writer
            .write_all(b"\n")
            .await
            .map_err(|e| format!("Failed to write newline: {}", e))?;
        writer
            .flush()
            .await
            .map_err(|e| format!("Failed to flush stdin: {}", e))?;
        drop(writer); // Close stdin — Python exits after completing download

        // Poll directory size for progress updates
        let poll_dest = download_dest.clone();
        let poll_model_id = model_id.clone();
        let poll_app_handle = app_handle.clone();
        let poll_download_queue = download_queue.clone();
        let poll_cancel = cancel_token.clone();

        let poll_handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                if poll_cancel.is_cancelled() {
                    break;
                }

                let current_size =
                    LlmModelManager::calculate_dir_size(&poll_dest).unwrap_or(0);
                let progress = if estimated_bytes > 0 {
                    ((current_size as f64 / estimated_bytes as f64) * 100.0).min(99.0) as f32
                } else {
                    0.0
                };

                if let Some(state) = poll_download_queue.lock().await.get_mut(&poll_model_id) {
                    state.progress = progress;
                }

                let downloaded_mb = current_size as f64 / (1024.0 * 1024.0);
                let _ = poll_app_handle.emit(
                    "llm-model-download-progress",
                    serde_json::json!({
                        "model_id": poll_model_id,
                        "progress": progress,
                        "downloaded_mb": downloaded_mb,
                    }),
                );
            }
        });

        // Read stdout for completion/error response from Python
        let mut reader = tokio::io::BufReader::new(stdout);
        let mut result = Err("Download process ended without response".to_string());

        loop {
            let mut line = String::new();
            let read_result = tokio::select! {
                r = reader.read_line(&mut line) => r,
                _ = cancel_token.cancelled() => {
                    let _ = child.kill().await;
                    poll_handle.abort();
                    return Err("Download cancelled".to_string());
                }
            };

            match read_result {
                Ok(0) => break, // EOF — process exited
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    if let Ok(response) = serde_json::from_str::<serde_json::Value>(trimmed) {
                        let resp_type = response
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        match resp_type {
                            "progress" => {
                                // Progress from Python (if huggingface_hub reports it)
                                if let Some(progress) =
                                    response.get("progress").and_then(|v| v.as_f64())
                                {
                                    if let Some(state) =
                                        download_queue.lock().await.get_mut(&model_id)
                                    {
                                        state.progress = progress as f32;
                                    }
                                    let downloaded_mb = response
                                        .get("downloaded_mb")
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(0.0);
                                    let _ = app_handle.emit(
                                        "llm-model-download-progress",
                                        serde_json::json!({
                                            "model_id": model_id,
                                            "progress": progress,
                                            "downloaded_mb": downloaded_mb,
                                        }),
                                    );
                                }
                            }
                            "response" => {
                                if response
                                    .get("success")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false)
                                {
                                    result = Ok(());
                                } else {
                                    let error = response
                                        .get("error")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("Unknown error")
                                        .to_string();
                                    result = Err(error);
                                }
                                break;
                            }
                            "error" => {
                                let error = response
                                    .get("error")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown error")
                                    .to_string();
                                result = Err(error);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    result = Err(format!("Failed to read download output: {}", e));
                    break;
                }
            }
        }

        // Stop progress polling
        poll_handle.abort();

        // Wait for Python process to exit
        let _ = child.wait().await;

        // Check download result
        result?;

        // Validate downloaded model
        if !download_dest.join("config.json").exists() {
            return Err("Downloaded model is missing config.json".to_string());
        }

        // Atomic rename from .downloads/ to final location
        if final_dest.exists() {
            std::fs::remove_dir_all(&final_dest)
                .map_err(|e| format!("Failed to remove existing model directory: {}", e))?;
        }
        std::fs::rename(&download_dest, &final_dest)
            .map_err(|e| format!("Failed to move model to final location: {}", e))?;

        eprintln!(
            "LLM download: {} complete ({})",
            model_id,
            final_dest.display()
        );

        Ok(())
    }

    /// Cancels an in-progress download.
    ///
    /// Triggers the CancellationToken which kills the Python process
    /// and cleans up the .downloads/ directory.
    pub async fn cancel_download(&self, model_id: String) -> Result<(), String> {
        let mut download_queue = self.download_queue.lock().await;

        let state = download_queue
            .remove(&model_id)
            .ok_or_else(|| format!("Model {} is not being downloaded", model_id))?;

        drop(download_queue);

        // Trigger cancellation — kills process, stops polling
        state.cancel_token.cancel();

        // Clean up download directory (best effort)
        if let Some(entry) = Self::catalog_entry(&model_id) {
            let model_dir_name = entry.repo_id.split('/').next_back().unwrap_or(&model_id);
            let download_dest = self.models_dir.join(".downloads").join(model_dir_name);
            let _ = std::fs::remove_dir_all(&download_dest);
        }

        Ok(())
    }

    /// Deletes a downloaded model from disk.
    ///
    /// Note: Active model protection (preventing deletion of the currently active model)
    /// is enforced at the Tauri command layer where settings are accessible.
    pub async fn delete_model(&self, model_id: String) -> Result<(), String> {
        let model_path = self.model_path(&model_id);

        if !model_path.exists() {
            return Err(format!("Model {} is not downloaded", model_id));
        }

        std::fs::remove_dir_all(&model_path)
            .map_err(|e| format!("Failed to delete model: {}", e))?;

        // Clear any error state for this model
        self.error_states.lock().await.remove(&model_id);

        Ok(())
    }

    /// Resolve the MLX sidecar script path (server.py).
    ///
    /// Checks production bundle (Contents/Resources/) first,
    /// then falls back to dev mode path (CARGO_MANIFEST_DIR/sidecars/).
    fn resolve_sidecar_path() -> Result<PathBuf, String> {
        // Production: look in app bundle Resources/
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                // macOS bundle structure: Contents/MacOS/binary → Contents/Resources/
                if let Some(contents_dir) = exe_dir.parent() {
                    let bundled_path = contents_dir
                        .join("Resources")
                        .join("sidecars")
                        .join("mlx-server")
                        .join("server.py");
                    if bundled_path.exists() {
                        return Ok(bundled_path);
                    }
                }
            }
        }

        // Dev mode: CARGO_MANIFEST_DIR/sidecars/mlx-server/server.py
        let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("sidecars")
            .join("mlx-server")
            .join("server.py");
        if dev_path.exists() {
            return Ok(dev_path);
        }

        Err(format!(
            "MLX sidecar script not found. Checked:\n  - app bundle Resources/sidecars/mlx-server/server.py\n  - {:?}",
            dev_path
        ))
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

    /// Parses a size estimate string like "~5 GB" to bytes
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
}
