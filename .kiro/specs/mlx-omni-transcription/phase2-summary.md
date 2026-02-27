# Phase 2 Summary: Backend Infrastructure - Python Sidecar & Rust Provider

**Date**: February 27, 2026
**Status**: ✅ Complete

## Overview

Phase 2 implemented the transcript generation capability in the Python sidecar and wired it up through the Rust provider. All backend infrastructure is now in place to support multimodal audio transcription.

## Completed Tasks

### Section 3: IntelProvider Trait Extension (1 task)
- ✅ 3.1: Added `generate_transcript()` method to IntelProvider trait with default "not supported" implementation

### Section 4: Model Catalog Updates (4 tasks)
- ✅ 4.1: Added `capabilities` field to `LlmModelEntry` struct and updated all existing models to include `capabilities: &["text"]`
- ✅ 4.2: Added Qwen 2.5 Omni models (3B 8-bit, 7B 4-bit) to `LLM_MODEL_CATALOG` with `capabilities: &["audio", "text"]`
- ✅ 4.3: Updated `LlmModelInfo` struct to include capabilities field and copy from `LlmModelEntry`
- ✅ 4.4: Updated TypeScript `LlmModelInfo` interface with `capabilities: string[]` field

### Section 5: Python Sidecar Extensions (7 tasks)
- ✅ 5.1: Added mlx-lm-omni and audio dependencies to `requirements.txt`
- ✅ 5.2: Implemented `apply_runtime_patches()` function with version-gated patches for mlx-lm-omni <= 0.1.3 (5 runtime patches: AudioTower, AudioMel, ExtendedQuantizedEmbedding, Model, AudioEncoder conv layout; Bug #6 prefill chunking handled at call-site)
- ✅ 5.3: Updated `MLXServer.__init__()` to track capabilities
- ✅ 5.4: Updated `load_model()` to accept and store capabilities parameter (default to ["text"])
- ✅ 5.5: Implemented `generate_transcript()` method (load audio, convert formats, clear queue, generate with large prefill, parse response)
- ✅ 5.6: Updated `handle_command()` to route generate-transcript command
- ✅ 5.7: Called `apply_runtime_patches()` at startup before model loading

### Section 6: MlxProvider Implementation (5 tasks)
- ✅ 6.1: Updated `NdjsonCommand` struct with audio_path and capabilities fields
- ✅ 6.2: Updated `NdjsonResponse` struct with language, transcript, and capabilities fields
- ✅ 6.3: Implemented `generate_transcript_internal()` method with 120s timeout
- ✅ 6.4: Implemented `generate_transcript()` trait method with error handling
- ✅ 6.5: Updated `load_model_internal()` to look up and pass capabilities from catalog

## Key Changes

### Rust Backend

**IntelProvider Trait** (`provider.rs`):
- Added `TranscriptResult` struct with language and transcript fields
- Added `generate_transcript()` method with default implementation returning "not supported" error

**LLM Model Catalog** (`llm_model_manager.rs`):
- Added `capabilities` field to `LlmModelEntry` struct
- Updated all existing text-only models with `capabilities: &["text"]`
- Added two new multimodal models:
  - Qwen 2.5 Omni 3B (8-bit): ~5 GB, good quality
  - Qwen 2.5 Omni 7B (4-bit): ~8 GB, better quality
- Updated `LlmModelInfo` struct and TypeScript interface to include capabilities

**MlxProvider** (`mlx_provider.rs`):
- Extended `NdjsonCommand` with `audio_path` and `capabilities` fields
- Extended `NdjsonResponse` with `language`, `transcript`, and `capabilities` fields
- Implemented `generate_transcript_internal()` with 120s timeout
- Implemented `generate_transcript()` trait method
- Updated `load_model_internal()` to look up capabilities from catalog and pass to sidecar
- Added `lookup_capabilities()` helper function to map model directory names to capabilities

### Python Sidecar

**Runtime Patches** (`server.py`):
- Implemented `apply_runtime_patches()` function with version-gated patches for mlx-lm-omni <= 0.1.3
- Fixes 6 critical bugs total (5 via runtime patches, 1 via call-site configuration):
  1. AudioTower reshape bug (causes failure on audio > 15s) - **Runtime patch**
  2. AudioMel precision loss (float16 → float32) - **Runtime patch**
  3. ExtendedQuantizedEmbedding kwargs compatibility - **Runtime patch**
  4. Model attribute delegation to tokenizer - **Runtime patch**
  5. 7B model conv layout detection (auto-detect and fix PyTorch layout) - **Runtime patch**
  6. Prefill chunking bug (causes IndexError on audio > 30s) - **Call-site fix via prefill_step_size=32768**
- Bug #6 is intentionally NOT patched because the fix is cleaner at call-site (avoids patching generate() internals)
- Patches automatically disabled for versions > 0.1.3

**MLXServer Class** (`server.py`):
- Added `capabilities` field to track model capabilities from catalog
- Updated `load_model()` to accept and store capabilities parameter (defaults to ["text"])
- Implemented `generate_transcript()` method:
  - Loads audio from .wav or .pcm files
  - Converts to float32 format
  - Clears ExtendedEmbedding queue to prevent state leakage
  - Generates transcript with language detection prompt
  - Uses large prefill_step_size (32768) to prevent chunking issues
  - Parses response to extract language and transcript
- Updated `handle_command()` to route generate-transcript command
- Called `apply_runtime_patches()` at startup

**Dependencies** (`requirements.txt`):
- Added mlx-lm-omni>=0.1.3
- Added librosa>=0.10.0
- Added soundfile>=0.12.0
- Added numpy>=1.24.0
- Added packaging>=20.0

### TypeScript Frontend

**Types** (`types.ts`):
- Updated `LlmModelInfo` interface with `capabilities: string[]` field

## Files Modified

### Rust Files
- `jarvis-app/src-tauri/src/intelligence/provider.rs`
- `jarvis-app/src-tauri/src/intelligence/llm_model_manager.rs`
- `jarvis-app/src-tauri/src/intelligence/mlx_provider.rs`
- `jarvis-app/src-tauri/src/commands.rs` (added transcript fields to Gem initialization)
- `jarvis-app/src/state/types.ts`

### Python Files
- `jarvis-app/src-tauri/sidecars/mlx-server/server.py`
- `jarvis-app/src-tauri/sidecars/mlx-server/requirements.txt`

## Verification

✅ Code compiles successfully with `cargo check`
✅ All 17 Phase 2 tasks completed
✅ No compilation errors
✅ Only 1 harmless warning (unused `capabilities` field in NdjsonResponse - will be used in Phase 3)

## Next Steps

Phase 3 will implement:
- Gem enrichment flow integration
- Settings extensions for transcription engine selection
- Helper functions to extract recording paths from gems

## Notes

- The capabilities lookup in `MlxProvider::lookup_capabilities()` uses pattern matching on model directory names to avoid circular dependencies with `LlmModelManager`
- The 120s timeout for transcript generation is longer than tags/summary (60s) to accommodate longer audio files
- Runtime patches are version-gated and will automatically disable when mlx-lm-omni releases fixes
- The Python sidecar gracefully handles missing audio dependencies (librosa, soundfile) with clear error messages
- **CRITICAL**: The 7B model conv weight layout patch (Patch 5) was implemented in this phase. This is essential for the 7B model (qwen-omni-7b-4bit) to work correctly. Without this patch, the 7B model would fail to produce correct transcription results due to mismatched weight layouts between PyTorch and MLX formats.
