# Knowledge Store + Browser Vision — Architecture Plan

**Date**: Feb 19, 2026
**Current state**: Record → Transcribe → Display (in-memory only)
**Goal**: Add persistent knowledge storage and passive browser observation

---

## Overview

Two new capabilities that transform JARVIS from a transcription tool into a knowledge-aware assistant:

1. **Knowledge Store** — Every transcript, browser observation, and LLM summary persists in a local SQLite database with full-text search
2. **Browser Vision** — JARVIS passively observes Chrome, detects YouTube videos, auto-fetches transcripts, and generates LLM summaries — without the user lifting a finger

Together, these create the foundation for the "digital shadow" narrative: JARVIS follows you across tools and remembers everything.

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      JARVIS APP                          │
│                                                          │
│  ┌──────────────┐    ┌──────────────┐    ┌────────────┐ │
│  │  Recording    │    │   Browser     │    │  Knowledge │ │
│  │  Manager      │    │   Observer    │    │  Store     │ │
│  │  (existing)   │    │   (NEW)       │    │  (NEW)     │ │
│  └──────┬───────┘    └──────┬───────┘    └─────┬──────┘ │
│         │                    │                   │        │
│         │  transcript        │  youtube URL      │        │
│         │  segments          │  detected         │        │
│         ▼                    ▼                   │        │
│  ┌──────────────┐    ┌──────────────┐           │        │
│  │  Transcript   │    │  YouTube      │           │        │
│  │  (existing)   │    │  Fetcher     │           │        │
│  │              │    │  (NEW)        │           │        │
│  └──────┬───────┘    └──────┬───────┘           │        │
│         │                    │                   │        │
│         │  final text        │  raw transcript   │        │
│         │                    │                   │        │
│         ▼                    ▼                   │        │
│  ┌─────────────────────────────────┐            │        │
│  │        LLM Provider             │            │        │
│  │   (Ollama local / Bedrock API)  │            │        │
│  │           (NEW)                 │            │        │
│  └──────────────┬──────────────────┘            │        │
│                 │                                │        │
│                 │  summary + action items        │        │
│                 │                                │        │
│                 └───────────────► STORE ─────────┘        │
│                                                          │
│  ┌──────────────────────────────────────────────────┐    │
│  │              React Frontend                       │    │
│  │  [Record] [Browser] [Knowledge] [Settings]       │    │
│  └──────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

---

## 1. Knowledge Store (SQLite + FTS5)

### Why SQLite

- Zero setup — single file at `~/.jarvis/knowledge.db`
- FTS5 built-in — full-text search with ranking, no external dependencies
- Portable — the entire knowledge base is one file you can back up or move
- Proven at scale — handles millions of rows, way beyond personal use

### Schema

```sql
-- Core knowledge entries
CREATE TABLE knowledge (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    source_type TEXT NOT NULL,          -- 'transcript', 'youtube', 'browser', 'note'
    source_id   TEXT,                   -- filename, URL, etc.
    title       TEXT,                   -- human-readable title
    content     TEXT NOT NULL,          -- raw text content
    summary     TEXT,                   -- LLM-generated summary (nullable)
    metadata    TEXT,                   -- JSON blob for source-specific data
    created_at  TEXT NOT NULL,          -- ISO 8601 timestamp
    updated_at  TEXT NOT NULL           -- ISO 8601 timestamp
);

-- Full-text search index
CREATE VIRTUAL TABLE knowledge_fts USING fts5(
    title,
    content,
    summary,
    content=knowledge,
    content_rowid=id
);

-- Triggers to keep FTS in sync
CREATE TRIGGER knowledge_ai AFTER INSERT ON knowledge BEGIN
    INSERT INTO knowledge_fts(rowid, title, content, summary)
    VALUES (new.id, new.title, new.content, new.summary);
END;

CREATE TRIGGER knowledge_ad AFTER DELETE ON knowledge BEGIN
    INSERT INTO knowledge_fts(knowledge_fts, rowid, title, content, summary)
    VALUES ('delete', old.id, old.title, old.content, old.summary);
END;

CREATE TRIGGER knowledge_au AFTER UPDATE ON knowledge BEGIN
    INSERT INTO knowledge_fts(knowledge_fts, rowid, title, content, summary)
    VALUES ('delete', old.id, old.title, old.content, old.summary);
    INSERT INTO knowledge_fts(rowid, title, content, summary)
    VALUES (new.id, new.title, new.content, new.summary);
END;

-- Action items extracted by LLM
CREATE TABLE action_items (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    knowledge_id  INTEGER NOT NULL REFERENCES knowledge(id),
    description   TEXT NOT NULL,
    is_completed  INTEGER DEFAULT 0,
    created_at    TEXT NOT NULL
);
```

### Rust Module: `src/knowledge/`

```
src/knowledge/
  mod.rs          — pub mod store; pub mod types;
  store.rs        — KnowledgeStore struct (wraps rusqlite Connection)
  types.rs        — KnowledgeEntry, ActionItem, SearchResult structs
```

**Key API:**

```
KnowledgeStore::new(db_path) → Result<Self>       // opens/creates DB, runs migrations
KnowledgeStore::insert(entry) → Result<i64>        // returns row ID
KnowledgeStore::search(query) → Result<Vec<SearchResult>>  // FTS5 search with snippets
KnowledgeStore::get(id) → Result<KnowledgeEntry>   // by ID
KnowledgeStore::list(source_type, limit) → Result<Vec<KnowledgeEntry>>  // filtered list
KnowledgeStore::update_summary(id, summary) → Result<()>  // after LLM processes
KnowledgeStore::add_action_item(knowledge_id, desc) → Result<()>
KnowledgeStore::list_action_items(completed?) → Result<Vec<ActionItem>>
```

**Crate**: `rusqlite` with `bundled` and `bundled-full` features (includes FTS5)

### Tauri Commands

```
get_knowledge_entries(source_type?, limit?) → Vec<KnowledgeEntry>
search_knowledge(query) → Vec<SearchResult>
get_knowledge_entry(id) → KnowledgeEntry
delete_knowledge_entry(id) → ()
get_action_items() → Vec<ActionItem>
toggle_action_item(id) → ()
```

### Auto-Ingest Points

1. **Transcription stopped** → save final transcript to knowledge store (source_type = "transcript")
2. **YouTube transcript fetched** → save to knowledge store (source_type = "youtube")
3. **LLM summary generated** → update the knowledge entry with summary

---

## 2. Browser Vision (Chrome Observer)

### How It Works — No Extension Needed

macOS AppleScript can query Chrome directly:

```applescript
tell application "Google Chrome"
    return URL of active tab of front window
end tell
```

This requires no browser extension, no permissions prompt, and works with any Chrome window.

### Rust Module: `src/browser/`

```
src/browser/
  mod.rs          — pub mod observer; pub mod youtube;
  observer.rs     — BrowserObserver (polls Chrome every 3-5 seconds)
  youtube.rs      — YouTubeTranscriptFetcher (extracts captions from video page)
```

### Browser Observer

**How it works:**

1. A background thread polls Chrome's active tab URL every 3-5 seconds via `osascript`
2. Compares current URL to last-seen URL (debounce — only fires on change)
3. If new URL is a YouTube video (`youtube.com/watch?v=`), emits a `youtube-video-detected` event
4. Frontend shows the detection in the Browser panel
5. User can click "Fetch Transcript" or it auto-fetches (configurable)

**Rust crate**: `osascript` (by mitsuhiko) — simple, well-maintained

**AppleScript execution:**

```rust
use std::process::Command;

fn get_chrome_url() -> Option<String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"Google Chrome\" to return URL of active tab of front window")
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None  // Chrome not running or no window open
    }
}
```

**Edge cases to handle:**
- Chrome not running → return None, don't error
- No window open → AppleScript returns error, catch gracefully
- Multiple windows → AppleScript returns front window (good default)
- Non-YouTube URLs → log for future use, don't process yet

### YouTube Transcript Fetcher

**How it works (no API key needed):**

1. Fetch the YouTube video page HTML via `reqwest`
2. Parse `ytInitialPlayerResponse` JSON from the page source
3. Extract `captionTracks` array from the player response
4. Pick the best caption track (prefer manual captions > auto-generated, prefer English)
5. Fetch the caption track XML URL
6. Parse XML `<text>` elements into timestamped segments
7. Join into plain text transcript

**Why no API key**: YouTube embeds caption data directly in the video page HTML. The `youtube-transcript-api` Python library works this way. We replicate the same approach in Rust with `reqwest` + `regex` + `quick-xml`.

**Data extracted:**

```
YouTubeVideoInfo {
    video_id: String,         // "dQw4w9WgXcQ"
    title: String,            // from <title> tag or ytInitialPlayerResponse
    channel: String,          // channel name
    duration: Option<u64>,    // seconds
    transcript: Vec<CaptionSegment>,  // timestamped text
}

CaptionSegment {
    text: String,
    start_ms: u64,
    duration_ms: u64,
}
```

**Crates needed:**
- `reqwest` (already in deps for model downloads)
- `regex` (for parsing ytInitialPlayerResponse from HTML)
- `quick-xml` (for parsing caption XML)
- `serde_json` (already in deps)

### Event Flow

```
BrowserObserver (3s poll loop)
    │
    ├── URL changed? No → sleep 3s → loop
    │
    ├── URL changed? Yes
    │       │
    │       ├── Is YouTube video?
    │       │       │
    │       │       ├── Yes → emit "youtube-video-detected" { video_id, url, title }
    │       │       │         │
    │       │       │         ├── Auto-fetch enabled?
    │       │       │         │       ├── Yes → YouTubeTranscriptFetcher::fetch(video_id)
    │       │       │         │       │         │
    │       │       │         │       │         ├── Success → emit "youtube-transcript-ready" { transcript }
    │       │       │         │       │         │              │
    │       │       │         │       │         │              ├── Save to KnowledgeStore
    │       │       │         │       │         │              └── Send to LLM for summary
    │       │       │         │       │         │
    │       │       │         │       │         └── Fail → emit "youtube-transcript-error" { error }
    │       │       │         │       │
    │       │       │         │       └── No → show "Fetch" button in UI
    │       │       │         │
    │       │       │         └── Frontend updates Browser panel
    │       │       │
    │       │       └── No → log URL (future: detect articles, docs, etc.)
    │       │
    │       └── Update last_seen_url
    │
    └── sleep 3s → loop
```

---

## 3. LLM Provider (Dual: Local + Cloud)

### Provider Trait

Following the same pattern as `TranscriptionProvider`:

```
trait LlmProvider {
    fn summarize(text: &str, context: &str) -> Result<LlmResponse>
    fn extract_actions(text: &str) -> Result<Vec<String>>
    fn is_available() -> bool
}
```

### Two Implementations

| Provider | When to use | Latency | Cost | Privacy |
|----------|-------------|---------|------|---------|
| **OllamaProvider** | Default, local | 2-10s | Free | Full privacy |
| **BedrockProvider** | Better quality, cloud | 1-3s | ~$0.01/call | AWS |

### Rust Module: `src/intelligence/`

```
src/intelligence/
  mod.rs           — pub mod provider; pub mod ollama; pub mod bedrock;
  provider.rs      — LlmProvider trait + LlmResponse type
  ollama.rs        — OllamaProvider (HTTP to localhost:11434)
  bedrock.rs       — BedrockProvider (AWS SDK, Claude Haiku/Sonnet)
```

### Ollama Integration

- HTTP API at `localhost:11434`
- Models: `llama3.2:3b` (fast), `mistral` (balanced), `llama3.1:8b` (better)
- Availability check: `GET /api/tags` — if Ollama is running and has models
- Simple `POST /api/generate` with prompt

### Bedrock Integration

- AWS SDK for Rust (`aws-sdk-bedrockruntime`)
- Models: Claude Haiku (fast/cheap), Claude Sonnet (better quality)
- Requires AWS credentials configured (`~/.aws/credentials` or env vars)
- Uses `InvokeModel` API with Messages format

### Settings Addition

```
intelligence:
  provider: "ollama" | "bedrock"     # which LLM to use
  ollama_model: "llama3.2:3b"        # which Ollama model
  bedrock_model: "claude-3-haiku"    # which Bedrock model
  auto_summarize: true               # auto-summarize new knowledge entries
  auto_extract_actions: true         # auto-extract action items
```

---

## 4. Frontend Components

### New Components

| Component | Purpose |
|-----------|---------|
| `KnowledgePanel.tsx` | Main knowledge view — search bar, entry list, detail view |
| `BrowserPanel.tsx` | Browser observation status, detected videos, transcript previews |
| `ActionItems.tsx` | Extracted action items with checkboxes |
| `KnowledgeSearch.tsx` | Search input + results with highlighted snippets |

### UI Layout Change

Current app is a single vertical layout. Add a tab system:

```
┌─────────────────────────────────────────┐
│  JarvisApp                    [⚙️]       │
├────────┬──────────┬───────────┬─────────┤
│ Record │ Browser  │ Knowledge │ Actions │
├────────┴──────────┴───────────┴─────────┤
│                                          │
│  [Active tab content here]               │
│                                          │
└──────────────────────────────────────────┘
```

### Record Tab (existing, enhanced)
- Same as current: record button, recordings list, transcript display
- NEW: "Save to Knowledge" button on transcript display
- NEW: Auto-save toggle in settings

### Browser Tab (new)
- Observer status: ON/OFF toggle
- Current detected URL
- YouTube detection card:
  ```
  ┌─ YouTube Detected ────────────────────┐
  │ 🎬 "How to Build AI Agents in 2026"   │
  │ Channel: TechWithTim • 15:32          │
  │                                        │
  │ Transcript: ✅ Fetched (2,340 words)   │
  │ Summary: ✅ Generated                  │
  │                                        │
  │ "This video covers building AI agents  │
  │  using AWS Bedrock and Strands SDK..." │
  │                                        │
  │ [View Full] [Copy Summary] [Save]      │
  └────────────────────────────────────────┘
  ```

### Knowledge Tab (new)
- Search bar at top
- Filtered tabs: All | Transcripts | YouTube | Notes
- Entry list with source icon, title, date, snippet
- Click to expand full content + summary

### Actions Tab (new)
- Checklist of extracted action items
- Source link (which knowledge entry it came from)
- Filter: All | Pending | Completed

---

## 5. File Manifest

### New Files (Backend — Rust)

| File | Purpose |
|------|---------|
| `src/knowledge/mod.rs` | Module declaration |
| `src/knowledge/store.rs` | KnowledgeStore — SQLite + FTS5 operations |
| `src/knowledge/types.rs` | KnowledgeEntry, ActionItem, SearchResult |
| `src/browser/mod.rs` | Module declaration |
| `src/browser/observer.rs` | BrowserObserver — Chrome URL polling |
| `src/browser/youtube.rs` | YouTubeTranscriptFetcher — caption extraction |
| `src/intelligence/mod.rs` | Module declaration |
| `src/intelligence/provider.rs` | LlmProvider trait |
| `src/intelligence/ollama.rs` | OllamaProvider implementation |
| `src/intelligence/bedrock.rs` | BedrockProvider implementation |

### New Files (Frontend — React/TypeScript)

| File | Purpose |
|------|---------|
| `src/components/KnowledgePanel.tsx` | Knowledge store UI |
| `src/components/BrowserPanel.tsx` | Browser observer UI |
| `src/components/ActionItems.tsx` | Action items checklist |
| `src/components/KnowledgeSearch.tsx` | Search with FTS5 highlights |
| `src/components/TabLayout.tsx` | Tab navigation wrapper |

### Modified Files

| File | Change |
|------|--------|
| `src-tauri/Cargo.toml` | Add `rusqlite`, `quick-xml`, `aws-sdk-bedrockruntime` |
| `src-tauri/src/lib.rs` | Register new modules + Tauri commands |
| `src-tauri/src/commands.rs` | Add knowledge, browser, intelligence commands |
| `src/App.tsx` | Add tab layout, integrate new panels |
| `src/state/types.ts` | Add knowledge, browser, LLM types |
| `src/App.css` | Tab styles, new panel styles |
| `src-tauri/src/settings/manager.rs` | Add intelligence settings section |

---

## 6. Implementation Phases

### Phase 1: Knowledge Store (Foundation) — ~1.5 days

- Add `rusqlite` to Cargo.toml
- Create `src/knowledge/` module with store, types
- Create SQLite database at `~/.jarvis/knowledge.db`
- Implement CRUD + FTS5 search
- Add Tauri commands for knowledge operations
- Wire transcription-stopped event to auto-save transcripts
- Frontend: KnowledgePanel with search and entry list

### Phase 2: Browser Observer — ~1 day

- Create `src/browser/observer.rs` with Chrome polling
- Detect YouTube URLs via regex pattern matching
- Emit Tauri events for URL changes and YouTube detection
- Frontend: BrowserPanel showing observer status and detections
- Settings: observer ON/OFF toggle, poll interval

### Phase 3: YouTube Transcript Fetcher — ~1 day

- Create `src/browser/youtube.rs`
- Fetch video page HTML, parse `ytInitialPlayerResponse`
- Extract caption tracks, fetch caption XML
- Parse into timestamped segments
- Auto-save to knowledge store
- Frontend: YouTube card in BrowserPanel with transcript preview

### Phase 4: LLM Integration — ~1.5 days

- Create `src/intelligence/` module
- Implement OllamaProvider (HTTP to localhost:11434)
- Implement BedrockProvider (AWS SDK)
- Auto-summarize new knowledge entries
- Extract action items from transcripts and YouTube videos
- Frontend: Summary display, ActionItems component
- Settings: provider selection, model choice

### Phase 5: Tab Layout + Polish — ~1 day

- Refactor App.tsx with TabLayout
- Integrate all panels
- Add loading states, error handling
- Test end-to-end flows

**Total: ~6 days of focused work**

---

## 7. Rust Dependencies to Add

```toml
[dependencies]
# Knowledge Store
rusqlite = { version = "0.31", features = ["bundled", "bundled-full"] }

# YouTube transcript parsing
quick-xml = "0.31"
# reqwest already present for model downloads
# regex already present
# serde_json already present

# LLM - Bedrock (optional, behind feature flag)
aws-sdk-bedrockruntime = "1.0"
aws-config = "1.0"
```

---

## 8. How This Connects to the AWS Competition

| Local Feature | AWS Competition Equivalent | Story |
|---------------|---------------------------|-------|
| Knowledge Store (SQLite) | Bedrock Knowledge Bases + S3 | "Remember everything" |
| Browser Observer | EventBridge + custom source | "Follow the user" |
| YouTube Fetcher | Lambda + Transcribe | "Ingest any source" |
| LLM Summary (Ollama) | Bedrock Claude | "Augment thinking" |
| FTS5 Search | OpenSearch / Neptune | "Unified memory" |
| Action Items | Action Agent (Bedrock) | "Capture commitments" |

The local app proves the concept works offline with zero cloud cost. The AWS version scales it with managed services, multi-user support, and more sophisticated retrieval (GraphRAG, episodic memory).

Same narrative, two implementations. The local app is the prototype. The AWS version is the production vision.

---

## 9. Key Design Decisions

1. **SQLite over files** — Structured storage with FTS5 beats flat JSON files once you have >10 entries
2. **AppleScript over browser extension** — Zero user setup, works immediately, macOS-only is fine for the prototype
3. **Dual LLM provider** — Local Ollama for privacy/speed, Bedrock for quality. User chooses in settings
4. **No API key for YouTube** — Caption data is embedded in page HTML. No Google API setup needed
5. **Polling over events** — 3-second poll is simple and reliable. Chrome doesn't offer native observation hooks without an extension
6. **Trait pattern for LLM** — Same pattern used for transcription providers. Consistent, testable, extensible

---

## 10. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| YouTube blocks automated transcript fetching | Breaks YouTube feature | Fallback: use `yt-dlp` CLI as subprocess |
| Ollama not installed on user's machine | No local LLM | Graceful fallback: skip summarization, show raw text |
| AppleScript requires accessibility permissions | Observer won't work | Detect permission error, show dialog like mic permission |
| rusqlite bundled increases binary size | Larger .app | ~5MB increase — acceptable for desktop app |
| Bedrock requires AWS credentials | Cloud LLM won't work | Make it optional, Ollama is the default |
| Caption XML format changes | YouTube parsing breaks | Version the parser, add fallback regex patterns |
