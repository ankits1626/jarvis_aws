# Requirements: Transcribe Recordings from Recordings List

## Introduction

Jarvis saves audio recordings as raw PCM files in `~/Library/Application Support/com.jarvis.app/recordings/`. Currently, recordings can only become gems through the `TranscriptDisplay` component's "Save Gem" button, which is available only during/after the live recording session. If a user forgets to click "Save Gem" before navigating away, the recording file persists but has no gem -- and there is no way to transcribe or save it later.

The existing `transcribe_gem` command requires a gem ID, meaning it operates only on recordings that were already saved as gems. There is no pathway to transcribe a raw recording file that lacks a gem.

This feature adds a two-step flow directly on the recordings list: first transcribe the raw audio file (displaying the transcript inline), then optionally save it as a gem (or update an existing gem if one was previously created for that recording).

### Key Architecture Context

- **Recordings** are raw PCM files managed by `FileManager`, listed via `list_recordings` command, each identified by `RecordingMetadata { filename, size_bytes, created_at, duration_seconds }`.
- **Recording gems** are identified by `domain === "jarvis-app"` and `source_meta.recording_filename` containing the PCM filename.
- **Recording path resolution**: `dirs::data_dir()/com.jarvis.app/recordings/{filename}` (on macOS: `~/Library/Application Support/com.jarvis.app/recordings/`) -- already implemented in `extract_recording_path()` in `commands.rs`.
- **`IntelProvider::generate_transcript(audio_path)`** exists on `MlxProvider` and returns `TranscriptResult { language, transcript }`. Other providers return "not supported" errors.
- **`GemStore` trait** currently has no method to query gems by recording filename. The only unique lookup is by `id` (UUID) or upsert key `source_url`.
- **Recordings list** is rendered inline in `App.tsx` (lines 346-385), not using the extracted `RecordingRow.tsx` / `RecordingsList.tsx` components.
- **Existing gem creation** via `TranscriptDisplay.tsx` uses `jarvis://recording/${Date.now()}` as `source_url` (timestamp-based, non-deterministic).

### Goals

1. Users can transcribe any raw recording from the recordings list with a single click, without needing an existing gem.
2. After transcription, users can review the transcript and save the recording as a gem (with the transcript already populated) or update an existing gem's transcript.
3. The system avoids creating duplicate gems when a gem already exists for a recording.
4. The recordings list visually indicates which recordings already have associated gems.

---

## Requirements

### Requirement 1: New `transcribe_recording` Tauri Command

**User Story:** As a user, I want to transcribe any raw recording from my recordings list without first creating a gem, so that I can review the transcript before deciding whether to save it.

#### Acceptance Criteria

1. THE SYSTEM SHALL expose a new Tauri command `transcribe_recording(filename: String)` that generates a transcript directly from a recording file.
2. WHEN `transcribe_recording` is called THEN THE SYSTEM SHALL:
   - Construct the full recording path as `dirs::data_dir()/com.jarvis.app/recordings/{filename}`
   - Verify the `IntelProvider` is available via `check_availability()`
   - Verify the recording file exists on disk
   - Call `provider.generate_transcript(recording_path)` on the active `IntelProvider`
   - Return `TranscriptResult { language, transcript }` directly to the frontend
3. THE SYSTEM SHALL NOT create, modify, or interact with any gem -- this command performs only transcription.
4. WHEN the `IntelProvider` is not available THEN THE SYSTEM SHALL return `Err("AI provider not available: {reason}")`.
5. WHEN the recording file does not exist on disk THEN THE SYSTEM SHALL return `Err("Recording file not found: {path}")`.
6. WHEN the `IntelProvider` does not support transcription (e.g., `IntelligenceKitProvider`, `NoOpProvider`) THEN THE SYSTEM SHALL return `Err("Current AI provider does not support transcription")`.
7. THE SYSTEM SHALL register `transcribe_recording` in `lib.rs` alongside existing commands.

**File:** `src-tauri/src/commands.rs` -- new command; `src-tauri/src/lib.rs` -- register command

---

### Requirement 2: New `find_by_recording_filename` GemStore Method and `check_recording_gem` Command

**User Story:** As a user, I want the system to know whether a gem already exists for a given recording, so that duplicate gems are not created and I can see which recordings are already saved.

#### Acceptance Criteria

1. THE SYSTEM SHALL add a new method to the `GemStore` trait:
   ```rust
   async fn find_by_recording_filename(&self, filename: &str) -> Result<Option<GemPreview>, String>;
   ```
2. THE `SqliteGemStore` implementation SHALL execute a query using `json_extract(source_meta, '$.recording_filename') = ?1` and return the first matching row as a `GemPreview`, or `None` if no match.
3. WHEN multiple gems exist for the same recording filename (edge case) THEN THE SYSTEM SHALL return the most recently captured one (`ORDER BY captured_at DESC LIMIT 1`).
4. THE SYSTEM SHALL expose a new Tauri command `check_recording_gem(filename: String)` that calls `gem_store.find_by_recording_filename(&filename)` and returns `Option<GemPreview>`.
5. THE SYSTEM SHALL register `check_recording_gem` in `lib.rs`.

**Files:** `src-tauri/src/gems/store.rs` -- add trait method; `src-tauri/src/gems/sqlite_store.rs` -- add implementation; `src-tauri/src/commands.rs` -- new command; `src-tauri/src/lib.rs` -- register command

---

### Requirement 3: New `save_recording_gem` Tauri Command

**User Story:** As a user, I want to save a transcribed recording as a gem (or update an existing gem) with the transcript already populated, so that I don't have to re-transcribe and I avoid creating duplicate gems.

#### Acceptance Criteria

1. THE SYSTEM SHALL expose a new Tauri command `save_recording_gem(filename: String, transcript: String, language: String)` that creates or updates a gem for the specified recording.
2. WHEN `save_recording_gem` is called THEN THE SYSTEM SHALL first call `gem_store.find_by_recording_filename(&filename)` to check for an existing gem.
3. WHEN a gem already exists for the recording THEN THE SYSTEM SHALL:
   - Fetch the full gem via `gem_store.get(&id)`
   - Update `gem.transcript` and `gem.transcript_language` with the provided values
   - Regenerate `tags` and `summary` from the transcript using the `IntelProvider`
   - Update `gem.ai_enrichment` with enrichment metadata
   - Save via `gem_store.save(gem)`
4. WHEN no gem exists for the recording THEN THE SYSTEM SHALL create a new `Gem`:
   - `id`: new UUID v4
   - `source_url`: `jarvis://recording/{filename}` (deterministic, using filename for idempotent upsert)
   - `domain`: `"jarvis-app"`
   - `source_type`: `"Other"`
   - `title`: `"Audio Transcript - {formatted_timestamp}"` (derived from recording `created_at` or filename timestamp)
   - `content`: `None` (no Whisper real-time transcript in this flow)
   - `source_meta`: `{ "recording_filename": "{filename}", "source": "recording_transcription" }`
   - `transcript`: the provided transcript string
   - `transcript_language`: the provided language string
   - Generate tags and summary from the transcript, populate `ai_enrichment`
   - Save via `gem_store.save(gem)`
5. THE SYSTEM SHALL return the saved/updated `Gem` to the frontend.
6. WHEN the `IntelProvider` is not available for tags/summary generation THEN THE SYSTEM SHALL still save the gem with `transcript` and `transcript_language` populated but `ai_enrichment` set to `None` (graceful degradation).
7. THE SYSTEM SHALL register `save_recording_gem` in `lib.rs`.

**File:** `src-tauri/src/commands.rs` -- new command; `src-tauri/src/lib.rs` -- register command

---

### Requirement 4: UI -- Transcribe Button on Recordings List

**User Story:** As a user, I want to see a "Transcribe" button on each recording in the recordings list, so that I can generate a transcript for any recording without needing to save it as a gem first.

#### Acceptance Criteria

1. WHEN AI is available (`check_intel_availability` returns `available: true`) THEN THE SYSTEM SHALL show a "Transcribe" button on each recording row in the recordings list.
2. WHEN AI is not available THEN THE SYSTEM SHALL hide the Transcribe button or show it disabled with a tooltip explaining why.
3. WHEN the user clicks "Transcribe" on a recording THEN THE SYSTEM SHALL:
   - Call `invoke('transcribe_recording', { filename: recording.filename })`
   - Show a loading/spinner state on the button during transcription
   - On success, display the transcript text inline below the recording row
   - On failure, display the error message inline below the recording row
4. THE SYSTEM SHALL check AI availability on mount (once, when the recordings list renders) and cache the result.
5. THE SYSTEM SHALL disable the Transcribe button on the active recording while transcription is in progress to prevent concurrent operations.
6. WHEN a recording already has a visible transcript (from the current session) THEN THE SYSTEM SHALL keep the Transcribe button active for re-transcription.

**File:** `src/App.tsx` -- modify recordings list section (lines 346-385); `src/state/types.ts` -- add `TranscriptResult` TypeScript interface if not present

---

### Requirement 5: UI -- Save as Gem / Update Gem Button After Transcription

**User Story:** As a user, after seeing the transcript for a recording, I want to save it as a gem or update an existing gem, so that the transcript is persisted in my knowledge base.

#### Acceptance Criteria

1. WHEN a recording has been successfully transcribed (transcript is displayed inline) THEN THE SYSTEM SHALL show a "Save as Gem" button below the transcript.
2. WHEN a gem already exists for the recording (determined by calling `check_recording_gem(filename)`) THEN THE SYSTEM SHALL show "Update Gem" instead of "Save as Gem".
3. WHEN the user clicks "Save as Gem" or "Update Gem" THEN THE SYSTEM SHALL:
   - Call `invoke('save_recording_gem', { filename, transcript, language })` with the transcript data from the transcription step
   - Show a loading state on the button during save
   - On success, show a success indicator (e.g., checkmark, "Saved" text)
   - On failure, show the error message inline
4. AFTER a successful save THEN THE SYSTEM SHALL update the gem status indicator on the recording row (see Requirement 6).
5. THE SYSTEM SHALL call `check_recording_gem(filename)` after transcription completes to determine the correct button label ("Save as Gem" vs "Update Gem").

**File:** `src/App.tsx` -- modify recordings list section

---

### Requirement 6: UI -- Gem Status Indicator on Recordings

**User Story:** As a user, I want to see at a glance which of my recordings already have associated gems, so that I know which ones need attention.

#### Acceptance Criteria

1. WHEN the recordings list is rendered THEN THE SYSTEM SHALL check which recordings have associated gems.
2. WHEN a recording has an associated gem THEN THE SYSTEM SHALL display a small indicator (e.g., gem icon, colored dot, or badge) on the recording row.
3. WHEN a recording does not have an associated gem THEN THE SYSTEM SHALL NOT display any gem indicator.
4. THE SYSTEM SHALL batch gem status checks to avoid excessive backend calls. Options include: a single `check_recording_gem` call per recording on mount, or a dedicated batch command `check_recording_gems_batch(filenames: Vec<String>)`.
5. THE SYSTEM SHALL update the indicator when a gem is saved or updated (after Requirement 5 actions).
6. WHEN a gem is deleted from the Gems panel THEN the indicator SHALL be updated on next recordings list render (no real-time sync required).

**File:** `src/App.tsx` -- modify recordings list section; optionally `src-tauri/src/commands.rs` for batch command

---

## Technical Constraints

1. **Recording file path**: Always resolved as `dirs::data_dir()/com.jarvis.app/recordings/{filename}`. On macOS this is `~/Library/Application Support/com.jarvis.app/recordings/`.
2. **Transcription timeout**: 120 seconds, inherited from `MlxProvider::generate_transcript_internal()`. Frontend should handle long waits gracefully.
3. **Model requirement**: The active MLX model must support audio (have `"audio"` in capabilities). Text-only models will fail transcription with a clear error.
4. **GemStore trait extension**: Adding `find_by_recording_filename` is a breaking change for trait implementors. Both `SqliteGemStore` and any test mocks must be updated.
5. **Source URL uniqueness**: The `gems` table has a `UNIQUE` constraint on `source_url`. Using `jarvis://recording/{filename}` (not timestamp) ensures deterministic URLs and idempotent upsert behavior.
6. **Existing gems have different source_url format**: Gems created via `TranscriptDisplay.tsx` use `jarvis://recording/{Date.now()}`. The `find_by_recording_filename` query searches `source_meta.recording_filename`, not `source_url`, so it finds these gems correctly regardless of URL format.
7. **No state sharing between panels**: The recordings list (main view) and GemsPanel are separate with independent state. Changes in one are reflected in the other only on re-render.

---

## Out of Scope

- Batch transcription of all recordings at once
- Auto-transcription when a recording finishes (live Whisper already handles this)
- Streaming transcript progress to the UI (transcription returns as a single result)
- Deleting recordings when a gem is deleted (or vice versa)
- Real-time sync between recordings list gem indicators and GemsPanel actions
- Playback of audio alongside the transcript in the expanded section (playback already exists via the audio player)
- Refactoring to use `RecordingRow.tsx` / `RecordingsList.tsx` extracted components (can be done separately)
