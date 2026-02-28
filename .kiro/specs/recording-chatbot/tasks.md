# Implementation Plan: Recording Chatbot — Trait-Based Reusable Chat Component

## Overview

This implementation adds a reusable chatbot system built on a `Chatable` trait. Any content source implementing `Chatable` gets a chatbot — the chatbot fetches context from the source, sends it to the LLM via an `IntelQueue`, and logs every exchange to a markdown file.

The first conformer is `RecordingChatSource`, which makes recordings chatbot-compatible. An `IntelQueue` serializes all LLM requests from all agents (chatbot, co-pilot, enrichment) through a single mpsc channel with oneshot response routing, replacing direct provider calls.

## Implementation Phases

### Phase 1: IntelQueue — Request Serialization Infrastructure (Tasks 1-3)

**Goal:** Create the IntelQueue that serializes all IntelProvider access. This is foundational — both the chatbot and the migration of existing callers depend on it.

**Tasks:**
- Task 1: Create IntelQueue module
- Task 2: Wire IntelQueue into app state
- Task 3: Checkpoint — Verify queue processes requests end-to-end

**Validation:** A request submitted to the queue reaches the provider and the response routes back to the caller.

---

### Phase 2: Chat Capability — Provider & Sidecar (Tasks 4-6)

**Goal:** Add `chat()` to the IntelProvider trait and implement it in MlxProvider + sidecar.

**Tasks:**
- Task 4: Extend IntelProvider trait with chat() method
- Task 5: Implement chat command in MLX sidecar
- Task 6: Implement chat() in MlxProvider

**Validation:** A multi-turn conversation submitted via `provider.chat()` returns a text response.

---

### Phase 3: Chatable Trait & Chatbot Engine (Tasks 7-10)

**Goal:** Define the Chatable trait, build the reusable Chatbot engine, and implement RecordingChatSource.

**Tasks:**
- Task 7: Define Chatable trait
- Task 8: Implement Chatbot engine
- Task 9: Implement RecordingChatSource
- Task 10: Checkpoint — Verify chatbot works with RecordingChatSource via queue

**Validation:** Chatbot can start a session with a recording, generate transcript if missing, send messages, and log to `.md`.

---

### Phase 4: Tauri Commands & Wiring (Tasks 11-13)

**Goal:** Expose chatbot functionality to the frontend via Tauri commands.

**Tasks:**
- Task 11: Implement Tauri chat commands
- Task 12: Register commands and state in lib.rs
- Task 13: Checkpoint — Verify commands work from frontend invoke

**Validation:** Frontend can call `chat_with_recording`, `chat_send_message`, `chat_get_history`, `chat_end_session`.

---

### Phase 5: Frontend — ChatPanel & Integration (Tasks 14-18)

**Goal:** Build the ChatPanel component and integrate it into the recording UI.

**Tasks:**
- Task 14: Create ChatPanel component
- Task 15: Add Chat button to RecordingDetailPanel
- Task 16: Add Chat tab to RightPanel
- Task 17: Add CSS styles for chat components
- Task 18: Checkpoint — Verify full UI flow works

**Validation:** User can click Chat on a recording, see transcript preparation, send messages, and view responses.

---

### Phase 6: Migration & End-to-End Testing (Tasks 19-21)

**Goal:** Migrate existing provider callers to IntelQueue and validate the complete system.

**Tasks:**
- Task 19: Migrate existing callers to IntelQueue
- Task 20: End-to-end integration testing
- Task 21: Final checkpoint — All requirements validated

**Validation:** All agents use the queue. Chat, Co-Pilot, and enrichment coexist without interference.

---

## Tasks

### Phase 1: IntelQueue — Request Serialization Infrastructure

- [x] 1. Create IntelQueue module
  - [x] 1.1 Create `jarvis-app/src-tauri/src/intelligence/queue.rs`
    - Define `IntelRequest` struct with `command: IntelCommand` and `reply_tx: oneshot::Sender`
    - Define `IntelCommand` enum with variants: `Chat`, `GenerateTranscript`, `CopilotAnalyze`, `GenerateTags`, `Summarize`
    - Define `IntelResponse` enum with corresponding variants: `Chat(String)`, `Transcript(TranscriptResult)`, `CopilotAnalysis(CoPilotCycleResult)`, `Tags(Vec<String>)`, `Summary(String)`
    - Define `IntelQueue` struct with `tx: mpsc::Sender<IntelRequest>`
    - _Requirements: 4.1, 4.2, 4.3, 4.4_

  - [x] 1.2 Implement IntelQueue worker loop
    - `IntelQueue::new(provider: Arc<dyn IntelProvider>)` — creates mpsc channel (buffer: 32), spawns tokio task
    - Worker loop: `recv()` → match command → call appropriate provider method → `reply_tx.send(result)`
    - Match arms: `Chat` → `provider.chat()`, `GenerateTranscript` → `provider.generate_transcript()`, `CopilotAnalyze` → `provider.copilot_analyze()`, `GenerateTags` → `provider.generate_tags()`, `Summarize` → `provider.summarize()`
    - _Requirements: 4.5_

  - [x] 1.3 Implement `submit()` method
    - Create oneshot channel
    - Send `IntelRequest { command, reply_tx }` to mpsc
    - Await oneshot receiver
    - Return `Result<IntelResponse, String>` with errors for "Queue closed" and "Worker dropped"
    - _Requirements: 4.2_

  - [x] 1.4 Export queue module from `intelligence/mod.rs`
    - Add `pub mod queue;` to `intelligence/mod.rs`
    - Re-export `IntelQueue`, `IntelCommand`, `IntelResponse`
    - _Requirements: 4.6_

- [x] 2. Wire IntelQueue into app state
  - [x] 2.1 Create IntelQueue instance in `lib.rs` setup
    - Create `IntelQueue::new(provider.clone())` after provider initialization
    - Add `app.manage(intel_queue)` to make it available to Tauri commands
    - _Requirements: 4.6_

- [x] 3. Checkpoint — Verify queue processes requests end-to-end
  - Verify the queue compiles and the worker task spawns correctly
  - Verify existing provider methods are callable through the queue's match arms
  - _Requirements: 4.1–4.6_

---

### Phase 2: Chat Capability — Provider & Sidecar

- [x] 4. Extend IntelProvider trait with chat() method
  - [x] 4.1 Add `chat()` method to IntelProvider trait in `provider.rs`
    - Signature: `async fn chat(&self, messages: &[(String, String)]) -> Result<String, String>`
    - Default implementation: `Err("Chat not supported by this provider")`
    - Messages are `(role, content)` tuples where role is "system", "user", or "assistant"
    - _Requirements: 2.1, 2.2_

  - [ ]* 4.2 Add default chat() to NoOpProvider and IntelligenceKitProvider
    - Both use the default (unsupported) implementation — no code changes needed if default works
    - Verify compilation
    - _Requirements: 2.2_

- [x] 5. Implement chat command in MLX sidecar
  - [x] 5.1 Add `handle_chat()` method to MLXServer class in `server.py`
    - Accept `messages` (array of `{role, content}` objects) and `model_path`
    - Build conversation from messages array
    - Apply chat template via `self.tokenizer.apply_chat_template(conversation, add_generation_prompt=True)`
    - Generate with `mlx_lm_generate` (text-only, not omni) with `max_tokens=2048`
    - Return `{"type": "response", "command": "chat", "response": "..."}`
    - Handle errors: return `{"type": "error", "command": "chat", "error": "..."}`
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

  - [x] 5.2 Add `chat` command to NDJSON command dispatch
    - Add `"chat"` case to the command dispatch in the main NDJSON loop
    - Call `handle_chat()` with the parsed data
    - _Requirements: 3.4_

- [x] 6. Implement chat() in MlxProvider
  - [x] 6.1 Implement `chat()` method in `mlx_provider.rs`
    - Build NDJSON command: `{"command": "chat", "messages": [...], "model_path": "..."}`
    - Send via `self.send_command()` with 120s timeout
    - Parse `response` field from result
    - Return `Ok(response_text)` or `Err(error_message)`
    - _Requirements: 2.3, 2.4, 2.5_

  - [ ]* 6.2 Write integration test for chat round-trip
    - Send a simple conversation through MlxProvider.chat()
    - Verify response is returned correctly
    - _Requirements: 2.1–2.5_

---

### Phase 3: Chatable Trait & Chatbot Engine

- [x] 7. Define Chatable trait
  - [x] 7.1 Create `jarvis-app/src-tauri/src/agents/chatable.rs`
    - Define `Chatable` async trait with `Send + Sync` bounds
    - Methods: `get_context(&self, intel_queue: &IntelQueue) -> Result<String, String>`, `label(&self) -> String`, `session_dir(&self) -> PathBuf`, `needs_preparation(&self) -> bool`, `on_preparation_status(&self, status: &str, message: &str)` (default no-op)
    - _Requirements: 1.1, 1.2, 1.3, 1.4_

  - [x] 7.2 Export from `agents/mod.rs`
    - Add `pub mod chatable;` to `agents/mod.rs`
    - _Requirements: 1.1_

- [x] 8. Implement Chatbot engine
  - [x] 8.1 Create `jarvis-app/src-tauri/src/agents/chatbot.rs`
    - Define `Chatbot` struct with `sessions: HashMap<String, ChatSession>`
    - Define `ChatSession` struct with `session_id`, `messages: Vec<ChatMessage>`, `log_path`, `created_at`
    - Define `ChatMessage` struct with `role`, `content`, `timestamp` — derive `Serialize`, `Deserialize`, `Clone`
    - _Requirements: 5.1_

  - [x] 8.2 Implement `start_session()`
    - Call `source.get_context(intel_queue)` to trigger preparation if needed
    - Generate session_id: `format!("chat_{}", chrono::Utc::now().timestamp())`
    - Create log file path: `source.session_dir().join(format!("chat_session_{}.md", timestamp))`
    - Write session header to log file (label, start time)
    - Create `ChatSession` and insert into sessions map
    - Return session_id
    - _Requirements: 5.2, 7.1, 7.2_

  - [x] 8.3 Implement `send_message()`
    - Call `source.get_context(intel_queue)` for fresh context (every message, not cached)
    - Build system prompt with truncated context (last 14,000 chars)
    - Append chat history (last 10 exchanges = 20 messages)
    - Append user message
    - Submit `IntelCommand::Chat { messages }` to intel_queue
    - Record user + assistant messages in session
    - Append exchange to session `.md` log (append-only)
    - Return assistant response text
    - _Requirements: 5.2, 5.3, 5.4, 5.5, 5.6_

  - [x] 8.4 Implement `get_history()` and `end_session()`
    - `get_history`: return clone of session messages, error if not found
    - `end_session`: remove session from HashMap
    - _Requirements: 5.2_

  - [x] 8.5 Implement `truncate_context()` helper
    - If text ≤ max_chars, return full text
    - Otherwise return tail (last max_chars characters) — most recent content is most relevant
    - _Requirements: 5.5_

  - [x] 8.6 Export from `agents/mod.rs`
    - Add `pub mod chatbot;` to `agents/mod.rs`
    - _Requirements: 5.8_

- [x] 9. Implement RecordingChatSource
  - [x] 9.1 Create `jarvis-app/src-tauri/src/agents/recording_chat.rs`
    - Define `RecordingChatSource` struct with `app_handle: AppHandle`, `filename: String`, `recordings_dir: PathBuf`
    - Implement `new(app_handle, filename)` — resolve recordings_dir via `get_recordings_dir()`
    - Implement helper `stem()` — filename without `.pcm`
    - Implement helper `transcript_path()` — `{recordings_dir}/{stem}_transcript.md`
    - _Requirements: 6.1, 6.2_

  - [x] 9.2 Implement Chatable for RecordingChatSource
    - `get_context()`:
      - If transcript file exists → read from disk (fast path)
      - If not → emit "preparing" status → convert PCM to WAV → submit `GenerateTranscript` via intel_queue → save transcript to `{stem}_transcript.md` → emit "ready" status → return text
    - `label()`: `format!("Recording {}", self.stem())`
    - `session_dir()`: `self.recordings_dir.clone()`
    - `needs_preparation()`: `!self.transcript_path().exists()`
    - `on_preparation_status()`: emit `chat-status` Tauri event with `{ status, message }`
    - _Requirements: 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8_

  - [x] 9.3 Export from `agents/mod.rs`
    - Add `pub mod recording_chat;` to `agents/mod.rs`
    - _Requirements: 6.1_

- [x] 10. Checkpoint — Verify chatbot works with RecordingChatSource via queue
  - Verify all three modules compile
  - Verify RecordingChatSource can be constructed from a filename
  - Verify chatbot.start_session() creates a session and log file
  - Verify chatbot.send_message() routes through the queue and returns a response
  - _Requirements: 1–7_

---

### Phase 4: Tauri Commands & Wiring

- [x] 11. Implement Tauri chat commands
  - [x] 11.1 Implement `chat_with_recording` command in `commands.rs`
    - Accept `recording_filename: String`
    - Create `RecordingChatSource::new(app_handle, recording_filename)`
    - Lock `state.chatbot` (TokioMutex)
    - Call `chatbot.start_session(&source, &intel_queue)`
    - Return session_id
    - _Requirements: 8.1_

  - [x] 11.2 Implement `chat_send_message` command in `commands.rs`
    - Accept `session_id: String`, `recording_filename: String`, `message: String`
    - Recreate `RecordingChatSource` from filename (stateless — cheap to construct)
    - Lock `state.chatbot`
    - Call `chatbot.send_message(&session_id, &message, &source, &intel_queue)`
    - Return assistant response string
    - _Requirements: 8.2_

  - [x] 11.3 Implement `chat_get_history` command in `commands.rs`
    - Accept `session_id: String`
    - Lock `state.chatbot`
    - Return `chatbot.get_history(&session_id)`
    - _Requirements: 8.3_

  - [x] 11.4 Implement `chat_end_session` command in `commands.rs`
    - Accept `session_id: String`
    - Lock `state.chatbot`
    - Call `chatbot.end_session(&session_id)`
    - _Requirements: 8.4_

- [x] 12. Register commands and state in lib.rs
  - [x] 12.1 Add Chatbot to managed state
    - `app.manage(TokioMutex::new(Chatbot::new()))` in setup
    - _Requirements: 5.7, 8.5_

  - [x] 12.2 Register chat commands in `generate_handler!`
    - Add `chat_with_recording`, `chat_send_message`, `chat_get_history`, `chat_end_session`
    - _Requirements: 8.5_

- [x] 13. Checkpoint — Verify commands work from frontend invoke
  - Verify all four commands are callable from the frontend
  - Verify session lifecycle: start → send messages → get history → end
  - _Requirements: 8.1–8.6_

---

### Phase 5: Frontend — ChatPanel & Integration

- [x] 14. Create ChatPanel component
  - [x] 14.1 Create `jarvis-app/src/components/ChatPanel.tsx`
    - Define `ChatPanelProps` interface: `sessionId`, `recordingFilename`, `status`, `preparingMessage?`, `placeholder?`
    - Define `ChatMessage` type: `{ role: 'user' | 'assistant', content: string }`
    - State: `messages`, `input`, `thinking`
    - _Requirements: 9.1, 9.2_

  - [x] 14.2 Implement preparing state view
    - When `status === 'preparing'`, show spinner + `preparingMessage` text
    - _Requirements: 9.3_

  - [x] 14.3 Implement chat message display
    - Scrollable message area with user bubbles (right-aligned) and assistant bubbles (left-aligned)
    - Empty state placeholder: "Ask me anything about this recording."
    - "Thinking..." bubble with pulse animation when waiting for response
    - Auto-scroll to latest message via `useRef` + `scrollIntoView`
    - _Requirements: 9.4, 9.5, 9.8_

  - [x] 14.4 Implement message sending
    - Text input + Send button at bottom
    - Send on Enter key or button click
    - Call `chat_send_message` Tauri command
    - Add user message immediately, add assistant response when received
    - Display errors as assistant message prefixed with "Error:"
    - Disable input while thinking
    - _Requirements: 9.6, 9.7, 9.9_

  - [x] 14.5 Listen for `chat-status` Tauri events
    - Update component status when preparation completes
    - _Requirements: 9.11_

- [x] 15. Add Chat button to RecordingDetailPanel
  - [x] 15.1 Add Chat button alongside existing action buttons
    - Button text: "Chat" (with chat icon)
    - Disabled when `!aiAvailable` (no model loaded) with tooltip
    - _Requirements: 10.1, 10.7_

  - [x] 15.2 Implement `handleStartChat` handler
    - Set chat status to 'preparing'
    - Call `chat_with_recording` Tauri command
    - Store returned session_id in state
    - Set chat status to 'ready'
    - Switch right panel to Chat tab
    - Handle errors
    - _Requirements: 10.2, 10.3_

- [x] 16. Add Chat tab to RightPanel
  - [x] 16.1 Add tab state management for recordings view
    - When chat session active: show `[Details]` and `[Chat]` tabs
    - Tab buttons with active/inactive styling matching Co-Pilot tab pattern
    - Default to Chat tab when session starts
    - _Requirements: 10.4, 10.5_

  - [x] 16.2 Render ChatPanel in Chat tab
    - Pass `sessionId`, `recordingFilename`, `status`, `preparingMessage` to ChatPanel
    - Show RecordingDetailPanel in Details tab
    - _Requirements: 10.2_

  - [x] 16.3 Handle session cleanup on navigation
    - Call `chat_end_session` when user navigates away from recording
    - Clear chat state
    - _Requirements: 10.6_

- [x] 17. Add CSS styles for chat components
  - [x] 17.1 Add chat panel styles to `App.css`
    - `.chat-panel` — flex column, full height
    - `.chat-messages` — flex-grow, overflow-y scroll, padding
    - `.chat-message`, `.chat-user`, `.chat-assistant` — flex layout, alignment
    - `.chat-bubble` — max-width 80%, padding, border-radius, `var(--radius-md)`
    - `.chat-user .chat-bubble` — `background: var(--accent-primary)`, white text
    - `.chat-assistant .chat-bubble` — `background: var(--bg-elevated)`, `color: var(--text-primary)`
    - `.chat-bubble.thinking` — opacity 0.6, pulse animation
    - _Requirements: 9.10_

  - [x] 17.2 Add input bar styles
    - `.chat-input-bar` — flex, gap, padding, border-top
    - Input — flex-grow, border-radius, `var(--bg-primary)`, `var(--text-primary)`
    - Button — `var(--accent-primary)` background, disabled opacity
    - _Requirements: 9.10_

  - [x] 17.3 Add preparing and empty state styles
    - `.chat-preparing` — centered spinner + text
    - `.chat-empty` — centered, `var(--text-secondary)`
    - _Requirements: 9.10_

- [x] 18. Checkpoint — Verify full UI flow works
  - Click Chat on a recording → session starts → transcript generates if needed
  - Send a message → response appears in chat bubbles
  - Navigate away → session cleans up
  - _Requirements: 9, 10_

---

### Phase 6: Migration & End-to-End Testing

- [x] 19. Migrate existing callers to IntelQueue
  - [x] 19.1 Migrate Co-Pilot agent to use IntelQueue
    - Replace direct `provider.copilot_analyze()` calls with `intel_queue.submit(CopilotAnalyze { ... })`
    - Pass `IntelQueue` reference to CoPilotAgent instead of `Arc<dyn IntelProvider>`
    - _Requirements: 4.7_

  - [x] 19.2 Migrate gem enrichment to use IntelQueue
    - Replace direct `provider.generate_tags()` and `provider.summarize()` calls with queue submissions
    - _Requirements: 4.7_

  - [ ]* 19.3 Migrate transcription commands to use IntelQueue
    - Replace direct `provider.generate_transcript()` calls with queue submissions
    - This is optional for V1 — transcription can continue using direct provider calls if migration is complex
    - _Requirements: 4.7_

- [x] 20. End-to-end integration testing
  - [x] 20.1 Test chat with recording that has no transcript
    - Click Chat → verify "Generating transcript..." shown
    - Verify transcript is generated and saved to disk
    - Verify chat is ready after transcript generation
    - Send messages and verify responses
    - _Requirements: 6.3, 6.4, 9.3_

  - [x] 20.2 Test chat with recording that has existing transcript
    - Generate transcript first (via existing transcribe button)
    - Click Chat → verify chat is immediately ready (no preparation)
    - Send messages and verify responses use the transcript
    - _Requirements: 6.3, 6.7_

  - [x] 20.3 Test session logging
    - Start a chat session and exchange several messages
    - Verify `.md` log file is created in recordings directory
    - Verify log contains header, all exchanges with timestamps, and correct formatting
    - _Requirements: 7.1–7.6_

  - [x] 20.4 Test concurrent agent access via queue
    - Start a Co-Pilot cycle and send a chat message simultaneously
    - Verify both get responses (sequentially, no crash)
    - Verify no response cross-talk between agents
    - _Requirements: 4.5, 4.7_

  - [x] 20.5 Test error scenarios
    - Chat without a model loaded → verify disabled button / error message
    - Send message with invalid session_id → verify error displayed
    - Provider timeout → verify error displayed in chat as "Error: ..."
    - _Requirements: 9.9, 10.7_

- [x] 21. Final checkpoint — All requirements validated
  - Verify all 10 requirements are met
  - Verify no disruption to existing recording, transcription, co-pilot, or gem workflows
  - Ensure all tests pass, ask the user if questions arise

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirement numbers for traceability
- Checkpoints (Tasks 3, 10, 13, 18, 21) ensure incremental validation — do not skip these
- Phase 1 (IntelQueue) is the most impactful change — it affects all agents. Build and validate it first.
- Phase 6 migration (Task 19) can be done incrementally — migrate one caller at a time
- The chatbot is reusable from Phase 3 onward. Adding `GemChatSource` or `LiveRecordingChatSource` later is a single-file addition with zero chatbot changes.
