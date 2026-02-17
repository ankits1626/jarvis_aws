# Requirements Document

## Introduction

The JARVIS Settings Module provides persistent configuration management for the JARVIS desktop application's transcription pipeline. Users can toggle transcription engines (VAD, Vosk, Whisper), adjust VAD sensitivity, select Whisper models, and download models directly from the UI. Settings are persisted to `~/.jarvis/settings.json` and loaded on application startup, allowing the transcription system to adapt its behavior based on user preferences.

## Glossary

- **Settings_Manager**: The Rust component responsible for reading, writing, and validating settings from `~/.jarvis/settings.json`
- **Transcription_Engine**: A component that processes audio (VAD, Vosk, or Whisper)
- **VAD**: Voice Activity Detection using Silero model to skip silence
- **Vosk**: Lightweight speech recognition engine providing instant partial transcriptions
- **Whisper**: OpenAI's speech recognition model providing high-quality final transcriptions
- **Hybrid_Provider**: The orchestrator that coordinates VAD, Vosk, and Whisper based on settings
- **Model_File**: A Whisper GGML model file (e.g., `ggml-base.en.bin`) stored in `~/.jarvis/models/`
- **Settings_UI**: The React/TypeScript frontend component for displaying and modifying settings
- **Tauri_Command**: A Rust function exposed to the frontend via Tauri's IPC mechanism
- **Progress_Event**: A Tauri event emitted during model download containing progress percentage

## Requirements

### Requirement 1: Settings Persistence

**User Story:** As a user, I want my transcription preferences saved between sessions, so that I don't have to reconfigure the application every time I launch it.

#### Acceptance Criteria

1. WHEN the application starts, THE Settings_Manager SHALL load settings from `~/.jarvis/settings.json`
2. IF `~/.jarvis/settings.json` does not exist, THEN THE Settings_Manager SHALL create it with default values
3. WHEN a user modifies a setting, THE Settings_Manager SHALL persist the change to disk BEFORE updating in-memory state
4. WHEN writing settings fails, THE Settings_Manager SHALL return an error without modifying in-memory state
5. THE Settings_Manager SHALL validate the settings schema before persisting to disk
6. THE Settings_Manager SHALL use atomic file operations to prevent partial writes

### Requirement 2: VAD Engine Configuration

**User Story:** As a user, I want to enable or disable voice activity detection, so that I can control whether silence is skipped during transcription.

#### Acceptance Criteria

1. WHEN VAD is enabled, THE Hybrid_Provider SHALL initialize the Silero_Vad component
2. WHEN VAD is disabled, THE Hybrid_Provider SHALL process all audio without silence detection
3. THE Settings_Manager SHALL store the VAD enabled state as a boolean in the settings file
4. THE Settings_Manager SHALL provide a default value of `true` for `vad_enabled`
5. WHEN the VAD threshold is modified, THE Settings_Manager SHALL validate it is between 0.0 and 1.0
6. THE Settings_Manager SHALL store the VAD threshold as a floating-point number in the settings file
7. THE Settings_Manager SHALL provide a default value of 0.3 for `vad_threshold`
8. WHEN the VAD threshold changes, THE Silero_Vad component SHALL use the new threshold for subsequent audio processing

### Requirement 3: Vosk Engine Configuration

**User Story:** As a user, I want to enable or disable Vosk partial transcriptions, so that I can choose whether to see instant preview text.

#### Acceptance Criteria

1. WHEN Vosk is enabled, THE Hybrid_Provider SHALL initialize the Vosk_Provider component
2. WHEN Vosk is disabled, THE Hybrid_Provider SHALL skip partial transcription processing
3. THE Settings_Manager SHALL store the Vosk enabled state as a boolean in the settings file
4. THE Settings_Manager SHALL provide a default value of `true` for `vosk_enabled`

### Requirement 4: Whisper Engine Configuration

**User Story:** As a user, I want to select which Whisper model to use for transcription, so that I can balance quality and performance based on my needs.

#### Acceptance Criteria

1. THE Whisper_Provider SHALL always be initialized regardless of settings
2. THE Settings_Manager SHALL store the selected Whisper model filename in the settings file
3. THE Settings_Manager SHALL provide a default value of `ggml-base.en.bin` for `whisper_model`
4. WHEN the selected model changes, THE Whisper_Provider SHALL use the new model path for subsequent transcriptions
5. THE Settings_Manager SHALL validate that the selected model file exists in `~/.jarvis/models/` before persisting
6. WHEN a user requests to delete a model, THE Settings_Manager SHALL delete the model file from `~/.jarvis/models/`
7. WHEN a user attempts to delete the currently selected model, THE Settings_UI SHALL display a warning dialog
8. WHEN a user confirms deletion of the selected model, THE Settings_Manager SHALL delete the file and the Settings_UI SHALL prompt the user to select a different model
9. THE Settings_Manager SHALL return an error if attempting to delete a model that doesn't exist
10. THE Settings_Manager SHALL return an error if attempting to delete a model that is currently downloading

### Requirement 5: Model Discovery

**User Story:** As a user, I want to see which Whisper models are available on my system, so that I can select one for transcription.

#### Acceptance Criteria

1. WHEN the Settings_UI requests available models, THE Settings_Manager SHALL scan `~/.jarvis/models/` for files matching the pattern `ggml-*.bin`
2. THE Settings_Manager SHALL return a list of discovered model files with their filenames and file sizes
3. THE Settings_Manager SHALL support the following model names: `ggml-tiny.en.bin`, `ggml-base.en.bin`, `ggml-small.en.bin`, `ggml-medium.en.bin`
4. IF `~/.jarvis/models/` does not exist, THEN THE Settings_Manager SHALL create the directory
5. WHERE the JARVIS_WHISPER_MODEL environment variable is set, THE System SHALL use that model path instead of the settings value
6. WHERE the JARVIS_VAD_ENABLED environment variable is set, THE System SHALL use that boolean value instead of the settings value
7. WHERE the JARVIS_VAD_THRESHOLD environment variable is set, THE System SHALL use that threshold value instead of the settings value
8. WHERE the JARVIS_VOSK_ENABLED environment variable is set, THE System SHALL use that boolean value instead of the settings value
9. THE System SHALL apply environment variable overrides at runtime without modifying the persisted settings file

### Requirement 6: Model Download

**User Story:** As a user, I want to download Whisper models that aren't available locally, so that I can use different quality levels without manual file management.

#### Acceptance Criteria

1. WHEN a user requests a model download, THE Settings_Manager SHALL download the model from `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{model_name}`
2. WHEN downloading a model, THE Settings_Manager SHALL emit Progress_Event messages with the download percentage
3. WHEN a download completes successfully, THE Settings_Manager SHALL save the model to `~/.jarvis/models/{model_name}`
4. IF a download fails, THEN THE Settings_Manager SHALL return an error and clean up any partial files
5. WHEN a download is in progress, THE Settings_Manager SHALL prevent concurrent downloads of the same model
6. THE Settings_Manager SHALL validate that the downloaded file is a valid GGML model before marking it as complete

### Requirement 7: Model Status Reporting

**User Story:** As a user, I want to see the status of each Whisper model, so that I know which models are ready to use and which are being downloaded.

#### Acceptance Criteria

1. WHEN the Settings_UI requests model status, THE Settings_Manager SHALL return the status for each supported model
2. THE Settings_Manager SHALL report status as one of: `downloaded`, `downloading`, `error`, or `not_downloaded`
3. WHEN a model is downloaded, THE Settings_Manager SHALL include the file size in bytes
4. WHEN a model is downloading, THE Settings_Manager SHALL include the current progress percentage as a floating-point number between 0.0 and 100.0
5. WHEN a model download has failed, THE Settings_Manager SHALL include the error message in the status response
6. THE Settings_Manager SHALL track download errors in persistent state and surface them via `list_models`

### Requirement 8: Settings Schema Validation

**User Story:** As a developer, I want settings to be validated against a schema, so that invalid configurations are caught early and don't cause runtime errors.

#### Acceptance Criteria

1. THE Settings_Manager SHALL validate that `vad_enabled` is a boolean
2. THE Settings_Manager SHALL validate that `vad_threshold` is a floating-point number between 0.0 and 1.0
3. THE Settings_Manager SHALL validate that `vosk_enabled` is a boolean
4. THE Settings_Manager SHALL validate that `whisper_enabled` is a boolean
5. THE Settings_Manager SHALL validate that `whisper_model` is a non-empty string
6. IF validation fails, THEN THE Settings_Manager SHALL return a descriptive error message

### Requirement 9: Tauri Command Integration

**User Story:** As a frontend developer, I want to interact with settings through Tauri commands, so that I can build a reactive settings UI.

#### Acceptance Criteria

1. THE Settings_Manager SHALL expose a `get_settings` Tauri_Command that returns the current settings
2. THE Settings_Manager SHALL expose an `update_settings` Tauri_Command that accepts partial settings updates
3. THE Settings_Manager SHALL expose a `list_models` Tauri_Command that returns available models and their status
4. THE Settings_Manager SHALL expose a `download_model` Tauri_Command that initiates a model download
5. WHEN a setting is updated via Tauri_Command, THE Settings_Manager SHALL emit a settings change event to the frontend

### Requirement 10: Settings UI Component

**User Story:** As a user, I want a dedicated settings page in the application, so that I can easily configure transcription preferences.

#### Acceptance Criteria

1. THE Settings_UI SHALL display toggle switches for VAD, Vosk, and Whisper engines
2. THE Settings_UI SHALL display a slider for VAD threshold with range 0.0 to 1.0
3. THE Settings_UI SHALL display the current VAD threshold value as a percentage
4. THE Settings_UI SHALL display a list of available Whisper models with their status
5. WHEN a model is not downloaded, THE Settings_UI SHALL display a download button
6. WHEN a model is downloading, THE Settings_UI SHALL display a progress bar
7. WHEN a model is downloaded, THE Settings_UI SHALL display the file size
8. THE Settings_UI SHALL highlight the currently selected Whisper model
9. WHEN a user changes a setting, THE Settings_UI SHALL call the appropriate Tauri_Command
10. THE Settings_UI SHALL disable the Whisper toggle to prevent users from disabling it

### Requirement 11: Hybrid Provider Integration

**User Story:** As a developer, I want the Hybrid_Provider to respect settings, so that transcription behavior matches user preferences.

#### Acceptance Criteria

1. WHEN Hybrid_Provider is initialized, THE Hybrid_Provider SHALL read settings from Settings_Manager
2. WHEN VAD is disabled in settings, THE Hybrid_Provider SHALL not initialize Silero_Vad
3. WHEN Vosk is disabled in settings, THE Hybrid_Provider SHALL not initialize Vosk_Provider
4. WHEN settings change at runtime, THE Hybrid_Provider SHALL reinitialize affected components
5. THE Hybrid_Provider SHALL always initialize Whisper_Provider regardless of settings

### Requirement 12: Error Handling

**User Story:** As a user, I want clear error messages when settings operations fail, so that I can understand and resolve issues.

#### Acceptance Criteria

1. WHEN settings file cannot be read, THE Settings_Manager SHALL return an error describing the file system issue
2. WHEN settings file contains invalid JSON, THE Settings_Manager SHALL return an error with the parse failure location
3. WHEN a model download fails, THE Settings_Manager SHALL return an error with the HTTP status code or network error
4. WHEN a model file is corrupted, THE Settings_Manager SHALL return an error indicating validation failure
5. WHEN the `~/.jarvis/` directory cannot be created, THE Settings_Manager SHALL return an error describing the permission issue
