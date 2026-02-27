# Tasks: MLX Omni Transcription

## Phase 1: Foundation - Data Models & Database Schema

**Goal**: Establish the data structures and database schema to support transcript storage.

**Dependencies**: None

### 1. Data Model Extensions

- [x] 1.1 Add `TranscriptResult` struct to `provider.rs` with language and transcript fields (Debug, Clone, Serialize, Deserialize)

- [x] 1.2 Extend `Gem` struct in `store.rs` with transcript and transcript_language fields (with documentation)

- [x] 1.3 Extend `GemPreview` struct in `store.rs` with transcript_language field (with documentation)

- [x] 1.4 Update TypeScript `Gem` interface in `types.ts` with transcript and transcript_language fields

- [x] 1.5 Update TypeScript `GemPreview` interface in `types.ts` with transcript_language field

### 2. Database Schema Migration

- [x] 2.1 Add transcript and transcript_language columns to gems table in `initialize_schema()` (with PRAGMA checks and ALTER TABLE statements)

- [x] 2.2 Update `save()` SQL to include transcript and transcript_language in INSERT and UPDATE statements

- [x] 2.3 Update `get()` SQL to SELECT transcript and transcript_language columns and map them to Gem struct

- [x] 2.4 Update list/preview query to include transcript_language in SELECT and map to GemPreview struct

- [x] 2.5 Update FTS5 virtual table to include transcript column in content and all triggers (INSERT, UPDATE, DELETE)

## Phase 2: Backend Infrastructure - Python Sidecar & Rust Provider

**Goal**: Implement the transcript generation capability in the Python sidecar and wire it up through the Rust provider.

**Dependencies**: Phase 1 (data models needed for return types)

### 3. IntelProvider Trait Extension

- [x] 3.1 Add `generate_transcript()` method to `IntelProvider` trait with default "not supported" implementation

### 4. Model Catalog Updates

- [x] 4.1 Add `capabilities` field to `LlmModelEntry` struct and update all existing models to include `capabilities: &["text"]`

- [x] 4.2 Add Qwen 2.5 Omni models (3B 8-bit, 7B 4-bit) to `LLM_MODEL_CATALOG` with `capabilities: &["audio", "text"]`

- [x] 4.3 Update `LlmModelInfo` struct to include capabilities field and copy from `LlmModelEntry`

- [x] 4.4 Update TypeScript `LlmModelInfo` interface with `capabilities: string[]` field

### 5. Python Sidecar Extensions

- [x] 5.1 Add mlx-lm-omni and audio dependencies to `requirements.txt` (mlx-lm-omni, librosa, soundfile, numpy, packaging)

- [x] 5.2 Implement `apply_runtime_patches()` function with version-gated patches for mlx-lm-omni <= 0.1.3 (5 runtime patches: AudioTower, AudioMel, ExtendedQuantizedEmbedding, Model, AudioEncoder conv layout; Bug #6 prefill chunking handled at call-site via prefill_step_size parameter)

- [x] 5.3 Update `MLXServer.__init__()` to track capabilities

- [x] 5.4 Update `load_model()` to accept and store capabilities parameter (default to ["text"])

- [x] 5.5 Implement `generate_transcript()` method (load audio, convert formats, clear queue, generate with large prefill, parse response)

- [x] 5.6 Update `handle_command()` to route generate-transcript command

- [x] 5.7 Call `apply_runtime_patches()` at startup before model loading

### 6. MlxProvider Implementation

- [x] 6.1 Update `NdjsonCommand` struct with audio_path and capabilities fields

- [x] 6.2 Update `NdjsonResponse` struct with language, transcript, and capabilities fields

- [x] 6.3 Implement `generate_transcript_internal()` method with 120s timeout

- [x] 6.4 Implement `generate_transcript()` trait method with error handling

- [x] 6.5 Update `load_model_internal()` to look up and pass capabilities from catalog

## Phase 3: Integration - Gem Enrichment & Settings

**Goal**: Wire transcript generation into the gem enrichment flow and add user-facing settings.

**Dependencies**: Phase 2 (backend infrastructure must be working)

### 7. Gem Enrichment Flow

- [x] 7.1 Implement `extract_recording_path()` helper to build full path from gem source_data

- [x] 7.2 Update `enrich_content()` helper signature to accept gem and return transcript fields

- [x] 7.3 Extend `enrich_content()` to call generate_transcript() when audio path exists (with graceful error handling)

- [x] 7.4 Update `save_gem()` command to pass gem to enrich_content() and set transcript fields

- [x] 7.5 Update `enrich_gem()` command to pass gem to enrich_content() and set transcript fields

- [x] 7.6 Add transcription_engine check in enrich_content() to skip transcript generation unless engine is "mlx-omni"

### 8. Settings Extensions

- [x] 8.1 Update `TranscriptionSettings` struct with transcription_engine and mlx_omni_model fields (default to "whisper-rs")

- [x] 8.2 Update TypeScript `TranscriptionSettings` interface with transcription_engine and mlx_omni_model fields

## Phase 4: User Interface

**Goal**: Build the UI components for settings and gem display.

**Dependencies**: Phase 3 (settings and enrichment flow must be working)

### 9. Frontend UI - Settings Page

- [x] 9.1 Add MLX Omni transcription engine radio button option with description

- [x] 9.2 Implement multimodal models panel (shown when MLX Omni selected) with model cards, download buttons, progress bars, and venv status

- [x] 9.3 Update Intelligence section to filter models by capabilities (show multimodal models in both sections)

- [x] 9.4 Add informational note explaining Whisper for real-time, MLX for post-recording

### 10. Frontend UI - Gem Detail View

- [x] 10.1 Implement transcript display component (show MLX transcript prominently, Whisper collapsed when both exist, loading indicator during enrichment)

- [x] 10.2 Handle transcript-only display (show only Whisper when MLX is null, no empty placeholder)

## Phase 5: Testing & Validation

**Goal**: Comprehensive testing to ensure correctness and reliability.

**Dependencies**: Phases 1-4 (all implementation must be complete)

### 11. Unit Tests

- [ ] 11.1 Test runtime patches are logged at startup and version-gated (only for <= 0.1.3)

- [ ] 11.2 Test database migration adds transcript columns and is idempotent

- [ ] 11.3 Test Settings UI shows MLX Omni option, model picker, and capability filtering

- [ ] 11.4 Test edge cases (no model loaded, missing file, empty audio, text-only model errors)

- [ ] 11.5 Test gem detail UI (loading indicator, transcript display, no transcript section)

### 12. Property-Based Tests

- [ ] 12.1 Property 1: Version-Gated Patches (test patches only applied to versions <= 0.1.3)

- [ ] 12.2 Property 2: Audio File Format Handling (test .wav and .pcm files load correctly)

- [ ] 12.3 Property 3: Transcript Generation Round Trip (test generate_transcript returns result or error)

- [ ] 12.4 Property 4: State Isolation Between Transcriptions (test consecutive transcriptions are independent)

- [ ] 12.5 Property 5: Model Storage Location (test models stored at ~/.jarvis/models/llm/)

- [ ] 12.6 Property 6: UI Model Filtering by Capability (test models appear in correct UI sections)

- [ ] 12.7 Property 7: Enrichment Method Invocation (test correct methods called based on gem type)

- [ ] 12.8 Property 8: Graceful Enrichment Failure (test transcript failure doesn't fail enrichment)

- [ ] 12.9 Property 9: Default Trait Implementation (test providers without generate_transcript() return "not supported" error)

- [ ] 12.10 Property 10: Gem Serialization Completeness (test gems with transcript serialize completely)

- [ ] 12.11 Property 11: Sidecar Command Protocol (test generate_transcript sends correct NDJSON format and parses responses)

- [ ] 12.12 Property 12: Real-Time Transcription Independence (test Whisper continues for real-time when engine is "mlx-omni")

- [ ] 12.13 Property 13: UI Conditional Transcript Display (test transcript section visibility based on field)

- [ ] 12.14 Property 14: Settings-Based Enrichment Control (test transcript generation skipped/runs based on engine setting)

### 13. Integration Testing

- [ ] 13.1 Test end-to-end transcript generation (record audio, save as gem, verify transcript generated and displayed)

- [ ] 13.2 Test model download and selection (download multimodal model, select as active, verify transcript generation works)

- [ ] 13.3 Test graceful degradation (no multimodal model, text-only model, missing audio file, verify tags and summary still work)

## Phase 6: Documentation & Polish

**Goal**: Complete documentation and final polish.

**Dependencies**: Phase 5 (testing validates everything works)

### 14. Documentation

- [ ] 14.1 Update README with MLX Omni feature description, system requirements, and usage instructions

- [ ] 14.2 Add inline code documentation for all new public APIs with examples and error conditions

- [ ] 14.3 Update MANUAL_TESTING_GUIDE.md with MLX Omni testing scenarios, expected behaviors, and troubleshooting tips
