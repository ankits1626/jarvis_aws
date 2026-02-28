# Kiro Prompt — Searchable Gems Phase 2: FtsResultProvider (Default Provider)

## What You're Building

Implement `FtsResultProvider` — the **default** search provider that wraps the existing `GemStore::search()` FTS5 keyword search. This provider is always available, requires zero setup, and makes every `index_gem()` / `remove_gem()` / `reindex_all()` call a no-op (SQLite FTS5 triggers handle indexing automatically).

This is the simplest provider — structurally similar to `NoOpProvider` in `intelligence/noop_provider.rs`, except `search()` actually delegates to `GemStore`.

## Spec Files

- **Requirements**: `.kiro/specs/searchable-gems/requirements.md` — Requirement 2
- **Design**: `.kiro/specs/searchable-gems/design.md` — Section `fts_provider.rs`
- **Tasks**: `.kiro/specs/searchable-gems/tasks.md` — Phase 2, Task 3

## Context: What Already Exists

Phase 1 created the trait and types in `src/search/provider.rs`:

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

The `GemStore` trait (in `gems/store.rs`) has:
```rust
#[async_trait]
pub trait GemStore: Send + Sync {
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<GemPreview>, String>;
    // ... other methods
}
```

`GemPreview` fields: `id`, `source_type`, `source_url`, `domain`, `title`, `author`, `description`, `content_preview`, `captured_at`, `tags`, `summary`, `enrichment_source`, `transcript_language`.

## Exact Task

### Replace the contents of `jarvis-app/src-tauri/src/search/fts_provider.rs`

Currently it's a placeholder comment. Replace with the full implementation.

**Imports:**
```rust
use std::sync::Arc;
use async_trait::async_trait;
use crate::gems::GemStore;
use crate::intelligence::AvailabilityResult;
use super::provider::{SearchResultProvider, SearchResult, MatchType};
```

**Struct:**
```rust
pub struct FtsResultProvider {
    gem_store: Arc<dyn GemStore>,
}
```

**Constructor:**
```rust
impl FtsResultProvider {
    pub fn new(gem_store: Arc<dyn GemStore>) -> Self {
        Self { gem_store }
    }
}
```

**Trait Implementation — method by method:**

1. **`check_availability()`** — Always returns available. FTS5 is built into SQLite, it can't be "unavailable":
   ```rust
   AvailabilityResult { available: true, reason: None }
   ```

2. **`search()`** — Delegates to `self.gem_store.search(query, limit).await`. Then maps each `GemPreview` to a `SearchResult`:
   - `gem_id` ← `gem.id`
   - `score` ← derived from rank order: `(1.0 - (index as f64 * 0.05)).max(0.0)` where `index` is the position (0-based) in the results. First result gets 1.0, second 0.95, third 0.90, etc. Clamped to minimum 0.0.
   - `matched_chunk` ← empty `String::new()` (FTS5 doesn't provide snippets via this API)
   - `match_type` ← `MatchType::Keyword`

3. **`index_gem()`** — No-op, return `Ok(())`. Add comment: FTS5 triggers handle indexing automatically.

4. **`remove_gem()`** — No-op, return `Ok(())`. Add comment: FTS5 triggers handle deletion automatically.

5. **`reindex_all()`** — Return `Ok(0)`. Add comment: FTS5 index is maintained by SQLite triggers. Nothing to rebuild.

**Add a doc comment on the struct:**
```rust
/// Default search provider — wraps existing SQLite FTS5 keyword search.
///
/// Always available, zero setup. Returns MatchType::Keyword.
/// FTS5 indexing is handled by SQLite triggers, so index_gem/remove_gem are no-ops.
```

### Update `mod.rs` re-exports

Add this line to `src/search/mod.rs` after the existing `provider` re-exports:
```rust
pub use fts_provider::FtsResultProvider;
```

## Pattern to Follow

Here's the `NoOpProvider` from `intelligence/noop_provider.rs` — your implementation is structurally identical but with `gem_store` instead of `reason`, and `search()` delegates instead of returning `Err`:

```rust
use async_trait::async_trait;
use super::provider::{AvailabilityResult, IntelProvider};

pub struct NoOpProvider {
    reason: String,
}

impl NoOpProvider {
    pub fn new(reason: String) -> Self {
        Self { reason }
    }
}

#[async_trait]
impl IntelProvider for NoOpProvider {
    async fn check_availability(&self) -> AvailabilityResult {
        AvailabilityResult {
            available: false,
            reason: Some(self.reason.clone()),
        }
    }

    async fn generate_tags(&self, _content: &str) -> Result<Vec<String>, String> {
        Err("IntelligenceKit unavailable".to_string())
    }

    async fn summarize(&self, _content: &str) -> Result<String, String> {
        Err("IntelligenceKit unavailable".to_string())
    }
}
```

## Important Details

- **`Arc<dyn GemStore>`** — The gem store is shared across the app. FtsResultProvider holds an `Arc` to it, same pattern as how `MlxProvider` holds `Arc<Mutex<ProviderState>>`.
- **Score derivation** — FTS5 doesn't return a normalized score via `GemStore::search()`. We derive one from rank position. This is intentional and noted in the design doc. First result = 1.0, each subsequent result loses 0.05. After 20 results they'd all be 0.0, which is fine — we never return more than `limit` results.
- **No `_` prefix on parameters** — Unlike `NoOpProvider` which uses `_content`, `FtsResultProvider::search()` actually USES `query` and `limit`. Only `index_gem` and `remove_gem` should prefix the parameter with `_` since they're no-ops: `_gem_id`.
- **Don't import `GemPreview`** — You don't need to import it. `gem_store.search()` returns `Result<Vec<GemPreview>, String>` and you destructure each item by field access (`.id`, etc.). The type is inferred.

## Verification

Run `cargo check` from `jarvis-app/src-tauri/`. Must pass with zero new errors.

**Expected outcome:**
- 1 file modified: `src/search/fts_provider.rs` (placeholder → full implementation)
- 1 file modified: `src/search/mod.rs` (added re-export)
- `cargo check` passes

## If You're Unsure

- **What does `GemStore::search()` return?** → `Result<Vec<GemPreview>, String>`. Each `GemPreview` has an `id: String` field.
- **Should I import `GemPreview`?** → No. You access `.id` on each item from the iterator. Rust infers the type.
- **What if `gem_store.search()` returns an error?** → Propagate it with `?`. The `search()` method returns `Result<Vec<SearchResult>, String>` — same error type.
- **Should the struct be `pub`?** → Yes. Both the struct and `new()` are `pub`.
- **Anything else confusing?** → Ask before guessing.

## When Done

Stop and ask for review. Show me:
1. The full `fts_provider.rs` content
2. The updated `mod.rs`
3. `cargo check` output
4. Any questions or decisions you made

Do NOT proceed to Phase 3 until I review and approve.
