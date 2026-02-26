# Implementation Plan: MLX Intelligence Provider

## Overview

This feature adds local LLM inference capabilities to Jarvis using MLX on Apple Silicon. The implementation follows the existing sidecar pattern (IntelligenceKitProvider) and model management pattern (ModelManager for Whisper). The system enables private, on-device AI enrichment of gems with tags and summaries.

Key components: Python MLX sidecar (NDJSON protocol), LlmModelManager (Rust), MlxProvider (IntelProvider trait), provider selection with fallback chain, IntelligenceSettings extension, 5 new Tauri commands, and frontend Settings UI.

## Tasks

### Phase 1: Python MLX Sidecar (Foundation)

- [x] 1. Create Python MLX sidecar with NDJSON protocol
  - [x] 1.1 Set up sidecar directory structure and dependencies
    - Create `src-tauri/sidecars/mlx-server/` directory
    - Create `requirements.txt` with mlx, mlx-lm, huggingface-hub dependencies
    - Create `server.py` with basic stdin/stdout NDJSON loop
    - Add platform check for Apple Silicon (arm64)
    - _Requirements: 1.13, Technical Constraints 1_
  
  - [x] 1.2 Implement check-availability and load-model commands
    - Implement `check-availability` command handler (check mlx_lm import)
    - Implement `load-model` command handler using mlx_lm.load()
    - Track loaded model state (model instance, tokenizer, model name)
    - Handle invalid model paths gracefully
    - _Requirements: 1.2, 1.3, 1.4_
  
  - [x] 1.3 Implement generate-tags and summarize commands
    - Implement `generate-tags` command with `/no_think` suffix, max_tokens=200
    - Implement `summarize` command with `/no_think` suffix, max_tokens=150
    - Parse tag generation output as JSON array
    - Return error if no model loaded
    - _Requirements: 1.5, 1.6, 1.7, 1.14_
  
  - [x] 1.4 Implement download-model command with progress reporting
    - Implement `download-model` command using huggingface_hub.snapshot_download()
    - Emit progress updates as NDJSON (progress percentage, downloaded_mb)
    - Emit completion message when download finishes
    - Handle download failures and invalid repo_ids
    - _Requirements: 1.8, 1.9_
  
  - [x] 1.5 Implement model-info and shutdown commands
    - Implement `model-info` command (return model name and param count)
    - Implement `shutdown` command (graceful exit)
    - Handle malformed JSON without crashing
    - _Requirements: 1.10, 1.11, 1.12_

### Phase 2: Model Management (LlmModelManager)

- [ ] 2. Create LlmModelManager for model catalog and downloads
  - [ ] 2.1 Define model catalog and data structures
    - Create `src/intelligence/llm_model_manager.rs`
    - Define `LLM_MODEL_CATALOG` with 4 models (Qwen 8B/4B/14B, Llama 3.2 3B)
    - Define `LlmModelInfo`, `LlmModelEntry`, `DownloadState` structs
    - Import and reuse `ModelStatus` enum from settings/model_manager.rs
    - Add `pub use model_manager::ModelStatus;` to `src/settings/mod.rs` for cross-module access
    - Add `pub mod llm_model_manager;` to `src/intelligence/mod.rs`
    - _Requirements: 2.1, 2.10_
  
  - [ ] 2.2 Implement LlmModelManager initialization and model listing
    - Implement `LlmModelManager::new()` (create ~/.jarvis/models/llm/ directory)
    - Implement `list_models()` (scan catalog, check disk, return status)
    - Implement `model_path()` helper (resolve model directory from catalog)
    - Implement `validate_model()` (check config.json exists)
    - _Requirements: 2.2, 2.3, 2.4, 2.11_
  
  - [ ] 2.3 Implement model download orchestration
    - Implement `download_model()` (spawn separate Python process for download)
    - Track download state with CancellationToken
    - Emit `llm-model-download-progress` Tauri events
    - Emit `llm-model-download-complete` on success
    - Use `.downloads/` temporary directory, atomic rename on completion
    - Prevent concurrent downloads of same model
    - _Requirements: 2.5, 2.6, 2.12_
  
  - [ ] 2.4 Implement download cancellation and model deletion
    - Implement `cancel_download()` (cancel task, clean up .downloads/)
    - Implement `delete_model()` (remove model directory)
    - Prevent deletion of active model
    - Handle error states and cleanup
    - _Requirements: 2.7, 2.8, 2.9_

### Phase 3: MlxProvider Implementation

- [x] 3. Implement MlxProvider (IntelProvider trait)
  - [x] 3.1 Create MlxProvider structure and initialization
    - Create `src/intelligence/mlx_provider.rs`
    - Define `MlxProvider` and `ProviderState` structs
    - Update `tauri.conf.json` to add `sidecars/mlx-server/**` to bundle.resources
    - Implement sidecar script path resolution (dev vs production)
    - Implement `MlxProvider::new()` (spawn sidecar, check availability, load model)
    - Add timeout handling (15s for initialization to accommodate model loading)
    - Add `pub mod mlx_provider;` to `src/intelligence/mod.rs`
    - _Requirements: 3.1, 3.2, 3.3_
  
  - [x] 3.2 Implement content chunking utility
    - Create `src/intelligence/utils.rs`
    - Implement `split_content()` function (15,000 char chunks, paragraph boundaries)
    - Implement `snap_to_char_boundary()` helper for UTF-8 safety
    - Add unit tests for chunking edge cases
    - Refactor `IntelligenceKitProvider` to use shared utility (remove duplicate functions)
    - Update `IntelligenceKitProvider` to import from `intelligence/utils` with MAX_CONTENT_CHARS = 10,000
    - Add `pub mod utils;` to `src/intelligence/mod.rs`
    - _Requirements: 3.5, 3.6_
  
  - [x] 3.3 Implement IntelProvider trait methods
    - Implement `check_availability()` (verify sidecar running and model loaded)
    - Implement `generate_tags()` (chunk content, deduplicate tags, max 5)
    - Implement `summarize()` (chunk content, combine multi-chunk summaries)
    - Use 60-second timeout for inference commands
    - Detect broken pipe on sidecar death
    - _Requirements: 3.4, 3.5, 3.6, 3.7, 3.8_
  
  - [x] 3.4 Implement model switching and shutdown
    - Implement `switch_model()` method (send load-model to running sidecar)
    - Preserve previous model state on switch failure
    - Implement `shutdown()` method (send shutdown command, cleanup)
    - _Requirements: 3.9, 3.10_
  
  - [ ]* 3.5 Write property test for content chunking
    - **Property 3: Content Chunking Preserves Information**
    - **Validates: Requirements 3.5, 3.6**
    - Generate random content strings of various sizes
    - Verify concatenation of chunks equals original
    - Verify all chunk boundaries are valid UTF-8
  
  - [ ]* 3.6 Write property test for tag deduplication
    - **Property 4: Tag Deduplication is Case-Insensitive**
    - **Validates: Requirements 3.5**
    - Generate random tag lists with case-varying duplicates
    - Verify no case-insensitive duplicates in result
    - Verify max 5 tags returned

### Phase 4: Provider Selection & Settings

- [x] 4. Implement provider selection and fallback chain
  - [x] 4.1 Update intelligence module for provider selection
    - Modify `src/intelligence/mod.rs` to accept Settings parameter
    - Implement `create_provider()` with fallback chain (MLX → IntelligenceKit → NoOp)
    - Return both trait object and direct MlxProvider reference (Arc-based)
    - Log which provider was successfully initialized
    - _Requirements: 4.1, 4.2, 4.3_
  
  - [x] 4.2 Update app initialization to pass settings
    - Modify `src/lib.rs` to pass settings to `create_provider()`
    - Register both managed state entries (trait object and MlxProvider reference)
    - Handle initialization timeout (15s max to accommodate model loading)
    - _Requirements: 4.4, 8.6_
  
  - [ ]* 4.3 Write property test for provider fallback chain
    - **Property 9: Provider Fallback Chain**
    - **Validates: Requirements 4.2**
    - Simulate various provider initialization failures
    - Verify fallback chain is followed correctly
    - Verify a provider is always returned (even NoOpProvider)

- [x] 5. Extend settings with IntelligenceSettings
  - [x] 5.1 Add IntelligenceSettings struct and validation
    - Add `IntelligenceSettings` struct to `src/settings/manager.rs`
    - Add fields: provider, active_model, python_path with defaults
    - Add `#[serde(default)]` to Settings.intelligence field
    - Implement validation rules (valid provider, non-empty fields)
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_
  
  - [x] 5.2 Test settings persistence and backward compatibility
    - Test settings round-trip (save and load)
    - Test loading old settings.json without intelligence key
    - Test validation error cases
    - _Requirements: 5.6, 8.5_
  
  - [ ]* 5.3 Write property test for backward compatibility
    - **Property 11: Backward Compatibility with Missing Intelligence Settings**
    - **Validates: Requirements 5.4**
    - Generate settings.json files without intelligence key
    - Verify loading succeeds with default values
    - Verify no errors or data loss

### Phase 5: Backend Commands & Checkpoint

- [ ] 6. Add new Tauri commands for LLM management
  - [ ] 6.1 Add LlmModelManager to Tauri managed state
    - Wrap LlmModelManager in Arc
    - Register in lib.rs setup
    - Update `enrich_content` command in commands.rs to use actual provider name from settings instead of hardcoded "intelligencekit"
    - Verify existing enrich_gem command still works unchanged
    - _Requirements: 6.4, 6.5_
  
  - [ ] 6.2 Implement model listing and download commands
    - Add `list_llm_models` command to `src/commands.rs`
    - Add `download_llm_model` command
    - Add `cancel_llm_download` command
    - Register commands in `src/lib.rs`
    - _Requirements: 6.1, 6.2_
  
  - [ ] 6.3 Implement model deletion and switching commands
    - Add `delete_llm_model` command
    - Add `switch_llm_model` command (verify downloaded, update settings, reload sidecar)
    - Handle error cases (model not downloaded, active model deletion)
    - _Requirements: 6.1, 6.2, 6.3_
  
  - [ ]* 6.4 Write property test for model list completeness
    - **Property 12: Model List Completeness**
    - **Validates: Requirements 2.4**
    - Call list_llm_models() in various states
    - Verify one entry per catalog model
    - Verify status accuracy matches disk state

- [x] 7. Checkpoint - Ensure backend tests pass
  - Run `cargo test` in src-tauri directory
  - Run `cargo clippy` to check for linting issues
  - Verify sidecar can be spawned and responds to commands
  - Ensure all tests pass, ask the user if questions arise

### Phase 6: Frontend UI

- [x] 8. Implement frontend Settings UI for Intelligence
  - [x] 8.1 Create IntelligenceSettings component structure
    - Add Intelligence section to `src/components/Settings.tsx` (verify file exists, create if needed)
    - Create provider selector (MLX, IntelligenceKit, API)
    - Set up state management for models list and active model
    - Add event listeners for download progress events
    - _Requirements: 7.1, 7.2, 7.11_
  
  - [x] 8.2 Create ModelList and ModelCard components
    - Create ModelCard component with status-based rendering
    - Display model info (name, size, quality tier, description)
    - Show appropriate actions based on status (Download/Cancel/Set Active/Delete)
    - Add progress bar for downloading state
    - _Requirements: 7.3, 7.4, 7.5, 7.8, 7.9_
  
  - [x] 8.3 Wire up model management actions
    - Implement download button handler (invoke download_llm_model)
    - Implement cancel button handler (invoke cancel_llm_download)
    - Implement set active button handler (invoke switch_llm_model)
    - Implement delete button handler (invoke delete_llm_model)
    - Update UI on download completion event
    - _Requirements: 7.6, 7.7, 7.9, 7.10_

### Phase 7: Error Handling & Robustness

- [ ] 9. Implement graceful degradation and error handling
  - [x] 9.1 Add error handling for missing dependencies
    - Handle Python not installed (fallback to IntelligenceKit)
    - Handle mlx not installed (fallback chain)
    - Show clear error messages in UI when AI unavailable
    - Disable enrich button with tooltip when no model downloaded
    - _Requirements: 8.1, 8.2_
  
  - [x] 9.2 Add error handling for runtime failures
    - Handle sidecar crashes (show toast notification)
    - Handle download failures (cleanup, show error, allow retry)
    - Handle missing model directory (detect during init, fallback)
    - Add timeout enforcement (15s init, 60s inference)
    - _Requirements: 8.3, 8.4, 8.5, 8.6_
  
  - [ ]* 9.3 Write property test for download atomicity
    - **Property 5: Model Download Atomicity**
    - **Validates: Requirements 2.12**
    - Simulate various download scenarios (success, failure, cancel)
    - Verify model dir is either complete or absent
    - Verify no partial state in final location
  
  - [ ]* 9.4 Write property test for download cancellation cleanup
    - **Property 6: Download Cancellation Cleanup**
    - **Validates: Requirements 2.7**
    - Cancel in-progress downloads
    - Verify all partial files removed from .downloads/
    - Verify model removed from download queue
  
  - [ ]* 9.5 Write property test for active model protection
    - **Property 7: Active Model Protection**
    - **Validates: Requirements 2.9**
    - Attempt to delete currently active model
    - Verify error returned
    - Verify model directory unchanged

- [ ] 10. Integration testing and validation
  - [ ] 10.1 Test model management flow
    - List models (verify all NotDownloaded initially)
    - Download a model (monitor progress)
    - Switch to downloaded model (verify settings updated)
    - Download second model
    - Switch between models (verify inference uses correct model)
    - Delete non-active model (verify directory removed)
    - _Requirements: 2.x, 6.x_
  
  - [ ] 10.2 Test end-to-end enrichment flow
    - Start app with MLX provider configured (requires model downloaded from 10.1)
    - Create a gem and enrich it
    - Verify tags and summary are generated
    - Verify gem is updated in database
    - _Requirements: All requirements (integration)_
  
  - [ ] 10.3 Test provider fallback scenarios
    - Configure MLX with invalid python path (verify fallback to IntelligenceKit)
    - Kill sidecar process manually (verify next command fails gracefully)
    - Test with no model downloaded (verify UI shows appropriate message)
    - _Requirements: 4.2, 8.x_

- [ ] 11. Final checkpoint - Ensure all tests pass
  - Run full test suite (unit, property, integration)
  - Verify all 15 correctness properties pass
  - Test manual scenarios (large model download, model switching, sidecar robustness)
  - Ensure all tests pass, ask the user if questions arise

## Notes

- Tasks marked with `*` are optional property-based tests and can be skipped for faster MVP
- The design uses Rust for backend implementation and TypeScript/React for frontend
- Python sidecar uses Python 3.10+ with MLX dependencies
- Each task references specific requirements for traceability
- Property tests validate universal correctness properties from the design document
- The implementation follows existing patterns: IntelligenceKitProvider (sidecar) and ModelManager (downloads)
- Two separate Python processes: long-lived inference sidecar (MlxProvider) and short-lived download processes (LlmModelManager)
- Content chunking utility extracted to intelligence/utils.rs for reuse
- Arc-based provider sharing eliminates need for Clone trait on MlxProvider
