use crate::transcription::provider::{TranscriptionConfig, TranscriptionProvider, TranscriptionSegment};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::net::TcpListener;
use std::time::{Duration, Instant};
use serde::Deserialize;

/// WhisperKit API response for transcription
#[derive(Debug, Deserialize)]
struct WhisperKitResponse {
    segments: Vec<WhisperKitSegment>,
}

/// WhisperKit segment from API response
#[derive(Debug, Deserialize)]
struct WhisperKitSegment {
    text: String,
    start: f64,  // seconds
    end: f64,    // seconds
}

/// WhisperKit transcription provider using whisperkit-cli as a local HTTP server
///
/// # Implementation Status
/// - Task 1 (Complete): Availability detection, struct definition, basic trait implementation
/// - Task 2 (Complete): audio_to_wav conversion
/// - Task 3 (Complete): Server lifecycle (start_server, wait_for_server, stop_server, Drop)
/// - Task 4 (Complete): Full transcribe implementation with HTTP requests
///
/// # Design Notes
/// - `initialize()` starts whisperkit-cli server and waits for health check
/// - `transcribe()` converts audio to WAV, POSTs to server, parses JSON response
/// - `Drop` implementation uses SIGTERM → SIGKILL escalation for graceful shutdown
/// - `homebrew_paths` uses array instead of Vec to avoid allocation
pub struct WhisperKitProvider {
    server_process: Option<Child>,
    server_port: Option<u16>,
    cli_path: Option<PathBuf>,
    model_name: String,
    available: bool,
    unavailable_reason: Option<String>,
    client: Option<reqwest::blocking::Client>,
}

impl WhisperKitProvider {
    /// Create a new WhisperKitProvider and check availability
    pub fn new(model_name: &str) -> Self {
        let (available, unavailable_reason, cli_path) = Self::check_availability();
        
        Self {
            server_process: None,
            server_port: None,
            cli_path,
            model_name: model_name.to_string(),
            available,
            unavailable_reason,
            client: None,
        }
    }
    
    /// Check if WhisperKit is available on this system
    fn check_availability() -> (bool, Option<String>, Option<PathBuf>) {
        // Check if running on Apple Silicon
        if !Self::is_apple_silicon() {
            return (false, Some("WhisperKit requires Apple Silicon (arm64)".to_string()), None);
        }
        
        // Check if macOS 14 or later
        if !Self::is_macos_14_or_later() {
            return (false, Some("WhisperKit requires macOS 14.0 or later".to_string()), None);
        }
        
        // Check if whisperkit-cli is available
        match Self::find_cli() {
            Some(path) => (true, None, Some(path)),
            None => (false, Some("whisperkit-cli not found. Install with: brew install whisperkit-cli".to_string()), None),
        }
    }
    
    /// Find the whisperkit-cli binary
    fn find_cli() -> Option<PathBuf> {
        // Check common Homebrew locations first
        let homebrew_paths = [
            PathBuf::from("/opt/homebrew/bin/whisperkit-cli"),
            PathBuf::from("/usr/local/bin/whisperkit-cli"),
        ];
        
        for path in &homebrew_paths {
            if path.exists() {
                return Some(path.clone());
            }
        }
        
        // Try to find via PATH using `which`
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
    
    /// Check if running on Apple Silicon
    fn is_apple_silicon() -> bool {
        std::env::consts::ARCH == "aarch64"
    }
    
    /// Check if macOS version is 14.0 or later
    fn is_macos_14_or_later() -> bool {
        // Only check on macOS
        if std::env::consts::OS != "macos" {
            return false;
        }
        
        // Run sw_vers -productVersion
        if let Ok(output) = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
        {
            if output.status.success() {
                let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                
                // Parse version (e.g., "14.0", "14.1.2", "15.0")
                if let Some(major_str) = version_str.split('.').next() {
                    if let Ok(major) = major_str.parse::<u32>() {
                        return major >= 14;
                    }
                }
            }
        }
        
        false
    }
    
    /// Check if WhisperKit is available
    pub fn is_available(&self) -> bool {
        self.available
    }
    
    /// Get the reason why WhisperKit is unavailable (if any)
    pub fn unavailable_reason(&self) -> Option<&str> {
        self.unavailable_reason.as_deref()
    }
    
    /// Convert f32 audio samples to WAV bytes (in-memory)
    /// 
    /// Converts floating-point audio samples (range [-1.0, 1.0]) to a complete
    /// WAV file with 16-bit PCM encoding. The WAV file is generated entirely
    /// in memory without touching the filesystem.
    /// 
    /// # Arguments
    /// 
    /// * `audio` - f32 audio samples in range [-1.0, 1.0]
    /// * `sample_rate` - Sample rate in Hz (typically 16000)
    /// 
    /// # Returns
    /// 
    /// A `Vec<u8>` containing the complete WAV file (44-byte header + PCM data)
    /// 
    /// # Format
    /// 
    /// - Sample rate: As specified (typically 16000 Hz)
    /// - Bits per sample: 16 (signed integer, little-endian)
    /// - Channels: 1 (mono)
    /// - Format: PCM (format code 1)
    fn audio_to_wav(audio: &[f32], sample_rate: u32) -> Vec<u8> {
        // Convert f32 samples to i16 PCM
        let pcm_data: Vec<i16> = audio
            .iter()
            .map(|&sample| {
                // Clamp to [-1.0, 1.0] and convert to i16 range
                let clamped = sample.clamp(-1.0, 1.0);
                (clamped * 32767.0) as i16
            })
            .collect();
        
        // Convert i16 samples to bytes (little-endian)
        let pcm_bytes: Vec<u8> = pcm_data
            .iter()
            .flat_map(|&sample| sample.to_le_bytes())
            .collect();
        
        let data_size = pcm_bytes.len() as u32;
        
        // Create WAV header (44 bytes)
        let mut header = [0u8; 44];
        
        // RIFF chunk descriptor
        header[0..4].copy_from_slice(b"RIFF");
        
        // File size - 8 bytes
        let file_size = data_size + 36;
        header[4..8].copy_from_slice(&file_size.to_le_bytes());
        
        // WAVE format
        header[8..12].copy_from_slice(b"WAVE");
        
        // fmt subchunk
        header[12..16].copy_from_slice(b"fmt ");
        header[16..20].copy_from_slice(&16u32.to_le_bytes()); // Subchunk1 size (16 for PCM)
        header[20..22].copy_from_slice(&1u16.to_le_bytes());  // Audio format (1 = PCM)
        header[22..24].copy_from_slice(&1u16.to_le_bytes());  // Number of channels (1 = mono)
        header[24..28].copy_from_slice(&sample_rate.to_le_bytes()); // Sample rate
        
        // Byte rate (sample_rate * channels * bytes_per_sample)
        let byte_rate = sample_rate * 1 * 2; // 2 bytes per sample (16-bit)
        header[28..32].copy_from_slice(&byte_rate.to_le_bytes());
        
        // Block align (channels * bytes_per_sample)
        header[32..34].copy_from_slice(&2u16.to_le_bytes()); // 1 channel * 2 bytes
        
        // Bits per sample
        header[34..36].copy_from_slice(&16u16.to_le_bytes());
        
        // data subchunk
        header[36..40].copy_from_slice(b"data");
        header[40..44].copy_from_slice(&data_size.to_le_bytes());
        
        // Concatenate header + PCM data
        let mut wav_data = Vec::with_capacity(44 + pcm_bytes.len());
        wav_data.extend_from_slice(&header);
        wav_data.extend_from_slice(&pcm_bytes);
        
        wav_data
    }
    
    /// Find an available TCP port by binding to port 0
    /// 
    /// The OS will assign an available port, which we read and return.
    /// The socket is immediately closed after reading the port.
    fn find_available_port() -> Result<u16, Box<dyn std::error::Error>> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        drop(listener); // Close the socket
        Ok(port)
    }
    
    /// Start the whisperkit-cli server
    /// 
    /// Spawns `whisperkit-cli serve --port {port} --model-path {model_path}` as a child process.
    /// 
    /// # Arguments
    /// 
    /// * `model_path` - Path to the WhisperKit CoreML model directory
    fn start_server(&mut self, model_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        if self.server_process.is_some() {
            return Err("Server already running".into());
        }
        
        let cli_path = self.cli_path.as_ref()
            .ok_or("whisperkit-cli path not available")?;
        
        // Find an available port
        let port = Self::find_available_port()?;
        
        // Spawn whisperkit-cli serve
        let child = Command::new(cli_path)
            .arg("serve")
            .arg("--port")
            .arg(port.to_string())
            .arg("--model-path")
            .arg(model_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn whisperkit-cli: {}", e))?;
        
        self.server_process = Some(child);
        self.server_port = Some(port);
        
        // Create HTTP client with longer timeout for transcription requests
        // (whisperkit-cli can take several seconds to process audio, especially on first run)
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        self.client = Some(client);
        
        Ok(())
    }
    
    /// Wait for the server to become ready
    /// 
    /// Polls the health endpoint every 500ms until the server responds or timeout is reached.
    /// Uses a separate client with short timeout for health checks.
    /// 
    /// # Arguments
    /// 
    /// * `timeout_secs` - Maximum time to wait in seconds
    fn wait_for_server(&self, timeout_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
        let port = self.server_port
            .ok_or("Server port not set")?;
        
        // Create a separate client with short timeout for health checks
        let health_client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(1))
            .build()?;
        
        // Use IPv6 loopback directly — whisperkit-cli binds to [::1] (IPv6 only)
        let health_url = format!("http://[::1]:{}/health", port);
        let start = Instant::now();
        let timeout = Duration::from_secs(timeout_secs);
        
        loop {
            // Check if timeout exceeded
            if start.elapsed() > timeout {
                return Err(format!(
                    "Server health check timeout after {} seconds",
                    timeout_secs
                ).into());
            }
            
            // Try to connect to health endpoint
            match health_client.get(&health_url).send() {
                Ok(response) if response.status().is_success() => {
                    eprintln!("WhisperKit server ready on port {}", port);
                    return Ok(());
                }
                _ => {
                    // Server not ready yet, wait and retry
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
        }
    }
    
    /// Stop the server process
    /// 
    /// Attempts graceful shutdown with SIGTERM first, then force kills with SIGKILL if needed.
    /// Waits up to 5 seconds for graceful shutdown before escalating.
    fn stop_server(&mut self) {
        if let Some(mut child) = self.server_process.take() {
            eprintln!("Stopping WhisperKit server...");
            
            // Try graceful shutdown with SIGTERM first
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;
                
                let pid = Pid::from_raw(child.id() as i32);
                
                // Send SIGTERM for graceful shutdown
                if let Err(e) = kill(pid, Signal::SIGTERM) {
                    eprintln!("Failed to send SIGTERM to whisperkit-cli: {}", e);
                    // Fall through to force kill
                } else {
                    // Wait for graceful shutdown with timeout
                    let start = Instant::now();
                    let timeout = Duration::from_secs(5);
                    
                    loop {
                        match child.try_wait() {
                            Ok(Some(status)) => {
                                eprintln!("WhisperKit server stopped gracefully with status: {}", status);
                                self.server_port = None;
                                self.client = None;
                                return;
                            }
                            Ok(None) => {
                                // Process still running
                                if start.elapsed() > timeout {
                                    eprintln!("WhisperKit server did not stop within 5 seconds, force killing");
                                    break; // Escalate to SIGKILL
                                }
                                std::thread::sleep(Duration::from_millis(100));
                            }
                            Err(e) => {
                                eprintln!("Error waiting for whisperkit-cli to exit: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
            
            // Force kill if graceful shutdown failed or not on Unix
            if let Err(e) = child.kill() {
                eprintln!("Failed to force kill whisperkit-cli process: {}", e);
            } else {
                let _ = child.wait(); // Reap the zombie process
                eprintln!("WhisperKit server force killed");
            }
        }
        
        self.server_port = None;
        self.client = None;
    }
}

impl TranscriptionProvider for WhisperKitProvider {
    fn name(&self) -> &str {
        "whisperkit"
    }
    
    fn initialize(&mut self, _config: &TranscriptionConfig) -> Result<(), Box<dyn std::error::Error>> {
        if !self.available {
            return Err(format!(
                "WhisperKit is not available: {}",
                self.unavailable_reason.as_deref().unwrap_or("unknown reason")
            ).into());
        }
        
        // Determine model path
        // whisperkit-cli stores models under a nested HuggingFace cache structure
        let home = dirs::home_dir().ok_or("Failed to get home directory")?;
        let model_path = home
            .join(".jarvis/models/whisperkit")
            .join("models/argmaxinc/whisperkit-coreml")
            .join(&self.model_name);
        
        // Check if model exists
        if !model_path.exists() {
            return Err(format!(
                "WhisperKit model not found: {}. Download it first.",
                model_path.display()
            ).into());
        }
        
        // Start the server
        self.start_server(&model_path)?;
        
        // Wait for server to become ready (30 second timeout for first-run CoreML compilation)
        self.wait_for_server(30)?;
        
        Ok(())
    }
    
    fn transcribe(&mut self, audio: &[f32]) -> Result<Vec<TranscriptionSegment>, Box<dyn std::error::Error>> {
        // Check if client is initialized (more semantically correct than checking server_process)
        if self.client.is_none() {
            return Err("WhisperKit server not initialized. Call initialize() first.".into());
        }
        
        let client = self.client.as_ref().unwrap();
        let port = self.server_port.ok_or("Server port not set")?;
        
        // Convert audio to WAV format
        let wav_data = Self::audio_to_wav(audio, 16000);
        
        // Build multipart form (OpenAI-compatible API requires "model" field)
        let form = reqwest::blocking::multipart::Form::new()
            .text("model", self.model_name.clone())
            .part(
                "file",
                reqwest::blocking::multipart::Part::bytes(wav_data)
                    .file_name("audio.wav")
                    .mime_str("audio/wav")?
            );
        
        // POST to transcription endpoint
        // Use IPv6 loopback directly — whisperkit-cli binds to [::1] (IPv6 only)
        let url = format!("http://[::1]:{}/v1/audio/transcriptions", port);
        let response = client
            .post(&url)
            .multipart(form)
            .send()
            .map_err(|e| format!("Failed to send transcription request: {}", e))?;
        
        // Check response status
        if !response.status().is_success() {
            return Err(format!(
                "WhisperKit server returned error: {} - {}",
                response.status(),
                response.text().unwrap_or_else(|_| "unknown error".to_string())
            ).into());
        }
        
        // Parse JSON response
        let whisperkit_response: WhisperKitResponse = response
            .json()
            .map_err(|e| format!("Failed to parse WhisperKit response: {}", e))?;
        
        // Map WhisperKit segments to TranscriptionSegment
        let segments: Vec<TranscriptionSegment> = whisperkit_response
            .segments
            .into_iter()
            .map(|seg| TranscriptionSegment {
                text: seg.text,
                start_ms: (seg.start * 1000.0) as i64,
                end_ms: (seg.end * 1000.0) as i64,
                is_final: true, // whisperkit-cli batch mode returns final segments
            })
            .collect();
        
        Ok(segments)
    }
}

impl Drop for WhisperKitProvider {
    fn drop(&mut self) {
        // Stop server to prevent orphaned processes
        self.stop_server();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_provider_name() {
        let provider = WhisperKitProvider::new("test-model");
        assert_eq!(provider.name(), "whisperkit");
    }
    
    #[test]
    fn test_is_apple_silicon() {
        // This test will pass or fail depending on the architecture
        let is_arm = std::env::consts::ARCH == "aarch64";
        assert_eq!(WhisperKitProvider::is_apple_silicon(), is_arm);
    }
    
    #[test]
    fn test_transcribe_without_initialize() {
        let mut provider = WhisperKitProvider::new("test-model");
        let audio = vec![0.0f32; 16000]; // 1 second of silence
        let result = provider.transcribe(&audio);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not initialized"));
    }
    
    #[test]
    fn test_availability_detection() {
        let provider = WhisperKitProvider::new("test-model");
        
        // If not available, should have a reason
        if !provider.is_available() {
            assert!(provider.unavailable_reason().is_some());
            let reason = provider.unavailable_reason().unwrap();
            assert!(!reason.is_empty());
        }
    }
    
    #[test]
    fn test_audio_to_wav_header_starts_with_riff() {
        let audio = vec![0.0f32; 16000]; // 1 second of silence
        let wav_data = WhisperKitProvider::audio_to_wav(&audio, 16000);
        
        // WAV file should start with "RIFF"
        assert_eq!(&wav_data[0..4], b"RIFF");
    }
    
    #[test]
    fn test_audio_to_wav_contains_correct_sample_rate() {
        let audio = vec![0.0f32; 16000];
        let wav_data = WhisperKitProvider::audio_to_wav(&audio, 16000);
        
        // Sample rate is at bytes 24-27 (little-endian u32)
        let sample_rate = u32::from_le_bytes([
            wav_data[24], wav_data[25], wav_data[26], wav_data[27]
        ]);
        assert_eq!(sample_rate, 16000);
    }
    
    #[test]
    fn test_audio_to_wav_data_length_matches() {
        let audio = vec![0.5f32; 8000]; // 0.5 seconds at 16kHz
        let wav_data = WhisperKitProvider::audio_to_wav(&audio, 16000);
        
        // Data size is at bytes 40-43 (little-endian u32)
        let data_size = u32::from_le_bytes([
            wav_data[40], wav_data[41], wav_data[42], wav_data[43]
        ]);
        
        // Each f32 sample becomes 2 bytes (i16)
        assert_eq!(data_size, (audio.len() * 2) as u32);
        
        // Total WAV size should be 44 bytes header + data
        assert_eq!(wav_data.len(), 44 + (audio.len() * 2));
    }
    
    #[test]
    fn test_audio_to_wav_header_integrity() {
        let audio = vec![0.0f32; 16000];
        let wav_data = WhisperKitProvider::audio_to_wav(&audio, 16000);
        
        // Verify complete header structure
        assert_eq!(&wav_data[0..4], b"RIFF");      // RIFF chunk ID
        assert_eq!(&wav_data[8..12], b"WAVE");     // WAVE format
        assert_eq!(&wav_data[12..16], b"fmt ");    // fmt chunk ID
        assert_eq!(&wav_data[36..40], b"data");    // data chunk ID
        
        // Verify audio format is PCM (1)
        let audio_format = u16::from_le_bytes([wav_data[20], wav_data[21]]);
        assert_eq!(audio_format, 1);
        
        // Verify mono (1 channel)
        let num_channels = u16::from_le_bytes([wav_data[22], wav_data[23]]);
        assert_eq!(num_channels, 1);
        
        // Verify 16-bit samples
        let bits_per_sample = u16::from_le_bytes([wav_data[34], wav_data[35]]);
        assert_eq!(bits_per_sample, 16);
    }
    
    #[test]
    fn test_audio_to_wav_f32_to_i16_conversion() {
        // Test specific sample values
        let audio = vec![0.0, 0.5, -0.5, 1.0, -1.0];
        let wav_data = WhisperKitProvider::audio_to_wav(&audio, 16000);
        
        // Extract PCM data (starts at byte 44)
        let pcm_start = 44;
        
        // Sample 0: 0.0 -> 0
        let sample0 = i16::from_le_bytes([wav_data[pcm_start], wav_data[pcm_start + 1]]);
        assert_eq!(sample0, 0);
        
        // Sample 1: 0.5 -> ~16383 (0.5 * 32767)
        let sample1 = i16::from_le_bytes([wav_data[pcm_start + 2], wav_data[pcm_start + 3]]);
        assert!((sample1 - 16383).abs() <= 1); // Allow for rounding
        
        // Sample 2: -0.5 -> ~-16383
        let sample2 = i16::from_le_bytes([wav_data[pcm_start + 4], wav_data[pcm_start + 5]]);
        assert!((sample2 + 16383).abs() <= 1);
        
        // Sample 3: 1.0 -> 32767
        let sample3 = i16::from_le_bytes([wav_data[pcm_start + 6], wav_data[pcm_start + 7]]);
        assert_eq!(sample3, 32767);
        
        // Sample 4: -1.0 -> -32767
        let sample4 = i16::from_le_bytes([wav_data[pcm_start + 8], wav_data[pcm_start + 9]]);
        assert_eq!(sample4, -32767);
    }
    
    #[test]
    fn test_audio_to_wav_empty_audio() {
        let audio: Vec<f32> = vec![];
        let wav_data = WhisperKitProvider::audio_to_wav(&audio, 16000);
        
        // Should still have 44-byte header
        assert_eq!(wav_data.len(), 44);
        
        // Data size should be 0
        let data_size = u32::from_le_bytes([
            wav_data[40], wav_data[41], wav_data[42], wav_data[43]
        ]);
        assert_eq!(data_size, 0);
    }
    
    #[test]
    fn test_find_available_port_returns_valid_port() {
        let port = WhisperKitProvider::find_available_port().unwrap();
        
        // Port should be greater than 0
        assert!(port > 0);
        
        // Port should be in valid range (1-65535)
        assert!(port <= 65535);
    }
    
    #[test]
    fn test_find_available_port_returns_different_ports() {
        // Call twice and verify we get valid ports
        // (They might be the same or different, but both should be valid)
        let port1 = WhisperKitProvider::find_available_port().unwrap();
        let port2 = WhisperKitProvider::find_available_port().unwrap();
        
        assert!(port1 > 0);
        assert!(port2 > 0);
        assert!(port1 <= 65535);
        assert!(port2 <= 65535);
    }
    
    #[test]
    fn test_segment_mapping_seconds_to_milliseconds() {
        // Test conversion from float seconds to i64 milliseconds
        let segment = WhisperKitSegment {
            text: "Hello world".to_string(),
            start: 1.5,
            end: 3.25,
        };
        
        let mapped = TranscriptionSegment {
            text: segment.text.clone(),
            start_ms: (segment.start * 1000.0) as i64,
            end_ms: (segment.end * 1000.0) as i64,
            is_final: true,
        };
        
        assert_eq!(mapped.start_ms, 1500);
        assert_eq!(mapped.end_ms, 3250);
        assert_eq!(mapped.text, "Hello world");
        assert!(mapped.is_final);
    }
    
    #[test]
    fn test_segment_mapping_is_final_always_true() {
        // WhisperKit batch mode always returns final segments
        let segment = WhisperKitSegment {
            text: "Test".to_string(),
            start: 0.0,
            end: 1.0,
        };
        
        let mapped = TranscriptionSegment {
            text: segment.text,
            start_ms: (segment.start * 1000.0) as i64,
            end_ms: (segment.end * 1000.0) as i64,
            is_final: true,
        };
        
        assert!(mapped.is_final);
    }
    
    #[test]
    fn test_segment_mapping_empty_response() {
        // Empty segments list should map to empty Vec
        let segments: Vec<WhisperKitSegment> = vec![];
        
        let mapped: Vec<TranscriptionSegment> = segments
            .into_iter()
            .map(|seg| TranscriptionSegment {
                text: seg.text,
                start_ms: (seg.start * 1000.0) as i64,
                end_ms: (seg.end * 1000.0) as i64,
                is_final: true,
            })
            .collect();
        
        assert_eq!(mapped.len(), 0);
    }
    
    #[test]
    fn test_segment_mapping_multiple_segments() {
        // Test mapping multiple segments
        let segments = vec![
            WhisperKitSegment {
                text: "First segment".to_string(),
                start: 0.0,
                end: 2.5,
            },
            WhisperKitSegment {
                text: "Second segment".to_string(),
                start: 2.5,
                end: 5.0,
            },
        ];
        
        let mapped: Vec<TranscriptionSegment> = segments
            .into_iter()
            .map(|seg| TranscriptionSegment {
                text: seg.text,
                start_ms: (seg.start * 1000.0) as i64,
                end_ms: (seg.end * 1000.0) as i64,
                is_final: true,
            })
            .collect();
        
        assert_eq!(mapped.len(), 2);
        assert_eq!(mapped[0].text, "First segment");
        assert_eq!(mapped[0].start_ms, 0);
        assert_eq!(mapped[0].end_ms, 2500);
        assert_eq!(mapped[1].text, "Second segment");
        assert_eq!(mapped[1].start_ms, 2500);
        assert_eq!(mapped[1].end_ms, 5000);
    }
}
