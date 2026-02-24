# Requirements Document

## Introduction

The JARVIS Gems module adds a persistence and retrieval layer for extracted knowledge. Currently, when a user extracts a gist from a browser tab (YouTube, Medium, Gmail, ChatGPT, or any page), the result is ephemeral — displayed once and lost unless manually exported to a flat .md file. Gems transforms these extractions into persistent, searchable knowledge units called **Gems**. The storage layer is backend-agnostic: a `GemStore` trait defines the interface (save, list, search, delete), and implementations can be swapped or composed. The default implementation uses local SQLite with FTS5 full-text search (`~/.jarvis/gems.db`), but the architecture supports plugging in remote APIs, other databases, or composites (e.g., SQLite + API sync) without changing the commands or frontend. This module lays the foundation for a future intelligence layer where agents can query the user's accumulated knowledge.

## Glossary

- **Gem**: A persistent knowledge unit extracted from a browser source. Contains structured metadata (title, author, source type, domain) plus the full extracted content.
- **GemStore**: A Rust trait defining the storage interface for gems. Operations: save, list, search, delete, get. Implementations are swappable.
- **SqliteGemStore**: The default GemStore implementation using a local SQLite database at `~/.jarvis/gems.db` with FTS5 full-text search.
- **PageGist**: The existing in-memory struct returned by extractors (YouTube, Medium, Gmail, ChatGPT, generic). A PageGist becomes a Gem when the user saves it.
- **FTS5**: SQLite's full-text search extension. Enables fast keyword search across gem titles, descriptions, and content. Used by SqliteGemStore.
- **GemsPanel**: A new frontend component that displays saved gems in a scrollable list with search, filtering, and delete capabilities.
- **SourceType**: The existing enum classifying browser tabs (YouTube, Article, Code, Docs, Email, Chat, QA, News, Research, Social, Other).

## Requirements

### Requirement 1: Storage Trait Abstraction

**User Story:** As a developer, I want the gem storage interface to be decoupled from any specific backend, so that I can swap SQLite for an API, another database, or a composite of multiple sources without changing commands or frontend.

#### Acceptance Criteria

1. THE System SHALL define a `GemStore` trait with async methods: `save`, `get`, `list`, `search`, `delete`
2. THE trait SHALL operate on a `Gem` struct that is backend-agnostic (no SQLite-specific types)
3. THE Tauri commands SHALL depend on the `GemStore` trait (via trait object or generic), NOT on any concrete implementation
4. THE System SHALL allow the GemStore implementation to be chosen at app startup (configured in `setup()`)
5. FUTURE implementations (e.g., `ApiGemStore`, `CompositeGemStore`) SHALL be addable by implementing the trait without modifying commands or frontend
6. THE `Gem` struct SHALL contain: `id` (String, UUID), `source_type` (String), `source_url` (String), `domain` (String), `title` (String), `author` (Option<String>), `description` (Option<String>), `content` (Option<String>), `source_meta` (serde_json::Value), `captured_at` (String, ISO 8601)

### Requirement 2: SQLite Implementation (Default)

**User Story:** As a JARVIS user, I want my gems stored in a reliable local database that works offline and initializes automatically.

#### Acceptance Criteria

1. THE System SHALL provide a `SqliteGemStore` struct implementing the `GemStore` trait
2. WHEN the application starts, THE SqliteGemStore SHALL initialize a SQLite database at `~/.jarvis/gems.db`
3. THE SqliteGemStore SHALL create the `gems` table if it does not exist, with columns matching the `Gem` struct fields: `id` (TEXT PRIMARY KEY), `source_type` (TEXT), `source_url` (TEXT UNIQUE), `domain` (TEXT), `title` (TEXT), `author` (TEXT nullable), `description` (TEXT nullable), `content` (TEXT nullable), `source_meta` (TEXT, JSON string), `captured_at` (TEXT, ISO 8601)
4. THE SqliteGemStore SHALL create an FTS5 virtual table `gems_fts` indexed on `title`, `description`, and `content` columns for full-text search
5. THE SqliteGemStore SHALL be the default implementation used at app startup
6. WHEN the database file is corrupted or inaccessible, THE SqliteGemStore SHALL return a clear error message

### Requirement 3: Save Gem

**User Story:** As a JARVIS user, I want to save an extracted gist as a Gem with one click, so that I can build my personal knowledge collection.

#### Acceptance Criteria

1. THE System SHALL expose a `save_gem` Tauri command that accepts a PageGist and persists it via the GemStore trait
2. THE System SHALL generate a UUID v4 as the gem's `id`
3. THE System SHALL record the current timestamp as `captured_at` in ISO 8601 format
4. THE System SHALL map PageGist fields to Gem fields: `title`, `author`, `description` from PageGist; `content` from `content_excerpt`; `source_meta` from `extra` (serialized as JSON)
5. WHEN a gem with the same `source_url` already exists, THE System SHALL update the existing record (upsert) rather than creating a duplicate
6. THE `save_gem` command SHALL return the saved Gem (including its `id` and `captured_at`) to the frontend
7. WHEN the save fails, THE System SHALL return a descriptive error message

### Requirement 4: List Gems

**User Story:** As a JARVIS user, I want to browse all my saved gems sorted by most recent, so that I can revisit knowledge I've captured.

#### Acceptance Criteria

1. THE System SHALL expose a `list_gems` Tauri command that returns all gems ordered by `captured_at` descending
2. THE command SHALL accept an optional `limit` parameter (default: 50) to paginate results
3. THE command SHALL accept an optional `offset` parameter (default: 0) for pagination
4. EACH returned Gem SHALL include all stored fields
5. THE `content` field in list results SHALL be truncated to 200 characters with a `content_preview` field to keep responses lightweight

### Requirement 5: Search Gems

**User Story:** As a JARVIS user, I want to search my gems by keyword, so that I can quickly find relevant knowledge.

#### Acceptance Criteria

1. THE System SHALL expose a `search_gems` Tauri command that accepts a `query` string
2. THE search SHALL match against `title`, `description`, and `content` fields
3. THE search results SHALL be ranked by relevance
4. THE search results SHALL include the same fields as `list_gems`, with `content` truncated to 200 characters
5. WHEN the query is empty, THE command SHALL return the same results as `list_gems`
6. THE search SHALL support basic keyword queries (e.g., "OAuth token", "Rust async")

### Requirement 6: Delete Gem

**User Story:** As a JARVIS user, I want to delete gems I no longer need, so that I can keep my knowledge collection clean.

#### Acceptance Criteria

1. THE System SHALL expose a `delete_gem` Tauri command that accepts a gem `id`
2. THE command SHALL delete the gem via the GemStore trait
3. WHEN the gem ID does not exist, THE command SHALL return a descriptive error
4. THE command SHALL return success confirmation to the frontend

### Requirement 7: Save Gem Button in Browser Tool

**User Story:** As a JARVIS user, I want a "Save Gem" button on every extracted gist, so that I can save knowledge directly from the extraction flow.

#### Acceptance Criteria

1. THE GistCard component SHALL display a "Save Gem" button alongside the existing "Copy" and "Export" buttons
2. WHEN the user clicks "Save Gem", THE frontend SHALL call the `save_gem` Tauri command with the current PageGist data
3. WHEN the save succeeds, THE button SHALL change to "Saved" (disabled state) to indicate the gem was persisted
4. WHEN the save fails, THE frontend SHALL display the error message below the gist card
5. THE "Save Gem" button SHALL replace the existing "Export" button (flat .md export is superseded by database storage)

### Requirement 8: Gems Panel

**User Story:** As a JARVIS user, I want a dedicated panel to browse and search my saved gems, so that I can access my knowledge collection at any time.

#### Acceptance Criteria

1. THE application SHALL provide a GemsPanel component accessible via a "Gems" button in the main navigation (hamburger menu or toolbar)
2. THE GemsPanel SHALL display a search input at the top
3. THE GemsPanel SHALL display saved gems in a scrollable list, most recent first
4. EACH gem card SHALL display: source type badge, title, domain, author (if present), description (if present), content preview (first 200 chars), and captured date
5. EACH gem card SHALL have a "Delete" button that removes the gem after confirmation
6. WHEN the user types in the search input, THE panel SHALL call `search_gems` and update the list (debounced at 300ms)
7. WHEN there are no gems saved, THE panel SHALL display an empty state: "No gems yet. Extract a gist from the Browser tool and save it."
8. THE GemsPanel SHALL load gems on mount via `list_gems`

### Requirement 9: Existing Functionality Preservation

**User Story:** As a developer, I want the Gems module to integrate cleanly without breaking existing features.

#### Acceptance Criteria

1. THE existing `prepare_tab_gist` command SHALL remain unchanged — extraction and persistence are separate concerns
2. THE existing `export_gist` command SHALL remain available as a secondary option
3. THE PageGist struct SHALL NOT be modified — the Gem maps from PageGist at the command layer
4. THE Browser Tool tab listing, classification, and badge system SHALL NOT be affected
5. THE recording and transcription pipelines SHALL NOT be affected
6. ALL existing tests SHALL continue to pass
