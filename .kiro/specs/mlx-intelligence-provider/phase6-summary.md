# Phase 6 Summary: Frontend Settings UI for Intelligence

## Completed: February 26, 2026

### Overview
Successfully implemented the frontend Settings UI for the MLX Intelligence Provider, integrating seamlessly with the existing Settings component and reusing the ModelList component pattern.

### Changes Made

#### 1. Type Definitions (`jarvis-app/src/state/types.ts`)

**Added LLM Model Types:**
- `LlmModelInfo` interface matching Rust `LlmModelInfo` struct
  - Fields: id, display_name, repo_id, description, size_estimate, quality_tier, status
- `LlmModelProgressEvent` for download progress events
- `LlmModelDownloadCompleteEvent` for completion events
- `LlmModelDownloadErrorEvent` for error events

**Extended Settings Interface:**
- Added `IntelligenceSettings` interface
  - Fields: provider (string), active_model (string), python_path (string)
- Updated `Settings` interface to include `intelligence: IntelligenceSettings`

#### 2. ModelList Component Enhancement (`jarvis-app/src/components/ModelList.tsx`)

**Added Custom Select Handler Support:**
- New optional prop: `customSelectHandler?: (modelName: string) => Promise<void>`
- Modified `handleSelect()` to use custom handler when provided
- Falls back to default settings update behavior for Whisper models
- Enables different selection logic for LLM models (uses `switch_llm_model` command)

**Key Implementation:**
```typescript
if (customSelectHandler) {
  await customSelectHandler(modelName);
} else {
  // Default settings update for Whisper models
  const settings = await invoke<Settings>('get_settings');
  // ... update transcription settings
}
```

#### 3. Settings Component (`jarvis-app/src/components/Settings.tsx`)

**State Management:**
- Added `llmModels` state: `useState<LlmModelInfo[]>([])`
- Loads LLM models on mount via `invoke('list_llm_models')`

**Event Listeners:**
- `llm-model-download-progress` - Updates model progress in real-time
- `llm-model-download-complete` - Refreshes model list on completion
- `llm-model-download-error` - Updates model status to error state

**Handler Functions:**
- `handleProviderChange(provider: string)` - Updates intelligence.provider in settings
- `handleLlmModelSwitch(modelId: string)` - Calls `switch_llm_model` command and refreshes list

**UI Structure:**
```tsx
<section className="settings-section">
  <h3>Intelligence Provider</h3>
  <div className="provider-options">
    <label>MLX (Local, Private)</label>
    <label>IntelligenceKit (Local, Fast)</label>
    <label>Cloud API (Coming Soon)</label>
  </div>
</section>

{settings.intelligence.provider === "mlx" && (
  <section className="settings-section">
    <h3>MLX Models</h3>
    <ModelList
      models={llmModels.map(...)}
      selectedModel={settings.intelligence.active_model}
      customSelectHandler={handleLlmModelSwitch}
      downloadCommand="download_llm_model"
      cancelCommand="cancel_llm_download"
      deleteCommand="delete_llm_model"
    />
  </section>
)}
```

### Design Patterns Used

**Component Reuse:**
- Leveraged existing `ModelList` component for LLM models
- Extended with `customSelectHandler` prop for flexibility
- Maintains consistent UX with Whisper model management

**Data Transformation:**
- Maps `LlmModelInfo` to `ModelInfo` format for ModelList compatibility
- Uses `id` field as `filename` (ModelList expects this)
- Preserves all display fields (display_name, description, size_estimate, quality_tier, status)

**Event-Driven Updates:**
- Real-time progress updates via Tauri events
- Optimistic UI updates on download start
- Automatic refresh on completion/error

### User Experience

**Provider Selection:**
- Radio buttons for provider choice (MLX, IntelligenceKit, API)
- Changes take effect immediately
- MLX models section only visible when MLX provider selected

**Model Management:**
- Download button for NotDownloaded models
- Progress bar with percentage for Downloading models
- Cancel button during download
- Select/Delete buttons for Downloaded models
- Active model shows "Selected" badge
- Error state with Retry button

**Status Indicators:**
- Quality tier badges (Basic, Good, Great, Best)
- Size estimates displayed
- Download progress in real-time
- Error messages inline

### Integration Points

**Backend Commands Used:**
- `list_llm_models()` - Load model catalog with status
- `download_llm_model(model_id)` - Start download
- `cancel_llm_download(model_id)` - Cancel in-progress download
- `delete_llm_model(model_id)` - Remove downloaded model
- `switch_llm_model(model_id)` - Switch active model
- `update_settings(settings)` - Update provider preference

**Events Consumed:**
- `llm-model-download-progress` - Progress updates
- `llm-model-download-complete` - Download finished
- `llm-model-download-error` - Download failed

### Testing Status

**TypeScript Compilation:**
- ✅ No diagnostics in Settings.tsx
- ✅ No diagnostics in ModelList.tsx
- ✅ No diagnostics in types.ts

**Manual Testing Required:**
- Provider switching (MLX ↔ IntelligenceKit)
- Model download flow (start, progress, complete)
- Model cancellation (cleanup verification)
- Model switching (hot reload)
- Model deletion (active model protection)
- Error handling (network failures, invalid models)

### Files Modified

1. `jarvis-app/src/state/types.ts` - Added LLM types and IntelligenceSettings
2. `jarvis-app/src/components/ModelList.tsx` - Added customSelectHandler support
3. `jarvis-app/src/components/Settings.tsx` - Added Intelligence section with provider selector and model list
4. `jarvis-app/src-tauri/src/commands.rs` - Fixed settings rollback on model switch failure

### Bug Fixes

**Settings Rollback on Model Switch Failure:**
- Issue: `switch_llm_model` updated settings before attempting sidecar reload
- If sidecar failed to load new model, settings pointed to new model but sidecar was stuck on old one
- Fix: Save old model ID before settings update, roll back on sidecar failure
- Error messages now indicate whether rollback succeeded or failed
- Maintains consistency between settings and runtime state

**Parameter Name Mismatch for LLM Commands:**
- Issue: ModelList component invoked all commands with `{ modelName }` payload
- Whisper commands expect `model_name: String` (→ `modelName` in frontend)
- LLM commands expect `model_id: String` (→ `modelId` in frontend)
- All LLM operations (download, cancel, delete) would fail due to missing `model_id` parameter
- Fix: Added `invokeParamKey` prop to ModelList (defaults to "modelName")
- LLM usage passes `invokeParamKey="modelId"` to use correct parameter name
- Uses computed property syntax `{ [invokeParamKey]: modelName }` for dynamic keys

**Active Model Deletion UX Improvement:**
- Issue: ModelList allowed attempting to delete active model, which fails at backend
- Confusing UX: confirmation dialog → backend error
- Fix: Added `disableActiveModelDeletion` prop to ModelList (defaults to false)
- When enabled, Delete button is disabled for selected model with tooltip
- LLM models use `disableActiveModelDeletion={true}` for clearer UX
- Whisper models keep existing behavior (allows deletion with warning) for backward compatibility

### Next Steps

Phase 6 is complete. Ready to proceed to Phase 7 (Error Handling & Robustness) or Phase 8 (Integration Testing).

### Notes

- The UI follows the same pattern as Whisper model management for consistency
- Provider changes are immediate (no app restart required)
- Model switching uses hot-reload via `switch_llm_model` command
- Active model protection is enforced at the backend (delete command will fail)
- The ModelList component is now more flexible and can be reused for other model types
