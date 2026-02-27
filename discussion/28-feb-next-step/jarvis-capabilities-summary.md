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

### 4. AI Enrichment (On-Device)
- **Auto-tagging**: Generate 3–5 topic tags from content
- **Auto-summarization**: One-sentence summary
- **Transcript generation**: High-quality post-recording transcript (MLX Omni)
- **Enrichment on save**: Gems are auto-enriched during creation if AI is available
- **Manual re-enrichment**: Enrich any existing gem on demand
- Pluggable provider architecture: **MLX** (local LLM) → **IntelligenceKit** (Apple Foundation Models) → **NoOp** (graceful fallback)

### 5. Model Management
- **Whisper models**: Download/delete OpenAI Whisper models from Hugging Face
- **WhisperKit models**: Download/manage Apple WhisperKit models
- **LLM models**: Download/delete/switch MLX-compatible LLMs (Qwen3, etc.)
- Real-time download progress tracking
- Switch active LLM model at runtime

### 6. Settings & Configuration
- Transcription engine selection (whisper-rs / whisperkit / mlx-omni)
- VAD enable/disable with threshold tuning
- Whisper model selection
- Browser observer toggle (YouTube auto-detection)
- AI provider selection (MLX vs IntelligenceKit)
- Python path configuration for MLX
- Active LLM model switching
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
| Gems database | `~/Library/Application Support/com.jarvis.app/gems.db` |
| Whisper models | `~/Library/Application Support/com.jarvis.app/models/` |
| LLM models | `~/Library/Application Support/com.jarvis.app/llm_models/` |
| WhisperKit models | `~/.cache/huggingface/hub/` |
| Settings | `~/.jarvis/settings.json` |
| Gist exports | `~/.jarvis/gists/` |
| MLX venv | `~/.jarvis/mlx_venv/` |

---

## Backend Commands (Tauri RPC)

**45+ registered commands** across these domains:

- **Recording** (5): start, stop, list, delete, convert to WAV
- **Transcription** (4): get transcript, status, transcribe recording, transcribe gem
- **Gems** (10): save, list, search, filter by tag, get, delete, enrich, save recording gem, check recording gem, batch check
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
