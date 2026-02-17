# Implementation Plan: JARVIS Settings Module

## Overview

This plan implements the JARVIS Settings Module in three phases: backend infrastructure (SettingsManager and ModelManager), HybridProvider integration, and frontend UI. Each phase builds incrementally with testing to catch errors early.

## Tasks

- [x] 1. Create settings module structure and SettingsManager
  - Create `jarvis-app/src-tauri/src/settings/` directory
  - Create `jarvis-app/src-tauri/src/settings/mod.rs` with module exports
  - Create `jarvis-app/src-tauri/src/settings/manager.rs` with SettingsManager struct
  - Implement Settings and TranscriptionSettings structs with serde derives
  - Implement SettingsManager::new() to load or create settings file
  - Implement SettingsManager::get() to return current settings
  - Implement SettingsManager::default_settings() with correct defaults
  - _Requirements: 1.1, 1.2, 2.4, 2.7, 3.4, 4.3_

- [ ]* 1.1 Write property test for SettingsManager initialization
  - **Property: Settings file creation with defaults**
  - **Validates: Requirements 1.2**

- [x] 2. Implement settings validation and persistence
  - [x] 2.1 Implement SettingsManager::validate() method
    - Validate vad_threshold range [0.0, 1.0]
    - Validate whisper_model is non-empty string
    - Validate all boolean fields are boolean type
    - Return descriptive error messages
    - _Requirements: 2.5, 8.1, 8.3, 8.4, 8.5, 8.6_
  
  - [ ]* 2.2 Write property test for validation
    - **Property 5: VAD threshold validation**
    - **Validates: Requirements 2.5**
  
  - [ ]* 2.3 Write property test for schema validation
    - **Property 23: Settings schema validation**
    - **Validates: Requirements 8.1, 8.3, 8.4, 8.5, 8.6**
  
  - [x] 2.4 Implement SettingsManager::save_to_file() with atomic writes
    - Write to temporary file `~/.jarvis/settings.json.tmp`
    - Serialize settings to JSON
    - Atomic rename to `~/.jarvis/settings.json`
    - _Requirements: 1.6_
  
  - [ ]* 2.5 Write property test for atomic file operations
    - **Property 2: Atomic file operations**
    - **Validates: Requirements 1.6**
  
  - [x] 2.6 Implement SettingsManager::update() method
    - Call validate() first
    - Call save_to_file() second (persist to disk FIRST)
    - Update in-memory state last (only if save succeeded)
    - Return error without modifying state if save fails
    - _Requirements: 1.3, 1.4, 1.5_
  
  - [ ]* 2.7 Write property test for persist-before-update ordering
    - **Property 1: Settings persist before in-memory update**
    - **Validates: Requirements 1.3, 1.4**
  
  - [ ]* 2.8 Write property test for validation-before-persistence
    - **Property 3: Validation before persistence**
    - **Validates: Requirements 1.5**

- [x] 3. Implement ModelManager for model discovery and downloads
  - [x] 3.1 Create ModelManager struct and initialization
    - Create `jarvis-app/src-tauri/src/settings/model_manager.rs`
    - Define ModelManager struct with models_dir, app_handle, download_queue, error_states
    - Define ModelInfo and ModelStatus types
    - Implement ModelManager::new() to create models directory if needed
    - Implement ModelManager::supported_models() returning static list
    - _Requirements: 5.3, 5.4_
  
  - [x] 3.2 Implement model discovery
    - Implement ModelManager::list_models() to scan ~/.jarvis/models/
    - Match files against pattern `ggml-*.bin`
    - Return ModelInfo with filename and file size for discovered models
    - Check download_queue for downloading status
    - Check error_states for error status
    - Return not_downloaded for supported models not on disk
    - _Requirements: 5.1, 5.2, 7.1, 7.2, 7.3, 7.5, 7.6_
  
  - [ ]* 3.3 Write property test for model discovery
    - **Property 11: Model discovery pattern matching**
    - **Validates: Requirements 5.1**
  
  - [ ]* 3.4 Write property test for model info completeness
    - **Property 12: Model info completeness**
    - **Validates: Requirements 5.2**
  
  - [ ]* 3.5 Write unit tests for model status reporting
    - Test all four status types returned correctly
    - Test file size included for downloaded models
    - Test progress included for downloading models
    - Test error message included for error status
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

- [x] 4. Checkpoint - Ensure settings and model discovery tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 5. Implement model download functionality
  - [x] 5.1 Implement GGML validation
    - Implement ModelManager::validate_ggml_file()
    - Check file size > 1MB
    - Read first 4 bytes and verify magic number 0x67676d6c
    - Return error if validation fails
    - _Requirements: 6.6_
  
  - [ ]* 5.2 Write unit test for GGML validation
    - Test valid GGML file passes
    - Test invalid magic number fails
    - Test file too small fails
    - _Requirements: 6.6_
  
  - [x] 5.3 Implement download URL construction
    - Implement method to build Hugging Face URL from model name
    - URL pattern: `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{model_name}`
    - _Requirements: 6.1_
  
  - [ ]* 5.4 Write property test for download URL construction
    - **Property 13: Download URL construction**
    - **Validates: Requirements 6.1**
  
  - [x] 5.5 Implement ModelManager::download_model()
    - Check if model already downloading (reject if so)
    - Add to download_queue with progress 0.0
    - Spawn async task with tokio::spawn for download (returns immediately, does NOT block)
    - In spawned task: Use reqwest to stream download to temp file
    - In spawned task: Emit progress events every 1% or 100KB
    - In spawned task: On completion, validate GGML format
    - In spawned task: Atomic rename to final location
    - In spawned task: Remove from download_queue
    - In spawned task: On error, clean up temp file and store in error_states
    - Note: The Tauri command must return immediately after spawning, not await the download
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6_
  
  - [ ]* 5.6 Write property test for download progress reporting
    - **Property 14: Download progress reporting**
    - **Validates: Requirements 6.2, 7.4**
    - Note: This test requires mocking the HTTP client (reqwest) or abstracting the HTTP layer to avoid real network calls
  
  - [ ]* 5.7 Write property test for concurrent download prevention
    - **Property 17: Concurrent download prevention**
    - **Validates: Requirements 6.5**
  
  - [ ]* 5.8 Write unit tests for download error handling
    - Test network error cleanup
    - Test invalid GGML cleanup
    - Test error state persistence
    - _Requirements: 6.4, 7.5, 7.6_
  
  - [x] 5.9 Implement ModelManager::cancel_download()
    - Check if model is downloading
    - Trigger cancellation token
    - Clean up temp file
    - Remove from download_queue
    - _Requirements: 6.5_
  
  - [x] 5.10 Implement ModelManager::delete_model()
    - Check if model file exists
    - Delete model file from ~/.jarvis/models/
    - Remove from error_states if present
    - Return error if file doesn't exist or deletion fails
    - _Requirements: 4.10, 4.11_

- [x] 6. Add Tauri commands for settings and models
  - [x] 6.1 Add settings commands to commands.rs
    - Implement get_settings command
    - Implement update_settings command with event emission
    - Add comprehensive doc comments
    - _Requirements: 9.1, 9.2, 9.5_
  
  - [x] 6.2 Write property test for settings change event emission
    - **Property 25: Settings change event emission**
    - **Validates: Requirements 9.5**
  
  - [ ]* 6.3 Write property test for partial settings updates
    - **Property 24: Partial settings updates**
    - **Validates: Requirements 9.2**
  
  - [x] 6.4 Add model commands to commands.rs
    - Implement list_models command
    - Implement download_model command
    - Implement cancel_download command
    - Implement delete_model command
    - Add comprehensive doc comments
    - _Requirements: 9.3, 9.4, 4.10_
  
  - [x] 6.5 Update lib.rs to register commands and manage state
    - Initialize SettingsManager in setup (Arc<RwLock<SettingsManager>>)
    - Initialize ModelManager in setup (Arc<ModelManager>)
    - Register all new commands in invoke_handler
    - _Requirements: 9.1, 9.2, 9.3, 9.4_

- [x] 7. Checkpoint - Ensure backend commands work
  - Ensure all tests pass, ask the user if questions arise.

- [x] 8. Integrate settings with HybridProvider
  - [x] 8.1 Update SileroVad to accept threshold parameter
    - Modify SileroVad::new() to accept threshold parameter
    - Add set_threshold() method for runtime updates
    - _Requirements: 2.8_
  
  - [x] 8.2 Update HybridProvider::new() to accept settings
    - Add settings parameter to new()
    - Conditionally initialize VAD based on vad_enabled
    - Conditionally initialize Vosk based on vosk_enabled
    - Always initialize Whisper
    - Pass vad_threshold to SileroVad::new()
    - _Requirements: 2.1, 2.2, 3.1, 3.2, 4.1_
  
  - [ ]* 8.3 Write property test for VAD conditional initialization
    - **Property 4: VAD conditional initialization**
    - **Validates: Requirements 2.1, 2.2**
  
  - [ ]* 8.4 Write property test for Vosk conditional initialization
    - **Property 7: Vosk conditional initialization**
    - **Validates: Requirements 3.1, 3.2**
  
  - [ ]* 8.5 Write property test for Whisper always initialized
    - **Property 8: Whisper always initialized**
    - **Validates: Requirements 4.1**
  
  - [x] 8.6 Implement HybridProvider::update_settings() method
    - Reinitialize VAD if enabled state changed
    - Update VAD threshold if VAD enabled
    - Reinitialize Vosk if enabled state changed
    - Reload Whisper model if model path changed
    - _Requirements: 2.8, 4.4, 11.4_
  
  - [ ]* 8.7 Write property test for VAD threshold runtime update
    - **Property 6: VAD threshold runtime update**
    - **Validates: Requirements 2.8**
  
  - [ ]* 8.8 Write property test for Whisper model runtime update
    - **Property 9: Whisper model runtime update**
    - **Validates: Requirements 4.4**
  
  - [ ]* 8.9 Write property test for runtime settings reconfiguration
    - **Property 30: Runtime settings reconfiguration**
    - **Validates: Requirements 11.4**
  
  - [x] 8.10 Update lib.rs to initialize HybridProvider with settings
    - Load settings from SettingsManager before creating HybridProvider
    - Pass settings.transcription to HybridProvider::new()
    - _Requirements: 11.1_
  
  - [x] 8.11 Update TranscriptionConfig to use settings
    - Check if TranscriptionConfig::from_env() exists in current codebase
    - If it exists: Modify to accept settings parameter and use settings.whisper_model
    - If it doesn't exist: Create it or modify HybridProvider initialization directly
    - Implement environment variable override chain:
      - JARVIS_WHISPER_MODEL overrides settings.whisper_model
      - JARVIS_VAD_ENABLED overrides settings.vad_enabled
      - JARVIS_VAD_THRESHOLD overrides settings.vad_threshold
      - JARVIS_VOSK_ENABLED overrides settings.vosk_enabled
    - Keep other fields from environment for backward compatibility
    - _Requirements: 4.2, 4.3, 5.8, 5.9_
  
  - [ ]* 8.12 Write property test for model path validation
    - **Property 10: Model path validation**
    - **Validates: Requirements 4.5**

- [x] 9. Create Settings UI component
  - [x] 9.1 Add Settings types to state/types.ts
    - Add Settings, TranscriptionSettings interfaces
    - Add ModelInfo, ModelStatus types
    - Add ModelProgressEvent interface
    - _Requirements: 10.1, 10.2, 10.3, 10.4_
  
  - [x] 9.2 Create Settings.tsx component
    - Create `jarvis-app/src/components/Settings.tsx`
    - Implement Settings component with onClose prop
    - Add state for settings, models, loading, error
    - Load settings and models on mount
    - Listen for settings-changed events
    - Listen for model-download-progress events
    - Listen for model-download-complete events
    - Listen for model-download-error events
    - _Requirements: 10.1, 10.2, 10.3, 10.4_
  
  - [x] 9.3 Implement VAD settings section
    - Add toggle for VAD enabled
    - Add slider for VAD threshold (0.0 to 1.0, step 0.05)
    - Display threshold as percentage
    - Disable slider when VAD disabled
    - Call update_settings on changes
    - _Requirements: 10.1, 10.2, 10.3, 10.9_
  
  - [ ]* 9.4 Write property test for VAD threshold percentage display
    - **Property 26: VAD threshold percentage display**
    - **Validates: Requirements 10.3**
  
  - [ ]* 9.5 Write property test for settings update command invocation
    - **Property 29: Settings update command invocation**
    - **Validates: Requirements 10.9**
  
  - [x] 9.6 Implement Vosk settings section
    - Add toggle for Vosk enabled
    - Call update_settings on changes
    - _Requirements: 10.1, 10.9_
  
  - [x] 9.7 Implement Whisper settings section
    - Add disabled toggle for Whisper (always enabled)
    - Add info text explaining Whisper is required
    - _Requirements: 10.1, 10.10_

- [x] 10. Create ModelList component
  - [x] 10.1 Create ModelList.tsx component
    - Create `jarvis-app/src/components/ModelList.tsx`
    - Accept models, selectedModel, callbacks as props
    - Render list of models with status-dependent UI
    - _Requirements: 10.4_
  
  - [x] 10.2 Implement model item rendering
    - Display model filename
    - Display file size for downloaded models
    - Display progress bar for downloading models
    - Display error message for error status
    - Display download button for not_downloaded models
    - Display select button for downloaded models
    - Display delete button for downloaded models
    - Highlight selected model
    - Show warning dialog when deleting selected model
    - _Requirements: 10.4, 10.5, 10.6, 10.7, 10.8, 4.10, 4.11_
  
  - [ ]* 10.3 Write property test for model status conditional UI
    - **Property 27: Model status conditional UI**
    - **Validates: Requirements 10.5, 10.6, 10.7**
  
  - [ ]* 10.4 Write property test for selected model highlighting
    - **Property 28: Selected model highlighting**
    - **Validates: Requirements 10.8**
  
  - [x] 10.5 Integrate ModelList into Settings component
    - Import and render ModelList in Settings.tsx
    - Pass models state and callbacks
    - Handle model selection by calling update_settings
    - Handle download by calling download_model command
    - Handle cancel by calling cancel_download command
    - _Requirements: 10.4, 10.5, 10.6, 10.7, 10.8, 10.9_

- [x] 11. Add Settings button to main UI
  - Update App.tsx to add Settings button
  - Add state for showing/hiding settings panel
  - Render Settings component when visible
  - _Requirements: 10.1_

- [x] 12. Add error handling and user feedback
  - [x] 12.1 Implement error display in Settings UI
    - Show error messages from failed operations
    - Add retry buttons for failed downloads
    - Add dismiss button for errors
    - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5_
  
  - [ ]* 12.2 Write property test for comprehensive error reporting
    - **Property 31: Comprehensive error reporting**
    - **Validates: Requirements 12.1, 12.2, 12.3, 12.4, 12.5**
  
  - [x] 12.3 Add loading states
    - Show spinner while loading settings
    - Show spinner while loading models
    - Disable controls during updates
    - _Requirements: 10.1, 10.4_

- [x] 13. Final checkpoint - Integration testing
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional property-based tests and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties (minimum 100 iterations each)
- Unit tests validate specific examples and edge cases
- Backend tasks (1-8) can be completed before frontend tasks (9-12)
- Settings persistence happens BEFORE in-memory updates to prevent stale state
- ModelManager uses internal Arc<Mutex<...>> for thread-safe mutable state without outer mutex
- HybridProvider integration requires modifying existing transcription code
