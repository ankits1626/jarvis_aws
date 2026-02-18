# Requirements Document

## Introduction

The JARVIS WhisperKit Module adds Apple-native speech recognition as a toggleable alternative to the existing whisper.cpp (whisper-rs) transcription engine. WhisperKit runs the full Whisper pipeline on Apple's Neural Engine (ANE) via CoreML, providing significantly lower latency and better accuracy than whisper.cpp on Apple Silicon. The integration uses `whisperkit-cli` as a Tauri sidecar process communicating via a local HTTP server with SSE streaming, matching the existing sidecar pattern used by JarvisListen.

## Glossary

- **WhisperKit**: Open-source Swift package by Argmax Inc. for on-device speech recognition on Apple Silicon via CoreML/ANE
- **whisperkit-cli**: Command-line interface for WhisperKit, installable via Homebrew, supports file transcription, streaming, and a local HTTP server
- **ANE**: Apple Neural Engine — dedicated ML accelerator on Apple Silicon chips
- **CoreML_Model**: WhisperKit's model format (`.mlmodelc`), distinct from GGML models used by whisper.cpp
- **Sidecar**: An external binary bundled with the Tauri app, spawned as a child process
- **SSE**: Server-Sent Events — HTTP streaming protocol used by whisperkit-cli's local server
- **Hypothesis_Text**: WhisperKit's fast interim transcription (~0.45s latency), may be corrected later
- **Confirmed_Text**: WhisperKit's stable verified transcription (~1.7s latency)
- **Dual_Stream**: WhisperKit's output mode providing both hypothesis and confirmed text
- **Transcription_Engine**: The active speech-to-text backend — either "whisper-rs" (current) or "whisperkit" (new)
- **HybridProvider**: The existing orchestrator that coordinates VAD, Vosk, and Whisper based on settings
- **WhisperKitProvider**: The NEW provider that wraps whisperkit-cli sidecar communication

## Requirements

### Requirement 1: Engine Selection Setting

**User Story:** As a user, I want to choose between whisper.cpp and WhisperKit transcription engines, so that I can use the fastest option available on my hardware.

#### Acceptance Criteria

1. THE Settings_Manager SHALL store a `transcription_engine` field with values `"whisper-rs"` or `"whisperkit"`
2. THE Settings_Manager SHALL default `transcription_engine` to `"whisper-rs"` for backward compatibility
3. WHEN the user changes the engine, THE Settings_UI SHALL display a description of each engine option
4. THE Settings_UI SHALL indicate that WhisperKit is recommended for Apple Silicon Macs
5. THE Settings_UI SHALL show WhisperKit as unavailable if the whisperkit-cli binary is not found
6. WHEN the engine setting changes, THE System SHALL require an app restart to take effect (engine swap at runtime is not required for MVP)

### Requirement 2: WhisperKit Sidecar Management

**User Story:** As a developer, I want whisperkit-cli managed as a Tauri sidecar, so that it follows the same pattern as JarvisListen.

#### Acceptance Criteria

1. THE WhisperKitProvider SHALL check for `whisperkit-cli` at a known path on initialization
2. IF whisperkit-cli is not found, THE WhisperKitProvider SHALL report status as `disabled` with a descriptive message
3. WHEN transcription starts, THE WhisperKitProvider SHALL spawn whisperkit-cli as a local HTTP server on a random available port
4. THE WhisperKitProvider SHALL wait for the server to become ready (health check) before sending audio
5. WHEN transcription stops, THE WhisperKitProvider SHALL gracefully shut down the whisperkit-cli process
6. IF whisperkit-cli crashes, THE WhisperKitProvider SHALL emit a transcription-error event and set status to error
7. THE WhisperKitProvider SHALL NOT leave orphaned whisperkit-cli processes on app exit

### Requirement 3: Audio Transcription via WhisperKit

**User Story:** As a user, I want WhisperKit to transcribe my audio with the same interface as whisper.cpp, so that switching engines is seamless.

#### Acceptance Criteria

1. THE WhisperKitProvider SHALL implement the existing `TranscriptionProvider` trait (`name()`, `initialize()`, `transcribe()`)
2. WHEN `transcribe()` is called with f32 audio, THE WhisperKitProvider SHALL send the audio to the local whisperkit-cli server as a WAV payload
3. THE WhisperKitProvider SHALL return `Vec<TranscriptionSegment>` matching the existing segment format
4. THE WhisperKitProvider SHALL map WhisperKit hypothesis text to `TranscriptionSegment { is_final: false }`
5. THE WhisperKitProvider SHALL map WhisperKit confirmed text to `TranscriptionSegment { is_final: true }`
6. WHEN WhisperKit is the active engine, THE existing AudioBuffer windowing and TranscriptionManager orchestration SHALL continue to work unchanged

### Requirement 4: WhisperKit CoreML Model Management

**User Story:** As a user, I want to download and manage WhisperKit CoreML models separately from whisper.cpp GGML models, so that each engine has its own model set.

#### Acceptance Criteria

1. THE ModelManager SHALL maintain a separate catalog of WhisperKit CoreML models
2. THE WhisperKit model catalog SHALL include at minimum: `openai_whisper-large-v3_turbo` and its compressed variant
3. WHEN listing models, THE ModelManager SHALL indicate which models are for which engine (whisper-rs or whisperkit)
4. THE ModelManager SHALL download WhisperKit models from the argmaxinc HuggingFace repository
5. THE ModelManager SHALL store WhisperKit models in `~/.jarvis/models/whisperkit/` (separate from GGML models in `~/.jarvis/models/`)
6. THE Settings_Manager SHALL store a `whisperkit_model` field for the selected WhisperKit model
7. THE Settings_Manager SHALL default `whisperkit_model` to `"openai_whisper-large-v3_turbo"`

### Requirement 5: WhisperKit Availability Detection

**User Story:** As a user, I want the app to tell me if WhisperKit can run on my system, so that I don't select an engine that won't work.

#### Acceptance Criteria

1. THE WhisperKitProvider SHALL detect whether the system is Apple Silicon (arm64)
2. THE WhisperKitProvider SHALL detect whether macOS version is 14.0 or later
3. THE WhisperKitProvider SHALL detect whether whisperkit-cli binary is available
4. IF any prerequisite is not met, THE Settings_UI SHALL disable the WhisperKit engine option with an explanation
5. THE WhisperKitProvider SHALL expose an `is_available()` method returning availability status and reason

### Requirement 6: Settings UI Engine Toggle

**User Story:** As a user, I want to see and switch between transcription engines in the Settings panel, so that I can experiment with both.

#### Acceptance Criteria

1. THE Settings_UI SHALL display an "Engine" section above the existing model selection
2. THE Engine section SHALL show two radio buttons: "whisper.cpp (Metal GPU)" and "WhisperKit (Apple Neural Engine)"
3. WHEN WhisperKit is selected, THE Settings_UI SHALL show WhisperKit-specific model list instead of GGML model list
4. WHEN whisper-rs is selected, THE Settings_UI SHALL show the existing GGML model list (current behavior)
5. THE Settings_UI SHALL display a note that engine changes require app restart
6. WHEN WhisperKit is unavailable, THE radio button SHALL be disabled with a tooltip explaining why

### Requirement 7: Graceful Fallback

**User Story:** As a developer, I want the system to fall back to whisper-rs if WhisperKit fails, so that transcription never silently stops working.

#### Acceptance Criteria

1. IF WhisperKit engine is selected but whisperkit-cli is not found at startup, THE System SHALL fall back to whisper-rs and log a warning
2. IF the WhisperKit local server fails to start, THE System SHALL fall back to whisper-rs and emit a transcription-error event
3. THE System SHALL NOT fall back mid-session — fallback only happens at initialization time
4. WHEN fallback occurs, THE Settings_UI SHALL display a notification explaining the fallback

### Requirement 8: WhisperKit CLI Installation Guide

**User Story:** As a user, I want instructions on how to install whisperkit-cli, so that I can enable the WhisperKit engine.

#### Acceptance Criteria

1. WHEN WhisperKit is unavailable, THE Settings_UI SHALL display installation instructions
2. THE instructions SHALL include the Homebrew command: `brew install whisperkit-cli`
3. THE Settings_UI SHALL include a "Check Again" button to re-detect whisperkit-cli after installation
4. THE instructions SHALL note that WhisperKit requires Apple Silicon and macOS 14+
