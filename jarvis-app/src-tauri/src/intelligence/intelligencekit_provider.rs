// IntelligenceKitProvider - manages IntelligenceKit sidecar process and NDJSON communication

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use super::provider::{AvailabilityResult, IntelProvider};

/// Max characters per chunk to stay within Apple Foundation Models' 4096-token context window.
/// Apple's tokenizer averages ~2.5 chars/token. Budget ~3000 tokens for content,
/// leaving ~1000 tokens for prompt, instructions, and output.
const MAX_CONTENT_CHARS: usize = 7_000;

/// Snap a byte index down to the nearest valid UTF-8 char boundary.
fn snap_to_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    let mut i = index;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Split content into chunks at paragraph/line boundaries, each <= max_chars.
/// All slice boundaries are snapped to valid UTF-8 char boundaries.
fn split_content(content: &str, max_chars: usize) -> Vec<&str> {
    if content.len() <= max_chars {
        return vec![content];
    }

    let mut chunks = Vec::new();
    let mut start = 0;

    while start < content.len() {
        if start + max_chars >= content.len() {
            chunks.push(&content[start..]);
            break;
        }

        let end = snap_to_char_boundary(content, start + max_chars);

        // Try to break at paragraph boundary (double newline) within last 500 chars
        let search_start = snap_to_char_boundary(content, if end > start + 500 { end - 500 } else { start });
        let break_pos = content[search_start..end]
            .rfind("\n\n")
            .map(|pos| search_start + pos + 2)
            .or_else(|| {
                content[search_start..end]
                    .rfind('\n')
                    .map(|pos| search_start + pos + 1)
            })
            .or_else(|| {
                content[search_start..end]
                    .rfind(' ')
                    .map(|pos| search_start + pos + 1)
            })
            .unwrap_or(end);

        chunks.push(&content[start..break_pos]);
        start = break_pos;
    }

    chunks
}

/// NDJSON command structure for IntelligenceKit protocol
#[derive(Serialize)]
struct NdjsonCommand {
    command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_format: Option<String>,
}

/// NDJSON response structure from IntelligenceKit
#[derive(Deserialize)]
struct NdjsonResponse {
    ok: bool,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    available: Option<bool>,
    #[serde(default)]
    reason: Option<String>,
}

/// IntelligenceKit provider state
struct ProviderState {
    /// Child process handle (tokio::process::Child, not Tauri CommandChild)
    child: Option<Child>,
    /// Current session ID (None if no session open)
    session_id: Option<String>,
    /// Cached availability result
    availability: AvailabilityResult,
    /// Stdin writer (buffered for efficiency)
    stdin: Option<BufWriter<tokio::process::ChildStdin>>,
    /// Stdout reader (buffered for line reading)
    stdout: Option<BufReader<tokio::process::ChildStdout>>,
}

/// IntelligenceKit provider - manages sidecar lifecycle and NDJSON communication
pub struct IntelligenceKitProvider {
    /// Shared state protected by mutex (only one command in-flight at a time)
    state: Arc<Mutex<ProviderState>>,
}

impl IntelligenceKitProvider {
    /// Resolve the IntelligenceKit binary path.
    ///
    /// Production (bundled .app): Tauri places externalBin at Contents/MacOS/<name>
    /// (no target-triple suffix, no binaries/ prefix). We find it next to the main executable.
    ///
    /// Dev mode: Binary is at src-tauri/binaries/IntelligenceKit-<target-triple>.
    fn resolve_binary_path() -> Result<std::path::PathBuf, String> {
        // Production: look next to the running executable (Contents/MacOS/)
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                let prod_path = exe_dir.join("IntelligenceKit");
                if prod_path.exists() {
                    eprintln!("IntelligenceKit: Using bundled binary at {:?}", prod_path);
                    return Ok(prod_path);
                }
            }
        }

        // Dev mode: CARGO_MANIFEST_DIR/binaries/IntelligenceKit-<target-triple>
        let target_triple = format!(
            "{}-{}-{}",
            std::env::consts::ARCH,
            if cfg!(target_os = "macos") { "apple" }
            else if cfg!(target_os = "linux") { "unknown-linux" }
            else if cfg!(target_os = "windows") { "pc-windows" }
            else { std::env::consts::OS },
            if cfg!(target_os = "macos") { "darwin" }
            else if cfg!(target_os = "windows") { "msvc" }
            else { "gnu" }
        );

        let dev_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("binaries/IntelligenceKit-{}", target_triple));
        if dev_path.exists() {
            eprintln!("IntelligenceKit: Using dev mode path: {:?}", dev_path);
            return Ok(dev_path);
        }

        Err(format!(
            "IntelligenceKit binary not found. Checked:\n  - next to executable\n  - {:?}",
            dev_path
        ))
    }

    /// Create a new provider and spawn the sidecar using tokio::process::Command
    pub async fn new(app_handle: tauri::AppHandle) -> Result<Self, String> {
        // Resolve IntelligenceKit binary path.
        //
        // Production: Tauri bundles externalBin into Contents/MacOS/ (no target triple suffix).
        // Dev mode: Binary lives at src-tauri/binaries/IntelligenceKit-<target-triple>.
        let _ = &app_handle; // used for future resolution if needed

        let binary_path = Self::resolve_binary_path()?;

        // Spawn using tokio::process::Command for direct stdin/stdout access
        let mut child = Command::new(binary_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn IntelligenceKit: {}", e))?;

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
                eprint!("[IntelligenceKit] {}", line);
                line.clear();
            }
        });

        let state = ProviderState {
            child: Some(child),
            session_id: None,
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

        // Check availability and open initial session
        let availability = provider.check_availability_internal().await?;
        {
            let mut state = provider.state.lock().await;
            state.availability = availability.clone();
        }

        if availability.available {
            provider.open_session().await?;
        }

        Ok(provider)
    }

    /// Send a command and receive a response (with 30s timeout)
    async fn send_command(&self, cmd: NdjsonCommand) -> Result<NdjsonResponse, String> {
        let mut state = self.state.lock().await;

        // Serialize command to JSON + newline
        let json = serde_json::to_string(&cmd)
            .map_err(|e| format!("Failed to serialize command: {}", e))?;

        // Write to stdin with 30s timeout
        {
            let stdin = state.stdin.as_mut().ok_or("Stdin not available")?;
            let write_future = async {
                stdin.write_all(json.as_bytes()).await?;
                stdin.write_all(b"\n").await?;
                stdin.flush().await?;
                Ok::<_, std::io::Error>(())
            };

            tokio::time::timeout(std::time::Duration::from_secs(30), write_future)
                .await
                .map_err(|_| "Command write timeout".to_string())?
                .map_err(|e| format!("Failed to write command: {}", e))?;
        }

        // Read one line from stdout with 30s timeout
        let mut response_line = String::new();
        {
            let stdout = state.stdout.as_mut().ok_or("Stdout not available")?;
            let read_future = stdout.read_line(&mut response_line);

            tokio::time::timeout(std::time::Duration::from_secs(30), read_future)
                .await
                .map_err(|_| "Command read timeout".to_string())?
                .map_err(|e| format!("Failed to read response: {}", e))?;
        }

        // Deserialize response
        serde_json::from_str(&response_line)
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Internal availability check (sends check-availability command)
    async fn check_availability_internal(&self) -> Result<AvailabilityResult, String> {
        let cmd = NdjsonCommand {
            command: "check-availability".to_string(),
            session_id: None,
            instructions: None,
            prompt: None,
            content: None,
            output_format: None,
        };

        let response = self.send_command(cmd).await?;

        if !response.ok {
            return Ok(AvailabilityResult {
                available: false,
                reason: response.error.or(Some("Unknown error".to_string())),
            });
        }

        Ok(AvailabilityResult {
            available: response.available.unwrap_or(false),
            reason: response.reason,
        })
    }

    /// Open a new session
    async fn open_session(&self) -> Result<String, String> {
        let cmd = NdjsonCommand {
            command: "open-session".to_string(),
            session_id: None,
            instructions: Some(
                "You are a content analysis assistant. Follow the user's instructions precisely."
                    .to_string(),
            ),
            prompt: None,
            content: None,
            output_format: None,
        };

        let response = self.send_command(cmd).await?;

        if !response.ok {
            return Err(response
                .error
                .unwrap_or_else(|| "Failed to open session".to_string()));
        }

        let session_id = response.session_id.ok_or("No session_id in response")?;

        {
            let mut state = self.state.lock().await;
            state.session_id = Some(session_id.clone());
        }

        Ok(session_id)
    }

    /// Ensure a session is open (create if needed)
    async fn ensure_session(&self) -> Result<String, String> {
        let state = self.state.lock().await;
        if let Some(session_id) = &state.session_id {
            return Ok(session_id.clone());
        }
        drop(state);

        self.open_session().await
    }

    /// Shutdown the sidecar gracefully
    pub async fn shutdown(&self) {
        let cmd = NdjsonCommand {
            command: "shutdown".to_string(),
            session_id: None,
            instructions: None,
            prompt: None,
            content: None,
            output_format: None,
        };

        // Try to send shutdown command (ignore errors)
        let _ = self.send_command(cmd).await;

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

    /// Internal helper for generate_tags (no retry logic)
    async fn generate_tags_internal(&self, content: &str) -> Result<Vec<String>, String> {
        let session_id = self.ensure_session().await?;

        let cmd = NdjsonCommand {
            command: "message".to_string(),
            session_id: Some(session_id),
            instructions: None,
            prompt: Some(
                "Generate 3-5 topic tags (1-3 words each) that capture the main themes of this content. Return only the tags as a list."
                    .to_string(),
            ),
            content: Some(content.to_string()),
            output_format: Some("string_list".to_string()),
        };

        let response = self.send_command(cmd).await?;

        if !response.ok {
            return Err(response
                .error
                .unwrap_or_else(|| "Tag generation failed".to_string()));
        }

        let tags: Vec<String> = serde_json::from_value(
            response.result.unwrap_or(serde_json::Value::Array(vec![])),
        )
        .map_err(|e| format!("Failed to parse tags: {}", e))?;

        // Validate and trim tags
        if tags.is_empty() {
            return Err("Model returned no tags".to_string());
        }

        // Trim to max 5 tags if model returned more
        let trimmed_tags: Vec<String> = tags.into_iter().take(5).collect();

        Ok(trimmed_tags)
    }

    /// Generate tags with session-expired retry (single chunk, no chunking logic)
    async fn generate_tags_with_retry(&self, content: &str) -> Result<Vec<String>, String> {
        match self.generate_tags_internal(content).await {
            Err(e) if e.contains("session_not_found") => {
                self.open_session().await?;
                self.generate_tags_internal(content).await
            }
            result => result,
        }
    }

    /// Summarize with session-expired retry (single chunk, no chunking logic)
    async fn summarize_with_retry(&self, content: &str) -> Result<String, String> {
        match self.summarize_internal(content).await {
            Err(e) if e.contains("session_not_found") => {
                self.open_session().await?;
                self.summarize_internal(content).await
            }
            result => result,
        }
    }

    /// Internal helper for summarize (no retry logic)
    async fn summarize_internal(&self, content: &str) -> Result<String, String> {
        let session_id = self.ensure_session().await?;

        let cmd = NdjsonCommand {
            command: "message".to_string(),
            session_id: Some(session_id),
            instructions: None,
            prompt: Some(
                "Summarize this content in one sentence (max 100 words) suitable for display in a list view."
                    .to_string(),
            ),
            content: Some(content.to_string()),
            output_format: Some("text".to_string()),
        };

        let response = self.send_command(cmd).await?;

        if !response.ok {
            return Err(response
                .error
                .unwrap_or_else(|| "Summarization failed".to_string()));
        }

        let summary: String = serde_json::from_value(
            response
                .result
                .unwrap_or(serde_json::Value::String(String::new())),
        )
        .map_err(|e| format!("Failed to parse summary: {}", e))?;

        if summary.is_empty() {
            return Err("Model returned empty summary".to_string());
        }

        Ok(summary)
    }
}

#[async_trait]
impl IntelProvider for IntelligenceKitProvider {
    async fn check_availability(&self) -> AvailabilityResult {
        let state = self.state.lock().await;
        state.availability.clone()
    }

    async fn generate_tags(&self, content: &str) -> Result<Vec<String>, String> {
        let chunks = split_content(content, MAX_CONTENT_CHARS);

        if chunks.len() == 1 {
            return self.generate_tags_with_retry(content).await;
        }

        eprintln!(
            "IntelligenceKit: Content too large ({} chars), splitting into {} chunks for tag generation",
            content.len(),
            chunks.len()
        );

        let mut all_tags = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            // Open fresh session per chunk to avoid context accumulation
            if let Err(e) = self.open_session().await {
                eprintln!("IntelligenceKit: Failed to open session for chunk {}: {}", i + 1, e);
                continue;
            }
            match self.generate_tags_with_retry(chunk).await {
                Ok(tags) => {
                    eprintln!(
                        "IntelligenceKit: Chunk {}/{} produced {} tags",
                        i + 1,
                        chunks.len(),
                        tags.len()
                    );
                    all_tags.extend(tags);
                }
                Err(e) => {
                    eprintln!(
                        "IntelligenceKit: Chunk {}/{} failed: {}",
                        i + 1,
                        chunks.len(),
                        e
                    );
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
            return self.summarize_with_retry(content).await;
        }

        eprintln!(
            "IntelligenceKit: Content too large ({} chars), splitting into {} chunks for summarization",
            content.len(),
            chunks.len()
        );

        let mut chunk_summaries = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            // Open fresh session per chunk to avoid context accumulation
            if let Err(e) = self.open_session().await {
                eprintln!("IntelligenceKit: Failed to open session for chunk {}: {}", i + 1, e);
                continue;
            }
            match self.summarize_with_retry(chunk).await {
                Ok(summary) => {
                    eprintln!(
                        "IntelligenceKit: Chunk {}/{} summarized",
                        i + 1,
                        chunks.len()
                    );
                    chunk_summaries.push(summary);
                }
                Err(e) => {
                    eprintln!(
                        "IntelligenceKit: Chunk {}/{} failed: {}",
                        i + 1,
                        chunks.len(),
                        e
                    );
                }
            }
        }

        if chunk_summaries.is_empty() {
            return Err("No summaries generated from any content chunk".to_string());
        }

        if chunk_summaries.len() == 1 {
            return Ok(chunk_summaries.into_iter().next().unwrap());
        }

        // Combine chunk summaries into a final summary (fresh session)
        let _ = self.open_session().await;
        let combined = chunk_summaries.join("\n");
        match self.summarize_with_retry(&combined).await {
            Ok(final_summary) => Ok(final_summary),
            Err(_) => {
                // If combining fails, return the first chunk's summary
                Ok(chunk_summaries.into_iter().next().unwrap())
            }
        }
    }
}
