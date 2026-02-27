# Co-Pilot Agent — Live Recording Intelligence

## Introduction

Jarvis currently records audio and produces a real-time transcript via Whisper/WhisperKit. However, live transcripts are low-quality — words get dropped, sentences fragment, technical terms get mangled. The user has no actionable intelligence during the conversation itself; all enrichment (tags, summary) happens after the fact.

This spec defines requirements for a **Co-Pilot agent** that runs alongside the recording and produces high-quality, actionable output in real-time: a rolling summary, suggested questions, and key concept alerts. The agent bypasses live transcript quality issues by feeding **raw audio chunks directly to Qwen Omni** (a multimodal model that understands audio natively), then aggregating results across cycles to maintain a running understanding of the entire conversation.

**Reference:** Design elaboration in `discussion/28-feb-next-step/active-recording-agents.md`.

## Glossary

- **Co-Pilot**: The AI agent that analyzes audio during a live recording and produces structured intelligence.
- **Cycle**: One iteration of the agent loop — accumulate audio chunk, run inference, update state, emit results. Occurs at a user-configurable interval (default: 60 seconds).
- **Running Context**: The agent's compressed text summary of everything discussed so far, fed back as input to each cycle so the model has full conversation awareness.
- **Audio Chunk**: A slice of raw PCM audio extracted from the live recording and converted to WAV for inference. Duration = `cycle_interval + audio_overlap` (default: 65 seconds).
- **Audio Overlap**: Configurable overlap (default: 5 seconds) between consecutive audio chunks to bridge mid-sentence boundaries. The running context prevents the model from double-counting overlapped audio.
- **Agent State**: The complete output of the agent at any point: summary, key points, decisions, action items, open questions, suggested questions, and key concepts.
- **Agent Log**: A human-readable markdown file recording every prompt sent to and response received from the model, for debugging and quality tuning.
- **MLX Sidecar**: The Python process (`sidecars/mlx-server/server.py`) that runs Qwen Omni locally via MLX framework.

## Frozen Design Decisions

These decisions were made and locked during design review (2026-02-28):

1. **Concurrency — Option A**: Real-time transcription uses whisper-rs or WhisperKit (no MLX sidecar). Co-Pilot agent has exclusive use of the MLX sidecar for Qwen Omni inference. No scheduling or resource contention needed.
2. **Cycle interval**: Fixed default of 60s, user-configurable (30–120s). Adaptive intervals deferred to future.
3. **Audio overlap**: 5s default overlap between chunks to bridge mid-sentence boundaries. User-configurable (0–15s).
4. **Agent logging**: Full prompt/response audit trail in markdown files. Default ON.
5. **Self-compressing aggregation**: Each cycle, the model reads old summary + new audio → produces new summary. Running context never grows unbounded.
6. **Provider abstraction**: The Co-Pilot agent calls the `IntelProvider` trait (via `copilot_analyze` method), not `MlxProvider` directly. This ensures a single point of change if the backend switches from local MLX to a cloud API provider. The same trait is used for tag generation, summarization, transcription, and Co-Pilot analysis.

---

## Requirement 1: Audio Chunk Extraction

**User Story:** As the Co-Pilot agent, I need to extract the most recent audio from the live recording as a WAV file (with configurable overlap), so I can send it to Qwen Omni for analysis without interrupting the recording or missing content at chunk boundaries.

### Acceptance Criteria

1. THE System SHALL extract audio chunks from the live PCM recording file at the interval specified by `copilot.cycle_interval` (default: 60 seconds)
2. THE System SHALL read the last `(cycle_interval + audio_overlap)` seconds of PCM data from the recording file (calculated as `total_seconds × 16000 × 2` bytes from end of file) and prepend a WAV header to produce a valid WAV file
3. THE `audio_overlap` parameter SHALL be read from `copilot.audio_overlap` setting (default: 5 seconds, range: 0–15 seconds), providing a configurable window that bridges mid-sentence boundaries between consecutive chunks
4. THE System SHALL write the extracted chunk to a temporary file in the system temp directory with a unique name (e.g., `jarvis_copilot_chunk_{cycle_number}.wav`)
5. THE System SHALL clean up temporary chunk files after each cycle's inference completes
6. THE extraction SHALL NOT lock, block, or interfere with the ongoing recording or live transcription
7. THE System SHALL handle the case where the recording file is shorter than `cycle_interval + audio_overlap` (e.g., first cycle) by extracting whatever audio is available
8. THE chunk extraction SHALL reuse the existing WAV header logic from `convert_to_wav` in the recording module

---

## Requirement 2: IntelProvider Trait Extension & MLX Sidecar Command

**User Story:** As the Co-Pilot agent, I need a provider-agnostic method to send an audio file plus text context and receive structured analysis, so I can produce summaries, questions, and concept alerts from each audio chunk — and so the backend can be swapped to a cloud API without changing agent code.

### Acceptance Criteria

1. THE `IntelProvider` trait (`provider.rs`) SHALL be extended with a new method:
   ```rust
   async fn copilot_analyze(&self, audio_path: &Path, context: &str) -> Result<CoPilotCycleResult, String>
   ```
   with a default implementation returning `Err("Co-Pilot analysis not supported by this provider")`.
2. THE `CoPilotCycleResult` struct SHALL be defined in `provider.rs` (alongside `TranscriptResult`) with fields: `new_content` (String), `updated_summary` (String), `key_points` (Vec<String>), `decisions` (Vec<String>), `action_items` (Vec<String>), `open_questions` (Vec<String>), `suggested_questions` (Vec of `{question, reason}`), `key_concepts` (Vec of `{term, context}`)
3. THE `MlxProvider` SHALL implement `copilot_analyze` by sending a `copilot-analyze` NDJSON command to the sidecar with `audio_path` and `context` parameters, parsing the response into `CoPilotCycleResult`
4. THE MLX sidecar (`server.py`) SHALL support the `copilot-analyze` command that loads the audio file, constructs a prompt including the running context, and returns structured JSON
5. THE prompt SHALL request structured JSON output containing all fields defined in `CoPilotCycleResult`
6. THE `copilot_analyze` method SHALL have a timeout of 120 seconds
7. THE method SHALL gracefully handle cases where the model returns partial or malformed JSON by returning whatever fields were successfully parsed
8. WHEN `context` is empty (first cycle), THE prompt SHALL indicate this is the start of the conversation
9. THE Co-Pilot agent SHALL call `provider.copilot_analyze(audio_path, context)` — never constructing NDJSON commands directly or referencing `MlxProvider` by concrete type

---

## Requirement 3: Agent Lifecycle Management

**User Story:** As a user, I want to start and stop the Co-Pilot agent independently from the recording, so I can choose when to use AI assistance and save compute when I don't need it.

### Acceptance Criteria

1. THE System SHALL expose a `start_copilot` Tauri command that begins the agent cycle loop
2. THE System SHALL expose a `stop_copilot` Tauri command that stops the agent cycle loop and returns the final agent state
3. THE System SHALL expose a `get_copilot_state` Tauri command that returns the current agent state at any time
4. THE `start_copilot` command SHALL only succeed when a recording is active; it SHALL return an error if no recording is in progress
5. THE `stop_copilot` command SHALL stop the agent gracefully: wait for any in-flight inference to complete (up to the 120s timeout), then stop
6. THE agent SHALL automatically stop when the recording stops, without requiring a separate `stop_copilot` call
7. THE agent cycle loop SHALL run on a background tokio task, not blocking the main thread or the recording/transcription pipeline
8. THE System SHALL prevent multiple concurrent Co-Pilot instances — calling `start_copilot` while one is already running SHALL return an error
9. THE agent state SHALL be stored in a `Mutex<CoPilotState>` managed by Tauri app state, accessible from both commands and the cycle loop

---

## Requirement 4: Agent Cycle Loop

**User Story:** As the Co-Pilot agent, I need to run an inference cycle every 60–90 seconds during recording, feeding each cycle's output back as context for the next cycle, so I maintain a continuously updated understanding of the conversation.

### Acceptance Criteria

1. THE cycle loop SHALL execute at the interval specified by `copilot.cycle_interval` setting (default: 60 seconds, range: 30–120s), measured from the **end** of one cycle to the **start** of the next
2. EACH cycle SHALL perform these steps in order: (a) extract audio chunk (with overlap per `copilot.audio_overlap`), (b) call `provider.copilot_analyze(audio_path, context)` via the `IntelProvider` trait, (c) parse response, (d) update agent state, (e) emit Tauri event, (f) append to agent log if `copilot.agent_logging` is enabled
3. THE running context for cycle N SHALL be the `updated_summary` field returned by cycle N-1
4. FOR cycle 1 (first cycle), the running context SHALL be empty and the audio chunk SHALL cover all audio recorded so far
5. IF a cycle's inference fails or times out, THE System SHALL skip that cycle, keep the existing running context unchanged, increment a `failed_cycles` counter, and proceed to the next cycle
6. IF three consecutive cycles fail, THE System SHALL emit a `copilot-error` event and pause the agent (user can restart)
7. THE System SHALL track and expose cycle metadata: `cycle_number`, `last_updated_at` (timestamp), `processing` (boolean), `failed_cycles` (count)
8. THE cycle loop SHALL respect recording state — if the recording stops mid-cycle, the agent SHALL complete the in-flight inference then stop

---

## Requirement 5: Agent State Structure

**User Story:** As a developer, I want a well-defined agent state structure that holds all Co-Pilot output, so the frontend can render it and the gem system can persist it after recording.

### Acceptance Criteria

1. THE `CoPilotState` struct SHALL contain: `running_summary` (String), `key_points` (Vec<String>), `decisions` (Vec<String>), `action_items` (Vec<String>), `open_questions` (Vec<String>), `suggested_questions` (Vec<SuggestedQuestion>), `key_concepts` (Vec<KeyConcept>), `cycle_metadata` (CycleMetadata)
2. THE `SuggestedQuestion` struct SHALL contain: `question` (String), `reason` (String), `cycle_added` (u32), `dismissed` (bool)
3. THE `KeyConcept` struct SHALL contain: `term` (String), `context` (String), `cycle_added` (u32), `mention_count` (u32)
4. THE `CycleMetadata` struct SHALL contain: `cycle_number` (u32), `last_updated_at` (String, ISO 8601), `processing` (bool), `failed_cycles` (u32), `total_audio_seconds` (u64)
5. WHEN a new cycle completes, THE System SHALL replace `running_summary` with the model's `updated_summary`, append new items to `key_points`/`decisions`/`action_items`/`open_questions` (deduplicating against existing), replace `suggested_questions` with the latest cycle's output (max 5, preserving dismissed state for questions with matching text), and merge `key_concepts` (incrementing `mention_count` for existing terms)
6. ALL state fields SHALL be serializable to JSON via `serde::Serialize` for Tauri event emission and gem storage
7. THE state SHALL be resettable to empty when a new recording starts

---

## Requirement 6: Tauri Events — Frontend Communication

**User Story:** As the frontend, I need to receive real-time updates from the Co-Pilot agent via Tauri events, so I can display the latest summary, questions, and concepts without polling.

### Acceptance Criteria

1. THE System SHALL emit a `copilot-updated` event after each successful cycle, with the full `CoPilotState` as the payload
2. THE System SHALL emit a `copilot-status` event when the agent status changes, with payload `{ status: "starting" | "active" | "processing" | "paused" | "stopped" | "error", message?: string }`
3. THE System SHALL emit a `copilot-error` event when a cycle fails, with payload `{ cycle: number, error: string }`
4. THE `copilot-updated` event SHALL be emitted only when the state actually changes (not on failed/skipped cycles)
5. THE events SHALL follow the existing Tauri event pattern: `app_handle.emit("event-name", payload)`

---

## Requirement 7: Frontend — Co-Pilot Tab in Right Panel

**User Story:** As a user, I want a "Co-Pilot" tab next to the "Transcript" tab in the right panel during recording, so I can switch between viewing the raw transcript and the agent's structured analysis.

### Acceptance Criteria

1. WHEN a recording is active AND the user is on the Record nav, THE right panel SHALL display tab buttons: `[Transcript]` and `[Co-Pilot]`
2. THE Transcript tab SHALL show the existing `TranscriptDisplay` component (no changes)
3. THE Co-Pilot tab SHALL show a new `CoPilotPanel` component
4. THE tab buttons SHALL use design tokens: `font-size: var(--text-sm)`, `font-weight: var(--font-medium)`, active tab with `color: var(--accent-primary)` and bottom border `2px solid var(--accent-primary)`, inactive tab with `color: var(--text-secondary)`
5. WHEN the user is on the Transcript tab and a `copilot-updated` event arrives, THE Co-Pilot tab SHALL show a dot indicator (same style as YouTube notification badge) to signal new content
6. THE dot indicator SHALL clear when the user switches to the Co-Pilot tab
7. WHEN no recording is active, THE right panel SHALL show the existing placeholder text with no tabs
8. THE active tab state SHALL be stored in component state and default to `Transcript`

---

## Requirement 8: Frontend — CoPilotPanel Component

**User Story:** As a user, I want the Co-Pilot panel to display a rolling summary, suggested questions, and key concepts in clearly separated sections, so I can quickly scan actionable intelligence during my conversation.

### Acceptance Criteria

1. THE `CoPilotPanel` component SHALL display four sections in order: Summary, Decisions & Action Items, Suggested Questions, Key Concepts
2. THE Summary section SHALL show `running_summary` as body text, followed by `key_points` as a bullet list, followed by `open_questions` as a bullet list with a warning/attention style
3. THE Decisions & Action Items section SHALL show `decisions` as a checklist (checkmark prefix) and `action_items` as a bullet list
4. THE Suggested Questions section SHALL show each `SuggestedQuestion` as a card with the question text, a smaller `reason` line below, a dismiss `[×]` button, and a copy-to-clipboard action on click
5. WHEN a suggested question is dismissed, THE System SHALL call a `dismiss_copilot_question` Tauri command with the question index, and the question SHALL fade out
6. WHEN a suggested question is clicked (not the dismiss button), THE System SHALL copy the question text to the clipboard and show a brief "Copied" indicator
7. THE Key Concepts section SHALL show each `KeyConcept` as a chip/pill with the term and mention count, with the `context` shown as a tooltip or expanded text below
8. THE component SHALL show a status footer: cycle number, time since last update, and agent status indicator
9. WHEN the agent is processing a cycle, THE status footer SHALL show a subtle pulse animation
10. WHEN the agent state is empty (before first cycle completes), THE component SHALL show a placeholder: "Co-Pilot is listening... first analysis in ~60 seconds"
11. THE component SHALL subscribe to `copilot-updated` and `copilot-status` Tauri events and update in real-time

---

## Requirement 9: Frontend — Co-Pilot Toggle

**User Story:** As a user, I want a toggle switch on the recording screen to enable/disable the Co-Pilot agent, so I can control when AI analysis runs and save compute when I don't need it.

### Acceptance Criteria

1. THE center panel Record section SHALL show a "Co-Pilot" toggle switch below the record button when a recording is active
2. THE toggle SHALL default to OFF
3. WHEN the toggle is switched ON, THE System SHALL call `start_copilot` Tauri command and show the Co-Pilot tab in the right panel
4. WHEN the toggle is switched OFF, THE System SHALL call `stop_copilot` Tauri command and hide the Co-Pilot tab (revert to Transcript-only view)
5. THE toggle SHALL be disabled when no recording is active (grayed out with tooltip: "Start recording to enable Co-Pilot")
6. THE toggle SHALL use design tokens: track background `var(--bg-elevated)`, active track `var(--accent-primary)`, thumb `var(--text-primary)`, label `font-size: var(--text-sm)`, `color: var(--text-secondary)`
7. WHEN the recording stops, THE toggle SHALL automatically reset to OFF

---

## Requirement 10: Gem Integration — Post-Recording Enrichment

**User Story:** As a user, I want the Co-Pilot agent's output (summary, decisions, action items, concepts) to be saved into the gem when I save a recording as a gem, so I get a richer record than just a transcript.

### Acceptance Criteria

1. WHEN a recording with active Co-Pilot data is saved as a gem, THE System SHALL include the final `CoPilotState` in the gem's `source_meta` field under a `copilot` key
2. THE `copilot` field SHALL contain: `summary` (the final running_summary), `key_points`, `decisions`, `action_items`, `open_questions`, `key_concepts` (array of {term, context}), `total_cycles` (number of successful cycles), `total_audio_analyzed_seconds`
3. THE `GemDetailPanel` component SHALL detect the presence of `source_meta.copilot` and render the Co-Pilot summary, decisions, action items, and concepts in dedicated sections
4. THE Co-Pilot sections in the gem detail SHALL use the same styling as the live `CoPilotPanel` but without the interactive elements (no dismiss buttons, no copy-on-click, no status footer)
5. THE existing `ai_enrichment` (tags, summary from enrichment) SHALL remain separate from the `copilot` data — both can coexist on the same gem
6. WHEN a gem has both `ai_enrichment` and `copilot` data, THE `GemDetailPanel` SHALL show Co-Pilot sections first (they are more detailed), then AI enrichment tags/summary below

---

## Requirement 11: Concurrency and Resource Management

**User Story:** As a developer, I need the Co-Pilot agent to coexist with the recording and transcription pipeline without resource conflicts, so all three systems run reliably in parallel.

### Acceptance Criteria

1. THE Co-Pilot agent SHALL read audio from the recording file directly (file read, not via the AudioRouter mpsc channel) to avoid interfering with the transcription pipeline
2. THE Co-Pilot SHALL access the intelligence provider through `Arc<dyn IntelProvider>` (the same instance used for gem enrichment), ensuring concurrency is managed by the provider implementation (e.g., `MlxProvider` uses an internal mutex)
3. IF the provider is busy (e.g., MLX mutex held by another operation), THE Co-Pilot cycle SHALL wait up to 30 seconds, then skip the cycle if still blocked. The timeout behavior SHALL be handled by the provider's `copilot_analyze` implementation or by the agent wrapping the call in `tokio::time::timeout`
4. THE Co-Pilot SHALL NOT prevent or delay recording start/stop operations
5. THE temporary WAV files for audio chunks SHALL be written to the system temp directory (not the recordings directory) and cleaned up after each cycle
6. THE Co-Pilot's background tokio task SHALL be properly cancelled when the agent stops, releasing all resources

---

## Requirement 12: Co-Pilot Settings

**User Story:** As a user, I want to configure Co-Pilot agent parameters (cycle interval, audio overlap, logging) in Settings, so I can tune the agent for my use case without recompiling the app.

### Acceptance Criteria

1. THE System SHALL add a new `CopilotSettings` struct to the settings system with these fields:

   | Field | Type | Default | Range | Description |
   |---|---|---|---|---|
   | `enabled` | `bool` | `false` | — | Whether Co-Pilot starts automatically when recording begins |
   | `cycle_interval` | `u64` | `60` | 30–120 | Seconds between agent cycles |
   | `audio_overlap` | `u64` | `5` | 0–15 | Seconds of overlap between consecutive audio chunks |
   | `agent_logging` | `bool` | `true` | — | Whether to write prompt/response logs to disk |

2. THE System SHALL add `#[serde(default)]` on the `copilot` field in the main `Settings` struct so that existing `settings.json` files without a `copilot` key deserialize correctly with defaults
3. THE System SHALL expose Co-Pilot settings in the Settings UI as a new "Co-Pilot" section with:
   - A toggle for `enabled` (label: "Auto-start with recording", default OFF)
   - A slider for `cycle_interval` (label: "Analysis interval", 30s–120s, step 10s, showing current value)
   - A slider for `audio_overlap` (label: "Audio overlap", 0s–15s, step 1s, showing current value)
   - A toggle for `agent_logging` (label: "Save agent logs", default ON)
4. THE Settings UI sliders SHALL use the same design tokens and component patterns as the existing VAD threshold slider
5. WHEN the user changes Co-Pilot settings during an active recording THEN THE System SHALL apply `cycle_interval` and `audio_overlap` changes starting from the next cycle (not mid-cycle)
6. THE `CopilotSettings` struct SHALL follow the same pattern as `TranscriptionSettings`, `BrowserSettings`, and `IntelligenceSettings` — with `Default` impl and `#[serde(default = "...")]` on each field

---

## Requirement 13: Agent Logging (Audit Trail)

**User Story:** As a developer, I want a human-readable markdown log of every agent prompt and response, so I can review what worked well, debug quality issues, and tune the agent prompt over time.

### Acceptance Criteria

1. WHEN `copilot.agent_logging` is enabled in settings THEN THE System SHALL create a markdown log file at:
   ```
   ~/Library/Application Support/com.jarvis.app/agent_logs/YYYYMMDD_HHMMSS_copilot.md
   ```
   The timestamp SHALL match the recording's timestamp for easy correlation with the PCM file.

2. THE System SHALL write the following header when the log file is created:
   ```markdown
   # Co-Pilot Agent Log — YYYY-MM-DD HH:MM:SS

   **Recording:** YYYYMMDD_HHMMSS.pcm
   **Settings:** cycle_interval=60s, audio_overlap=5s
   **Model:** <model display name>
   ```

3. THE System SHALL append to the log file after each cycle:
   ```markdown
   ---

   ## Cycle N — MM:SS → MM:SS

   **Audio chunk:** M:SS–M:SS (Xs, includes Ys overlap)
   **Inference time:** X.Xs
   **Status:** success | error | skipped

   ### Prompt
   <full prompt text sent to model, including system prompt and running context>

   ### Response
   <full JSON response from model, or error message>
   ```

4. WHEN the agent stops (recording ends or user toggles off) THEN THE System SHALL append a summary section:
   ```markdown
   ---

   ## Summary

   | Metric | Value |
   |---|---|
   | Total cycles | N |
   | Successful | N |
   | Skipped | N |
   | Errors | N |
   | Avg inference time | X.Xs |
   | Total recording duration | Xm Ys |
   ```

5. THE System SHALL use append-only file writes — non-blocking, no file locking beyond the single writer
6. WHEN `copilot.agent_logging` is disabled THEN THE System SHALL NOT create or write to any log file
7. THE System SHALL NOT log raw audio binary data — only the file path reference to the temporary WAV chunk
8. THE `agent_logs/` directory SHALL be created automatically on first use if it does not exist

---

## Technical Constraints

1. **Provider-agnostic inference**: The Co-Pilot agent calls `IntelProvider::copilot_analyze()`, not `MlxProvider` directly. For V1, the only implementation is `MlxProvider` (local MLX sidecar), but the architecture supports future API providers as a single point of change.
2. **Concurrency — Option A (frozen)**: Real-time transcription uses whisper-rs or WhisperKit. Co-Pilot uses the `IntelProvider` (currently `MlxProvider` with internal mutex) for Qwen Omni. No scheduling needed.
3. **Qwen Omni required**: The Co-Pilot requires a multimodal model with audio understanding. It SHALL check model capabilities before starting and return an error if the active model does not support audio.
4. **Single CSS file**: All new CSS goes in `App.css` using existing design tokens. No new CSS files.
5. **Existing sidecar protocol**: The new `copilot-analyze` command follows the existing NDJSON protocol. No protocol changes.
6. **PCM format**: Audio is 16kHz, 16-bit signed, mono. WAV conversion uses the same header logic as existing `convert_to_wav`.
7. **Rust module structure**: New agent code goes in `src-tauri/src/agents/` as a new module. New Tauri commands registered in `lib.rs`.
8. **No recording changes**: The recording and transcription pipeline SHALL NOT be modified. The Co-Pilot is additive only.
9. **Settings backward compatibility**: Existing `settings.json` without `copilot` key must deserialize correctly via `#[serde(default)]`.
10. **Temporary files**: Audio chunks written to system temp dir must be cleaned up after each cycle.
11. **Agent log size**: Each log file is ~50–200KB depending on meeting length. No automatic cleanup — user manages disk space.
12. **Memory**: Qwen 2.5 Omni 3B (8-bit) needs ~5.3 GB. The agent and gem enrichment share the same loaded model instance.

## Out of Scope

1. Speaker diarization — the agent does not track who said what
2. Multiple concurrent agents — only one Co-Pilot instance at a time
3. Custom prompt configuration — the analysis prompt is hardcoded for V1
4. Agent memory across recordings — each recording starts with fresh state
5. Offline concept lookup — "Key Concepts" are flagged but not auto-defined
6. Adaptive cycle intervals based on speech density — deferred to future polish phase
7. Agent log browsing UI — logs exist as markdown files but no in-app viewer
8. Cloud-based agent inference
