# Prompt: Implement Gem Knowledge Files — Phase 3 (Migration + Tauri Integration)

## Your Task

Implement **Phase 3: Migration Logic** (Tasks 9–11) and **Phase 4: Tauri Integration** (Tasks 12–16) together. This phase makes the knowledge store actually work end-to-end — registered in Tauri state, wired into gem lifecycle, migration on first launch.

**After completing this, stop and ask me to review.**

If you have any confusion during implementation — about how commands access state, where to inject the knowledge store call, or any edge case — feel free to ask.

---

## Context

Phases 1–2 are complete and compiling:
- `src/knowledge/store.rs` — trait + all data types
- `src/knowledge/assembler.rs` — formatting + assembly functions
- `src/knowledge/local_store.rs` — `LocalKnowledgeStore` with full CRUD + `migrate_all`

**Spec:** `.kiro/specs/gem-knowledge-files/requirements.md` (Req 7–12)
**Tasks:** `.kiro/specs/gem-knowledge-files/tasks.md` (Tasks 9–16)

---

## What This Phase Produces

```
src/knowledge/
├── mod.rs            ← add migration re-export
├── store.rs          ← (unchanged)
├── assembler.rs      ← (unchanged)
├── local_store.rs    ← (unchanged)
├── migration.rs      ← NEW — check_and_run_migration(), copilot backfill
└── commands.rs       ← NEW — Tauri command handlers

src/commands.rs       ← MODIFY — wire knowledge store into save_gem, enrich_gem,
                         delete_gem, transcribe_gem, save_recording_gem
src/lib.rs            ← MODIFY — register LocalKnowledgeStore + migration + new commands
```

---

## Part A: Migration Logic (Tasks 9–10)

### Create `src/knowledge/migration.rs`

#### `check_and_run_migration()`

Called during app startup. Checks if knowledge files need to be generated.

```rust
use std::path::Path;
use crate::gems::{Gem, GemStore};
use crate::knowledge::store::*;
use crate::knowledge::KnowledgeStore;
use crate::knowledge::local_store::CURRENT_KNOWLEDGE_VERSION;
use std::sync::Arc;

/// Check if migration is needed and run it
pub async fn check_and_run_migration(
    knowledge_store: &dyn KnowledgeStore,
    gem_store: &dyn GemStore,
    event_emitter: &(dyn KnowledgeEventEmitter + Sync),
    knowledge_base_path: &Path,
) -> Result<(), String> {
    let version_file = knowledge_base_path.join(".version");

    // Check if version file exists
    let needs_migration = if version_file.exists() {
        // Read stored version
        let stored = tokio::fs::read_to_string(&version_file).await
            .unwrap_or_default();
        let stored_version: u32 = stored.trim().parse().unwrap_or(0);

        if stored_version < CURRENT_KNOWLEDGE_VERSION {
            eprintln!("Knowledge: version {} → {}, reassembly needed", stored_version, CURRENT_KNOWLEDGE_VERSION);
            true  // version bump — reassemble all
        } else {
            eprintln!("Knowledge: up to date (version {})", stored_version);
            false
        }
    } else {
        eprintln!("Knowledge: no version marker, running initial migration");
        true
    };

    if !needs_migration {
        return Ok(());
    }

    // Load ALL gems for migration
    // GemStore::list() returns GemPreview (truncated), we need full Gem objects
    // Strategy: list all IDs, then get() each one
    let previews = gem_store.list(10000, 0).await
        .map_err(|e| format!("Failed to list gems for migration: {}", e))?;

    let mut gems: Vec<Gem> = Vec::new();
    for preview in &previews {
        match gem_store.get(&preview.id).await {
            Ok(Some(gem)) => gems.push(gem),
            Ok(None) => eprintln!("Knowledge migration: gem {} not found, skipping", preview.id),
            Err(e) => eprintln!("Knowledge migration: failed to load gem {}: {}", preview.id, e),
        }
    }

    eprintln!("Knowledge: migrating {} gems", gems.len());

    // Run migration
    let result = knowledge_store.migrate_all(gems, event_emitter).await?;

    eprintln!(
        "Knowledge migration complete: {} created, {} skipped, {} failed",
        result.created, result.skipped, result.failed
    );

    // Write version marker
    tokio::fs::write(&version_file, CURRENT_KNOWLEDGE_VERSION.to_string())
        .await
        .map_err(|e| format!("Failed to write version marker: {}", e))?;

    Ok(())
}
```

#### Co-pilot log backfill (simplified)

For existing recording gems, co-pilot data is already in `gem.source_meta["copilot"]` (see `save_recording_gem` command at line 970–972 in commands.rs: `existing.source_meta["copilot"] = copilot;`).

So during migration, if a gem has `source_meta.copilot`, we should write `copilot.md`. The simplest approach is to handle this inside `write_all_subfiles()` in `local_store.rs` rather than a separate backfill step.

**Add to `write_all_subfiles()` in `local_store.rs`, after the transcript write block:**

```rust
// Write copilot.md (if copilot data exists in source_meta)
if let Some(copilot_data) = gem.source_meta.get("copilot") {
    if !copilot_data.is_null() {
        let formatted = assembler::format_copilot(copilot_data);
        if !formatted.is_empty() {
            tokio::fs::write(folder.join("copilot.md"), &formatted)
                .await
                .map_err(|e| format!("Failed to write copilot.md: {}", e))?;
        }
    }
}
```

This handles both migration (existing gems with copilot data) and new recording gems. No separate backfill function needed.

---

## Part B: Tauri Command Handlers (Task 12)

### Create `src/knowledge/commands.rs`

```rust
use std::sync::Arc;
use tauri::State;
use crate::gems::GemStore;
use crate::knowledge::store::{KnowledgeEntry, KnowledgeStore};
use crate::intelligence::provider::AvailabilityResult;

#[tauri::command]
pub async fn get_gem_knowledge(
    gem_id: String,
    knowledge_store: State<'_, Arc<dyn KnowledgeStore>>,
) -> Result<Option<KnowledgeEntry>, String> {
    knowledge_store.get(&gem_id).await
}

#[tauri::command]
pub async fn get_gem_knowledge_assembled(
    gem_id: String,
    knowledge_store: State<'_, Arc<dyn KnowledgeStore>>,
) -> Result<Option<String>, String> {
    knowledge_store.get_assembled(&gem_id).await
}

#[tauri::command]
pub async fn regenerate_gem_knowledge(
    gem_id: String,
    knowledge_store: State<'_, Arc<dyn KnowledgeStore>>,
    gem_store: State<'_, Arc<dyn GemStore>>,
) -> Result<KnowledgeEntry, String> {
    let gem = gem_store.get(&gem_id).await?
        .ok_or_else(|| format!("Gem '{}' not found", gem_id))?;
    knowledge_store.create(&gem).await
}

#[tauri::command]
pub async fn check_knowledge_availability(
    knowledge_store: State<'_, Arc<dyn KnowledgeStore>>,
) -> Result<AvailabilityResult, String> {
    Ok(knowledge_store.check_availability().await)
}
```

---

## Part C: Register in lib.rs (Task 13)

### Add to setup closure (after GemStore registration, before IntelProvider)

```rust
// Initialize Knowledge Store
let app_data_dir = app.path().app_data_dir()
    .map_err(|e| format!("Failed to get app data dir: {}", e))?;
let knowledge_path = app_data_dir.join("knowledge");
let knowledge_event_emitter = Arc::new(
    crate::knowledge::store::TauriKnowledgeEventEmitter::new(app.handle().clone())
) as Arc<dyn crate::knowledge::store::KnowledgeEventEmitter + Send + Sync>;
let knowledge_store = crate::knowledge::LocalKnowledgeStore::new(
    knowledge_path.clone(),
    knowledge_event_emitter.clone(),
);
let knowledge_store_arc = Arc::new(knowledge_store) as Arc<dyn crate::knowledge::KnowledgeStore>;
app.manage(knowledge_store_arc.clone());
```

### Spawn migration check (after all state registration, before Ok(()))

```rust
// Run knowledge migration in background (non-blocking)
let ks_clone = knowledge_store_arc.clone();
let gs_clone: Arc<dyn GemStore> = app.state::<Arc<dyn GemStore>>().inner().clone();
let ee_clone = knowledge_event_emitter.clone();
let kp_clone = knowledge_path.clone();
tauri::async_runtime::spawn(async move {
    if let Err(e) = crate::knowledge::migration::check_and_run_migration(
        ks_clone.as_ref(),
        gs_clone.as_ref(),
        ee_clone.as_ref(),
        &kp_clone,
    ).await {
        eprintln!("Knowledge migration error: {}", e);
    }
});
```

### Register knowledge commands in invoke_handler

Add to the `tauri::generate_handler![]` list:

```rust
crate::knowledge::commands::get_gem_knowledge,
crate::knowledge::commands::get_gem_knowledge_assembled,
crate::knowledge::commands::regenerate_gem_knowledge,
crate::knowledge::commands::check_knowledge_availability,
```

---

## Part D: Wire into Existing Commands (Tasks 14–15)

The pattern: after each gem lifecycle operation, call the knowledge store. Use `app_handle.try_state()` for **graceful degradation** — if knowledge store isn't registered (impossible in practice, but defensive), don't fail the primary operation.

### Helper pattern to use in commands.rs

```rust
// At the end of a command, after gem_store operation succeeds:
if let Some(ks) = app_handle.try_state::<Arc<dyn KnowledgeStore>>() {
    if let Err(e) = ks.create(&gem).await {
        eprintln!("Knowledge file creation failed: {}", e);
    }
}
```

Note: Some commands already have `app_handle: tauri::AppHandle` as a parameter (like `enrich_gem`). For commands that don't (like `save_gem`, `delete_gem`), you'll need to **add `app_handle: tauri::AppHandle`** as a parameter. Tauri automatically injects `AppHandle` — no caller changes needed.

### Commands to modify:

#### 1. `save_gem` — After successful `gem_store.save(gem)` (line ~302)

Add `app_handle: tauri::AppHandle` parameter to the function signature.

After `let result = gem_store.save(gem).await;` and before returning:

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

#### 2. `enrich_gem` — After successful `gem_store.save(gem)` (line ~667)

Already has `app_handle`. After the final `gem_store.save(gem).await`:

```rust
let result = gem_store.save(gem).await;
if let Ok(ref enriched_gem) = result {
    if let Some(ks) = app_handle.try_state::<Arc<dyn crate::knowledge::KnowledgeStore>>() {
        // Update enrichment subfile
        let formatted = crate::knowledge::assembler::format_enrichment(
            enriched_gem.ai_enrichment.as_ref().unwrap()
        );
        if let Err(e) = ks.update_subfile(&enriched_gem.id, "enrichment.md", &formatted).await {
            eprintln!("Knowledge enrichment update failed: {}", e);
        }
    }
}
result
```

#### 3. `delete_gem` — After successful `gem_store.delete()` (line ~499)

Add `app_handle: tauri::AppHandle` parameter.

```rust
pub async fn delete_gem(
    app_handle: tauri::AppHandle,
    id: String,
    gem_store: State<'_, Arc<dyn GemStore>>,
) -> Result<(), String> {
    gem_store.delete(&id).await?;

    // Delete knowledge files
    if let Some(ks) = app_handle.try_state::<Arc<dyn crate::knowledge::KnowledgeStore>>() {
        if let Err(e) = ks.delete(&id).await {
            eprintln!("Knowledge file deletion failed for gem {}: {}", id, e);
        }
    }

    Ok(())
}
```

#### 4. `transcribe_gem` — After successful save (line ~752)

Add `app_handle: tauri::AppHandle` parameter.

After the final `gem_store.save(gem).await`:

```rust
let result = gem_store.save(gem).await;
if let Ok(ref gem) = result {
    if let Some(ks) = app_handle.try_state::<Arc<dyn crate::knowledge::KnowledgeStore>>() {
        // Recreate all knowledge files (transcript + re-enrichment changes multiple things)
        if let Err(e) = ks.create(gem).await {
            eprintln!("Knowledge file update failed: {}", e);
        }
    }
}
result
```

#### 5. `save_recording_gem` — After successful save (line ~1050 area)

Add `app_handle: tauri::AppHandle` parameter.

After the final `gem_store.save(gem).await`:

```rust
let result = gem_store.save(gem).await;
if let Ok(ref saved_gem) = result {
    if let Some(ks) = app_handle.try_state::<Arc<dyn crate::knowledge::KnowledgeStore>>() {
        if let Err(e) = ks.create(saved_gem).await {
            eprintln!("Knowledge file creation failed for recording gem {}: {}", saved_gem.id, e);
        }
    }
}
result
```

---

## Part E: Module Updates

### Update `src/knowledge/mod.rs`

```rust
pub mod store;
pub mod assembler;
pub mod local_store;
pub mod migration;
pub mod commands;

pub use store::{
    KnowledgeStore, KnowledgeEntry, KnowledgeSubfile,
    MigrationResult, KnowledgeEvent, KnowledgeEventEmitter, GemMeta,
};
pub use local_store::LocalKnowledgeStore;
```

---

## Imports to add in `commands.rs`

At the top of `src/commands.rs`, add:

```rust
use crate::knowledge::KnowledgeStore;
```

You may or may not need this depending on whether you use the fully-qualified path `crate::knowledge::KnowledgeStore` inline or import it.

---

## Verification Checklist

- [ ] `cargo build` succeeds with no errors
- [ ] `migration.rs` compiles — `check_and_run_migration` function
- [ ] `knowledge/commands.rs` compiles — all 4 Tauri commands
- [ ] `lib.rs` registers `Arc<dyn KnowledgeStore>` in managed state
- [ ] `lib.rs` spawns migration check in background
- [ ] `lib.rs` includes all 4 knowledge commands in `generate_handler!`
- [ ] `save_gem` creates knowledge files after save
- [ ] `enrich_gem` updates enrichment.md after enrichment
- [ ] `delete_gem` deletes knowledge folder after delete
- [ ] `transcribe_gem` recreates knowledge files after transcription
- [ ] `save_recording_gem` creates knowledge files (including copilot.md if present)
- [ ] `write_all_subfiles` in `local_store.rs` writes copilot.md from `source_meta.copilot`
- [ ] All knowledge store calls are wrapped in `try_state` + error logging (never fail primary operation)
- [ ] App still builds and starts without errors

---

## Important Notes

- **Graceful degradation is critical.** Knowledge file failures must NEVER block gem save/enrich/delete. Always `eprintln!` the error and continue.
- **`app_handle.try_state::<Arc<dyn KnowledgeStore>>()`** — returns `Option`. Use this pattern, not `.state()` which panics if not registered.
- **Adding `app_handle: tauri::AppHandle` to command signatures** — Tauri injects this automatically. No frontend changes needed. The frontend's `invoke()` calls remain unchanged.
- **`app.path().app_data_dir()`** — this is the Tauri 2.x API for getting the app data directory. Returns `Result<PathBuf>`.
- **Migration runs in background** — `tauri::async_runtime::spawn()`. App startup is not blocked.
- **`GemStore::list()` returns `GemPreview`** (lightweight, truncated content). For migration we need full `Gem` objects, so we list IDs then `get()` each. Not ideal but simple and correct.
- Follow existing code style in commands.rs — verbose doc comments exist on many commands, but for the knowledge wiring just add brief inline comments.
