# Tasks: Transcribe Recordings from Recordings List

## Phase 1: Backend Foundation - GemStore Extension

**Goal**: Extend GemStore trait with recording filename lookup capability

**Deliverable**: Working `find_by_recording_filename` method with tests

### 1. Backend - GemStore Extension

### 1.1 Add `find_by_recording_filename` to GemStore trait
- [x] Add method signature to `GemStore` trait in `src-tauri/src/gems/store.rs`
- [x] Add documentation explaining the method searches by `source_meta.recording_filename`
- [x] Note that it returns the most recent gem if multiple exist

### 1.2 Update test mocks for GemStore trait change
- [x] Search for any existing GemStore test mocks in test files
- [x] Add `find_by_recording_filename` implementation to each mock
- [x] Return appropriate test data or `Ok(None)` for simple mocks

### 1.3 Implement `find_by_recording_filename` in SqliteGemStore
- [x] Implement method in `SqliteGemStore` in `src-tauri/src/gems/sqlite_store.rs`
- [x] Use `json_extract(source_meta, '$.recording_filename')` in SQL query
- [x] Order by `captured_at DESC LIMIT 1` to handle duplicate filenames
- [x] Use existing `row_to_gem` and `gem_to_preview` helper methods
- [x] Return `Result<Option<GemPreview>, String>`

### 2. Phase 1 Testing
- [x] Test `find_by_recording_filename` with existing gem
- [x] Test `find_by_recording_filename` with no gem
- [x] Test `find_by_recording_filename` with multiple gems (returns most recent)
- [x] Property 3: Recording Filename Query Correctness (min 100 iterations)

---

## Phase 2: Backend Commands - Transcription

**Goal**: Implement transcription command with security validation

**Deliverable**: Working `transcribe_recording` command with unit tests

### 1. Backend - Transcribe Recording Command

### 2.1 Implement `transcribe_recording` command
- [x] Add command function in `src-tauri/src/commands.rs`
- [x] Accept `filename: String` and `intel_provider: State<'_, Arc<dyn IntelProvider>>`
- [x] Validate filename doesn't contain path separators (`/`, `\`, `..`) before constructing path
- [x] Construct full path using `dirs::data_dir()/com.jarvis.app/recordings/{filename}` with `PathBuf::join()`
- [x] Check provider availability via `check_availability()`
- [x] Verify file exists on disk
- [x] Call `provider.generate_transcript(path)`
- [x] Return `Result<TranscriptResult, String>`
- [x] Handle errors: provider unavailable, file not found, unsupported provider, invalid filename

### 2. Register Command
- [x] Register `transcribe_recording` in `src-tauri/src/lib.rs`

### 3. Phase 2 Testing
- [x] Test `transcribe_recording` with valid file
- [x] Test `transcribe_recording` with missing file
- [x] Test `transcribe_recording` with unavailable provider
- [x] Test `transcribe_recording` with invalid filename (path traversal attempts: `../`, `../../etc/passwd`, `subdir/file.pcm`)
- [ ] Property 1: Transcription Isolation (min 100 iterations)
- [ ] Property 2: Transcription Path Construction (min 100 iterations)

---

## Phase 3: Backend Commands - Gem Status Checks

**Goal**: Implement gem status check commands (individual and batch)

**Deliverable**: Working `check_recording_gem` and `check_recording_gems_batch` commands

**Note**: These commands were implemented in Phase 2 as they are tightly coupled with the transcription workflow.

### 1. Backend - Gem Status Commands

#### 1.1 Implement `check_recording_gem` command
- [x] Add command function in `src-tauri/src/commands.rs`
- [x] Accept `filename: String` and `gem_store: State<'_, Arc<dyn GemStore>>`
- [x] Call `gem_store.find_by_recording_filename(&filename)`
- [x] Return `Result<Option<GemPreview>, String>`

#### 1.2 Implement `check_recording_gems_batch` command
- [x] Add command function in `src-tauri/src/commands.rs`
- [x] Accept `filenames: Vec<String>` and `gem_store: State<'_, Arc<dyn GemStore>>`
- [x] Loop through filenames and call `find_by_recording_filename` for each
- [x] Build `HashMap<String, GemPreview>` with only recordings that have gems
- [x] Return `Result<HashMap<String, GemPreview>, String>`

### 2. Register Commands
- [x] Register `check_recording_gem` in `src-tauri/src/lib.rs`
- [x] Register `check_recording_gems_batch` in `src-tauri/src/lib.rs`

### 3. Phase 3 Testing
- [x] Test `check_recording_gem` with existing gem
- [x] Test `check_recording_gem` with no gem
- [x] Test `check_recording_gems_batch` with mixed results

---

## Phase 4: Backend Commands - Save Recording Gem

**Goal**: Implement gem save/update command with AI enrichment

**Deliverable**: Working `save_recording_gem` command with create and update flows

**Note**: This command was implemented in Phase 2 as it is tightly coupled with the transcription workflow.

### 1. Backend - Save Recording Gem Command

#### 1.1 Implement `save_recording_gem` command
- [x] Add command function in `src-tauri/src/commands.rs`
- [x] Accept `filename: String`, `transcript: String`, `language: String`, `created_at: u64`
- [x] Accept `gem_store`, `intel_provider`, `settings_manager` states
- [x] Check for existing gem via `find_by_recording_filename`
- [x] If gem exists: fetch full gem, update transcript/language, regenerate tags/summary, save
- [x] If gem doesn't exist: create new gem with deterministic URL `jarvis://recording/{filename}`
- [x] Format title from `created_at` using `chrono::DateTime::from_timestamp`
- [x] Set `source_meta` with `recording_filename` and `source` fields
- [x] Generate tags and summary from transcript
- [x] Handle graceful degradation when AI enrichment unavailable
- [x] Return `Result<Gem, String>`

### 2. Register Command
- [x] Register `save_recording_gem` in `src-tauri/src/lib.rs`

### 3. Phase 4 Testing
- [x] Test `save_recording_gem` create flow (no existing gem)
- [x] Test `save_recording_gem` update flow (existing gem)
- [x] Test `save_recording_gem` with unavailable AI enrichment
- [ ] Property 4: Save Recording Gem Checks Existing (min 100 iterations)
- [ ] Property 5: Update Existing Gem Preserves ID (min 100 iterations)
- [ ] Property 6: Create New Gem Uses Deterministic URL (min 100 iterations)
- [ ] Property 7: Save Returns Saved Gem (min 100 iterations)

---

## Phase 5: Frontend - UI Implementation

**Goal**: Implement complete UI flow for transcription and gem management

**Deliverable**: Working recordings list with transcribe, save, and status indicators

### 1. Frontend - Setup and Types
- [x] Verify `TranscriptResult` interface exists in `src/state/types.ts`
- [x] Add `RecordingState` interface if needed for component state

### 2. Frontend - Gem Status on Mount

#### 2.1 Implement batch gem status check on mount
- [x] In `src/App.tsx`, call `check_recording_gems_batch` when recordings list renders
- [x] Store result as `Record<string, GemPreview>`
- [x] Set `hasGem` flag for each recording using `filename in gemStatusMap`
- [x] Cache AI availability check result on mount

#### 2.2 Add gem status indicator
- [x] Display gem indicator icon/badge on recording row when `hasGem` is true
- [x] Update indicator after successful gem save/update
- [x] Use visual indicator (icon, colored dot, or badge)

### 3. Frontend - Transcription Flow

#### 3.1 Add Transcribe button
- [x] Add "Transcribe" button to each recording row in `src/App.tsx`
- [x] Show button only when AI is available
- [x] Disable button during transcription (loading state)
- [x] Keep button active after transcription completes (allow re-transcription)
- [x] Call `invoke('transcribe_recording', { filename })`
- [x] Display spinner/loading indicator during transcription

#### 3.2 Display transcript inline
- [x] Add transcript display area below recording row
- [x] Show transcript text after successful transcription
- [x] Display error message inline if transcription fails
- [x] Keep transcript visible after transcription completes

### 4. Frontend - Save Gem Flow

#### 4.1 Add Save/Update Gem button
- [x] Show button below transcript after successful transcription
- [x] Call `check_recording_gem` after transcription to determine button label
- [x] Display "Save as Gem" if no gem exists
- [x] Display "Update Gem" if gem exists
- [x] Call `invoke('save_recording_gem', { filename, transcript, language, created_at })`
- [x] Pass `created_at` from `recording.metadata.created_at`
- [x] Show loading state during save operation
- [x] Display success indicator after save completes
- [x] Display error message if save fails

### 5. Frontend - Error Handling

#### 5.1 Add error handling
- [x] Catch and display errors from `transcribe_recording`
- [x] Catch and display errors from `save_recording_gem`
- [x] Reset loading states on error
- [x] Provide retry capability where appropriate

---

## Phase 6: Testing - Unit and Property-Based Tests

**Goal**: Comprehensive test coverage for all components

**Deliverable**: Full unit test suite and property-based tests

### 1. Backend Unit Tests (Remaining)

### 2. Frontend Unit Tests
- [ ] Test Transcribe button visibility based on AI availability
- [ ] Test button disabled state during transcription
- [ ] Test transcript display after successful transcription
- [ ] Test error display on transcription failure
- [ ] Test Save/Update button label logic
- [ ] Test gem indicator rendering
- [ ] Test state updates after gem save

### 3. Property-Based Tests (Frontend - TypeScript with fast-check)
- [ ] Property 8: Transcribe Button Disabled During Operation - verify button disabled when `transcribing = true` (min 100 iterations)
- [ ] Property 9: Button Label Reflects Gem Status - verify label is "Update Gem" when `hasGem = true`, "Save as Gem" when false (min 100 iterations)
- [ ] Property 10: Save Operation Updates UI State - verify `hasGem = true` and indicator visible after save (min 100 iterations)
- [ ] Property 11: Gem Status Check After Transcription - verify `check_recording_gem` called after transcription (min 100 iterations)
- [ ] Property 12: Gem Indicator Reflects Gem Existence - verify indicator visible iff `hasGem = true` (min 100 iterations)
- [ ] Property 13: Gem Indicator Updates After Save - verify indicator changes from hidden to visible after save (min 100 iterations)

---

## Phase 7: Integration and Manual Testing

**Goal**: End-to-end validation and user acceptance testing

**Deliverable**: Fully tested feature ready for production

### 1. Integration Tests
- [ ] Test end-to-end transcription flow (file → transcript)
- [ ] Test end-to-end gem creation flow (transcribe → save)
- [ ] Test end-to-end gem update flow (transcribe → update existing)
- [ ] Test batch gem status check with multiple recordings
- [ ] Test UI updates after gem save

### 2. Manual Testing
- [ ] Verify Transcribe button appears when AI available
- [ ] Verify transcription works with valid recording
- [ ] Verify transcript displays correctly
- [ ] Verify Save as Gem creates new gem
- [ ] Verify Update Gem updates existing gem
- [ ] Verify gem indicator appears after save
- [ ] Verify error messages display correctly
- [ ] Test with MLX provider (supported)
- [ ] Test with IntelligenceKit provider (unsupported)
- [ ] Test with missing recording file
- [ ] Test with very long transcript
- [ ] Test batch gem status check performance

---

## Phase 8: Documentation

**Goal**: Complete API and user documentation

**Deliverable**: Comprehensive documentation for developers and users

### 1. API Documentation
- [ ] Document `transcribe_recording` command
- [ ] Document `check_recording_gem` command
- [ ] Document `check_recording_gems_batch` command
- [ ] Document `save_recording_gem` command
- [ ] Document `find_by_recording_filename` GemStore method

### 2. User Documentation
- [ ] Document how to transcribe recordings from list
- [ ] Document how to save transcribed recordings as gems
- [ ] Document gem status indicators
- [ ] Note MLX provider requirement for transcription
