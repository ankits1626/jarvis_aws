# Kiro Prompt — Searchable Gems Phase 1: Core Types and Trait Definition

## What You're Building

Phase 1 of the Searchable Gems feature — define the `SearchResultProvider` trait and all shared data types in a new `src/search/` module. **No implementations yet** (no FtsResultProvider, no QmdResultProvider). Just the trait contract, data types, and module wiring.

## Spec Files (Read These First)

1. **Requirements**: `.kiro/specs/searchable-gems/requirements.md` — Requirement 1 (trait + data types), Requirement 10 (module structure)
2. **Design**: `.kiro/specs/searchable-gems/design.md` — Section "Modules and Interfaces" > `provider.rs` for exact code, `mod.rs` for re-exports
3. **Tasks**: `.kiro/specs/searchable-gems/tasks.md` — Phase 1 (Tasks 1 and 2)

## Exact Tasks

### Task 1: Create `jarvis-app/src-tauri/src/search/provider.rs`

Define these types and the trait. Follow the derive macro and import patterns exactly as shown below.

**Imports:**
```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::intelligence::AvailabilityResult;  // reuse, don't redefine
```

**Types to define (all derive `Debug, Clone, Serialize, Deserialize`):**

1. **`MatchType`** enum — variants: `Keyword`, `Semantic`, `Hybrid`

2. **`SearchResult`** struct — the standard return type from every provider:
   - `gem_id: String` — the gem UUID that matched
   - `score: f64` — relevance score, normalized to 0.0–1.0 (1.0 = best)
   - `matched_chunk: String` — snippet of text that matched (empty if provider doesn't support snippets)
   - `match_type: MatchType`

3. **`GemSearchResult`** struct — enriched result returned to the frontend (SearchResult joined with gem metadata):
   - Search fields: `score: f64`, `matched_chunk: String`, `match_type: MatchType`
   - Gem fields (match `GemPreview` from `gems/store.rs`): `id: String`, `source_type: String`, `source_url: String`, `domain: String`, `title: String`, `author: Option<String>`, `description: Option<String>`, `captured_at: String`, `tags: Option<Vec<String>>`, `summary: Option<String>`
   - **Note:** `GemSearchResult` does NOT include `content_preview`, `enrichment_source`, or `transcript_language` from `GemPreview` — those aren't needed for search results

4. **`QmdSetupResult`** struct — result of the semantic search setup flow:
   - `success: bool`
   - `node_version: Option<String>`
   - `qmd_version: Option<String>`
   - `docs_indexed: Option<usize>`
   - `error: Option<String>`

5. **`SetupProgressEvent`** struct — progress event emitted during setup:
   - `step: usize`
   - `total: usize`
   - `description: String`
   - `status: String` (values will be "running", "done", "failed")
   - **Note:** This only needs `Debug, Clone, Serialize` (not `Deserialize` — it's only sent from backend to frontend)

**Trait to define:**

```rust
#[async_trait]
pub trait SearchResultProvider: Send + Sync {
    async fn check_availability(&self) -> AvailabilityResult;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, String>;
    async fn index_gem(&self, gem_id: &str) -> Result<(), String>;
    async fn remove_gem(&self, gem_id: &str) -> Result<(), String>;
    async fn reindex_all(&self) -> Result<usize, String>;
}
```

Add doc comments on the trait:
```
/// Backend-agnostic search result provider.
///
/// Tauri commands call this trait, never a concrete implementation.
/// Each provider fulfills the contract — returns results in the standard format.
///
/// Adding a new search backend = implement this trait + register in lib.rs.
///
/// Follows the same pattern as IntelProvider (AI), KnowledgeStore (knowledge files),
/// GemStore (database), Chatable (chat).
```

Add doc comments on each method — see `design.md` section for `provider.rs` for the exact comments.

### Task 2: Create `jarvis-app/src-tauri/src/search/mod.rs`

```rust
pub mod provider;
pub mod fts_provider;
pub mod qmd_provider;
pub mod commands;

pub use provider::{
    SearchResultProvider,
    SearchResult,
    MatchType,
    GemSearchResult,
    QmdSetupResult,
    SetupProgressEvent,
};
// FtsResultProvider and QmdResultProvider will be re-exported in Phase 2 and 3
```

**IMPORTANT:** Since `fts_provider.rs`, `qmd_provider.rs`, and `commands.rs` don't exist yet, you have two options:
- **Option A (recommended):** Create empty placeholder files for them so `mod.rs` compiles:
  - `fts_provider.rs` — empty file
  - `qmd_provider.rs` — empty file
  - `commands.rs` — empty file
- **Option B:** Comment out those module declarations for now and add a `// TODO: uncomment in Phase 2-5` comment

Pick whichever you prefer, but `cargo check` must pass at the end.

### Task 2b: Add module declaration to `lib.rs`

Add `pub mod search;` to the module declarations in `jarvis-app/src-tauri/src/lib.rs`. Place it alphabetically (after `pub mod recording;`, before `pub mod settings;`).

## Patterns to Follow

**The existing codebase uses these exact patterns — match them:**

Module declaration in `lib.rs`:
```rust
pub mod agents;
pub mod browser;
pub mod commands;
pub mod error;
pub mod files;
pub mod gems;
pub mod intelligence;
pub mod knowledge;
pub mod platform;
pub mod recording;
pub mod search;      // ← ADD HERE (alphabetical)
pub mod settings;
pub mod shortcuts;
pub mod transcription;
pub mod wav;
```

Trait definition pattern (from `intelligence/provider.rs`):
```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityResult { ... }

#[async_trait]
pub trait IntelProvider: Send + Sync {
    async fn check_availability(&self) -> AvailabilityResult;
    ...
}
```

Module re-export pattern (from `intelligence/mod.rs`):
```rust
pub mod provider;
pub mod mlx_provider;
...
pub use provider::{AvailabilityResult, IntelProvider};
pub use mlx_provider::MlxProvider;
```

## Verification

When done, run `cargo check` from `jarvis-app/src-tauri/`. It must pass with zero errors.

**Expected outcome:**
- 4 new files created: `src/search/mod.rs`, `src/search/provider.rs`, `src/search/fts_provider.rs` (empty), `src/search/qmd_provider.rs` (empty), `src/search/commands.rs` (empty)
- 1 file modified: `src/lib.rs` (added `pub mod search;`)
- `cargo check` passes

## If You're Unsure About Anything

- **Field names/types**: Check `gems/store.rs` for `GemPreview` fields, `intelligence/provider.rs` for `AvailabilityResult`
- **Import paths**: The crate root is `jarvis_app` but internal modules use `crate::` (e.g., `crate::intelligence::AvailabilityResult`)
- **async_trait**: Already in `Cargo.toml` as a dependency — just `use async_trait::async_trait;`
- **serde**: Already in `Cargo.toml` — just `use serde::{Deserialize, Serialize};`
- **Confused about a field type or whether to include it?** Ask me before guessing. It's better to clarify than to get it wrong.

## When Done

Stop and ask for review. Show me:
1. The files you created/modified
2. The `cargo check` output
3. Any decisions you made (e.g., Option A vs B for placeholder files)

Do NOT proceed to Phase 2 until I review and approve Phase 1.
