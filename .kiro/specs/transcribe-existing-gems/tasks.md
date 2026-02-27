# Tasks: Transcribe Existing Recording Gems

## Phase 1: Backend Foundation - Path Extraction Fix

### 1. Backend: Fix `extract_recording_path()` Bug
- [x] 1.1 Remove `source_type` check from `extract_recording_path()` function
- [x] 1.2 Update logic to detect recording gems by presence of filename keys in `source_meta`
- [x] 1.3 Check keys in priority order: `recording_filename`, `filename`, `recording_path`, `file`, `path`
- [x] 1.4 Construct path using `dirs::data_dir()/com.jarvis.app/recordings/{filename}`
- [x] 1.5 Write unit tests for path extraction with various filename keys
- [x] 1.6 Write unit test verifying `source_type` is ignored
- [x] 1.7 Write property test for path extraction with recording metadata (Property 1)
- [x] 1.8 Write property test for path extraction without recording metadata (Property 2)

## Phase 2: Backend Core - Transcription Flow

### 2. Backend: Reorder `enrich_content()` Flow
- [x] 2.1 Move transcript generation to execute before tags/summary generation
- [x] 2.2 Update tags generation to use `transcript` if available, otherwise `content`
- [x] 2.3 Update summary generation to use `transcript` if available, otherwise `content`
- [ ] 2.4 Add unit tests verifying transcript-first ordering
- [x] 2.5 Add unit tests verifying tags/summary use transcript when available

### 3. Backend: Implement `transcribe_gem` Command
- [x] 3.1 Create `transcribe_gem` function with correct signature
- [x] 3.2 Implement provider availability check (step 0)
- [x] 3.3 Implement gem fetching from store (step 1)
- [x] 3.4 Implement recording path extraction (step 2)
- [x] 3.5 Implement file existence verification (step 3)
- [x] 3.6 Implement transcript generation call (step 4)
- [x] 3.7 Update gem with transcript and language (step 5)
- [x] 3.8 Implement tag generation from transcript (step 6)
- [x] 3.9 Implement summary generation from transcript (step 7)
- [x] 3.10 Update `ai_enrichment` with metadata (step 8)
- [x] 3.11 Save and return updated gem (steps 9-10)
- [x] 3.12 Implement error handling for all error cases
- [x] 3.13 Implement graceful degradation for tag/summary failures
- [x] 3.14 Register command in `invoke_handler!` macro

## Phase 3: Backend Testing - Unit & Property Tests

### 4. Backend: Unit Tests for `transcribe_gem`
- [x] 4.1 Write test for successful transcription (happy path)
- [x] 4.2 Write test for gem not found error
- [x] 4.3 Write test for no recording metadata error
- [x] 4.4 Write test for recording file not found error
- [x] 4.5 Write test for provider unavailable error
- [x] 4.6 Write test for provider doesn't support transcription error
- [x] 4.7 Write test verifying only expected fields are updated
- [x] 4.8 Write test verifying tags are generated from transcript
- [x] 4.9 Write test verifying summary is generated from transcript
- [x] 4.10 Write test for graceful degradation when tag generation fails
- [x] 4.11 Write test for graceful degradation when summary generation fails

### 5. Backend: Property Tests for `transcribe_gem`
- [ ] 5.1 Write property test for field preservation (Property 3)
- [ ] 5.2 Create generator for gems with recording metadata
- [ ] 5.3 Create generator for gems without recording metadata
- [ ] 5.4 Create generator for source_meta with filename keys
- [ ] 5.5 Create generator for source_meta without filename keys

## Phase 4: Frontend Implementation - UI Components

### 6. Frontend: Add Transcribe Button
- [x] 6.1 Add state variables for transcribing and transcribeError
- [x] 6.2 Implement visibility logic for Transcribe button
- [x] 6.3 Render Transcribe button with correct styling
- [x] 6.4 Implement `handleTranscribe` function
- [x] 6.5 Extract provider and model from `ai_enrichment`
- [x] 6.6 Construct `enrichment_source` from provider/model
- [x] 6.7 Update local gem state with transcript, tags, summary
- [x] 6.8 Update fullGem cache if expanded
- [x] 6.9 Implement error handling and display
- [x] 6.10 Implement loading state with "..." button text
- [x] 6.11 Add "Transcribing audio..." status banner during transcription (shared with enriching banner)

### 7. Frontend: Add Transcript Status Badge
- [x] 7.1 Implement visibility logic for transcript badge
- [x] 7.2 Render badge with language from `transcript_language`
- [x] 7.3 Style badge similar to source type badge
- [x] 7.4 Position badge near gem title or in metadata row

## Phase 5: Frontend Testing - Unit & Property Tests

### 8. Frontend: Unit Tests for UI Components
- [ ] 8.1 Write test for button visible on recording without transcript
- [ ] 8.2 Write test for button hidden on recording with transcript
- [ ] 8.3 Write test for button hidden when AI unavailable
- [ ] 8.4 Write test for button visible with existing tags
- [ ] 8.5 Write test for badge visible with language
- [ ] 8.6 Write test for badge hidden without language
- [ ] 8.7 Write test for button click calls command
- [ ] 8.8 Write test for button shows loading state
- [ ] 8.9 Write test for error display
- [ ] 8.10 Write test for expanded gem shows MLX transcript
- [ ] 8.11 Write test for "Transcribing audio..." banner during transcription

### 9. Frontend: Property Tests for UI Components
- [ ] 9.1 Write property test for button visibility (Property 4)
- [ ] 9.2 Write property test for button hidden with transcript (Property 5)
- [ ] 9.3 Write property test for badge visibility (Property 6)
- [ ] 9.4 Write property test for transcript display when expanded (Property 7)
- [ ] 9.5 Create generator for recording gems
- [ ] 9.6 Create generator for gems with transcript
- [ ] 9.7 Create generator for gems without transcript

## Phase 6: Integration & Manual Testing

### 10. Integration Testing
- [ ] 10.1 Manual test: Transcribe recording without transcript
- [ ] 10.2 Manual test: Transcribe recording with existing tags/summary
- [ ] 10.3 Manual test: Attempt to transcribe non-recording gem
- [ ] 10.4 Manual test: Attempt to transcribe with IntelligenceKit provider
- [ ] 10.5 Manual test: Transcribe with missing recording file
- [ ] 10.6 Manual test: View expanded gem with both transcripts
- [ ] 10.7 Manual test: Filter gems by tag after transcription
- [ ] 10.8 End-to-end test: Complete transcription workflow

## Phase 7: Performance & Error Recovery

### 11. Performance and Error Recovery Testing
- [ ] 11.1 Test transcription duration for short audio (< 1 min)
- [ ] 11.2 Test transcription duration for medium audio (1-5 min)
- [ ] 11.3 Test transcription duration for long audio (5-10 min)
- [ ] 11.4 Test UI responsiveness during transcription
- [ ] 11.5 Test sidecar crash during transcription
- [ ] 11.6 Test error recovery and retry functionality
- [ ] 11.7 Test timeout handling for very long audio
