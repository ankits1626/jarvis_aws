# Implementation Plan: JarvisTranscribe

## Overview

This implementation plan breaks down the JarvisTranscribe real-time transcription module into discrete coding tasks. The approach follows a bottom-up strategy: build foundational components first (data models, traits, audio buffer), then core engines (VAD, Vosk, Whisper), then composition (HybridProvider), then orchestration (AudioRouter, TranscriptionManager), and finally integration with the existing Tauri app.

Each task builds incrementally on previous work, with property-based tests and unit tests integrated as sub-tasks to validate correctness early. The implementation uses Rust with tokio for async runtime, whisper-rs for Whisper, vosk for Vosk, and silero-vad-rs for VAD.

## Tasks

- [x] 1. Set up Rust module structure and dependencies
  - Create src-tauri/src/transcription/ directory
  - Add dependencies to Cargo.toml: whisper-rs (with metal feature), silero-vad-rs, vosk, tokio, serde, nix, dirs, uuid
  - Create mod.rs with module exports
  - _Requirements: All_

- [x] 2. Implement core data models and traits
  - [x] 2.1 Create provider.rs with TranscriptionProvider trait
    - Define TranscriptionProvider trait (name, initialize, transcribe methods)
    - Define TranscriptionSegment struct (text, start_ms, end_ms, is_final)
    - Define TranscriptionConfig struct with from_env() and validate() methods
    - Define TranscriptionStatus enum (Idle, Active, Error, Disabled)
    - Ensure trait is object-safe (Send + Sync bounds)
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.7_
  
  - [ ]* 2.2 Write unit test for TranscriptionConfig validation
    - Test window duration range validation (2-30 seconds)
    - Test overlap < window constraint
    - Test environment variable overrides
    - Test default values: window=3.0s (Property 15), overlap=0.5s (Property 16)
    - _Requirements: 2.6, 2.7, 11.1-11.6_
  
  - [ ]* 2.3 Write property test for window duration validation
    - **Property 8: Window Duration Validation**
    - **Validates: Requirements 2.6**
    - Generate random float values, verify acceptance in [2.0, 30.0] range
    - Run 100 iterations
  
  - [ ]* 2.4 Write property test for overlap duration constraint
    - **Property 9: Overlap Duration Constraint**
    - **Validates: Requirements 2.7**
    - Generate random (window, overlap) pairs, verify overlap < window
    - Run 100 iterations
  
  - [ ]* 2.5 Write property test for timestamp overlap detection (Rust)
    - **Property 10: Timestamp Overlap Detection (Rust)**
    - **Validates: Requirements 9.2, 9.7**
    - Add segments_overlap(seg1, seg2) helper function to provider.rs
    - Generate random segment pairs, verify overlap detection
    - Test formula: (seg1.start_ms < seg2.end_ms) && (seg2.start_ms < seg1.end_ms)
    - Run 100 iterations

- [x] 3. Implement AudioBuffer for windowing
  - [x] 3.1 Create audio_buffer.rs with AudioBuffer struct
    - Implement new(window_duration, overlap_duration, sample_rate)
    - Implement push(&[u8]) to accumulate bytes
    - Implement extract_window() -> Option<Vec<f32>> with s16le → i16 → f32 conversion
    - Implement drain_remaining(min_duration) -> Option<Vec<f32>>
    - Calculate window_size_bytes and advance_size_bytes from durations
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 14.1, 14.2, 14.3_
  
  - [ ]* 3.2 Write property test for window extraction
    - **Property 6: Audio Buffer Window Extraction**
    - **Validates: Requirements 2.1, 2.2**
    - Generate random bytes, push until threshold, extract window
    - Verify returned length and buffer advance
    - Run 100 iterations
  
  - [ ]* 3.3 Write property test for underflow waiting
    - **Property 7: Audio Buffer Underflow Waiting**
    - **Validates: Requirements 2.3**
    - Push bytes below threshold, verify extract_window() returns None
    - Run 100 iterations
  
  - [ ]* 3.4 Write property test for final window drain
    - **Property 11: Final Window Drain Threshold**
    - **Validates: Requirements 2.4, 2.5**
    - Test drain_remaining() with buffers above/below 1-second threshold
    - Run 100 iterations
  
  - [ ]* 3.5 Write unit tests for audio format calculations
    - **Property 31: Bytes Per Second Calculation**
    - **Property 32: Bytes Per Window Calculation**
    - **Property 30: Audio Format Expectations**
    - **Validates: Requirements 14.7, 14.8, 14.1, 14.2, 14.3**
    - Test 1 second = 32000 bytes at 16kHz s16le mono
    - Test window_size_bytes = duration × 32000
    - Verify AudioBuffer expects 16kHz sample rate, s16le format, mono channel (Property 30)

- [x] 4. Checkpoint - Ensure foundational components work
  - Run all tests for data models and audio buffer
  - Verify no compilation errors
  - Ask user if questions arise

- [x] 5. Implement Silero VAD wrapper
  - [x] 5.1 Create vad.rs with SileroVad struct
    - Load ONNX model from configured path
    - Implement contains_speech(&[f32]) for 512-sample chunks
    - Maintain pre-roll buffer (100ms = 1600 samples)
    - Maintain post-roll buffer (300ms = 4800 samples)
    - Handle graceful degradation if model missing (return None)
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 4.9_
  
  - [ ]* 5.2 Write unit test for VAD chunk size
    - **Property 12: VAD Chunk Size**
    - **Validates: Requirements 4.1**
    - Verify VAD processes 512-sample chunks
    - Test with various audio lengths
  
  - [ ]* 5.3 Write property test for VAD processing latency
    - **Property 13: VAD Processing Latency**
    - **Validates: Requirements 4.2**
    - Measure processing time for 512-sample chunks
    - Verify <2ms latency (performance property)
    - Run 100 iterations
  
  - [ ]* 5.4 Write unit tests for VAD graceful degradation
    - Test behavior when model file missing
    - Verify None is returned and warning logged
    - _Requirements: 4.7_


- [x] 6. Implement Vosk provider
  - [x] 6.1 Create vosk_provider.rs with VoskProvider struct
    - Load Vosk model from configured path
    - Implement accept_waveform(&[i16]) method to feed audio to Vosk recognizer
    - Implement partial_result() method to get instant partial text
    - Return partial text with timestamps
    - Handle graceful degradation if model missing or init fails on macOS ARM
    - Note: VoskProvider does NOT implement TranscriptionProvider trait (only HybridProvider does)
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8_
  
  - [ ]* 6.2 Write property test for Vosk partial latency
    - **Property 14: Vosk Partial Latency**
    - **Validates: Requirements 5.2**
    - Measure time from accept_waveform() to partial_result()
    - Verify <100ms latency (performance property)
    - Run 100 iterations
  
  - [ ]* 6.3 Write property test for Vosk is_final flag
    - **Property 1: Segment Finality Semantics (Vosk part)**
    - **Validates: Requirements 5.3**
    - Verify all segments returned by VoskProvider have appropriate timestamps
    - Note: is_final flag is set by HybridProvider, not VoskProvider
    - Run 100 iterations with random audio
  
  - [ ]* 6.4 Write unit tests for Vosk graceful degradation
    - Test behavior when model file missing
    - Test behavior when init fails on macOS ARM
    - Verify warnings logged and None returned
    - _Requirements: 5.7, 5.8_

- [x] 7. Implement Whisper provider
  - [x] 7.1 Create whisper_provider.rs with WhisperProvider struct
    - Load GGML model from configured path with Metal GPU acceleration
    - Implement TranscriptionProvider trait
    - Convert s16le → f32 using whisper_rs::convert_integer_to_float_audio
    - Use Greedy sampling with best_of=1
    - Extract segments and convert timestamps (centiseconds → milliseconds)
    - Pass previous tokens as prompt for context continuity
    - Configure thread count from JARVIS_WHISPER_THREADS env var
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8, 6.9, 6.10_
  
  - [ ]* 7.2 Write property test for timestamp conversion
    - **Property 5: Whisper Timestamp Conversion**
    - **Validates: Requirements 6.6**
    - Mock whisper-rs to return centisecond timestamps
    - Verify output has timestamps × 10 (milliseconds)
    - Run 100 iterations
  
  - [ ]* 7.3 Write property test for Whisper is_final flag
    - **Property 1: Segment Finality Semantics (Whisper part)**
    - **Validates: Requirements 6.10**
    - Verify all Whisper segments have is_final=true
    - Run 100 iterations with random audio
  
  - [ ]* 7.4 Write unit tests for Whisper configuration
    - Test model loading from default path
    - Test environment variable override (JARVIS_WHISPER_MODEL)
    - Test thread count configuration (JARVIS_WHISPER_THREADS)
    - Test Metal GPU acceleration enabled
    - _Requirements: 6.2, 6.3, 6.8, 6.9_
  
  - [ ]* 7.5 Write unit test for context carryover
    - Test that previous tokens are passed to next inference
    - Verify continuity across windows
    - _Requirements: 6.7_

- [x] 8. Implement HybridProvider composition
  - [x] 8.1 Create hybrid_provider.rs with HybridProvider struct
    - Own instances of SileroVad (Option), VoskProvider (Option), WhisperProvider
    - Implement TranscriptionProvider trait
    - Implement transcribe() flow: VAD → Vosk → Whisper
    - Convert f32 audio to i16 samples when feeding to Vosk (inside HybridProvider)
    - Return empty vec when VAD detects silence
    - Handle graceful degradation when engines unavailable
    - Return name "hybrid-vad-vosk-whisper"
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7, 7.8, 7.9, 7.10_
  
  - [ ]* 8.2 Write property test for VAD silence gating
    - **Property 4: VAD Silence Gating**
    - **Validates: Requirements 7.5, 12.8**
    - Generate silent audio, mock VAD to return false
    - Verify empty vec returned and Vosk/Whisper not called
    - Run 100 iterations
  
  - [ ]* 8.3 Write unit test for Vosk optional handling
    - Test HybridProvider with Vosk=None
    - Verify only Whisper finals emitted, no partials
    - _Requirements: 7.3_
  
  - [ ]* 8.4 Write unit test for VAD optional handling
    - Test HybridProvider with VAD=None
    - Verify all audio processed (no silence skipping)
    - _Requirements: 11.9_
  
  - [ ]* 8.5 Write integration test for hybrid pipeline
    - Test full flow: VAD detects speech → Vosk partial → Whisper final
    - Verify segment ordering and is_final flags
    - _Requirements: 7.4, 7.5, 7.6, 7.7_

- [x] 9. Checkpoint - Ensure all engines work
  - Run all tests for VAD, Vosk, Whisper, and HybridProvider
  - Verify no compilation errors
  - Ask user if questions arise

- [x] 10. Implement AudioRouter for FIFO handling
  - [x] 10.1 Create audio_router.rs with AudioRouter struct
    - Create FIFO with nix::unistd::mkfifo()
    - Implement new() to create FIFO and return path
    - Implement start_routing() to open FIFO (use spawn_blocking), read chunks, route to file + mpsc
    - Implement retry logic: retry FIFO read up to 3 times with 100ms delay on transient errors
    - Implement Drop to unlink FIFO file
    - Use read-only mode on macOS (not read_write)
    - Note: SIGPIPE→EPIPE safety already handled by JarvisListen (ignores SIGPIPE), so sidecar survives if AudioRouter crashes
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 1.10, 1.11, 13.6, 13.9_
  
  - [ ]* 10.2 Write property test for dual audio routing
    - **Property 3: Dual Audio Routing**
    - **Validates: Requirements 1.5, 1.6**
    - Generate random PCM chunks, feed to AudioRouter
    - Verify each chunk in both recording file and mpsc receiver
    - Run 100 iterations
  
  - [ ]* 10.3 Write unit test for FIFO startup sequence
    - **Property 19: FIFO Startup Sequence**
    - **Validates: Requirements 1.1, 1.2, 1.3, 1.10**
    - Mock FIFO operations, verify order: create → open → spawn
    - Use sequence counter to verify ordering
  
  - [ ]* 10.4 Write unit test for FIFO cleanup
    - **Property 20: FIFO Cleanup on Drop**
    - **Validates: Requirements 1.8**
    - Create AudioRouter, drop it, verify FIFO file deleted
  
  - [ ]* 10.5 Write integration test for backpressure
    - **Property 21: Backpressure Flow Control**
    - **Validates: Requirements 1.11**
    - Simulate slow processing, verify sidecar write() blocks
    - Verify no data loss

- [x] 11. Implement TranscriptionManager orchestration
  - [x] 11.1 Create manager.rs with TranscriptionManager struct
    - Own provider as Arc<TokioMutex<Box<dyn TranscriptionProvider>>>
    - Implement new(provider, app_handle)
    - Implement start(rx: mpsc::Receiver<Vec<u8>>) to spawn background task
    - Implement stop() to signal task completion
    - Implement get_transcript() and get_status() methods
    - Use tokio::sync::watch for stop signal
    - Accumulate transcript in Arc<TokioMutex<Vec<TranscriptionSegment>>>
    - Handle audio conversion failures: skip chunk and continue with next chunk (log error to stderr)
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7, 8.8, 12.1, 12.2, 12.3, 12.4, 12.5, 13.7_
  
  - [ ]* 11.2 Write property test for immediate segment emission
    - **Property 2: Immediate Segment Emission**
    - **Validates: Requirements 8.2, 8.7, 8.8**
    - Mock provider to return segments at known times
    - Verify events emitted immediately (measure latency)
    - Run 100 iterations
  
  - [ ]* 11.3 Write unit test for transcription lifecycle
    - **Property 26: Transcription Started Event**
    - **Property 27: Transcription Stopped Event**
    - **Validates: Requirements 8.1, 8.3**
    - Test start() emits transcription-started
    - Test stop() emits transcription-stopped with full transcript
  
  - [ ]* 11.4 Write property test for error event emission
    - **Property 28: Transcription Error Event on Failure**
    - **Validates: Requirements 8.4, 13.2**
    - Mock provider to return errors
    - Verify transcription-error events emitted
    - Run 100 iterations
  
  - [ ]* 11.5 Write property test for non-fatal error recovery
    - **Property 29: Non-Fatal Error Recovery**
    - **Validates: Requirements 8.5, 13.3**
    - Mock provider to fail once then succeed
    - Verify transcription continues after error
    - Run 100 iterations
  
  - [ ]* 11.6 Write unit test for sequential Whisper processing
    - Verify only one Whisper window processed at a time
    - Test CPU usage limitation
    - _Requirements: 12.2_

- [x] 12. Checkpoint - Ensure orchestration works
  - Run all tests for AudioRouter and TranscriptionManager
  - Verify no compilation errors
  - Ask user if questions arise

- [x] 13. Integrate with RecordingManager
  - [x] 13.1 Modify recording.rs to create FIFO and AudioRouter
    - In start_recording(): create AudioRouter before spawning sidecar
    - Pass FIFO path to sidecar via --output flag
    - Spawn AudioRouter.start_routing() task
    - Create mpsc channel for audio routing
    - Pass mpsc receiver to TranscriptionManager.start()
    - _Requirements: 10.1, 10.2_
  
  - [x] 13.2 Modify recording.rs to stop transcription
    - In stop_recording(): call TranscriptionManager.stop()
    - Wait for transcription task to complete
    - _Requirements: 10.3_
  
  - [ ]* 13.3 Write integration test for recording + transcription
    - **Property 22: Recording Continues on Transcription Failure**
    - **Validates: Requirements 13.3**
    - Trigger transcription errors, verify recording continues
    - Verify recording file integrity

- [x] 14. Add TranscriptionManager to Tauri state
  - [x] 14.1 Modify lib.rs to initialize TranscriptionManager
    - Create TranscriptionConfig from environment
    - Initialize HybridProvider with graceful degradation
    - Wrap TranscriptionManager in tokio::sync::Mutex
    - Add to Tauri managed state
    - _Requirements: 10.5, 10.6, 11.7, 11.8, 11.9, 11.10, 11.11_
  
  - [ ]* 14.2 Write property test for Whisper missing graceful degradation
    - **Property 23: Graceful Degradation - Whisper Missing**
    - **Validates: Requirements 11.7, 11.10, 11.11, 13.4**
    - Remove Whisper model, start recording
    - Verify status=Disabled, warning logged, recording works, no events
  
  - [ ]* 14.3 Write property test for Vosk missing graceful degradation
    - **Property 24: Graceful Degradation - Vosk Missing**
    - **Validates: Requirements 11.8, 13.5**
    - Remove Vosk model, start transcription
    - Verify warning logged, only finals emitted, no partials
  
  - [ ]* 14.4 Write property test for VAD missing graceful degradation
    - **Property 25: Graceful Degradation - VAD Missing**
    - **Validates: Requirements 11.9, 13.5**
    - Remove VAD model, start transcription
    - Verify warning logged, all audio processed

- [x] 15. Implement Tauri commands
  - [x] 15.1 Add get_transcript command to commands.rs
    - Accept State<'_, TokioMutex<TranscriptionManager>>
    - Call manager.get_transcript().await
    - Return Vec<TranscriptionSegment>
    - _Requirements: 10.4_
  
  - [x] 15.2 Add get_transcription_status command to commands.rs
    - Accept State<'_, TokioMutex<TranscriptionManager>>
    - Call manager.get_status().await
    - Return TranscriptionStatus
    - _Requirements: 10.5_
  
  - [x] 15.3 Register commands in lib.rs invoke_handler
    - Add get_transcript and get_transcription_status to handler
    - _Requirements: 10.4, 10.5_
  
  - [ ]* 15.4 Write unit tests for Tauri commands
    - Test get_transcript returns accumulated segments
    - Test get_transcription_status returns current status
    - Mock TranscriptionManager for testing

- [ ] 16. Checkpoint - Ensure backend integration complete
  - Run all integration tests
  - Verify Tauri commands work
  - Test with actual models (optional, slow)
  - Ask user if questions arise

- [x] 17. Add frontend TypeScript types
  - [x] 17.1 Add TranscriptionSegment interface to src/state/types.ts
    - Define text, start_ms, end_ms, is_final fields
    - _Requirements: 3.7, 3.8_
  
  - [x] 17.2 Add TranscriptionStatus type to src/state/types.ts
    - Define "idle" | "active" | "error" | "disabled"
    - _Requirements: 10.5_
  
  - [x] 17.3 Add transcription state to AppState interface
    - Add transcriptionStatus, transcript, transcriptionError fields
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_
  
  - [x] 17.4 Add transcription actions to AppAction union type
    - Add TRANSCRIPTION_STARTED, TRANSCRIPTION_UPDATE, TRANSCRIPTION_STOPPED, TRANSCRIPTION_ERROR, CLEAR_TRANSCRIPT
    - _Requirements: 8.1, 8.2, 8.3, 8.4_
  
  - [x] 17.5 Add event payload interfaces
    - Define TranscriptionUpdateEvent, TranscriptionStoppedEvent, TranscriptionErrorEvent
    - _Requirements: 8.2, 8.3, 8.4_

- [x] 18. Implement frontend reducer actions
  - [x] 18.1 Add transcription action handlers to src/state/reducer.ts
    - Handle TRANSCRIPTION_STARTED: set status to "active"
    - Handle TRANSCRIPTION_UPDATE: append segment to transcript
    - Handle TRANSCRIPTION_STOPPED: set status to "idle", store final transcript
    - Handle TRANSCRIPTION_ERROR: set status to "error", store error message
    - Handle CLEAR_TRANSCRIPT: clear transcript array
    - _Requirements: 8.1, 8.2, 8.3, 8.4_
  
  - [ ]* 18.2 Write unit tests for reducer actions
    - Test each transcription action updates state correctly
    - Test state transitions (idle → active → idle)

- [x] 19. Implement TranscriptDisplay component
  - [x] 19.1 Create src/components/TranscriptDisplay.tsx
    - Display transcript segments in scrollable container
    - Render partial segments (is_final=false) in light gray
    - Render final segments (is_final=true) in normal text
    - Implement auto-scroll to latest text
    - Show "Transcribing..." indicator when status=active
    - Show error indicator when status=error
    - Clear transcript on new recording start
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6_
  
  - [x] 19.2 Implement partial → final replacement logic
    - When final segment arrives, find overlapping partials
    - Remove overlapping partials from display
    - Insert final segment at correct position
    - Use timestamp overlap formula: (seg1.start_ms < seg2.end_ms) && (seg2.start_ms < seg1.end_ms)
    - _Requirements: 9.2, 9.7_
  
  - [ ]* 19.3 Write TypeScript unit test for timestamp overlap detection
    - **Property 10: Timestamp Overlap Detection (TypeScript)**
    - **Validates: Requirements 9.2, 9.7**
    - Test overlap detection formula: (seg1.start_ms < seg2.end_ms) && (seg2.start_ms < seg1.end_ms)
    - Test with various segment pairs (overlapping, adjacent, disjoint)
    - Use Jest or Vitest for TypeScript testing

- [x] 20. Add transcription event listeners
  - [x] 20.1 Modify src/hooks/useRecording.ts to listen for transcription events
    - Add useTauriEvent listeners for transcription-started, transcription-update, transcription-stopped, transcription-error
    - Dispatch appropriate actions to reducer
    - _Requirements: 8.1, 8.2, 8.3, 8.4_
  
  - [x] 20.2 Clear transcript on new recording start
    - Dispatch CLEAR_TRANSCRIPT action when recording starts
    - _Requirements: 9.6_

- [x] 21. Integrate TranscriptDisplay into App.tsx
  - [x] 21.1 Add TranscriptDisplay component to main UI
    - Position below or beside recording controls
    - Pass transcript and status from state
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6_
  
  - [x] 21.2 Add CSS styling for transcript display
    - Style partial segments (gray text)
    - Style final segments (normal text)
    - Style transcribing indicator
    - Style error indicator
    - Ensure auto-scroll works smoothly

- [ ] 22. Final checkpoint - Ensure all tests pass
  - Run complete test suite (unit + property + integration)
  - Verify no compilation errors or warnings
  - Test manual execution with actual models (requires model downloads)
  - Test graceful degradation scenarios (missing models)
  - Test end-to-end flow: record → transcribe → display
  - Ask user if questions arise

## Notes

- Tasks marked with `*` are optional test tasks and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation at logical breakpoints
- Property tests validate universal correctness properties (minimum 100 iterations each)
- Unit tests validate specific examples, edge cases, and error conditions
- Integration tests validate end-to-end flows with mocked or real dependencies
- The implementation follows a bottom-up approach: foundations → engines → composition → orchestration → integration
- All tests use Rust's built-in test framework + proptest for property-based testing
- Property-based testing tag format: `// Feature: jarvis-transcribe, Property N: <property title>`
- Models must be downloaded separately to ~/.jarvis/models/ for full functionality
- Graceful degradation ensures recording works even if transcription models are missing
- All PCM data to recording file, all logs/errors to stderr
- Transcription runs in background tokio task, never blocks recording or UI

