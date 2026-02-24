# Requirements Document

## Introduction

This spec integrates IntelligenceKit — the macOS Swift server that wraps Apple's on-device Foundation Models (3B parameter LLM) — into the Jarvis Tauri app as a sidecar process. The goal is to automatically enrich Gems with AI-generated tags and summaries at save time, running entirely on-device with zero API keys or network requests.

Today, when a user saves a Gem (extracted from YouTube, Medium, Gmail, ChatGPT, or any page), it stores only the raw extracted content. There is no automatic intelligence layer — no tags for browsing, no summary for quick scanning. Users must read the full content to recall what a Gem contains.

This integration adds an **IntelProvider trait** (analogous to GemStore for persistence — backend-agnostic, swappable implementations) with **IntelligenceKitProvider** as the default implementation. When a Gem is saved, the enrichment pipeline automatically generates topic tags and a one-sentence summary via IntelligenceKit. If Apple Intelligence is unavailable (older Mac, Intel hardware, or user hasn't enabled it), Gems are saved without enrichment — the system degrades gracefully, never blocks.

IntelligenceKit already exists as a fully built and tested Swift binary at `intelligence-kit/.build/debug/IntelligenceKit`. It communicates over stdin/stdout using NDJSON (newline-delimited JSON), supports multiple concurrent sessions, and handles `check-availability`, `open-session`, `message`, `close-session`, and `shutdown` commands. The Rust side is purely a client — it sends prompts and content, and IntelligenceKit returns structured results via guided generation. All task-specific intelligence (what prompts to use, how to interpret results) lives in the Rust client.

The integration follows the same sidecar pattern as JarvisListen: spawned via `tauri_plugin_shell`, monitored for stderr/termination events, and shut down gracefully on app exit.

## Glossary

- **IntelligenceKit**: The macOS Swift server binary that provides a generic gateway to Apple's Foundation Models. Communicates via NDJSON over stdin/stdout. Already built and located at `intelligence-kit/.build/debug/IntelligenceKit`.
- **IntelProvider**: A Rust trait defining the intelligence backend interface. Operations: `check_availability`, `generate_tags`, `summarize`. Implementations are swappable (IntelligenceKitProvider, future ClaudeProvider, KeywordProvider).
- **IntelligenceKitProvider**: The default IntelProvider implementation that communicates with the IntelligenceKit sidecar process via NDJSON over stdin/stdout.
- **Enrichment**: The process of augmenting a Gem with AI-generated metadata (tags, summary) at save time. Enrichment is optional — if the provider is unavailable, the Gem is saved without it.
- **ai_enrichment**: A single JSON column on the `gems` table that holds all AI-derived metadata. Mirrors the existing `source_meta` pattern (extraction-origin metadata). Contains `tags`, `summary`, `provider`, `enriched_at`, and future enrichment fields. NULL when no AI enrichment has been applied.
- **Tags**: An array of 3-5 short topic strings (1-3 words each) auto-generated from a Gem's content. Stored inside `ai_enrichment.tags`. Used for browsing and filtering.
- **Summary**: A single sentence (max ~100 words) that captures the key idea of a Gem's content. Stored inside `ai_enrichment.summary`. Used for quick scanning in list views.
- **NDJSON**: Newline-Delimited JSON — the wire protocol between the Rust client and IntelligenceKit server. Each message is a single JSON object terminated by `\n`.
- **Sidecar**: An external binary spawned by Tauri as a child process. IntelligenceKit runs as a persistent sidecar, similar to JarvisListen.
- **Guided Generation**: IntelligenceKit's technique where the model's output is constrained to match a declared schema (`string_list` or `text` format), guaranteeing valid structured output.
- **Graceful Degradation**: When IntelligenceKit is unavailable (hardware, OS, or user choice), the system continues to work — Gems are saved without tags/summary, and the UI indicates enrichment is unavailable.

## Requirements

### Requirement 1: IntelProvider Trait

**User Story:** As a developer, I want the intelligence backend to be abstracted behind a trait, so that I can swap IntelligenceKit for a cloud API, a keyword extractor, or a composite of providers without changing commands or frontend.

#### Acceptance Criteria

1. THE System SHALL define an `IntelProvider` trait in a new `intelligence` module with async methods: `check_availability`, `generate_tags`, `summarize`
2. THE `check_availability` method SHALL return a result indicating whether the provider is ready, with an optional reason string when unavailable
3. THE `generate_tags` method SHALL accept content (String) and return a `Vec<String>` of 3-5 topic tags
4. THE `summarize` method SHALL accept content (String) and return a single summary String
5. THE trait SHALL be `Send + Sync` so it can be used as Tauri managed state behind `Arc<dyn IntelProvider>`
6. THE Tauri commands SHALL depend on the `IntelProvider` trait (via trait object), NOT on IntelligenceKitProvider directly
7. FUTURE implementations (e.g., `ClaudeProvider`, `KeywordProvider`) SHALL be addable by implementing the trait without modifying commands or frontend

### Requirement 2: IntelligenceKit Sidecar Lifecycle

**User Story:** As the Tauri app, I want IntelligenceKit to be managed as a persistent sidecar process, so that enrichment requests are fast (no process spawn overhead per request).

#### Acceptance Criteria

1. THE System SHALL register IntelligenceKit as an external binary in `tauri.conf.json` under `externalBin`, alongside JarvisListen
2. THE IntelligenceKitProvider SHALL spawn IntelligenceKit once during app initialization in `setup()`, using `tauri_plugin_shell`'s sidecar API
3. THE provider SHALL hold references to the child process's stdin (for writing commands) and stdout (for reading responses)
4. THE provider SHALL monitor stderr for log messages and emit them to Rust's stderr prefixed with `[IntelligenceKit]`
5. THE provider SHALL monitor the `Terminated` event and log the exit code; if unexpected termination occurs, it SHALL mark itself as unavailable
6. WHEN the Tauri app shuts down, THE provider SHALL send a `{"command":"shutdown"}` message to IntelligenceKit and wait up to 3 seconds for graceful exit before sending SIGTERM
7. THE provider SHALL call `check-availability` on IntelligenceKit at startup to determine initial availability state
8. IF IntelligenceKit is not available (binary not found, or check-availability returns `available: false`), THE provider SHALL mark itself as unavailable and log the reason — the app SHALL continue to function without enrichment

### Requirement 3: NDJSON Client Communication

**User Story:** As a developer, I want a reliable Rust client for IntelligenceKit's NDJSON protocol, so that commands are sent and responses received correctly.

#### Acceptance Criteria

1. THE IntelligenceKitProvider SHALL implement an internal `send_command` method that writes a JSON object + newline to IntelligenceKit's stdin and reads one JSON line from stdout
2. THE client SHALL serialize commands using `serde_json` and deserialize responses into typed Rust structs
3. THE client SHALL handle the following response shapes: `{"ok":true, ...}` (success with optional fields) and `{"ok":false, "error":"..."}` (error)
4. THE client SHALL use a mutex or channel to ensure only one command is in-flight at a time (IntelligenceKit processes sequentially)
5. THE client SHALL enforce a 30-second timeout per command to prevent indefinite hangs
6. WHEN a command times out or IntelligenceKit returns an error, THE client SHALL return a descriptive Rust error — it SHALL NOT crash or panic

### Requirement 4: Session Management

**User Story:** As the Tauri app, I want IntelligenceKit sessions managed automatically, so that enrichment requests can leverage multi-turn context without manual session handling.

#### Acceptance Criteria

1. THE IntelligenceKitProvider SHALL open a single shared session at startup (after confirming availability) with instructions optimized for content analysis tasks
2. THE session instructions SHALL be generic (e.g., "You are a content analysis assistant. Follow the user's instructions precisely.") — task-specific instructions go in individual message prompts
3. IF the session is closed by IntelligenceKit (idle timeout after 120s of inactivity), THE provider SHALL transparently re-open a new session on the next request
4. THE provider SHALL store the current `session_id` and detect `session_not_found` errors to trigger automatic session re-creation
5. THE provider SHALL close the session as part of the shutdown sequence

### Requirement 5: Gem Enrichment Pipeline

**User Story:** As a JARVIS user, I want my Gems automatically enriched with AI-generated tags and a summary when I save them, so that I can browse and scan my knowledge collection efficiently.

#### Acceptance Criteria

1. WHEN a Gem is saved via the `save_gem` command, THE System SHALL attempt to enrich it with tags and a summary before persisting
2. THE enrichment SHALL use the IntelProvider trait to generate tags (via `generate_tags`) and a summary (via `summarize`) from the Gem's content (or `description` if content is empty)
3. THE tag generation prompt SHALL instruct the model to produce 3-5 topic tags, each 1-3 words, covering the main themes of the content
4. THE summarization prompt SHALL instruct the model to produce a single sentence capturing the key idea, suitable for display in a list view
5. THE enrichment SHALL be **non-blocking to the save operation**: if the IntelProvider is unavailable or returns an error, THE Gem SHALL be saved without tags/summary (fields set to null)
6. THE enrichment SHALL run both tag generation and summarization in the same session for efficiency
7. WHEN enrichment succeeds, THE Gem's `ai_enrichment` field SHALL be populated with a JSON object containing `tags`, `summary`, `provider`, and `enriched_at` before persisting to the database
8. THE `save_gem` command SHALL return the saved Gem (with or without enrichment) to the frontend — the presence of a non-null `ai_enrichment` field indicates whether AI enrichment was applied

### Requirement 6: Gem Schema Extension (ai_enrichment JSON column)

**User Story:** As a developer, I want the Gem data model extended with a single JSON column for all AI-derived metadata, so that enrichment results are persisted, searchable, and future-proof for new enrichment types without schema migrations.

**Design Decision:** AI-generated metadata (tags, summary, and future enrichment types like embeddings, sentiment, key_quotes) is stored in a single `ai_enrichment` JSON column rather than as direct columns. This mirrors the existing `source_meta` pattern — `source_meta` holds extraction-origin metadata, `ai_enrichment` holds AI-origin metadata. The key reasons:
- **Separation of concerns**: Derived AI data is semantically different from user-provided data (it's regenerable, provider-dependent, and optional)
- **Future-proof**: New enrichment types (embeddings, sentiment, reading difficulty) are new JSON keys, not schema migrations
- **Provenance**: The JSON naturally includes `provider` and `enriched_at` fields, answering "who generated this and when?"
- **Re-enrichment**: Swapping to a better model just overwrites the blob; the core Gem is untouched

#### Acceptance Criteria

1. THE `Gem` struct SHALL be extended with one new field: `ai_enrichment` (Option<serde_json::Value>) containing a JSON object with AI-generated metadata
2. THE `ai_enrichment` JSON object SHALL have the following structure when populated: `{"tags": ["tag1", "tag2", ...], "summary": "...", "provider": "intelligencekit", "enriched_at": "ISO 8601 timestamp"}`
3. THE `GemPreview` struct SHALL be extended with `tags` (Option<Vec<String>>) and `summary` (Option<String>), extracted from `ai_enrichment` at the Rust layer for frontend convenience
4. THE `gems` SQLite table SHALL be migrated to add one new column: `ai_enrichment` (TEXT, nullable, JSON string)
5. THE schema migration SHALL be backwards-compatible: existing gems without enrichment SHALL have `ai_enrichment` as NULL (not empty JSON — NULL means "never enriched")
6. THE FTS5 index SHALL be updated to include the summary extracted from `ai_enrichment` via `json_extract(ai_enrichment, '$.summary')` in the sync triggers, so that AI summaries are searchable alongside title, description, and content
7. THE `gem_to_preview` helper SHALL extract `tags` and `summary` from the `ai_enrichment` JSON and populate the corresponding GemPreview fields (summary is NOT truncated — it's already short)
8. TAG filtering SHALL use SQLite's `json_each(json_extract(ai_enrichment, '$.tags'))` for exact tag matching
9. FUTURE enrichment types (embeddings, sentiment, key_quotes, etc.) SHALL be addable as new keys in the `ai_enrichment` JSON without requiring schema migrations

### Requirement 7: On-Demand Enrichment

**User Story:** As a JARVIS user, I want to manually trigger AI enrichment on any gem — whether it was saved before IntelligenceKit was available, or I want to re-enrich with updated results — so that my entire knowledge collection can benefit from AI tags and summaries.

#### Acceptance Criteria

1. THE System SHALL expose an `enrich_gem` Tauri command that accepts a gem `id` and runs the enrichment pipeline on that gem
2. THE command SHALL fetch the gem by ID, run `generate_tags` and `summarize` via the IntelProvider, and update the gem's `ai_enrichment` field in the database
3. WHEN the gem already has `ai_enrichment`, THE command SHALL overwrite it with fresh results (re-enrichment)
4. WHEN the IntelProvider is unavailable, THE command SHALL return a descriptive error (not silently succeed)
5. THE command SHALL return the updated Gem with the new `ai_enrichment` to the frontend
6. THE command SHALL work on any gem regardless of when it was saved — this enables enriching the backlog of pre-existing gems

### Requirement 8: Availability Tauri Command

**User Story:** As the frontend, I want to query whether AI enrichment is available, so that I can show the appropriate UI state.

#### Acceptance Criteria

1. THE System SHALL expose a `check_intel_availability` Tauri command that checks the IntelProvider's availability
2. THE command SHALL return `{ available: bool, reason?: string }` matching the IntelProvider trait's check_availability result
3. THE frontend SHALL call this command on app startup and cache the result
4. THE command SHALL be lightweight and fast (no model invocation — just checks cached state from the IntelProvider)

### Requirement 9: Frontend - Tags, Summary Display, and Enrich Button

**User Story:** As a JARVIS user, I want to see AI-generated tags and summaries on my Gems, and be able to trigger enrichment on any gem, so that I can quickly scan my knowledge collection and enrich older gems on demand.

#### Acceptance Criteria

1. THE GemsPanel gem cards SHALL display tags as small badges/chips below the title (if tags are present)
2. THE GemsPanel gem cards SHALL display the summary below the tags (if summary is present), visually distinct from the content preview
3. WHEN tags are not present (unenriched gem), THE card SHALL NOT show a tags section (no empty state, no placeholder)
4. WHEN summary is not present (unenriched gem), THE card SHALL fall back to displaying the existing content_preview
5. THE GistCard in the Browser Tool SHALL show an "AI enrichment available" indicator when the IntelProvider is available, so the user knows tags/summary will be generated on save
6. THE GemsPanel SHALL support filtering by tag: clicking a tag badge SHALL filter the list to gems sharing that tag
7. EACH gem card in the GemsPanel list SHALL display an "Enrich" button (sparkle/wand icon) when AI is available and the gem has no `ai_enrichment` (NULL), enabling on-demand enrichment of unenriched gems
8. EACH gem card SHALL display a "Re-enrich" button (refresh icon) when AI is available and the gem already has `ai_enrichment`, enabling re-enrichment with fresh results
9. THE gem detail view SHALL display a prominent "Enrich with AI" button when AI is available, triggering the `enrich_gem` command
10. WHEN the user clicks Enrich/Re-enrich, THE button SHALL show a loading spinner, call the `enrich_gem` Tauri command, and update the card in-place with the new tags and summary on success
11. WHEN enrichment fails, THE System SHALL show an error toast with the failure reason — the gem remains unchanged

### Requirement 10: Frontend - Enrichment Status

**User Story:** As a JARVIS user, I want to know whether AI enrichment is active, so that I understand why some gems have tags and others don't.

#### Acceptance Criteria

1. THE GemsPanel header SHALL display an enrichment status indicator: a small badge showing "AI" (green) when available or "AI" (gray) when unavailable
2. WHEN the user hovers over the AI badge, THE System SHALL show a tooltip explaining the status (e.g., "Apple Intelligence active — gems will be enriched with tags and summary" or "Apple Intelligence unavailable — gems saved without enrichment")
3. THE status SHALL be determined by calling `check_intel_availability` on panel mount and caching the result

### Requirement 11: Existing Functionality Preservation

**User Story:** As a developer, I want the IntelligenceKit integration to be additive and not break any existing features.

#### Acceptance Criteria

1. THE existing `save_gem` command SHALL continue to work when IntelProvider is unavailable — saving without enrichment is identical to current behavior
2. THE existing `list_gems` and `search_gems` commands SHALL continue to work — the new `ai_enrichment` column is nullable and backwards-compatible
3. THE existing `prepare_tab_gist` command SHALL remain unchanged — extraction and enrichment are separate concerns
4. THE existing `delete_gem` command SHALL continue to work unchanged
5. THE JarvisListen sidecar SHALL NOT be affected by IntelligenceKit integration — both sidecars run independently
6. THE recording, transcription, and browser observation pipelines SHALL NOT be affected
7. ALL existing tests SHALL continue to pass
8. THE app SHALL start successfully even if the IntelligenceKit binary is not present (graceful degradation)
