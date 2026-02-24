# Part 2: Tauri Integration via IntelProvider Trait

## Goal

Wire the IntelligenceKit persistent server (from Part 1) into the Jarvis Tauri app
using a provider-agnostic trait. Tags appear on gem cards, searchable via FTS5.

## Naming Convention

| Layer | Name | Purpose |
|-------|------|---------|
| Generic trait | `IntelProvider` | Provider-agnostic intelligence abstraction |
| Apple impl | `IntelligenceKitProvider` | Manages IntelligenceKit sidecar (persistent server) |
| Rust fallback | `KeywordProvider` | Pure Rust TF-IDF/YAKE extraction |
| Future | `ClaudeProvider` | Anthropic API |

## Prerequisites

- Part 1 complete: `IntelligenceKit` server works standalone from terminal
- Existing gems system working (save, list, search, delete, view)

## Deliverable

User saves a gem → tags automatically appear on the gem card a few seconds later.
User can search gems by tag text. Settings page shows which intel engine is active.

## Key Change from Part 1

IntelligenceKit is now a **persistent server**, not a short-lived process. The Rust
`IntelligenceKitProvider` spawns it once at app startup and communicates via the
session-based NDJSON protocol:

```
App startup → spawn IntelligenceKit sidecar → stays alive
Gem saved   → open-session → message(generate-tags) → close-session
App quit    → shutdown command → sidecar exits
```

## Tasks

### 1. Define IntelProvider Trait

New module: `src-tauri/src/intel/`

```
src-tauri/src/intel/
├── mod.rs                         # Module exports
├── provider.rs                    # IntelProvider trait + IntelResult type
├── intelligencekit_provider.rs    # Manages IntelligenceKit persistent sidecar
└── keyword_provider.rs            # TF-IDF/RAKE fallback (keyword-extraction-rs)
```

Trait shape:
- `name() -> &str`
- `is_available() -> bool`
- `unavailable_reason() -> Option<&str>`
- `generate_tags(title, content, source_type) -> Result<Vec<String>, String>`
- `generate_summary(title, content, source_type) -> Result<Option<String>, String>` (default: Ok(None))

Follows same conventions as `TranscriptionProvider` and `GemStore`.

### 2. Implement IntelligenceKitProvider

- Spawns IntelligenceKit sidecar **once** at app startup using `app_handle.shell().sidecar("IntelligenceKit")`
- Keeps process alive — writes NDJSON to stdin, reads responses from stdout
- On `generate_tags()`:
  1. Send `open-session` → get session_id
  2. If content > 10,000 chars: split into chunks, send multiple `message` commands on same session
  3. If content ≤ 10,000 chars: send single `message`
  4. Send `close-session`
  5. Merge and deduplicate tags from all responses
- On init: send `check-availability` to verify Foundation Models is ready
- On app shutdown: send `shutdown` command
- If sidecar dies unexpectedly: log error, mark as unavailable

### 3. Implement KeywordProvider

- Uses `keyword-extraction` crate (add to Cargo.toml)
- YAKE algorithm for single-document keyword extraction
- Always available, no external dependencies
- Returns 3-5 keywords as tags
- No summary support (default impl returns None)

### 4. Binary Setup

- Build IntelligenceKit release: `cd intelligence-kit && swift build -c release`
- Copy to `jarvis-app/src-tauri/binaries/IntelligenceKit-aarch64-apple-darwin`
- Add `"binaries/IntelligenceKit"` to `tauri.conf.json` externalBin array
- Add sidecar permission to `capabilities/default.json`

### 5. Database Changes

Add `tags` column to gems table:
- `ALTER TABLE gems ADD COLUMN tags TEXT DEFAULT '[]'`
- Handle migration: check if column exists before adding
- Include `tags` in FTS5 virtual table rebuild
- Update FTS5 triggers to sync tags column
- Add `update_tags(id, tags)` method to GemStore trait + SqliteGemStore

### 6. Update Gem Types

Rust:
- Add `tags: Vec<String>` to `Gem` struct (serialized as JSON array)
- Add `tags: Vec<String>` to `GemPreview` struct

TypeScript:
- Add `tags: string[]` to `Gem` and `GemPreview` interfaces

### 7. New Tauri Commands

```
generate_gem_tags(gem_id) -> Vec<String>
    Fetches gem content, calls intel.generate_tags(), updates DB, returns tags

get_intel_status() -> { engine: String, available: bool }
    Returns current provider name and availability
```

Register both in `invoke_handler` in `lib.rs`.

### 8. Provider Wiring in lib.rs

- Add `IntelSettings` to Settings struct (intel_engine field)
- In `setup()`: select provider based on settings (same pattern as transcription engine)
- IntelligenceKitProvider → try first, fall back to KeywordProvider
- Manage as `Arc<dyn IntelProvider>`

### 9. Auto-Tag on Gem Save

In `save_gem` command (or as a separate background task):
- After gem is saved to SQLite, spawn async task
- Call `intel.generate_tags()` with gem's content
- Update gem tags in DB via `gem_store.update_tags()`
- Emit `gem-tags-updated` Tauri event with gem ID
- Save remains instant — tagging happens in background

### 10. Frontend: Tag Display

In GemCard component:
- Render `gem.tags` as small badge chips below the title/meta area
- Style: pill-shaped, light indigo background, small font
- Only show if `tags.length > 0`

In GemsPanel:
- Listen for `gem-tags-updated` event
- Refresh the specific gem or full list when tags arrive

### 11. Frontend: Tag Search

Tags are included in FTS5 index, so existing search already works.
No frontend changes needed — searching "AI agents" will match gems tagged with it.

### 12. Settings UI

Add to Settings panel:
- "Intelligence Engine" dropdown: IntelligenceKit (Apple) / Keyword Extraction
- Show current status: "Active" or "Unavailable: {reason}"
- (Claude API option added in future phase)

## Provider Swap Summary

| Setting Value | Provider | How it works |
|---------------|----------|-------------|
| `intelligencekit` | IntelligenceKitProvider | Manages persistent IntelligenceKit sidecar |
| `keyword` | KeywordProvider | Pure Rust YAKE/TF-IDF |
| `claude-api` (future) | ClaudeProvider | HTTPS to Anthropic API |

All share `Arc<dyn IntelProvider>`. Commands don't change when adding new providers.

## Success Criteria

Part 2 is done when:
1. Saving a gem triggers background tag generation
2. Tags appear on gem cards within a few seconds
3. Tags are searchable via the existing search bar
4. Settings shows which engine is active
5. Falls back to KeywordProvider if IntelligenceKit unavailable
6. Existing gem functionality (save, view, delete, play audio) unaffected
7. IntelligenceKit sidecar starts with app, stays alive, shuts down with app

## What Part 2 Does NOT Include

- Claude API provider (future phase)
- Summary generation (future phase)
- Chat interface (future phase)
- Settings UI for API keys (future phase)
