# Design Document: JarvisTranscribe

## Overview

JarvisTranscribe is a real-time speech-to-text transcription module that implements a three-engine hybrid pipeline for the JARVIS desktop application. The design prioritizes instant user feedback (via Vosk partials) combined with high accuracy (via Whisper finals), while using voice activity detection (Silero VAD) to gate the pipeline and prevent hallucinations during silence.

The system operates as a zero-delay streaming pipeline:
1. **Audio Routing**: JarvisListen sidecar writes PCM to a named pipe (FIFO) → AudioRouter reads from FIFO and routes to both recording file and transcription pipeline
2. **Voice Activity Detection**: Silero VAD processes 512-sample chunks (~32ms) in <2ms to detect speech presence
3. **Fast Partials**: Vosk provides instant transcription (<100ms latency) displayed as gray text
4. **Accurate Finals**: Whisper provides high-accuracy transcription (1-2s latency) displayed as black text, replacing gray partials
5. **Event Emission**: TranscriptionManager emits Tauri events for frontend display

This design follows a provider-swappable architecture where the entire hybrid pipeline is wrapped in a HybridProvider implementing the TranscriptionProvider trait, enabling future replacement with cloud services like AWS Transcribe Streaming without changing the orchestration logic.

## Architecture

### Component Diagram

```
JarvisListen Sidecar (Swift)
    │
    │ writes PCM to named pipe (FIFO) via --output flag
    │ (sidecar doesn't know it's a FIFO — treats it as a regular file)
    ▼
┌──────────────────────────────────────────────────────────┐
│  AudioRouter (Rust)                                      │
│  • Creates FIFO with mkfifo()                            │
│  • Opens FIFO for reading (blocks until writer connects) │
│  • Reads 3200-byte chunks every 100ms                    │
└──────┬───────────────────────────────────────────┬───────┘
       │                                           │
       │ writes to disk                            │ sends via mpsc
       ▼                                           ▼
┌─────────────────┐                    ┌──────────────────────┐
│ Recording File  │                    │ TranscriptionManager │
│ (recording.pcm) │                    │ (tokio background)   │
└─────────────────┘                    └──────────┬───────────┘
                                                  │
                                                  ▼
                                       ┌──────────────────────┐
                                       │  HybridProvider      │
                                       │  (trait object)      │
                                       └──────────┬───────────┘
                                                  │
                        ┌─────────────────────────┼─────────────────────────┐
                        │                         │                         │
                        ▼                         ▼                         ▼
                ┌───────────────┐       ┌─────────────────┐       ┌─────────────────┐
                │  Silero VAD   │       │  Vosk Provider  │       │ Whisper Provider│
                │  (gatekeeper) │       │  (fast partials)│       │ (accurate finals)│
                │               │       │                 │       │                 │
                │ 512 samples   │       │ <100ms latency  │       │ 1-2s latency    │
                │ <2ms process  │       │ is_final=false  │       │ is_final=true   │
                └───────┬───────┘       └────────┬────────┘       └────────┬────────┘
                        │                        │                         │
                        │ speech detected?       │                         │
                        │                        │                         │
                        ├── NO → skip (save CPU, prevent hallucinations)   │
                        │                        │                         │
                        └── YES ────────────────┬┴─────────────────────────┘
                                                │
                                                ▼
                                    ┌───────────────────────┐
                                    │ Tauri Event Emission  │
                                    │ • transcription-update│
                                    │ • transcription-error │
                                    └───────────┬───────────┘
                                                │
                                                ▼
                                    ┌───────────────────────┐
                                    │ React Frontend        │
                                    │ • Gray text (partials)│
                                    │ • Black text (finals) │
                                    │ • Auto-replacement    │
                                    └───────────────────────┘
```


### Component Descriptions

#### 1. AudioRouter
**Responsibilities:**
- Create named pipe (FIFO) at temporary path before sidecar spawn
- Open FIFO for reading (blocks until sidecar connects as writer)
- Read PCM chunks as they arrive from sidecar (3200 bytes per 100ms)
- Write each chunk to recording file for playback
- Send each chunk via tokio::sync::mpsc channel to TranscriptionManager
- Signal completion when sidecar closes FIFO (EOF)
- Clean up (unlink) FIFO file on drop

**Key APIs:**
- `nix::unistd::mkfifo()` - create FIFO special file
- `tokio::net::unix::pipe::OpenOptions::new().open_receiver()` - async FIFO reading
- `tokio::fs::File::write_all()` - write to recording file
- `tokio::sync::mpsc::Sender::send()` - send to transcription pipeline

**Startup Sequence (Critical):**
1. Create FIFO with `mkfifo(path, Mode::S_IRUSR | Mode::S_IWUSR)`
2. Spawn tokio task that opens FIFO for reading (blocks until writer)
3. Spawn JarvisListen sidecar with `--output <fifo_path>` (unblocks reader)
4. Begin reading chunks and routing

**Backpressure Handling:**
- FIFO kernel buffer: 16KB default (expandable to 64KB)
- At 32KB/s throughput, buffer provides ~500ms cushion
- If Rust reader falls behind, sidecar's `write()` blocks automatically
- Natural flow control prevents data loss

**SIGPIPE Safety:**
- JarvisListen already ignores SIGPIPE (main.swift line 85)
- If AudioRouter crashes, sidecar gets EPIPE instead of terminating
- Recording continues even if transcription fails

#### 2. TranscriptionManager
**Responsibilities:**
- Receive PCM chunks from mpsc channel (sent by AudioRouter)
- Own the TranscriptionProvider (Box<dyn TranscriptionProvider>)
- Spawn tokio background task for transcription loop
- Accumulate transcript in Vec<TranscriptionSegment>
- Emit Tauri events for each segment
- Handle stop signal and drain remaining audio
- Manage graceful shutdown

**Key APIs:**
- `tokio::sync::Mutex` - async mutex for provider access
- `tokio::sync::watch` - stop signal channel
- `tokio::sync::mpsc::Receiver` - receive audio chunks
- `tauri::AppHandle::emit()` - emit events to frontend

**State:**
- `provider: Arc<TokioMutex<Box<dyn TranscriptionProvider>>>`
- `transcript: Arc<TokioMutex<Vec<TranscriptionSegment>>>`
- `stop_signal: watch::Sender<bool>`
- `app_handle: AppHandle`

**Engine-Agnostic Design:**
- TranscriptionManager does NOT know about VAD, Vosk, or Whisper
- It only calls `provider.transcribe(audio)` and emits events
- Provider swapping requires zero changes to TranscriptionManager

#### 3. TranscriptionProvider Trait
**Responsibilities:**
- Define interface for speech-to-text engines
- Enable provider swapping (HybridProvider, AwsTranscribeProvider, etc.)
- Ensure object safety for trait objects
- Enforce thread safety with Send + Sync bounds

**Trait Definition:**
```rust
pub trait TranscriptionProvider: Send + Sync {
    fn name(&self) -> &str;
    fn initialize(&mut self, config: &TranscriptionConfig) -> Result<(), Box<dyn Error>>;
    fn transcribe(&mut self, audio: &[f32]) -> Result<Vec<TranscriptionSegment>, Box<dyn Error>>;
}
```

**Design Constraints:**
- Must be object-safe (no generic methods, no Self return types)
- No engine-specific types in interface (no WhisperContext, VoskRecognizer, etc.)
- `transcribe()` takes f32 audio (16kHz mono) - universal format
- Returns `Vec<TranscriptionSegment>` with `is_final` flag for partial/final distinction

#### 4. HybridProvider
**Responsibilities:**
- Implement TranscriptionProvider trait
- Own and orchestrate three engines: Silero VAD, Vosk, Whisper
- Gate pipeline with VAD to skip silence
- Provide instant partials via Vosk (if available)
- Provide accurate finals via Whisper
- Handle graceful degradation when engines are unavailable

**Composition:**
```rust
pub struct HybridProvider {
    vad: Option<SileroVad>,           // gatekeeper — always try to load
    vosk: Option<VoskRecognizer>,     // instant partials — optional
    whisper: WhisperProvider,         // accurate finals — required
    audio_buffer: AudioBuffer,        // accumulate for Whisper windows
}
```

**Processing Flow:**
1. Check VAD for speech presence (if available)
2. If no speech → return empty vec (skip Vosk and Whisper)
3. If speech → feed to Vosk for instant partial (if available)
4. Buffer audio for Whisper batch inference
5. When buffer reaches 3s → transcribe with Whisper
6. Return segments with appropriate `is_final` flags

#### 5. Silero VAD
**Responsibilities:**
- Detect speech presence in audio chunks
- Process 512-sample chunks (~32ms at 16kHz)
- Complete processing in <2ms per chunk
- Maintain pre-roll buffer (100ms) for word onsets
- Maintain post-roll buffer (300ms) for word endings
- Enable graceful degradation if model missing

**Key APIs:**
- `silero_vad_rs::VADIterator` - streaming VAD
- `ort::Session` - ONNX runtime for model inference

**Configuration:**
- Model path: `~/.jarvis/models/silero_vad.onnx` (1.8MB)
- Override: `JARVIS_VAD_MODEL` environment variable
- Chunk size: 512 samples (32ms)
- Pre-roll: 100ms (1600 samples)
- Post-roll: 300ms (4800 samples)

**Performance:**
- <2ms processing per 32ms chunk
- Essentially free compared to Whisper (1-2s per 3s window)
- 4x speed improvement by skipping silence


#### 6. VoskProvider
**Responsibilities:**
- Provide instant partial transcriptions (<100ms latency)
- Accept i16 audio samples at 16kHz mono
- Emit segments with `is_final=false`
- Enable graceful degradation if model missing or initialization fails

**Key APIs:**
- `vosk::Model` - load Vosk model
- `vosk::Recognizer` - streaming recognition
- `recognizer.accept_waveform()` - feed audio
- `recognizer.partial_result()` - get instant partials

**Configuration:**
- Model path: `~/.jarvis/models/vosk-model-small-en-us-0.15/` (40MB)
- Override: `JARVIS_VOSK_MODEL` environment variable
- Sample rate: 16kHz
- Format: i16 (no conversion needed from s16le)

**macOS Apple Silicon Caveat:**
- Vosk has iOS ARM support but desktop ARM64 is unverified
- If initialization fails → log warning, set vosk=None, continue with VAD + Whisper
- Graceful degradation: system still works, just no instant partials

**Performance:**
- <100ms latency for partial results
- 10-15% WER (word error rate) - acceptable for partials
- Lightweight enough to run inline with audio chunks

#### 7. WhisperProvider
**Responsibilities:**
- Provide accurate final transcriptions (2-5% WER)
- Process 3-second audio windows with 0.5s overlap
- Convert s16le to f32 for whisper.cpp
- Extract segments with timestamps (centiseconds → milliseconds)
- Pass previous tokens as prompt for context continuity
- Enable Metal GPU acceleration on macOS

**Key APIs:**
- `whisper_rs::WhisperContext` - load GGML model
- `whisper_rs::WhisperState` - inference state
- `state.full()` - batch inference
- `whisper_rs::convert_integer_to_float_audio()` - s16le → f32

**Configuration:**
- Model path: `~/.jarvis/models/ggml-base.en.bin` (142MB)
- Override: `JARVIS_WHISPER_MODEL` environment variable
- Sampling: Greedy with best_of=1
- Thread count: auto-detect or `JARVIS_WHISPER_THREADS` env var
- GPU: Metal acceleration enabled via `features = ["metal"]`

**Context Carryover:**
- Store previous segment tokens
- Pass as prompt to next inference
- Improves continuity across windows
- Prevents repeated phrases

**Performance:**
- 1-2s latency per 3s window
- 2-5% WER (best-in-class accuracy)
- Metal GPU: ~3x speedup on Apple Silicon
- Sequential processing to limit CPU usage

#### 8. AudioBuffer
**Responsibilities:**
- Accumulate PCM bytes into fixed-duration windows
- Extract windows with sliding overlap
- Convert s16le bytes to i16 samples to f32 samples
- Handle final window on stop (drain if >= 1 second)

**Configuration:**
- Window duration: 3 seconds (default), configurable 2-30s
- Overlap: 0.5 seconds (default), must be < window duration
- Bytes per window: 3s × 32000 bytes/s = 96000 bytes
- Advance per window: 2.5s × 32000 bytes/s = 80000 bytes

**Windowing Algorithm:**
```
Buffer: [--------------------]
Window 1:  [======]
           advance 2.5s
Window 2:      [======]
               advance 2.5s
Window 3:          [======]
```

**Conversion Pipeline:**
```
s16le bytes → i16 samples → f32 samples
[u8; 96000] → [i16; 48000] → [f32; 48000]
```

## Data Models

### TranscriptionSegment
Represents a piece of transcribed text with timing and finality information.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    pub text: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub is_final: bool,  // false = Vosk partial, true = Whisper final
}
```

### TranscriptionConfig
Holds runtime configuration for transcription.

```rust
#[derive(Debug, Clone)]
pub struct TranscriptionConfig {
    pub window_duration_secs: f32,  // default: 3.0, range: 2.0-30.0
    pub overlap_duration_secs: f32, // default: 0.5, must be < window_duration
    pub whisper_model_path: PathBuf,
    pub vad_model_path: PathBuf,
    pub vosk_model_path: PathBuf,
    pub whisper_threads: Option<usize>,
}

impl TranscriptionConfig {
    pub fn from_env() -> Self {
        let home = dirs::home_dir().expect("Failed to get home directory");
        let models_dir = home.join(".jarvis/models");
        
        Self {
            window_duration_secs: 3.0,
            overlap_duration_secs: 0.5,
            whisper_model_path: env::var("JARVIS_WHISPER_MODEL")
                .map(PathBuf::from)
                .unwrap_or_else(|_| models_dir.join("ggml-base.en.bin")),
            vad_model_path: env::var("JARVIS_VAD_MODEL")
                .map(PathBuf::from)
                .unwrap_or_else(|_| models_dir.join("silero_vad.onnx")),
            vosk_model_path: env::var("JARVIS_VOSK_MODEL")
                .map(PathBuf::from)
                .unwrap_or_else(|_| models_dir.join("vosk-model-small-en-us-0.15")),
            whisper_threads: env::var("JARVIS_WHISPER_THREADS")
                .ok()
                .and_then(|s| s.parse().ok()),
        }
    }
    
    pub fn validate(&self) -> Result<(), String> {
        if self.window_duration_secs < 2.0 || self.window_duration_secs > 30.0 {
            return Err(format!(
                "Window duration must be between 2 and 30 seconds, got {}",
                self.window_duration_secs
            ));
        }
        
        if self.overlap_duration_secs >= self.window_duration_secs {
            return Err(format!(
                "Overlap duration ({}) must be less than window duration ({})",
                self.overlap_duration_secs, self.window_duration_secs
            ));
        }
        
        Ok(())
    }
}
```

### TranscriptionStatus
Represents the current state of the transcription system.

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptionStatus {
    Idle,      // not transcribing
    Active,    // currently transcribing
    Error,     // error occurred
    Disabled,  // Whisper model missing, transcription unavailable
}
```

### Frontend TypeScript Types

These types mirror the Rust types for the React frontend.

```typescript
// src/state/types.ts additions

/**
 * Transcription segment matching Rust TranscriptionSegment struct
 */
export interface TranscriptionSegment {
  /** Transcribed text */
  text: string;
  
  /** Start time in milliseconds */
  start_ms: number;
  
  /** End time in milliseconds */
  end_ms: number;
  
  /** false = Vosk partial (gray text), true = Whisper final (normal text) */
  is_final: boolean;
}

/**
 * Transcription status matching Rust TranscriptionStatus enum
 */
export type TranscriptionStatus = "idle" | "active" | "error" | "disabled";

/**
 * Transcription state additions to AppState
 */
export interface AppState {
  // ... existing fields ...
  
  /** Current transcription status */
  transcriptionStatus: TranscriptionStatus;
  
  /** Accumulated transcript segments */
  transcript: TranscriptionSegment[];
  
  /** Current transcription error message (null if no error) */
  transcriptionError: string | null;
}

/**
 * Transcription action additions to AppAction union type
 */
export type AppAction =
  // ... existing actions ...
  | { type: "TRANSCRIPTION_STARTED" }
  | { type: "TRANSCRIPTION_UPDATE"; segment: TranscriptionSegment }
  | { type: "TRANSCRIPTION_STOPPED"; transcript: TranscriptionSegment[] }
  | { type: "TRANSCRIPTION_ERROR"; message: string }
  | { type: "CLEAR_TRANSCRIPT" };

/**
 * Event payload types for Tauri transcription events
 */

/** Payload for transcription-update event */
export interface TranscriptionUpdateEvent {
  segment: TranscriptionSegment;
}

/** Payload for transcription-stopped event */
export interface TranscriptionStoppedEvent {
  transcript: TranscriptionSegment[];
}

/** Payload for transcription-error event */
export interface TranscriptionErrorEvent {
  message: string;
}
```

### AudioBuffer Implementation
Manages audio accumulation and windowing.

```rust
pub struct AudioBuffer {
    buffer: Vec<u8>,
    window_size_bytes: usize,
    advance_size_bytes: usize,
    sample_rate: usize,
}

impl AudioBuffer {
    pub fn new(window_duration_secs: f32, overlap_duration_secs: f32, sample_rate: usize) -> Self {
        let bytes_per_second = sample_rate * 2; // 2 bytes per sample (s16le)
        let window_size_bytes = (window_duration_secs * bytes_per_second as f32) as usize;
        let advance_duration_secs = window_duration_secs - overlap_duration_secs;
        let advance_size_bytes = (advance_duration_secs * bytes_per_second as f32) as usize;
        
        Self {
            buffer: Vec::new(),
            window_size_bytes,
            advance_size_bytes,
            sample_rate,
        }
    }
    
    pub fn push(&mut self, chunk: &[u8]) {
        self.buffer.extend_from_slice(chunk);
    }
    
    pub fn extract_window(&mut self) -> Option<Vec<f32>> {
        if self.buffer.len() < self.window_size_bytes {
            return None;
        }
        
        // Extract window
        let window_bytes = &self.buffer[..self.window_size_bytes];
        
        // Convert s16le bytes to i16 samples
        let samples_i16: Vec<i16> = window_bytes
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        
        // Convert i16 to f32 using whisper_rs helper
        let samples_f32 = whisper_rs::convert_integer_to_float_audio(&samples_i16);
        
        // Advance buffer (remove processed bytes, keep overlap)
        self.buffer.drain(..self.advance_size_bytes);
        
        Some(samples_f32)
    }
    
    pub fn drain_remaining(&mut self, min_duration_secs: f32) -> Option<Vec<f32>> {
        let min_bytes = (min_duration_secs * (self.sample_rate * 2) as f32) as usize;
        
        if self.buffer.len() < min_bytes {
            self.buffer.clear();
            return None;
        }
        
        // Convert remaining bytes to f32
        let samples_i16: Vec<i16> = self.buffer
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        
        let samples_f32 = whisper_rs::convert_integer_to_float_audio(&samples_i16);
        
        self.buffer.clear();
        Some(samples_f32)
    }
}
```


## Components and Interfaces

### AudioRouter Implementation
Handles FIFO creation, reading, and routing.

```rust
pub struct AudioRouter {
    fifo_path: PathBuf,
    recording_file: PathBuf,
    tx: mpsc::Sender<Vec<u8>>,
}

impl AudioRouter {
    pub async fn new(
        recording_file: PathBuf,
        tx: mpsc::Sender<Vec<u8>>,
    ) -> Result<Self, String> {
        // Generate unique FIFO path
        let session_id = uuid::Uuid::new_v4();
        let fifo_path = std::env::temp_dir().join(format!("jarvis_audio_{}.fifo", session_id));
        
        // Create FIFO
        nix::unistd::mkfifo(&fifo_path, nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR)
            .map_err(|e| format!("Failed to create FIFO: {}", e))?;
        
        Ok(Self {
            fifo_path,
            recording_file,
            tx,
        })
    }
    
    pub fn fifo_path(&self) -> &Path {
        &self.fifo_path
    }
    
    pub async fn start_routing(&self) -> Result<(), String> {
        // Open FIFO for reading (blocks until writer connects)
        // Note: open_receiver() is synchronous and blocks, so use spawn_blocking
        let fifo_path = self.fifo_path.clone();
        let mut fifo_reader = tokio::task::spawn_blocking(move || {
            tokio::net::unix::pipe::OpenOptions::new()
                .open_receiver(&fifo_path)
        })
        .await
        .map_err(|e| format!("Join error: {}", e))?
        .map_err(|e| format!("Failed to open FIFO: {}", e))?;
        
        // Open recording file for writing
        let mut recording_file = tokio::fs::File::create(&self.recording_file)
            .await
            .map_err(|e| format!("Failed to create recording file: {}", e))?;
        
        // Read chunks and route
        let mut buffer = vec![0u8; 3200]; // 100ms at 16kHz mono s16le
        
        loop {
            match fifo_reader.read(&mut buffer).await {
                Ok(0) => {
                    // EOF - sidecar closed FIFO
                    break;
                }
                Ok(n) => {
                    let chunk = &buffer[..n];
                    
                    // Write to recording file
                    recording_file.write_all(chunk)
                        .await
                        .map_err(|e| format!("Failed to write to recording file: {}", e))?;
                    
                    // Send to transcription pipeline
                    self.tx.send(chunk.to_vec())
                        .await
                        .map_err(|e| format!("Failed to send chunk to transcription: {}", e))?;
                }
                Err(e) => {
                    return Err(format!("Failed to read from FIFO: {}", e));
                }
            }
        }
        
        Ok(())
    }
}

impl Drop for AudioRouter {
    fn drop(&mut self) {
        // Clean up FIFO file
        let _ = std::fs::remove_file(&self.fifo_path);
    }
}
```

### TranscriptionManager Implementation
Orchestrates the transcription lifecycle.

```rust
pub struct TranscriptionManager {
    provider: Arc<TokioMutex<Box<dyn TranscriptionProvider>>>,
    transcript: Arc<TokioMutex<Vec<TranscriptionSegment>>>,
    status: Arc<TokioMutex<TranscriptionStatus>>,
    stop_tx: Option<watch::Sender<bool>>,
    app_handle: AppHandle,
}

impl TranscriptionManager {
    pub fn new(
        provider: Box<dyn TranscriptionProvider>,
        app_handle: AppHandle,
    ) -> Self {
        Self {
            provider: Arc::new(TokioMutex::new(provider)),
            transcript: Arc::new(TokioMutex::new(Vec::new())),
            status: Arc::new(TokioMutex::new(TranscriptionStatus::Idle)),
            stop_tx: None,
            app_handle,
        }
    }
    
    pub async fn start(&mut self, mut rx: mpsc::Receiver<Vec<u8>>) -> Result<(), String> {
        // Set status to active
        *self.status.lock().await = TranscriptionStatus::Active;
        
        // Emit transcription-started event
        self.app_handle.emit("transcription-started", ())
            .map_err(|e| format!("Failed to emit transcription-started: {}", e))?;
        
        // Create stop signal channel
        let (stop_tx, mut stop_rx) = watch::channel(false);
        self.stop_tx = Some(stop_tx);
        
        // Clone Arc references for background task
        let provider = self.provider.clone();
        let transcript = self.transcript.clone();
        let status = self.status.clone();
        let app_handle = self.app_handle.clone();
        
        // Spawn background transcription task
        tokio::spawn(async move {
            let mut audio_buffer = AudioBuffer::new(3.0, 0.5, 16000);
            
            loop {
                tokio::select! {
                    // Check for stop signal
                    _ = stop_rx.changed() => {
                        if *stop_rx.borrow() {
                            // Drain remaining audio
                            if let Some(audio) = audio_buffer.drain_remaining(1.0) {
                                if let Ok(segments) = provider.lock().await.transcribe(&audio) {
                                    for segment in segments {
                                        transcript.lock().await.push(segment.clone());
                                        let _ = app_handle.emit("transcription-update", segment);
                                    }
                                }
                            }
                            break;
                        }
                    }
                    
                    // Receive audio chunks
                    Some(chunk) = rx.recv() => {
                        audio_buffer.push(&chunk);
                        
                        // Extract windows and transcribe
                        while let Some(audio) = audio_buffer.extract_window() {
                            match provider.lock().await.transcribe(&audio) {
                                Ok(segments) => {
                                    for segment in segments {
                                        transcript.lock().await.push(segment.clone());
                                        let _ = app_handle.emit("transcription-update", segment);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Transcription error: {}", e);
                                    let _ = app_handle.emit("transcription-error", json!({ "message": e.to_string() }));
                                }
                            }
                        }
                    }
                }
            }
            
            // Emit transcription-stopped event
            let final_transcript = transcript.lock().await.clone();
            let _ = app_handle.emit("transcription-stopped", json!({ "transcript": final_transcript }));
            
            // Set status to idle
            *status.lock().await = TranscriptionStatus::Idle;
        });
        
        Ok(())
    }
    
    pub async fn stop(&mut self) -> Result<(), String> {
        if let Some(stop_tx) = &self.stop_tx {
            stop_tx.send(true)
                .map_err(|e| format!("Failed to send stop signal: {}", e))?;
        }
        Ok(())
    }
    
    pub async fn get_transcript(&self) -> Vec<TranscriptionSegment> {
        self.transcript.lock().await.clone()
    }
    
    pub async fn get_status(&self) -> TranscriptionStatus {
        *self.status.lock().await
    }
}
```

## Algorithms

### FIFO Startup Sequence

```
1. RecordingManager.start_recording() is called
   
2. Create AudioRouter:
   a. Generate unique FIFO path: /tmp/jarvis_audio_<uuid>.fifo
   b. Call mkfifo(path, 0600) to create FIFO special file
   c. Create mpsc channel for audio routing
   
3. Spawn AudioRouter task:
   a. Call tokio::net::unix::pipe::OpenOptions::new().open_receiver(fifo_path)
   b. This operation BLOCKS until a writer connects
   c. Task is now waiting for sidecar to connect
   
4. Spawn JarvisListen sidecar:
   a. Pass --output <fifo_path> to sidecar
   b. Sidecar opens FIFO for writing
   c. This UNBLOCKS the AudioRouter reader
   
5. AudioRouter begins reading:
   a. Read 3200-byte chunks as they arrive
   b. Write each chunk to recording file
   c. Send each chunk via mpsc to TranscriptionManager
   
6. On sidecar exit:
   a. Sidecar closes FIFO (writer side)
   b. AudioRouter receives EOF (read returns 0)
   c. AudioRouter exits loop and signals completion
   d. FIFO file is unlinked in Drop implementation
```

### Hybrid Provider Transcription Flow

```
Input: audio [f32] (16kHz mono, variable length)
Output: Vec<TranscriptionSegment>

1. Check if VAD is available:
   - If VAD is None → skip to step 3 (process all audio)
   - If VAD is Some → proceed to step 2
   
2. Run VAD on audio:
   - Split audio into 512-sample chunks
   - For each chunk:
     * Call vad.contains_speech(chunk)
     * If any chunk contains speech → proceed to step 3
     * If all chunks are silence → return empty vec (skip Vosk and Whisper)
   
3. Process with Vosk (if available):
   - If Vosk is None → skip to step 4
   - Convert f32 audio to i16 samples
   - Call vosk.accept_waveform(&i16_samples)
   - Get partial result: vosk.partial_result()
   - If partial text available:
     * Create TranscriptionSegment with is_final=false
     * Add to results vector
   
4. Buffer audio for Whisper:
   - Add audio to internal AudioBuffer
   - If buffer reaches 3 seconds:
     * Extract window (3s with 0.5s overlap)
     * Proceed to step 5
   - If buffer < 3 seconds:
     * Wait for more audio (return current results)
   
5. Process with Whisper:
   - Call whisper.transcribe(&f32_audio)
   - Extract segments from Whisper result
   - For each segment:
     * Convert timestamps from centiseconds to milliseconds
     * Set is_final=true
     * Add to results vector
   
6. Return results vector:
   - May contain Vosk partials (is_final=false)
   - May contain Whisper finals (is_final=true)
   - May be empty if VAD detected silence
```

### Whisper Context Carryover

```
State: previous_tokens: Vec<i32> = Vec::new()

For each window:
  1. Create WhisperFullParams with:
     - strategy: SamplingStrategy::Greedy { best_of: 1 }
     - prompt_tokens: Some(&previous_tokens)
     - language: Some("en")
     - n_threads: config.whisper_threads or auto-detect
     
  2. Run inference:
     - state.full(params, &f32_audio)
     
  3. Extract segments:
     - For i in 0..state.full_n_segments():
       * text = state.full_get_segment_text(i)
       * t0 = state.full_get_segment_t0(i) * 10  // centiseconds → milliseconds
       * t1 = state.full_get_segment_t1(i) * 10
       * Create TranscriptionSegment { text, start_ms: t0, end_ms: t1, is_final: true }
       
  4. Update previous_tokens:
     - Extract tokens from last segment
     - Store for next window
     - Provides context continuity
```

### VAD Pre-Roll and Post-Roll Buffering

```
State:
  - pre_roll_buffer: VecDeque<[f32; 512]> with capacity 3 chunks (96ms)
  - post_roll_buffer: VecDeque<[f32; 512]> with capacity 10 chunks (320ms)
  - speech_active: bool = false

For each 512-sample chunk:
  1. Add chunk to pre_roll_buffer (keep last 3 chunks)
  
  2. Run VAD on chunk:
     - speech_detected = vad.contains_speech(chunk)
     
  3. Handle state transitions:
     - If !speech_active && speech_detected:
       * Transition to speech_active = true
       * Emit pre_roll_buffer contents (captures word onset)
       * Emit current chunk
       
     - If speech_active && speech_detected:
       * Emit current chunk
       * Clear post_roll_buffer
       
     - If speech_active && !speech_detected:
       * Add chunk to post_roll_buffer
       * If post_roll_buffer full (10 chunks = 320ms):
         - Transition to speech_active = false
         - Emit post_roll_buffer contents (captures word ending)
         - Clear post_roll_buffer
       
     - If !speech_active && !speech_detected:
       * Skip chunk (silence)
```


### Frontend Partial Replacement Algorithm

```
State: displayed_segments: Vec<TranscriptionSegment>

On transcription-update event with segment:
  1. If segment.is_final == false:
     - Append segment to displayed_segments
     - Render in light gray color
     - Auto-scroll to bottom
     
  2. If segment.is_final == true:
     - Find overlapping partials:
       * For each partial in displayed_segments where partial.is_final == false:
         - Check if time ranges overlap:
           overlap = (partial.start_ms < segment.end_ms) && (segment.start_ms < partial.end_ms)
         - If overlap: mark partial for removal
     
     - Remove overlapping partials from displayed_segments
     
     - Insert final segment at appropriate position (sorted by start_ms)
     
     - Render final segment in normal text color
     
     - Auto-scroll to bottom

Time Range Overlap Check:
  Two segments overlap if:
    (seg1.start_ms < seg2.end_ms) AND (seg2.start_ms < seg1.end_ms)
  
  Examples:
    seg1: [0, 1000], seg2: [500, 1500] → overlap (500 < 1500 AND 0 < 1000)
    seg1: [0, 1000], seg2: [1000, 2000] → no overlap (1000 < 2000 BUT 0 < 1000 is false at boundary)
    seg1: [0, 1000], seg2: [2000, 3000] → no overlap (2000 < 3000 BUT 0 < 2000 is false)
```

## Error Handling

### Error Categories

#### 1. FIFO Creation and Routing Errors

**FIFO Creation Failure:**
- Error: `mkfifo()` fails (permission denied, path exists, etc.)
- Handling: Return error from `AudioRouter::new()`, prevent recording start
- Message: "Failed to create FIFO: <error>"

**FIFO Open Failure:**
- Error: `open_receiver()` fails (file not found, permission denied)
- Handling: Return error from `start_routing()`, prevent recording start
- Message: "Failed to open FIFO: <error>"

**FIFO Read Failure:**
- Error: Transient I/O error during read
- Handling: Retry up to 3 times with 100ms delay, then fail
- Message: "Failed to read from FIFO: <error>"

**Recording File Write Failure:**
- Error: Disk full, permission denied, etc.
- Handling: Log error to stderr, emit transcription-error event, continue transcription (recording fails but transcription continues)
- Message: "Failed to write to recording file: <error>"

**Channel Send Failure:**
- Error: Receiver dropped (TranscriptionManager crashed)
- Handling: Log error to stderr, stop routing, recording file still written
- Message: "Failed to send chunk to transcription: <error>"

#### 2. Model Loading Errors

**Whisper Model Missing:**
- Error: Model file does not exist at configured path
- Handling: Set status to Disabled, log warning to stderr, allow recording to proceed
- Message: "Warning: Whisper model not found at <path>. Transcription disabled. Recording will continue."
- Behavior: No transcription events emitted, recording works normally

**Vosk Model Missing:**
- Error: Model file does not exist at configured path
- Handling: Set vosk=None in HybridProvider, log warning to stderr, continue with VAD + Whisper
- Message: "Warning: Vosk model not found at <path>. Instant partials disabled. Using VAD + Whisper only."
- Behavior: No partial segments emitted, only final segments from Whisper

**VAD Model Missing:**
- Error: Model file does not exist at configured path
- Handling: Set vad=None in HybridProvider, log warning to stderr, continue with Whisper only
- Message: "Warning: VAD model not found at <path>. Voice activity detection disabled. Processing all audio."
- Behavior: All audio processed (no silence skipping), higher CPU usage

**Vosk Initialization Failure (macOS ARM):**
- Error: Vosk fails to initialize on Apple Silicon
- Handling: Set vosk=None, log warning, continue with VAD + Whisper
- Message: "Warning: Vosk failed to initialize on macOS Apple Silicon. Using VAD + Whisper only."
- Behavior: Same as Vosk model missing

#### 3. Transcription Runtime Errors

**Audio Conversion Failure:**
- Error: s16le → f32 conversion fails
- Handling: Log error to stderr, skip that chunk, continue with next chunk
- Message: "Warning: Audio conversion failed for chunk: <error>. Skipping."

**VAD Processing Failure:**
- Error: VAD inference fails on a chunk
- Handling: Assume speech present (conservative), continue processing
- Message: "Warning: VAD processing failed: <error>. Assuming speech present."

**Vosk Processing Failure:**
- Error: Vosk accept_waveform fails
- Handling: Log error to stderr, skip Vosk for that chunk, continue with Whisper
- Message: "Warning: Vosk processing failed: <error>. Skipping partial for this chunk."

**Whisper Processing Failure:**
- Error: Whisper inference fails on a window
- Handling: Log error to stderr, emit transcription-error event, skip that window, continue with next
- Message: "Error: Whisper transcription failed: <error>"
- Event: `transcription-error` with message

#### 4. Integration Errors

**TranscriptionManager Start Failure:**
- Error: Provider initialization fails
- Handling: Return error to RecordingManager, prevent recording start if Whisper missing
- Message: "Failed to start transcription: <error>"

**Event Emission Failure:**
- Error: `app_handle.emit()` fails
- Handling: Log error to stderr, continue transcription (don't fail transcription if frontend disconnected)
- Message: "Warning: Failed to emit <event_name> event: <error>"

**Stop Signal Failure:**
- Error: `stop_tx.send()` fails (receiver dropped)
- Handling: Log error to stderr, transcription task already exited
- Message: "Warning: Failed to send stop signal: <error>"

### Error Handling Strategy

1. **Recording Never Blocked**: Transcription errors must never prevent or interrupt recording. If transcription fails, recording continues.

2. **Graceful Degradation**: Missing models trigger warnings and feature disablement, not application failure.
   - Whisper missing → Transcription disabled, recording works
   - Vosk missing → No partials, finals only
   - VAD missing → No silence skipping, all audio processed

3. **Non-Fatal Errors**: Runtime transcription errors emit events and log to stderr but don't crash the application.

4. **Retry Transient Errors**: I/O errors are retried up to 3 times before failing.

5. **All Errors to stderr**: Never write errors to stdout (reserved for PCM data in sidecar).

6. **Exit Codes**: Not applicable (transcription runs as background task, doesn't control process exit).

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property Reflection

After analyzing all acceptance criteria, I identified several areas where properties can be consolidated:

1. **is_final field semantics** (3.8, 5.3, 6.10) - Combined into Property 1 about segment finality
2. **Event emission timing** (8.2, 8.7, 8.8) - Combined into Property 2 about immediate emission
3. **Chunk routing** (1.5, 1.6) - Combined into Property 3 about dual routing
4. **VAD silence skipping** (7.5, 12.8) - Redundant, covered by Property 4
5. **Environment variable overrides** - Each tests different variable, kept separate as examples
6. **Timestamp conversion** (6.6) - Specific to Whisper, kept as Property 5

### Core Properties

#### Property 1: Segment Finality Semantics
*For any* transcription segment produced by the system, if the segment originates from Vosk, it SHALL have is_final=false, and if the segment originates from Whisper, it SHALL have is_final=true.

**Validates: Requirements 3.8, 5.3, 6.10**

**Testing approach:** Generate random audio and transcribe with HybridProvider. Verify all segments have correct is_final values based on their source. Mock Vosk and Whisper to return known segments and verify the is_final flag is set correctly.

#### Property 2: Immediate Segment Emission
*For any* transcription segment produced by the provider, the TranscriptionManager SHALL emit a transcription-update event immediately without batching or delay.

**Validates: Requirements 8.2, 8.7, 8.8**

**Testing approach:** Mock the provider to return segments at known times. Verify events are emitted immediately (within milliseconds) of segment production. Use tokio::time::Instant to measure emission latency.

#### Property 3: Dual Audio Routing
*For any* PCM chunk received from the FIFO, the AudioRouter SHALL both write the chunk to the recording file AND send the chunk via mpsc channel to the transcription pipeline.

**Validates: Requirements 1.5, 1.6**

**Testing approach:** Generate random PCM chunks and feed to AudioRouter. Verify each chunk appears in both the recording file and the mpsc receiver. Check that chunk order and content are preserved.

#### Property 4: VAD Silence Gating
*For any* audio chunk where VAD detects no speech, the HybridProvider SHALL return an empty vector and SHALL NOT invoke Vosk or Whisper processing.

**Validates: Requirements 7.5, 12.8**

**Testing approach:** Generate silent audio (all zeros or low amplitude). Mock VAD to return false. Verify HybridProvider returns empty vec and that Vosk/Whisper mocks are never called.

#### Property 5: Whisper Timestamp Conversion
*For any* Whisper segment with timestamps in centiseconds, the WhisperProvider SHALL convert them to milliseconds by multiplying by 10.

**Validates: Requirements 6.6**

**Testing approach:** Mock whisper-rs to return segments with known centisecond timestamps. Verify WhisperProvider output has timestamps multiplied by 10.

#### Property 6: Audio Buffer Window Extraction
*For any* AudioBuffer with accumulated bytes >= window_size_bytes, calling extract_window() SHALL return Some(Vec<f32>) with length equal to (window_size_bytes / 2) samples, and SHALL advance the buffer by advance_size_bytes.

**Validates: Requirements 2.1, 2.2**

**Testing approach:** Create AudioBuffer with various window/overlap configurations. Push random bytes until threshold reached. Call extract_window() and verify: (1) returned audio has correct length, (2) buffer advanced by correct amount, (3) overlap bytes retained.

#### Property 7: Audio Buffer Underflow Waiting
*For any* AudioBuffer with accumulated bytes < window_size_bytes, calling extract_window() SHALL return None without modifying the buffer.

**Validates: Requirements 2.3**

**Testing approach:** Create AudioBuffer and push bytes below threshold. Call extract_window() and verify None is returned and buffer length unchanged.

#### Property 8: Window Duration Validation
*For any* window duration value, the TranscriptionConfig validation SHALL accept values in the range [2.0, 30.0] seconds and SHALL reject all other values with an error.

**Validates: Requirements 2.6**

**Testing approach:** Generate random float values including valid (2.0-30.0) and invalid (<2.0, >30.0) ranges. Call validate() and verify correct acceptance/rejection.

#### Property 9: Overlap Duration Constraint
*For any* pair of (window_duration, overlap_duration) values, the TranscriptionConfig validation SHALL accept the pair if and only if overlap_duration < window_duration.

**Validates: Requirements 2.7**

**Testing approach:** Generate random pairs of durations. Call validate() and verify acceptance when overlap < window, rejection when overlap >= window.

#### Property 10: Timestamp Overlap Detection
*For any* two TranscriptionSegments, they overlap if and only if (seg1.start_ms < seg2.end_ms) AND (seg2.start_ms < seg1.end_ms).

**Validates: Requirements 9.2, 9.7**

**Testing approach:** Generate random pairs of segments with various time ranges (overlapping, adjacent, disjoint). Verify overlap detection matches the formula. Test edge cases: identical ranges, zero-length segments, boundary touching.


### Edge Case Properties

#### Property 11: Final Window Drain Threshold
*For any* AudioBuffer on stop, if the buffer contains >= 1 second of audio (>= 32000 bytes), drain_remaining(1.0) SHALL return Some(Vec<f32>), and if the buffer contains < 1 second, it SHALL return None and clear the buffer.

**Validates: Requirements 2.4, 2.5**

**Testing approach:** Create AudioBuffer and push various amounts of bytes (above and below 1-second threshold). Call drain_remaining(1.0) and verify correct Some/None return and buffer clearing.

#### Property 12: VAD Chunk Size
*For any* audio fed to Silero VAD, the audio SHALL be split into 512-sample chunks, and each chunk SHALL be processed independently.

**Validates: Requirements 4.1**

**Testing approach:** Generate audio of various lengths (512, 1024, 1536 samples). Mock VAD and verify it receives exactly 512-sample chunks. Verify remainder handling for non-multiple lengths.

#### Property 13: VAD Processing Latency
*For any* 512-sample audio chunk, Silero VAD processing SHALL complete in less than 2ms.

**Validates: Requirements 4.2**

**Testing approach:** Generate random 512-sample chunks. Measure VAD processing time using tokio::time::Instant. Run 100 iterations and verify all complete in <2ms. This is a performance property.

#### Property 14: Vosk Partial Latency
*For any* audio fed to Vosk, partial results SHALL be available within 100ms of feeding the audio.

**Validates: Requirements 5.2**

**Testing approach:** Generate random audio. Measure time from accept_waveform() call to partial_result() return. Run 100 iterations and verify all complete in <100ms. This is a performance property.

### Configuration Properties

#### Property 15: Default Window Duration
*When* TranscriptionConfig is created with default values, the window_duration_secs SHALL equal 3.0.

**Validates: Requirements 2.1**

**Testing approach:** Create TranscriptionConfig::default() and verify window_duration_secs == 3.0.

#### Property 16: Default Overlap Duration
*When* TranscriptionConfig is created with default values, the overlap_duration_secs SHALL equal 0.5.

**Validates: Requirements 2.8**

**Testing approach:** Create TranscriptionConfig::default() and verify overlap_duration_secs == 0.5.

#### Property 17: Environment Variable Model Path Override
*For any* environment variable (JARVIS_WHISPER_MODEL, JARVIS_VAD_MODEL, JARVIS_VOSK_MODEL), when the variable is set, TranscriptionConfig::from_env() SHALL use that path instead of the default.

**Validates: Requirements 4.9, 5.6, 6.3**

**Testing approach:** Set each environment variable to a test path. Call from_env() and verify the config uses the test path. Unset variable and verify default path is used.

#### Property 18: Whisper Thread Count Configuration
*When* the JARVIS_WHISPER_THREADS environment variable is set to a valid integer N, the WhisperProvider SHALL configure whisper.cpp to use N threads.

**Validates: Requirements 6.9, 12.6**

**Testing approach:** Set JARVIS_WHISPER_THREADS to various values. Initialize WhisperProvider and verify thread count configuration. This may require inspecting whisper-rs internal state or observing CPU usage patterns.

### Integration Properties

#### Property 19: FIFO Startup Sequence
*For any* recording start, the system SHALL follow the sequence: (1) create FIFO, (2) spawn reader task that opens FIFO (blocks), (3) spawn sidecar with FIFO path (unblocks reader), ensuring no race conditions.

**Validates: Requirements 1.1, 1.2, 1.3, 1.10**

**Testing approach:** Mock the FIFO operations and sidecar spawn. Verify the order of operations using a sequence counter or timeline. Verify reader task is spawned before sidecar spawn call.

#### Property 20: FIFO Cleanup on Drop
*For any* AudioRouter instance, when the instance is dropped, the FIFO file SHALL be unlinked (deleted) from the filesystem.

**Validates: Requirements 1.8**

**Testing approach:** Create AudioRouter, note the FIFO path, drop the AudioRouter, verify the file no longer exists using std::fs::metadata().

#### Property 21: Backpressure Flow Control
*When* the AudioRouter or mpsc channel falls behind in processing, the FIFO kernel buffer SHALL provide backpressure that throttles the sidecar's write() operations without data loss.

**Validates: Requirements 1.11**

**Testing approach:** Simulate slow processing by adding delays in the AudioRouter read loop. Verify sidecar write() calls block (measure write latency). Verify no chunks are lost by comparing sidecar write count to AudioRouter receive count.

#### Property 22: Recording Continues on Transcription Failure
*For any* transcription error (model missing, inference failure, etc.), the recording process SHALL continue without interruption.

**Validates: Requirements 13.3**

**Testing approach:** Trigger various transcription errors (missing models, mock inference failures). Verify recording file continues to grow and sidecar remains running. Verify recording file integrity.

#### Property 23: Graceful Degradation - Whisper Missing
*When* the Whisper model file does not exist, the system SHALL set transcription status to Disabled, log a warning to stderr, and allow recording to proceed normally without emitting transcription events.

**Validates: Requirements 11.7, 11.10, 11.11, 13.4**

**Testing approach:** Remove Whisper model file. Start recording. Verify: (1) status is Disabled, (2) warning logged to stderr, (3) recording file is created and grows, (4) no transcription events emitted.

#### Property 24: Graceful Degradation - Vosk Missing
*When* the Vosk model file does not exist, the system SHALL skip Vosk initialization, log a warning to stderr, and continue with VAD + Whisper only (no partial segments emitted).

**Validates: Requirements 11.8, 13.5**

**Testing approach:** Remove Vosk model file. Start transcription. Verify: (1) warning logged, (2) HybridProvider.vosk is None, (3) only final segments (is_final=true) are emitted, (4) no partial segments emitted.

#### Property 25: Graceful Degradation - VAD Missing
*When* the VAD model file does not exist, the system SHALL skip VAD initialization, log a warning to stderr, and continue with Whisper only (all audio processed, no silence skipping).

**Validates: Requirements 11.9, 13.5**

**Testing approach:** Remove VAD model file. Start transcription. Verify: (1) warning logged, (2) HybridProvider.vad is None, (3) all audio is processed (no empty results for silence), (4) transcription still produces segments.

### Event Emission Properties

#### Property 26: Transcription Started Event
*When* TranscriptionManager.start() is called, a "transcription-started" event SHALL be emitted before any transcription processing begins.

**Validates: Requirements 8.1**

**Testing approach:** Mock the event system. Call start(). Verify transcription-started event is emitted before any transcribe() calls to the provider.

#### Property 27: Transcription Stopped Event with Full Transcript
*When* TranscriptionManager.stop() completes, a "transcription-stopped" event SHALL be emitted with the complete accumulated transcript.

**Validates: Requirements 8.3**

**Testing approach:** Transcribe some audio to accumulate segments. Call stop(). Verify transcription-stopped event is emitted with all accumulated segments.

#### Property 28: Transcription Error Event on Failure
*For any* transcription error during processing, a "transcription-error" event SHALL be emitted with a descriptive error message.

**Validates: Requirements 8.4, 13.2**

**Testing approach:** Mock the provider to return errors. Verify transcription-error events are emitted with the error messages. Verify transcription continues after error (non-fatal).

#### Property 29: Non-Fatal Error Recovery
*For any* transcription error during processing, the TranscriptionManager SHALL continue running and processing subsequent audio chunks.

**Validates: Requirements 8.5, 13.3**

**Testing approach:** Mock the provider to return an error for one chunk, then success for subsequent chunks. Verify: (1) error event emitted, (2) subsequent chunks are still processed, (3) subsequent segments are emitted.

### Audio Format Properties

#### Property 30: Audio Format Expectations
*For any* audio input to the transcription pipeline, the system SHALL expect 16kHz sample rate, s16le format (16-bit signed little-endian), mono (single channel).

**Validates: Requirements 14.1, 14.2, 14.3**

**Testing approach:** Verify AudioBuffer and providers expect this format. Test with audio in this format and verify correct processing. Test with wrong format and verify errors or incorrect results.

#### Property 31: Bytes Per Second Calculation
*For any* audio at 16kHz s16le mono, the system SHALL calculate bytes per second as 32000 (16000 samples/sec × 2 bytes/sample).

**Validates: Requirements 14.7**

**Testing approach:** Verify AudioBuffer uses this calculation. Create 1 second of audio (32000 bytes) and verify it's treated as exactly 1 second.

#### Property 32: Bytes Per Window Calculation
*For any* window duration D seconds, the system SHALL calculate bytes per window as D × 32000.

**Validates: Requirements 14.8**

**Testing approach:** Create AudioBuffer with various window durations (2s, 3s, 5s). Verify window_size_bytes equals duration × 32000.

#### Property 33: VAD Samples Per Chunk
*For any* audio fed to VAD, the system SHALL use 512 samples per chunk, which equals 1024 bytes at 16kHz s16le mono.

**Validates: Requirements 14.9**

**Testing approach:** Verify VAD processing uses 512-sample chunks. Feed audio and verify chunk boundaries align with 512 samples.

## Testing Strategy

### Dual Testing Approach

This project requires both unit tests and property-based tests to ensure comprehensive correctness:

**Unit Tests** focus on:
- Specific examples and scenarios (e.g., FIFO startup sequence, model loading)
- Integration points between components (e.g., AudioRouter → TranscriptionManager)
- Edge cases that are difficult to generate randomly (e.g., Vosk ARM failure)
- Configuration validation for specific values
- Event emission verification

**Property-Based Tests** focus on:
- Universal properties that hold across all inputs (e.g., segment finality, timestamp overlap)
- Randomized input generation to find edge cases
- Invariants that must be maintained (e.g., buffer windowing, dual routing)
- Mathematical properties (e.g., timestamp conversion, bytes calculations)
- Performance properties (e.g., VAD latency, Vosk latency)

Together, these approaches provide comprehensive coverage: unit tests catch concrete bugs in specific scenarios, while property tests verify general correctness across the input space.

### Property-Based Testing Configuration

**Framework:** Use `proptest` crate for property-based testing in Rust.

**Test Configuration:**
- Minimum 100 iterations per property test (due to randomization)
- Each test must reference its design document property in a comment
- Tag format: `// Feature: jarvis-transcribe, Property N: <property title>`
- Use `proptest::prelude::*` for strategy generation

**Example:**
```rust
use proptest::prelude::*;

// Feature: jarvis-transcribe, Property 10: Timestamp Overlap Detection
proptest! {
    #[test]
    fn test_timestamp_overlap_detection(
        start1 in 0i64..10000,
        end1 in 0i64..10000,
        start2 in 0i64..10000,
        end2 in 0i64..10000,
    ) {
        // Ensure valid ranges
        let (start1, end1) = if start1 <= end1 { (start1, end1) } else { (end1, start1) };
        let (start2, end2) = if start2 <= end2 { (start2, end2) } else { (end2, start2) };
        
        let seg1 = TranscriptionSegment {
            text: "test1".to_string(),
            start_ms: start1,
            end_ms: end1,
            is_final: true,
        };
        
        let seg2 = TranscriptionSegment {
            text: "test2".to_string(),
            start_ms: start2,
            end_ms: end2,
            is_final: false,
        };
        
        let overlaps = (start1 < end2) && (start2 < end1);
        let detected_overlap = segments_overlap(&seg1, &seg2);
        
        prop_assert_eq!(overlaps, detected_overlap);
    }
}
```

### Unit Testing Strategy

**Framework:** Use Rust's built-in `#[cfg(test)]` and `#[test]` attributes.

**Test Organization:**
- `src/transcription/tests/audio_router_tests.rs` - FIFO creation, routing, cleanup
- `src/transcription/tests/manager_tests.rs` - Lifecycle, event emission, error handling
- `src/transcription/tests/provider_tests.rs` - Trait implementation, provider swapping
- `src/transcription/tests/hybrid_provider_tests.rs` - VAD gating, Vosk/Whisper coordination
- `src/transcription/tests/audio_buffer_tests.rs` - Windowing, conversion, draining
- `src/transcription/tests/integration_tests.rs` - End-to-end flows with mocks

**Key Test Cases:**
- FIFO startup sequence (create → open → spawn → unblock)
- Model loading (success, missing files, graceful degradation)
- Event emission (started, update, stopped, error)
- Error handling (transient failures, retries, non-fatal errors)
- Graceful degradation (Whisper/Vosk/VAD missing)
- Audio format validation (16kHz s16le mono)
- Configuration validation (window/overlap ranges)

### Testing Challenges and Mitigations

**Challenge 1: FIFO Behavior**
- Mitigation: Use temporary directories for test FIFOs
- Mock sidecar with a test writer process
- Test FIFO cleanup in Drop implementation
- Verify backpressure with slow reader simulation

**Challenge 2: Model Dependencies**
- Mitigation: Use small test models or mock the model loading
- Test graceful degradation with missing models
- Mock whisper-rs, vosk, and silero-vad-rs for unit tests
- Integration tests use real models (optional, slow)

**Challenge 3: Real-Time Audio Timing**
- Mitigation: Use deterministic time in tests (tokio::time::pause)
- Mock audio chunks with known timestamps
- Test windowing logic independently of actual audio timing
- Use synthetic audio buffers with known patterns

**Challenge 4: Event Emission**
- Mitigation: Mock Tauri AppHandle for testing
- Capture emitted events in a test channel
- Verify event order and content
- Test event emission failures (disconnected frontend)

**Challenge 5: Performance Properties**
- Mitigation: Run performance tests separately (marked with #[ignore])
- Use tokio::time::Instant for latency measurement
- Allow some variance (e.g., <2ms ± 0.5ms)
- Run on consistent hardware for reproducibility

### Test Coverage Goals

- **Line Coverage:** >80% for core logic (TranscriptionManager, HybridProvider, AudioBuffer)
- **Branch Coverage:** >70% for error handling paths
- **Property Coverage:** 100% of identified correctness properties implemented as tests
- **Integration Coverage:** Key end-to-end flows tested (FIFO → routing → transcription → events)

### Continuous Testing

- Run unit tests on every commit
- Run property tests (100 iterations) on every commit
- Run extended property tests (1000 iterations) nightly
- Run performance tests weekly (latency measurements)
- Monitor for flaky tests (especially timing-dependent tests)
- Track test execution time (property tests may be slower)

