# Requirements Document

## Introduction

JarvisTranscribe is a real-time speech-to-text transcription module for the JARVIS desktop application. It serves as the "Transcribe" step in the JARVIS AI assistant pipeline (Listen → Transcribe → Augment → Display) for the AWS 10,000 AIdeas Competition. The module implements a three-engine hybrid pipeline: Silero VAD (voice activity detection) gates the pipeline to skip silence, Vosk provides instant partial transcriptions (<100ms latency), and Whisper provides accurate final transcriptions (1-2s latency). Audio flows from JarvisListen through a named pipe (FIFO) to an AudioRouter component that simultaneously writes to a recording file and streams to the transcription pipeline with zero delay. The module is designed with a provider-swappable architecture to enable future integration with cloud-based transcription services like AWS Transcribe Streaming.

## Glossary

- **Transcription_Provider**: A trait-based abstraction for speech-to-text engines (HybridProvider, AWS Transcribe, etc.)
- **Hybrid_Provider**: Composite provider combining Silero VAD, Vosk, and Whisper behind a single interface
- **Whisper_Provider**: Batch inference engine using whisper.cpp via whisper-rs for accurate final transcriptions
- **Vosk_Provider**: Streaming engine using Vosk for instant partial transcriptions
- **Silero_VAD**: Voice activity detection engine that gates the pipeline to skip silence
- **Transcription_Manager**: Orchestrates the transcription lifecycle (start, stop, event emission) without knowing engine details
- **Audio_Router**: Component that reads from a named pipe (FIFO) and routes audio to both recording file and transcription pipeline
- **Audio_Buffer**: Accumulates PCM bytes into fixed-duration windows for batch transcription (Whisper)
- **Transcription_Segment**: A piece of transcribed text with start/end timestamps and is_final flag
- **Named_Pipe**: A FIFO (first-in-first-out) special file that enables zero-delay streaming from sidecar to Rust
- **Audio_Window**: A fixed-duration chunk of audio (default 3 seconds) fed to Whisper for batch inference
- **Sliding_Window**: Overlapping audio windows (e.g., 3s window, 2.5s advance, 0.5s overlap)
- **GGML_Model**: Quantized machine learning model format used by whisper.cpp
- **ONNX_Model**: Open Neural Network Exchange format used by Silero VAD
- **s16le**: 16-bit signed integer audio format, little-endian byte order
- **Pre_Roll_Buffer**: Audio buffer (100ms) that captures word onsets VAD might miss
- **Post_Roll_Buffer**: Audio buffer (300ms) that captures word endings after VAD detects silence

## Requirements

### Requirement 1: Named Pipe Audio Routing

**User Story:** As a transcription system, I want to receive audio with zero delay via a named pipe, so that I can transcribe in real-time without polling overhead or disk I/O latency.

#### Acceptance Criteria

1. WHEN recording starts, THE Audio_Router SHALL create a named pipe (FIFO) at a temporary path using nix::unistd::mkfifo
2. WHEN the FIFO is created, THE Audio_Router SHALL spawn a tokio task that opens the FIFO for reading (this operation blocks until a writer connects)
3. WHEN the reader task is spawned, THE System SHALL then spawn the JarvisListen sidecar with --output pointing to the FIFO path (sidecar opens for writing, unblocking the reader)
4. WHEN the JarvisListen sidecar opens the FIFO for writing, THE sidecar SHALL write PCM chunks to the FIFO as if it were a regular file
5. WHEN the FIFO reader receives PCM chunks, THE Audio_Router SHALL write each chunk to the actual recording file for playback
6. WHEN the FIFO reader receives PCM chunks, THE Audio_Router SHALL send each chunk via tokio::sync::mpsc channel to the Transcription_Manager
7. WHEN the sidecar closes the FIFO (EOF), THE Audio_Router SHALL signal completion to the Transcription_Manager
8. WHEN the Audio_Router is dropped, THE System SHALL unlink (delete) the FIFO file
9. WHEN opening the FIFO on macOS, THE Audio_Router SHALL use read-only mode (not read_write, which is Linux-only)
10. THE System SHALL follow the startup sequence: create FIFO → spawn reader task (blocks) → spawn sidecar (unblocks reader) to avoid race conditions
11. WHEN the mpsc channel or AudioRouter falls behind in processing, THE System SHALL rely on FIFO kernel backpressure to throttle the sidecar write operations (natural flow control, no data loss)

### Requirement 2: Audio Windowing for Batch Transcription

**User Story:** As a transcription system, I want to accumulate audio into fixed-duration windows for Whisper batch inference, so that I can balance latency and accuracy.

#### Acceptance Criteria

1. THE Audio_Buffer SHALL accumulate incoming PCM bytes until reaching the configured window duration (default 3 seconds)
2. WHEN the buffer reaches the window duration, THE Audio_Buffer SHALL extract a window and advance the read position by the overlap amount (default 2.5 seconds)
3. WHEN the buffer contains less than the window duration, THE Audio_Buffer SHALL wait for more data before extracting
4. WHEN transcription is stopped and the buffer contains at least 1 second of audio, THE Audio_Buffer SHALL extract the remaining audio as a final window
5. WHEN transcription is stopped and the buffer contains less than 1 second of audio, THE Audio_Buffer SHALL discard the remaining audio
6. WHERE the window duration is configurable, THE System SHALL accept values between 2 and 30 seconds
7. WHERE the overlap duration is configurable, THE System SHALL ensure overlap is less than window duration
8. THE System SHALL use 0.5 second overlap by default to provide context continuity between windows

### Requirement 3: TranscriptionProvider Trait

**User Story:** As a developer, I want a provider-swappable transcription interface, so that I can replace the hybrid pipeline with AWS Transcribe Streaming or other services without changing the orchestration logic.

#### Acceptance Criteria

1. THE Transcription_Provider SHALL define a trait with initialize, transcribe, and name methods
2. THE Transcription_Provider trait SHALL be object-safe (usable as Box<dyn TranscriptionProvider>)
3. THE Transcription_Provider trait SHALL require Send + Sync bounds for thread safety
4. WHEN initialize is called, THE Transcription_Provider SHALL prepare resources (load models, establish connections)
5. WHEN transcribe is called with f32 audio samples, THE Transcription_Provider SHALL return a vector of Transcription_Segment structs
6. THE Transcription_Provider trait SHALL NOT expose engine-specific types (Whisper, Vosk, VAD) in its interface
7. THE Transcription_Segment struct SHALL contain text, start_ms, end_ms, and is_final fields
8. THE is_final field SHALL be false for partial transcriptions (Vosk) and true for final transcriptions (Whisper)

### Requirement 4: Voice Activity Detection (Silero VAD)

**User Story:** As a transcription system, I want to detect when speech is present, so that I can skip silence and prevent Whisper hallucinations while saving CPU.

#### Acceptance Criteria

1. THE Silero_VAD SHALL process audio in 512-sample chunks at 16kHz (approximately 32ms per chunk)
2. WHEN a chunk is processed, THE Silero_VAD SHALL complete processing in less than 2ms
3. WHEN speech is detected in a chunk, THE Silero_VAD SHALL return true to allow downstream processing
4. WHEN no speech is detected in a chunk, THE Silero_VAD SHALL return false to skip Vosk and Whisper processing
5. THE Silero_VAD SHALL maintain a 100ms pre-roll buffer to capture word onsets that might be missed
6. THE Silero_VAD SHALL maintain a 300ms post-roll buffer to capture word endings after silence is detected
7. WHEN the VAD model file does not exist, THE System SHALL skip VAD and process all audio (graceful degradation)
8. THE System SHALL use ~/.jarvis/models/silero_vad.onnx as the default model path
9. WHERE the JARVIS_VAD_MODEL environment variable is set, THE System SHALL use that path instead

### Requirement 5: Fast Partial Transcription (Vosk)

**User Story:** As a user, I want to see transcribed text appear instantly as I speak, so that I get immediate feedback that the system is working.

#### Acceptance Criteria

1. THE Vosk_Provider SHALL accept i16 audio samples at 16kHz mono
2. WHEN audio is fed to Vosk, THE Vosk_Provider SHALL return partial results within 100ms
3. WHEN a partial result is available, THE Vosk_Provider SHALL emit a Transcription_Segment with is_final=false
4. WHEN a complete utterance is detected, THE Vosk_Provider SHALL emit a Transcription_Segment with is_final=false (Whisper will provide the final)
5. THE System SHALL use ~/.jarvis/models/vosk-model-small-en-us-0.15/ as the default model path
6. WHERE the JARVIS_VOSK_MODEL environment variable is set, THE System SHALL use that path instead
7. WHEN the Vosk model file does not exist, THE System SHALL skip Vosk and use VAD + Whisper only (graceful degradation)
8. WHEN Vosk fails to initialize on macOS Apple Silicon, THE System SHALL log a warning and continue with VAD + Whisper only

### Requirement 6: Accurate Final Transcription (Whisper)

**User Story:** As a user, I want highly accurate final transcriptions, so that the text I save is correct even if the instant partials had errors.

#### Acceptance Criteria

1. THE Whisper_Provider SHALL use the whisper-rs crate to interface with whisper.cpp
2. WHEN initialize is called, THE Whisper_Provider SHALL load the GGML model from the configured path (default: ~/.jarvis/models/ggml-base.en.bin)
3. WHERE the JARVIS_WHISPER_MODEL environment variable is set, THE Whisper_Provider SHALL use that path instead of the default
4. WHEN the model file does not exist, THE Whisper_Provider SHALL return an error from initialize
5. WHEN transcribe is called with f32 audio samples, THE Whisper_Provider SHALL use Greedy sampling strategy with best_of=1
6. WHEN transcribe completes, THE Whisper_Provider SHALL extract segments with text and timestamps (converted from centiseconds to milliseconds)
7. WHEN transcribe is called, THE Whisper_Provider SHALL pass previous segment tokens as prompt for context continuity
8. THE Whisper_Provider SHALL enable Metal GPU acceleration on macOS via the "metal" feature flag
9. WHERE the JARVIS_WHISPER_THREADS environment variable is set, THE Whisper_Provider SHALL configure whisper.cpp to use that thread count
10. WHEN a Whisper segment is produced, THE Whisper_Provider SHALL emit it with is_final=true

### Requirement 7: Hybrid Provider Composition

**User Story:** As a system architect, I want the three engines (VAD, Vosk, Whisper) hidden behind a single provider interface, so that TranscriptionManager remains engine-agnostic.

#### Acceptance Criteria

1. THE Hybrid_Provider SHALL implement the Transcription_Provider trait
2. THE Hybrid_Provider SHALL own instances of Silero_VAD, Vosk_Provider (optional), and Whisper_Provider
3. WHEN Vosk_Provider is None (unavailable), THE Hybrid_Provider SHALL skip Vosk processing and produce only Whisper final segments
4. WHEN transcribe is called, THE Hybrid_Provider SHALL first check VAD for speech presence
5. WHEN VAD detects no speech, THE Hybrid_Provider SHALL return an empty vector and skip Vosk and Whisper
6. WHEN VAD detects speech, THE Hybrid_Provider SHALL feed audio to Vosk for instant partials (if available)
7. WHEN VAD detects speech, THE Hybrid_Provider SHALL buffer audio for Whisper batch inference
8. WHEN Whisper produces a segment, THE Hybrid_Provider SHALL return it with is_final=true
9. WHEN Vosk produces a partial, THE Hybrid_Provider SHALL return it with is_final=false
10. THE Hybrid_Provider SHALL return the name "hybrid-vad-vosk-whisper" from the name() method

### Requirement 8: Event Emission

**User Story:** As a frontend developer, I want to receive real-time transcription updates via Tauri events, so that I can display live transcript to the user with instant partials and accurate finals.

#### Acceptance Criteria

1. WHEN transcription starts for a recording, THE Transcription_Manager SHALL emit a "transcription-started" event
2. WHEN a transcription segment is produced, THE Transcription_Manager SHALL emit a "transcription-update" event with the segment (text, start_ms, end_ms, is_final)
3. WHEN transcription stops, THE Transcription_Manager SHALL emit a "transcription-stopped" event with the complete transcript
4. WHEN a transcription error occurs, THE Transcription_Manager SHALL emit a "transcription-error" event with an error message
5. WHEN an error occurs, THE Transcription_Manager SHALL continue running (non-fatal errors)
6. THE Transcription_Manager SHALL emit all events to the Tauri event system using app_handle.emit()
7. THE Transcription_Manager SHALL emit partial segments (is_final=false) from Vosk as they arrive
8. THE Transcription_Manager SHALL emit final segments (is_final=true) from Whisper as they arrive

### Requirement 9: Frontend Display with Partial Replacement

**User Story:** As a user, I want to see instant gray text appear as I speak, then watch it get replaced by accurate black text, so that I get immediate feedback with eventual accuracy.

#### Acceptance Criteria

1. WHEN a transcription-update event is received with is_final=false, THE Frontend SHALL display the segment in light gray text
2. WHEN a transcription-update event is received with is_final=true, THE Frontend SHALL find any partial segments with overlapping timestamps and replace them with the final segment in normal text
3. WHEN new segments are added, THE Frontend SHALL auto-scroll to the latest text
4. WHEN transcription is active, THE Frontend SHALL display a "Transcribing..." indicator
5. WHEN a transcription-error event is received, THE Frontend SHALL display an error indicator
6. WHEN a new recording starts, THE Frontend SHALL clear the previous transcript
7. THE Frontend SHALL determine timestamp overlap by checking if final segment's time range intersects with any partial segment's time range

### Requirement 10: Tauri Integration

**User Story:** As a system integrator, I want transcription to start automatically with recording, so that users don't need separate controls for transcription.

#### Acceptance Criteria

1. WHEN RecordingManager.start_recording creates the FIFO and spawns the AudioRouter, THE System SHALL pass the FIFO path to the sidecar via --output flag
2. WHEN RecordingManager.start_recording spawns the AudioRouter, THE System SHALL pass the mpsc receiver to TranscriptionManager.start
3. WHEN RecordingManager.stop_recording is called, THE System SHALL call TranscriptionManager.stop
4. THE System SHALL provide a get_transcript Tauri command that returns the accumulated transcript
5. THE System SHALL provide a get_transcription_status Tauri command that returns the current status (idle, active, error, disabled)
6. WHEN TranscriptionManager is added to Tauri state, THE System SHALL wrap it in tokio::sync::Mutex (not std::sync::Mutex)

### Requirement 11: Model Management

**User Story:** As a user, I want the system to use default model locations, so that I don't need to configure paths manually.

#### Acceptance Criteria

1. THE System SHALL use ~/.jarvis/models/ggml-base.en.bin as the default Whisper model path
2. THE System SHALL use ~/.jarvis/models/silero_vad.onnx as the default VAD model path
3. THE System SHALL use ~/.jarvis/models/vosk-model-small-en-us-0.15/ as the default Vosk model path
4. WHERE the JARVIS_WHISPER_MODEL environment variable is set, THE System SHALL use that path for Whisper
5. WHERE the JARVIS_VAD_MODEL environment variable is set, THE System SHALL use that path for VAD
6. WHERE the JARVIS_VOSK_MODEL environment variable is set, THE System SHALL use that path for Vosk
7. WHEN the Whisper model file does not exist at startup, THE System SHALL set transcription status to "disabled" and allow recording to proceed normally (Whisper is the core engine)
8. WHEN the Vosk model file does not exist at startup, THE System SHALL skip Vosk initialization and use VAD + Whisper only (graceful degradation)
9. WHEN the VAD model file does not exist at startup, THE System SHALL skip VAD initialization and use Whisper only (graceful degradation)
10. WHEN the Whisper model is missing, THE System SHALL NOT emit transcription events
11. WHEN the Whisper model is missing, THE System SHALL log a warning to stderr explaining that transcription is disabled

### Requirement 12: Performance

**User Story:** As a user, I want transcription to run in the background, so that it doesn't block recording or the UI.

#### Acceptance Criteria

1. THE Transcription_Manager SHALL spawn a tokio background task for the transcription loop
2. THE Transcription_Manager SHALL process Whisper windows sequentially (not in parallel) to limit CPU usage
3. THE Transcription_Manager SHALL process Vosk audio inline with chunk arrival (lightweight, no batching needed)
4. WHEN transcription is running, THE System SHALL NOT block the Tauri main thread
5. WHEN transcription is running, THE System SHALL NOT block the recording process
6. WHERE the JARVIS_WHISPER_THREADS environment variable is set, THE Whisper_Provider SHALL configure whisper.cpp to use that thread count (default: auto-detect)
7. THE Whisper_Provider SHALL enable Metal GPU acceleration on macOS for approximately 3x speedup
8. WHEN VAD detects silence, THE System SHALL skip Vosk and Whisper processing to save CPU

### Requirement 13: Error Handling

**User Story:** As a user, I want transcription errors to be logged clearly, so that I can diagnose issues without affecting my recording.

#### Acceptance Criteria

1. WHEN a transcription error occurs, THE System SHALL log the error to stderr with a descriptive message
2. WHEN a transcription error occurs, THE System SHALL emit a "transcription-error" event to the frontend
3. WHEN a transcription error occurs, THE System SHALL NOT stop the recording process
4. WHEN the Whisper model fails to load, THE System SHALL set overall transcription status to "disabled" and continue with recording only
5. WHEN the Vosk or VAD model fails to load, THE System SHALL skip that engine and continue with the remaining engines
6. WHEN FIFO reading fails transiently, THE System SHALL retry up to 3 times before emitting an error event
7. WHEN audio conversion fails, THE System SHALL skip that chunk and continue with the next chunk
8. WHEN Vosk fails on macOS Apple Silicon, THE System SHALL log a warning and continue with VAD + Whisper only
9. WHEN the AudioRouter crashes and closes the FIFO while the sidecar is writing, THE JarvisListen sidecar SHALL receive EPIPE error instead of SIGPIPE termination (sidecar already ignores SIGPIPE)

### Requirement 14: Audio Format

**User Story:** As a transcription system, I want to receive audio in the correct format, so that I can process it without additional conversion.

#### Acceptance Criteria

1. THE System SHALL expect PCM audio at 16kHz sample rate
2. THE System SHALL expect PCM audio in 16-bit signed integer format (s16le)
3. THE System SHALL expect PCM audio in mono (single channel)
4. WHEN converting s16le to f32 for Whisper, THE System SHALL use whisper_rs::convert_integer_to_float_audio
5. WHEN feeding audio to Vosk, THE System SHALL use i16 samples directly (no conversion needed)
6. WHEN feeding audio to Silero VAD, THE System SHALL convert i16 to f32 samples
7. THE System SHALL calculate bytes per second as: 16000 samples/sec × 2 bytes/sample = 32000 bytes/sec
8. THE System SHALL calculate bytes per window as: window_duration_seconds × 32000 bytes/sec
9. THE System SHALL calculate samples per VAD chunk as: 512 samples (approximately 32ms at 16kHz)

