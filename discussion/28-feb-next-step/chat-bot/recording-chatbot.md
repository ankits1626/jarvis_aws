# Chatbot — Trait-Based Reusable Chat Component

**Date:** 2026-02-28

## What

A reusable chatbot that works with anything implementing the `Chatable` trait. The chatbot asks the `Chatable` for what it needs — context, label, session directory — and the `Chatable` provides it.

First implementation: `Recording` conforms to `Chatable`.

---

## The Chatable Trait

```rust
/// src-tauri/src/agents/chatable.rs
///
/// Anything that implements this trait can have a chatbot attached to it.

#[async_trait]
pub trait Chatable: Send + Sync {
    /// The text context the chatbot will answer questions from.
    /// For a recording, this is the transcript.
    /// For a gem, this might be title + tags + content.
    /// For a live call, this is the accumulated transcript so far.
    async fn get_context(&self, intel_queue: &IntelQueue) -> Result<String, String>;

    /// Human-readable label for the session log header.
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

---

## Recording Implements Chatable

```rust
/// src-tauri/src/agents/recording_chat.rs

pub struct RecordingChatSource {
    app_handle: AppHandle,
    filename: String,          // "20260228_143022.pcm"
    recordings_dir: PathBuf,
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

#[async_trait]
impl Chatable for RecordingChatSource {
    async fn get_context(&self, intel_queue: &IntelQueue) -> Result<String, String> {
        let transcript_path = self.transcript_path();

        // If transcript exists on disk, return it
        if transcript_path.exists() {
            return tokio::fs::read_to_string(&transcript_path).await
                .map_err(|e| format!("Failed to read transcript: {}", e));
        }

        // Otherwise, generate it
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

        // Persist for reuse
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

---

## Future: Gem Implements Chatable

```rust
pub struct GemChatSource {
    app_handle: AppHandle,
    gem: Gem,
}

#[async_trait]
impl Chatable for GemChatSource {
    async fn get_context(&self, _intel_queue: &IntelQueue) -> Result<String, String> {
        // Gem already has everything — no generation needed
        Ok(format!(
            "Title: {}\nTags: {}\nSummary: {}\n\nContent:\n{}",
            self.gem.title,
            self.gem.tags.join(", "),
            self.gem.enrichment.summary.clone().unwrap_or_default(),
            self.gem.content,
        ))
    }

    fn label(&self) -> String { format!("Gem: {}", self.gem.title) }
    fn session_dir(&self) -> PathBuf { get_app_dir(&self.app_handle).unwrap().join("gem_chats") }
    async fn needs_preparation(&self) -> bool { false }  // gems are always ready
}
```

---

## Future: Live Recording Implements Chatable

```rust
pub struct LiveRecordingChatSource {
    app_handle: AppHandle,
    context_manager: Arc<LiveCallContextManager>,
}

#[async_trait]
impl Chatable for LiveRecordingChatSource {
    async fn get_context(&self, _intel_queue: &IntelQueue) -> Result<String, String> {
        // Grab latest transcript from the live context manager
        let ctx = self.context_manager.get_context()
            .ok_or("No active recording")?;
        Ok(ctx.transcript)
    }

    fn label(&self) -> String { "Live Recording".to_string() }
    fn session_dir(&self) -> PathBuf { get_app_dir(&self.app_handle).unwrap().join("live_chats") }
    async fn needs_preparation(&self) -> bool { false }  // transcript is already accumulating
}
```

Note: For live recordings, the chatbot calls `source.get_context()` on **every message** (not just session start) because the transcript keeps growing. See `send_message` below.

---

## Chatbot

The chatbot doesn't know about recordings, gems, or anything. It just talks to a `Chatable`.

```rust
/// src-tauri/src/agents/chatbot.rs

pub struct Chatbot {
    sessions: HashMap<String, ChatSession>,
}

pub struct ChatSession {
    pub session_id: String,
    pub messages: Vec<ChatMessage>,
    pub log_path: PathBuf,
    pub created_at: String,
}

pub struct ChatMessage {
    pub role: String,       // "user" | "assistant"
    pub content: String,
    pub timestamp: String,
}

impl Chatbot {
    /// Start a new chat session against any Chatable source
    pub async fn start_session(
        &mut self,
        source: &dyn Chatable,
        intel_queue: &IntelQueue,
    ) -> Result<String, String> {
        // Let the source prepare if needed (e.g. transcript generation)
        let _context = source.get_context(intel_queue).await?;

        let timestamp = chrono::Utc::now().timestamp();
        let session_id = format!("chat_{}", timestamp);
        let log_path = source.session_dir().join(format!("chat_session_{}.md", timestamp));

        // Write session header
        let header = format!(
            "# Chat Session\n\n\
             **Label:** {}\n\
             **Started:** {}\n\n\
             ---\n\n",
            source.label(),
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        );
        tokio::fs::create_dir_all(source.session_dir()).await.ok();
        tokio::fs::write(&log_path, &header).await
            .map_err(|e| format!("Failed to write session log: {}", e))?;

        let session = ChatSession {
            session_id: session_id.clone(),
            messages: vec![],
            log_path,
            created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        };

        self.sessions.insert(session_id.clone(), session);
        Ok(session_id)
    }

    /// Send a message — chatbot asks the source for fresh context each time
    pub async fn send_message(
        &mut self,
        session_id: &str,
        user_message: &str,
        source: &dyn Chatable,
        intel_queue: &IntelQueue,
    ) -> Result<String, String> {
        // Always get fresh context from source
        // (for recordings: same transcript each time — cheap read from disk)
        // (for live calls: growing transcript — always fresh)
        let context = source.get_context(intel_queue).await?;

        let session = self.sessions.get_mut(session_id)
            .ok_or("Session not found")?;

        let now = chrono::Local::now().format("%H:%M:%S").to_string();

        // Build LLM messages
        let mut llm_messages: Vec<(String, String)> = vec![
            ("system".into(), format!(
                "You are a helpful assistant. Answer questions based on the \
                 following context. Be concise and accurate. If the answer \
                 isn't in the context, say so.\n\n\
                 --- CONTEXT ---\n{}",
                truncate_context(&context, 14_000)
            )),
        ];

        // Add chat history (last 10 exchanges = 20 messages)
        let history_start = session.messages.len().saturating_sub(20);
        for msg in &session.messages[history_start..] {
            llm_messages.push((msg.role.clone(), msg.content.clone()));
        }

        // Add user message
        llm_messages.push(("user".into(), user_message.to_string()));

        // Submit to queue
        let response = intel_queue.submit(IntelCommand::Chat {
            messages: llm_messages,
        }).await?;

        let assistant_text = match response {
            IntelResponse::Chat(text) => text,
            _ => return Err("Unexpected response type".into()),
        };

        let response_time = chrono::Local::now().format("%H:%M:%S").to_string();

        // Record in memory
        session.messages.push(ChatMessage {
            role: "user".into(),
            content: user_message.to_string(),
            timestamp: now.clone(),
        });
        session.messages.push(ChatMessage {
            role: "assistant".into(),
            content: assistant_text.clone(),
            timestamp: response_time.clone(),
        });

        // Append to session .md log
        let log_entry = format!(
            "## User ({})\n{}\n\n## Assistant ({})\n{}\n\n---\n\n",
            now, user_message, response_time, assistant_text,
        );
        use tokio::io::AsyncWriteExt;
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(&session.log_path)
            .await
            .map_err(|e| format!("Failed to open log: {}", e))?
            .write_all(log_entry.as_bytes())
            .await
            .map_err(|e| format!("Failed to write log: {}", e))?;

        Ok(assistant_text)
    }

    pub fn get_history(&self, session_id: &str) -> Result<Vec<ChatMessage>, String> {
        let session = self.sessions.get(session_id)
            .ok_or("Session not found")?;
        Ok(session.messages.clone())
    }

    pub fn end_session(&mut self, session_id: &str) {
        self.sessions.remove(session_id);
    }
}

fn truncate_context(text: &str, max_chars: usize) -> &str {
    if text.len() <= max_chars {
        text
    } else {
        &text[text.len() - max_chars..]
    }
}
```

---

## How It All Fits Together

```
User taps [Chat] on recording "20260228_143022.pcm"
  │
  ▼
Tauri command: chat_with_recording("20260228_143022.pcm")
  │
  ▼
Create RecordingChatSource { filename: "20260228_143022.pcm" }
  │  ↑ implements Chatable
  ▼
chatbot.start_session(&source, &intel_queue)
  │
  ├── chatbot calls source.get_context()
  │     └── RecordingChatSource checks for transcript on disk
  │           ├── Found → returns transcript text
  │           └── Not found → generates via intel_queue → saves to disk → returns
  │
  ├── chatbot creates session .md log file
  └── returns session_id
  │
  ▼
User types: "What were the action items?"
  │
  ▼
Tauri command: chat_send_message(session_id, "What were the action items?")
  │
  ▼
chatbot.send_message(session_id, message, &source, &intel_queue)
  │
  ├── chatbot calls source.get_context() again (fresh context)
  ├── builds prompt: system + context + history + user message
  ├── submits to intel_queue → waits turn → gets response
  ├── appends to session .md log
  └── returns assistant response to UI
```

---

## Session Log Files

**Created by:** Chatbot
**Location:** Provided by the `Chatable` via `session_dir()`

### For Recordings

```
~/Library/Application Support/com.jarvis.app/recordings/
  ├── 20260228_143022.pcm                     ← raw audio
  ├── 20260228_143022_transcript.md           ← transcript (RecordingChatSource creates)
  ├── chat_session_1709150000.md              ← session log (Chatbot creates)
  └── chat_session_1709153600.md              ← another session
```

### Log Format

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

---

## IntelProvider Queue

Multiple agents may call IntelProvider simultaneously. The queue serializes requests and routes each response back to its caller.

```
┌──────────┐  ┌──────────┐  ┌──────────┐
│ CO-PILOT │  │ CHATBOT  │  │ ENRICHMENT│
└────┬─────┘  └────┬─────┘  └────┬─────┘
     │             │              │
     │  submit(cmd, reply_tx)     │
     ▼             ▼              ▼
┌──────────────────────────────────────┐
│         INTEL REQUEST QUEUE          │
│                                      │
│  mpsc channel → single worker loop   │
│  Worker: pop → call provider → reply │
└──────────────────────────────────────┘
```

```rust
/// src-tauri/src/intelligence/queue.rs

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

impl IntelQueue {
    pub fn new(provider: Arc<dyn IntelProvider>) -> Self {
        let (tx, mut rx) = mpsc::channel::<IntelRequest>(32);

        tokio::spawn(async move {
            while let Some(req) = rx.recv().await {
                let result = match req.command {
                    IntelCommand::Chat { messages } =>
                        provider.chat(&messages).await.map(IntelResponse::Chat),
                    IntelCommand::GenerateTranscript { audio_path } =>
                        provider.generate_transcript(&audio_path).await.map(IntelResponse::Transcript),
                    IntelCommand::CopilotAnalyze { audio_path, context } =>
                        provider.copilot_analyze(&audio_path, &context).await.map(IntelResponse::CopilotAnalysis),
                    IntelCommand::GenerateTags { content } =>
                        provider.generate_tags(&content).await.map(IntelResponse::Tags),
                    IntelCommand::Summarize { content } =>
                        provider.summarize(&content).await.map(IntelResponse::Summary),
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

---

## Adding `chat()` to IntelProvider

```rust
// provider.rs — add to trait
async fn chat(&self, messages: &[(String, String)]) -> Result<String, String>;
```

```rust
// mlx_provider.rs
async fn chat(&self, messages: &[(String, String)]) -> Result<String, String> {
    let command = NdjsonCommand {
        command: "chat".to_string(),
        messages: Some(messages.iter().map(|(r, c)| json!({"role": r, "content": c})).collect()),
        model_path: Some(self.get_model_path()?),
        ..Default::default()
    };
    let response = self.send_command(command, Duration::from_secs(120)).await?;
    response.get("response")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or("No response in chat result".to_string())
}
```

```python
# server.py — add handler
def handle_chat(data):
    messages = data["messages"]
    model_path = data["model_path"]
    conversation = [{"role": m["role"], "content": m["content"]} for m in messages]
    response_text = generate(model, tokenizer, conversation, max_tokens=2048)
    return {"response": response_text}
```

---

## Tauri Commands

```rust
// The Chatable source lives in app state alongside the chatbot.
// For recordings, we create the source on-demand from the filename.

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
    recording_filename: String,    // needed to recreate the Chatable source
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

---

## Frontend — Reusable ChatPanel

```tsx
// components/ChatPanel.tsx
//
// Reusable. Doesn't know what it's chatting about.
// Takes a sessionId and a Tauri command to send messages.

interface ChatPanelProps {
  sessionId: string;
  recordingFilename: string;      // passed through to send_message command
  status: 'preparing' | 'ready' | 'error';
  preparingMessage?: string;
  placeholder?: string;
}

function ChatPanel({ sessionId, recordingFilename, status, preparingMessage, placeholder }: ChatPanelProps) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [thinking, setThinking] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

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
      setMessages(prev => [...prev, { role: 'assistant', content: `Error: ${err}` }]);
    } finally {
      setThinking(false);
    }
  };

  if (status === 'preparing') {
    return (
      <div className="chat-preparing">
        <div className="spinner" />
        <p>{preparingMessage || 'Preparing...'}</p>
      </div>
    );
  }

  return (
    <div className="chat-panel">
      <div className="chat-messages">
        {messages.length === 0 && (
          <div className="chat-empty">Ask me anything.</div>
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

---

## File Summary

```
New files:

src-tauri/src/agents/chatable.rs         ← Chatable trait definition
src-tauri/src/agents/chatbot.rs          ← Chatbot (trait-driven, reusable)
src-tauri/src/agents/recording_chat.rs   ← RecordingChatSource implements Chatable
src-tauri/src/intelligence/queue.rs      ← IntelQueue
src/components/ChatPanel.tsx             ← Reusable chat UI

Files to modify:

src-tauri/src/intelligence/provider.rs   ← Add chat() to IntelProvider
src-tauri/src/intelligence/mlx_provider.rs ← Implement chat()
src-tauri/src/intelligence/mod.rs        ← Export queue
src-tauri/src/agents/mod.rs              ← Export chatable, chatbot, recording_chat
src-tauri/src/commands.rs                ← chat_with_recording, chat_send_message, etc.
src-tauri/src/lib.rs                     ← Register commands
src/components/RightPanel.tsx            ← Add Chat tab
src/components/RecordingDetailPanel.tsx   ← Add Chat button
sidecars/mlx-server/server.py            ← handle_chat()
```

---

## Implementation Sequence

### Phase 1: IntelProvider Queue
1. Create `IntelQueue` — mpsc + oneshot response routing
2. Wire into app state
3. Migrate existing callers (Co-Pilot, enrichment) to use queue

### Phase 2: Chat Capability
4. Add `chat()` to `IntelProvider` trait
5. Implement in `MlxProvider` + sidecar `handle_chat()`

### Phase 3: Chatable + Chatbot
6. Define `Chatable` trait
7. Create `Chatbot` — works with any `Chatable`
8. Implement `RecordingChatSource` — recording conforms to `Chatable`

### Phase 4: Wire Up
9. Add Tauri commands: `chat_with_recording`, `chat_send_message`, `chat_get_history`, `chat_end_session`
10. Build `ChatPanel` component (reusable)
11. Add Chat button to `RecordingDetailPanel`
12. Add Chat tab to `RightPanel`

### Phase 5: Polish
13. Error handling + input validation
14. Style to match dark theme
15. Session cleanup

---

## Key Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Architecture | **Trait-based (`Chatable`)** | Anything conforming to the trait gets a chatbot. No special-casing. |
| What `Chatable` provides | **Context string, label, session dir** | Minimum the chatbot needs. Source handles all prep internally. |
| What chatbot owns | **Chat history + session .md log + LLM prompt assembly** | Same regardless of source. |
| Context freshness | **`get_context()` called on every message** | Handles live recordings (growing transcript) and static sources equally. |
| First conformer | **`RecordingChatSource`** | Simplest — transcript from disk or generated on demand. |
| Request handling | **Queue with oneshot routing** | Multiple agents, one sidecar. Each caller gets its own response. |
