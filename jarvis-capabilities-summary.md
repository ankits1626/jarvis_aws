# Jarvis — What It Can Do (Feb 28, 2026)

Jarvis is a **local-first desktop knowledge capture and enrichment app** built with Tauri (Rust + React/TypeScript). Everything runs on-device — no cloud APIs required.

---

## Core Workflows

### 1. Record & Transcribe Audio
- Start/stop audio recording (macOS screen recording + microphone)
- **Real-time live transcription** during recording with segment timestamps
- Three transcription engines: **whisper-rs** (default), **WhisperKit** (Apple Silicon optimized), **MLX Omni** (multimodal LLM)
- Voice Activity Detection (VAD) to skip silence
- Language auto-detection (MLX Omni)
- PCM recording with on-demand WAV conversion for playback
- Save transcribed recordings as "gems" with one click

### 2. Extract Web Content from Chrome
- **YouTube**: Auto-detect videos via background observer (polls Chrome every 3s), extract title/channel/duration/description
- **Articles/Medium**: Extract readable content via Readability algorithm, specialized Medium parser
- **Gmail/Email**: Extract subject, body, metadata
- **ChatGPT**: Extract conversations and messages
- **Claude Extension**: Capture Claude side panel conversations via macOS Accessibility API
- **Any webpage**: Generic fallback extractor for title/author/description/content
- **Tab listing**: See all open Chrome tabs with auto-classified source types
- **Page + Claude merging**: Combine extracted page content with Claude conversation into a single gist

### 3. Gems — Knowledge Library
- Save any extracted content or transcription as a **gem** (persistent record)
- **Browse** all gems with pagination
- **Full-text search** across title, description, and content (SQLite FTS5)
- **Filter by tag** — click any AI-generated tag to filter
- **View gem details** — full content, metadata, enrichment info, transcript
- **Upsert by URL** — re-capturing the same URL updates the existing gem
- **Delete gems** with confirmation
- **Export** gists to markdown files

### 4. Co-Pilot Agent (Live Recording Intelligence)
- **Real-time AI analysis** during recording — feeds raw audio chunks directly to Qwen Omni (multimodal LLM) every 60s
- Produces a **rolling summary** of the conversation updated each cycle
- Extracts **key points**, **decisions**, **action items**, and **open questions** with deduplication across cycles
- Generates **suggested questions** (up to 5) contextual to the current discussion
- Identifies **key concepts** with mention counts
- Self-compressing context: each cycle's summary becomes the next cycle's input, so context never grows unbounded
- Configurable cycle interval (30–120s) and audio overlap (0–15s) for sentence boundary bridging
- Full prompt/response **agent logging** to markdown files for debugging and quality tuning
- Co-Pilot data saved into gems alongside transcript and AI enrichment
- Toggle on/off independently from recording — user controls when AI runs
- Provider-agnostic: calls `IntelProvider::copilot_analyze()` trait, not a specific backend
- **Card Stack UX** — each insight rendered as an individual animated card:
  - Cards slide in from top with entrance animation + subtle pulse highlight
  - Auto-collapse after timeout (5s for summaries, 8s for others), hover pauses timer
  - Color-coded type badges: Insight (blue), Decision (green), Action (amber), Question (red), Summary (purple)
  - Expand/collapse individual cards or bulk expand/collapse all
  - State diffing engine (`createCardsFromStateDiff`) deduplicates across cycles — only new insights create cards
  - **Session Summary Card** appears when recording stops — aggregates summary, key takeaways, action items, decisions, open questions
  - Sticky footer shows cycle countdown, processing status, and session stats (cycles done, total audio analyzed)
  - Co-Pilot tab persists after recording stops so accumulated intelligence remains accessible
  - Notification dot on Co-Pilot tab when new data arrives while viewing transcript

### 5. Chat with Recordings (Conversational Q&A)
- **Chat with any recording** — click "Chat" on any recording to start a conversation about its content
- Trait-based architecture (`Chatable` trait) — the chatbot is reusable for any content source (recordings now, gems/live recordings later)
- **Automatic transcript generation** — if no transcript exists, generates one in the background via MLX Omni; shows spinner while preparing
- **Transcript reuse** — transcripts saved to `recordings/{stem}/transcript.md`; both Transcribe button and Chat share the same cached file
- **Non-blocking UX** — chat interface appears instantly, transcript generates in background with status events
- **Fresh context on every message** — chatbot re-reads the transcript on each turn (handles growing sources like live recordings)
- **Session logging** — every conversation saved as readable markdown (`recordings/{stem}/chat_session_{ts}.md`)
- **Chat history in LLM prompt** — last 10 exchanges included for multi-turn coherence, context truncated to 14K chars
- **Per-recording folder organization** — each recording gets its own subfolder for transcripts and chat sessions
- Session cleanup on recording switch — selecting a different recording ends the previous chat session
- **Repetition penalty** — logits processor prevents degenerate looping responses from the LLM
- Powered by `IntelQueue` — serializes all LLM requests (chat, co-pilot, enrichment) through a single mpsc channel

### 6. AI Enrichment (On-Device)
- **Auto-tagging**: Generate 3–5 topic tags from content
- **Auto-summarization**: One-sentence summary
- **Transcript generation**: High-quality post-recording transcript (MLX Omni)
- **Enrichment on save**: Gems are auto-enriched during creation if AI is available
- **Manual re-enrichment**: Enrich any existing gem on demand
- Pluggable provider architecture: **MLX** (local LLM) → **IntelligenceKit** (Apple Foundation Models) → **NoOp** (graceful fallback)

### 7. Model Management
- **Whisper models**: Download/delete OpenAI Whisper models from Hugging Face
- **WhisperKit models**: Download/manage Apple WhisperKit models
- **LLM models**: Download/delete/switch MLX-compatible LLMs (Qwen3, Qwen2.5-Omni 3B/7B, etc.)
- Real-time download progress tracking
- Switch active LLM model at runtime
- **7B conv weight auto-fix** — auto-detects and corrects PyTorch→MLX weight layout mismatch after model load

### 8. Settings & Configuration
- Transcription engine selection (whisper-rs / whisperkit / mlx-omni)
- VAD enable/disable with threshold tuning
- Whisper model selection
- Browser observer toggle (YouTube auto-detection)
- AI provider selection (MLX vs IntelligenceKit)
- Python path configuration for MLX
- Active LLM model switching
- Co-Pilot settings: auto-start toggle, cycle interval, audio overlap, agent logging
- MLX virtual environment setup/reset with diagnostics

---

## UI Architecture

**Three-panel layout:**

| Left Nav (180px, collapsible) | Center Panel (flex) | Right Panel (resizable, 250px–60%) |
|---|---|---|
| Record, Recordings, Gems, YouTube, Browser, Settings tabs | Main content for active section | Context panel: live transcript, recording details, gem details |

- Dark theme with design token system (CSS custom properties)
- Self-hosted Inter + JetBrains Mono fonts
- Resizable right panel via drag handle
- Tabbed right panel during recording: **Transcript** and **Co-Pilot** tabs with notification dot for unseen updates
- Tabbed right panel for recordings: **Details** and **Chat** tabs when a chat session is active
- Co-Pilot Card Stack with animated card entrance, auto-collapse timers, hover-to-pause, and keyboard-accessible expand/collapse
- Notification badge on YouTube tab when video detected
- Error toasts for runtime issues (MLX sidecar crashes)

---

## Technical Stack

| Layer | Technology |
|---|---|
| Desktop framework | Tauri 2.x |
| Backend | Rust (async/await, tokio) |
| Frontend | React 18 + TypeScript |
| Database | SQLite with FTS5 full-text search |
| AI inference | MLX (Python sidecar), Apple IntelligenceKit |
| Transcription | whisper-rs, WhisperKit (CLI), MLX Omni |
| Browser integration | AppleScript (Chrome), Accessibility API |
| IPC | Tauri commands (async request/response) + events |

## Data Storage

| Data | Location |
|---|---|
| Recordings (PCM) | `~/Library/Application Support/com.jarvis.app/recordings/` |
| Transcripts & chat logs | `~/Library/Application Support/com.jarvis.app/recordings/{stem}/` |
| Gems database | `~/Library/Application Support/com.jarvis.app/gems.db` |
| Whisper models | `~/Library/Application Support/com.jarvis.app/models/` |
| LLM models | `~/Library/Application Support/com.jarvis.app/llm_models/` |
| WhisperKit models | `~/.cache/huggingface/hub/` |
| Co-Pilot agent logs | `~/Library/Application Support/com.jarvis.app/agent_logs/` |
| Settings | `~/.jarvis/settings.json` |
| Gist exports | `~/.jarvis/gists/` |
| MLX venv | `~/.jarvis/mlx_venv/` |

---

## Backend Commands (Tauri RPC)

**55+ registered commands** across these domains:

- **Recording** (5): start, stop, list, delete, convert to WAV
- **Transcription** (4): get transcript, status, transcribe recording, transcribe gem
- **Gems** (10): save, list, search, filter by tag, get, delete, enrich, save recording gem, check recording gem, batch check
- **Chat** (5): start chat with recording, send message, get history, end session, get saved transcript
- **Co-Pilot** (4): start, stop, get state, dismiss question
- **AI/Intelligence** (9): availability check, MLX dependency check, venv setup/reset, MLX status, list/download/cancel/delete/switch LLM models
- **Model Management** (7): list/download/cancel/delete Whisper models, WhisperKit status/list/download
- **Browser** (10): start/stop observer, status, settings, list tabs, fetch YouTube gist, prepare gist, prepare with Claude, export, capture Claude, check Claude panel, accessibility permission
- **Settings** (3): get, update, browser settings
- **Platform** (2): support check, open system settings

---

## Key Design Decisions

- **Local-first**: All processing happens on-device — no data leaves the machine
- **Pluggable AI providers**: Fallback chain ensures graceful degradation
- **SQLite FTS5**: Fast full-text search without external search service
- **Sidecar architecture**: MLX inference runs as separate Python process to avoid blocking the Rust runtime
- **Upsert by URL**: Natural deduplication — recapturing a page updates rather than duplicates
- **PCM storage**: Raw audio for maximum flexibility, WAV conversion only for playback
- **Trait-based chatbot**: `Chatable` trait decouples the chat engine from content types — adding chat to gems means implementing one trait, zero chatbot changes
- **IntelQueue serialization**: All LLM access (chat, co-pilot, enrichment, transcription) goes through one mpsc queue — no mutex contention, predictable ordering
- **Per-recording folders**: Each recording gets a subfolder for transcripts and chat sessions, keeping related artifacts together
- **MLX sidecar runtime patches**: Six monkey-patches fix critical bugs in `mlx-lm-omni` v0.1.3 (AudioTower reshape, float32 precision, conv weight layout, tokenizer compat, prefill chunking)
