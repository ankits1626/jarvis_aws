# Searchable Gems — SearchResultProvider Trait + QMD Semantic Search

## Introduction

Jarvis has keyword-based gem search via SQLite FTS5, but it only matches exact words. Searching for "database migration" won't find a gem titled "RDS Transition Strategy" even though they're about the same topic. Users need semantic search — finding gems by meaning, not just keywords.

This spec defines a **`SearchResultProvider` trait** — a backend-agnostic interface that any search backend must implement. Tauri commands call the trait and receive results in a standard format (`Vec<SearchResult>`). The trait consumer never knows which backend is active.

Two implementations ship:

1. **`FtsResultProvider`** (default) — wraps the existing `GemStore::search()` FTS5 method. Always available, zero setup. Returns `MatchType::Keyword`.
2. **`QmdResultProvider`** (opt-in) — wraps [QMD](https://github.com/tobi/qmd) CLI for semantic search over `gem.md` knowledge files. User enables it in Settings, Jarvis installs everything automatically. Returns `MatchType::Hybrid` (QMD combines BM25 + vector embeddings + LLM reranker).

Adding a future search backend (Qdrant, Ollama embeddings, etc.) means implementing the trait — zero changes to commands or frontend.

**Reference:** Design doc at `discussion/29-feb-next-step/searchable-gems-via-qmd.md`. Depends on: [Gem Knowledge Files spec](../gem-knowledge-files/requirements.md) (fully implemented — knowledge files exist at `knowledge/{gem_id}/gem.md`).

## Glossary

- **SearchResultProvider**: The backend-agnostic trait defining the contract for search operations. Analogous to `IntelProvider`, `KnowledgeStore`, `GemStore`. Any search backend must implement this trait.
- **SearchResult**: The standard return type from `SearchResultProvider::search()` — contains `gem_id`, `score` (0.0–1.0 normalized), `matched_chunk` (snippet), and `match_type`.
- **MatchType**: Enum indicating how a result was matched — `Keyword` (FTS5/BM25), `Semantic` (vector similarity), or `Hybrid` (combined).
- **FtsResultProvider**: The default implementation wrapping the existing `GemStore::search()` FTS5 method. Always available.
- **QmdResultProvider**: The opt-in implementation wrapping QMD CLI. Requires Node.js 22+, Homebrew SQLite, and ~1.9GB of search models.
- **QMD**: [Query Markdown](https://github.com/tobi/qmd) — a local-first CLI search engine by Tobi Lutke that combines BM25 keyword matching, vector embeddings, and LLM-based reranking over markdown files.
- **Collection**: A QMD concept — a directory of markdown files registered for indexing. Jarvis creates a `jarvis-gems` collection pointing at the `knowledge/` directory.
- **Semantic Search**: Finding content by meaning rather than exact keyword matches. "container orchestration" finds "ECS vs EKS" because QMD understands they're related concepts.
- **GemSearchResult**: The enriched result type returned to the frontend — `SearchResult` joined with gem metadata (title, source_type, source_url, etc.) from the database.

## Frozen Design Decisions

These decisions were made during design review (2026-03-01):

1. **Single trait, not two.** `SearchResultProvider` is the only abstraction. Tauri commands call it directly. No intermediate "SearchManager" or "SearchRouter" layer.
2. **Provider is selected in Settings, not per-search.** When semantic search is enabled, all searches go through QMD. When disabled, all searches go through FTS5. No search mode toggle in the search bar.
3. **FTS5 is always the fallback.** If QMD breaks, is disabled, or fails availability check, search falls back to FTS5. The user always has working search.
4. **Automated setup.** When user enables semantic search in Settings, Jarvis checks prerequisites (Node.js, SQLite), installs QMD, creates collection, downloads models (~1.9GB), and indexes — all automatically. Same UX pattern as MLX venv setup.
5. **Providers fulfill a contract.** The trait defines _what_ results look like (`Vec<SearchResult>` with normalized scores). Each provider decides _how_ to get them. Commands never know which backend is active.
6. **Provider doesn't join metadata.** The provider returns `SearchResult` (gem_id + score + chunk). The Tauri command joins with gem metadata from `GemStore` to produce `GemSearchResult` for the frontend.
7. **App restart after provider switch.** Enabling/disabling semantic search in Settings requires an app restart. No hot-swap of `Arc<dyn SearchResultProvider>` at runtime in v1.
8. **QMD CLI mode.** `QmdResultProvider` shells out to the `qmd` CLI binary. No QMD MCP HTTP server in v1.
9. **Fire-and-forget indexing.** `index_gem()` and `remove_gem()` for QMD are fire-and-forget (spawn process, don't await). They don't block the UI.
10. **Existing search_gems command replaced.** The current `search_gems` Tauri command in `commands.rs` (which calls `gem_store.search()` directly) is replaced with the trait-based version in `search/commands.rs`.

---

## Requirement 1: SearchResultProvider Trait and Data Types

**User Story:** As a developer, I need a backend-agnostic trait defining the contract for search operations, so all consumers (Tauri commands, lifecycle hooks) interact through a stable interface that can be swapped without code changes.

### Acceptance Criteria

1. THE System SHALL define a `SearchResultProvider` trait in `src/search/provider.rs` with the `#[async_trait]` attribute and `Send + Sync` bounds
2. THE trait SHALL define the following methods:
   - **Search:** `search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, String>`
   - **Index:** `index_gem(&self, gem_id: &str) -> Result<(), String>`
   - **Remove:** `remove_gem(&self, gem_id: &str) -> Result<(), String>`
   - **Rebuild:** `reindex_all(&self) -> Result<usize, String>`
   - **Availability:** `check_availability(&self) -> AvailabilityResult`
3. THE System SHALL define a `SearchResult` struct with fields: `gem_id` (String), `score` (f64 — 0.0 to 1.0 normalized), `matched_chunk` (String — snippet that matched, empty if provider doesn't support snippets), `match_type` (MatchType)
4. THE System SHALL define a `MatchType` enum with variants: `Keyword`, `Semantic`, `Hybrid`
5. THE `SearchResultProvider` trait SHALL reuse `AvailabilityResult` from `src/intelligence/provider.rs`
6. ALL structs and enums SHALL derive `Debug`, `Clone`, `Serialize`, `Deserialize`
7. THE trait doc comment SHALL state: "Backend-agnostic search result provider. Tauri commands call this trait, never a concrete implementation. Each provider fulfills the contract — returns results in the standard format."

---

## Requirement 2: FtsResultProvider Implementation

**User Story:** As the Jarvis system, I need a default search provider that wraps the existing FTS5 keyword search, so gem search works out of the box without any setup.

### Acceptance Criteria

1. THE System SHALL implement `FtsResultProvider` struct in `src/search/fts_provider.rs` with field: `gem_store` (Arc<dyn GemStore>)
2. THE `check_availability()` method SHALL always return `AvailabilityResult { available: true, reason: None }` — FTS5 is always available
3. THE `search()` method SHALL call `self.gem_store.search(query, limit).await` and convert each `GemPreview` to a `SearchResult` with:
   - `gem_id`: from `GemPreview.id`
   - `score`: derived from rank order — `1.0 - (index as f64 * 0.05)`, clamped to minimum 0.0
   - `matched_chunk`: empty string (FTS5 doesn't provide snippets)
   - `match_type`: `MatchType::Keyword`
4. THE `index_gem()` method SHALL return `Ok(())` — FTS5 triggers handle indexing automatically
5. THE `remove_gem()` method SHALL return `Ok(())` — FTS5 triggers handle deletion automatically
6. THE `reindex_all()` method SHALL return `Ok(0)` — FTS5 index is maintained by SQLite triggers
7. THE `FtsResultProvider` SHALL implement `SearchResultProvider` via `#[async_trait]`

---

## Requirement 3: QmdResultProvider Implementation

**User Story:** As a user who has enabled semantic search, I need a provider that calls QMD CLI to search my gem knowledge files by meaning, so I can find gems that are semantically related to my query even without exact keyword matches.

### Acceptance Criteria

1. THE System SHALL implement `QmdResultProvider` struct in `src/search/qmd_provider.rs` with fields: `qmd_path` (PathBuf — path to qmd binary), `knowledge_path` (PathBuf — root knowledge directory)
2. THE `check_availability()` method SHALL:
   a. Check that the `qmd_path` binary exists and is executable
   b. Run `qmd --version` and verify it succeeds
   c. Run `qmd status --json` and verify the `jarvis-gems` collection exists and has indexed documents
   d. Return `AvailabilityResult { available: true, reason: None }` if all checks pass
   e. Return `AvailabilityResult { available: false, reason: Some(description) }` with a specific reason if any check fails
3. THE `search()` method SHALL:
   a. Run `qmd query "{query}" --json -n {limit}` via `tokio::process::Command`
   b. Parse the JSON output from stdout
   c. Extract `gem_id` from each result's file path (`knowledge/{gem_id}/gem.md` → `gem_id`)
   d. Normalize scores to 0.0–1.0 range
   e. Return `Vec<SearchResult>` with `match_type: MatchType::Hybrid`
   f. If QMD command fails, return `Err` with the stderr output
4. THE `index_gem()` method SHALL spawn `qmd update && qmd embed` as a fire-and-forget background process (spawn, don't await the result)
5. THE `remove_gem()` method SHALL spawn `qmd update` as a fire-and-forget background process
6. THE `reindex_all()` method SHALL run `qmd update && qmd embed -f` (force re-embed all) and await completion, returning the number of documents indexed
7. ALL QMD CLI commands SHALL use the `qmd_path` field for the binary path, not assume `qmd` is in `$PATH`
8. THE `QmdResultProvider` SHALL handle paths with spaces correctly (macOS `Application Support` directory)

---

## Requirement 4: Tauri Commands

**User Story:** As the frontend, I need Tauri commands that delegate to the active search provider, so the UI can search gems, check availability, trigger setup, and rebuild the index.

### Acceptance Criteria

1. THE System SHALL expose a `search_gems` Tauri command in `src/search/commands.rs` that:
   a. Accepts parameters: `query: String`, `limit: Option<usize>`
   b. Calls `provider.search(&query, limit.unwrap_or(20)).await`
   c. Joins each `SearchResult` with gem metadata from `GemStore` (title, source_type, source_url, captured_at, description, ai_enrichment)
   d. Returns `Result<Vec<GemSearchResult>, String>` where `GemSearchResult` contains both search metadata (score, matched_chunk, match_type) and gem metadata
   e. Uses `State<'_, Arc<dyn SearchResultProvider>>` for dependency injection
2. THE System SHALL expose a `check_search_availability` Tauri command that returns `AvailabilityResult` by calling `provider.check_availability().await`
3. THE System SHALL expose a `setup_semantic_search` Tauri command that runs the automated QMD setup flow (Requirement 6) and returns `Result<QmdSetupResult, String>`
4. THE System SHALL expose a `rebuild_search_index` Tauri command that calls `provider.reindex_all().await` and returns `Result<usize, String>`
5. THE System SHALL define a `GemSearchResult` struct with fields from both `SearchResult` (score, matched_chunk, match_type) and gem metadata (id, title, source_type, source_url, captured_at, description, tags)
6. THE existing `search_gems` command in `src/commands.rs` SHALL be removed and replaced by the new one in `src/search/commands.rs`
7. ALL new commands SHALL be registered in `lib.rs` in the `generate_handler!` macro

---

## Requirement 5: Provider Registration and Selection

**User Story:** As the Jarvis system, I need to select and register the correct search provider on app startup based on user settings, so the right backend is active when the user searches.

### Acceptance Criteria

1. THE System SHALL register the active `SearchResultProvider` as Tauri managed state: `app.manage(Arc<dyn SearchResultProvider>)` in `lib.rs`
2. ON app startup, THE System SHALL check `settings.semantic_search_enabled` (boolean field in Settings)
3. IF `semantic_search_enabled` is `false` or not set, THE System SHALL register `FtsResultProvider` as the active provider
4. IF `semantic_search_enabled` is `true`, THE System SHALL attempt to create a `QmdResultProvider`:
   a. Locate the `qmd` binary (check common paths: `/opt/homebrew/bin/qmd`, `/usr/local/bin/qmd`, result of `which qmd`)
   b. If found and `check_availability()` returns `available: true`, register `QmdResultProvider`
   c. If not found or not available, fall back to `FtsResultProvider` and log a warning
5. THE Settings struct SHALL be extended with a `semantic_search_enabled: Option<bool>` field (default: `None`, treated as `false`)
6. THE provider selection logic SHALL follow the same pattern as `IntelProvider` selection in `intelligence/mod.rs`

---

## Requirement 6: Automated QMD Setup Flow

**User Story:** As a user enabling semantic search for the first time, I want Jarvis to automatically install QMD and its dependencies, create a collection, download models, and index my gems — so I don't have to run any terminal commands myself.

### Acceptance Criteria

1. THE `setup_semantic_search` command SHALL execute the following steps in order:
   - **Step 1: Check Node.js** — run `node --version`, verify >= 22. If missing or old, return error with download link `https://nodejs.org/`
   - **Step 2: Check/Install SQLite** — run `brew list sqlite`. If missing, run `brew install sqlite` and await completion
   - **Step 3: Install QMD** — run `qmd --version`. If missing, run `npm install -g @tobilu/qmd` and await completion
   - **Step 4: Create collection** — run `qmd collection list`. If `jarvis-gems` not found, run `qmd collection add {knowledge_path} --name jarvis-gems --mask "**/*.md"`
   - **Step 5: Index & embed** — run `qmd update` then `qmd embed`. Emit progress events to frontend
   - **Step 6: Save setting** — set `semantic_search_enabled: true` in settings file
2. THE command SHALL return `QmdSetupResult` with fields: `success` (bool), `node_version` (String), `qmd_version` (String), `docs_indexed` (usize), `error` (Option<String>)
3. IF any step fails, THE command SHALL stop, return `success: false` with a descriptive `error`, and NOT change the settings
4. THE command SHALL emit Tauri events on the `"semantic-search-setup"` channel with step progress: `{ step: number, total: 6, description: String, status: "running" | "done" | "failed" }`
5. ALL shell commands SHALL use `tokio::process::Command` with proper error handling (capture stderr on failure)
6. THE `npm install -g @tobilu/qmd` command SHALL use the Node.js path discovered in Step 1 to locate `npm`
7. THE first `qmd embed` run SHALL download ~1.9GB of models to `~/.cache/qmd/models/`. The setup SHALL warn the user about this download size before starting

---

## Requirement 7: Lifecycle Integration

**User Story:** As the Jarvis system, I need the search index to stay in sync with gem changes, so new, updated, and deleted gems are reflected in search results without manual intervention.

### Acceptance Criteria

1. WHEN a gem is saved via `GemStore::save()`, THE System SHALL call `search_provider.index_gem(gem_id).await` after the save completes
2. WHEN a gem is enriched (tags/summary updated), THE System SHALL call `search_provider.index_gem(gem_id).await` after enrichment completes
3. WHEN a gem's transcript is generated, THE System SHALL call `search_provider.index_gem(gem_id).await` after the transcript is saved
4. WHEN a gem is deleted via `GemStore::delete()`, THE System SHALL call `search_provider.remove_gem(gem_id).await` after deletion completes
5. ON app launch, IF semantic search is enabled, THE System SHALL call `search_provider.reindex_all().await` in the background (fire-and-forget) to catch up on any changes made while the app was closed
6. FAILURES in search indexing SHALL be logged but SHALL NOT block or fail the primary operation (e.g., a failed `index_gem()` does not roll back `GemStore::save()`)
7. ALL lifecycle calls SHALL use the injected `dyn SearchResultProvider` — never a concrete type

---

## Requirement 8: Settings UI — Semantic Search Section

**User Story:** As a user, I want a section in Settings to enable/disable semantic search, see its status, and rebuild the index — so I can control whether Jarvis uses semantic or keyword search.

### Acceptance Criteria

1. THE Settings panel SHALL include a "Semantic Search" section, positioned after the existing MLX/AI sections
2. WHEN semantic search is not configured, THE section SHALL show:
   - Status indicator: "Not configured"
   - Description: "Semantic search finds gems by meaning, not just keywords. Powered by QMD (local, on-device). Requires Node.js 22+ and ~2GB for search models."
   - An "Enable Semantic Search" button
3. WHEN the "Enable" button is clicked, THE frontend SHALL call `invoke('setup_semantic_search')` and show step-by-step progress:
   - Each step shows a label and status (spinner → checkmark → error)
   - If a step fails, show the error message and a "Retry" option
4. WHEN semantic search is configured and active, THE section SHALL show:
   - Status indicator: "Ready" with gem count (e.g., "Ready (26 gems indexed)")
   - Provider info: "QMD v{version}"
   - Model cache info: "~/.cache/qmd/models/ ({size})"
   - A "Rebuild Index" button that calls `invoke('rebuild_search_index')`
   - A "Disable" button that sets `semantic_search_enabled: false` in settings and shows a restart prompt
5. THE "Disable" action SHALL NOT uninstall QMD or delete models — it only changes the setting. QMD stays installed for re-enabling later
6. AFTER enabling or disabling, THE frontend SHALL show a message: "Restart Jarvis to apply changes"

---

## Requirement 9: Frontend Search Integration

**User Story:** As a user searching gems, I want the search bar to work the same regardless of which provider is active, with optional relevance scores shown when semantic search is enabled.

### Acceptance Criteria

1. THE `GemsPanel.tsx` search bar SHALL call `invoke('search_gems', { query, limit })` — the same command regardless of active provider
2. THE frontend SHALL define a `GemSearchResult` TypeScript interface matching the Rust struct from Requirement 4
3. WHEN search results include `match_type` of `Semantic` or `Hybrid`, THE gem cards SHALL display a relevance score badge (e.g., "87%") derived from `score * 100`
4. WHEN search results include `match_type` of `Keyword`, NO score badge SHALL be shown (current behavior preserved)
5. THE search debounce (300ms) SHALL remain unchanged
6. THE `GemPreview` type used by list/filter SHALL be compatible with `GemSearchResult` — the gems panel should handle both types for rendering
7. IF `search_gems` returns an error, THE panel SHALL show the error message (existing error handling pattern)

---

## Requirement 10: Module Structure and Registration

**User Story:** As a developer, I need the search module to follow Jarvis's existing module patterns, so the codebase remains consistent and the search provider is properly registered.

### Acceptance Criteria

1. THE System SHALL create a `src/search/` module with the following files:
   - `mod.rs` — module root, re-exports public types
   - `provider.rs` — `SearchResultProvider` trait, `SearchResult`, `MatchType`, `GemSearchResult`
   - `fts_provider.rs` — `FtsResultProvider` implementation
   - `qmd_provider.rs` — `QmdResultProvider` implementation
   - `commands.rs` — Tauri command handlers (`search_gems`, `check_search_availability`, `setup_semantic_search`, `rebuild_search_index`)
2. THE `mod.rs` SHALL re-export: `SearchResultProvider`, `SearchResult`, `MatchType`, `GemSearchResult`, `FtsResultProvider`, `QmdResultProvider`
3. THE search module SHALL be added to `src/lib.rs` module declarations
4. ALL new Tauri commands SHALL be registered in `lib.rs` in the `generate_handler!` macro
5. THE existing `search_gems` command in `src/commands.rs` SHALL be removed (replaced by the new one in `search/commands.rs`)
6. THE `Cargo.toml` SHALL NOT add any new dependencies — the search module uses `tokio::process::Command` for QMD CLI interaction, which is already available

---

## Technical Constraints

1. **Rust + Tauri 2.x**: All backend code is Rust. Search provider operations are async (tokio). Tauri commands bridge to the frontend.
2. **Trait-based architecture**: Consumers use `dyn SearchResultProvider` — never a concrete type. This is consistent with `GemStore`, `IntelProvider`, `KnowledgeStore`.
3. **Existing FTS5 stays untouched**: `SqliteGemStore::search()` and the `gems_fts` virtual table are not modified. `FtsResultProvider` wraps them.
4. **QMD is a CLI sidecar**: `QmdResultProvider` calls the `qmd` binary via `tokio::process::Command`. No QMD library dependency, no MCP HTTP mode.
5. **Knowledge files as corpus**: QMD indexes the `gem.md` files in `knowledge/{gem_id}/`. The knowledge layer (gem-knowledge-files spec) must be complete and functioning.
6. **No new Rust crate dependencies**: Uses existing `tokio`, `serde`, `serde_json`, `async-trait`. QMD interaction is via process spawning.
7. **Fire-and-forget indexing**: `index_gem()` and `remove_gem()` in `QmdResultProvider` spawn processes without awaiting. Only `reindex_all()` awaits completion.
8. **Score normalization**: All providers must return scores in the 0.0–1.0 range. QMD scores may need normalization from whatever range QMD returns.
9. **macOS paths with spaces**: `Application Support` has a space. All paths passed to QMD CLI must be properly quoted.
10. **App restart for provider switch**: Enabling/disabling semantic search requires restart. No runtime hot-swap of the managed `Arc<dyn SearchResultProvider>`.

## Out of Scope

1. QMD MCP HTTP server mode — CLI mode is sufficient for v1
2. `find_similar(gem_id)` — "related gems" feature is a future enhancement
3. Search within project scope — Projects feature is a separate spec
4. Hybrid score weighting configuration — use QMD's defaults
5. Hot-swapping provider at runtime — requires app restart
6. Multiple providers active simultaneously — only one provider is active at a time
7. Custom embedding models — QMD manages its own models
8. Windows/Linux support — QMD setup flow is macOS-specific (Homebrew)
9. QMD model management UI — models are auto-managed in `~/.cache/qmd/models/`
10. Rendered markdown in search results — matched_chunk is plain text
