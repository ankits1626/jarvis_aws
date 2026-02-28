# Gem Knowledge Files — Design Document

## Overview

This design introduces a knowledge file system that generates one folder per gem containing markdown subfiles and an assembled `gem.md`. The system sits behind a `KnowledgeStore` trait (CRUD contract) with `LocalKnowledgeStore` as the v1 filesystem implementation. It integrates with every gem lifecycle event — creation, enrichment, transcription, co-pilot analysis, and deletion — to keep knowledge files in sync with the SQLite database.

### Design Goals

1. **Search-ready**: Every gem gets a single `gem.md` that external tools (QMD, ripgrep, Spotlight) can index
2. **Agent-consumable**: LLM agents read one file per gem instead of assembling from 6+ DB columns
3. **Incrementally updatable**: Individual subfiles (`enrichment.md`, `transcript.md`) are written independently, then `gem.md` is reassembled
4. **Swappable backend**: The `KnowledgeStore` trait allows future implementations (cloud, API, hybrid) without changing any consumer
5. **Concurrent-safe**: Per-gem mutex ensures overlapping writes never corrupt `gem.md`
6. **Non-blocking**: Knowledge file failures never block primary operations (DB save, enrichment, etc.)

### Key Design Decisions

- **Trait-based architecture**: `KnowledgeStore` trait with CRUD contract, following `GemStore`, `IntelProvider`, `SearchProvider` patterns
- **Subfiles + assembled**: Individual subfiles for granular updates, assembled `gem.md` for consumers — both exist simultaneously
- **DB is source of truth**: Knowledge files are derived artifacts, always regenerable via `migrate_all()`
- **Per-gem DashMap mutex**: Concurrent write safety without global locking — different gems write in parallel
- **Event-driven progress**: All operations emit `KnowledgeEvent` via `KnowledgeEventEmitter` for frontend status indicators
- **Idempotent create**: `create()` overwrites existing files, making it safe to retry

### Operational Flow

1. **Gem saved** → `GemStore::save()` completes → `knowledge_store.create(gem)` → folder created, subfiles written, `gem.md` assembled
2. **Enrichment completes** → `knowledge_store.update_subfile(id, "enrichment.md", content)` → subfile written → `gem.md` reassembled
3. **Transcript generated** → `knowledge_store.update_subfile(id, "transcript.md", content)` → subfile written → `gem.md` reassembled
4. **Co-pilot analysis saved** → `knowledge_store.update_subfile(id, "copilot.md", content)` → subfile written → `gem.md` reassembled
5. **Gem deleted** → `GemStore::delete()` completes → `knowledge_store.delete(id)` → entire folder removed
6. **App upgrade (first launch)** → migration detects missing `knowledge/` → batch-generates all files → emits progress events

---

## Architecture

### Module Hierarchy

```
src/knowledge/
├── mod.rs              — Module root, re-exports public types
├── store.rs            — KnowledgeStore trait, KnowledgeEventEmitter trait, all data types
├── local_store.rs      — LocalKnowledgeStore (filesystem implementation)
├── assembler.rs        — assemble_gem_md() and formatting helpers
├── commands.rs         — Tauri command handlers
└── migration.rs        — Initial migration, co-pilot log backfill, version migration
```

This follows the existing module pattern:
- `gems/` has `store.rs` (trait) + `sqlite_store.rs` (impl) + `mod.rs`
- `intelligence/` has `provider.rs` (trait) + `mlx_provider.rs` / `intelligencekit_provider.rs` (impls) + `mod.rs`

### Dependency Graph

```
                     ┌─────────────┐
                     │   lib.rs    │
                     │   (setup)   │
                     └──────┬──────┘
                            │ constructs & registers
                            ▼
                ┌───────────────────────────┐
                │  Arc<dyn KnowledgeStore>  │
                │     (Tauri managed state) │
                └───────────┬───────────────┘
                            │
            ┌───────────────┼───────────────┐
            ▼               ▼               ▼
    ┌──────────────┐ ┌────────────┐ ┌──────────────┐
    │ commands.rs  │ │ gems/      │ │ agents/      │
    │ (Tauri cmds) │ │ commands   │ │ copilot      │
    └──────────────┘ │ (save,     │ └──────────────┘
                     │  enrich,   │
                     │  delete)   │
                     └────────────┘
```

### Data Flow

```
Gem Lifecycle Event
    │
    ├── GemStore::save()        ─── success ──→ knowledge_store.create(gem)
    ├── enrich_gem command      ─── success ──→ knowledge_store.update_subfile("enrichment.md")
    ├── transcribe_gem command  ─── success ──→ knowledge_store.update_subfile("transcript.md")
    ├── copilot agent save      ─── success ──→ knowledge_store.update_subfile("copilot.md")
    └── GemStore::delete()      ─── success ──→ knowledge_store.delete(gem_id)
                                                    │
                                                    ▼
                                            LocalKnowledgeStore
                                                    │
                                        ┌───────────┼───────────┐
                                        ▼           ▼           ▼
                                    write       reassemble    emit
                                    subfile     gem.md        event
                                        │           │           │
                                        ▼           ▼           ▼
                                    knowledge/   knowledge/   "knowledge-progress"
                                    {id}/        {id}/        Tauri event channel
                                    *.md         gem.md
```

### Integration Points

1. **`lib.rs` (setup)**: Construct `LocalKnowledgeStore`, register as `tauri::State<Arc<dyn KnowledgeStore>>`, run migration check
2. **`commands.rs` (existing gem commands)**: After `save_gem`, `enrich_gem`, `transcribe_gem`, `delete_gem` — call knowledge store methods
3. **`agents/copilot.rs`**: After saving co-pilot analysis — call `knowledge_store.update_subfile()`
4. **`knowledge/commands.rs` (new)**: Tauri commands for `get_gem_knowledge`, `regenerate_gem_knowledge`, etc.
5. **Frontend**: Listen to `"knowledge-progress"` events for migration progress bar and sync indicators

### Filesystem Layout

```
~/Library/Application Support/com.jarvis.app/
├── gems.db                              # SQLite — source of truth
├── knowledge/                           # Knowledge files — derived, regenerable
│   ├── .version                         # Version marker (knowledge_version number)
│   ├── 550e8400-e29b-41d4-a716-.../
│   │   ├── meta.json                    # Machine-readable metadata
│   │   ├── content.md                   # Raw extracted content
│   │   ├── enrichment.md               # AI tags + summary
│   │   ├── transcript.md               # Full transcript (if recording)
│   │   ├── copilot.md                  # Co-pilot analysis (if recording)
│   │   └── gem.md                       # Assembled document (primary consumer)
│   ├── 7c9e6d30-abcd-1234-.../
│   │   ├── meta.json
│   │   ├── content.md
│   │   ├── enrichment.md
│   │   └── gem.md
│   └── ...
├── agent_logs/                          # Legacy co-pilot logs (kept as backup)
└── recordings/                          # Audio recordings
```

---

## Modules and Interfaces

### `store.rs` — Trait and Data Types

**File**: `src/knowledge/store.rs`

**Responsibilities**: Define the `KnowledgeStore` trait, `KnowledgeEventEmitter` trait, and all shared data types.

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::intelligence::AvailabilityResult;
use crate::gems::Gem;

/// What a knowledge entry looks like — provider-agnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    /// Gem UUID this entry belongs to
    pub gem_id: String,
    /// The full gem.md content (assembled from subfiles)
    pub assembled: String,
    /// List of subfiles with metadata
    pub subfiles: Vec<KnowledgeSubfile>,
    /// Knowledge format version
    pub version: u32,
    /// ISO 8601 timestamp of last assembly
    pub last_assembled: String,
}

/// Metadata about a single subfile within a gem's knowledge folder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSubfile {
    /// Filename (e.g., "content.md", "enrichment.md")
    pub filename: String,
    /// Whether the file exists on disk
    pub exists: bool,
    /// File size in bytes (0 if not exists)
    pub size_bytes: u64,
    /// ISO 8601 timestamp of last modification (None if not exists)
    pub last_modified: Option<String>,
}

/// Result of a bulk migration operation
#[derive(Debug, Clone, Serialize)]
pub struct MigrationResult {
    pub total: usize,
    pub created: usize,
    pub skipped: usize,
    pub failed: usize,
    pub errors: Vec<(String, String)>, // (gem_id, error message)
}

/// Metadata stored in meta.json — used by the assembler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GemMeta {
    pub id: String,
    pub source_type: String,
    pub source_url: String,
    pub domain: String,
    pub title: String,
    pub author: Option<String>,
    pub captured_at: String,
    pub project_id: Option<String>,
    pub source_meta: serde_json::Value,
    pub knowledge_version: u32,
    pub last_assembled: String,
}

/// Backend-agnostic knowledge store interface
///
/// CRUD contract for gem knowledge files. Implementations can store
/// files locally (filesystem), remotely (S3, GCS), or in any other
/// backend. Consumers never know which.
///
/// Analogous to GemStore (DB), SearchProvider (search), IntelProvider (AI).
#[async_trait]
pub trait KnowledgeStore: Send + Sync {
    /// Check if the knowledge store is available and ready
    async fn check_availability(&self) -> AvailabilityResult;

    // ── CREATE ──────────────────────────────────────────

    /// Create knowledge entry for a new gem
    ///
    /// Generates all subfiles from the gem data and assembles gem.md.
    /// Idempotent — if files already exist, they are overwritten.
    async fn create(&self, gem: &Gem) -> Result<KnowledgeEntry, String>;

    // ── READ ────────────────────────────────────────────

    /// Get the full knowledge entry for a gem (metadata + subfile listing)
    async fn get(&self, gem_id: &str) -> Result<Option<KnowledgeEntry>, String>;

    /// Get the assembled gem.md content
    async fn get_assembled(&self, gem_id: &str) -> Result<Option<String>, String>;

    /// Get a specific subfile's content
    async fn get_subfile(
        &self,
        gem_id: &str,
        filename: &str,
    ) -> Result<Option<String>, String>;

    /// Check if a gem has knowledge files
    async fn exists(&self, gem_id: &str) -> Result<bool, String>;

    // ── UPDATE ──────────────────────────────────────────

    /// Update a specific subfile and reassemble gem.md
    ///
    /// Implementations MUST handle concurrent calls for the same gem_id safely.
    async fn update_subfile(
        &self,
        gem_id: &str,
        filename: &str,
        content: &str,
    ) -> Result<(), String>;

    /// Force reassemble gem.md from existing subfiles
    async fn reassemble(&self, gem_id: &str) -> Result<(), String>;

    // ── DELETE ──────────────────────────────────────────

    /// Delete all knowledge files for a gem
    async fn delete(&self, gem_id: &str) -> Result<(), String>;

    /// Delete a specific subfile and reassemble gem.md
    async fn delete_subfile(
        &self,
        gem_id: &str,
        filename: &str,
    ) -> Result<(), String>;

    // ── BULK ────────────────────────────────────────────

    /// Generate knowledge files for all gems (migration / rebuild)
    async fn migrate_all(
        &self,
        gems: Vec<Gem>,
        event_emitter: &(dyn KnowledgeEventEmitter + Sync),
    ) -> Result<MigrationResult, String>;

    /// List all gem_ids that have knowledge files
    async fn list_indexed(&self) -> Result<Vec<String>, String>;
}

// ── Event Emitter ───────────────────────────────────────

/// Event trait for knowledge file operations
pub trait KnowledgeEventEmitter: Send + Sync {
    fn emit_progress(&self, event: KnowledgeEvent);
}

/// Events emitted during knowledge file operations
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum KnowledgeEvent {
    /// A subfile was created or updated for a gem
    SubfileUpdated {
        gem_id: String,
        filename: String,
        status: String, // "writing", "assembling", "done"
    },
    /// Migration progress for a single gem
    MigrationProgress {
        current: usize,
        total: usize,
        gem_id: String,
        gem_title: String,
        status: String, // "generating", "done", "failed"
    },
    /// Migration completed
    MigrationComplete {
        result: MigrationResult,
    },
}
```

### `local_store.rs` — Filesystem Implementation

**File**: `src/knowledge/local_store.rs`

**Responsibilities**: Implement `KnowledgeStore` for the local filesystem. Manages per-gem write locks, file I/O, and event emission.

```rust
use std::path::{Path, PathBuf};
use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::Mutex;
use tokio::fs;

use crate::gems::Gem;
use crate::intelligence::AvailabilityResult;
use super::store::*;
use super::assembler;

/// Current knowledge file format version
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

/// Filesystem-based knowledge store implementation
pub struct LocalKnowledgeStore {
    /// Root knowledge directory
    /// ~/Library/Application Support/com.jarvis.app/knowledge/
    base_path: PathBuf,
    /// Per-gem locks to serialize concurrent writes to the same gem
    gem_locks: DashMap<String, Arc<Mutex<()>>>,
    /// Event emitter for progress notifications
    event_emitter: Arc<dyn KnowledgeEventEmitter + Send + Sync>,
}

impl LocalKnowledgeStore {
    /// Create a new LocalKnowledgeStore
    ///
    /// # Arguments
    /// * `base_path` - Root knowledge directory (e.g., ~/Library/.../knowledge/)
    /// * `event_emitter` - Event emitter for progress notifications
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

    /// Get the folder path for a gem
    fn gem_folder(&self, gem_id: &str) -> PathBuf {
        self.base_path.join(gem_id)
    }

    /// Acquire the per-gem write lock (lazy-initialized)
    fn get_lock(&self, gem_id: &str) -> Arc<Mutex<()>> {
        self.gem_locks
            .entry(gem_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Build GemMeta from a Gem struct
    fn gem_to_meta(gem: &Gem) -> GemMeta {
        GemMeta {
            id: gem.id.clone(),
            source_type: gem.source_type.clone(),
            source_url: gem.source_url.clone(),
            domain: gem.domain.clone(),
            title: gem.title.clone(),
            author: gem.author.clone(),
            captured_at: gem.captured_at.clone(),
            project_id: None, // Set when project system is active
            source_meta: gem.source_meta.clone(),
            knowledge_version: CURRENT_KNOWLEDGE_VERSION,
            last_assembled: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Read subfile metadata from filesystem
    async fn read_subfile_metadata(
        &self,
        gem_id: &str,
    ) -> Vec<KnowledgeSubfile> {
        let folder = self.gem_folder(gem_id);
        let mut subfiles = Vec::new();

        for &filename in KNOWN_SUBFILES {
            let path = folder.join(filename);
            let (exists, size_bytes, last_modified) = match fs::metadata(&path).await {
                Ok(meta) => {
                    let modified = meta.modified()
                        .ok()
                        .and_then(|t| {
                            let datetime: chrono::DateTime<chrono::Utc> = t.into();
                            Some(datetime.to_rfc3339())
                        });
                    (true, meta.len(), modified)
                }
                Err(_) => (false, 0, None),
            };

            subfiles.push(KnowledgeSubfile {
                filename: filename.to_string(),
                exists,
                size_bytes,
                last_modified,
            });
        }

        subfiles
    }

    /// Internal: write all subfiles from a Gem, then assemble gem.md
    async fn write_all_subfiles(&self, gem: &Gem) -> Result<KnowledgeEntry, String> {
        let folder = self.gem_folder(&gem.id);

        // Ensure directory exists
        fs::create_dir_all(&folder).await
            .map_err(|e| format!("Failed to create knowledge folder: {}", e))?;

        // Write meta.json
        let meta = Self::gem_to_meta(gem);
        let meta_json = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("Failed to serialize meta.json: {}", e))?;
        fs::write(folder.join("meta.json"), &meta_json).await
            .map_err(|e| format!("Failed to write meta.json: {}", e))?;

        // Write content.md (if content exists and is non-empty)
        if let Some(ref content) = gem.content {
            if !content.trim().is_empty() {
                let formatted = assembler::format_content(&gem.title, content);
                fs::write(folder.join("content.md"), &formatted).await
                    .map_err(|e| format!("Failed to write content.md: {}", e))?;
            }
        }

        // Write enrichment.md (if ai_enrichment exists)
        if let Some(ref enrichment) = gem.ai_enrichment {
            let formatted = assembler::format_enrichment(enrichment);
            fs::write(folder.join("enrichment.md"), &formatted).await
                .map_err(|e| format!("Failed to write enrichment.md: {}", e))?;
        }

        // Write transcript.md (if transcript exists and is non-empty)
        if let Some(ref transcript) = gem.transcript {
            if !transcript.trim().is_empty() {
                let language = gem.transcript_language.as_deref().unwrap_or("unknown");
                let formatted = assembler::format_transcript(transcript, language);
                fs::write(folder.join("transcript.md"), &formatted).await
                    .map_err(|e| format!("Failed to write transcript.md: {}", e))?;
            }
        }

        // Assemble gem.md
        let assembled = assembler::assemble_gem_md(&folder, &meta).await?;
        fs::write(folder.join("gem.md"), &assembled).await
            .map_err(|e| format!("Failed to write gem.md: {}", e))?;

        // Update last_assembled in meta.json
        let mut updated_meta = meta.clone();
        updated_meta.last_assembled = chrono::Utc::now().to_rfc3339();
        let updated_meta_json = serde_json::to_string_pretty(&updated_meta)
            .map_err(|e| format!("Failed to serialize updated meta.json: {}", e))?;
        fs::write(folder.join("meta.json"), &updated_meta_json).await
            .map_err(|e| format!("Failed to update meta.json: {}", e))?;

        // Build KnowledgeEntry
        let subfiles = self.read_subfile_metadata(&gem.id).await;
        Ok(KnowledgeEntry {
            gem_id: gem.id.clone(),
            assembled,
            subfiles,
            version: CURRENT_KNOWLEDGE_VERSION,
            last_assembled: updated_meta.last_assembled,
        })
    }

    /// Internal: read meta.json for a gem
    async fn read_meta(&self, gem_id: &str) -> Result<GemMeta, String> {
        let path = self.gem_folder(gem_id).join("meta.json");
        let content = fs::read_to_string(&path).await
            .map_err(|e| format!("Failed to read meta.json: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse meta.json: {}", e))
    }
}

#[async_trait::async_trait]
impl KnowledgeStore for LocalKnowledgeStore {
    async fn check_availability(&self) -> AvailabilityResult {
        // Check if base_path exists or can be created
        match fs::create_dir_all(&self.base_path).await {
            Ok(_) => AvailabilityResult {
                available: true,
                reason: None,
            },
            Err(e) => AvailabilityResult {
                available: false,
                reason: Some(format!("Cannot access knowledge directory: {}", e)),
            },
        }
    }

    async fn create(&self, gem: &Gem) -> Result<KnowledgeEntry, String> {
        let lock = self.get_lock(&gem.id);
        let _guard = lock.lock().await;

        self.event_emitter.emit_progress(KnowledgeEvent::SubfileUpdated {
            gem_id: gem.id.clone(),
            filename: "*".to_string(),
            status: "writing".to_string(),
        });

        let entry = self.write_all_subfiles(gem).await?;

        self.event_emitter.emit_progress(KnowledgeEvent::SubfileUpdated {
            gem_id: gem.id.clone(),
            filename: "gem.md".to_string(),
            status: "done".to_string(),
        });

        Ok(entry)
    }

    async fn get(&self, gem_id: &str) -> Result<Option<KnowledgeEntry>, String> {
        let folder = self.gem_folder(gem_id);
        if !folder.exists() {
            return Ok(None);
        }

        let gem_md_path = folder.join("gem.md");
        if !gem_md_path.exists() {
            return Ok(None);
        }

        let assembled = fs::read_to_string(&gem_md_path).await
            .map_err(|e| format!("Failed to read gem.md: {}", e))?;
        let subfiles = self.read_subfile_metadata(gem_id).await;
        let meta = self.read_meta(gem_id).await?;

        Ok(Some(KnowledgeEntry {
            gem_id: gem_id.to_string(),
            assembled,
            subfiles,
            version: meta.knowledge_version,
            last_assembled: meta.last_assembled,
        }))
    }

    async fn get_assembled(&self, gem_id: &str) -> Result<Option<String>, String> {
        let path = self.gem_folder(gem_id).join("gem.md");
        match fs::read_to_string(&path).await {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("Failed to read gem.md: {}", e)),
        }
    }

    async fn get_subfile(
        &self,
        gem_id: &str,
        filename: &str,
    ) -> Result<Option<String>, String> {
        let path = self.gem_folder(gem_id).join(filename);
        match fs::read_to_string(&path).await {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("Failed to read {}: {}", filename, e)),
        }
    }

    async fn exists(&self, gem_id: &str) -> Result<bool, String> {
        let gem_md = self.gem_folder(gem_id).join("gem.md");
        Ok(gem_md.exists())
    }

    async fn update_subfile(
        &self,
        gem_id: &str,
        filename: &str,
        content: &str,
    ) -> Result<(), String> {
        let lock = self.get_lock(gem_id);
        let _guard = lock.lock().await;

        let folder = self.gem_folder(gem_id);

        // Ensure folder exists (in case update_subfile is called before create)
        fs::create_dir_all(&folder).await
            .map_err(|e| format!("Failed to create knowledge folder: {}", e))?;

        // Emit writing event
        self.event_emitter.emit_progress(KnowledgeEvent::SubfileUpdated {
            gem_id: gem_id.to_string(),
            filename: filename.to_string(),
            status: "writing".to_string(),
        });

        // Write the subfile
        fs::write(folder.join(filename), content).await
            .map_err(|e| format!("Failed to write {}: {}", filename, e))?;

        // Emit assembling event
        self.event_emitter.emit_progress(KnowledgeEvent::SubfileUpdated {
            gem_id: gem_id.to_string(),
            filename: filename.to_string(),
            status: "assembling".to_string(),
        });

        // Reassemble gem.md (lock already held)
        let meta = self.read_meta(gem_id).await?;
        let assembled = assembler::assemble_gem_md(&folder, &meta).await?;
        fs::write(folder.join("gem.md"), &assembled).await
            .map_err(|e| format!("Failed to write gem.md: {}", e))?;

        // Update last_assembled in meta.json
        let mut updated_meta = meta;
        updated_meta.last_assembled = chrono::Utc::now().to_rfc3339();
        let meta_json = serde_json::to_string_pretty(&updated_meta)
            .map_err(|e| format!("Failed to serialize meta.json: {}", e))?;
        fs::write(folder.join("meta.json"), &meta_json).await
            .map_err(|e| format!("Failed to update meta.json: {}", e))?;

        // Emit done event
        self.event_emitter.emit_progress(KnowledgeEvent::SubfileUpdated {
            gem_id: gem_id.to_string(),
            filename: filename.to_string(),
            status: "done".to_string(),
        });

        Ok(())
    }

    async fn reassemble(&self, gem_id: &str) -> Result<(), String> {
        let lock = self.get_lock(gem_id);
        let _guard = lock.lock().await;

        let folder = self.gem_folder(gem_id);
        let meta = self.read_meta(gem_id).await?;
        let assembled = assembler::assemble_gem_md(&folder, &meta).await?;
        fs::write(folder.join("gem.md"), &assembled).await
            .map_err(|e| format!("Failed to write gem.md: {}", e))?;

        // Update last_assembled
        let mut updated_meta = meta;
        updated_meta.last_assembled = chrono::Utc::now().to_rfc3339();
        let meta_json = serde_json::to_string_pretty(&updated_meta)
            .map_err(|e| format!("Failed to serialize meta.json: {}", e))?;
        fs::write(folder.join("meta.json"), &meta_json).await
            .map_err(|e| format!("Failed to update meta.json: {}", e))?;

        Ok(())
    }

    async fn delete(&self, gem_id: &str) -> Result<(), String> {
        let folder = self.gem_folder(gem_id);
        if folder.exists() {
            fs::remove_dir_all(&folder).await
                .map_err(|e| format!("Failed to delete knowledge folder: {}", e))?;
        }
        // Clean up lock entry
        self.gem_locks.remove(gem_id);
        Ok(())
    }

    async fn delete_subfile(
        &self,
        gem_id: &str,
        filename: &str,
    ) -> Result<(), String> {
        let lock = self.get_lock(gem_id);
        let _guard = lock.lock().await;

        let path = self.gem_folder(gem_id).join(filename);
        if path.exists() {
            fs::remove_file(&path).await
                .map_err(|e| format!("Failed to delete {}: {}", filename, e))?;
        }

        // Reassemble gem.md without the deleted subfile (lock already held)
        let folder = self.gem_folder(gem_id);
        let meta = self.read_meta(gem_id).await?;
        let assembled = assembler::assemble_gem_md(&folder, &meta).await?;
        fs::write(folder.join("gem.md"), &assembled).await
            .map_err(|e| format!("Failed to write gem.md: {}", e))?;

        Ok(())
    }

    async fn migrate_all(
        &self,
        gems: Vec<Gem>,
        event_emitter: &(dyn KnowledgeEventEmitter + Sync),
    ) -> Result<MigrationResult, String> {
        // Delegated to migration.rs — see Migration section
        super::migration::run_migration(self, gems, event_emitter).await
    }

    async fn list_indexed(&self) -> Result<Vec<String>, String> {
        let mut gem_ids = Vec::new();
        let mut entries = fs::read_dir(&self.base_path).await
            .map_err(|e| format!("Failed to read knowledge directory: {}", e))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| format!("Failed to read directory entry: {}", e))?
        {
            if entry.file_type().await
                .map(|ft| ft.is_dir())
                .unwrap_or(false)
            {
                if let Some(name) = entry.file_name().to_str() {
                    // Skip hidden files (like .version)
                    if !name.starts_with('.') {
                        gem_ids.push(name.to_string());
                    }
                }
            }
        }

        Ok(gem_ids)
    }
}
```

### `assembler.rs` — gem.md Assembly and Formatting

**File**: `src/knowledge/assembler.rs`

**Responsibilities**: Format individual subfiles from raw data, assemble the consolidated `gem.md` from existing subfiles.

```rust
use std::path::Path;
use tokio::fs;
use super::store::GemMeta;

/// Assemble gem.md from subfiles in a gem's knowledge folder
///
/// Produces the consolidated markdown document by reading existing
/// subfiles and combining them in a fixed order. Sections for
/// non-existent subfiles are omitted entirely.
pub async fn assemble_gem_md(gem_folder: &Path, meta: &GemMeta) -> Result<String, String> {
    let mut doc = String::new();

    // ── Title heading ──
    doc.push_str(&format!("# {}\n\n", meta.title));

    // ── Metadata block ──
    doc.push_str(&format!("- **Source:** {}\n", meta.source_type));
    doc.push_str(&format!("- **URL:** {}\n", meta.source_url));
    if let Some(ref author) = meta.author {
        doc.push_str(&format!("- **Author:** {}\n", author));
    }
    doc.push_str(&format!("- **Captured:** {}\n", &meta.captured_at[..10])); // date only

    // Tags from enrichment.md (if exists)
    if let Ok(enrichment) = read_subfile(gem_folder, "enrichment.md").await {
        let tags = extract_tags(&enrichment);
        if !tags.is_empty() {
            doc.push_str(&format!("- **Tags:** {}\n", tags.join(", ")));
        }
    }

    // Project (if assigned)
    if let Some(ref project_id) = meta.project_id {
        doc.push_str(&format!("- **Project:** {}\n", project_id));
    }

    doc.push('\n');

    // ── Summary (from enrichment.md) ──
    if let Ok(enrichment) = read_subfile(gem_folder, "enrichment.md").await {
        let summary = extract_summary(&enrichment);
        if !summary.is_empty() {
            doc.push_str("## Summary\n\n");
            doc.push_str(&summary);
            doc.push_str("\n\n");
        }
    }

    // ── Content (from content.md) ──
    if let Ok(content) = read_subfile(gem_folder, "content.md").await {
        doc.push_str("## Content\n\n");
        doc.push_str(&content);
        doc.push_str("\n\n");
    }

    // ── Transcript (from transcript.md) ──
    if let Ok(transcript) = read_subfile(gem_folder, "transcript.md").await {
        // transcript.md already has ## Transcript heading
        doc.push_str(&transcript);
        doc.push_str("\n\n");
    }

    // ── Co-Pilot Analysis (from copilot.md) ──
    if let Ok(copilot) = read_subfile(gem_folder, "copilot.md").await {
        doc.push_str("## Co-Pilot Analysis\n\n");
        doc.push_str(&copilot);
        doc.push_str("\n\n");
    }

    Ok(doc.trim_end().to_string())
}

/// Read a subfile, returning Err if it doesn't exist
async fn read_subfile(gem_folder: &Path, filename: &str) -> Result<String, ()> {
    let path = gem_folder.join(filename);
    fs::read_to_string(&path).await.map_err(|_| ())
}

/// Extract tags from enrichment.md content
///
/// Looks for a `## Tags` section and parses bulleted list items
pub fn extract_tags(enrichment: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut in_tags_section = false;

    for line in enrichment.lines() {
        if line.starts_with("## Tags") {
            in_tags_section = true;
            continue;
        }
        if in_tags_section {
            if line.starts_with("## ") {
                break; // Next section
            }
            if let Some(tag) = line.strip_prefix("- ") {
                let tag = tag.trim();
                if !tag.is_empty() {
                    tags.push(tag.to_string());
                }
            }
        }
    }

    tags
}

/// Extract summary text from enrichment.md content
///
/// Looks for a `## Summary` section and returns text until next heading
fn extract_summary(enrichment: &str) -> String {
    let mut summary_lines = Vec::new();
    let mut in_summary_section = false;

    for line in enrichment.lines() {
        if line.starts_with("## Summary") {
            in_summary_section = true;
            continue;
        }
        if in_summary_section {
            if line.starts_with("## ") {
                break;
            }
            summary_lines.push(line);
        }
    }

    summary_lines.join("\n").trim().to_string()
}

// ── Formatting functions ────────────────────────────────

/// Format raw content into content.md
pub fn format_content(title: &str, content: &str) -> String {
    format!("# {}\n\n{}", title, content)
}

/// Format ai_enrichment JSON into enrichment.md
pub fn format_enrichment(enrichment: &serde_json::Value) -> String {
    let mut doc = String::new();

    // Summary
    if let Some(summary) = enrichment.get("summary").and_then(|v| v.as_str()) {
        doc.push_str("## Summary\n\n");
        doc.push_str(summary);
        doc.push_str("\n\n");
    }

    // Tags
    if let Some(tags) = enrichment.get("tags").and_then(|v| v.as_array()) {
        doc.push_str("## Tags\n\n");
        for tag in tags {
            if let Some(tag_str) = tag.as_str() {
                doc.push_str(&format!("- {}\n", tag_str));
            }
        }
        doc.push('\n');
    }

    // Enrichment metadata
    doc.push_str("## Enrichment Metadata\n\n");
    if let Some(provider) = enrichment.get("provider").and_then(|v| v.as_str()) {
        doc.push_str(&format!("- Provider: {}\n", provider));
    }
    if let Some(enriched_at) = enrichment.get("enriched_at").and_then(|v| v.as_str()) {
        doc.push_str(&format!("- Enriched: {}\n", enriched_at));
    }

    doc.trim_end().to_string()
}

/// Format transcript text into transcript.md
pub fn format_transcript(transcript: &str, language: &str) -> String {
    format!("## Transcript\n\nLanguage: {}\n\n{}", language, transcript)
}

/// Format co-pilot analysis into copilot.md
///
/// Accepts the structured co-pilot data and produces markdown
/// with sections for each non-empty category.
pub fn format_copilot(
    summary: Option<&str>,
    key_points: &[String],
    decisions: &[String],
    action_items: &[String],
    open_questions: &[String],
    key_concepts: &[String],
) -> String {
    let mut doc = String::new();

    if let Some(s) = summary {
        if !s.is_empty() {
            doc.push_str("## Rolling Summary\n\n");
            doc.push_str(s);
            doc.push_str("\n\n");
        }
    }

    if !key_points.is_empty() {
        doc.push_str("## Key Points\n\n");
        for point in key_points {
            doc.push_str(&format!("- {}\n", point));
        }
        doc.push('\n');
    }

    if !decisions.is_empty() {
        doc.push_str("## Decisions\n\n");
        for decision in decisions {
            doc.push_str(&format!("- {}\n", decision));
        }
        doc.push('\n');
    }

    if !action_items.is_empty() {
        doc.push_str("## Action Items\n\n");
        for item in action_items {
            doc.push_str(&format!("- {}\n", item));
        }
        doc.push('\n');
    }

    if !open_questions.is_empty() {
        doc.push_str("## Open Questions\n\n");
        for question in open_questions {
            doc.push_str(&format!("- {}\n", question));
        }
        doc.push('\n');
    }

    if !key_concepts.is_empty() {
        doc.push_str("## Key Concepts\n\n");
        for concept in key_concepts {
            doc.push_str(&format!("- {}\n", concept));
        }
        doc.push('\n');
    }

    doc.trim_end().to_string()
}
```

### `commands.rs` — Tauri Command Handlers

**File**: `src/knowledge/commands.rs`

**Responsibilities**: Expose knowledge file operations as Tauri commands for the frontend.

```rust
use std::sync::Arc;
use tauri::State;

use crate::gems::GemStore;
use crate::intelligence::AvailabilityResult;
use super::store::{KnowledgeStore, KnowledgeEntry};

/// Get the full knowledge entry for a gem
#[tauri::command]
pub async fn get_gem_knowledge(
    gem_id: String,
    knowledge_store: State<'_, Arc<dyn KnowledgeStore>>,
) -> Result<Option<KnowledgeEntry>, String> {
    knowledge_store.get(&gem_id).await
}

/// Get the assembled gem.md content
#[tauri::command]
pub async fn get_gem_knowledge_assembled(
    gem_id: String,
    knowledge_store: State<'_, Arc<dyn KnowledgeStore>>,
) -> Result<Option<String>, String> {
    knowledge_store.get_assembled(&gem_id).await
}

/// Force-regenerate all knowledge files for a gem
#[tauri::command]
pub async fn regenerate_gem_knowledge(
    gem_id: String,
    knowledge_store: State<'_, Arc<dyn KnowledgeStore>>,
    gem_store: State<'_, Arc<dyn GemStore>>,
) -> Result<KnowledgeEntry, String> {
    // Read the gem from the database
    let gem = gem_store.get(&gem_id).await?
        .ok_or_else(|| format!("Gem not found: {}", gem_id))?;

    // Regenerate all knowledge files
    knowledge_store.create(&gem).await
}

/// Check if the knowledge store is available
#[tauri::command]
pub async fn check_knowledge_availability(
    knowledge_store: State<'_, Arc<dyn KnowledgeStore>>,
) -> Result<AvailabilityResult, String> {
    Ok(knowledge_store.check_availability().await)
}
```

### `migration.rs` — Migration Logic

**File**: `src/knowledge/migration.rs`

**Responsibilities**: Initial migration (generate knowledge files for all existing gems), co-pilot log backfill, and version migration.

```rust
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::gems::Gem;
use super::store::*;
use super::local_store::CURRENT_KNOWLEDGE_VERSION;

/// Concurrency limit for parallel gem processing during migration
const MIGRATION_CONCURRENCY: usize = 10;

/// Run the migration: generate knowledge files for all gems
///
/// Called by `LocalKnowledgeStore::migrate_all()`. Processes gems in
/// parallel with a concurrency limit, emitting progress events.
pub async fn run_migration(
    store: &super::local_store::LocalKnowledgeStore,
    gems: Vec<Gem>,
    event_emitter: &(dyn KnowledgeEventEmitter + Sync),
) -> Result<MigrationResult, String> {
    let total = gems.len();
    let mut created = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;
    let mut errors: Vec<(String, String)> = Vec::new();

    // Process gems with concurrency limit using chunks
    // (A more sophisticated approach would use tokio::sync::Semaphore,
    //  but sequential-with-progress is simpler and sufficient for v1
    //  given the I/O-bound nature of the work)
    for (i, gem) in gems.iter().enumerate() {
        event_emitter.emit_progress(KnowledgeEvent::MigrationProgress {
            current: i + 1,
            total,
            gem_id: gem.id.clone(),
            gem_title: gem.title.clone(),
            status: "generating".to_string(),
        });

        match store.create(gem).await {
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
                eprintln!("Knowledge migration: Failed for gem {}: {}", gem.id, e);
                event_emitter.emit_progress(KnowledgeEvent::MigrationProgress {
                    current: i + 1,
                    total,
                    gem_id: gem.id.clone(),
                    gem_title: gem.title.clone(),
                    status: "failed".to_string(),
                });
            }
        }
    }

    let result = MigrationResult {
        total,
        created,
        skipped,
        failed,
        errors,
    };

    event_emitter.emit_progress(KnowledgeEvent::MigrationComplete {
        result: result.clone(),
    });

    Ok(result)
}

/// Check if migration is needed and run it if so
///
/// Called during app setup. Checks for version marker file.
pub async fn check_and_run_migration(
    knowledge_base_path: &Path,
    store: &super::local_store::LocalKnowledgeStore,
    gems: Vec<Gem>,
    event_emitter: &(dyn KnowledgeEventEmitter + Sync),
) -> Result<(), String> {
    let version_marker = knowledge_base_path.join(".version");

    if version_marker.exists() {
        // Check version
        let stored_version = fs::read_to_string(&version_marker).await
            .unwrap_or_default()
            .trim()
            .parse::<u32>()
            .unwrap_or(0);

        if stored_version >= CURRENT_KNOWLEDGE_VERSION {
            eprintln!("Knowledge: Files up to date (version {})", stored_version);
            return Ok(());
        }

        // Version outdated — run reassembly migration (subfiles kept, gem.md regenerated)
        eprintln!(
            "Knowledge: Format version outdated ({} < {}), reassembling all gem.md files",
            stored_version, CURRENT_KNOWLEDGE_VERSION
        );
        // For version migration, we just reassemble — don't regenerate subfiles
        let indexed = store.list_indexed().await?;
        for gem_id in &indexed {
            if let Err(e) = store.reassemble(gem_id).await {
                eprintln!("Knowledge: Failed to reassemble {}: {}", gem_id, e);
            }
        }
    } else {
        // No version marker — first-time migration
        eprintln!("Knowledge: First-time migration for {} gems", gems.len());
        let _result = run_migration(store, gems, event_emitter).await?;
    }

    // Write version marker
    fs::create_dir_all(knowledge_base_path).await
        .map_err(|e| format!("Failed to create knowledge directory: {}", e))?;
    fs::write(&version_marker, CURRENT_KNOWLEDGE_VERSION.to_string()).await
        .map_err(|e| format!("Failed to write version marker: {}", e))?;

    Ok(())
}

/// Backfill co-pilot logs from agent_logs/ into knowledge files
///
/// For each recording gem with a recording_filename in source_meta,
/// searches agent_logs/ for a matching log file and copies it to copilot.md.
pub async fn backfill_copilot_logs(
    agent_logs_path: &Path,
    store: &super::local_store::LocalKnowledgeStore,
    gems: &[Gem],
) -> Result<usize, String> {
    if !agent_logs_path.exists() {
        eprintln!("Knowledge: No agent_logs/ directory found, skipping co-pilot backfill");
        return Ok(0);
    }

    // Scan all agent log files once
    let mut log_files: Vec<(PathBuf, String)> = Vec::new();
    let mut entries = fs::read_dir(agent_logs_path).await
        .map_err(|e| format!("Failed to read agent_logs: {}", e))?;

    while let Some(entry) = entries.next_entry().await
        .map_err(|e| format!("Failed to read log entry: {}", e))?
    {
        let path = entry.path();
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            if let Ok(content) = fs::read_to_string(&path).await {
                // Read first 10 lines for header matching
                let header: String = content.lines().take(10).collect::<Vec<_>>().join("\n");
                log_files.push((path, header));
            }
        }
    }

    let mut backfilled = 0;

    for gem in gems {
        // Only process recording gems with a recording_filename
        let recording_filename = gem.source_meta
            .get("recording_filename")
            .and_then(|v| v.as_str());

        let Some(filename) = recording_filename else {
            continue;
        };

        // Check if copilot.md already exists
        if let Ok(Some(_)) = store.get_subfile(&gem.id, "copilot.md").await {
            continue; // Already has co-pilot data
        }

        // Find matching log files
        let matches: Vec<&(PathBuf, String)> = log_files
            .iter()
            .filter(|(_, header)| header.contains(filename))
            .collect();

        let log_content = match matches.len() {
            0 => continue, // No match — skip
            1 => {
                // Exact match
                fs::read_to_string(&matches[0].0).await.ok()
            }
            _ => {
                // Multiple matches — pick closest timestamp to gem.captured_at
                // For v1, just take the first match (simplest)
                fs::read_to_string(&matches[0].0).await.ok()
            }
        };

        if let Some(content) = log_content {
            if let Err(e) = store.update_subfile(&gem.id, "copilot.md", &content).await {
                eprintln!("Knowledge: Failed to backfill copilot for {}: {}", gem.id, e);
            } else {
                backfilled += 1;
            }
        }
    }

    eprintln!("Knowledge: Backfilled {} co-pilot logs", backfilled);
    Ok(backfilled)
}
```

### `mod.rs` — Module Root

**File**: `src/knowledge/mod.rs`

```rust
pub mod store;
pub mod local_store;
pub mod assembler;
pub mod commands;
pub mod migration;

pub use store::{
    KnowledgeStore,
    KnowledgeEntry,
    KnowledgeSubfile,
    MigrationResult,
    KnowledgeEvent,
    KnowledgeEventEmitter,
    GemMeta,
};
pub use local_store::LocalKnowledgeStore;
```

### `TauriKnowledgeEventEmitter` — Tauri Bridge

Defined alongside the setup in `lib.rs` (or in `knowledge/store.rs`):

```rust
/// Tauri-based event emitter that bridges knowledge events to the frontend
pub struct TauriKnowledgeEventEmitter {
    app_handle: tauri::AppHandle,
}

impl TauriKnowledgeEventEmitter {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }
}

impl KnowledgeEventEmitter for TauriKnowledgeEventEmitter {
    fn emit_progress(&self, event: KnowledgeEvent) {
        if let Err(e) = self.app_handle.emit("knowledge-progress", &event) {
            eprintln!("Knowledge: Failed to emit event: {}", e);
        }
    }
}
```

---

## Data Models

### GemMeta (meta.json)

The serialized form written to each gem's `meta.json`:

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "source_type": "YouTube",
  "source_url": "https://youtube.com/watch?v=abc123",
  "domain": "youtube.com",
  "title": "ECS vs EKS Comparison",
  "author": "TechChannel",
  "captured_at": "2026-02-25T14:30:00Z",
  "project_id": null,
  "source_meta": {
    "duration": "22:15",
    "channel": "TechChannel",
    "recording_filename": null
  },
  "knowledge_version": 1,
  "last_assembled": "2026-02-25T15:00:00Z"
}
```

### KnowledgeEntry (API return type)

Returned by `get()` and `create()`:

```json
{
  "gem_id": "550e8400-e29b-41d4-a716-446655440000",
  "assembled": "# ECS vs EKS Comparison\n\n- **Source:** YouTube\n...",
  "subfiles": [
    { "filename": "meta.json", "exists": true, "size_bytes": 312, "last_modified": "2026-02-25T15:00:00Z" },
    { "filename": "content.md", "exists": true, "size_bytes": 4521, "last_modified": "2026-02-25T14:30:00Z" },
    { "filename": "enrichment.md", "exists": true, "size_bytes": 287, "last_modified": "2026-02-25T15:00:00Z" },
    { "filename": "transcript.md", "exists": false, "size_bytes": 0, "last_modified": null },
    { "filename": "copilot.md", "exists": false, "size_bytes": 0, "last_modified": null },
    { "filename": "gem.md", "exists": true, "size_bytes": 5143, "last_modified": "2026-02-25T15:00:00Z" }
  ],
  "version": 1,
  "last_assembled": "2026-02-25T15:00:00Z"
}
```

### Assembled gem.md (example)

A fully assembled `gem.md` for a recording gem with enrichment, transcript, and co-pilot data:

```markdown
# Infrastructure Planning Call — ECS Migration

- **Source:** Recording
- **URL:** recording://recording_1234567890.pcm
- **Captured:** 2026-02-25
- **Tags:** AWS, ECS, Docker, Postgres, Migration
- **Project:** AWS Migration Q1

## Summary
45-minute infrastructure planning session discussing ECS migration strategy,
database migration approach, and timeline for the Q1 AWS migration project.

## Content
Recording captured during team infrastructure planning session.

## Transcript

Language: en

[00:00:00] So let's kick off the infrastructure planning session.
[00:00:05] We've been looking at ECS versus EKS for the past week.
[00:00:12] I think ECS is the right call for us given the team's experience.
...

## Co-Pilot Analysis

## Rolling Summary
The team discussed container orchestration options for the AWS migration,
settling on ECS over EKS due to team experience constraints.

## Key Points
- ECS chosen over EKS for simplicity
- Fargate pricing estimated at $2,400/mo
- Timeline target: March completion

## Decisions
- Use ECS Fargate (not EC2 launch type)
- Skip CloudEndure — workloads already containerized

## Action Items
- Ankit: Evaluate RDS vs Aurora pricing by Friday
- Team: Set up staging VPC by next week

## Open Questions
- VPC peering vs Transit Gateway — needs architect input
- Blue-green deployment strategy for database cutover
```

---

## Correctness Properties

### Property 1: Idempotent Create

*For any* gem, calling `create(gem)` multiple times with the same data should produce identical file contents each time. The second call overwrites the first with the same result.

**Validates: Requirement 5.3**

### Property 2: Reassembly Consistency

*For any* set of subfiles in a gem folder, calling `reassemble()` should produce a `gem.md` that includes exactly the sections corresponding to existing subfiles — no more, no less. Adding a subfile and reassembling should add its section; removing a subfile and reassembling should remove its section.

**Validates: Requirements 4.1, 4.2**

### Property 3: Concurrent Write Safety

*For any* two concurrent `update_subfile()` calls on the same gem_id with different filenames, both subfiles should be written correctly and the final `gem.md` should contain both sections. No data loss or corruption.

**Validates: Requirements 6.1–6.4**

### Property 4: Parallel Gem Independence

*For any* two concurrent write operations on different gem_ids, they should complete independently without blocking each other. The per-gem mutex should not cause cross-gem contention.

**Validates: Requirement 6.4**

### Property 5: Tag Extraction Roundtrip

*For any* `ai_enrichment` JSON with a `tags` array, formatting to `enrichment.md` via `format_enrichment()` then extracting tags via `extract_tags()` should return the same tag list.

**Validates: Requirements 3.6, 4.3**

### Property 6: Assembly Section Order

*For any* gem with all subfiles present, the sections in `gem.md` should always appear in the fixed order: Title → Metadata → Summary → Content → Transcript → Co-Pilot Analysis. Regardless of the order in which subfiles were written.

**Validates: Requirement 4.1**

### Property 7: Migration Completeness

*For any* list of gems passed to `migrate_all()`, the sum `created + skipped + failed` should equal `total`. Every gem should be accounted for.

**Validates: Requirements 8.3, 8.4**

### Property 8: Delete Cleanup

*For any* gem_id, calling `delete()` should remove the entire directory. A subsequent `exists()` call should return `false`, and `get()` should return `Ok(None)`.

**Validates: Requirements 5.10, 5.7**

### Property 9: Meta Timestamp Update

*For any* successful `create()`, `update_subfile()`, or `reassemble()` call, the `last_assembled` field in `meta.json` should be updated to a timestamp within the last few seconds.

**Validates: Requirement 4.6**

### Property 10: Non-Blocking Integration

*For any* failure in `knowledge_store.create()`, the calling `save_gem` command should still succeed (return the saved gem). Knowledge file errors are logged but never propagated as command failures.

**Validates: Requirement 7.8**

---

## Error Handling

### Error Scenarios

1. **Disk full / write permission denied**: `create()` and `update_subfile()` return `Err(String)` with the I/O error message. The caller (gem lifecycle integration) logs this and continues — the gem is saved in the DB regardless.

2. **Missing meta.json during reassemble**: If `reassemble()` is called but `meta.json` doesn't exist (should not happen in normal flow), return `Err` and log. The subfile write succeeds but `gem.md` is not updated. A subsequent `create()` call will regenerate everything.

3. **Corrupted meta.json**: If `meta.json` contains invalid JSON, `read_meta()` returns `Err`. The caller should catch this and either regenerate from DB or log and skip.

4. **Directory doesn't exist during update_subfile**: The method creates the directory if missing (defensive), writes the subfile, but cannot reassemble without `meta.json`. Returns error suggesting a `create()` call is needed first.

5. **Migration interrupted (app killed mid-migration)**: On next launch, `check_and_run_migration()` detects no version marker and re-runs the full migration. `create()` is idempotent, so previously-generated files are safely overwritten.

6. **Concurrent delete + update race**: If `delete()` runs while `update_subfile()` is in progress for the same gem, the per-gem mutex serializes them. The delete will wait for the update to finish (or vice versa). After delete completes, any queued update will fail with "directory not found" which is logged and ignored.

### Error Recovery Strategy

- **All errors return `Result<T, String>`**: Following the existing Jarvis pattern (no custom error types in v1)
- **Integration calls use `if let Err(e) = ... { eprintln!(...) }`**: Never `unwrap()` or `?` propagation from knowledge calls in gem lifecycle code
- **Migration errors are accumulated**: Failed gems are collected in `MigrationResult.errors`, not thrown
- **Regeneration as recovery**: Any corrupted state can be fixed by calling `create()` (from DB data) or `migrate_all()` (for all gems)

---

## Testing Strategy

### Unit Tests

**`assembler.rs` tests** (`src/knowledge/assembler_test.rs` or inline `#[cfg(test)]`):

- `format_content`: Empty content, content with special characters, very long content
- `format_enrichment`: Full enrichment JSON, missing fields, empty tags array, null provider
- `format_transcript`: Different languages, empty transcript, very long transcript
- `format_copilot`: All sections filled, some sections empty, all sections empty
- `extract_tags`: Valid tags section, no tags section, empty tags, tags with whitespace
- `extract_summary`: Valid summary, no summary section, empty summary
- `assemble_gem_md`: All subfiles present, only meta + content, recording gem with all subfiles, no subfiles except meta

**`local_store.rs` tests**:

- `create`: Normal gem, gem with only required fields, gem with all optional fields
- `get` / `get_assembled` / `get_subfile`: Existing gem, non-existent gem, missing specific subfile
- `exists`: Existing gem, non-existent gem, gem folder exists but no gem.md
- `update_subfile`: Normal update, update before create (creates directory)
- `reassemble`: After subfile added, after subfile removed
- `delete`: Existing gem, non-existent gem (no-op), delete then get
- `delete_subfile`: Remove enrichment then verify gem.md no longer has summary/tags
- `list_indexed`: Empty directory, multiple gems, hidden files ignored

**`migration.rs` tests**:

- `run_migration`: Empty gem list, multiple gems, gem that fails (permissions)
- `check_and_run_migration`: No version marker, current version, outdated version
- `backfill_copilot_logs`: No agent_logs directory, matching logs, no matches, multiple matches

### Integration Tests

- **Full lifecycle**: Create gem → save to DB → knowledge files created → enrich gem → enrichment.md updated → gem.md reassembled → delete gem → folder removed
- **Concurrent writes**: Spawn multiple `update_subfile()` tasks for the same gem_id, verify all subfiles exist and gem.md contains all sections
- **Migration end-to-end**: Populate DB with test gems → run migration → verify all knowledge folders created → verify version marker

### Property-Based Tests

Using the `proptest` crate:

```rust
use proptest::prelude::*;

// Property 1: Idempotent create
proptest! {
    #[test]
    fn create_is_idempotent(
        title in "[a-zA-Z0-9 ]{1,100}",
        content in "[a-zA-Z0-9 ]{0,500}",
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let gem = make_test_gem(&title, &content);
            let store = make_test_store();

            let entry1 = store.create(&gem).await.unwrap();
            let entry2 = store.create(&gem).await.unwrap();

            assert_eq!(entry1.assembled, entry2.assembled);
            assert_eq!(entry1.subfiles.len(), entry2.subfiles.len());
        });
    }
}

// Property 5: Tag extraction roundtrip
proptest! {
    #[test]
    fn tag_extraction_roundtrip(
        tags in prop::collection::vec("[a-zA-Z]{1,20}", 0..10),
    ) {
        let enrichment = serde_json::json!({
            "tags": tags,
            "summary": "Test summary",
        });

        let formatted = assembler::format_enrichment(&enrichment);
        let extracted = assembler::extract_tags(&formatted);

        assert_eq!(extracted, tags);
    }
}

// Property 7: Migration completeness
proptest! {
    #[test]
    fn migration_accounts_for_all_gems(
        gem_count in 0usize..50,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let gems: Vec<Gem> = (0..gem_count).map(|i| make_test_gem_n(i)).collect();
            let store = make_test_store();
            let emitter = NoOpEmitter;

            let result = store.migrate_all(gems, &emitter).await.unwrap();

            assert_eq!(result.total, gem_count);
            assert_eq!(result.created + result.skipped + result.failed, gem_count);
        });
    }
}
```

### Manual Testing Checklist

- [ ] Create a gem via the UI → verify `knowledge/{gem_id}/` folder appears with `meta.json`, `content.md`, `gem.md`
- [ ] Enrich the gem → verify `enrichment.md` appears, `gem.md` now includes Summary and Tags
- [ ] Transcribe a recording gem → verify `transcript.md` appears, `gem.md` now includes Transcript section
- [ ] Delete a gem → verify the entire `knowledge/{gem_id}/` folder is removed
- [ ] Fresh install with existing gems → verify migration runs, progress events emitted, all folders created
- [ ] Open `gem.md` in a text editor → verify it's readable and well-formatted markdown
- [ ] Run `regenerate_gem_knowledge` Tauri command → verify files are regenerated

---

## Implementation Notes

### Performance Considerations

1. **Migration speed**: ~50ms per gem (mostly file I/O). 100 gems ≈ 5 seconds. 1000 gems ≈ 50 seconds. The progress events ensure the user sees movement. For v1, sequential processing is acceptable. Future optimization: use `tokio::sync::Semaphore` for bounded parallelism.

2. **Reassembly cost**: Each `update_subfile()` triggers a full reassembly. This reads all subfiles (~5 files) and writes `gem.md`. For a typical gem, this is <10ms. The per-gem mutex ensures only one reassembly runs at a time per gem.

3. **DashMap memory**: Each entry is `(String, Arc<Mutex<()>>)` ≈ 80 bytes. For 10,000 gems, this is ~800KB. Negligible. Entries are cleaned up on `delete()`.

4. **File I/O**: All I/O uses `tokio::fs` for non-blocking operations. File writes use `write()` (atomic on most platforms for small files). For very large transcripts, consider `BufWriter` in a future iteration.

### Registration in lib.rs

The `LocalKnowledgeStore` is constructed and registered alongside existing providers:

```rust
// In lib.rs setup closure, after GemStore initialization:

// Initialize knowledge directory path
let app_data_dir = app.path().app_data_dir()
    .map_err(|e| format!("Failed to get app data dir: {}", e))?;
let knowledge_path = app_data_dir.join("knowledge");

// Create event emitter
let knowledge_emitter = Arc::new(
    knowledge::TauriKnowledgeEventEmitter::new(app.handle().clone())
);

// Create LocalKnowledgeStore
let knowledge_store = knowledge::LocalKnowledgeStore::new(
    knowledge_path.clone(),
    knowledge_emitter.clone(),
);
let knowledge_store_arc = Arc::new(knowledge_store) as Arc<dyn knowledge::KnowledgeStore>;

// Run migration check (async)
let ks_for_migration = knowledge_store_arc.clone();
let gem_store_for_migration = app.state::<Arc<dyn GemStore>>().inner().clone();
let emitter_for_migration = knowledge_emitter.clone();
tauri::async_runtime::spawn(async move {
    // Load all gems for migration
    match gem_store_for_migration.list(10000, 0).await {
        Ok(previews) => {
            // For migration, we need full Gem objects — load them
            // (simplified: in practice, use a dedicated migration query)
            eprintln!("Knowledge: Checking migration for {} gems", previews.len());
        }
        Err(e) => {
            eprintln!("Knowledge: Failed to load gems for migration: {}", e);
        }
    }
});

// Register in Tauri managed state
app.manage(knowledge_store_arc);
```

### Integration with Existing Commands

In existing gem commands (`commands.rs`), add knowledge store calls after primary operations:

```rust
// In save_gem command, after successful DB save:
if let Some(knowledge_store) = app_handle.try_state::<Arc<dyn KnowledgeStore>>() {
    if let Err(e) = knowledge_store.create(&saved_gem).await {
        eprintln!("Warning: Failed to create knowledge files for gem {}: {}", saved_gem.id, e);
        // Don't fail the save — knowledge files are non-critical
    }
}

// In enrich_gem command, after successful enrichment:
if let Some(knowledge_store) = app_handle.try_state::<Arc<dyn KnowledgeStore>>() {
    let formatted = knowledge::assembler::format_enrichment(&enrichment_data);
    if let Err(e) = knowledge_store.update_subfile(&gem_id, "enrichment.md", &formatted).await {
        eprintln!("Warning: Failed to update knowledge enrichment for gem {}: {}", gem_id, e);
    }
}

// In delete_gem command, after successful DB delete:
if let Some(knowledge_store) = app_handle.try_state::<Arc<dyn KnowledgeStore>>() {
    if let Err(e) = knowledge_store.delete(&gem_id).await {
        eprintln!("Warning: Failed to delete knowledge files for gem {}: {}", gem_id, e);
    }
}
```

The pattern `try_state::<Arc<dyn KnowledgeStore>>()` ensures commands work even if the knowledge store isn't registered (graceful degradation).

### Cargo.toml Addition

```toml
[dependencies]
# ... existing dependencies ...
dashmap = "6"  # For per-gem concurrent write locks
```

---

## Migration Plan

### Phase 1: Core Types and Assembler (no integration)

1. Create `src/knowledge/` module with `mod.rs`, `store.rs`
2. Define all traits (`KnowledgeStore`, `KnowledgeEventEmitter`) and data types
3. Implement `assembler.rs` with all formatting and assembly functions
4. Unit test assembler functions thoroughly
5. No integration with gem lifecycle yet

### Phase 2: LocalKnowledgeStore Implementation

1. Implement `local_store.rs` with all CRUD methods
2. Implement `TauriKnowledgeEventEmitter`
3. Unit test all store methods against a temp directory
4. Test concurrent write safety with multiple tokio tasks
5. No integration with gem lifecycle yet

### Phase 3: Migration Logic

1. Implement `migration.rs` with `run_migration()` and `check_and_run_migration()`
2. Implement `backfill_copilot_logs()`
3. Test migration with sample gems in a temp directory
4. Test version checking and format migration

### Phase 4: Tauri Integration

1. Implement `commands.rs` with Tauri command handlers
2. Register `LocalKnowledgeStore` in `lib.rs` setup
3. Add knowledge commands to `invoke_handler`
4. Wire migration check into app startup
5. Add knowledge store calls to existing gem commands (`save_gem`, `enrich_gem`, `delete_gem`, etc.)
6. End-to-end testing with the running app

### Phase 5: Polish and Testing

1. Integration tests for full gem lifecycle
2. Property-based tests with proptest
3. Manual testing checklist
4. Error scenario testing (disk full, permissions, concurrent)

### Rollback Plan

If critical issues are discovered:
1. Remove knowledge store calls from existing commands (one-line removals)
2. Remove knowledge store registration from `lib.rs`
3. No data loss — DB is unaffected, knowledge files can be deleted
4. Module code stays in the codebase but is inactive

---

## Summary

This design introduces a filesystem-based knowledge layer for Jarvis gems, built behind a `KnowledgeStore` CRUD trait. The `LocalKnowledgeStore` implementation writes markdown subfiles to `knowledge/{gem_id}/` and assembles them into a consolidated `gem.md`. Per-gem mutexes ensure concurrent write safety. Progress events bridge to the frontend via Tauri. The entire system is derived from the SQLite database — regenerable at any time — and integrates non-blockingly with every gem lifecycle event. The module follows Jarvis's established patterns (trait-based providers, `Arc<dyn T>` in managed state, fallback chains) and is designed to be swapped for cloud or hybrid backends in the future.
