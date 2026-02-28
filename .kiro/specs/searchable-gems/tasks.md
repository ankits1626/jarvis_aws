# Searchable Gems — Implementation Tasks

## Phase 1: Core Types and Trait Definition

### Task 1: Create `src/search/provider.rs` — Trait and Data Types

- [ ] 1.1 Create `src-tauri/src/search/` directory
- [ ] 1.2 Create `provider.rs` with the `MatchType` enum (`Keyword`, `Semantic`, `Hybrid`), deriving `Debug`, `Clone`, `Serialize`, `Deserialize`
- [ ] 1.3 Add `SearchResult` struct: `gem_id: String`, `score: f64`, `matched_chunk: String`, `match_type: MatchType`
- [ ] 1.4 Add `GemSearchResult` struct combining search metadata (score, matched_chunk, match_type) with gem metadata (id, source_type, source_url, domain, title, author, description, captured_at, tags, summary)
- [ ] 1.5 Add `QmdSetupResult` struct: `success: bool`, `node_version: Option<String>`, `qmd_version: Option<String>`, `docs_indexed: Option<usize>`, `error: Option<String>`
- [ ] 1.6 Add `SetupProgressEvent` struct: `step: usize`, `total: usize`, `description: String`, `status: String`
- [ ] 1.7 Define the `SearchResultProvider` trait with `#[async_trait]` and `Send + Sync` bounds:
  - `check_availability(&self) -> AvailabilityResult`
  - `search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, String>`
  - `index_gem(&self, gem_id: &str) -> Result<(), String>`
  - `remove_gem(&self, gem_id: &str) -> Result<(), String>`
  - `reindex_all(&self) -> Result<usize, String>`
- [ ] 1.8 Import `AvailabilityResult` from `crate::intelligence::provider` — reuse, don't redefine
- [ ] 1.9 Add doc comments on the trait stating: "Backend-agnostic search result provider. Tauri commands call this trait, never a concrete implementation."

_Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7_

### Task 2: Create `src/search/mod.rs` — Module Root

- [ ] 2.1 Create `mod.rs` with `pub mod provider; pub mod fts_provider; pub mod qmd_provider; pub mod commands;`
- [ ] 2.2 Re-export public types: `SearchResultProvider`, `SearchResult`, `MatchType`, `GemSearchResult`, `QmdSetupResult`, `SetupProgressEvent`, `FtsResultProvider`, `QmdResultProvider`
- [ ] 2.3 Add `pub mod search;` to `src-tauri/src/lib.rs` module declarations

_Requirements: 10.1, 10.2, 10.3_

**Checkpoint**: `cargo check` passes. All types defined, no implementations yet.

---

## Phase 2: FtsResultProvider (Default Provider)

### Task 3: Create `src/search/fts_provider.rs`

- [ ] 3.1 Define `FtsResultProvider` struct with field `gem_store: Arc<dyn GemStore>`
- [ ] 3.2 Add `FtsResultProvider::new(gem_store: Arc<dyn GemStore>) -> Self` constructor
- [ ] 3.3 Implement `SearchResultProvider for FtsResultProvider`:
  - `check_availability()` → always returns `AvailabilityResult { available: true, reason: None }`
  - `search()` → calls `self.gem_store.search(query, limit).await`, maps each `GemPreview` to `SearchResult` with score derived from rank order `(1.0 - (i as f64 * 0.05)).max(0.0)`, empty `matched_chunk`, `MatchType::Keyword`
  - `index_gem()` → `Ok(())` (no-op, FTS5 triggers handle it)
  - `remove_gem()` → `Ok(())` (no-op)
  - `reindex_all()` → `Ok(0)` (FTS5 maintained by triggers)
- [ ] 3.4 Verify `FtsResultProvider` compiles with existing `GemStore` trait import

_Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7_

**Checkpoint**: `cargo check` passes. `FtsResultProvider` wraps `GemStore::search()`.

---

## Phase 3: QmdResultProvider (Semantic Search)

### Task 4: Create `src/search/qmd_provider.rs`

- [ ] 4.1 Define `QmdResultProvider` struct with fields: `qmd_path: PathBuf`, `knowledge_path: PathBuf`
- [ ] 4.2 Add `QmdResultProvider::new(qmd_path: PathBuf, knowledge_path: PathBuf) -> Self`
- [ ] 4.3 Add `QmdResultProvider::find_qmd_binary() -> Option<PathBuf>` static method — checks `/opt/homebrew/bin/qmd`, `/usr/local/bin/qmd`, then `which qmd`
- [ ] 4.4 Add helper fn `extract_gem_id_from_path(file_path: &str, knowledge_path: &PathBuf) -> Option<String>` — extracts gem UUID from `knowledge/{gem_id}/gem.md` path
- [ ] 4.5 Add helper fn `normalize_qmd_score(raw_score: f64) -> f64` — clamps to 0.0–1.0
- [ ] 4.6 Implement `SearchResultProvider for QmdResultProvider`:
  - `check_availability()` → check binary exists, run `qmd --version`, run `qmd status --json`, verify `jarvis-gems` collection exists
  - `search()` → run `qmd query "{query}" --json -n {limit}`, parse JSON, extract gem_ids from paths, normalize scores, return `Vec<SearchResult>` with `MatchType::Hybrid`
  - `index_gem()` → fire-and-forget `tokio::spawn` of `qmd update && qmd embed`
  - `remove_gem()` → fire-and-forget `tokio::spawn` of `qmd update`
  - `reindex_all()` → await `qmd update && qmd embed -f`, return doc count
- [ ] 4.7 All `Command::new()` calls use `self.qmd_path` (not bare `"qmd"`)

_Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8_

**Checkpoint**: `cargo check` passes. Both providers compile. QMD integration is CLI-based only.

---

## Phase 4: Settings Extension

### Task 5: Add `SearchSettings` to Settings struct

- [ ] 5.1 In `src-tauri/src/settings/manager.rs`, add `SearchSettings` struct with `semantic_search_enabled: bool` (default `false`)
- [ ] 5.2 Add `search: SearchSettings` field to `Settings` struct with `#[serde(default)]` for backward compatibility
- [ ] 5.3 Implement `Default for SearchSettings`
- [ ] 5.4 Verify existing settings files without `search` field still deserialize correctly (serde default)

_Requirements: 5.5_

**Checkpoint**: Existing settings deserialization works. New `search` field defaults to `{ semantic_search_enabled: false }`.

---

## Phase 5: Tauri Commands

### Task 6: Create `src/search/commands.rs` — `search_gems` Command

- [ ] 6.1 Implement `search_gems` Tauri command: accepts `query: String`, `limit: Option<usize>`, uses `State<'_, Arc<dyn SearchResultProvider>>` and `State<'_, Arc<dyn GemStore>>`
- [ ] 6.2 Handle empty query: delegate to `gem_store.list(limit, 0).await`, map to `GemSearchResult` with score 1.0 and `MatchType::Keyword`
- [ ] 6.3 For non-empty query: call `provider.search()`, then join each `SearchResult` with gem metadata via `gem_store.get(gem_id)`
- [ ] 6.4 Extract `tags` and `summary` from gem's `ai_enrichment` JSON field
- [ ] 6.5 Silently skip results where gem not found in DB (orphaned index entries)
- [ ] 6.6 Return `Result<Vec<GemSearchResult>, String>`

_Requirements: 4.1, 4.5, 4.6_

### Task 7: Create remaining Tauri commands in `commands.rs`

- [ ] 7.1 Implement `check_search_availability` command: calls `provider.check_availability().await`, returns `Result<AvailabilityResult, String>`
- [ ] 7.2 Implement `rebuild_search_index` command: calls `provider.reindex_all().await`, returns `Result<usize, String>`
- [ ] 7.3 Implement `setup_semantic_search` command: runs the 6-step automated setup flow with progress events emitted on `"semantic-search-setup"` channel
- [ ] 7.4 Implement setup helper functions:
  - `check_node_version()` → verify Node.js >= 22
  - `check_or_install_sqlite()` → brew list/install sqlite
  - `check_or_install_qmd()` → qmd --version or npm install -g @tobilu/qmd
  - `create_qmd_collection(knowledge_path)` → qmd collection add if not exists
  - `run_qmd_index()` → qmd update && qmd embed
- [ ] 7.5 Setup must save `semantic_search_enabled: true` only after all steps succeed
- [ ] 7.6 If any step fails, return `QmdSetupResult { success: false, error: Some(...) }` and do NOT change settings

_Requirements: 4.2, 4.3, 4.4, 4.7, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7_

**Checkpoint**: `cargo check` passes. All 4 Tauri commands defined.

---

## Phase 6: Provider Registration and Command Wiring

### Task 8: Register search provider in `lib.rs`

- [ ] 8.1 In `lib.rs` setup, after gem_store and settings_manager are created, add search provider registration:
  - Read `settings.search.semantic_search_enabled`
  - If `false` → create `FtsResultProvider::new(gem_store.clone())` → `app.manage(Arc::new(fts) as Arc<dyn SearchResultProvider>)`
  - If `true` → attempt `QmdResultProvider::find_qmd_binary()`, verify availability, register QMD or fall back to FTS
- [ ] 8.2 Log which provider was selected: `eprintln!("Search: Using ... provider")`
- [ ] 8.3 Register all 4 search commands in `generate_handler![]`:
  - `search::commands::search_gems`
  - `search::commands::check_search_availability`
  - `search::commands::setup_semantic_search`
  - `search::commands::rebuild_search_index`
- [ ] 8.4 Remove the old `search_gems` command from `src/commands.rs` (line ~461) and its registration from `generate_handler!`

_Requirements: 5.1, 5.2, 5.3, 5.4, 10.4, 10.5_

### Task 9: Wire lifecycle hooks

- [ ] 9.1 In the `save_gem` command (src/commands.rs), after successful save + knowledge file generation, add:
  ```rust
  if let Some(provider) = app_handle.try_state::<Arc<dyn SearchResultProvider>>() {
      if let Err(e) = provider.index_gem(&gem_id).await {
          eprintln!("Search: Failed to index gem {}: {}", gem_id, e);
      }
  }
  ```
- [ ] 9.2 Add the same hook to `enrich_gem` command (after enrichment completes)
- [ ] 9.3 Add the same hook to `transcribe_gem` command (after transcript saved)
- [ ] 9.4 In `delete_gem` command, add `provider.remove_gem(&gem_id)` after DB delete
- [ ] 9.5 On app launch, if semantic search enabled, spawn `provider.reindex_all()` in background (fire-and-forget)
- [ ] 9.6 All lifecycle hooks use `try_state` (not `state`) so commands work even if search provider not registered
- [ ] 9.7 All lifecycle hook failures are logged but NEVER propagate as command failures

_Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7_

**Checkpoint**: `cargo build` succeeds. Provider registered, commands wired, lifecycle hooks in place. FTS search works end-to-end via the new trait-based command.

---

## Phase 7: Frontend — TypeScript Types and Search Integration

### Task 10: Add TypeScript types

- [ ] 10.1 Add `GemSearchResult` interface to `src/state/types.ts`:
  ```typescript
  interface GemSearchResult {
    score: number;
    matched_chunk: string;
    match_type: 'Keyword' | 'Semantic' | 'Hybrid';
    id: string;
    source_type: string;
    source_url: string;
    domain: string;
    title: string;
    author: string | null;
    description: string | null;
    captured_at: string;
    tags: string[] | null;
    summary: string | null;
  }
  ```
- [ ] 10.2 Add `QmdSetupResult` and `SetupProgressEvent` interfaces
- [ ] 10.3 Export all new types

_Requirements: 9.2_

### Task 11: Update `GemsPanel.tsx` to use new `search_gems` command

- [ ] 11.1 Update `invoke('search_gems', { query })` to `invoke<GemSearchResult[]>('search_gems', { query, limit: 50 })`
- [ ] 11.2 Update the state type from `GemPreview[]` to `GemSearchResult[]` (or a union type that handles both)
- [ ] 11.3 For `filter_gems_by_tag` results, wrap `GemPreview[]` as `GemSearchResult[]` with default search fields (score: 1.0, matched_chunk: '', match_type: 'Keyword')
- [ ] 11.4 Keep the 300ms debounce unchanged
- [ ] 11.5 Verify existing search behavior works identically with FTS provider active

_Requirements: 9.1, 9.5, 9.6, 9.7_

### Task 12: Add relevance score badge to gem cards

- [ ] 12.1 When `match_type` is `Semantic` or `Hybrid`, show a relevance badge: `{Math.round(score * 100)}%`
- [ ] 12.2 When `match_type` is `Keyword`, show no badge (preserves current behavior)
- [ ] 12.3 Add CSS for `.relevance-badge` — inline block, blue tint background, small font, rounded corners
- [ ] 12.4 Position badge on gem card (before or after title, consistent with existing metadata badges)

_Requirements: 9.3, 9.4_

**Checkpoint**: Frontend builds. Gem search works through the new trait-based Tauri command. Score badges appear for semantic results.

---

## Phase 8: Frontend — Settings UI

### Task 13: Add Semantic Search section to `SettingsPanel.tsx`

- [ ] 13.1 Add state variables: `semanticSearchEnabled`, `setupInProgress`, `setupSteps: SetupProgressEvent[]`
- [ ] 13.2 On mount, call `invoke('check_search_availability')` to determine current status
- [ ] 13.3 Add "Semantic Search" section after existing MLX/AI sections
- [ ] 13.4 When NOT configured, show:
  - Status: "Not configured"
  - Description text explaining semantic search + requirements (Node.js 22+, ~2GB models)
  - "Enable Semantic Search" button
- [ ] 13.5 When "Enable" clicked, call `invoke('setup_semantic_search')`:
  - Listen for `"semantic-search-setup"` Tauri events
  - Display each step with spinner → checkmark → error icon
  - On success, show "Restart Jarvis to apply changes"
  - On failure, show error message and "Retry" option
- [ ] 13.6 When CONFIGURED and active, show:
  - Status: "Ready" (green dot)
  - Provider info, model cache location
  - "Rebuild Index" button → calls `invoke('rebuild_search_index')`
  - "Disable" button → sets `semantic_search_enabled: false` in settings, shows restart prompt
- [ ] 13.7 "Disable" does NOT uninstall QMD or delete models — only changes the setting

_Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6_

### Task 14: Add CSS for Settings semantic search section

- [ ] 14.1 Style the setup progress steps: step number, description, status icon
- [ ] 14.2 Style the status indicators (green dot for active, gray for inactive)
- [ ] 14.3 Style the "Restart required" message
- [ ] 14.4 Follow existing SettingsPanel styling patterns

**Checkpoint**: Full UI works. User can enable/disable semantic search from Settings. Setup flow shows step-by-step progress.

---

## Phase 9: Testing and Polish

### Task 15: Unit tests for provider types and FtsResultProvider

- [ ] 15.1 Test `MatchType` serialization/deserialization (Keyword → "Keyword", etc.)
- [ ] 15.2 Test `SearchResult` and `GemSearchResult` serde roundtrip
- [ ] 15.3 Test `FtsResultProvider::check_availability()` always returns true
- [ ] 15.4 Test `FtsResultProvider::index_gem()` and `remove_gem()` return `Ok(())`
- [ ] 15.5 Test `FtsResultProvider::reindex_all()` returns `Ok(0)`
- [ ] 15.6 Test `FtsResultProvider::search()` score derivation: first result = 1.0, second = 0.95, etc.

_Validates: Requirements 1, 2; Properties 1, 2_

### Task 16: Unit tests for QmdResultProvider helpers

- [ ] 16.1 Test `extract_gem_id_from_path` with absolute path containing knowledge_path prefix
- [ ] 16.2 Test `extract_gem_id_from_path` with relative path `{gem_id}/gem.md`
- [ ] 16.3 Test `extract_gem_id_from_path` with path containing spaces (macOS `Application Support`)
- [ ] 16.4 Test `extract_gem_id_from_path` with non-matching path returns `None`
- [ ] 16.5 Test `normalize_qmd_score` with values in range (0.5 → 0.5)
- [ ] 16.6 Test `normalize_qmd_score` with values out of range (1.5 → 1.0, -0.3 → 0.0)

_Validates: Requirements 3; Properties 2, 5_

### Task 17: Unit tests for setup helpers

- [ ] 17.1 Test `check_node_version` parsing: "v24.1.0" → 24, "v18.0.0" → error (< 22)
- [ ] 17.2 Test `check_node_version` with invalid input: empty string, missing "v" prefix
- [ ] 17.3 Test empty query handling in `search_gems` command (returns all gems, no error)

_Validates: Requirements 4, 6; Property 3_

### Task 18: End-to-end verification

- [ ] 18.1 Verify FTS search works through the new trait-based command (same results as before)
- [ ] 18.2 Verify empty query returns all gems (backward compatible)
- [ ] 18.3 Verify gem save → `index_gem()` called (check logs)
- [ ] 18.4 Verify gem delete → `remove_gem()` called (check logs)
- [ ] 18.5 Verify Settings UI shows "Not configured" state correctly
- [ ] 18.6 Verify app starts with `semantic_search_enabled: true` but QMD not installed → falls back to FTS with log warning
- [ ] 18.7 Verify `filter_gems_by_tag` still works alongside new search command
- [ ] 18.8 Verify no score badge shown for keyword search results (current behavior preserved)

_Validates: Properties 1, 3, 6, 8_

### Task 19*: QMD integration testing (requires QMD installed)

- [ ] 19.1 Install QMD manually: `npm install -g @tobilu/qmd`
- [ ] 19.2 Run `setup_semantic_search` → verify all 6 steps complete
- [ ] 19.3 Search with QMD active → verify semantic results with score badges
- [ ] 19.4 Test semantic match: search "container orchestration" → find gem titled "ECS vs EKS" (no keyword overlap)
- [ ] 19.5 Save new gem → verify `qmd status` shows updated count
- [ ] 19.6 "Rebuild Index" button → verify full re-index runs
- [ ] 19.7 Disable semantic search → restart → verify FTS5 is active

*\* Optional — requires actual QMD installation and gem corpus. Skip for CI, run manually before release.*

**Checkpoint**: All tests pass. Feature is complete and verified against all correctness properties.
