# Prompt: Implement Gem Knowledge Files — Phase 2

## Your Task

Implement **Phase 2: LocalKnowledgeStore Implementation** (Tasks 5–8 from tasks.md). This phase adds the filesystem-based implementation of the `KnowledgeStore` trait — all CRUD operations, per-gem locking, and the `migrate_all` bulk operation.

**After completing Phase 2, stop and ask me to review before moving to Phase 3.**

If you have any confusion or questions during implementation — about file paths, how data is stored, existing patterns, or edge cases — feel free to ask rather than guessing.

---

## Context

Phase 1 is complete and compiling. You have:
- `src/knowledge/store.rs` — `KnowledgeStore` trait, all data types (`GemMeta`, `KnowledgeEntry`, `KnowledgeSubfile`, `MigrationResult`, `KnowledgeEvent`), `KnowledgeEventEmitter` trait, `TauriKnowledgeEventEmitter`
- `src/knowledge/assembler.rs` — `format_content()`, `format_enrichment()`, `format_transcript()`, `format_copilot()`, `extract_tags()`, `extract_summary()`, `assemble_gem_md()`
- `dashmap = "6"` in Cargo.toml

**Spec:** `.kiro/specs/gem-knowledge-files/requirements.md` (Requirements 5–6 cover Phase 2)
**Tasks:** `.kiro/specs/gem-knowledge-files/tasks.md` (Tasks 5–8)

---

## What Phase 2 Produces

One new file:

```
src/knowledge/
├── mod.rs            ← update re-exports to include LocalKnowledgeStore
├── store.rs          ← (unchanged)
├── assembler.rs      ← (unchanged)
└── local_store.rs    ← NEW — LocalKnowledgeStore implementing KnowledgeStore
```

---

## Existing Patterns

### How state is managed (follow this pattern)

```rust
// lib.rs — GemStore registration pattern
let gem_store = SqliteGemStore::new()
    .map_err(|e| format!("Failed to initialize gem store: {}", e))?;
app.manage(Arc::new(gem_store) as Arc<dyn GemStore>);

// Commands access it via State:
pub async fn save_gem(
    gem_store: State<'_, Arc<dyn GemStore>>,
) -> Result<Gem, String> { ... }
```

The knowledge store will follow the same pattern — constructed in `setup()`, registered as `Arc<dyn KnowledgeStore>`. But that wiring is Phase 4. For now, just implement the struct.

### How Gem looks (what you'll consume)

```rust
pub struct Gem {
    pub id: String,                              // UUID v4
    pub source_type: String,                     // "YouTube", "Article", "Email", etc.
    pub source_url: String,
    pub domain: String,
    pub title: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub source_meta: serde_json::Value,
    pub captured_at: String,                     // ISO 8601
    pub ai_enrichment: Option<serde_json::Value>, // {"tags": [...], "summary": "...", "provider": "...", "enriched_at": "..."}
    pub transcript: Option<String>,
    pub transcript_language: Option<String>,
}
```

### Knowledge folder structure (what you create on disk)

```
{base_path}/
├── {gem_id_1}/
│   ├── meta.json
│   ├── content.md
│   ├── enrichment.md     (if ai_enrichment exists)
│   ├── transcript.md     (if transcript exists)
│   ├── copilot.md        (if copilot data in source_meta)
│   └── gem.md            (assembled from all above)
├── {gem_id_2}/
│   └── ...
└── .version              (migration marker, written by migrate_all)
```

The `base_path` will be `~/Library/Application Support/com.jarvis.app/knowledge/` in production, but the struct just takes a `PathBuf` — it doesn't know the full path.

---

## Task 5: LocalKnowledgeStore Struct + Helpers

### 5.1 — Create `local_store.rs` with struct

```rust
use std::path::PathBuf;
use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::Mutex;
use async_trait::async_trait;

use crate::gems::Gem;
use crate::intelligence::provider::AvailabilityResult;
use crate::knowledge::store::*;
use crate::knowledge::assembler;

pub const CURRENT_KNOWLEDGE_VERSION: u32 = 1;

/// Known subfiles in assembly order
const KNOWN_SUBFILES: &[&str] = &[
    "meta.json",
    "content.md",
    "enrichment.md",
    "transcript.md",
    "copilot.md",
    "gem.md",
];

pub struct LocalKnowledgeStore {
    base_path: PathBuf,
    gem_locks: DashMap<String, Arc<Mutex<()>>>,
    event_emitter: Arc<dyn KnowledgeEventEmitter + Send + Sync>,
}
```

### 5.2 — Constructor

```rust
impl LocalKnowledgeStore {
    pub fn new(
        base_path: PathBuf,
        event_emitter: Arc<dyn KnowledgeEventEmitter + Send + Sync>,
    ) -> Self {
        Self {
            base_path,
            gem_locks: DashMap::new(),
            event_emitter,
        }
    }
}
```

### 5.3 — Helper methods

Implement these private helpers:

- **`gem_folder(&self, gem_id: &str) -> PathBuf`** — returns `{base_path}/{gem_id}`

- **`get_lock(&self, gem_id: &str) -> Arc<Mutex<()>>`** — lazy per-gem lock:
  ```rust
  self.gem_locks
      .entry(gem_id.to_string())
      .or_insert_with(|| Arc::new(Mutex::new(())))
      .clone()
  ```

- **`gem_to_meta(gem: &Gem) -> GemMeta`** — builds `GemMeta` from a `Gem` struct. Set `knowledge_version` to `CURRENT_KNOWLEDGE_VERSION`, `last_assembled` to current ISO 8601 timestamp (use `chrono::Utc::now().to_rfc3339()`), `project_id` to `None` (projects don't exist yet).

- **`read_subfile_metadata(&self, gem_id: &str) -> Vec<KnowledgeSubfile>`** — for each file in `KNOWN_SUBFILES`, check if it exists in the gem folder, get file size and modified time. Return the list.

- **`read_meta(&self, gem_id: &str) -> Result<GemMeta, String>`** — read and parse `{gem_folder}/meta.json` using `tokio::fs::read_to_string` + `serde_json::from_str`.

---

## Task 6: Create and Read Operations

### 6.1 — Internal `write_all_subfiles()` method

This does the actual file writing. Called by `create()` (with lock held).

```rust
async fn write_all_subfiles(&self, gem: &Gem) -> Result<KnowledgeEntry, String> {
    let folder = self.gem_folder(&gem.id);

    // Create directory
    tokio::fs::create_dir_all(&folder).await
        .map_err(|e| format!("Failed to create knowledge folder: {}", e))?;

    // Build meta
    let mut meta = Self::gem_to_meta(gem);

    // Write meta.json
    let meta_json = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("Failed to serialize meta: {}", e))?;
    tokio::fs::write(folder.join("meta.json"), &meta_json).await
        .map_err(|e| format!("Failed to write meta.json: {}", e))?;

    // Write content.md (if content exists and non-empty)
    if let Some(ref content) = gem.content {
        if !content.is_empty() {
            let formatted = assembler::format_content(&gem.title, content);
            tokio::fs::write(folder.join("content.md"), &formatted).await
                .map_err(|e| format!("Failed to write content.md: {}", e))?;
        }
    }

    // Write enrichment.md (if ai_enrichment exists)
    if let Some(ref enrichment) = gem.ai_enrichment {
        let formatted = assembler::format_enrichment(enrichment);
        if !formatted.is_empty() {
            tokio::fs::write(folder.join("enrichment.md"), &formatted).await
                .map_err(|e| format!("Failed to write enrichment.md: {}", e))?;
        }
    }

    // Write transcript.md (if transcript exists and non-empty)
    if let Some(ref transcript) = gem.transcript {
        if !transcript.is_empty() {
            let language = gem.transcript_language.as_deref().unwrap_or("en");
            let formatted = assembler::format_transcript(transcript, language);
            tokio::fs::write(folder.join("transcript.md"), &formatted).await
                .map_err(|e| format!("Failed to write transcript.md: {}", e))?;
        }
    }

    // NOTE: copilot.md is NOT written here from gem data.
    // Co-pilot data lives in gem.source_meta for recording gems, but it's
    // written via update_subfile() during co-pilot agent save flow (Phase 4).
    // During migration (migrate_all), co-pilot backfill handles this separately.

    // Assemble gem.md
    let assembled = assembler::assemble_gem_md(&folder, &meta).await?;
    tokio::fs::write(folder.join("gem.md"), &assembled).await
        .map_err(|e| format!("Failed to write gem.md: {}", e))?;

    // Update last_assembled timestamp in meta
    meta.last_assembled = chrono::Utc::now().to_rfc3339();
    let meta_json = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("Failed to serialize meta: {}", e))?;
    tokio::fs::write(folder.join("meta.json"), &meta_json).await
        .map_err(|e| format!("Failed to write meta.json: {}", e))?;

    // Build return value
    let subfiles = self.read_subfile_metadata(&gem.id).await;
    Ok(KnowledgeEntry {
        gem_id: gem.id.clone(),
        assembled,
        subfiles,
        version: CURRENT_KNOWLEDGE_VERSION,
        last_assembled: meta.last_assembled,
    })
}
```

**Important:** Notice `copilot.md` is NOT written from `gem.source_meta` during `create()`. The co-pilot data shape in `source_meta` is complex and is handled by `format_copilot()` during the co-pilot agent flow. If you want to write copilot.md during `create()` for recording gems that have copilot data, check if `gem.source_meta` has a `"copilot_data"` key — if so, pass it to `assembler::format_copilot()` and write. **Ask me if you're unsure about this.**

### 6.2 — Implement `KnowledgeStore::create()`

Acquire per-gem lock, emit events, call `write_all_subfiles()`, release lock. Idempotent (overwrites existing).

### 6.3 — Implement `check_availability()`

Try to create `base_path` directory. Return `AvailabilityResult { available: true, reason: None }` on success.

### 6.4 — Implement Read methods

- **`get()`** — return `Ok(None)` if folder doesn't exist. Otherwise read `gem.md`, subfile metadata, and meta.json. Return `KnowledgeEntry`.
- **`get_assembled()`** — read `{gem_folder}/gem.md`, return `Ok(None)` if file doesn't exist.
- **`get_subfile()`** — read `{gem_folder}/{filename}`, return `Ok(None)` if doesn't exist.
- **`exists()`** — check if `{gem_folder}/gem.md` exists (not just the directory).

---

## Task 7: Update, Delete, and Bulk Operations

### 7.1 — `update_subfile()`

1. Acquire per-gem lock
2. Create folder if missing (defensive)
3. Emit `SubfileUpdated { status: "writing" }`
4. Write the subfile
5. Emit `SubfileUpdated { status: "assembling" }`
6. Read meta.json, call `assemble_gem_md()`, write gem.md
7. Update `last_assembled` in meta.json
8. Emit `SubfileUpdated { status: "done" }`
9. Release lock

### 7.2 — `reassemble()`

Acquire lock, read meta.json, assemble, write gem.md, update meta.json timestamp.

### 7.3 — `delete()` and `delete_subfile()`

- **`delete()`** — remove entire `{base_path}/{gem_id}/` directory. Also remove the DashMap lock entry.
- **`delete_subfile()`** — acquire lock, remove the file, reassemble gem.md, release lock.

### 7.4 — `list_indexed()`

List directories under `base_path`. Return directory names as gem_ids. Skip hidden files (`.version`, `.DS_Store`).

### 7.5 — `migrate_all()`

Process a `Vec<Gem>` sequentially:

```rust
async fn migrate_all(
    &self,
    gems: Vec<Gem>,
    event_emitter: &(dyn KnowledgeEventEmitter + Sync),
) -> Result<MigrationResult, String> {
    let total = gems.len();
    let mut created = 0;
    let mut skipped = 0;
    let mut failed = 0;
    let mut errors = Vec::new();

    for (i, gem) in gems.iter().enumerate() {
        event_emitter.emit_progress(KnowledgeEvent::MigrationProgress {
            current: i + 1,
            total,
            gem_id: gem.id.clone(),
            gem_title: gem.title.clone(),
            status: "generating".to_string(),
        });

        match self.create(gem).await {
            Ok(_) => {
                created += 1;
                event_emitter.emit_progress(KnowledgeEvent::MigrationProgress {
                    current: i + 1,
                    total,
                    gem_id: gem.id.clone(),
                    gem_title: gem.title.clone(),
                    status: "done".to_string(),
                });
            }
            Err(e) => {
                failed += 1;
                errors.push((gem.id.clone(), e.clone()));
                event_emitter.emit_progress(KnowledgeEvent::MigrationProgress {
                    current: i + 1,
                    total,
                    gem_id: gem.id.clone(),
                    gem_title: gem.title.clone(),
                    status: "failed".to_string(),
                });
                // Continue — don't fail the whole migration for one gem
            }
        }
    }

    let result = MigrationResult { total, created, skipped, failed, errors };
    event_emitter.emit_progress(KnowledgeEvent::MigrationComplete {
        result: result.clone(),
    });
    Ok(result)
}
```

Note: `MigrationResult` needs `Clone` derive for this to work. **Check if it's already derived** — if not, add it.

---

## Task 8: Verification Checkpoint

Before asking for review, verify:

- [ ] `cargo build` succeeds with no errors
- [ ] `LocalKnowledgeStore` compiles and implements all `KnowledgeStore` trait methods
- [ ] Module re-export works: add `pub use local_store::LocalKnowledgeStore;` to `mod.rs`
- [ ] All file I/O uses `tokio::fs` (not `std::fs`)
- [ ] Per-gem locks are acquired before every write operation
- [ ] `create()` is idempotent — calling twice produces the same result
- [ ] `delete()` removes the DashMap lock entry too (cleanup)
- [ ] `list_indexed()` skips hidden files

---

## Important Notes

- **This is still Rust-only.** No Tauri command registration, no lib.rs wiring. That's Phase 4.
- **All file I/O must use `tokio::fs`** — this is async code.
- **Per-gem lock pattern:** `let _lock = self.get_lock(gem_id).lock().await;` — the underscore-prefixed variable holds the lock until it drops at end of scope. Don't drop it early.
- **Error handling:** All errors are `String` (matching the trait). Use `.map_err(|e| format!("...: {}", e))` pattern.
- **No `unwrap()` or `expect()` on file operations.** Always propagate errors.
- **`read_subfile_metadata` should be async** — it reads file metadata from disk via `tokio::fs::metadata()`.
- **chrono for timestamps:** `chrono::Utc::now().to_rfc3339()` for ISO 8601 strings. `chrono` is already in Cargo.toml.
- Follow existing code style — no unnecessary comments, doc comments only on public methods.
