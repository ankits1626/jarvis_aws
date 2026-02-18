# Implementation Plan: JARVIS WhisperKit Module

## Overview

This plan implements WhisperKit as a toggleable transcription engine alongside the existing whisper-rs (whisper.cpp). The integration uses whisperkit-cli as a sidecar HTTP server, matching the existing Tauri sidecar pattern. Implementation is split into 4 phases: backend provider, settings integration, model management, and frontend UI.

## Prerequisites

Before starting, verify:
- `brew install whisperkit-cli` works on the dev machine
- `whisperkit-cli --help` shows available commands
- `whisperkit-cli serve --help` shows server options

## Tasks

- [x] 1. Create WhisperKitProvider struct and availability detection
  - Create `jarvis-app/src-tauri/src/transcription/whisperkit_provider.rs`
  - Define `WhisperKitProvider` struct with fields: `server_process`, `server_port`, `cli_path`, `model_name`, `available`, `unavailable_reason`, `client`
  - Implement `WhisperKitProvider::new(model_name: &str)` that checks availability
  - Implement `find_cli()` — search for whisperkit-cli at: `/opt/homebrew/bin/whisperkit-cli`, `/usr/local/bin/whisperkit-cli`, then `which whisperkit-cli` via PATH
  - Implement `is_apple_silicon()` — check `std::env::consts::ARCH == "aarch64"`
  - Implement `is_macos_14_or_later()` — parse `sw_vers -productVersion` output
  - Implement `is_available()` and `unavailable_reason()` getters
  - Add `pub mod whisperkit_provider;` to `transcription/mod.rs`
  - _Requirements: 2.1, 2.2, 5.1, 5.2, 5.3_

- [x] 1.1 Write unit tests for availability detection
  - Test `is_available()` returns false when no CLI found
  - Test `is_apple_silicon()` returns correct value
  - Test provider `name()` returns `"whisperkit"`
  - Test `transcribe()` returns error when not initialized
  - _Requirements: 5.1, 5.2, 5.3, 5.4_

- [x] 2. Implement audio-to-WAV conversion
  - Implement `WhisperKitProvider::audio_to_wav(audio: &[f32], sample_rate: u32) -> Vec<u8>`
  - Write a valid WAV header (44 bytes): RIFF, fmt chunk (PCM 16-bit mono 16kHz), data chunk
  - Convert f32 samples ([-1.0, 1.0]) to i16 PCM bytes (s16le)
  - This is needed because the HTTP API expects a WAV file, not raw f32 samples
  - _Requirements: 3.2_

- [x] 2.1 Write unit test for audio_to_wav
  - Test WAV header starts with "RIFF"
  - Test WAV contains correct sample rate (16000)
  - Test WAV data section length matches audio samples
  - Test round-trip: f32 → WAV bytes → check header integrity

- [x] 3. Implement server lifecycle management
  - Implement `find_available_port()` — bind to port 0, read assigned port, close socket
  - Implement `start_server(model_path: &PathBuf)`:
    - Spawn `whisperkit-cli serve --port {port} --model-path {model_path}` as child process
    - Store `Child` in `self.server_process`
    - Store port in `self.server_port`
  - Implement `wait_for_server(timeout_secs: u64)`:
    - Poll `GET http://localhost:{port}/health` every 500ms
    - Return Ok when server responds, Err on timeout
    - Use `reqwest::blocking::Client` with 1s timeout per request
  - Implement `stop_server()`:
    - Send SIGTERM to child process (via `child.kill()` or nix::sys::signal)
    - Wait for process exit with timeout
    - Force kill if doesn't exit in 5 seconds
  - Implement `Drop` for WhisperKitProvider — calls `stop_server()` to prevent orphans
  - _Requirements: 2.3, 2.4, 2.5, 2.7_

- [x] 3.1 Write unit test for find_available_port
  - Test returns a port > 0
  - Test returned port is not already in use

- [x] 4. Implement TranscriptionProvider trait for WhisperKitProvider
  - Implement `name()` — returns `"whisperkit"`
  - Implement `initialize(config: &TranscriptionConfig)`:
    - Read model path from config or use `~/.jarvis/models/whisperkit/{model_name}`
    - Call `start_server(model_path)`
    - Call `wait_for_server(30)` (30 second timeout for first-run CoreML compilation)
    - On failure, return error with descriptive message
  - Implement `transcribe(audio: &[f32])`:
    - Convert audio to WAV via `audio_to_wav(audio, 16000)`
    - Build multipart form: `file` = WAV bytes, `model` = model_name
    - POST to `http://localhost:{port}/v1/audio/transcriptions`
    - Parse JSON response into segments
    - Map each segment: `text`, `start * 1000 → start_ms`, `end * 1000 → end_ms`, `is_final = true`
    - Return `Vec<TranscriptionSegment>`
    - On HTTP error, return descriptive error (don't crash)
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_

- [x] 4.1 Write unit test for segment mapping
  - Test float seconds to i64 milliseconds conversion
  - Test is_final is always true for batch mode
  - Test empty response returns empty Vec

- [x] 5. Checkpoint — WhisperKitProvider compiles and unit tests pass
  - Run `cargo build` in `jarvis-app/src-tauri`
  - Run `cargo test` — all existing + new tests pass
  - Verify no compilation warnings in new code
  - Ask user if questions arise

- [x] 6. Add transcription_engine and whisperkit_model to settings
  - Update `TranscriptionSettings` struct in `jarvis-app/src-tauri/src/settings/manager.rs`:
    - Add `transcription_engine: String` with `#[serde(default = "default_engine")]`
    - Add `whisperkit_model: String` with `#[serde(default = "default_whisperkit_model")]`
    - Add `fn default_engine() -> String { "whisper-rs".to_string() }`
    - Add `fn default_whisperkit_model() -> String { "openai_whisper-large-v3_turbo".to_string() }`
  - Update `default_settings()` to include the new fields
  - Update validation: `transcription_engine` must be `"whisper-rs"` or `"whisperkit"`
  - _Requirements: 1.1, 1.2, 4.6, 4.7_

- [x] 6.1 Write test for backward compatibility
  - Parse a settings.json without the new fields — should use defaults
  - Verify `transcription_engine` defaults to `"whisper-rs"`
  - Verify `whisperkit_model` defaults to `"openai_whisper-large-v3_turbo"`

- [x] 7. Update lib.rs for engine selection at startup
  - Import `WhisperKitProvider` in `lib.rs`
  - After loading settings, branch on `settings.transcription.transcription_engine`:
    - `"whisperkit"` → Create `WhisperKitProvider`, check `is_available()`, call `initialize()`
    - On failure → Log warning, fall back to `HybridProvider`
    - `"whisper-rs"` (or anything else) → Create `HybridProvider` (existing behavior)
  - Pass the selected `Box<dyn TranscriptionProvider>` to `TranscriptionManager::new()`
  - _Requirements: 1.6, 7.1, 7.2, 7.3_

- [x] 8. Add check_whisperkit_status Tauri command
  - Add `WhisperKitStatus` struct to `settings/` or `transcription/` module:
    ```rust
    #[derive(Serialize)]
    pub struct WhisperKitStatus {
        pub available: bool,
        pub reason: Option<String>,
    }
    ```
  - Implement `check_whisperkit_status` command in `commands.rs`:
    - Create a temporary `WhisperKitProvider` to check availability
    - Return `WhisperKitStatus`
  - Register command in `lib.rs` invoke_handler
  - _Requirements: 5.4, 5.5_

- [x] 9. Checkpoint — Backend engine switching works end-to-end
  - Run `cargo build` and `cargo test`
  - Manually test: set `transcription_engine: "whisperkit"` in settings.json
  - Launch app → verify WhisperKit starts (if CLI installed) or falls back
  - Set back to `"whisper-rs"` → verify original behavior
  - Ask user if questions arise

- [x] 10. Add WhisperKit model catalog to ModelManager
  - Add `WhisperKitModelEntry` struct and `WHISPERKIT_MODEL_CATALOG` constant in `model_manager.rs`
  - Include models: `openai_whisper-large-v3_turbo`, `openai_whisper-large-v3_turbo_632MB`, `openai_whisper-large-v3`, `openai_whisper-large-v3_947MB`, `openai_whisper-base.en`
  - Each entry: name, display_name, description, size_estimate, quality_tier
  - Implement `list_whisperkit_models()`:
    - Iterate catalog
    - Check `~/.jarvis/models/whisperkit/{name}/` directory exists for status
    - Return `Vec<ModelInfo>` with model metadata and status
  - Implement `whisperkit_model_exists(name)`:
    - Check if directory exists and contains `.mlmodelc` files
  - Create `~/.jarvis/models/whisperkit/` directory in `ModelManager::new()` if it doesn't exist
  - _Requirements: 4.1, 4.2, 4.3, 4.5_

- [x] 10.1 Write test for WhisperKit model catalog
  - Test catalog has at least 3 models
  - Test all entries have non-empty fields
  - Test `list_whisperkit_models()` returns catalog entries

- [x] 11. Implement WhisperKit model download
  - Implement `download_whisperkit_model(model_name: String)`:
    - Validate model_name is in WHISPERKIT_MODEL_CATALOG
    - Spawn background task that runs: `whisperkit-cli download --model {model_name} --output-dir ~/.jarvis/models/whisperkit/`
    - Parse stdout for download progress
    - Emit `model-download-progress` events
    - On completion, emit `model-download-complete`
    - On error, emit `model-download-error`
  - Add `list_whisperkit_models` and `download_whisperkit_model` Tauri commands
  - Register new commands in `lib.rs` invoke_handler
  - _Requirements: 4.4, 4.5_

- [x] 12. Checkpoint — Model management works
  - `cargo build` and `cargo test` pass
  - Manually test: call `list_whisperkit_models` from dev tools → see catalog
  - Download a model → verify progress events → verify files on disk
  - Ask user if questions arise

- [x] 13. Update frontend types for engine support
  - Update `TranscriptionSettings` in `jarvis-app/src/state/types.ts`:
    - Add `transcription_engine: "whisper-rs" | "whisperkit"`
    - Add `whisperkit_model: string`
  - Add `WhisperKitStatus` interface:
    ```typescript
    export interface WhisperKitStatus {
      available: boolean;
      reason?: string;
    }
    ```
  - _Requirements: 1.1, 6.1_

- [x] 14. Add Engine Selection UI to Settings.tsx
  - Add state: `whisperKitStatus` (loaded via `check_whisperkit_status` command on mount)
  - Add "Transcription Engine" section ABOVE the existing model section
  - Render two radio buttons: "whisper.cpp (Metal GPU)" and "WhisperKit (Apple Neural Engine)"
  - Disable WhisperKit radio if `!whisperKitStatus.available`, show reason
  - Show restart note: "Engine changes take effect after app restart."
  - On radio change: call `update_settings` with new `transcription_engine` value
  - _Requirements: 6.1, 6.2, 6.5, 6.6_

- [x] 15. Add conditional model list based on engine
  - When engine is `"whisperkit"`:
    - Load models via `list_whisperkit_models` command
    - Render WhisperKit model list (reuse ModelList component with WhisperKit data)
    - Download via `download_whisperkit_model` command
  - When engine is `"whisper-rs"`:
    - Use existing `list_models` command and ModelList (current behavior, unchanged)
  - Update model selection to store in `whisperkit_model` or `whisper_model` depending on engine
  - _Requirements: 6.3, 6.4_

- [x] 16. Add WhisperKit installation instructions UI
  - When WhisperKit is unavailable, show an info box below the disabled radio:
    - "WhisperKit requires Apple Silicon and macOS 14+."
    - "Install: `brew install whisperkit-cli`"
    - "Check Again" button that re-calls `check_whisperkit_status`
  - _Requirements: 8.1, 8.2, 8.3, 8.4_

- [x] 17. Add CSS styles for engine section
  - Add `.engine-options`, `.engine-option`, `.engine-unavailable`, `.engine-note` styles in `App.css`
  - Match existing settings section styling patterns
  - Disabled option should be visually dimmed
  - _Requirements: 6.2_

- [x] 18. Final build and verification
  - Run `cargo build` in `jarvis-app/src-tauri` — compiles without errors
  - Run `cargo test` — all tests pass (existing + new)
  - Run `npm run build` in `jarvis-app` — frontend compiles
  - Launch app: `make dev`
  - Open Settings → verify engine toggle appears
  - With whisperkit-cli installed: select WhisperKit, download model, restart, verify transcription
  - Without whisperkit-cli: verify WhisperKit option disabled with install instructions
  - Verify no orphaned whisperkit-cli processes after closing app
  - Verify switching back to whisper-rs works correctly

## Files Changed/Created

### New Files
- `jarvis-app/src-tauri/src/transcription/whisperkit_provider.rs` — WhisperKitProvider implementation

### Modified Files
- `jarvis-app/src-tauri/src/transcription/mod.rs` — add `pub mod whisperkit_provider`
- `jarvis-app/src-tauri/src/settings/manager.rs` — add `transcription_engine`, `whisperkit_model` fields
- `jarvis-app/src-tauri/src/settings/model_manager.rs` — add WhisperKit model catalog and methods
- `jarvis-app/src-tauri/src/commands.rs` — add `check_whisperkit_status`, `list_whisperkit_models`, `download_whisperkit_model`
- `jarvis-app/src-tauri/src/lib.rs` — engine selection logic at startup, register new commands
- `jarvis-app/src/state/types.ts` — add engine and whisperkit types
- `jarvis-app/src/components/Settings.tsx` — engine toggle, conditional model list, install instructions
- `jarvis-app/src/App.css` — engine section styles

## Notes

- Engine switching happens at app startup only (no runtime hot-swap for MVP)
- WhisperKit models are managed by whisperkit-cli's own download tooling, not raw HTTP like GGML models
- First CoreML model load may take 2-4 minutes (ANE compilation) — this is expected, show a note in UI
- `Drop` implementation on WhisperKitProvider is critical to prevent orphaned server processes
- The TranscriptionManager, AudioBuffer, and AudioRouter are completely unchanged — provider abstraction handles everything
- `reqwest::blocking::Client` is used (not async) because `transcribe()` is called inside `block_in_place()` already
- whisperkit-cli server port is randomized to avoid conflicts with other local services
