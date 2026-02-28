# Design Document: Recording Chatbot — Trait-Based Reusable Chat Component

## Overview

The Recording Chatbot is a reusable conversational system built on a `Chatable` trait. Any content source that implements `Chatable` gets a chatbot attached to it — the chatbot asks the source for context, sends it to the LLM, and logs every exchange to a markdown file.

The first conformer is `RecordingChatSource`, which makes recordings chatbot-compatible. It loads or generates a transcript and returns it as context. Future conformers (gems, live recordings) require zero chatbot changes.

The system introduces an `IntelQueue` that serializes all LLM requests from all agents (chatbot, co-pilot, enrichment) through a single mpsc channel with oneshot response routing, replacing direct provider calls.

### Key Design Goals

1. **Reusable**: The chatbot works with any `Chatable` — it knows nothing about recordings, gems, or any content type
2. **Simple**: No analysis engine, no state aggregation — just context + question → answer + log
3. **Non-disruptive**: Additive only — no changes to recording, transcription, or co-pilot pipelines
4. **Queued access**: All agents share one `IntelQueue` — no mutex contention on the provider
5. **Persistent logs**: Every chat session produces a debuggable, exportable `.md` file

## Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                      Jarvis App (Tauri)                          │
├─────────────────────────────────────────────────────────────────┤
│  Frontend (React)                                                │
│  ├─ Recording Detail Panel (existing, extended)                  │
│  │  └─ Chat Button (NEW)                                         │
│  └─ Right Panel (existing, extended)                             │
│     ├─ Recording Detail View (existing)                          │
│     └─ Chat Tab (NEW)                                            │
│        └─ ChatPanel Component (NEW, reusable)                    │
│           ├─ Message bubbles (user + assistant)                  │
│           ├─ Text input bar                                      │
│           └─ Status indicator (preparing/thinking)               │
├─────────────────────────────────────────────────────────────────┤
│  Backend (Rust)                                                  │
│  ├─ Commands (extended)                                          │
│  │  ├─ chat_with_recording(filename)                             │
│  │  ├─ chat_send_message(session_id, filename, message)          │
│  │  ├─ chat_get_history(session_id)                              │
│  │  └─ chat_end_session(session_id)                              │
│  ├─ Agents Module (extended)                                     │
│  │  ├─ chatable.rs (NEW) — Chatable trait definition             │
│  │  ├─ chatbot.rs (NEW) — Chatbot engine (trait-driven)          │
│  │  ├─ recording_chat.rs (NEW) — RecordingChatSource             │
│  │  └─ copilot.rs (existing, unchanged)                          │
│  ├─ Intelligence Module (extended)                               │
│  │  ├─ queue.rs (NEW) — IntelQueue                               │
│  │  ├─ provider.rs (extended) — + chat() method                  │
│  │  └─ mlx_provider.rs (extended) — implements chat()            │
│  └─ Recording Module (existing, unchanged)                       │
├─────────────────────────────────────────────────────────────────┤
│  Python MLX Sidecar (server.py, extended)                        │
│  ├─ chat command (NEW)                                           │
│  │  ├─ Accept messages array [{role, content}]                   │
│  │  └─ Return {response: "..."}                                  │
│  └─ Existing commands (generate-tags, summarize, etc.)           │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
User clicks [Chat] on recording "20260228_143022.pcm"
    │
    ▼
Frontend: invoke("chat_with_recording", { recordingFilename })
    │
    ▼
Tauri command creates RecordingChatSource { filename }
    │  ↑ implements Chatable
    ▼
chatbot.start_session(&source, &intel_queue)
    │
    ├── source.needs_preparation() → true (no transcript)
    │     └── source.get_context(intel_queue)
    │           ├── Convert PCM → WAV
    │           ├── intel_queue.submit(GenerateTranscript { wav_path })
    │           │     └── → mpsc → worker → provider.generate_transcript() → oneshot → back
    │           ├── Save transcript to disk as {stem}_transcript.md
    │           └── Return transcript text
    │
    ├── Create session log: {recordings_dir}/chat_session_{timestamp}.md
    └── Return session_id to frontend
    │
    ▼
User types: "What were the action items?"
    │
    ▼
Frontend: invoke("chat_send_message", { sessionId, recordingFilename, message })
    │
    ▼
chatbot.send_message(session_id, message, &source, &intel_queue)
    │
    ├── source.get_context(intel_queue) → reads transcript from disk (fast)
    ├── Build LLM messages: [system + context, ...history, user message]
    ├── intel_queue.submit(Chat { messages })
    │     └── → mpsc → worker → provider.chat(messages) → oneshot → back
    ├── Append to session .md log
    └── Return assistant response to frontend
```

### Concurrency Model — IntelQueue

Multiple agents may need the IntelProvider simultaneously. The `IntelQueue` serializes all requests through a single mpsc channel. Each caller gets its response back via a dedicated oneshot channel.

```
┌──────────┐  ┌──────────┐  ┌───────────┐
│ CO-PILOT │  │ CHATBOT  │  │ ENRICHMENT│
└────┬─────┘  └────┬─────┘  └────┬──────┘
     │             │              │
     │  submit(cmd) + oneshot_tx  │
     ▼             ▼              ▼
┌──────────────────────────────────────┐
│         INTEL REQUEST QUEUE          │
│                                      │
│  mpsc::channel<IntelRequest>(32)     │
│                                      │
│  Worker loop (single tokio task):    │
│    recv() → match command →          │
│    call provider method →            │
│    reply_tx.send(result)             │
└──────────────────────────────────────┘
```

**Why a queue instead of a mutex on the provider?**
- The Co-Pilot agent, chatbot, and gem enrichment all call the provider. With a mutex, callers block opaquely. With a queue, requests are ordered, each gets a routed response, and the worker has a single point of control for timeout/retry.
- The existing `MlxProvider` communicates over stdin/stdout with the sidecar — inherently sequential. A queue matches this reality.

## Components and Interfaces

### 1. Chatable Trait

**Location**: `jarvis-app/src-tauri/src/agents/chatable.rs`

**Responsibilities**:
- Define the contract for any content source that can have a chatbot attached
- Provide context text for answering questions
- Specify session storage location
- Signal whether context preparation is needed
- Emit preparation progress events

**Definition**:

```rust
use async_trait::async_trait;
use std::path::PathBuf;
use crate::intelligence::queue::IntelQueue;

#[async_trait]
pub trait Chatable: Send + Sync {
    /// The text context the chatbot will answer questions from.
    /// Called on every message — must be fast for static sources (disk read),
    /// and fresh for growing sources (live transcript).
    async fn get_context(&self, intel_queue: &IntelQueue) -> Result<String, String>;

    /// Human-readable label for session log headers.
    /// e.g. "Recording 20260228_143022" or "Gem: Pricing Meeting"
    fn label(&self) -> String;

    /// Directory where chat session .md files are stored.
    fn session_dir(&self) -> PathBuf;

    /// Whether context preparation is needed (e.g. transcript generation).
    /// If true, chatbot shows "Preparing..." before first message.
    async fn needs_preparation(&self) -> bool;

    /// Optional: called during preparation to show progress to the user.
    /// Default: no-op.
    fn on_preparation_status(&self, _status: &str, _message: &str) {}
}
```

**Design decisions**:
- `get_context()` takes `&IntelQueue` so the source can submit generation requests (transcript, etc.) without holding a provider reference
- `get_context()` is called on every message, not cached by the chatbot — the source decides whether to do a cheap read or an expensive generation
- `on_preparation_status()` has a default no-op — sources that don't need preparation don't need to implement it
- The trait is `Send + Sync` to work across async task boundaries

### 2. RecordingChatSource — Recording Conforms to Chatable

**Location**: `jarvis-app/src-tauri/src/agents/recording_chat.rs`

**Responsibilities**:
- Make recordings chatbot-compatible
- Load transcript from disk if available
- Generate transcript via IntelQueue if missing
- Persist generated transcript for reuse
- Emit status events during transcript generation

**Data Structure**:

```rust
use tauri::AppHandle;
use std::path::PathBuf;

pub struct RecordingChatSource {
    app_handle: AppHandle,
    filename: String,          // "20260228_143022.pcm"
    recordings_dir: PathBuf,   // ~/Library/Application Support/com.jarvis.app/recordings/
}

impl RecordingChatSource {
    pub fn new(app_handle: AppHandle, filename: String) -> Result<Self, String> {
        let recordings_dir = get_recordings_dir(&app_handle)?;
        Ok(Self { app_handle, filename, recordings_dir })
    }

    fn stem(&self) -> String {
        self.filename.trim_end_matches(".pcm").to_string()
    }

    fn transcript_path(&self) -> PathBuf {
        self.recordings_dir.join(format!("{}_transcript.md", self.stem()))
    }
}
```

**Chatable Implementation**:

```rust
#[async_trait]
impl Chatable for RecordingChatSource {
    async fn get_context(&self, intel_queue: &IntelQueue) -> Result<String, String> {
        let transcript_path = self.transcript_path();

        // Fast path: transcript exists on disk
        if transcript_path.exists() {
            return tokio::fs::read_to_string(&transcript_path).await
                .map_err(|e| format!("Failed to read transcript: {}", e));
        }

        // Slow path: generate transcript
        self.on_preparation_status("preparing", "Generating transcript...");

        let audio_path = self.recordings_dir.join(&self.filename);
        let wav_path = convert_pcm_to_wav(&audio_path)?;

        let response = intel_queue.submit(IntelCommand::GenerateTranscript {
            audio_path: wav_path,
        }).await?;

        let transcript = match response {
            IntelResponse::Transcript(result) => result.transcript,
            _ => return Err("Unexpected response type".into()),
        };

        // Persist for reuse across sessions
        let transcript_md = format!(
            "# Transcript — {}\n\n**Generated:** {}\n\n---\n\n{}",
            self.stem(),
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            transcript,
        );
        tokio::fs::write(&transcript_path, &transcript_md).await.ok();

        self.on_preparation_status("ready", "Ready to chat");
        Ok(transcript)
    }

    fn label(&self) -> String {
        format!("Recording {}", self.stem())
    }

    fn session_dir(&self) -> PathBuf {
        self.recordings_dir.clone()
    }

    async fn needs_preparation(&self) -> bool {
        !self.transcript_path().exists()
    }

    fn on_preparation_status(&self, status: &str, message: &str) {
        self.app_handle.emit("chat-status", serde_json::json!({
            "status": status,
            "message": message,
        })).ok();
    }
}
```

**Transcript File Format** (stored at `{recordings_dir}/{stem}_transcript.md`):

```markdown
# Transcript — 20260228_143022

**Generated:** 2026-02-28 14:35:10

---

Hi everyone, let's get started. Today we're discussing the Q3 pricing
for the enterprise tier. Mike, can you walk us through the current numbers?
...
```

**Relationship to existing transcription**:
- The existing `transcribe_recording` Tauri command generates transcripts via `provider.generate_transcript()` and returns the text to the frontend
- `RecordingChatSource` uses the same provider method (via `IntelQueue`) but additionally persists the result to disk as `{stem}_transcript.md`
- If the user has already transcribed a recording through the existing UI, the transcript text is stored in the frontend state but NOT on disk. `RecordingChatSource.needs_preparation()` will return `true` and re-generate. This is acceptable for V1 — a future optimization could check the frontend state first

### 3. Chatbot — Reusable Chat Engine

**Location**: `jarvis-app/src-tauri/src/agents/chatbot.rs`

**Responsibilities**:
- Manage multiple concurrent chat sessions
- Build LLM prompts from context + history + user message
- Submit chat requests through IntelQueue
- Maintain in-memory message history per session
- Write persistent `.md` session logs

**Data Structures**:

```rust
pub struct Chatbot {
    sessions: HashMap<String, ChatSession>,
}

pub struct ChatSession {
    pub session_id: String,
    pub messages: Vec<ChatMessage>,
    pub log_path: PathBuf,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,       // "user" | "assistant"
    pub content: String,
    pub timestamp: String,  // "HH:MM:SS"
}
```

**Methods**:

```rust
impl Chatbot {
    pub fn new() -> Self {
        Self { sessions: HashMap::new() }
    }

    /// Start a new chat session against any Chatable source.
    /// Triggers context preparation if needed (e.g. transcript generation).
    /// Returns session_id.
    pub async fn start_session(
        &mut self,
        source: &dyn Chatable,
        intel_queue: &IntelQueue,
    ) -> Result<String, String>;

    /// Send a message. Fetches fresh context from source on every call.
    /// Returns assistant response.
    pub async fn send_message(
        &mut self,
        session_id: &str,
        user_message: &str,
        source: &dyn Chatable,
        intel_queue: &IntelQueue,
    ) -> Result<String, String>;

    /// Get in-memory message history for a session.
    pub fn get_history(&self, session_id: &str) -> Result<Vec<ChatMessage>, String>;

    /// Remove session from memory.
    pub fn end_session(&mut self, session_id: &str);
}
```

**Prompt Assembly** (inside `send_message`):

```rust
// 1. Get fresh context from source
let context = source.get_context(intel_queue).await?;

// 2. Build system message with context (truncated to last 14K chars)
let system_msg = format!(
    "You are a helpful assistant. Answer questions based on the \
     following context. Be concise and accurate. If the answer \
     isn't in the context, say so.\n\n\
     --- CONTEXT ---\n{}",
    truncate_context(&context, 14_000)
);

// 3. Assemble messages: system + history (last 10 exchanges) + user message
let mut llm_messages: Vec<(String, String)> = vec![
    ("system".into(), system_msg),
];

let history_start = session.messages.len().saturating_sub(20);
for msg in &session.messages[history_start..] {
    llm_messages.push((msg.role.clone(), msg.content.clone()));
}

llm_messages.push(("user".into(), user_message.to_string()));

// 4. Submit to queue
let response = intel_queue.submit(IntelCommand::Chat {
    messages: llm_messages,
}).await?;
```

**Context Truncation**:

```rust
fn truncate_context(text: &str, max_chars: usize) -> &str {
    if text.len() <= max_chars {
        text
    } else {
        // Take the tail — most recent content is most relevant
        &text[text.len() - max_chars..]
    }
}
```

The 14,000 character limit (~3,500 tokens) leaves room for the system prompt (~200 tokens), chat history (~1,000 tokens for 10 exchanges), and the user's message (~100 tokens) within a typical 8K context window.

**Session Log Writing** (inside `send_message`, after getting response):

```rust
let log_entry = format!(
    "## User ({})\n{}\n\n## Assistant ({})\n{}\n\n---\n\n",
    now, user_message, response_time, assistant_text,
);

tokio::fs::OpenOptions::new()
    .append(true)
    .open(&session.log_path)
    .await?
    .write_all(log_entry.as_bytes())
    .await?;
```

### 4. IntelQueue — Request Serialization

**Location**: `jarvis-app/src-tauri/src/intelligence/queue.rs`

**Responsibilities**:
- Accept requests from any agent via `submit()`
- Route each request to the appropriate IntelProvider method
- Return each response to the correct caller via oneshot channel
- Process requests sequentially (one at a time)

**Data Structures**:

```rust
use tokio::sync::{mpsc, oneshot};

pub struct IntelRequest {
    pub command: IntelCommand,
    pub reply_tx: oneshot::Sender<Result<IntelResponse, String>>,
}

pub enum IntelCommand {
    Chat { messages: Vec<(String, String)> },
    GenerateTranscript { audio_path: PathBuf },
    CopilotAnalyze { audio_path: PathBuf, context: String },
    GenerateTags { content: String },
    Summarize { content: String },
}

pub enum IntelResponse {
    Chat(String),
    Transcript(TranscriptResult),
    CopilotAnalysis(CoPilotCycleResult),
    Tags(Vec<String>),
    Summary(String),
}

pub struct IntelQueue {
    tx: mpsc::Sender<IntelRequest>,
}
```

**Implementation**:

```rust
impl IntelQueue {
    pub fn new(provider: Arc<dyn IntelProvider>) -> Self {
        let (tx, mut rx) = mpsc::channel::<IntelRequest>(32);

        tokio::spawn(async move {
            while let Some(req) = rx.recv().await {
                let result = match req.command {
                    IntelCommand::Chat { messages } =>
                        provider.chat(&messages).await.map(IntelResponse::Chat),
                    IntelCommand::GenerateTranscript { audio_path } =>
                        provider.generate_transcript(&audio_path).await
                            .map(IntelResponse::Transcript),
                    IntelCommand::CopilotAnalyze { audio_path, context } =>
                        provider.copilot_analyze(&audio_path, &context).await
                            .map(IntelResponse::CopilotAnalysis),
                    IntelCommand::GenerateTags { content } =>
                        provider.generate_tags(&content).await
                            .map(IntelResponse::Tags),
                    IntelCommand::Summarize { content } =>
                        provider.summarize(&content).await
                            .map(IntelResponse::Summary),
                };
                let _ = req.reply_tx.send(result);
            }
        });

        IntelQueue { tx }
    }

    pub async fn submit(&self, command: IntelCommand) -> Result<IntelResponse, String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx.send(IntelRequest { command, reply_tx }).await
            .map_err(|_| "Queue closed".to_string())?;
        reply_rx.await
            .map_err(|_| "Worker dropped".to_string())?
    }
}
```

**Queue capacity**: 32 buffered slots. In practice, there are at most 3 agents (co-pilot, chatbot, enrichment), so the buffer is never close to full. If the queue is full, `submit()` awaits until a slot opens.

### 5. IntelProvider Trait Extension — chat() Method

**Location**: `jarvis-app/src-tauri/src/intelligence/provider.rs`

The `IntelProvider` trait gets a new `chat()` method alongside the existing `generate_tags`, `summarize`, `generate_transcript`, and `copilot_analyze` methods.

**Trait Extension**:

```rust
#[async_trait]
pub trait IntelProvider: Send + Sync {
    // ... existing methods ...

    /// Send a multi-turn conversation to the LLM and receive a text response.
    /// Each tuple is (role, content) where role is "system", "user", or "assistant".
    /// Default: returns error for providers that don't support chat.
    async fn chat(
        &self,
        _messages: &[(String, String)],
    ) -> Result<String, String> {
        Err("Chat not supported by this provider".to_string())
    }
}
```

### 6. MlxProvider — chat() Implementation

**Location**: `jarvis-app/src-tauri/src/intelligence/mlx_provider.rs`

```rust
async fn chat(&self, messages: &[(String, String)]) -> Result<String, String> {
    let command = serde_json::json!({
        "command": "chat",
        "messages": messages.iter().map(|(role, content)| {
            serde_json::json!({"role": role, "content": content})
        }).collect::<Vec<_>>(),
        "model_path": self.get_model_path()?,
    });

    let response = self.send_command(command, Duration::from_secs(120)).await?;

    response.get("response")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or("No response field in chat result".to_string())
}
```

This follows the exact same pattern as `generate_tags` and `summarize` — construct an NDJSON command, send to sidecar, parse the response.

### 7. MLX Sidecar — chat Command

**Location**: `jarvis-app/src-tauri/sidecars/mlx-server/server.py`

**New Command**: `chat`

**Request Format**:
```json
{
  "command": "chat",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant..."},
    {"role": "user", "content": "What were the action items?"},
    {"role": "assistant", "content": "Three action items were..."},
    {"role": "user", "content": "Who was assigned item 2?"}
  ],
  "model_path": "/path/to/model"
}
```

**Response Format**:
```json
{
  "type": "response",
  "command": "chat",
  "response": "Item 2 was assigned to Sarah — she needs to pull churn data for the enterprise tier."
}
```

**Error Response**:
```json
{
  "type": "error",
  "command": "chat",
  "error": "Error message"
}
```

**Python Implementation**:

```python
def handle_chat(self, data):
    """Handle multi-turn chat conversation."""
    if self.model is None:
        return {"type": "error", "command": "chat", "error": "No model loaded"}

    messages = data.get("messages", [])
    if not messages:
        return {"type": "error", "command": "chat", "error": "No messages provided"}

    try:
        # Build conversation for the model
        conversation = [{"role": m["role"], "content": m["content"]} for m in messages]

        # Apply chat template and generate
        token_ids = self.tokenizer.apply_chat_template(
            conversation, add_generation_prompt=True
        )
        response_text = mlx_lm_generate(
            self.model, self.tokenizer,
            prompt=token_ids,
            max_tokens=2048,
            verbose=False,
        )

        return {
            "type": "response",
            "command": "chat",
            "response": response_text.strip(),
        }
    except Exception as e:
        return {"type": "error", "command": "chat", "error": str(e)}
```

**Note**: The chat handler uses `mlx_lm_generate` (text-only generation), not `mlx_omni_generate` (multimodal). Chat is text-in, text-out — the context is already text (transcript). This means chat works with any loaded model, not just Qwen Omni.

### 8. Tauri Commands

**Location**: `jarvis-app/src-tauri/src/commands.rs`

Four new commands, all following the existing command patterns in the file.

```rust
#[tauri::command]
pub async fn chat_with_recording(
    recording_filename: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let source = RecordingChatSource::new(
        state.app_handle.clone(),
        recording_filename,
    )?;

    let mut chatbot = state.chatbot.lock().await;
    let queue = &*state.intel_queue;
    chatbot.start_session(&source, queue).await
}

#[tauri::command]
pub async fn chat_send_message(
    session_id: String,
    recording_filename: String,
    message: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let source = RecordingChatSource::new(
        state.app_handle.clone(),
        recording_filename,
    )?;

    let mut chatbot = state.chatbot.lock().await;
    let queue = &*state.intel_queue;
    chatbot.send_message(&session_id, &message, &source, queue).await
}

#[tauri::command]
pub async fn chat_get_history(
    session_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ChatMessage>, String> {
    let chatbot = state.chatbot.lock().await;
    chatbot.get_history(&session_id)
}

#[tauri::command]
pub async fn chat_end_session(
    session_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut chatbot = state.chatbot.lock().await;
    chatbot.end_session(&session_id);
    Ok(())
}
```

**Key design choice**: `RecordingChatSource` is constructed on each command invocation, not stored in app state. It's cheap to create (just filename + directory lookup). The chatbot and intel_queue are stateful and live in app state; the source is transient.

**App State Additions** (in `lib.rs`):

```rust
// Add to existing setup:
let intel_queue = IntelQueue::new(provider.clone());
app.manage(intel_queue);
app.manage(TokioMutex::new(Chatbot::new()));
```

**Command Registration** (in `lib.rs`, add to `generate_handler!`):

```rust
chat_with_recording,
chat_send_message,
chat_get_history,
chat_end_session,
```

### 9. Session Log Files

**Created by**: Chatbot (not the source)
**Location**: Determined by `source.session_dir()`

**For Recordings**, session logs live alongside the audio:

```
~/Library/Application Support/com.jarvis.app/recordings/
  ├── 20260228_143022.pcm                     ← raw audio
  ├── 20260228_143022_transcript.md           ← transcript (RecordingChatSource creates)
  ├── chat_session_1709150000.md              ← session log (Chatbot creates)
  └── chat_session_1709153600.md              ← another session
```

**Session Log Format**:

```markdown
# Chat Session

**Label:** Recording 20260228_143022
**Started:** 2026-02-28 14:35:22

---

## User (14:35:22)
Summarize this recording

## Assistant (14:35:28)
The recording is a 12-minute team meeting about Q3 enterprise pricing.
Key points: current pricing at $50K/year, proposed 15% increase,
and Sarah's concern about churn risk.

---

## User (14:36:01)
What were the action items?

## Assistant (14:36:05)
Three action items:
1. Mike — competitive pricing comparison by Friday
2. Sarah — pull churn data for enterprise tier (last 6 months)
3. Team reconvene next Tuesday with data

---
```

## Data Models

### Frontend TypeScript Types

```typescript
// types.ts or inline in ChatPanel

interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
  timestamp?: string;
}

interface ChatPanelProps {
  sessionId: string;
  recordingFilename: string;
  status: 'preparing' | 'ready' | 'error';
  preparingMessage?: string;
  placeholder?: string;
}
```

### App State Additions (Rust)

```rust
// In lib.rs setup or a shared state module

pub struct AppState {
    // ... existing fields ...
    pub chatbot: TokioMutex<Chatbot>,
    pub intel_queue: IntelQueue,
}
```

## Correctness Properties

### Property 1: Chatable Trait Completeness
A `Chatable` implementation provides everything the chatbot needs. The chatbot never imports or references a concrete source type.

### Property 2: Context Freshness
`get_context()` is called on every `send_message`, not cached by the chatbot. For recordings, this is a cheap disk read. For future live recordings, this returns the latest accumulated transcript.

### Property 3: Transcript Persistence
Once `RecordingChatSource` generates a transcript, it is saved to `{stem}_transcript.md`. Subsequent sessions reuse the saved file — no redundant generation.

### Property 4: Queue Request-Response Pairing
Every `IntelRequest` carries a `oneshot::Sender`. The worker sends exactly one response per request. The caller's `submit()` returns that response — no cross-talk between agents.

### Property 5: Session Isolation
Each `ChatSession` has its own message history, log file, and session ID. Multiple sessions (for different recordings) coexist in the chatbot's `sessions` HashMap without interference.

### Property 6: Log Append-Only
Session logs are written with `OpenOptions::append(true)`. No rewriting. If the process crashes mid-write, previous entries are preserved.

### Property 7: Source Statelessness
`RecordingChatSource` is constructed per command invocation. It holds no mutable state. All persistent state lives in the chatbot (sessions) and on disk (transcript, log files).

### Property 8: IntelCommand ↔ IntelResponse Type Safety
Each `IntelCommand` variant maps to exactly one `IntelResponse` variant. The chatbot pattern-matches on the response and returns an error if the variant is unexpected.

### Property 9: Queue Capacity
The mpsc channel has a buffer of 32. With at most 3 concurrent agents, the buffer is never exhausted under normal operation. If full, `submit()` awaits (backpressure, not drop).

### Property 10: Graceful Session Cleanup
`end_session()` removes the session from memory. The `.md` log file remains on disk. No orphaned file handles.

## Error Handling

### Context Preparation Errors

| Error | Handling |
|---|---|
| PCM file missing | `get_context()` returns `Err("Failed to read transcript: ...")` → chatbot propagates to frontend |
| WAV conversion fails | Same — error propagated to frontend |
| Transcript generation fails (LLM error) | `IntelQueue` returns the provider error → chatbot propagates |
| Transcript file write fails | `tokio::fs::write().ok()` — transcript still returned in-memory, just not persisted |

### Chat Errors

| Error | Handling |
|---|---|
| Session not found | `send_message` returns `Err("Session not found")` |
| Queue closed | `submit()` returns `Err("Queue closed")` — provider crashed |
| LLM timeout (120s) | Provider returns timeout error → chatbot propagates |
| Log file write fails | Error propagated — user sees it but chat still returns the response |

### Frontend Error Display

All errors are displayed as an assistant message prefixed with "Error:" so the user can see what went wrong inline in the chat.

## Frontend Components

### 1. ChatPanel Component

**Location**: `jarvis-app/src/components/ChatPanel.tsx`

**Reusable** — doesn't know what it's chatting about. Takes a session ID and communicates via Tauri commands.

```tsx
interface ChatPanelProps {
  sessionId: string;
  recordingFilename: string;
  status: 'preparing' | 'ready' | 'error';
  preparingMessage?: string;
  placeholder?: string;
}

function ChatPanel({
  sessionId, recordingFilename, status, preparingMessage, placeholder
}: ChatPanelProps) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [thinking, setThinking] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll on new messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Listen for status changes from backend
  useEffect(() => {
    const unlisten = listen('chat-status', (event) => {
      // Update status based on preparation progress
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  const sendMessage = async () => {
    if (!input.trim() || thinking) return;
    const userMsg = input.trim();
    setInput('');
    setMessages(prev => [...prev, { role: 'user', content: userMsg }]);
    setThinking(true);

    try {
      const response = await invoke<string>('chat_send_message', {
        sessionId,
        recordingFilename,
        message: userMsg,
      });
      setMessages(prev => [...prev, { role: 'assistant', content: response }]);
    } catch (err) {
      setMessages(prev => [...prev, {
        role: 'assistant',
        content: `Error: ${err}`,
      }]);
    } finally {
      setThinking(false);
    }
  };

  // Preparing state
  if (status === 'preparing') {
    return (
      <div className="chat-preparing">
        <div className="spinner" />
        <p>{preparingMessage || 'Preparing...'}</p>
      </div>
    );
  }

  // Chat interface
  return (
    <div className="chat-panel">
      <div className="chat-messages">
        {messages.length === 0 && (
          <div className="chat-empty">
            {placeholder || 'Ask me anything about this recording.'}
          </div>
        )}
        {messages.map((msg, i) => (
          <div key={i} className={`chat-message chat-${msg.role}`}>
            <div className="chat-bubble">{msg.content}</div>
          </div>
        ))}
        {thinking && (
          <div className="chat-message chat-assistant">
            <div className="chat-bubble thinking">Thinking...</div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>
      <div className="chat-input-bar">
        <input
          type="text"
          value={input}
          onChange={e => setInput(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && sendMessage()}
          placeholder={placeholder || 'Ask a question...'}
          disabled={thinking}
        />
        <button onClick={sendMessage} disabled={thinking || !input.trim()}>
          Send
        </button>
      </div>
    </div>
  );
}
```

**CSS** (added to `App.css`):

```css
/* Chat Panel */
.chat-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.chat-message {
  display: flex;
}

.chat-message.chat-user {
  justify-content: flex-end;
}

.chat-message.chat-assistant {
  justify-content: flex-start;
}

.chat-bubble {
  max-width: 80%;
  padding: 8px 12px;
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  line-height: 1.5;
  white-space: pre-wrap;
}

.chat-user .chat-bubble {
  background: var(--accent-primary);
  color: white;
}

.chat-assistant .chat-bubble {
  background: var(--bg-elevated);
  color: var(--text-primary);
}

.chat-bubble.thinking {
  opacity: 0.6;
  animation: pulse 1.5s ease-in-out infinite;
}

.chat-input-bar {
  display: flex;
  gap: 8px;
  padding: 12px;
  border-top: 1px solid var(--border-primary);
}

.chat-input-bar input {
  flex: 1;
  padding: 8px 12px;
  border-radius: var(--radius-md);
  border: 1px solid var(--border-primary);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: var(--text-sm);
}

.chat-input-bar button {
  padding: 8px 16px;
  border-radius: var(--radius-md);
  background: var(--accent-primary);
  color: white;
  border: none;
  font-size: var(--text-sm);
  cursor: pointer;
}

.chat-input-bar button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.chat-empty {
  text-align: center;
  color: var(--text-secondary);
  font-size: var(--text-sm);
  padding: 40px 20px;
}

.chat-preparing {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 12px;
  color: var(--text-secondary);
  font-size: var(--text-sm);
}
```

### 2. Recording Detail Panel — Chat Button Integration

**Location**: `jarvis-app/src/components/RecordingDetailPanel.tsx`

Add a "Chat" button alongside the existing "Transcribe" and "Save as Gem" buttons.

```tsx
// Inside RecordingDetailPanel, alongside existing buttons:

<button
  className="action-btn chat-btn"
  onClick={handleStartChat}
  disabled={!aiAvailable}
  title={!aiAvailable ? 'Load a model to enable chat' : 'Chat with this recording'}
>
  💬 Chat
</button>
```

The `handleStartChat` function:

```tsx
const handleStartChat = async () => {
  try {
    setChatStatus('preparing');
    const sessionId = await invoke<string>('chat_with_recording', {
      recordingFilename: recording.filename,
    });
    setChatSessionId(sessionId);
    setChatStatus('ready');
    setShowChatTab(true);  // Switch right panel to show chat
  } catch (err) {
    setChatStatus('error');
    console.error('Failed to start chat:', err);
  }
};
```

### 3. Right Panel — Chat Tab Integration

**Location**: `jarvis-app/src/components/RightPanel.tsx`

When the user starts a chat session from a recording, the right panel switches to show the ChatPanel. This is added to the existing `recordings` nav case.

```tsx
// Inside RightPanel, when activeNav === 'recordings' and a chat session is active:

{chatSessionId ? (
  <div className="right-panel-content">
    <div className="panel-tabs">
      <button
        className={`tab-btn ${activeTab === 'detail' ? 'active' : ''}`}
        onClick={() => setActiveTab('detail')}
      >
        Details
      </button>
      <button
        className={`tab-btn ${activeTab === 'chat' ? 'active' : ''}`}
        onClick={() => setActiveTab('chat')}
      >
        Chat
      </button>
    </div>
    {activeTab === 'detail' && <RecordingDetailPanel ... />}
    {activeTab === 'chat' && (
      <ChatPanel
        sessionId={chatSessionId}
        recordingFilename={selectedRecording.filename}
        status={chatStatus}
        preparingMessage="Generating transcript..."
      />
    )}
  </div>
) : (
  <RecordingDetailPanel ... />
)}
```

## File Summary

```
New files:
jarvis-app/src-tauri/src/agents/chatable.rs         ← Chatable trait
jarvis-app/src-tauri/src/agents/chatbot.rs           ← Chatbot (reusable)
jarvis-app/src-tauri/src/agents/recording_chat.rs    ← RecordingChatSource
jarvis-app/src-tauri/src/intelligence/queue.rs       ← IntelQueue
jarvis-app/src/components/ChatPanel.tsx              ← Reusable chat UI

Modified files:
jarvis-app/src-tauri/src/intelligence/provider.rs    ← Add chat()
jarvis-app/src-tauri/src/intelligence/mlx_provider.rs ← Implement chat()
jarvis-app/src-tauri/src/intelligence/mod.rs         ← Export queue
jarvis-app/src-tauri/src/agents/mod.rs               ← Export modules
jarvis-app/src-tauri/src/commands.rs                 ← 4 new commands
jarvis-app/src-tauri/src/lib.rs                      ← Register commands + state
jarvis-app/src/components/RightPanel.tsx             ← Chat tab
jarvis-app/src/components/RecordingDetailPanel.tsx   ← Chat button
jarvis-app/src/App.css                               ← Chat styles
jarvis-app/src-tauri/sidecars/mlx-server/server.py   ← handle_chat()
```

## Technical Constraints

1. **Provider-agnostic**: Chatbot → IntelQueue → IntelProvider::chat(). Never references MlxProvider directly.
2. **Single IntelQueue**: One queue, one worker, sequential processing. Matches the sidecar's stdin/stdout reality.
3. **Existing NDJSON protocol**: The `chat` command follows the same format as `generate-tags`, `summarize`, etc.
4. **PCM format**: 16kHz, 16-bit signed, mono. WAV conversion reuses existing logic.
5. **Module structure**: Agent code in `src-tauri/src/agents/`, queue in `src-tauri/src/intelligence/`.
6. **Single CSS file**: All styles in `App.css` using existing design tokens.
7. **No recording changes**: Recording and transcription pipelines are untouched.
8. **No new settings**: V1 uses hardcoded values (14K context, 10 history pairs, 120s timeout).
9. **Same model**: Chat uses whatever model is currently loaded — no additional model loading.

## Out of Scope

1. Chat with gems — future `GemChatSource` implementing `Chatable`
2. Chat during live recording — future `LiveRecordingChatSource` implementing `Chatable`
3. Streaming token-by-token responses
4. Voice input for chat questions
5. Citations linking answers to specific transcript sections
6. Custom system prompt / settings UI for chat
7. Session persistence across app restarts
8. Multiple concurrent sessions for the same recording
9. Chat session browsing/management UI
