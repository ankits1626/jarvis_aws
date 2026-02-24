// IntelligenceKitProvider - manages IntelligenceKit sidecar process and NDJSON communication

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::sync::Arc;
use tauri::Manager;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use super::provider::{AvailabilityResult, IntelProvider};

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
    /// Create a new provider and spawn the sidecar using tokio::process::Command
    /// 
    /// # Binary Naming Convention
    /// 
    /// Tauri's externalBin requires binaries to be named with target-triple suffix:
    /// - macOS ARM64: `IntelligenceKit-aarch64-apple-darwin`
    /// - macOS Intel: `IntelligenceKit-x86_64-apple-darwin`
    /// - Linux: `IntelligenceKit-x86_64-unknown-linux-gnu`
    /// - Windows: `IntelligenceKit-x86_64-pc-windows-msvc`
    /// 
    /// In dev mode, place the binary in: `src-tauri/binaries/IntelligenceKit-<target-triple>`
    /// In production, Tauri bundles it automatically with the correct suffix.
    /// 
    /// Note: Tauri uses "apple" not "macos" in the target triple (std::env::consts::OS returns "macos").
    /// 
    /// This differs from tauri_plugin_shell::sidecar() which handles suffix resolution
    /// automatically, but we use tokio::process::Command directly for synchronous
    /// request-response communication.
    pub async fn new(app_handle: tauri::AppHandle) -> Result<Self, String> {
        // Resolve binary path with target-triple suffix (required by Tauri's externalBin)
        // Dev mode: src-tauri/binaries/IntelligenceKit-aarch64-apple-darwin
        // Production: Binary is bundled with the platform suffix
        
        // Tauri uses "apple" not "macos" in the target triple
        let os_part = if cfg!(target_os = "macos") {
            "apple"
        } else if cfg!(target_os = "linux") {
            "unknown-linux"
        } else if cfg!(target_os = "windows") {
            "pc-windows"
        } else {
            std::env::consts::OS
        };
        
        let target_triple = format!(
            "{}-{}-{}",
            std::env::consts::ARCH,
            os_part,
            if cfg!(target_os = "macos") {
                "darwin"
            } else if cfg!(target_os = "windows") {
                "msvc"
            } else {
                "gnu"
            }
        );
        
        let binary_name = format!("binaries/IntelligenceKit-{}", target_triple);
        
        let binary_path = app_handle
            .path()
            .resolve(&binary_name, tauri::path::BaseDirectory::Resource)
            .map_err(|e| format!("Failed to resolve IntelligenceKit binary path: {}", e))?;

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
        // Try with retry on session_not_found
        match self.generate_tags_internal(content).await {
            Err(e) if e.contains("session_not_found") => {
                // Session expired, re-open and retry once
                self.open_session().await?;
                self.generate_tags_internal(content).await
            }
            result => result,
        }
    }

    async fn summarize(&self, content: &str) -> Result<String, String> {
        // Try with retry on session_not_found
        match self.summarize_internal(content).await {
            Err(e) if e.contains("session_not_found") => {
                // Session expired, re-open and retry once
                self.open_session().await?;
                self.summarize_internal(content).await
            }
            result => result,
        }
    }
}
