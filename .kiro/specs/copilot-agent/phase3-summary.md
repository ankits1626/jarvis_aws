# Phase 3 Summary: Agent Lifecycle — Execution & Control

## Status: IN PROGRESS

## Completed Tasks

### Task 11: CoPilotAgent Lifecycle Management ✅
- **11.1**: Added lifecycle fields to CoPilotAgent struct (cycle_task, stop_tx)
- **11.2**: Implemented `start()` method with recording file validation and background task spawning
- **11.3**: Implemented `stop()` method with graceful shutdown and 150s timeout
- **11.4**: Implemented `get_state()` and `dismiss_question()` methods

### Task 12: Agent Cycle Loop ✅
- **12.1**: Implemented `run_cycle_loop()` function with:
  - Cycle interval management (configurable, default 60s)
  - Log file creation (conditional based on settings)
  - Cycle execution with failure tracking
  - 3 consecutive failure threshold with automatic pause
  - Event emission (copilot-updated, copilot-status, copilot-error)
  - Graceful stop signal handling
  - Log summary writing on completion

- **12.2**: Implemented `run_single_cycle()` function with:
  - Audio chunk extraction
  - Running context management (empty for cycle 1, summary for subsequent)
  - Provider call with 120s timeout
  - Temporary file cleanup
  - State update via `update_state_internal()` helper

- Created `update_state_internal()` helper function to avoid AppHandle issues

## Remaining Tasks

### Task 13: Implement Tauri Commands (NOT STARTED)
- 13.1: Implement start_copilot command
- 13.2: Implement stop_copilot command
- 13.3: Implement get_copilot_state command
- 13.4: Implement dismiss_copilot_question command
- 13.5: Register commands in lib.rs

### Task 14: Automatic Agent Stop on Recording Stop (NOT STARTED)
- 14.1: Add useEffect in App.tsx to stop agent when recording stops

### Task 15: Checkpoint - Backend Implementation Tests (NOT STARTED)

## Key Implementation Details

### Architecture
- Agent runs in background tokio task spawned by `start()`
- Uses `watch::channel` for stop signaling
- State shared via `Arc<TokioMutex<CoPilotState>>`
- Provider accessed via trait (`Arc<dyn IntelProvider>`)

### Cycle Loop Flow
1. Check stop signal (non-blocking)
2. Mark state as processing
3. Emit "processing" status event
4. Run single cycle (extract audio → analyze → update state)
5. Handle result (success: emit updated event, failure: emit error event)
6. Track consecutive failures (stop after 3)
7. Mark state as not processing
8. Sleep until next cycle or stop signal

### Error Handling
- Cycle failures are skipped (don't crash the agent)
- Failed cycles increment `failed_cycles` counter
- 3 consecutive failures trigger automatic pause
- Timeout on provider call (120s) treated as failure

### Logging
- Conditional based on `settings.agent_logging`
- Log file created at start with header
- Each cycle logged with prompt, response, timing, status
- Summary written on stop

## Files Modified
- `jarvis-app/src-tauri/src/agents/copilot.rs`:
  - Added imports: `watch`, `Emitter`
  - Added lifecycle fields to CoPilotAgent struct
  - Implemented lifecycle methods (start, stop, get_state, dismiss_question)
  - Implemented cycle loop functions (run_cycle_loop, run_single_cycle, update_state_internal)

## Next Steps
1. Implement Tauri commands (Task 13) - wire up agent to frontend
2. Add automatic stop on recording end (Task 14)
3. Run checkpoint tests (Task 15)
4. Proceed to Phase 4 (Frontend UI)

## Notes
- Optional unit tests (11.5*, 12.3*) skipped for MVP velocity
- Agent logging uses placeholder values for some fields (will be improved in polish)
- Model name hardcoded as "Qwen 2.5 Omni 3B" (TODO: get from provider)
