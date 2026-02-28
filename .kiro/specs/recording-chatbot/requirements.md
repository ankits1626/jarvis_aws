# Recording Chatbot — Trait-Based Reusable Chat Component

## Introduction

Jarvis records audio and can generate transcripts, but users have no way to interactively ask questions about their recordings. They can read a transcript or summary, but cannot have a conversation — "what were the action items?", "summarize the pricing discussion", "who disagreed with the proposal?"

This spec defines a **reusable chatbot** built on a `Chatable` trait. Anything that implements `Chatable` gets a chatbot attached to it. The first implementation makes recordings conform to `Chatable`, so users can chat with any recording. The same chatbot can later be attached to gems, live recordings, or any new content type — zero chatbot changes needed.

**Reference:** Design elaboration in `discussion/28-feb-next-step/chat-bot/recording-chatbot.md`.

## Glossary

- **Chatable**: A Rust trait that any content source implements to become chatbot-compatible. Defines what context to provide, where to store sessions, and whether preparation is needed.
- **Chatbot**: The reusable chat engine. It talks to a `Chatable` source and an `IntelQueue`. It knows nothing about recordings, gems, or any specific content type.
- **RecordingChatSource**: The struct that makes recordings conform to `Chatable`. Handles transcript loading/generation.
- **IntelQueue**: An mpsc channel with oneshot response routing that serializes all IntelProvider requests across agents (chatbot, co-pilot, enrichment).
- **Chat Session**: One conversation between a user and the chatbot about a specific source. Produces an in-memory message list and a persistent `.md` log file.
- **Session Log**: A markdown file recording every user question and assistant response, stored in the directory specified by the `Chatable` source.
- **Context**: The text content the chatbot uses to answer questions. For a recording, this is the transcript. Each `Chatable` source provides its own context.
- **MLX Sidecar**: The Python process (`sidecars/mlx-server/server.py`) that runs LLM inference locally via MLX framework.

## Frozen Design Decisions

These decisions were made and locked during design review (2026-02-28):

1. **Trait-based architecture**: The chatbot works with any `Chatable` implementor. No special-casing for recordings, gems, or any content type. Adding a new chat-capable source means implementing `Chatable` — zero chatbot changes.
2. **Context freshness**: The chatbot calls `source.get_context()` on every message, not just session start. This handles both static sources (recording transcript — cheap disk read) and growing sources (live recording — always fresh).
3. **IntelQueue for concurrency**: All agents (chatbot, co-pilot, enrichment) submit requests through a shared mpsc queue. One worker loop processes requests sequentially. Each caller gets its response via a oneshot channel. No mutex contention on the provider.
4. **Session persistence as `.md`**: Every chat session produces a human-readable markdown log. No database schema changes. The `Chatable` source decides the storage directory.
5. **Provider abstraction**: The chatbot calls `IntelQueue.submit(IntelCommand::Chat {...})`, never the provider directly. The `IntelProvider` trait gets a new `chat()` method. Same abstraction pattern as transcription, tags, and summarization.
6. **Transcript persistence for recordings**: Generated transcripts are saved to `{recording_stem}_transcript.md` alongside the PCM file. Subsequent chat sessions reuse the saved transcript — no redundant generation.

---

## Requirement 1: Chatable Trait Definition

**User Story:** As a developer, I need a well-defined trait that any content source can implement to become chatbot-compatible, so the chatbot is reusable across recordings, gems, live recordings, and future content types without modification.

### Acceptance Criteria

1. THE System SHALL define a `Chatable` async trait in `src-tauri/src/agents/chatable.rs` with the following methods:
   - `async fn get_context(&self, intel_queue: &IntelQueue) -> Result<String, String>` — returns the text context for answering questions
   - `fn label(&self) -> String` — human-readable name for session log headers (e.g. "Recording 20260228_143022")
   - `fn session_dir(&self) -> PathBuf` — directory where session `.md` logs are stored
   - `async fn needs_preparation(&self) -> bool` — whether context needs generating before chat can begin
   - `fn on_preparation_status(&self, _status: &str, _message: &str) {}` — optional callback for preparation progress (default: no-op)
2. THE trait SHALL require `Send + Sync` bounds so it can be used across async tasks
3. THE `get_context()` method SHALL accept an `IntelQueue` reference so the source can submit generation requests (e.g. transcript generation) if needed
4. THE trait SHALL be the only interface the `Chatbot` uses to interact with content sources — the chatbot SHALL NOT import or reference any concrete source type

---

## Requirement 2: IntelProvider Trait Extension — Chat Method

**User Story:** As the chatbot, I need a provider-agnostic method to send a multi-turn conversation (system prompt + history + user message) and receive a text response, so the chatbot works regardless of the underlying LLM backend.

### Acceptance Criteria

1. THE `IntelProvider` trait (`provider.rs`) SHALL be extended with a new method:
   ```rust
   async fn chat(&self, messages: &[(String, String)]) -> Result<String, String>
   ```
   where each tuple is `(role, content)` with roles: "system", "user", "assistant"
2. THE method SHALL have a default implementation returning `Err("Chat not supported by this provider")`
3. THE `MlxProvider` SHALL implement `chat()` by sending a `chat` NDJSON command to the sidecar with `messages` and `model_path` parameters, parsing the `response` field from the result
4. THE `chat()` method SHALL have a timeout of 120 seconds
5. THE method SHALL support multi-turn conversations: the messages array may contain interleaved user/assistant pairs representing chat history

---

## Requirement 3: MLX Sidecar Chat Handler

**User Story:** As the MLX sidecar, I need to handle a `chat` command that accepts a conversation (array of role/content messages) and returns a text response, so the local LLM can power chatbot conversations.

### Acceptance Criteria

1. THE sidecar (`server.py`) SHALL support a new `chat` NDJSON command with fields: `messages` (array of `{role, content}` objects) and `model_path`
2. THE handler SHALL construct a conversation from the messages array and call the model's `generate()` function with `max_tokens=2048`
3. THE handler SHALL return a JSON response with a `response` field containing the generated text
4. THE handler SHALL follow the existing NDJSON protocol: read from stdin, write to stdout, one JSON object per line
5. THE handler SHALL handle errors gracefully, returning `{"error": "..."}` if generation fails

---

## Requirement 4: IntelQueue — Request Serialization and Response Routing

**User Story:** As a developer, I need a queue that serializes all IntelProvider requests from multiple agents (chatbot, co-pilot, enrichment) and routes each response back to its caller, so agents don't contend on the provider and each gets its own response.

### Acceptance Criteria

1. THE System SHALL create an `IntelQueue` in `src-tauri/src/intelligence/queue.rs` using a tokio `mpsc` channel for request submission and `oneshot` channels for per-request response routing
2. THE `IntelQueue` SHALL expose a `submit(command: IntelCommand) -> Result<IntelResponse, String>` method that sends the command and awaits the response via the oneshot channel
3. THE `IntelCommand` enum SHALL include variants: `Chat { messages: Vec<(String, String)> }`, `GenerateTranscript { audio_path: PathBuf }`, `CopilotAnalyze { audio_path: PathBuf, context: String }`, `GenerateTags { content: String }`, `Summarize { content: String }`
4. THE `IntelResponse` enum SHALL include corresponding variants: `Chat(String)`, `Transcript(TranscriptResult)`, `CopilotAnalysis(CoPilotCycleResult)`, `Tags(Vec<String>)`, `Summary(String)`
5. THE queue worker loop SHALL process requests sequentially — one at a time — by calling the appropriate `IntelProvider` method and sending the result back via the oneshot sender
6. THE `IntelQueue` SHALL be stored in Tauri app state as a shared instance accessible to all agents and commands
7. ALL existing callers of `IntelProvider` (co-pilot agent, gem enrichment) SHALL be migrated to use the `IntelQueue` instead of calling the provider directly

---

## Requirement 5: Chatbot — Reusable Chat Engine

**User Story:** As a user, I want to have a conversational chat about any content source, where the chatbot maintains my conversation history, logs every exchange to a file, and answers based on the source's context.

### Acceptance Criteria

1. THE System SHALL create a `Chatbot` struct in `src-tauri/src/agents/chatbot.rs` that manages multiple concurrent `ChatSession` instances keyed by session ID
2. THE `Chatbot` SHALL expose the following methods:
   - `start_session(source: &dyn Chatable, intel_queue: &IntelQueue) -> Result<String, String>` — creates a session, calls `source.get_context()` for preparation, writes session log header, returns session ID
   - `send_message(session_id: &str, user_message: &str, source: &dyn Chatable, intel_queue: &IntelQueue) -> Result<String, String>` — fetches fresh context, builds prompt, submits to queue, logs exchange, returns response
   - `get_history(session_id: &str) -> Result<Vec<ChatMessage>, String>` — returns in-memory message history
   - `end_session(session_id: &str)` — removes session from memory
3. THE `send_message` method SHALL call `source.get_context()` on every invocation to get fresh context (not cached from session start)
4. THE system prompt SHALL instruct the LLM: "Answer questions based on the following context. Be concise and accurate. If the answer isn't in the context, say so."
5. THE system prompt SHALL include the context text, truncated to the last 14,000 characters if it exceeds that length
6. THE chat history included in the LLM prompt SHALL be limited to the last 10 exchanges (20 messages) to stay within context window limits
7. THE `Chatbot` SHALL be stored in Tauri app state under a `TokioMutex` so it is accessible from Tauri commands
8. THE `Chatbot` SHALL NOT import or reference any concrete `Chatable` implementation — it only interacts through the trait

---

## Requirement 6: RecordingChatSource — Recording Conforms to Chatable

**User Story:** As a user, I want to chat with any recording I've made, where the chatbot automatically generates a transcript if one doesn't exist and answers my questions based on the transcript content.

### Acceptance Criteria

1. THE System SHALL create a `RecordingChatSource` struct in `src-tauri/src/agents/recording_chat.rs` that implements `Chatable`
2. THE `RecordingChatSource` SHALL be constructed from an `AppHandle` and a recording filename (e.g. `"20260228_143022.pcm"`)
3. THE `get_context()` implementation SHALL:
   - Check if a transcript file exists at `{recordings_dir}/{stem}_transcript.md`
   - If it exists, read and return the transcript text
   - If it does not exist, emit a "preparing" status, convert the PCM to WAV, submit a `GenerateTranscript` command via `IntelQueue`, save the result to `{stem}_transcript.md`, emit a "ready" status, and return the transcript text
4. THE transcript file SHALL be formatted as:
   ```markdown
   # Transcript — {recording_stem}

   **Generated:** YYYY-MM-DD HH:MM:SS

   ---

   {transcript text}
   ```
5. THE `label()` SHALL return `"Recording {stem}"` where stem is the filename without the `.pcm` extension
6. THE `session_dir()` SHALL return the recordings directory path
7. THE `needs_preparation()` SHALL return `true` if the transcript file does not exist on disk, `false` otherwise
8. THE `on_preparation_status()` SHALL emit a `chat-status` Tauri event with `{ status, message }` payload so the frontend can show progress

---

## Requirement 7: Chat Session Logging

**User Story:** As a user, I want every chat conversation saved as a readable markdown file alongside my recording, so I can review past conversations, export them, or reference them later.

### Acceptance Criteria

1. WHEN a new chat session starts, THE System SHALL create a markdown log file at `{session_dir}/chat_session_{unix_timestamp}.md`
2. THE session log SHALL begin with a header:
   ```markdown
   # Chat Session

   **Label:** {source.label()}
   **Started:** YYYY-MM-DD HH:MM:SS

   ---

   ```
3. AFTER each user message and assistant response, THE System SHALL append to the log:
   ```markdown
   ## User (HH:MM:SS)
   {user message}

   ## Assistant (HH:MM:SS)
   {assistant response}

   ---

   ```
4. THE System SHALL use append-only file writes (no rewriting the entire file)
5. THE log file directory SHALL be created automatically if it does not exist
6. THE log file location SHALL be determined by the `Chatable` source's `session_dir()` method — the chatbot does not decide where logs go

---

## Requirement 8: Tauri Commands — Chat API

**User Story:** As the frontend, I need Tauri commands to start a chat session with a recording, send messages, retrieve history, and end sessions, so the React UI can drive the chatbot.

### Acceptance Criteria

1. THE System SHALL expose a `chat_with_recording` Tauri command that:
   - Accepts `recording_filename: String`
   - Creates a `RecordingChatSource` from the filename
   - Calls `chatbot.start_session(&source, &intel_queue)`
   - Returns the `session_id` string
2. THE System SHALL expose a `chat_send_message` Tauri command that:
   - Accepts `session_id: String`, `recording_filename: String`, `message: String`
   - Recreates the `RecordingChatSource` from the filename (stateless — source is cheap to construct)
   - Calls `chatbot.send_message(session_id, message, &source, &intel_queue)`
   - Returns the assistant's response string
3. THE System SHALL expose a `chat_get_history` Tauri command that:
   - Accepts `session_id: String`
   - Returns `Vec<ChatMessage>` with `role`, `content`, and `timestamp` fields
4. THE System SHALL expose a `chat_end_session` Tauri command that:
   - Accepts `session_id: String`
   - Removes the session from memory
5. ALL chat commands SHALL be registered in `lib.rs` alongside existing commands
6. THE `RecordingChatSource` SHALL be constructed on each command invocation (not stored in app state) — the chatbot and intel queue are stateful, the source is not

---

## Requirement 9: Frontend — ChatPanel Component

**User Story:** As a user, I want a clean chat interface in the right panel where I can type questions and see responses in a familiar message-bubble layout, so interacting with my recording feels like a conversation.

### Acceptance Criteria

1. THE System SHALL create a reusable `ChatPanel` React component in `src/components/ChatPanel.tsx`
2. THE `ChatPanel` SHALL accept props: `sessionId` (string), `recordingFilename` (string), `status` ('preparing' | 'ready' | 'error'), `preparingMessage?` (string), `placeholder?` (string)
3. WHEN `status` is `'preparing'`, THE component SHALL show a spinner and the `preparingMessage` text (default: "Preparing...")
4. WHEN `status` is `'ready'`, THE component SHALL show:
   - A scrollable message area displaying all messages as bubbles (user messages right-aligned, assistant messages left-aligned)
   - A text input bar at the bottom with a Send button
5. WHEN the message area is empty, THE component SHALL show a placeholder: "Ask me anything about this recording."
6. WHEN the user submits a message (Enter key or Send button), THE component SHALL:
   - Add the user message to the display immediately
   - Show a "Thinking..." indicator
   - Call `chat_send_message` Tauri command with the session ID, recording filename, and message
   - Add the assistant response to the display
   - Remove the "Thinking..." indicator
7. THE input SHALL be disabled while a response is pending (thinking state)
8. THE message area SHALL auto-scroll to the latest message
9. IF the Tauri command returns an error, THE component SHALL display the error as an assistant message prefixed with "Error:"
10. THE component SHALL use existing design tokens from `App.css` for styling: `var(--bg-primary)`, `var(--bg-elevated)`, `var(--text-primary)`, `var(--text-secondary)`, `var(--accent-primary)`, `var(--text-sm)`, `var(--radius-md)`
11. THE component SHALL listen for `chat-status` Tauri events to update the `status` prop when preparation completes

---

## Requirement 10: Frontend — Recording Chat Integration

**User Story:** As a user, I want a "Chat" button on every recording and a Chat tab in the right panel, so I can start a conversation with any recording in one click.

### Acceptance Criteria

1. THE `RecordingDetailPanel` component SHALL show a "Chat" button alongside the existing action buttons for each recording
2. WHEN the user clicks the Chat button, THE System SHALL:
   - Call `chat_with_recording` Tauri command with the recording filename
   - Switch the right panel to show the Chat tab with the `ChatPanel` component
   - Pass the returned `session_id` and `recording_filename` to `ChatPanel`
3. IF the recording has no transcript (`needs_preparation` is true), THE `ChatPanel` SHALL show in 'preparing' status with message "Generating transcript..." until the transcript is ready
4. THE right panel SHALL display tab buttons `[Transcript]` and `[Chat]` when a chat session is active during recording view
5. THE tab buttons SHALL use existing design token patterns matching the Co-Pilot tab styling: active tab with `color: var(--accent-primary)` and bottom border, inactive with `color: var(--text-secondary)`
6. WHEN the user navigates away from the recording, THE System SHALL call `chat_end_session` to clean up the session
7. THE Chat button SHALL be disabled if no intelligence provider is configured (no model loaded)

---

## Technical Constraints

1. **Provider-agnostic inference**: The chatbot submits requests through `IntelQueue`, which calls `IntelProvider::chat()`. For V1, the only implementation is `MlxProvider` (local MLX sidecar). The same trait supports future API providers.
2. **Single IntelQueue**: All agents share one queue. Requests are processed sequentially. No concurrent provider access.
3. **Existing sidecar protocol**: The new `chat` command follows the existing NDJSON protocol. No protocol changes.
4. **PCM format**: Audio is 16kHz, 16-bit signed, mono. WAV conversion uses existing `convert_to_wav` logic.
5. **Rust module structure**: New agent code goes in `src-tauri/src/agents/`. Queue code goes in `src-tauri/src/intelligence/`. New Tauri commands registered in `lib.rs`.
6. **No recording changes**: The recording and transcription pipeline SHALL NOT be modified. The chatbot is additive only.
7. **Single CSS file**: All new CSS goes in `App.css` using existing design tokens. No new CSS files.
8. **Settings backward compatibility**: No new settings required for V1. The chatbot uses the already-loaded model.
9. **Memory**: The chatbot uses the same loaded LLM instance as other agents — no additional model loading.
10. **Transcript file convention**: `{recording_stem}_transcript.md` stored alongside the PCM file in the recordings directory.

## Out of Scope

1. Chat with gems — future `GemChatSource` implementing `Chatable`
2. Chat during live recording — future `LiveRecordingChatSource` implementing `Chatable`
3. Chat settings (system prompt customization, context window size, max history) — hardcoded for V1
4. Chat session browsing/management UI — sessions exist as `.md` files but no in-app session list
5. Streaming responses — responses are returned complete, not token-by-token
6. Voice input for chat — text only for V1
7. Citations linking answers to specific transcript sections
8. Chat history persistence across app restarts — sessions are in-memory only (`.md` log is the persistent record)
9. Multiple concurrent chat sessions with the same recording
