# Implementation Plan: Co-Pilot Agent — Live Recording Intelligence

## Overview

This implementation adds a live recording intelligence system (Co-Pilot Agent) that runs alongside audio recording and produces real-time actionable insights. The agent feeds raw audio chunks directly to Qwen Omni (multimodal model), aggregates results across cycles, and provides rolling summaries, suggested questions, key concepts, decisions, and action items.

The implementation follows a provider-agnostic architecture where the Co-Pilot calls `IntelProvider::copilot_analyze()` — the same trait used for gem enrichment. This ensures a single point of change if the backend switches from local MLX to a cloud API provider.

## Implementation Phases

### Phase 1: Foundation — Provider Integration & Settings (Tasks 1-6)

**Goal:** Set up the basic infrastructure for Co-Pilot settings and provider integration.

**Tasks:**
- Task 1: Set up Co-Pilot agent module structure
- Task 2: Implement CoPilotSettings in settings module
- Task 3: Extend IntelProvider trait with copilot_analyze method
- Task 4: Implement copilot_analyze in MlxProvider
- Task 5: Implement copilot-analyze command in MLX sidecar
- Task 6: Checkpoint - Ensure provider integration tests pass

**Validation:** Provider can accept audio files and return structured analysis results.

---

### Phase 2: Core Agent — Audio Processing & State Management (Tasks 7-10)

**Goal:** Implement audio chunk extraction, state structures, and aggregation logic.

**Tasks:**
- Task 7: Implement audio chunk extraction
- Task 8: Implement CoPilotState data structures
- Task 9: Implement state aggregation logic
- Task 10: Implement agent logging

**Validation:** Agent can extract audio chunks, maintain state, and log operations.

---

### Phase 3: Agent Lifecycle — Execution & Control (Tasks 11-15)

**Goal:** Implement the agent lifecycle, cycle loop, and Tauri commands.

**Tasks:**
- Task 11: Implement CoPilotAgent lifecycle management
- Task 12: Implement agent cycle loop
- Task 13: Implement Tauri commands
- Task 14: Implement automatic agent stop on recording stop
- Task 15: Checkpoint - Ensure backend implementation tests pass

**Validation:** Agent can start, run cycles, stop gracefully, and handle errors.

---

### Phase 4: Frontend — UI Components & Integration (Tasks 16-20)

**Goal:** Build the frontend UI for displaying Co-Pilot data.

**Tasks:**
- Task 16: Implement frontend TypeScript types
- Task 17: Implement CoPilotPanel component
- Task 18: Implement tab integration in RightPanel
- Task 19: Implement Co-Pilot toggle in App.tsx
- Task 20: Implement CSS styling for Co-Pilot components

**Validation:** UI displays Co-Pilot data in real-time with proper styling and interactions.

---

### Phase 5: Integration — Gems & Settings (Tasks 21-23)

**Goal:** Integrate Co-Pilot data with gems and add settings UI.

**Tasks:**
- Task 21: Implement gem integration for Co-Pilot data
- Task 22: Implement Settings UI for Co-Pilot configuration
- Task 23: Checkpoint - Ensure frontend implementation is complete

**Validation:** Co-Pilot data persists in gems and settings are configurable.

---

### Phase 6: End-to-End Testing & Polish (Tasks 24-25)

**Goal:** Validate the complete system and ensure all requirements are met.

**Tasks:**
- Task 24: Integration and wiring
- Task 25: Final checkpoint - Ensure all tests pass

**Validation:** Full Co-Pilot workflow works end-to-end with proper error handling.

---

## Tasks

### Phase 1: Foundation — Provider Integration & Settings

- [x] 1. Set up Co-Pilot agent module structure
  - Create `jarvis-app/src-tauri/src/agents/` directory
  - Create `jarvis-app/src-tauri/src/agents/mod.rs` with module declaration
  - Create `jarvis-app/src-tauri/src/agents/copilot.rs` skeleton
  - Add agents module to `lib.rs`
  - _Requirements: 3.7_

- [x] 2. Implement CoPilotSettings in settings module
  - [x] 2.1 Add CoPilotSettings struct to settings/manager.rs
    - Define struct with fields: enabled (bool), cycle_interval (u64), audio_overlap (u64), agent_logging (bool)
    - Add default implementations: enabled=false, cycle_interval=60, audio_overlap=5, agent_logging=true
    - Add `#[serde(default)]` attributes for backward compatibility
    - _Requirements: 12.1, 12.2_

  - [ ]* 2.2 Write unit tests for CoPilotSettings validation
    - Test cycle_interval range validation (30-120s)
    - Test audio_overlap range validation (0-15s)
    - Test overlap < interval validation
    - Test backward compatibility (missing copilot key deserializes with defaults)
    - _Requirements: 12.2_

  - [x] 2.3 Add copilot field to main Settings struct
    - Add `#[serde(default)]` on copilot field for backward compatibility
    - Update Settings validation to include copilot settings checks
    - _Requirements: 12.2, 12.3_

- [x] 3. Extend IntelProvider trait with copilot_analyze method
  - [x] 3.1 Define CoPilotCycleResult types in provider.rs
    - Create CoPilotCycleResult struct with all required fields
    - Create CoPilotQuestion and CoPilotConcept helper structs
    - Add Serialize/Deserialize derives
    - _Requirements: 2.2_

  - [x] 3.2 Add copilot_analyze method to IntelProvider trait
    - Add async method signature with audio_path and context parameters
    - Provide default implementation returning "not supported" error
    - Add documentation comments
    - _Requirements: 2.1, 2.9_

  - [ ]* 3.3 Write unit tests for CoPilotCycleResult serialization
    - **Property 12: State JSON Serialization Round Trip**
    - **Validates: Requirements 5.6**

- [x] 4. Implement copilot_analyze in MlxProvider
  - [x] 4.1 Implement copilot_analyze method in MlxProvider
    - Send copilot-analyze NDJSON command to sidecar
    - Include audio_path and context in request
    - Parse JSON response into CoPilotCycleResult
    - Handle timeout (120s) and errors gracefully
    - Note: MlxProvider's internal mutex handles concurrency automatically
    - _Requirements: 2.3, 2.6, 11.2_

  - [x] 4.2 Add graceful JSON parsing for partial responses
    - Strip markdown code fences if present
    - Parse available fields, provide defaults for missing
    - Handle malformed JSON without crashing
    - _Requirements: 2.7_

  - [ ]* 4.3 Write unit tests for graceful JSON parsing
    - **Property 5: Graceful JSON Parsing**
    - **Validates: Requirements 2.7**

- [x] 5. Implement copilot-analyze command in MLX sidecar
  - [x] 5.1 Add copilot_analyze method to MLXServer class in server.py
    - Load audio file using librosa (16kHz, mono)
    - Construct prompt with context (different for first cycle vs subsequent)
    - Build messages with audio using apply_chat_template
    - Generate response using mlx_omni_generate
    - Parse JSON response and validate fields
    - Return structured response or error
    - _Requirements: 2.4, 2.5, 2.8_

  - [x] 5.2 Add copilot-analyze command handler to NDJSON loop
    - Parse copilot-analyze command from stdin
    - Call copilot_analyze method
    - Write response to stdout as NDJSON
    - Handle errors and return error response
    - _Requirements: 2.3_

  - [ ]* 5.3 Write integration test for copilot-analyze command
    - **Property 4: NDJSON Protocol Round Trip**
    - **Validates: Requirements 2.5**

- [x] 6. Checkpoint - Ensure provider integration tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 7. Implement audio chunk extraction
  - [x] 7.1 Implement extract_audio_chunk function in copilot.rs
    - Calculate chunk duration (cycle_interval + audio_overlap)
    - Calculate byte size (duration × 16000 × 2)
    - Open recording file and get file size
    - Handle case where file is shorter than chunk size (first cycle)
    - Read chunk from end of file using seek
    - Generate unique temp filename using UUID
    - Convert PCM to WAV using existing convert_to_wav logic
    - Write to temp file in system temp directory
    - Return temp file path
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.7, 1.8_

  - [ ]* 7.2 Write unit tests for audio chunk extraction
    - **Property 1: Audio Chunk Byte Calculation**
    - **Property 2: Unique Temporary File Names**
    - **Validates: Requirements 1.2, 1.4**

  - [x] 7.3 Add cleanup logic for temporary files
    - Clean up temp file after inference completes (success or failure)
    - Clean up temp file before early returns
    - Log warning if cleanup fails but continue
    - _Requirements: 1.5_

  - [ ]* 7.4 Write unit tests for temporary file cleanup
    - **Property 3: Temporary File Cleanup**
    - **Validates: Requirements 1.5**

- [x] 8. Implement CoPilotState data structures
  - [x] 8.1 Define CoPilotState and related structs in copilot.rs
    - Create CoPilotState struct with all required fields
    - Create SuggestedQuestion struct with question, reason, cycle_added, dismissed
    - Create KeyConcept struct with term, context, cycle_added, mention_count
    - Create CycleMetadata struct with cycle_number, last_updated_at, processing, failed_cycles, total_audio_seconds
    - Add Default implementation for CoPilotState
    - Add Serialize/Deserialize derives
    - _Requirements: 5.1, 5.2, 5.3, 5.4_

  - [ ]* 8.2 Write unit tests for state serialization
    - **Property 12: State JSON Serialization Round Trip**
    - **Validates: Requirements 5.6**

- [x] 9. Implement state aggregation logic
  - [x] 9.1 Implement update_state method in CoPilotAgent
    - Replace running_summary with latest updated_summary
    - Append new items to key_points, decisions, action_items, open_questions with deduplication
    - Replace suggested_questions (max 5, preserve dismissed state)
    - Merge key_concepts (increment mention_count for existing terms)
    - Update cycle_metadata (cycle_number, last_updated_at, processing, total_audio_seconds)
    - _Requirements: 5.5_

  - [ ]* 9.2 Write unit tests for state aggregation
    - **Property 11: State Update Deduplication**
    - **Validates: Requirements 5.5**

- [x] 10. Implement agent logging
  - [x] 10.1 Implement log file creation and header writing
    - Create agent_logs directory if it doesn't exist
    - Generate log filename with timestamp
    - Write log header with recording info, settings, model name
    - _Requirements: 13.1, 13.2, 13.8_

  - [x] 10.2 Implement cycle logging
    - Append cycle entry to log file after each cycle
    - Include cycle number, audio chunk info, inference time, status
    - Include full prompt and response
    - Use append-only file writes
    - _Requirements: 13.3, 13.5_

  - [x] 10.3 Implement log summary writing
    - Write summary section when agent stops
    - Include total cycles, successful, skipped, errors, avg inference time, total duration
    - _Requirements: 13.4_

  - [x] 10.4 Add conditional logging based on settings
    - Only create/write log files when agent_logging is enabled
    - Skip logging when disabled
    - _Requirements: 13.6_

  - [ ]* 10.5 Write unit tests for agent logging
    - **Property 20: Agent Logging Conditional Creation**
    - **Property 21: Log File Append-Only**
    - **Property 22: Log Excludes Binary Data**
    - **Property 23: Log Directory Auto-Creation**
    - **Validates: Requirements 13.1, 13.3, 13.6, 13.7, 13.8**

- [x] 11. Implement CoPilotAgent lifecycle management
  - [x] 11.1 Implement CoPilotAgent struct and new method
    - Define struct with app_handle, state, cycle_task, stop_tx
    - Implement new method to create agent instance
    - _Requirements: 3.9_

  - [x] 11.2 Implement start method
    - Check if recording is active (return error if not)
    - Accept Arc<dyn IntelProvider> parameter
    - Create stop signal channel (watch::channel)
    - Spawn tokio background task for cycle loop
    - Store task handle and stop_tx
    - _Requirements: 3.4, 3.7_

  - [x] 11.3 Implement stop method
    - Send stop signal via stop_tx
    - Wait for cycle task to complete (with timeout)
    - Return final state
    - _Requirements: 3.5_

  - [x] 11.4 Implement get_state and dismiss_question methods
    - get_state: return current state clone
    - dismiss_question: mark question as dismissed by index
    - _Requirements: 3.9_

  - [ ]* 11.5 Write unit tests for agent lifecycle
    - **Property 6: Start Requires Active Recording**
    - **Property 7: Concurrent Instance Prevention**
    - **Validates: Requirements 3.4, 3.8**

- [x] 12. Implement agent cycle loop
  - [x] 12.1 Implement run_cycle_loop function
    - Initialize cycle counter and failure tracking
    - Create log file if logging enabled
    - Loop: check stop signal, mark processing, run cycle, update state, emit events, sleep
    - Handle cycle failures (skip cycle, increment failed_cycles)
    - Stop after 3 consecutive failures
    - Write log summary on stop
    - _Requirements: 4.1, 4.2, 4.5, 4.6, 4.7_

  - [x] 12.2 Implement run_single_cycle function
    - Extract audio chunk
    - Get running context from state (empty string for cycle 1, running_summary for subsequent cycles)
    - Wrap provider.copilot_analyze call in tokio::time::timeout (120s)
    - Handle timeout by returning error (cycle will be skipped)
    - Clean up temp file after inference
    - Convert CoPilotCycleResult to internal format
    - Return result or error
    - _Requirements: 2.6, 4.2, 4.3, 4.4, 4.8, 11.3_

  - [ ]* 12.3 Write unit tests for cycle loop
    - **Property 9: Context Propagation Between Cycles**
    - **Property 10: Failure Skips Cycle**
    - **Validates: Requirements 4.3, 4.5**

- [x] 13. Implement Tauri commands
  - [x] 13.1 Implement start_copilot command
    - Check if recording is active
    - Get copilot settings
    - Get Arc<dyn IntelProvider> from Tauri app state
    - Check if agent already running (return error if yes)
    - Create and start CoPilotAgent, passing provider to agent.start()
    - Store agent in app state
    - _Requirements: 3.1, 3.4, 3.8_

  - [x] 13.2 Implement stop_copilot command
    - Get agent from app state
    - Call agent.stop() and wait for completion
    - Remove agent from app state
    - Return final state
    - _Requirements: 3.2_

  - [x] 13.3 Implement get_copilot_state command
    - Get agent from app state
    - Return current state
    - _Requirements: 3.3_

  - [x] 13.4 Implement dismiss_copilot_question command
    - Get agent from app state
    - Call agent.dismiss_question with index
    - _Requirements: 3.9, 8.5_

  - [x] 13.5 Register commands in lib.rs
    - Add all copilot commands to tauri::Builder
    - Initialize copilot agent state in app state
    - _Requirements: 3.1, 3.2, 3.3_

  - [ ]* 13.6 Write unit tests for Tauri commands
    - Test start_copilot without active recording returns error
    - Test concurrent start_copilot calls return error
    - Test stop_copilot returns final state
    - _Requirements: 3.4, 3.8_

- [x] 14. Implement automatic agent stop on recording stop
  - [x] 14.1 Add useEffect in App.tsx to stop agent when recording stops
    - Monitor recordingState changes via useEffect
    - When recordingState !== 'recording' and copilotEnabled is true, call stop_copilot command
    - Reset copilot state (setCopilotEnabled(false), setCopilotStatus('stopped'))
    - _Requirements: 3.6_

  - [ ]* 14.2 Write integration test for automatic stop
    - **Property 8: Recording Stop Triggers Agent Stop**
    - **Validates: Requirements 3.6**

- [x] 15. Checkpoint - Ensure backend implementation tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 16. Implement frontend TypeScript types
  - [x] 16.1 Add CoPilot types to src/state/types.ts
    - Define CoPilotState interface
    - Define SuggestedQuestion interface
    - Define KeyConcept interface
    - Define CycleMetadata interface
    - Define CoPilotStatus type
    - _Requirements: 5.1, 5.2, 5.3, 5.4_

- [x] 17. Implement CoPilotPanel component
  - [x] 17.1 Create CoPilotPanel.tsx component
    - Create component skeleton with props interface
    - Implement placeholder view for empty state
    - _Requirements: 8.10_

  - [x] 17.2 Implement Summary section
    - Display running_summary as body text
    - Display key_points as bullet list
    - Display open_questions as warning-styled bullet list
    - _Requirements: 8.2_

  - [x] 17.3 Implement Decisions & Action Items section
    - Display decisions as checklist
    - Display action_items as bullet list
    - _Requirements: 8.3_

  - [x] 17.4 Implement Suggested Questions section
    - Display each question as a card with question text and reason
    - Add dismiss button to each card
    - Add copy-to-clipboard on click
    - Show "Copied" indicator briefly after copy
    - _Requirements: 8.4, 8.5, 8.6_

  - [x] 17.5 Implement Key Concepts section
    - Display each concept as a chip/pill with term and mention count
    - Show context as tooltip
    - _Requirements: 8.7_

  - [x] 17.6 Implement status footer
    - Display cycle number and time since last update
    - Display agent status indicator
    - Show pulse animation when processing
    - _Requirements: 8.8, 8.9_

- [x] 18. Implement tab integration in RightPanel
  - [x] 18.1 Add copilot props to RightPanel interface
    - Add copilotEnabled, copilotStatus, copilotState, copilotError props
    - Add onDismissCopilotQuestion callback prop
    - _Requirements: 7.1_

  - [x] 18.2 Create RecordTabsView component in RightPanel.tsx
    - Add tab state management (transcript vs copilot)
    - Implement tab buttons with active styling
    - Implement notification dot for new copilot data
    - Render TranscriptDisplay or CoPilotPanel based on active tab
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6_

  - [x] 18.3 Modify activeNav === 'record' branch in RightPanel
    - Show RecordTabsView when copilotEnabled and recording
    - Show TranscriptDisplay only when copilot disabled
    - Show placeholder when no recording
    - _Requirements: 7.1, 7.7, 7.8_

- [x] 19. Implement Co-Pilot toggle in App.tsx
  - [x] 19.1 Add copilot state to App.tsx
    - Add copilotEnabled, copilotStatus, copilotState, copilotError state
    - _Requirements: 9.1_

  - [x] 19.2 Add Tauri event listeners for copilot events
    - Listen for copilot-updated event
    - Listen for copilot-status event
    - Listen for copilot-error event
    - _Requirements: 6.1, 6.2, 6.3, 8.11_

  - [ ]* 19.3 Write integration tests for event emission
    - **Property 14: Event Emission on State Change**
    - **Property 15: Status Event on State Transition**
    - **Property 16: Error Event on Cycle Failure**
    - **Validates: Requirements 6.1, 6.2, 6.3, 6.4**

  - [x] 19.4 Implement handleCopilotToggle function
    - Call start_copilot when toggled on
    - Call stop_copilot when toggled off
    - Handle errors and show toast
    - _Requirements: 9.3, 9.4_

  - [x] 19.5 Implement handleDismissCopilotQuestion function
    - Call dismiss_copilot_question command
    - Update local state
    - _Requirements: 8.5_

  - [x] 19.6 Add automatic reset on recording stop
    - Reset copilot state when recording stops
    - _Requirements: 9.7_

  - [x] 19.7 Add Co-Pilot toggle UI to record section
    - Show toggle below record button when recording active
    - Disable toggle when no recording
    - _Requirements: 9.1, 9.2, 9.5, 9.6_

  - [x] 19.8 Pass copilot props to RightPanel
    - Pass all copilot state and callbacks to RightPanel
    - _Requirements: 7.1_

- [x] 20. Implement CSS styling for Co-Pilot components
  - [x] 20.1 Add CoPilotPanel styles to App.css
    - Add styles for copilot-panel, copilot-section, summary, key-points, decisions, action-items
    - _Requirements: 8.2_

  - [x] 20.2 Add Suggested Questions styles to App.css
    - Add styles for questions-grid, question-card, dismiss-button, copied-indicator
    - _Requirements: 8.4, 8.6_

  - [x] 20.3 Add Key Concepts styles to App.css
    - Add styles for concepts-grid, concept-chip, mention-count
    - _Requirements: 8.7_

  - [x] 20.4 Add status footer styles to App.css
    - Add styles for copilot-footer, status-indicator, pulse animation
    - _Requirements: 8.8, 8.9_

  - [x] 20.5 Add tab button styles to App.css
    - Add styles for tab-buttons, tab-button, notification-dot
    - _Requirements: 7.4, 7.5, 7.6_

  - [x] 20.6 Add Co-Pilot toggle styles to App.css
    - Add styles for copilot-toggle, checkbox, toggle-label
    - _Requirements: 9.6_

- [x] 21. Implement gem integration for Co-Pilot data
  - [x] 21.1 Add copilot data to gem creation
    - Get final CoPilotState when saving recording as gem
    - Add copilot data to source_meta.copilot field
    - Include summary, key_points, decisions, action_items, open_questions, key_concepts, total_cycles, total_audio_analyzed_seconds
    - _Requirements: 10.1, 10.2_

  - [x] 21.2 Extend GemDetailPanel to display copilot data
    - Detect presence of source_meta.copilot
    - Render Co-Pilot sections (summary, decisions, action items, concepts)
    - Use same styling as live CoPilotPanel but without interactive elements
    - Show Co-Pilot sections before AI enrichment sections
    - _Requirements: 10.3, 10.4, 10.5, 10.6_

  - [ ]* 21.3 Write integration test for gem copilot data persistence
    - **Property 17: Gem Co-Pilot Data Persistence**
    - **Validates: Requirements 10.1, 10.2**

- [x] 22. Implement Settings UI for Co-Pilot configuration
  - [x] 22.1 Add Co-Pilot section to Settings component
    - Add "Co-Pilot" section header
    - Add toggle for enabled (auto-start with recording)
    - Add slider for cycle_interval (30-120s, step 10s)
    - Add slider for audio_overlap (0-15s, step 1s)
    - Add toggle for agent_logging
    - _Requirements: 12.3, 12.4_

  - [x] 22.2 Add validation for Co-Pilot settings
    - Validate cycle_interval range (30-120s)
    - Validate audio_overlap range (0-15s)
    - Validate overlap < interval
    - Show validation errors
    - _Requirements: 12.3_

  - [x] 22.3 Handle settings changes during active recording
    - Apply cycle_interval and audio_overlap changes starting from next cycle
    - _Requirements: 12.5_

- [x] 23. Checkpoint - Ensure frontend implementation is complete
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 24. Integration and wiring
  - [ ] 24.1 Test full Co-Pilot workflow end-to-end
    - Start recording, enable Co-Pilot, verify cycles run
    - Verify state updates and events emitted
    - Verify UI updates in real-time
    - Stop Co-Pilot, verify cleanup
    - Save as gem, verify copilot data persisted
    - _Requirements: All_

  - [ ] 24.2 Test concurrency with gem enrichment
    - Start Co-Pilot during recording
    - Trigger gem enrichment
    - Verify provider concurrency is handled correctly
    - _Requirements: 11.2_

  - [ ]* 24.3 Write integration test for provider concurrency
    - **Property 18: Provider Concurrency Serialization**
    - **Validates: Requirements 11.2**

  - [ ] 24.4 Test error handling scenarios
    - Test start without recording
    - Test concurrent start attempts
    - Test cycle failures and recovery
    - Test 3 consecutive failures threshold
    - Test graceful JSON parsing with malformed responses
    - _Requirements: 4.5, 4.6_

  - [ ] 24.5 Test settings backward compatibility
    - Load settings.json without copilot key
    - Verify defaults are applied
    - _Requirements: 12.2_

  - [ ]* 24.6 Write integration test for settings backward compatibility
    - **Property 19: Settings Backward Compatibility**
    - **Validates: Requirements 12.2**

- [ ] 25. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties
- Unit tests validate specific examples and edge cases
- The implementation follows a provider-agnostic architecture for future extensibility
