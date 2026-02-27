# Requirements: Transcribe Existing Recording Gems

## Introduction

Jarvis saves audio recordings as "gems" with a real-time Whisper transcript in the `content` field and the PCM recording filename in `source_meta.recording_filename`. The MLX Omni transcription feature (added separately) can generate accurate multilingual transcripts by processing the full audio file through a multimodal model — but currently this only happens during the `enrich_gem` flow, which also generates tags and summary.

Users need a way to **transcribe existing recording gems** that were saved before MLX Omni was set up, or gems where enrichment was done with a text-only model (so tags/summary exist but no transcript). This requires a dedicated "Transcribe" action separate from full enrichment.

### Key Architecture Context

- **Recording gems** are identified by `domain === "jarvis-app"` and `source_meta.recording_filename` being present.
- **Gems are saved with `source_type: "Other"`**, not `"Recording"`. The current `extract_recording_path()` in `commands.rs` checks `source_type != "Recording"` which means it **never finds recordings** — this is a critical bug that must be fixed.
- **`source_meta.recording_filename`** stores just the filename (e.g., `"20240315_143022.pcm"`). Full path is `~/.jarvis/recordings/{filename}`.
- **`IntelProvider::generate_transcript(audio_path)`** already exists on `MlxProvider` and works end-to-end.
- **`server.py` `generate_transcript()`** uses `apply_chat_template()` → token IDs → `mlx_lm.generate()` (fixed in the MLX Omni work).
- **Gem fields**: `transcript: Option<String>` and `transcript_language: Option<String>` already exist in the schema and model.
- **`GemPreview`** (used in list view) includes `transcript_language` but NOT `transcript` (full text).

### Goals

1. Users can transcribe any existing recording gem with a single click.
2. Transcription is independent from enrichment — no need to regenerate tags/summary.
3. The UI clearly shows which recording gems have/don't have transcripts.
4. The critical `extract_recording_path()` bug is fixed so enrichment also generates transcripts.

---

## Requirements

### Requirement 1: Fix `extract_recording_path()` Detection Logic

**User Story:** As a developer, I want recording gems to be correctly identified by their `source_meta` fields rather than `source_type`, so that transcription works for all recording gems.

#### Acceptance Criteria

1. THE SYSTEM SHALL detect recording gems by checking for the presence of `recording_filename` (or fallback keys: `filename`, `recording_path`, `file`, `path`) in `source_meta`, instead of checking `source_type == "Recording"`.
2. WHEN `source_meta` contains a valid recording filename THEN `extract_recording_path()` SHALL return `Some(~/.jarvis/recordings/{filename})`.
3. WHEN `source_meta` does not contain any recording filename key THEN `extract_recording_path()` SHALL return `None`.
4. THE SYSTEM SHALL remove the `source_type != "Recording"` guard that currently causes all recording gems to be skipped.

**File:** `src-tauri/src/commands.rs` — `extract_recording_path()` (lines 72-89)

---

### Requirement 2: New `transcribe_gem` Tauri Command

**User Story:** As a user, I want to generate a transcript for a specific recording gem without re-running full enrichment, so that I can transcribe old recordings quickly.

#### Acceptance Criteria

1. THE SYSTEM SHALL expose a new Tauri command `transcribe_gem(id: String)` that generates a transcript for the specified gem.
2. WHEN `transcribe_gem` is called THEN THE SYSTEM SHALL:
   - Fetch the gem by ID from the store
   - Extract the recording path from `source_meta`
   - Call `provider.generate_transcript(audio_path)` on the active `IntelProvider`
   - Update `gem.transcript` and `gem.transcript_language` with the result
   - Save the updated gem and return it
3. WHEN the gem has no recording file path in `source_meta` THEN THE SYSTEM SHALL return `Err("This gem has no associated recording file")`.
4. WHEN the recording file does not exist on disk THEN THE SYSTEM SHALL return `Err("Recording file not found: {path}")`.
5. WHEN the `IntelProvider` does not support transcription (e.g., IntelligenceKit, NoOp) THEN THE SYSTEM SHALL return `Err("Current AI provider does not support transcription")`.
6. THE SYSTEM SHALL NOT modify `ai_enrichment`, `tags`, or `summary` — only `transcript` and `transcript_language` are updated.
7. THE SYSTEM SHALL register `transcribe_gem` in `lib.rs` alongside the existing commands.

**File:** `src-tauri/src/commands.rs` — new command; `src-tauri/src/lib.rs` — register command

---

### Requirement 3: "Transcribe" Button in Gem List UI

**User Story:** As a user, I want to see a "Transcribe" button on recording gems that don't have a transcript yet, so that I can generate accurate transcripts for my existing recordings.

#### Acceptance Criteria

1. WHEN a gem is a recording gem (`domain === "jarvis-app"`) AND has no transcript (`transcript_language === null`) THEN THE SYSTEM SHALL show a "Transcribe" button (🎙️ icon) in the gem actions row.
2. WHEN a gem already has a transcript (`transcript_language !== null`) THEN THE SYSTEM SHALL NOT show the Transcribe button (transcription is already done).
3. WHEN the user clicks "Transcribe" THEN THE SYSTEM SHALL:
   - Call `invoke('transcribe_gem', { id: gem.id })`
   - Show a loading state on the button while transcription is in progress
   - On success, update the local gem state with the returned transcript data
   - On failure, show the error message in the gem card
4. THE SYSTEM SHALL only show the Transcribe button when AI is available (`aiAvailable === true`).
5. THE SYSTEM SHALL show the Transcribe button regardless of whether tags/summary already exist — transcription is independent from enrichment.

**File:** `src/components/GemsPanel.tsx` — gem actions section (around lines 248-303)

---

### Requirement 4: Show Transcript Status in Gem Preview

**User Story:** As a user, I want to see at a glance which recording gems have transcripts, so that I know which ones still need transcription.

#### Acceptance Criteria

1. WHEN a recording gem has `transcript_language !== null` THEN THE SYSTEM SHALL show a small language badge (e.g., "Hindi", "English") near the gem title or metadata area.
2. WHEN a recording gem has no transcript THEN THE SYSTEM SHALL NOT show any transcript indicator — the Transcribe button serves as the call-to-action.
3. THE SYSTEM SHALL use the existing `transcript_language` field from `GemPreview` (already available in the list response).

**File:** `src/components/GemsPanel.tsx` — gem card display

---

### Requirement 5: Display Transcript in Expanded Gem View

**User Story:** As a user, I want to read the full transcript when I expand a recording gem, so that I can see what was said.

#### Acceptance Criteria

1. WHEN a gem is expanded AND has a non-null `transcript` field THEN THE SYSTEM SHALL display the transcript in a labeled section: "Transcript ({language})" where language comes from `transcript_language`.
2. WHEN a gem has both a Whisper real-time transcript (`content`) and an MLX transcript (`transcript`) THEN THE SYSTEM SHALL show the MLX transcript prominently and the Whisper transcript in a collapsed/secondary section labeled "Real-time transcript (Whisper)".
3. WHEN a gem has only a Whisper transcript (`content` present, `transcript` null) THEN THE SYSTEM SHALL show the content normally as it does today.
4. THE SYSTEM SHALL fetch the full `transcript` field via the existing `get_gem` command when expanding (it's on the full `Gem` type, not `GemPreview`).

**File:** `src/components/GemsPanel.tsx` — expanded gem content section (around lines 145-245)

---

## Technical Constraints

1. **Reuse existing infrastructure**: The `IntelProvider::generate_transcript()` method and `server.py` `generate_transcript` command already work. No sidecar changes needed.
2. **No enrichment side effects**: `transcribe_gem` must NOT call `generate_tags()` or `summarize()`. It only updates transcript fields.
3. **Recording file path**: Always resolved as `~/.jarvis/recordings/{source_meta.recording_filename}`.
4. **Timeout**: Transcription uses 120s timeout (inherited from `MlxProvider::generate_transcript_internal()`).
5. **Model requirement**: The active MLX model must support audio (have `"audio"` in capabilities). If a text-only model is loaded, transcription will fail gracefully.

---

## Out of Scope

- Batch transcription of all recording gems at once
- Re-transcription (overwriting existing transcripts) — user can use Enrich for that
- Transcription progress/streaming to the UI
- Automatic transcription on gem creation (this already happens via `enrich_gem` when MLX Omni is the active engine)
