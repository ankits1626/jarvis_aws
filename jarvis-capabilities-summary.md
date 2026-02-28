# Jarvis — What It Can Do (Mar 1, 2026)

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

### 7. Gem Knowledge Files (Portable Markdown Documents)
- Every gem automatically generates a **knowledge directory** with structured markdown subfiles
- **Subfiles**: `content.md` (raw content), `enrichment.md` (AI tags/summary), `transcript.md` (transcript if audio gem), `copilot.md` (Co-Pilot analysis if recording gem), `gem.md` (assembled master document)
- `gem.md` is the **assembled output** — combines all subfiles into a single portable document with frontmatter metadata
- **Lifecycle integration**: knowledge files auto-generated on gem save, enrichment, transcription, and Co-Pilot data save
- **Co-Pilot backfill**: recording gems automatically pull Co-Pilot logs from `agent_logs/` and assemble into `copilot.md`
- **Migration**: existing gems without knowledge files are backfilled on first app launch
- **Versioned format** with `meta.json` tracking schema version and assembly timestamps
- `KnowledgeStore` trait with `LocalKnowledgeStore` filesystem implementation — `get()`, `create()`, `delete()`, `get_subfile()`, `get_assembled()`
- Knowledge files stored at `~/Library/Application Support/com.jarvis.app/knowledge/{gem_id}/`

### 8. Gem Knowledge Viewer (Tabbed File Browser)
- **File tree** in gem detail panel — lists all knowledge `.md` subfiles with icons, filenames (monospace), and human-readable sizes
- Files sorted by purpose: content.md, enrichment.md, transcript.md, copilot.md, gem.md (assembled last)
- `meta.json` excluded from display
- **Click to open** — clicking a file opens it as a new tab in the right panel alongside the "Detail" tab
- **Tabbed viewer** follows the same pattern as recording tabs (Details + Chat) — reuses `.record-tabs-view` CSS
- **Multiple tabs** — open several files simultaneously, switch between them freely
- **Closeable tabs** — each file tab has a `×` close button (with `stopPropagation` to prevent tab switching)
- **Lazy loading** — file content fetched on first click via `get_gem_knowledge_subfile` Tauri command
- **Content caching** — switching tabs doesn't re-fetch; cache cleared when closing tab or switching gems
- **Monospace `<pre>` display** — raw markdown rendered in preformatted text with word-wrap
- **Empty state** — gems without knowledge files show "No knowledge files" + "Generate" button
- **Generate button** — triggers `regenerate_gem_knowledge` to create knowledge files on demand
- **Gem switch reset** — all open tabs and cached content cleared when selecting a different gem

### 9. Searchable Gems — Semantic Search via QMD
- **SearchResultProvider trait** — backend-agnostic search interface (`search`, `index_gem`, `remove_gem`, `reindex_all`, `check_availability`). Consumers call `dyn SearchResultProvider`, never a concrete type
- **Two implementations ship:**
  - **FtsResultProvider** (default) — wraps existing SQLite FTS5 `GemStore::search()`. Always available, zero setup. Returns `MatchType::Keyword`
  - **QmdResultProvider** (opt-in) — wraps [QMD](https://github.com/tobi/qmd) CLI for hybrid semantic search over `gem.md` knowledge files. Returns `MatchType::Hybrid` (BM25 + vector embeddings via Gemma 300M + LLM reranking via Qwen3 0.6B)
- **Automated 7-step setup flow** — user clicks "Enable Semantic Search" in Settings, Jarvis handles everything:
  1. Check Node.js 22+
  2. Install Homebrew SQLite
  3. Install QMD (`npm install -g @tobilu/qmd`)
  4. Create `jarvis-gems` collection pointing at `knowledge/` directory
  5. Index & embed all gem knowledge files (~1.9GB model download: embedding 313MB + reranking 610MB)
  6. Warm up query expansion model (~1.2GB, runs dummy query to trigger download)
  7. Save `semantic_search_enabled: true` to settings
- **Step-by-step progress events** emitted on `"semantic-search-setup"` channel with spinner → checkmark → error UX
- **Provider selected on app startup** based on `settings.search.semantic_search_enabled`. Restart required to switch providers
- **Fallback chain**: QMD binary not found → QMD unavailable → falls back to FTS5 with warning log
- **QMD result parsing**: extracts `gem_id` from `qmd://jarvis-gems/{gem_id}/subfile.md` URIs, deduplicates multiple chunks per gem (keeps highest score), normalizes scores to 0.0–1.0
- **Configurable accuracy threshold** (`semantic_search_accuracy` setting, default 75%) — results below this score are discarded. Slider in Settings UI (50%–100%, step 5%)
- **Robustness**: query length capped at 200 chars (QMD reranker crashes on long inputs), graceful degradation on QMD failure (returns empty results instead of error)
- **Enter-to-search** — search only fires on Enter key press, not on every keystroke (avoids spamming QMD with concurrent queries)
- **Searching activity indicator** — spinner overlay with "Searching... Semantic search may take a few seconds" message, input disabled during search
- **Relevance score badges** — gem cards show percentage badge (e.g., "89%") for Semantic/Hybrid matches, hidden for Keyword matches
- **Fire-and-forget indexing** — `index_gem()` and `remove_gem()` spawn `qmd update && qmd embed` in background without awaiting
- **Lifecycle integration** — search index updated on gem save, enrichment, transcription, and deletion
- **Module structure**: `src/search/` with `provider.rs` (trait + types), `fts_provider.rs`, `qmd_provider.rs`, `commands.rs`, `mod.rs`
- **Tauri commands**: `search_gems`, `check_search_availability`, `setup_semantic_search`, `rebuild_search_index`

### 10. Application Logging
- **File-based logging** — captures all `eprintln!` output to timestamped log files
- Log files at `~/Library/Application Support/com.jarvis.app/logs/jarvis-YYYY-MM-DD_HH-MM-SS.log`
- **New log file per app launch** — each session gets its own log
- **Log rotation** — keeps last 5 log files, deletes older ones on startup
- **Timestamped lines** — each line prefixed with `[HH:MM:SS.mmm]` (e.g., `[14:30:05.123] Search: search_gems called...`)
- **Tee architecture** — OS-level pipe (`libc::pipe` + `libc::dup2`) redirects stderr to both terminal and log file simultaneously via background thread
- **Zero-config** — initialized at very start of `run()` before any other code runs, all existing `eprintln!` calls throughout the codebase automatically captured
- Module: `src/logging.rs` with `init()`, `logs_dir()`, `rotate_logs()`

### 11. Model Management
- **Whisper models**: Download/delete OpenAI Whisper models from Hugging Face
- **WhisperKit models**: Download/manage Apple WhisperKit models
- **LLM models**: Download/delete/switch MLX-compatible LLMs (Qwen3, Qwen2.5-Omni 3B/7B, etc.)
- Real-time download progress tracking
- Switch active LLM model at runtime
- **7B conv weight auto-fix** — auto-detects and corrects PyTorch→MLX weight layout mismatch after model load

### 12. Settings & Configuration
- Transcription engine selection (whisper-rs / whisperkit / mlx-omni)
- VAD enable/disable with threshold tuning
- Whisper model selection
- Browser observer toggle (YouTube auto-detection)
- AI provider selection (MLX vs IntelligenceKit)
- Python path configuration for MLX
- Active LLM model switching
- Co-Pilot settings: auto-start toggle, cycle interval, audio overlap, agent logging
- MLX virtual environment setup/reset with diagnostics
- **Semantic Search section**: enable/disable toggle, automated setup with step-by-step progress, rebuild index button, accuracy threshold slider (50–100%), availability status display

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
- Tabbed right panel for gems: **Detail** + open knowledge file tabs (closeable, lazy-loaded)
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
| Semantic search | QMD (local CLI — BM25 + Gemma 300M embeddings + Qwen3 0.6B reranker) |
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
| Knowledge files | `~/Library/Application Support/com.jarvis.app/knowledge/{gem_id}/` |
| Co-Pilot agent logs | `~/Library/Application Support/com.jarvis.app/agent_logs/` |
| Application logs | `~/Library/Application Support/com.jarvis.app/logs/` |
| QMD search models | `~/.cache/qmd/models/` (~2.1GB) |
| Settings | `~/.jarvis/settings.json` |
| Gist exports | `~/.jarvis/gists/` |
| MLX venv | `~/.jarvis/mlx_venv/` |

---

## Backend Commands (Tauri RPC)

**65+ registered commands** across these domains:

- **Recording** (5): start, stop, list, delete, convert to WAV
- **Transcription** (4): get transcript, status, transcribe recording, transcribe gem
- **Gems** (10): save, list, search, filter by tag, get, delete, enrich, save recording gem, check recording gem, batch check
- **Search** (4): search gems (trait-based), check search availability, setup semantic search, rebuild search index
- **Chat** (5): start chat with recording, send message, get history, end session, get saved transcript
- **Co-Pilot** (4): start, stop, get state, dismiss question
- **AI/Intelligence** (9): availability check, MLX dependency check, venv setup/reset, MLX status, list/download/cancel/delete/switch LLM models
- **Model Management** (7): list/download/cancel/delete Whisper models, WhisperKit status/list/download
- **Browser** (10): start/stop observer, status, settings, list tabs, fetch YouTube gist, prepare gist, prepare with Claude, export, capture Claude, check Claude panel, accessibility permission
- **Knowledge** (5): get knowledge, get assembled, get subfile, regenerate, check availability
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
