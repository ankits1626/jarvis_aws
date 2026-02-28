# Kiro Prompt — Searchable Gems Phase 6: Provider Registration & Lifecycle Hooks

## What You're Building

Wire everything together:
1. **Register the search provider** in `lib.rs` setup — FTS by default, QMD if enabled + available
2. **Register remaining Tauri commands** in `generate_handler![]`
3. **Add lifecycle hooks** to `save_gem`, `enrich_gem`, `transcribe_gem`, `delete_gem` — so the search index stays current

## Spec Files

- **Tasks**: `.kiro/specs/searchable-gems/tasks.md` — Phase 6, Tasks 8 and 9

## Context: What Already Exists

Phases 1–5 created:
- `src/search/provider.rs` — trait + types
- `src/search/fts_provider.rs` — `FtsResultProvider`
- `src/search/qmd_provider.rs` — `QmdResultProvider`
- `src/search/commands.rs` — 4 Tauri commands + setup helpers
- `src/settings/manager.rs` — `SearchSettings { semantic_search_enabled: bool }`
- `lib.rs` already has `pub mod search;` and `search::commands::search_gems` in `generate_handler![]`

## Exact Changes

### Part A: Register Search Provider in `lib.rs`

**Where to insert:** After the knowledge migration spawn block (line ~272), before `Ok(())`.

**Add these imports** at the top of `lib.rs`, alongside existing use statements:

```rust
use search::{FtsResultProvider, QmdResultProvider, SearchResultProvider};
```

**Insert this block** in the `setup` closure, after the knowledge migration spawn (after line 272, before `Ok(())`):

```rust
// Initialize Search Provider
// Read semantic_search_enabled from settings
let search_settings = {
    let manager = app.state::<Arc<RwLock<SettingsManager>>>();
    let settings = manager.read()
        .expect("Failed to acquire settings read lock")
        .get();
    settings.search.semantic_search_enabled
};

let search_provider: Arc<dyn SearchResultProvider> = if search_settings {
    // Try to initialize QMD provider
    match tauri::async_runtime::block_on(QmdResultProvider::find_qmd_binary()) {
        Some(qmd_path) => {
            let knowledge_path = app.path().app_data_dir()
                .expect("Failed to get app data dir")
                .join("knowledge");
            let qmd = QmdResultProvider::new(qmd_path.clone(), knowledge_path);

            // Check availability before committing
            let availability = tauri::async_runtime::block_on(qmd.check_availability());
            if availability.available {
                eprintln!("Search: Using QMD semantic search provider ({})", qmd_path.display());
                Arc::new(qmd)
            } else {
                eprintln!(
                    "Search: QMD unavailable ({}), falling back to FTS5",
                    availability.reason.unwrap_or_else(|| "unknown".to_string())
                );
                Arc::new(FtsResultProvider::new(gem_store_arc.clone()))
            }
        }
        None => {
            eprintln!("Search: QMD binary not found, falling back to FTS5");
            Arc::new(FtsResultProvider::new(gem_store_arc.clone()))
        }
    }
} else {
    eprintln!("Search: Using FTS5 keyword search provider (default)");
    Arc::new(FtsResultProvider::new(gem_store_arc.clone()))
};

app.manage(search_provider);
```

**Key points about this block:**
- Uses `tauri::async_runtime::block_on()` — same pattern as the `intel_provider` initialization on lines 92–94
- Uses `gem_store_arc.clone()` — the variable defined on line 44 (still in scope)
- Uses `app.state::<Arc<RwLock<SettingsManager>>>()` to re-read settings — same pattern as line 129
- The `app.path().app_data_dir()` call uses `.expect()` (not `?`) because we're past the initial error-handling section — consistent with how `knowledge_path` is also built in setup
- Falls back to FTS5 on ANY QMD failure — the user can retry from Settings

### Part B: Register Remaining Commands in `generate_handler![]`

Currently `search::commands::search_gems` is already registered (line 306). Add the other 3 commands.

**Find this in `lib.rs`** (line ~306):
```rust
            search::commands::search_gems,
```

**Replace with:**
```rust
            search::commands::search_gems,
            search::commands::check_search_availability,
            search::commands::setup_semantic_search,
            search::commands::rebuild_search_index,
```

### Part C: Lifecycle Hooks in `commands.rs`

Add a search index import at the top of `commands.rs`. **Add this line** after the existing imports (after line 17):

```rust
use crate::search::SearchResultProvider;
```

#### Hook 1: `save_gem` — After knowledge file creation

In `save_gem()` (line ~249), the function currently ends like this (lines 310–318):

```rust
    // Generate knowledge files
    if let Ok(ref saved_gem) = result {
        if let Some(ks) = app_handle.try_state::<Arc<dyn crate::knowledge::KnowledgeStore>>() {
            if let Err(e) = ks.create(saved_gem).await {
                eprintln!("Knowledge file creation failed for gem {}: {}", saved_gem.id, e);
            }
        }
    }

    result
```

**Replace with:**
```rust
    // Generate knowledge files
    if let Ok(ref saved_gem) = result {
        if let Some(ks) = app_handle.try_state::<Arc<dyn crate::knowledge::KnowledgeStore>>() {
            if let Err(e) = ks.create(saved_gem).await {
                eprintln!("Knowledge file creation failed for gem {}: {}", saved_gem.id, e);
            }
        }
        // Update search index
        if let Some(provider) = app_handle.try_state::<Arc<dyn SearchResultProvider>>() {
            if let Err(e) = provider.index_gem(&saved_gem.id).await {
                eprintln!("Search: Failed to index gem {}: {}", saved_gem.id, e);
            }
        }
    }

    result
```

#### Hook 2: `delete_gem` — After knowledge file deletion

In `delete_gem()` (line ~425), the function currently ends like this (lines 430–440):

```rust
    gem_store.delete(&id).await?;

    // Delete knowledge files
    if let Some(ks) = app_handle.try_state::<Arc<dyn crate::knowledge::KnowledgeStore>>() {
        if let Err(e) = ks.delete(&id).await {
            eprintln!("Knowledge file deletion failed for gem {}: {}", id, e);
        }
    }

    Ok(())
```

**Replace with:**
```rust
    gem_store.delete(&id).await?;

    // Delete knowledge files
    if let Some(ks) = app_handle.try_state::<Arc<dyn crate::knowledge::KnowledgeStore>>() {
        if let Err(e) = ks.delete(&id).await {
            eprintln!("Knowledge file deletion failed for gem {}: {}", id, e);
        }
    }
    // Remove from search index
    if let Some(provider) = app_handle.try_state::<Arc<dyn SearchResultProvider>>() {
        if let Err(e) = provider.remove_gem(&id).await {
            eprintln!("Search: Failed to remove gem {}: {}", id, e);
        }
    }

    Ok(())
```

#### Hook 3: `enrich_gem` — After knowledge enrichment update

In `enrich_gem()` (line ~547), the function currently ends like this (lines 607–622):

```rust
    // Save and return
    let result = gem_store.save(gem).await;

    // Update knowledge files
    if let Ok(ref enriched_gem) = result {
        if let Some(ks) = app_handle.try_state::<Arc<dyn crate::knowledge::KnowledgeStore>>() {
            // Update enrichment subfile
            if let Some(ref enrichment) = enriched_gem.ai_enrichment {
                let formatted = crate::knowledge::assembler::format_enrichment(enrichment);
                if let Err(e) = ks.update_subfile(&enriched_gem.id, "enrichment.md", &formatted).await {
                    eprintln!("Knowledge enrichment update failed: {}", e);
                }
            }
        }
    }

    result
```

**Replace with:**
```rust
    // Save and return
    let result = gem_store.save(gem).await;

    // Update knowledge files
    if let Ok(ref enriched_gem) = result {
        if let Some(ks) = app_handle.try_state::<Arc<dyn crate::knowledge::KnowledgeStore>>() {
            // Update enrichment subfile
            if let Some(ref enrichment) = enriched_gem.ai_enrichment {
                let formatted = crate::knowledge::assembler::format_enrichment(enrichment);
                if let Err(e) = ks.update_subfile(&enriched_gem.id, "enrichment.md", &formatted).await {
                    eprintln!("Knowledge enrichment update failed: {}", e);
                }
            }
        }
        // Update search index (enrichment changes tags/summary which improves search)
        if let Some(provider) = app_handle.try_state::<Arc<dyn SearchResultProvider>>() {
            if let Err(e) = provider.index_gem(&enriched_gem.id).await {
                eprintln!("Search: Failed to re-index gem {}: {}", enriched_gem.id, e);
            }
        }
    }

    result
```

#### Hook 4: `transcribe_gem` — After knowledge file recreation

In `transcribe_gem()` (line ~640), the function currently ends like this (lines 708–719):

```rust
    // Save and return
    let result = gem_store.save(gem).await;

    // Recreate all knowledge files (transcript + re-enrichment changes multiple things)
    if let Ok(ref gem) = result {
        if let Some(ks) = app_handle.try_state::<Arc<dyn crate::knowledge::KnowledgeStore>>() {
            if let Err(e) = ks.create(gem).await {
                eprintln!("Knowledge file update failed: {}", e);
            }
        }
    }

    result
```

**Replace with:**
```rust
    // Save and return
    let result = gem_store.save(gem).await;

    // Recreate all knowledge files (transcript + re-enrichment changes multiple things)
    if let Ok(ref gem) = result {
        if let Some(ks) = app_handle.try_state::<Arc<dyn crate::knowledge::KnowledgeStore>>() {
            if let Err(e) = ks.create(gem).await {
                eprintln!("Knowledge file update failed: {}", e);
            }
        }
        // Update search index (transcript changes searchable content)
        if let Some(provider) = app_handle.try_state::<Arc<dyn SearchResultProvider>>() {
            if let Err(e) = provider.index_gem(&gem.id).await {
                eprintln!("Search: Failed to re-index gem {}: {}", gem.id, e);
            }
        }
    }

    result
```

## Gotchas

1. **`try_state` not `state`** — All lifecycle hooks use `app_handle.try_state::<Arc<dyn SearchResultProvider>>()`. This returns `Option`, so the hooks gracefully no-op if the provider isn't registered. The existing knowledge hooks use the same pattern — follow their lead.

2. **Never propagate search errors** — Every hook wraps the provider call in `if let Err(e) = ... { eprintln!(...) }`. A search index failure must NEVER cause `save_gem`, `delete_gem`, `enrich_gem`, or `transcribe_gem` to return `Err`. The gem operation succeeded; the index update is best-effort.

3. **`gem_store_arc` is still in scope** — In `lib.rs`, the variable `gem_store_arc` is defined on line 44 and used throughout setup. It's still available where you insert the search provider block.

4. **`block_on` for async in setup** — The Tauri `setup` closure is synchronous. Use `tauri::async_runtime::block_on()` for `find_qmd_binary()` and `check_availability()` — same pattern as the `intel_provider` initialization (lines 92–94).

5. **Import path for SearchResultProvider** — In `commands.rs`, use `use crate::search::SearchResultProvider;`. In `lib.rs`, use `use search::{FtsResultProvider, QmdResultProvider, SearchResultProvider};` (crate-relative from lib.rs).

6. **The `settings` variable (line 76)** is out of scope by the time we need search settings. That's why we re-read settings from `app.state`. This is intentional — the same re-read pattern is used on line 129–132 for transcription settings.

7. **`app.path().app_data_dir()`** returns `Result<PathBuf, _>` in Tauri 2.x. Use `.expect()` here (not `?`) — consistent with the `knowledge_path` being constructed with `.map_err()?` earlier in setup. Actually check the existing pattern — if lines 48–49 use `?`, use `?` here too.

8. **Task 9.5 (background reindex on launch)** — The tasks.md mentions spawning `provider.reindex_all()` on launch when semantic search is enabled. **Skip this for now** — it's a nice-to-have and adds complexity. QMD's `index_gem()` fire-and-forget on each save/enrich/transcribe keeps the index current enough. We can add startup reindex later if needed.

## Verification

Run `cargo check` from `jarvis-app/src-tauri/`. Must pass with zero new errors.

**Expected outcome:**
- 1 file modified: `src/lib.rs` (search provider registration + 3 new command registrations)
- 1 file modified: `src/commands.rs` (1 new import + 4 lifecycle hooks)
- `cargo check` passes
- No changes to `src/search/` files (they're complete from Phases 1–5)

## Summary of All Changes

| File | Change |
|------|--------|
| `src/lib.rs` | Add `use search::{...}` import |
| `src/lib.rs` | Add search provider registration block in `setup` |
| `src/lib.rs` | Add 3 commands to `generate_handler![]` |
| `src/commands.rs` | Add `use crate::search::SearchResultProvider;` import |
| `src/commands.rs` | Add `provider.index_gem()` hook to `save_gem` |
| `src/commands.rs` | Add `provider.remove_gem()` hook to `delete_gem` |
| `src/commands.rs` | Add `provider.index_gem()` hook to `enrich_gem` |
| `src/commands.rs` | Add `provider.index_gem()` hook to `transcribe_gem` |

## When Done

Stop and ask for review. Show me:
1. The search provider registration block from `lib.rs`
2. The updated `generate_handler![]` with all 4 search commands
3. All 4 lifecycle hooks from `commands.rs`
4. `cargo check` output
5. Any questions or decisions you made

Do NOT proceed to Phase 7 until I review and approve.
