# Phase 1 Implementation Prompt — Search Infrastructure

## What You're Building

Implement Phase 1 from `.kiro/specs/project-research-assistant/tasks.md` — **Tasks 1, 2, and 3**. This phase adds web search capability to the existing `SearchResultProvider` trait, creates a Tavily API provider, and wraps everything in a composite delegator.

**Read these files before writing any code:**
- `.kiro/specs/project-research-assistant/requirements.md` — Requirements 1, 2, 3 (acceptance criteria)
- `.kiro/specs/project-research-assistant/design.md` — Full Rust code for `provider.rs` changes, `tavily_provider.rs`, `composite_provider.rs`, and `mod.rs`
- `jarvis-app/src-tauri/src/search/provider.rs` — Current trait you'll extend
- `jarvis-app/src-tauri/src/search/mod.rs` — Current module root you'll update
- `jarvis-app/src-tauri/src/search/fts_provider.rs` — Existing provider (DO NOT MODIFY, just verify it compiles)
- `jarvis-app/src-tauri/src/search/qmd_provider.rs` — Existing provider (DO NOT MODIFY, just verify it compiles)
- `jarvis-app/src-tauri/src/intelligence/provider.rs` — `AvailabilityResult` struct is imported from here

## Task 1: Extend `SearchResultProvider` with Web Search

**File to modify:** `jarvis-app/src-tauri/src/search/provider.rs`

Add **after** the existing `GemSearchResult` struct (line 56):

1. `WebSourceType` enum — variants: `Paper`, `Article`, `Video`, `Other`. Derive `Debug`, `Clone`, `Serialize`, `Deserialize`.
2. `WebSearchResult` struct — fields: `title` (String), `url` (String), `snippet` (String), `source_type` (WebSourceType), `domain` (String), `published_date` (Option<String>). Derive `Debug`, `Clone`, `Serialize`, `Deserialize`.

Add to the existing `SearchResultProvider` trait **after** the `reindex_all` method (line 117):

3. `web_search` default method: `async fn web_search(&self, _query: &str, _limit: usize) -> Result<Vec<WebSearchResult>, String>` — returns `Ok(Vec::new())`
4. `supports_web_search` default method: `fn supports_web_search(&self) -> bool` — returns `false`

**Critical:** These are **default methods** with implementations. Existing providers (`FtsResultProvider`, `QmdResultProvider`) must NOT need any changes — they inherit the defaults.

## Task 2: Create `TavilyProvider`

**New file:** `jarvis-app/src-tauri/src/search/tavily_provider.rs`

Implements `SearchResultProvider` for web search via the Tavily Search API. The design doc has the complete code. Key points:

- Struct holds `api_key: String` and `client: reqwest::Client`
- Private structs for Tavily API: `TavilySearchRequest` (Serialize), `TavilySearchResponse` and `TavilyResult` (Deserialize)
- `classify_source_type(url)` helper: youtube/vimeo → Video, arxiv/scholar/semanticscholar/ieee/acm → Paper, medium/dev.to/substack/hashnode/blog → Article, else → Other
- `extract_domain(url)` helper: strips protocol, www, path
- `web_search` implementation: POST to `https://api.tavily.com/search`, parse response, classify and map to `Vec<WebSearchResult>`
- `supports_web_search` → `true`
- `check_availability` → true if key non-empty
- Gem methods (`search`, `index_gem`, `remove_gem`, `reindex_all`) → no-ops returning empty/zero
- All logging via `eprintln!` with `Search/Tavily:` prefix

**Note:** `reqwest` is already in `Cargo.toml` with the `json` feature. No dependency changes needed.

## Task 3: Create `CompositeSearchProvider`

**New file:** `jarvis-app/src-tauri/src/search/composite_provider.rs`

Wraps a gem provider and an optional web provider behind a single `SearchResultProvider`. The design doc has the complete code. Key points:

- Struct holds `gem_provider: Arc<dyn SearchResultProvider>` and `web_provider: Option<Arc<dyn SearchResultProvider>>`
- `search`, `index_gem`, `remove_gem`, `reindex_all`, `check_availability` → delegate to `gem_provider`
- `web_search` → delegate to `web_provider` if `Some`, else return `Ok(Vec::new())`
- `supports_web_search` → true only if `web_provider` is `Some` and its `supports_web_search()` returns true
- Logging via `eprintln!` with `Search/Composite:` prefix

## Update Module Root

**File to modify:** `jarvis-app/src-tauri/src/search/mod.rs`

1. Add module declarations: `pub mod tavily_provider;` and `pub mod composite_provider;`
2. Add to the existing `pub use provider::` block: `WebSearchResult`, `WebSourceType`
3. Add new re-exports: `pub use tavily_provider::TavilyProvider;` and `pub use composite_provider::CompositeSearchProvider;`

## Phase Checkpoint

After implementation, verify:

```
cargo check
```

This MUST pass. Specifically verify:
- `FtsResultProvider` and `QmdResultProvider` compile without changes (they inherit the default `web_search` and `supports_web_search`)
- New types `WebSearchResult`, `WebSourceType` are re-exported from `search` module
- `TavilyProvider` and `CompositeSearchProvider` are re-exported from `search` module
- No warnings about unused imports

## Important Notes

- **Follow the design doc code exactly.** The complete Rust code for every file is in `design.md`. Use it as the source of truth.
- **Do NOT modify `fts_provider.rs` or `qmd_provider.rs`.** The whole point of default methods is that existing providers don't need changes.
- **Do NOT modify `commands.rs`.** Search commands are unchanged in this phase.
- **Do NOT modify `lib.rs`.** Provider registration happens in Phase 2.
- **Do NOT create any frontend files.** Frontend is Phase 5+.
- If you have confusion about how `AvailabilityResult` is imported, look at how `fts_provider.rs` or `qmd_provider.rs` does it — follow the same pattern.
- If you're unsure about `async_trait` usage on default methods, check how the existing trait methods are defined — the new default methods go inside the same `#[async_trait]` block.

## Deliverables

When done, present:
1. The 2 new files created (`tavily_provider.rs`, `composite_provider.rs`)
2. The 2 files modified (`provider.rs`, `mod.rs`)
3. `cargo check` output showing success
4. Ask me for review before proceeding to Phase 2
