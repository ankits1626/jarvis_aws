# Design: Transcribe Existing Recording Gems

## Overview

This feature enables users to generate accurate multilingual transcripts for existing recording gems using the MLX Omni transcription capability. Recording gems are audio recordings saved by Jarvis with real-time Whisper transcripts in the `content` field. The MLX Omni feature (implemented separately) can generate higher-quality transcripts by processing the full audio file through a multimodal model, but currently this only happens during the `enrich_gem` flow.

This design adds a dedicated "Transcribe" action that:
1. Fixes a critical bug in `extract_recording_path()` that prevents recording detection
2. Adds a new `transcribe_gem` Tauri command for standalone transcription with re-enrichment
3. Provides UI controls to transcribe and re-enrich individual gems based on the accurate MLX transcript
4. Displays transcript status and content in the gem list and expanded views

### Key Design Decisions

**Transcript-Based Re-enrichment**: When transcribing, the system also regenerates tags and summary based on the accurate MLX Omni transcript. This provides better metadata than the original Whisper real-time transcript, improving gem organization and searchability.

**Bug Fix First**: The `extract_recording_path()` function currently checks `source_type != "Recording"` which causes all recording gems to be skipped (they use `source_type: "Other"`). This must be fixed to enable both the new transcribe command and the existing enrich flow.

**Reuse Existing Infrastructure**: The `IntelProvider::generate_transcript()` method and MLX sidecar already work end-to-end. No sidecar changes needed.

**UI Clarity**: The interface clearly distinguishes between:
- Gems with no transcript (show Transcribe button)
- Gems with transcript (show language badge, no Transcribe button)
- MLX transcripts vs Whisper real-time transcripts (both displayed when available)

## Architecture

### Component Interaction

```
┌─────────────────┐
│   GemsPanel     │
│   (React UI)    │
└────────┬────────┘
         │ invoke('transcribe_gem')
         ▼
┌─────────────────┐
│  commands.rs    │
│ transcribe_gem()│
└────────┬────────┘
         │
         ├──► GemStore::get()
         │
         ├──► extract_recording_path() [FIXED]
         │
         ├──► IntelProvider::generate_transcript()
         │    └──► MlxProvider (via sidecar)
         │
         ├──► IntelProvider::generate_tags()
         │    └──► MlxProvider (via sidecar)
         │
         ├──► IntelProvider::summarize()
         │    └──► MlxProvider (via sidecar)
         │
         └──► GemStore::save()
```

### Data Flow

1. User clicks "Transcribe" button on a recording gem
2. Frontend calls `transcribe_gem(id)`
3. Backend fetches gem from store
4. Backend extracts recording path from `source_meta.recording_filename`
5. Backend calls `provider.generate_transcript(audio_path)`
6. MLX sidecar processes audio and returns `{language, transcript}`
7. Backend updates `gem.transcript` and `gem.transcript_language`
8. Backend calls `provider.generate_tags(transcript)` to generate tags from the transcript
9. Backend calls `provider.summarize(transcript)` to generate summary from the transcript
10. Backend updates `gem.ai_enrichment` with enrichment metadata (tags, summary, provider, timestamp)
11. Backend saves gem and returns updated version
12. Frontend updates local state to show transcript, tags, and summary

### Recording Gem Detection

Recording gems are identified by:
- `domain === "jarvis-app"` (frontend check)
- `source_meta` contains `recording_filename` (or fallback keys) (backend check)

**NOT** by `source_type` (which is "Other" for recordings, not "Recording").

## Components and Interfaces

### Backend: `extract_recording_path()` Fix

**Location**: `src-tauri/src/commands.rs` (lines 72-89)

**Current Bug**: Function checks `source_type != "Recording"` and returns `None` for all recording gems (which have `source_type: "Other"`).

**Fix**: Remove the `source_type` check entirely. Detect recording gems by presence of filename keys in `source_meta`.

**Signature**:
```rust
fn extract_recording_path(gem: &Gem) -> Option<PathBuf>
```

**Logic**:
1. Check `source_meta` for keys in priority order: `recording_filename`, `filename`, `recording_path`, `file`, `path`
2. If found, construct path: `dirs::data_dir()/com.jarvis.app/recordings/{filename}` (on macOS: `~/Library/Application Support/com.jarvis.app/recordings/{filename}`)
3. Return `Some(path)` if found, `None` otherwise

**Impact**: This fix enables both the new `transcribe_gem` command AND fixes the existing `enrich_gem` flow to generate transcripts when `transcription_engine == "mlx-omni"`.

### Backend: `enrich_content()` Flow Reordering

**Location**: `src-tauri/src/commands.rs` - `enrich_content()` function

**Change**: Reorder the enrichment steps to generate transcript BEFORE tags and summary, so that tags/summary can be based on the accurate MLX Omni transcript instead of the Whisper real-time content.

**Original Order**:
1. Generate tags from `content` (Whisper transcript)
2. Generate summary from `content` (Whisper transcript)
3. Generate transcript from audio file (if recording gem + MLX Omni enabled)

**New Order**:
1. Generate transcript from audio file (if recording gem + MLX Omni enabled)
2. Generate tags from `transcript` if available, otherwise from `content`
3. Generate summary from `transcript` if available, otherwise from `content`

**Rationale**: The MLX Omni transcript is more accurate than the Whisper real-time transcript, especially for multilingual content. By generating it first, we ensure that tags and summary are based on the best available text representation of the audio.

**Impact**: Recording gems enriched after this change will have better tags and summaries. Existing gems can use the `transcribe_gem` command to regenerate their metadata based on the MLX transcript.

**Note**: The `enrich_content()` function does not check provider availability for transcription. The caller (`enrich_gem` or `save_gem`) is responsible for verifying that the provider supports transcription before calling `enrich_content()`.

### Backend: `transcribe_gem` Command

**Location**: `src-tauri/src/commands.rs` (new function)

**Signature**:
```rust
#[tauri::command]
pub async fn transcribe_gem(
    id: String,
    gem_store: State<'_, Arc<dyn GemStore>>,
    intel_provider: State<'_, Arc<dyn IntelProvider>>,
    settings_manager: State<'_, Arc<RwLock<SettingsManager>>>,
) -> Result<Gem, String>
```

**Logic**:
0. Check provider availability for transcription support
1. Fetch gem by ID from store
2. Extract recording path using `extract_recording_path()`
3. Verify recording file exists on disk
4. Call `provider.generate_transcript(audio_path)`
5. Update `gem.transcript` and `gem.transcript_language`
6. Generate tags from transcript using `provider.generate_tags()`
7. Generate summary from transcript using `provider.summarize()`
8. Update `gem.ai_enrichment` with enrichment metadata (source, timestamp)
9. Save updated gem
10. Return updated gem

**Error Handling**:
- Gem not found: `"Gem with id '{id}' not found"`
- No recording metadata: `"This gem has no associated recording file"`
- File not found: `"Recording file not found: {path}"`
- Provider doesn't support transcription: `"Current AI provider does not support transcription"`
- Transcription failure: Forward error from provider

**Constraints**:
- Updates `transcript`, `transcript_language`, `tags`, `summary`, and `ai_enrichment`
- Tags and summary are generated from the MLX Omni transcript (not the Whisper content)
- Uses 120s timeout for transcription (inherited from `MlxProvider::generate_transcript_internal()`)
- Tag generation and summarization use standard timeouts from the provider

### Backend: Command Registration

**Location**: `src-tauri/src/lib.rs`

Add `transcribe_gem` to the `invoke_handler!` macro alongside existing commands:

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    enrich_gem,
    transcribe_gem,  // NEW
    // ... more commands ...
])
```

### Frontend: Transcribe Button

**Location**: `src/components/GemsPanel.tsx` (gem actions section, around lines 248-303)

**Visibility Logic**:
```typescript
const showTranscribeButton = 
  gem.domain === 'jarvis-app' &&           // Is recording gem
  gem.transcript_language === null &&      // No transcript yet
  aiAvailable;                             // AI provider available
```

**Button Rendering**:
```tsx
{showTranscribeButton && (
  <button
    onClick={handleTranscribe}
    className="gem-enrich-button"
    disabled={transcribing}
    title="Transcribe recording"
  >
    {transcribing ? '...' : 'Transcribe'}
  </button>
)}
```

**Handler Logic**:
```typescript
const handleTranscribe = async () => {
  setTranscribing(true);
  setTranscribeError(null);
  try {
    const updatedGem = await invoke<Gem>('transcribe_gem', { id: gem.id });
    // Update local state with transcript, tags, summary, and enrichment data
    // Extract provider and model from ai_enrichment to construct enrichment_source
    const provider = updatedGem.ai_enrichment?.provider;
    const model = updatedGem.ai_enrichment?.model;
    const source = provider && model ? `${provider} / ${model}` : provider || null;
    
    setLocalGem({
      ...localGem,
      transcript_language: updatedGem.transcript_language,
      tags: updatedGem.ai_enrichment?.tags || localGem.tags,
      summary: updatedGem.ai_enrichment?.summary || localGem.summary,
      enrichment_source: source || localGem.enrichment_source,
    });
    // Update fullGem cache if expanded
    if (fullGem) {
      setFullGem(updatedGem);
    }
  } catch (err) {
    setTranscribeError(String(err));
  } finally {
    setTranscribing(false);
  }
};
```

### Frontend: Transcript Status Badge

**Location**: `src/components/GemsPanel.tsx` (gem card header/metadata area)

**Visibility Logic**:
```typescript
const showTranscriptBadge = 
  gem.domain === 'jarvis-app' &&           // Is recording gem
  gem.transcript_language !== null;        // Has transcript
```

**Badge Rendering**:
```tsx
{showTranscriptBadge && (
  <span className="gem-lang-badge" title="Transcript available">
    {gem.transcript_language}
  </span>
)}
```

**Styling**: Small badge similar to source type badge, positioned near gem title or in metadata row.

### Frontend: Expanded Transcript Display

**Location**: `src/components/GemsPanel.tsx` (expanded gem content section, around lines 145-245)

**Current Implementation**: Already displays MLX transcript prominently and Whisper transcript in secondary section (lines 230-245).

**Required Changes**: None. The existing implementation already handles the requirements:
- Shows MLX transcript with language label when `fullGem.transcript` exists
- Shows Whisper transcript in collapsed section when both exist
- Shows Whisper transcript normally when only `fullGem.content` exists

**Verification**: Ensure `get_gem` command returns full `transcript` field (it does - `Gem` type includes it).

## Data Models

### Gem (Full)

```rust
pub struct Gem {
    pub id: String,
    pub source_type: String,
    pub source_url: String,
    pub domain: String,
    pub title: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,              // Whisper real-time transcript
    pub source_meta: serde_json::Value,       // Contains recording_filename
    pub captured_at: String,
    pub ai_enrichment: Option<serde_json::Value>,
    pub transcript: Option<String>,           // MLX Omni transcript (NEW USAGE)
    pub transcript_language: Option<String>,  // Human-readable language name (NEW USAGE)
}
```

### GemPreview (List View)

```rust
pub struct GemPreview {
    pub id: String,
    pub source_type: String,
    pub source_url: String,
    pub domain: String,
    pub title: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub content_preview: Option<String>,
    pub captured_at: String,
    pub tags: Option<Vec<String>>,
    pub summary: Option<String>,
    pub enrichment_source: Option<String>,
    pub transcript_language: Option<String>,  // Available in preview (no full transcript)
}
```

### TranscriptResult

```rust
pub struct TranscriptResult {
    pub language: String,    // Human-readable language name (e.g., "English", "Chinese", "Hindi")
    pub transcript: String,  // Full transcript text
}
```

### Recording Metadata in source_meta

```json
{
  "recording_filename": "20240315_143022.pcm"
}
```

**Fallback Keys** (checked in order): `filename`, `recording_path`, `file`, `path`

**Path Construction**: `dirs::data_dir()/com.jarvis.app/recordings/{recording_filename}` (on macOS: `~/Library/Application Support/com.jarvis.app/recordings/{recording_filename}`)

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property Reflection

After analyzing the acceptance criteria, I identified the following testable properties. I reviewed them for redundancy:

- Properties 1.1, 1.2, 1.3 all test `extract_recording_path()` behavior and can be combined into comprehensive properties
- Properties 3.1, 3.2, 3.4, 3.5 all test button visibility and can be combined
- Properties 4.1, 4.2 test badge visibility and can be combined
- Property 2.6 is a unique invariant about field preservation
- Properties 5.1, 5.2, 5.3 test different display scenarios and should remain separate

### Property 1: Recording Path Extraction from Metadata

*For any* gem with a recording filename key (`recording_filename`, `filename`, `recording_path`, `file`, or `path`) in `source_meta`, `extract_recording_path()` should return `Some(dirs::data_dir()/com.jarvis.app/recordings/{filename})` regardless of the gem's `source_type` value.

**Validates: Requirements 1.1, 1.2**

### Property 2: Recording Path Extraction Returns None for Non-Recordings

*For any* gem without any recording filename keys in `source_meta`, `extract_recording_path()` should return `None`.

**Validates: Requirements 1.3**

### Property 3: Transcription Updates Only Expected Fields

*For any* gem before and after calling `transcribe_gem`, only the following fields should change: `transcript`, `transcript_language`, `tags`, `summary`, and `ai_enrichment`. All other fields (id, source_type, source_url, domain, title, author, description, content, source_meta, captured_at) should remain unchanged.

**Validates: Requirements 2.6 (modified)**

### Property 4: Transcribe Button Visibility

*For any* recording gem (domain === "jarvis-app") without a transcript (transcript_language === null) when AI is available, the Transcribe button should be visible regardless of whether tags or summary exist.

**Validates: Requirements 3.1, 3.4, 3.5**

### Property 5: Transcribe Button Hidden When Transcript Exists

*For any* gem with a non-null `transcript_language`, the Transcribe button should not be visible.

**Validates: Requirements 3.2**

### Property 6: Transcript Badge Visibility

*For any* recording gem (domain === "jarvis-app") with a non-null `transcript_language`, a language badge should be displayed; for recording gems without transcript_language, no badge should be shown.

**Validates: Requirements 4.1, 4.2**

### Property 7: MLX Transcript Display When Expanded

*For any* expanded gem with a non-null `transcript` field, the transcript should be displayed in a labeled section with the language from `transcript_language`.

**Validates: Requirements 5.1**

## Error Handling

### Backend Errors

**Provider Unavailable**:
- Condition: `provider.check_availability()` returns `available: false`
- Response: `Err(format!("AI provider not available: {}", availability.reason.unwrap_or_else(|| "Unknown reason".to_string())))`
- User action: Switch to MLX provider in settings or check provider configuration

**Gem Not Found**:
- Condition: `gem_store.get(id)` returns `None`
- Response: `Err("Gem with id '{id}' not found")`
- HTTP equivalent: 404 Not Found

**No Recording Metadata**:
- Condition: `extract_recording_path()` returns `None`
- Response: `Err("This gem has no associated recording file")`
- User action: This gem is not a recording, cannot transcribe

**Recording File Not Found**:
- Condition: Recording path exists in metadata but file doesn't exist on disk
- Response: `Err("Recording file not found: {path}")`
- User action: Recording file may have been deleted, cannot transcribe

**Provider Doesn't Support Transcription**:
- Condition: Active provider is IntelligenceKit or NoOp (not MlxProvider)
- Response: `Err("Current AI provider does not support transcription")`
- User action: Switch to MLX provider in settings

**Transcription Failed**:
- Condition: `provider.generate_transcript()` returns error
- Response: Forward error from provider (e.g., "Model does not support audio", "Sidecar crashed")
- User action: Check model capabilities, restart sidecar, check logs

**Tag/Summary Generation Failed**:
- Condition: `provider.generate_tags()` or `provider.summarize()` returns error after successful transcription
- Response: Graceful degradation - transcript is saved, but tags/summary remain empty
- Behavior: Uses `unwrap_or_default()` to silently proceed with empty tags/summary
- User action: Transcript is still available; tags/summary can be regenerated by re-enriching the gem

**Sidecar Crash**:
- Condition: MLX sidecar process terminates during transcription
- Response: Error from provider (e.g., "broken pipe", "closed connection")
- Frontend: Show error in gem card, suggest checking settings
- Note: `enrich_gem` already emits `mlx-sidecar-error` event for toast notification

### Frontend Error Display

**Transcription Error**:
- Display error message in gem card below actions
- Keep Transcribe button enabled for retry
- Clear error on successful retry

**Loading States**:
- Disable Transcribe button during transcription
- Show "..." text in button
- Show status banner message "Transcribing audio..."

### Timeout Handling

**Transcription Timeout**: 120 seconds (inherited from `MlxProvider::generate_transcript_internal()`)
- Long audio files may take significant time to process
- Timeout prevents indefinite hangs
- Error message: "Transcription timed out after 120 seconds"

## Testing Strategy

### Dual Testing Approach

This feature requires both unit tests and property-based tests for comprehensive coverage:

**Unit Tests**: Verify specific examples, edge cases, and error conditions
- Specific error messages for each failure mode
- Integration between command and store/provider
- UI component rendering for specific gem states

**Property Tests**: Verify universal properties across all inputs
- Path extraction logic for various source_meta configurations
- Field preservation during transcription
- Button visibility rules for various gem states

Together, unit tests catch concrete bugs while property tests verify general correctness.

### Property-Based Testing

**Library**: Use `proptest` crate for Rust property tests

**Configuration**: Minimum 100 iterations per test (due to randomization)

**Test Tags**: Each property test must reference its design property:
```rust
// Feature: transcribe-existing-gems, Property 1: Recording Path Extraction from Metadata
#[test]
fn prop_extract_recording_path_with_metadata() { ... }
```

### Backend Unit Tests

**File**: `src-tauri/src/commands.rs` (tests module)

**Test Cases**:
1. `test_extract_recording_path_with_recording_filename` - Primary key
2. `test_extract_recording_path_with_fallback_keys` - filename, recording_path, file, path
3. `test_extract_recording_path_without_metadata` - Returns None
4. `test_extract_recording_path_ignores_source_type` - Works regardless of source_type
5. `test_transcribe_gem_success` - Happy path with mock provider
6. `test_transcribe_gem_not_found` - Gem doesn't exist
7. `test_transcribe_gem_no_recording` - Gem has no recording metadata
8. `test_transcribe_gem_file_not_found` - Recording file missing
9. `test_transcribe_gem_provider_unsupported` - Provider doesn't support transcription
10. `test_transcribe_gem_updates_expected_fields` - Only transcript, tags, summary, ai_enrichment change
11. `test_transcribe_gem_generates_tags_from_transcript` - Tags generated from MLX transcript
12. `test_transcribe_gem_generates_summary_from_transcript` - Summary generated from MLX transcript

### Backend Property Tests

**File**: `src-tauri/src/commands.rs` (tests module)

**Property Tests**:
1. **Property 1**: For any gem with recording filename keys, path extraction succeeds
2. **Property 2**: For any gem without recording filename keys, path extraction returns None
3. **Property 3**: For any gem, transcription only modifies transcript, tags, summary, and ai_enrichment fields

**Generators**:
- `arb_gem_with_recording()` - Generates gems with recording metadata
- `arb_gem_without_recording()` - Generates gems without recording metadata
- `arb_source_meta_with_filename()` - Generates source_meta with various filename keys
- `arb_source_meta_without_filename()` - Generates source_meta without filename keys

### Frontend Unit Tests

**File**: `src/components/GemsPanel.test.tsx`

**Test Cases**:
1. `test_transcribe_button_visible_for_recording_without_transcript` - Button shows
2. `test_transcribe_button_hidden_for_recording_with_transcript` - Button hidden
3. `test_transcribe_button_hidden_when_ai_unavailable` - Button hidden
4. `test_transcribe_button_visible_with_existing_tags` - Shows even if enriched
5. `test_transcript_badge_visible_with_language` - Badge shows
6. `test_transcript_badge_hidden_without_language` - Badge hidden
7. `test_transcribe_button_click_calls_command` - Invokes transcribe_gem
8. `test_transcribe_button_shows_loading_state` - Disabled during transcription
9. `test_transcribe_error_displayed` - Error message shown
10. `test_expanded_gem_shows_mlx_transcript` - Transcript displayed when expanded

### Frontend Property Tests

**Library**: Use `@fast-check/vitest` for TypeScript property tests

**Property Tests**:
1. **Property 4**: For any recording gem without transcript when AI available, button visible
2. **Property 5**: For any gem with transcript, button hidden
3. **Property 6**: For any recording gem, badge visibility matches transcript_language presence
4. **Property 7**: For any expanded gem with transcript, transcript section visible

**Generators**:
- `arbRecordingGem()` - Generates recording gems (domain: "jarvis-app")
- `arbGemWithTranscript()` - Generates gems with transcript_language
- `arbGemWithoutTranscript()` - Generates gems without transcript_language

### Integration Tests

**Manual Testing Scenarios**:
1. Transcribe a recording gem without transcript → Success, tags/summary generated
2. Transcribe a recording gem with existing tags/summary → Transcript, tags, and summary regenerated from MLX transcript
3. Attempt to transcribe non-recording gem → Error displayed
4. Attempt to transcribe with IntelligenceKit provider → Error displayed
5. Transcribe with missing recording file → Error displayed
6. View expanded gem with both MLX and Whisper transcripts → Both displayed correctly
7. Filter gems by tag after transcription → New tags are searchable and filterable

**End-to-End Test**:
1. Create a recording gem (via recording flow)
2. Verify Transcribe button appears
3. Click Transcribe button
4. Wait for transcription to complete
5. Verify transcript badge appears
6. Verify Transcribe button disappears
7. Expand gem and verify transcript displayed
8. Verify Whisper transcript still visible in secondary section

### Test Data

**Sample Recording Filenames**:
- `20240315_143022.pcm`
- `recording_2024-03-15_14-30-22.pcm`
- `test.pcm`

**Sample Languages**:
- `English`
- `Chinese`
- `Hindi`
- `Spanish`
- `French`

**Sample Transcripts**:
- Short: "Hello, how are you?"
- Medium: 100-500 words
- Long: 1000+ words
- Multilingual: Mixed language content

### Performance Testing

**Transcription Duration**:
- Short audio (< 1 min): Should complete in < 10 seconds
- Medium audio (1-5 min): Should complete in < 30 seconds
- Long audio (5-10 min): Should complete in < 60 seconds
- Very long audio (> 10 min): May approach 120s timeout

**UI Responsiveness**:
- Button click should show loading state immediately (< 100ms)
- Error messages should appear immediately after failure
- Transcript should display immediately after expansion (already cached)

### Error Recovery Testing

**Sidecar Crash During Transcription**:
1. Start transcription
2. Kill MLX sidecar process
3. Verify error displayed in UI
4. Verify gem state unchanged
5. Restart sidecar
6. Retry transcription → Success

**Network/Disk Issues**:
1. Make recording file read-only
2. Attempt transcription
3. Verify appropriate error message
4. Restore permissions
5. Retry → Success
