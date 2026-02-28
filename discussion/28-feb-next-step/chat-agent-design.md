# Chat Agent Design — Shared Live Context + Independent Agents

**Date:** 2026-02-28
**Status:** Design Proposal
**Depends on:** IntelProvider trait, Co-Pilot Agent, Gems system

---

## Problem

Co-Pilot is **passive intelligence** — it watches, listens, and surfaces insights on a timer. The user cannot steer it, ask follow-ups, or interrogate the content. There is no way to have a conversation _with_ the knowledge Jarvis has captured.

**The gap:** Users hear something interesting in a call and want to immediately ask "summarize the call so far for me" or open a gem from last week and ask "what were the action items from the pricing discussion?" Today, they can't.

---

## Vision

### Key Principle: Agents Own Their Own Context

Every agent builds and maintains its **own context** for what it needs. There is no god-object that does analysis for everyone.

During a live recording, there is a **Live Call Context Manager** — but it's intentionally dumb. It just accumulates raw materials (audio chunks + transcripts) as the recording progresses. Any agent can ask for these raw materials and do its own analysis.

```
┌──────────────────────────────────────────────────┐
│        LIVE CALL CONTEXT MANAGER                   │
│                                                  │
│  Accumulates (no analysis, no LLM calls):         │
│  ├── Audio chunks (60s segments of PCM → WAV)     │
│  └── Transcript text (from live transcription)    │
│                                                  │
│  Any agent calls: get_context() → audio + text    │
└──────────────────────┬───────────────────────────┘
                       │ raw materials
          ┌────────────┼────────────┐
          │            │            │
          ▼            ▼            ▼
   ┌─────────────┐ ┌─────────┐ ┌──────────────┐
   │ CO-PILOT    │ │  CHAT   │ │ SENTIMENT    │
   │             │ │  AGENT  │ │ AGENT        │
   │ Picks:      │ │ Picks:  │ │ Picks:       │
   │  audio      │ │  text   │ │  audio       │
   │             │ │         │ │              │
   │ Own work:   │ │ Own work│ │ Own work:    │
   │ copilot_    │ │ chat()  │ │ sentiment_   │
   │ analyze()   │ │ with    │ │ analyze()    │
   │ → key pts   │ │ user Q  │ │ → mood,tone  │
   │ → decisions │ │ → answer│ │              │
   │ → cards     │ │ → log   │ │              │
   └─────────────┘ └─────────┘ └──────────────┘
```

### What is Shared vs What is Agent-Owned

| | Live Call Context Manager (shared) | Agent-Owned |
|---|---|---|
| **What** | Raw audio chunks + accumulated transcript text | Analysis, history, state, prompts |
| **Nature** | Dumb accumulator — no LLM, no analysis | Smart — each agent does its own analysis |
| **Who writes** | Only the Context Manager (appends audio + transcript) | Each agent writes its own state |
| **Who reads** | Any agent during a live recording | Only the owning agent |
| **Lifetime** | Starts with recording, stops with recording | Agent decides |
| **Examples** | `audio_path`, `transcript`, `last_chunk_end_secs` | Chat history, Co-Pilot cards, sentiment scores, session logs |

### Per-Context Breakdown

| Context | Shared Raw Materials | Agent-Owned Context (Chat) |
|---|---|---|
| **Live Recording** | Audio chunks + transcript from Context Manager | Chat history, session log |
| **Recording Playback** | None — Chat loads its own data | Transcript, copilot data from gem, chat history, session log |
| **Gem** | None — Chat loads its own data | Full gem content, chat history, session log |

---

## Part 1: Live Call Context Manager

### What It Is

A simple **accumulator** that runs during any live recording. It collects two things:

1. **Complete audio chunks** — the raw PCM audio sliced into 60-second segments
2. **Transcripts** — the accumulated text from live transcription

It does **no analysis**. It just holds the raw materials. When an agent asks for context, it hands over whatever has been accumulated so far. The agent decides what to do with it.

### The Chunk Timeline

```
Time        Audio Chunks                      Transcripts
──────────────────────────────────────────────────────────────────
0–59s       (recording, no complete chunk)     "Hi everyone, let's get started..."
60s         Chunk 1 complete [0:00–1:00]       transcript for 0:00–1:00
120s        Chunk 1+2 [0:00–2:00]             transcript for 0:00–2:00
180s        Chunk 1+2+3 [0:00–3:00]           transcript for 0:00–3:00
...         ...                                ...
```

At any point, the Context Manager knows:
- **All complete chunks so far** — file paths to the audio WAVs (or one growing PCM)
- **Full transcript so far** — all accumulated transcript text from live transcription
- **Last chunk boundary** — timestamp of the latest complete chunk

### What Agents Get

When any agent calls `get_live_call_context()`, it receives **both**:

```rust
pub struct LiveCallContext {
    /// Path to complete audio recorded so far (all chunks joined)
    pub audio_path: PathBuf,
    pub audio_duration_secs: u64,
    pub last_chunk_end_secs: u64,

    /// Full transcript accumulated so far from live transcription
    pub transcript: String,
    pub transcript_char_count: usize,
}
```

**The agent picks what it needs:**

| Agent | Uses from LiveCallContext | What it does with it |
|---|---|---|
| **Chat Agent** | `transcript` | Sends transcript + user question to LLM via `provider.chat()` |
| **Co-Pilot Agent** | `audio_path` (latest chunk) | Sends audio to LLM via `provider.copilot_analyze()` for structured extraction |
| **Sentiment Agent** (future) | `audio_path` (latest chunk) | Sends audio to LLM with sentiment analysis prompt |
| **Action Item Agent** (future) | `transcript` | Scans transcript for action items with a focused prompt |

### Concrete Example

```
Recording at t=120s. User types: "Summarize what's been discussed so far"

1. Chat Agent calls: context_manager.get_live_call_context()
   ↓
2. Context Manager returns:
   LiveCallContext {
     audio_path: "/tmp/jarvis_recording_chunks/full.wav",
     audio_duration_secs: 120,
     last_chunk_end_secs: 120,
     transcript: "Hi everyone, let's get started. Today we're discussing
                  the Q3 pricing for the enterprise tier. Mike, can you
                  walk us through the current numbers?  Sure, so the
                  enterprise tier is priced at fifty thousand per year...
                  [~2 minutes of transcript text]",
     transcript_char_count: 4832,
   }
   ↓
3. Chat Agent picks: transcript (it's a text LLM, doesn't need audio)
   Builds prompt: system_prompt + transcript + chat_history + "Summarize what's been discussed"
   Calls: provider.chat(messages)
   ↓
4. LLM responds with summary. Chat logs to session .md. Returns to user.
```

```
Meanwhile, Co-Pilot Agent runs on its own cycle at t=120s:

1. Co-Pilot calls: context_manager.get_live_call_context()
   ↓
2. Gets same LiveCallContext
   ↓
3. Co-Pilot picks: audio_path (it's a multimodal LLM, processes raw audio)
   Extracts the latest chunk (60–120s) from the audio
   Calls: provider.copilot_analyze(chunk_wav, previous_summary)
   ↓
4. Gets back structured data: key_points, decisions, action_items, etc.
   Updates its own CoPilotState. Emits cards to UI.
```

```
Future: Sentiment Agent runs at t=120s:

1. Sentiment Agent calls: context_manager.get_live_call_context()
   ↓
2. Gets same LiveCallContext
   ↓
3. Sentiment Agent picks: audio_path (latest chunk for tone analysis)
   Calls: provider.analyze_sentiment(chunk_wav) — or wraps in a chat prompt
   ↓
4. Gets back: { sentiment: "tense", confidence: 0.82, reason: "pricing pushback" }
   Updates its own SentimentState. Shows indicator in UI.
```

### The Context Manager is Dumb (On Purpose)

It doesn't know about Co-Pilot, Chat, Sentiment, or any agent. It just:

1. Accumulates audio chunks as they complete every 60s
2. Accumulates transcript text as segments arrive from live transcription
3. Hands it all over when asked

No analysis. No prompts. No LLM calls. No opinion about what's important. That's each agent's job.

### Implementation

```rust
/// src-tauri/src/agents/live_call_context.rs

pub struct LiveCallContextManager {
    app_handle: AppHandle,

    /// Growing audio file path
    recording_path: PathBuf,

    /// Accumulated transcript from live transcription
    transcript: Arc<TokioMutex<String>>,

    /// Chunk tracking
    chunk_interval_secs: u64,       // default 60
    recording_start: Instant,
    last_chunk_end_secs: u64,
}

impl LiveCallContextManager {
    /// Called when recording starts
    pub fn start(&mut self, recording_path: PathBuf) { ... }

    /// Called when recording stops
    pub fn stop(&mut self) { ... }

    /// Called by the transcription system when a new segment arrives
    pub fn on_transcript_segment(&self, text: &str) {
        // Append to accumulated transcript
    }

    /// Any agent calls this to get the current state
    pub fn get_context(&self) -> Option<LiveCallContext> {
        // Return current audio path + full transcript + chunk info
    }

    /// Get just the latest chunk (for agents that only need recent audio)
    pub fn get_latest_chunk(&self) -> Option<AudioChunk> {
        // Extract the most recent complete chunk from the recording
    }
}

pub struct AudioChunk {
    pub path: PathBuf,          // WAV file for this chunk
    pub start_secs: u64,
    pub end_secs: u64,
    pub duration_secs: u64,
}
```

---

## Part 2: Chat Agent

The Chat Agent is an **independent agent** that manages its own context for each session. It reads the shared `LiveContext` when on a live recording, but for recordings and gems it does all its own context assembly.

### What Chat Owns

| Responsibility | Details |
|---|---|
| **Conversation history** | Per-session message list (user + assistant turns) |
| **Session `.md` log file** | Persistent record of every exchange |
| **Context assembly for recordings** | Checks transcript, triggers transcription if missing, loads copilot data from gem |
| **Context assembly for gems** | Loads full gem from DB: content, enrichment, transcript, source_meta |
| **LLM prompt construction** | Builds system prompt from whatever context it assembled |
| **`provider.chat()` invocation** | Sends messages to the LLM, gets response |

### What Chat Does NOT Own

| | Owned By |
|---|---|
| Live recording analysis loop | `LiveContextEngine` (shared) |
| Co-Pilot card state, timers, collapse | Co-Pilot UI component |
| Transcript generation | Transcription system (Chat just triggers it when needed) |
| Gem storage | Gems DB (Chat just reads from it) |

### How Chat Assembles Context — Per Scenario

#### Live Recording

```
chat_send_message("live_recording", "live", "summarize the call")
        │
        ▼
    Ask Context Manager: get_context()
    → Gets back: LiveCallContext { audio_path, transcript, ... }
    Chat picks: transcript (text-only LLM, doesn't need audio)
    Build system prompt with transcript + chat history + user question
    Call provider.chat(messages)
    Log exchange to session .md
    Return response
```

**Shared raw material consumed:** transcript text from Context Manager
**Chat's own context:** chat history + session log
**Time:** ~5ms prep + LLM inference

#### Recording Playback

```
chat_send_message("recording", "20260228_143022.pcm", "what were the action items?")
        │
        ▼
    Does Chat have a cached transcript for this recording?
    ├── YES → use it
    └── NO  → Does a gem exist with transcript?
              ├── YES → load from gem
              └── NO  → Does standalone transcript exist?
                        ├── YES → load it
                        └── NO  → Trigger transcription
                                  Show "Transcribing..." progress
                                  Wait for completion
                                  Cache result
    Also load gem.source_meta.copilot if available
    Build system prompt with transcript + copilot data
    Append chat history
    Call provider.chat(messages)
    Log exchange
    Return response
```

**Chat's own context:** transcript (loaded/prepared by Chat), copilot data from gem, chat history, session log
**Shared context consumed:** None (recording is not live)
**Time:** instant if transcript cached, 30-120s if transcription needed

#### Gem

```
chat_send_message("gem", "a1b2c3d4", "explain this concept")
        │
        ▼
    Does Chat have this gem cached?
    ├── YES → use it
    └── NO  → Load full gem from DB: get_gem(id)
              Cache it for this session
    Is it an audio gem without transcript?
    ├── YES → Trigger transcription, wait, re-load
    └── NO  → continue
    Build system prompt from:
    ├── gem.title, source_type, domain, author
    ├── gem.content (article body, email, etc.)
    ├── gem.ai_enrichment.summary + tags
    ├── gem.transcript (if audio gem)
    ├── gem.source_meta.copilot (if recording gem)
    └── gem.source_meta.youtube/email/etc.
    Append chat history
    Call provider.chat(messages)
    Log exchange
    Return response
```

**Chat's own context:** full gem data (loaded by Chat), chat history, session log
**Shared context consumed:** None
**Time:** ~50ms if gem loaded, longer if transcription needed

### ChatAgent Struct

```rust
/// src-tauri/src/agents/chat.rs

pub struct ChatAgent {
    app_handle: AppHandle,
    sessions: HashMap<String, ChatSession>,
}

pub struct ChatSession {
    pub id: String,
    pub context_type: String,           // "live_recording" | "recording" | "gem"
    pub context_id: String,             // "live", filename, or gem ID
    pub context_title: String,
    pub messages: Vec<ChatMessage>,
    pub log_path: Option<PathBuf>,
    pub created_at: String,
    pub last_active_at: String,

    // Agent-owned cached context (NOT shared)
    pub cached_transcript: Option<String>,    // for recordings
    pub cached_gem: Option<Gem>,              // for gems
    pub cached_copilot_data: Option<serde_json::Value>, // from gem source_meta
}
```

### Send Message

```rust
impl ChatAgent {
    async fn send_message(
        &mut self,
        context_type: &str,
        context_id: &str,
        message: &str,
        context_manager: Option<&LiveCallContextManager>,  // only during live recording
        provider: &dyn IntelProvider,
    ) -> Result<ChatResponse, String> {
        let session = self.get_or_create_session(context_type, context_id).await;

        // 1. BUILD OWN CONTEXT (each path is independent)
        let prep_start = Instant::now();
        let assembled_context = match context_type {
            "live_recording" => {
                // Get raw materials from Context Manager, pick transcript
                let raw = context_manager
                    .ok_or("No active recording")?
                    .get_context()
                    .ok_or("Recording not started")?;
                self.assemble_live_context(&raw).await
            }
            "recording" => {
                // Chat's own work: load/prepare transcript
                self.assemble_recording_context(session, context_id).await
            }
            "gem" => {
                // Chat's own work: load gem from DB
                self.assemble_gem_context(session, context_id).await
            }
            _ => return Err("Unknown context type".into()),
        }?;
        let prep_time = prep_start.elapsed();

        // 2. USER MESSAGE
        session.messages.push(ChatMessage::user(message));

        // 3. BUILD LLM PROMPT
        let system_prompt = self.build_system_prompt(&assembled_context);
        let llm_messages = self.build_llm_messages(&system_prompt, session);

        // 4. LLM INFERENCE
        let infer_start = Instant::now();
        let response = provider.chat(&llm_messages).await?;
        let infer_time = infer_start.elapsed();

        // 5. STORE RESPONSE
        session.messages.push(ChatMessage::assistant(&response.content));

        // 6. LOG
        self.log_exchange(session, message, &response, &assembled_context, prep_time, infer_time).await;

        Ok(response)
    }
}
```

### IntelProvider Extension

```rust
/// Added to the existing IntelProvider trait
async fn chat(
    &self,
    messages: &[ChatMessage],
) -> Result<ChatResponse, String> {
    Err("Chat not supported by this provider".to_string())
}
```

Note: the `chat()` method takes **only messages**. The system prompt (with context baked in) is already in the messages array. The LLM provider doesn't know or care about the context structure — it just sees a conversation.

### Tauri Commands

```rust
#[tauri::command]
async fn chat_send_message(
    context_type: String,
    context_id: String,
    message: String,
) -> Result<ChatResponse, String>;

#[tauri::command]
async fn chat_get_history(
    context_type: String,
    context_id: String,
) -> Result<Vec<ChatMessage>, String>;

#[tauri::command]
async fn chat_clear_history(
    context_type: String,
    context_id: String,
) -> Result<(), String>;
```

### Tauri Events

```rust
// Chat preparation progress (e.g., transcribing a recording)
emit("chat-preparation", {
    context_id: "20260228_143022.pcm",
    state: "transcribing",
    progress: 0.45,
});

// Chat status
emit("chat-status", {
    context_id: "live",
    status: "thinking",  // "idle" | "preparing" | "thinking" | "error"
});
```

---

## Part 3: How Agents Interact During a Live Recording

During a live recording, multiple agents run simultaneously. They all get raw materials from the same Context Manager but each does its own work independently.

```
┌──────────────────────────────────────────────────────────────┐
│                    LIVE RECORDING SESSION                      │
│                                                              │
│  ┌──────────────────────────────────────────────────┐        │
│  │  Live Call Context Manager (dumb accumulator)      │        │
│  │                                                  │        │
│  │  Audio:     [chunk1][chunk2][chunk3]...            │        │
│  │  Transcript: "Hi everyone... pricing... budget..." │        │
│  └──────────────────────┬───────────────────────────┘        │
│                         │                                     │
│              get_context() — returns audio + transcript       │
│         ┌───────────────┼──────────────────┐                  │
│         ▼               ▼                  ▼                  │
│  ┌─────────────┐  ┌──────────┐  ┌────────────────┐          │
│  │ CO-PILOT    │  │ CHAT     │  │ SENTIMENT      │          │
│  │             │  │ AGENT    │  │ AGENT (future) │          │
│  │ Takes:      │  │ Takes:   │  │ Takes:         │          │
│  │  audio      │  │  text    │  │  audio         │          │
│  │             │  │          │  │                │          │
│  │ Runs:       │  │ Runs:    │  │ Runs:          │          │
│  │ copilot_    │  │ chat()   │  │ sentiment_     │          │
│  │ analyze()   │  │ with     │  │ analyze()      │          │
│  │ every 60s   │  │ user Q   │  │ every 60s      │          │
│  │             │  │ on-demand│  │                │          │
│  │ Produces:   │  │ Produces:│  │ Produces:      │          │
│  │ key_points  │  │ answer   │  │ sentiment      │          │
│  │ decisions   │  │ + log.md │  │ score + reason │          │
│  │ summary     │  │          │  │                │          │
│  │ cards       │  │          │  │                │          │
│  └─────────────┘  └──────────┘  └────────────────┘          │
│                                                              │
│  Each agent does its own analysis, owns its own state.        │
│  Context Manager just supplies raw materials.                 │
└──────────────────────────────────────────────────────────────┘
```

### Queueing

The MLX sidecar handles requests sequentially (stdin/stdout). During a live recording:

- Co-Pilot calls `copilot_analyze()` every ~60s (~10-15s inference)
- Chat calls `provider.chat()` on user demand (~3-8s inference)
- Future agents make their own calls on their own schedules
- Requests queue naturally — if a chat message arrives during a Co-Pilot cycle, it waits
- The user sees "Thinking..." during the wait — acceptable latency

### After Recording Stops

- Context Manager freezes — final audio + transcript preserved for the session
- Co-Pilot keeps its cards and analysis state (tab persists)
- Chat keeps its conversation and can still answer questions from the frozen transcript
- When saved as gem, Co-Pilot's analysis goes into `gem.source_meta.copilot`

---

## Part 4: Session Log Files

Every chat session produces a **persistent `.md` file**.

### Location & Naming

```
~/Library/Application Support/com.jarvis.app/chat_logs/
├── 20260228_143022_live_chat.md
├── 20260228_150500_rec_20260225_091500.md
├── 20260228_160000_gem_a1b2c3d4.md
└── ...
```

### File Structure

```markdown
# Chat Session — Live Recording
**Started:** 2026-02-28 14:30:22
**Context:** Live Recording (20260228_143022.pcm)
**Model:** Qwen3-8B-4bit

## Context Snapshot
**Live Context (cycle 5):**
The call is a sales discussion between Acme Corp and BigCo...

**Key Points:**
- Enterprise tier pricing at $50K/yr
- Client budget concerns

**Decisions:**
- Agreed to explore 90-day pilot

---

## Exchange 1 — 14:32:15

**User:** Summarize the call so far

**Context used:** Live context (cycle 5) + transcript tail (8,432 chars)
**Preparation time:** 12ms
**Inference time:** 4.2s

**Jarvis:**
The call is a sales negotiation between Acme Corp and BigCo about
enterprise pricing for Q3...

---

## Exchange 2 — 14:35:42

**User:** What exactly did Mike say about the budget?

**Context used:** Live context (cycle 6) + transcript tail (10,201 chars)
**Preparation time:** 8ms
**Inference time:** 3.8s

**Jarvis:**
From the transcript at approximately 8:30, Mike said:
> "Fifty thousand a year is significantly above what we budgeted..."

---

## Session Closed — 14:45:00
**Total exchanges:** 6
**Total inference time:** 24.3s
```

### What Gets Logged Per Exchange

```rust
struct ChatExchangeLog {
    exchange_number: u32,
    timestamp: String,
    user_message: String,
    context_sources: Vec<String>,       // what context was used
    context_preparation_time_ms: u64,
    inference_time_secs: f64,
    assistant_response: String,
    token_usage: Option<TokenUsage>,
}
```

---

## Part 5: Frontend — Chat UI

### ChatPanel Component

```
┌─────────────────────────────────────────┐
│  Transcript │ Co-Pilot │  Chat          │
├─────────────────────────────────────────┤
│ ┌─ Live Recording ─────────────────── ┐ │
│ │ Live context cycle 5 · 12 min       │ │
│ └─────────────────────────────────────┘ │
│                                         │
│  ┌─────────────────────────────────┐    │
│  │  Jarvis                         │    │
│  │ Ask me anything about this      │    │
│  │ call. I can see the live         │    │
│  │ analysis and transcript.        │    │
│  └─────────────────────────────────┘    │
│                                         │
│  ┌─────────────────────────────────┐    │
│  │  You                            │    │
│  │ Summarize the call so far       │    │
│  └─────────────────────────────────┘    │
│                                         │
│  ┌─────────────────────────────────┐    │
│  │  Preparing context...            │    │
│  └─────────────────────────────────┘    │
│                                         │
│  ┌─────────────────────────────────┐    │
│  │  Thinking...                     │    │
│  └─────────────────────────────────┘    │
│                                         │
├─────────────────────────────────────────┤
│ ┌─────────────────────────────┐ ┌─────┐ │
│ │ Ask about this recording... │ │  >  │ │
│ └─────────────────────────────┘ └─────┘ │
└─────────────────────────────────────────┘
```

### Where Chat Appears

| Navigation | Right Panel Tabs | Chat Context |
|---|---|---|
| Record (recording active) | Transcript / Co-Pilot / **Chat** | Reads shared `LiveContext` + transcript tail |
| Record (stopped, data exists) | Transcript / Co-Pilot / **Chat** | Reads last `LiveContext` snapshot |
| Recordings (selected) | Details / **Chat** | Chat loads transcript itself |
| Gems (selected) | Details / **Chat** | Chat loads gem itself |
| YouTube / Browser / Settings | _(no chat)_ | — |

### Props

```typescript
interface ChatPanelProps {
  contextType: 'live_recording' | 'recording' | 'gem';
  contextId: string;
  contextTitle: string;
  isAvailable: boolean;
  copilotCycle?: number;
}
```

### UX Details

- **Two-phase loading:** "Preparing context..." → "Thinking..."
- **Auto-scroll** to bottom on new messages
- **Enter to send**, Shift+Enter for newline
- **Context badge** at top showing what context is active
- **Markdown rendering** for responses
- **Clear chat** button — resets session, starts new log file
- **Empty state** tailored per context type:
  - Live: _"Ask me anything about this call."_
  - Recording: _"Ask me about this recording. I'll prepare the transcript if needed."_
  - Gem: _"Ask me anything about this content."_

### Preparation State Display

```typescript
function PreparationIndicator({ state }) {
  switch (state.type) {
    case 'preparing':
      return <Spinner>Preparing context... {state.detail}</Spinner>;
    case 'transcribing':
      return <Spinner>Transcribing recording... <ProgressBar value={state.progress} /></Spinner>;
    case 'ready':
      return <FadeOut>Context ready: {state.summary}</FadeOut>;
  }
}
```

---

## Part 6: Co-Pilot ↔ Chat Bridge

### Suggested Questions Become Actionable

Co-Pilot's `suggested_questions` get an **"Ask this"** button:

```
┌───────────────────────────────────────────┐
│  Suggested Question                        │
│                                           │
│ "What metrics will be used to evaluate    │
│  the pilot program's success?"            │
│                                           │
│ Reason: No success criteria discussed.    │
│                                           │
│              [Ask this ->] [Dismiss]      │
└───────────────────────────────────────────┘
```

Clicking "Ask this" → switches to Chat tab → pre-fills the question.

Co-Pilot generated the question from its own audio analysis. Chat answers it using the transcript from the Context Manager. Different raw materials, same call, complementary results.

---

## Part 7: Context Window Strategy

### Live Recording
```
[System prompt + instructions          ~200 tokens  ]
[Transcript from Context Manager       ~3500 tokens  ]  ← last 14K chars or full if shorter
[Chat history (last 10 pairs)          ~1000 tokens  ]
[User's new message                     ~100 tokens  ]
───────────────────────────────────────────────────
                                       ~4800 tokens
```

Note: Chat works directly from the transcript. For long calls (>15 min), it uses the transcript tail. If Co-Pilot is running alongside, Chat could optionally read Co-Pilot's summary for compression — but that's an optimization, not a requirement. The transcript is the primary context.

### Recording Playback
- Full transcript if < 15K tokens
- If longer: copilot summary (from gem) + first 2K chars + last 10K chars

### Gem Content
- Priority: `enrichment.summary` → `tags` → `transcript` → `content` (truncated last)
- Most gems well within limits

### Chat History Truncation
- Last **10 message pairs** sent to LLM
- Older messages kept in UI, dropped from LLM context
- Session log retains everything

---

## Part 8: MLX Sidecar — Python Side

New handler in `server.py`:

```python
def handle_chat(request):
    messages = request["messages"]
    conversation = [{"role": m["role"], "content": m["content"]} for m in messages]

    response = generate(
        model=loaded_model,
        tokenizer=loaded_tokenizer,
        conversation=conversation,
        max_tokens=1024,
        temperature=0.7,
    )

    return {
        "status": "ok",
        "content": response.text,
        "usage": {
            "prompt_tokens": response.prompt_tokens,
            "completion_tokens": response.completion_tokens,
        }
    }
```

Python side is thin. Context is already baked into the system message by Rust.

---

## Implementation Sequence

### Phase 1: Live Call Context Manager
1. Create `LiveCallContextManager` — accumulates audio chunks + transcript text
2. Wire it into recording lifecycle: start → accumulate → stop
3. Feed transcript segments from live transcription into Context Manager
4. Expose `get_context()` and `get_latest_chunk()` for any agent
5. Refactor Co-Pilot to get its audio chunks from Context Manager instead of reading PCM directly

### Phase 2: Chat Agent Backend
5. Add `chat()` to `IntelProvider` trait
6. Implement `chat()` in `MlxProvider` — new NDJSON command
7. Add `handle_chat()` to sidecar `server.py`
8. Create `ChatAgent` with per-session context assembly:
   - Live: reads shared `LiveContext` + transcript tail
   - Recording: loads/prepares transcript itself
   - Gem: loads full gem from DB itself
9. Implement session `.md` logging
10. Register Tauri commands

### Phase 3: Frontend
11. Create `ChatPanel.tsx`
12. Add Chat tab to all three contexts (live, recording detail, gem detail)
13. Wire up events: `chat-preparation`, `chat-status`
14. Connect "Ask this" on suggested questions → Chat tab

### Phase 4: Polish
15. Markdown rendering
16. Error handling, retry, empty states
17. Chat history truncation
18. Session file cleanup

---

## Key Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Context ownership | **Each agent owns its own analysis** | No god-object. Context Manager just accumulates raw materials. |
| Shared state | **Audio chunks + transcript (raw, no analysis)** | Dumb accumulator. Each agent picks what it needs and does its own work. |
| Chat context for live | **Picks transcript from Context Manager** | Text LLM doesn't need audio. Co-Pilot picks audio instead. |
| Chat context for recordings | **Chat loads/prepares transcript itself** | Agent is self-sufficient. |
| Chat context for gems | **Chat loads gem from DB itself** | Same — agent owns what it needs. |
| Session persistence | **`.md` log file** + in-memory | Debuggable, exportable, no schema changes. |
| Model for chat | **Same loaded LLM** | Text-only, any model works. |

---

## Future Extensions (Out of Scope for v1)

| Extension | Description |
|---|---|
| **Session resume** | Parse `.md` to restore chat after restart |
| **Cross-gem chat** | Chat loads multiple gems into its own context |
| **Streaming responses** | Token streaming via Tauri events |
| **Voice input** | Speak questions, reuse transcription pipeline |
| **Citations** | Clickable references to source text |
| **Chat in gem export** | Attach session log to gem |

---

## Success Criteria

- Context Manager accumulates audio + transcript during live recording — no analysis, no LLM calls
- Chat picks transcript from Context Manager for live recording (~5ms)
- Co-Pilot picks audio chunks from Context Manager for analysis (~5ms)
- A future Sentiment Agent picks audio from the same Context Manager — zero new infrastructure
- Chat loads its own context for recordings and gems — no shared state needed
- Chat auto-transcribes recordings when transcript is missing
- Each chat session produces a readable `.md` log
- Each agent owns its own analysis, state, and context — no coupling between agents
- No disruption to existing functionality
