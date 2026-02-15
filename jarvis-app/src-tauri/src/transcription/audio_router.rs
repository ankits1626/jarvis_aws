// AudioRouter - FIFO-based audio routing for zero-delay streaming
// Routes PCM chunks from JarvisListen sidecar to both recording file and transcription pipeline

use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use nix::sys::stat::Mode;

/// AudioRouter manages a named pipe (FIFO) for receiving PCM audio from the JarvisListen sidecar.
/// It routes each chunk to both the recording file and the transcription pipeline via mpsc channel.
pub struct AudioRouter {
    fifo_path: PathBuf,
    recording_file: PathBuf,
    tx: mpsc::Sender<Vec<u8>>,
}

impl AudioRouter {
    /// Create a new AudioRouter with a unique FIFO path.
    /// 
    /// This creates the FIFO special file on the filesystem but does not open it yet.
    /// The FIFO will be opened for reading when start_routing() is called.
    /// 
    /// # Arguments
    /// * `recording_file` - Path where PCM audio will be written for playback
    /// * `tx` - mpsc sender for routing audio chunks to transcription pipeline
    /// 
    /// # Returns
    /// * `Ok(AudioRouter)` - Successfully created FIFO
    /// * `Err(String)` - Failed to create FIFO (permission denied, etc.)
    pub fn new(
        recording_file: PathBuf,
        tx: mpsc::Sender<Vec<u8>>,
    ) -> Result<Self, String> {
        // Generate unique FIFO path in temp directory
        let session_id = uuid::Uuid::new_v4();
        let fifo_path = std::env::temp_dir().join(format!("jarvis_audio_{}.fifo", session_id));
        
        // Create FIFO special file with read/write permissions for owner only
        nix::unistd::mkfifo(&fifo_path, Mode::S_IRUSR | Mode::S_IWUSR)
            .map_err(|e| format!("Failed to create FIFO at {:?}: {}", fifo_path, e))?;
        
        eprintln!("AudioRouter: Created FIFO at {:?}", fifo_path);
        
        Ok(Self {
            fifo_path,
            recording_file,
            tx,
        })
    }
    
    /// Get the FIFO path to pass to the JarvisListen sidecar via --output flag.
    pub fn fifo_path(&self) -> &Path {
        &self.fifo_path
    }
    
    /// Start routing audio from FIFO to recording file and transcription pipeline.
    /// 
    /// This method:
    /// 1. Opens the FIFO for reading (blocks until sidecar connects as writer)
    /// 2. Reads 3200-byte chunks (100ms at 16kHz stereo s16le)
    /// 3. Writes each chunk to the recording file
    /// 4. Sends each chunk via mpsc to the transcription pipeline
    /// 5. Handles EOF when sidecar closes the FIFO
    /// 6. Retries transient read errors up to 3 times with 100ms delay
    /// 
    /// # Returns
    /// * `Ok(())` - Routing completed successfully (sidecar closed FIFO)
    /// * `Err(String)` - Fatal error occurred (FIFO open failed, persistent read errors, etc.)
    pub async fn start_routing(&self) -> Result<(), String> {
        eprintln!("AudioRouter: Opening FIFO for reading (will block until writer connects)...");
        
        // Move entire routing loop to spawn_blocking to avoid blocking tokio worker threads
        // This is correct because FIFO reads are synchronous blocking operations
        let fifo_path = self.fifo_path.clone();
        let recording_path = self.recording_file.clone();
        let tx = self.tx.clone();
        
        tokio::task::spawn_blocking(move || {
            use std::io::{Read, Write};
            
            // Open FIFO for reading (blocks until writer connects)
            let mut fifo_reader = std::fs::OpenOptions::new()
                .read(true)
                .open(&fifo_path)
                .map_err(|e| format!("Failed to open FIFO for reading: {}", e))?;
            
            eprintln!("AudioRouter: FIFO opened, writer connected. Starting audio routing...");
            
            // Open recording file for writing (synchronous)
            let mut recording_file = std::fs::File::create(&recording_path)
                .map_err(|e| format!("Failed to create recording file at {:?}: {}", recording_path, e))?;
            
            // Read chunks and route to both destinations
            let mut buffer = vec![0u8; 3200]; // 100ms at 16kHz mono s16le (16000 Hz × 1 channel × 2 bytes × 0.1s)
            let mut total_bytes = 0usize;
            let mut retry_count = 0;
            let mut channel_closed = false;
            const MAX_RETRIES: usize = 3;
            const RETRY_DELAY_MS: u64 = 100;

            loop {
                // Read from FIFO with retry logic for transient errors
                match fifo_reader.read(&mut buffer) {
                    Ok(0) => {
                        // EOF - sidecar closed FIFO
                        eprintln!("AudioRouter: EOF detected, sidecar closed FIFO. Total bytes routed: {}", total_bytes);
                        break;
                    }
                    Ok(n) => {
                        // Successfully read n bytes
                        retry_count = 0; // Reset retry counter on success
                        let chunk = &buffer[..n];
                        total_bytes += n;

                        // Route 1: Write to recording file (synchronous)
                        if let Err(e) = recording_file.write_all(chunk) {
                            // Non-fatal: log error but continue transcription
                            eprintln!("AudioRouter: Warning - Failed to write to recording file: {}. Transcription continues.", e);
                        }

                        // Route 2: Send to transcription pipeline via mpsc (blocking send)
                        // Skip if channel already closed (TranscriptionManager stopped)
                        if !channel_closed {
                            if let Err(_) = tx.blocking_send(chunk.to_vec()) {
                                eprintln!("AudioRouter: Transcription channel closed. Recording continues (file-only mode).");
                                channel_closed = true;
                            }
                        }
                    }
                    Err(e) => {
                        // Read error - retry up to MAX_RETRIES times
                        retry_count += 1;

                        if retry_count <= MAX_RETRIES {
                            eprintln!("AudioRouter: Transient read error (attempt {}/{}): {}. Retrying in {}ms...",
                                      retry_count, MAX_RETRIES, e, RETRY_DELAY_MS);
                            std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
                            continue;
                        } else {
                            // Persistent error after retries - fail
                            return Err(format!("Failed to read from FIFO after {} retries: {}", MAX_RETRIES, e));
                        }
                    }
                }
            }
            
            eprintln!("AudioRouter: Routing completed successfully");
            Ok(())
        })
        .await
        .map_err(|e| format!("Join error in routing task: {}", e))?
    }
}

impl Drop for AudioRouter {
    /// Clean up FIFO file when AudioRouter is dropped.
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_file(&self.fifo_path) {
            eprintln!("AudioRouter: Warning - Failed to remove FIFO file {:?}: {}", self.fifo_path, e);
        } else {
            eprintln!("AudioRouter: Cleaned up FIFO at {:?}", self.fifo_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    
    #[tokio::test]
    async fn test_audio_router_creation() {
        let (tx, _rx) = mpsc::channel(100);
        let recording_file = std::env::temp_dir().join("test_recording.pcm");
        
        let router = AudioRouter::new(recording_file, tx);
        assert!(router.is_ok());
        
        let router = router.unwrap();
        assert!(router.fifo_path().exists());
        
        // Drop router to clean up FIFO
        drop(router);
    }
    
    #[tokio::test]
    async fn test_fifo_cleanup_on_drop() {
        let (tx, _rx) = mpsc::channel(100);
        let recording_file = std::env::temp_dir().join("test_recording2.pcm");
        
        let router = AudioRouter::new(recording_file, tx).unwrap();
        let fifo_path = router.fifo_path().to_path_buf();
        
        assert!(fifo_path.exists());
        
        // Drop router
        drop(router);
        
        // Verify FIFO file is removed
        assert!(!fifo_path.exists());
    }
}
