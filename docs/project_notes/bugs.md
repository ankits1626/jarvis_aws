# Bug Log

This file logs bugs encountered and their solutions for future reference.

## Format

### YYYY-MM-DD - Brief Bug Description
- **Issue**: What went wrong
- **Root Cause**: Why it happened
- **Solution**: How it was fixed
- **Prevention**: How to avoid it in the future

## Entries

### 2026-02-27 - MLX Sidecar Using Wrong generate() Function for Multimodal Models
- **Issue**: The `generate_transcript()` method in `server.py` was calling `generate()` from `mlx_lm`, which doesn't support the `audio` parameter needed for multimodal transcription. This would cause runtime errors when attempting to transcribe audio with multimodal models.
- **Root Cause**: Top-level import `from mlx_lm import load, generate` imported the text-only generate function. The `mlx_lm_omni` package provides its own `generate()` function that supports the `audio` parameter, but it wasn't being imported or used.
- **Solution**: 
  - Changed imports to use aliased names: `from mlx_lm import generate as mlx_lm_generate` and `from mlx_lm_omni import generate as mlx_omni_generate`
  - Added conditional import for `mlx_omni_generate` with `OMNI_AVAILABLE` flag
  - Updated `generate_tags()` and `summarize()` to use `mlx_lm_generate` (text-only)
  - Updated `generate_transcript()` to check `OMNI_AVAILABLE` and use `mlx_omni_generate` (multimodal)
  - Added error handling when `mlx_lm_omni` is not installed
- **Prevention**: When working with multiple packages that provide similar APIs (like `mlx_lm` and `mlx_lm_omni`), always use aliased imports to make it explicit which function is being called. Document which function supports which parameters.

### 2026-02-27 - Missing Patch 6 (7B Conv Weight Layout Detection)
- **Issue**: Requirement 1.7 specified a critical patch for 7B models to auto-detect and fix PyTorch conv weight layout mismatches, but this patch was not implemented in `apply_runtime_patches()`. The 7B model (`qwen-omni-7b-4bit`) would fail to produce correct transcription results without this fix.
- **Root Cause**: The patch was documented in requirements but was accidentally omitted during implementation of the runtime patches.
- **Solution**: Added Patch 5 (AudioEncoder conv layout detection) to `apply_runtime_patches()`:
  - Patches `AudioEncoder.__init__` to check if `conv1.weight.shape[1] != 3` (wrong layout)
  - Applies `mx.swapaxes(weight, 1, 2)` to both conv1 and conv2 when mismatch detected
  - Logs the fix to stderr for debugging
  - Version-gated like all other patches (only applies to mlx-lm-omni <= 0.1.3)
- **Prevention**: When implementing requirements, create a checklist of all specified patches/features and verify each one is implemented. Cross-reference requirements with implementation during code review.

### 2026-02-27 - Settings Validation Blocking MLX Omni Engine Selection (CRITICAL)
- **Issue**: The `validate()` function in `settings/manager.rs` rejected the "mlx-omni" transcription engine value, blocking users from selecting MLX Omni in the Settings UI. When users clicked the "MLX Omni" radio button, the frontend would call `update_settings()`, which calls `validate()`, which would return an error: "Transcription engine must be 'whisper-rs' or 'whisperkit', got 'mlx-omni'". This completely blocked the entire MLX Omni transcription feature.
- **Root Cause**: The validation check at `manager.rs:242-248` only allowed "whisper-rs" and "whisperkit" values. The "mlx-omni" value was added to the `TranscriptionSettings` struct and TypeScript types in Phase 3, but the validation function was not updated to accept it.
- **Solution**: Updated the validation check to include "mlx-omni":
  ```rust
  if engine != "whisper-rs" && engine != "whisperkit" && engine != "mlx-omni" {
      return Err(format!(
          "Transcription engine must be 'whisper-rs', 'whisperkit', or 'mlx-omni', got '{}'",
          engine
      ));
  }
  ```
- **Prevention**: When adding new enum-like string values to settings structs, always check for validation functions that might need updating. Search for all references to the field name to find validation logic. Consider using Rust enums with `#[serde(rename)]` instead of raw strings to get compile-time validation.
