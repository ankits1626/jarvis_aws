// MlxProvider - manages MLX sidecar process and NDJSON communication

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use super::provider::{AvailabilityResult, IntelProvider};
use super::utils::split_content;

/// Max characters per chunk for MLX models (15,000 chars ~= 6,000 tokens)
const MAX_CONTENT_CHARS: usize = 15_000;

/// NDJSON command structure for MLX sidecar protocol
#[derive(Serialize)]
struct NdjsonCommand {
    command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    destination: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    audio_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    capabilities: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<String>,
}

/// NDJSON response structure from MLX sidecar
#[derive(Deserialize, Debug)]
struct NdjsonResponse {
    #[serde(rename = "type")]
    response_type: String,
    #[serde(default)]
    _command: Option<String>,
    #[serde(default)]
    available: Option<bool>,
    #[serde(default)]
    success: Option<bool>,
    #[serde(default)]
    model_name: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    _param_count: Option<u64>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    transcript: Option<String>,
    #[serde(default)]
    capabilities: Option<Vec<String>>,
    // Co-Pilot analysis fields
    #[serde(default)]
    new_content: Option<String>,
    #[serde(default)]
    updated_summary: Option<String>,
    #[serde(default)]
    key_points: Option<Vec<String>>,
    #[serde(default)]
    decisions: Option<Vec<String>>,
    #[serde(default)]
    action_items: Option<Vec<String>>,
    #[serde(default)]
    open_questions: Option<Vec<String>>,
    #[serde(default)]
    suggested_questions: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    key_concepts: Option<Vec<serde_json::Value>>,
}

/// MLX provider state
struct ProviderState {
    /// Child process handle
    child: Option<Child>,
    /// Current model name
    model_name: Option<String>,
    /// Cached availability result
    availability: AvailabilityResult,
    /// Stdin writer (buffered for efficiency)
    stdin: Option<BufWriter<tokio::process::ChildStdin>>,
    /// Stdout reader (buffered for line reading)
    stdout: Option<BufReader<tokio::process::ChildStdout>>,
}

/// MLX provider - manages sidecar lifecycle and NDJSON communication
pub struct MlxProvider {
    /// Shared state protected by mutex (only one command in-flight at a time)
    state: Arc<Mutex<ProviderState>>,
}

impl MlxProvider {
    /// Resolve the MLX sidecar script path.
    ///
    /// Production (bundled .app): Tauri places resources at Contents/Resources/
    /// Dev mode: Script is at src-tauri/sidecars/mlx-server/server.py
    fn resolve_sidecar_path() -> Result<PathBuf, String> {
        // Production: look in Resources directory
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                // Contents/MacOS/ -> Contents/Resources/
                if let Some(contents_dir) = exe_dir.parent() {
                    let prod_path = contents_dir
                        .join("Resources/sidecars/mlx-server/server.py");
                    if prod_path.exists() {
                        eprintln!("MLX: Using bundled script at {:?}", prod_path);
                        return Ok(prod_path);
                    }
                }
            }
        }

        // Dev mode: CARGO_MANIFEST_DIR/sidecars/mlx-server/server.py
        let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("sidecars/mlx-server/server.py");
        if dev_path.exists() {
            eprintln!("MLX: Using dev mode path: {:?}", dev_path);
            return Ok(dev_path);
        }

        Err(format!(
            "MLX sidecar script not found. Checked:\n  - Contents/Resources/sidecars/mlx-server/server.py\n  - {:?}",
            dev_path
        ))
    }

    /// Check if Python is installed and accessible
    async fn check_python_installed(python_path: &str) -> Result<(), String> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        let output = Command::new(python_path)
            .arg("--version")
            .current_dir(&home)
            .output()
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    format!("Python not found at '{}'. Please install Python 3.10+ or update the python_path in settings.", python_path)
                } else {
                    format!("Failed to check Python version: {}", e)
                }
            })?;

        if !output.status.success() {
            return Err(format!("Python check failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        eprintln!("MLX: Python found: {}", String::from_utf8_lossy(&output.stdout).trim());
        Ok(())
    }

    /// Create a new provider and spawn the sidecar
    pub async fn new(
        _app_handle: tauri::AppHandle,
        model_path: PathBuf,
        python_path: String,
    ) -> Result<Self, String> {
        // Check if Python is installed before attempting to spawn sidecar
        Self::check_python_installed(&python_path).await?;

        let sidecar_path = Self::resolve_sidecar_path()?;

        // Spawn Python sidecar using tokio::process::Command
        // Set current_dir to home to avoid inheriting a stale/deleted cwd from the parent process
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        let mut child = Command::new(&python_path)
            .arg(&sidecar_path)
            .current_dir(&home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                format!("Failed to spawn MLX sidecar (Python found but spawn failed): {}", e)
            })?;

        // Take ownership of stdio handles
        let stdin = child
            .stdin
            .take()
            .ok_or("Failed to get stdin handle")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Failed to get stdout handle")?;
        let stderr = child
            .stderr
            .take()
            .ok_or("Failed to get stderr handle")?;

        // Spawn stderr monitoring task
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            while let Ok(n) = reader.read_line(&mut line).await {
                if n == 0 {
                    break;
                }
                eprint!("[MLX] {}", line);
                line.clear();
            }
        });

        let state = ProviderState {
            child: Some(child),
            model_name: None,
            availability: AvailabilityResult {
                available: false,
                reason: None,
            },
            stdin: Some(BufWriter::new(stdin)),
            stdout: Some(BufReader::new(stdout)),
        };

        let provider = Self {
            state: Arc::new(Mutex::new(state)),
        };

        // Check availability with 15s timeout (allows for model loading)
        let availability = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            provider.check_availability_internal(),
        )
        .await
        .map_err(|_| "MLX availability check timeout (15s). The sidecar may be unresponsive.".to_string())??;

        {
            let mut state = provider.state.lock().await;
            state.availability = availability.clone();
        }

        if !availability.available {
            let reason = availability.reason.unwrap_or_else(|| "Unknown reason".to_string());
            // Provide helpful error messages based on the reason
            if reason.contains("mlx") || reason.contains("import") {
                return Err(format!(
                    "MLX dependencies not installed: {}. Please install mlx and mlx-lm: pip install mlx mlx-lm",
                    reason
                ));
            } else {
                return Err(format!("MLX not available: {}", reason));
            }
        }

        // Load the model with 15s timeout
        tokio::time::timeout(
            std::time::Duration::from_secs(15),
            provider.load_model_internal(model_path),
        )
        .await
        .map_err(|_| "MLX model load timeout (15s)".to_string())??;

        Ok(provider)
    }

    /// Send a command and receive a response with configurable timeout.
    ///
    /// `timeout_secs` controls both the write and read timeout for this command.
    /// Use shorter timeouts (60s) for quick operations like tags/summary,
    /// and longer timeouts (600s) for audio transcription of large files.
    async fn send_command(&self, cmd: NdjsonCommand, timeout_secs: u64) -> Result<NdjsonResponse, String> {
        let command_name = cmd.command.clone();
        let mut state = self.state.lock().await;

        // Serialize command to JSON + newline
        let json = serde_json::to_string(&cmd)
            .map_err(|e| format!("Failed to serialize command: {}", e))?;

        // Write to stdin
        {
            let stdin = state.stdin.as_mut().ok_or("Stdin not available")?;
            let write_future = async {
                stdin.write_all(json.as_bytes()).await?;
                stdin.write_all(b"\n").await?;
                stdin.flush().await?;
                Ok::<_, std::io::Error>(())
            };

            tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), write_future)
                .await
                .map_err(|_| {
                    eprintln!("MLX: Write timeout after {}s for command '{}'. The sidecar may be unresponsive.", timeout_secs, command_name);
                    format!("Command write timeout ({}s) for '{}'", timeout_secs, command_name)
                })?
                .map_err(|e| format!("Failed to write command: {}", e))?;
        }

        // Read one line from stdout
        let mut response_line = String::new();
        {
            let stdout = state.stdout.as_mut().ok_or("Stdout not available")?;
            let read_future = stdout.read_line(&mut response_line);

            tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), read_future)
                .await
                .map_err(|_| {
                    eprintln!(
                        "MLX: Read timeout after {}s for command '{}'. This may indicate the operation is taking longer than expected. \
                        For audio transcription, larger files need more time.",
                        timeout_secs, command_name
                    );
                    format!("Command read timeout ({}s) for '{}'. Try a shorter audio file or increase timeout.", timeout_secs, command_name)
                })?
                .map_err(|e| format!("Failed to read response: {}", e))?;
        }

        if response_line.is_empty() {
            eprintln!("MLX: Sidecar closed connection (broken pipe) during command '{}'", command_name);
            return Err("Sidecar closed connection (broken pipe)".to_string());
        }

        // Deserialize response
        serde_json::from_str(&response_line)
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Internal availability check (sends check-availability command)
    async fn check_availability_internal(&self) -> Result<AvailabilityResult, String> {
        let cmd = NdjsonCommand {
            command: "check-availability".to_string(),
            model_path: None,
            content: None,
            repo_id: None,
            destination: None,
            audio_path: None,
            capabilities: None,
            context: None,
        };

        let response = self.send_command(cmd, 15).await?;

        if response.response_type == "error" {
            return Ok(AvailabilityResult {
                available: false,
                reason: response.error,
            });
        }

        Ok(AvailabilityResult {
            available: response.available.unwrap_or(false),
            reason: None,
        })
    }

    /// Load a model from disk
    async fn load_model_internal(&self, model_path: PathBuf) -> Result<(), String> {
        // Look up model capabilities from catalog
        // Extract model ID from path: ~/.jarvis/models/llm/Qwen3-8B-4bit → "Qwen3-8B-4bit"
        let model_id = model_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("Invalid model path")?;
        
        // Map directory name to catalog ID
        // The catalog uses kebab-case IDs like "qwen3-8b-4bit"
        // The directory names from HuggingFace use PascalCase like "Qwen3-8B-4bit"
        // We need to match them by converting to lowercase and comparing
        let capabilities = Self::lookup_capabilities(model_id);
        
        let cmd = NdjsonCommand {
            command: "load-model".to_string(),
            model_path: Some(model_path.to_string_lossy().to_string()),
            content: None,
            repo_id: None,
            destination: None,
            audio_path: None,
            capabilities: Some(capabilities),
            context: None,
        };

        let response = self.send_command(cmd, 60).await?;

        if response.response_type == "error" {
            return Err(response
                .error
                .unwrap_or_else(|| "Failed to load model".to_string()));
        }

        if !response.success.unwrap_or(false) {
            return Err("Model load failed".to_string());
        }

        // Update state with model name
        {
            let mut state = self.state.lock().await;
            state.model_name = response.model_name;
        }

        Ok(())
    }
    
    /// Look up model capabilities from the catalog
    /// 
    /// This duplicates the catalog from LlmModelManager to avoid circular dependencies.
    /// The catalog is small and static, so duplication is acceptable.
    /// 
    /// Matches against exact directory names derived from repo IDs to avoid false positives.
    fn lookup_capabilities(model_dir_name: &str) -> Vec<String> {
        // Map exact directory names to capabilities
        // Directory names come from HuggingFace repo IDs (last segment after '/')
        let model_lower = model_dir_name.to_lowercase();
        
        // Text-only models - match exact directory names
        if model_lower == "llama-3.2-3b-instruct-4bit" || 
           model_lower == "qwen3-4b-4bit" ||
           model_lower == "qwen3-8b-4bit" ||
           model_lower == "qwen3-14b-4bit" {
            return vec!["text".to_string()];
        }
        
        // Multimodal models (Qwen 2.5 Omni) - match exact directory names
        if model_lower == "qwen2.5-omni-3b-mlx-8bit" || 
           model_lower == "qwen2.5-omni-7b-mlx-4bit" {
            return vec!["audio".to_string(), "text".to_string()];
        }
        
        // Fallback: use contains() for partial matches (less precise but more flexible)
        // This handles cases where directory names might vary slightly
        if model_lower.contains("qwen") && model_lower.contains("omni") {
            return vec!["audio".to_string(), "text".to_string()];
        }
        
        // Default to text-only for unknown models
        vec!["text".to_string()]
    }

    /// Switch to a different model
    pub async fn switch_model(&self, model_path: PathBuf) -> Result<(), String> {
        // Save previous model name in case we need to rollback
        let previous_model = {
            let state = self.state.lock().await;
            state.model_name.clone()
        };

        match self.load_model_internal(model_path).await {
            Ok(()) => Ok(()),
            Err(e) => {
                // Restore previous model name on failure
                let mut state = self.state.lock().await;
                state.model_name = previous_model;
                Err(e)
            }
        }
    }

    /// Shutdown the sidecar gracefully
    pub async fn shutdown(&self) {
        let cmd = NdjsonCommand {
            command: "shutdown".to_string(),
            model_path: None,
            content: None,
            repo_id: None,
            destination: None,
            audio_path: None,
            capabilities: None,
            context: None,
        };

        // Try to send shutdown command (ignore errors)
        let _ = self.send_command(cmd, 5).await;

        // Wait up to 3 seconds for graceful exit
        let mut state = self.state.lock().await;
        if let Some(mut child) = state.child.take() {
            let wait_future = child.wait();
            if tokio::time::timeout(std::time::Duration::from_secs(3), wait_future)
                .await
                .is_err()
            {
                // Timeout - send SIGTERM
                let _ = child.kill().await;
            }
        }
    }

    /// Generate tags for a single chunk
    async fn generate_tags_chunk(&self, content: &str) -> Result<Vec<String>, String> {
        let cmd = NdjsonCommand {
            command: "generate-tags".to_string(),
            model_path: None,
            content: Some(content.to_string()),
            repo_id: None,
            destination: None,
            audio_path: None,
            capabilities: None,
            context: None,
        };

        let response = self.send_command(cmd, 60).await?;

        if response.response_type == "error" {
            return Err(response
                .error
                .unwrap_or_else(|| "Tag generation failed".to_string()));
        }

        let tags = response
            .tags
            .ok_or_else(|| "No tags in response".to_string())?;

        if tags.is_empty() {
            return Err("Model returned no tags".to_string());
        }

        Ok(tags)
    }

    /// Summarize a single chunk
    async fn summarize_chunk(&self, content: &str) -> Result<String, String> {
        let cmd = NdjsonCommand {
            command: "summarize".to_string(),
            model_path: None,
            content: Some(content.to_string()),
            repo_id: None,
            destination: None,
            audio_path: None,
            capabilities: None,
            context: None,
        };

        let response = self.send_command(cmd, 60).await?;

        if response.response_type == "error" {
            return Err(response
                .error
                .unwrap_or_else(|| "Summarization failed".to_string()));
        }

        let summary = response
            .summary
            .ok_or_else(|| "No summary in response".to_string())?;

        if summary.is_empty() {
            return Err("Model returned empty summary".to_string());
        }

        Ok(summary)
    }

    /// Generate transcript from audio file.
    ///
    /// Uses a 600s timeout to support large audio files (10+ minutes).
    /// The send_command itself also uses 600s so neither layer cuts off early.
    async fn generate_transcript_internal(&self, audio_path: &std::path::Path) -> Result<super::provider::TranscriptResult, String> {
        let audio_path_str = audio_path.to_string_lossy().to_string();
        eprintln!("MLX: Starting transcript generation for '{}'", audio_path_str);

        let cmd = NdjsonCommand {
            command: "generate-transcript".to_string(),
            model_path: None,
            content: None,
            repo_id: None,
            destination: None,
            audio_path: Some(audio_path_str.clone()),
            capabilities: None,
            context: None,
        };

        // 600s (10 min) timeout for transcript generation — large audio files need time
        let response = self.send_command(cmd, 600).await
            .map_err(|e| {
                eprintln!("MLX: Transcript generation failed for '{}': {}", audio_path_str, e);
                e
            })?;

        if response.response_type == "error" {
            let err = response.error.unwrap_or_else(|| "Transcript generation failed".to_string());
            eprintln!("MLX: Transcript generation error for '{}': {}", audio_path_str, err);
            return Err(err);
        }

        let language = response.language.ok_or("No language in response")?;
        let transcript = response.transcript.ok_or("No transcript in response")?;

        eprintln!("MLX: Transcript generation complete for '{}' (language: {})", audio_path_str, language);
        Ok(super::provider::TranscriptResult { language, transcript })
    }
    
    /// Analyze audio chunk with running context for Co-Pilot.
    ///
    /// Uses a 120s timeout as specified in requirements (R11.2).
    async fn copilot_analyze_internal(
        &self,
        audio_path: &std::path::Path,
        context: &str,
    ) -> Result<super::provider::CoPilotCycleResult, String> {
        let audio_path_str = audio_path.to_string_lossy().to_string();
        eprintln!("MLX: Starting Co-Pilot analysis for '{}'", audio_path_str);

        let cmd = NdjsonCommand {
            command: "copilot-analyze".to_string(),
            model_path: None,
            content: None,
            repo_id: None,
            destination: None,
            audio_path: Some(audio_path_str.clone()),
            capabilities: None,
            context: Some(context.to_string()),
        };

        // 120s timeout for Co-Pilot analysis (R11.2)
        let response = self.send_command(cmd, 120).await
            .map_err(|e| {
                eprintln!("MLX: Co-Pilot analysis failed for '{}': {}", audio_path_str, e);
                e
            })?;

        if response.response_type == "error" {
            let err = response.error.unwrap_or_else(|| "Co-Pilot analysis failed".to_string());
            eprintln!("MLX: Co-Pilot analysis error for '{}': {}", audio_path_str, err);
            return Err(err);
        }

        // Parse response with graceful handling of missing fields (R2.7)
        let new_content = response.new_content.unwrap_or_default();
        let updated_summary = response.updated_summary.unwrap_or_default();
        let key_points = response.key_points.unwrap_or_default();
        let decisions = response.decisions.unwrap_or_default();
        let action_items = response.action_items.unwrap_or_default();
        let open_questions = response.open_questions.unwrap_or_default();

        // Parse suggested_questions array
        let suggested_questions = response.suggested_questions
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| {
                if let (Some(question), Some(reason)) = (
                    v.get("question").and_then(|q| q.as_str()),
                    v.get("reason").and_then(|r| r.as_str()),
                ) {
                    Some(super::provider::CoPilotQuestion {
                        question: question.to_string(),
                        reason: reason.to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        // Parse key_concepts array
        let key_concepts = response.key_concepts
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| {
                if let (Some(term), Some(context)) = (
                    v.get("term").and_then(|t| t.as_str()),
                    v.get("context").and_then(|c| c.as_str()),
                ) {
                    Some(super::provider::CoPilotConcept {
                        term: term.to_string(),
                        context: context.to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        eprintln!("MLX: Co-Pilot analysis complete for '{}'", audio_path_str);
        Ok(super::provider::CoPilotCycleResult {
            new_content,
            updated_summary,
            key_points,
            decisions,
            action_items,
            open_questions,
            suggested_questions,
            key_concepts,
        })
    }
}

#[async_trait]
impl IntelProvider for MlxProvider {
    async fn check_availability(&self) -> AvailabilityResult {
        let state = self.state.lock().await;
        state.availability.clone()
    }

    async fn generate_tags(&self, content: &str) -> Result<Vec<String>, String> {
        let chunks = split_content(content, MAX_CONTENT_CHARS);

        if chunks.len() == 1 {
            return self.generate_tags_chunk(content).await;
        }

        eprintln!(
            "MLX: Content too large ({} chars), splitting into {} chunks for tag generation",
            content.len(),
            chunks.len()
        );

        let mut all_tags = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            match self.generate_tags_chunk(chunk).await {
                Ok(tags) => {
                    eprintln!(
                        "MLX: Chunk {}/{} produced {} tags",
                        i + 1,
                        chunks.len(),
                        tags.len()
                    );
                    all_tags.extend(tags);
                }
                Err(e) => {
                    eprintln!("MLX: Chunk {}/{} failed: {}", i + 1, chunks.len(), e);
                }
            }
        }

        // Deduplicate (case-insensitive) and take top 5
        let mut seen = std::collections::HashSet::new();
        all_tags.retain(|tag| seen.insert(tag.to_lowercase()));
        all_tags.truncate(5);

        if all_tags.is_empty() {
            return Err("No tags generated from any content chunk".to_string());
        }

        Ok(all_tags)
    }

    async fn summarize(&self, content: &str) -> Result<String, String> {
        let chunks = split_content(content, MAX_CONTENT_CHARS);

        if chunks.len() == 1 {
            return self.summarize_chunk(content).await;
        }

        eprintln!(
            "MLX: Content too large ({} chars), splitting into {} chunks for summarization",
            content.len(),
            chunks.len()
        );

        let mut chunk_summaries = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            match self.summarize_chunk(chunk).await {
                Ok(summary) => {
                    eprintln!("MLX: Chunk {}/{} summarized", i + 1, chunks.len());
                    chunk_summaries.push(summary);
                }
                Err(e) => {
                    eprintln!("MLX: Chunk {}/{} failed: {}", i + 1, chunks.len(), e);
                }
            }
        }

        if chunk_summaries.is_empty() {
            return Err("No summaries generated from any content chunk".to_string());
        }

        if chunk_summaries.len() == 1 {
            return Ok(chunk_summaries.into_iter().next().unwrap());
        }

        // Combine chunk summaries into a final summary
        let combined = chunk_summaries.join("\n");
        match self.summarize_chunk(&combined).await {
            Ok(final_summary) => Ok(final_summary),
            Err(_) => {
                // If combining fails, return the first chunk's summary
                Ok(chunk_summaries.into_iter().next().unwrap())
            }
        }
    }

    async fn generate_transcript(&self, audio_path: &std::path::Path) -> Result<super::provider::TranscriptResult, String> {
        self.generate_transcript_internal(audio_path).await
    }
    
    async fn copilot_analyze(
        &self,
        audio_path: &std::path::Path,
        context: &str,
    ) -> Result<super::provider::CoPilotCycleResult, String> {
        self.copilot_analyze_internal(audio_path, context).await
    }
}
