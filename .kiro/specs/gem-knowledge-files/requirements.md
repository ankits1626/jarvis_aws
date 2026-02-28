# Gem Knowledge Files — Per-Gem Filesystem Layer

## Introduction

Jarvis stores gem data in SQLite columns (`title`, `content`, `ai_enrichment`, `transcript`). This works for the app but is not indexable by external tools (QMD, Spotlight), not composable for LLM context assembly, and not debuggable without a DB viewer. Related data is scattered across `gems.db`, `agent_logs/`, and `recordings/`.

This spec defines a **knowledge file system** that generates one folder per gem containing individual markdown subfiles (`content.md`, `enrichment.md`, `transcript.md`, `copilot.md`, `meta.json`) and an assembled `gem.md`. The database remains the source of truth — knowledge files are derived, regenerable artifacts optimized for search indexing, agent consumption, and human inspection.

The system is built behind a `KnowledgeStore` trait (CRUD contract) following the same provider pattern as `IntelProvider`, `SearchProvider`, `GemStore`, and `BrowserAdapter`. The v1 implementation is `LocalKnowledgeStore` (filesystem). The trait enables future swaps to cloud, API, or hybrid backends without changing any consumer code.

**Reference:** Design doc at `discussion/28-feb-next-step/gem-knowledge-files.md`. Downstream consumers: [Gem Search & Knowledge Layer](../../discussion/28-feb-next-step/gem-search-and-knowledge-layer.md), [Projects & Summarizer](../../discussion/28-feb-next-step/projects-and-summarizer.md).

## Glossary

- **Knowledge File**: Any file within a gem's knowledge folder (`knowledge/{gem_id}/`). Includes subfiles and the assembled `gem.md`.
- **Subfile**: An individual markdown file representing one facet of a gem — `content.md`, `enrichment.md`, `transcript.md`, `copilot.md`. Each is independently writable and readable.
- **Assembled Document (`gem.md`)**: The consolidated markdown document built by combining the meta header with all existing subfiles in a fixed order. This is what search engines index and agents consume.
- **`meta.json`**: Machine-readable JSON metadata for the gem (id, source_type, source_url, title, author, captured_at, project_id, knowledge_version, last_assembled). Used by the assembler to generate the `gem.md` header.
- **Knowledge Folder**: The directory `knowledge/{gem_id}/` containing all knowledge files for a single gem.
- **Reassembly**: The process of regenerating `gem.md` from the current subfiles and `meta.json`. Triggered after any subfile write.
- **KnowledgeStore**: The backend-agnostic trait defining the CRUD contract for knowledge file operations. Analogous to `GemStore`, `IntelProvider`, `SearchProvider`.
- **LocalKnowledgeStore**: The v1 filesystem-based implementation of `KnowledgeStore`. Writes to `~/Library/Application Support/com.jarvis.app/knowledge/`.
- **KnowledgeEntry**: The return type from `KnowledgeStore::get()` — contains `gem_id`, `assembled` (gem.md content), `subfiles` listing, `version`, and `last_assembled` timestamp.
- **KnowledgeEventEmitter**: A trait for emitting progress events (subfile updates, migration progress) so the frontend can show status indicators.
- **Migration**: The one-time process of generating knowledge files for all existing gems in SQLite on first launch after upgrade.
- **Co-Pilot Log Backfill**: The migration sub-step that matches existing `agent_logs/` files to recording gems by `recording_filename` in the log header.

## Frozen Design Decisions

These decisions were made during design review (2026-02-28):

1. **Architecture**: `KnowledgeStore` trait with CRUD contract. `LocalKnowledgeStore` (filesystem) is v1. Swappable for cloud, API, or hybrid — same pattern as `IntelProvider`, `SearchProvider`, `GemStore`.
2. **Database is source of truth**: Knowledge files are derived from SQLite data. If files are lost or corrupted, `migrate_all()` regenerates everything. If DB and files disagree, DB wins.
3. **File size limits**: Include full content in `gem.md` for v1. QMD chunks large files anyway. The assembler is designed so a truncation strategy can be added later without changing the trait contract.
4. **Concurrent writes**: Per-gem mutex via `DashMap<String, Arc<Mutex<()>>>` in `LocalKnowledgeStore`. The trait contract requires implementations to handle concurrent `update_subfile()` calls safely.
5. **External edits**: Files are derived from DB, not the other way around. No file watching in v1. `reassemble()` always overwrites `gem.md` from subfiles.
6. **Agent log backfill**: Simple filename match in log header, timestamp tiebreaker for multiple matches. Old `agent_logs/` directory kept as backup.
7. **Storage overhead**: Accepted. Content lives in both DB and files. Can optimize later — `KnowledgeStore` abstraction makes this a localized change.
8. **Progress visibility**: All file operations emit Tauri events via `KnowledgeEventEmitter`. Migration shows a progress bar. Ongoing updates show a subtle sync indicator.
9. **Subfiles + assembled**: Individual subfiles for incremental updates, `gem.md` assembled from them for search/LLM consumption. Both exist simultaneously.
10. **No NoOp for v1**: `LocalKnowledgeStore` is the only implementation shipped. `NoOpKnowledgeStore` is defined in the trait design but not implemented in v1.

---

## Requirement 1: KnowledgeStore Trait and Data Types

**User Story:** As a developer, I need a backend-agnostic trait defining the CRUD contract for knowledge file operations, so all consumers (gem save, enrichment, transcript, co-pilot, search, agents) interact through a stable interface that can be swapped without code changes.

### Acceptance Criteria

1. THE System SHALL define a `KnowledgeStore` trait in `src/knowledge/store.rs` with the `#[async_trait]` attribute and `Send + Sync` bounds
2. THE trait SHALL define the following methods grouped by CRUD operation:
   - **Create:** `create(&self, gem: &Gem) -> Result<KnowledgeEntry, String>`
   - **Read:** `get(&self, gem_id: &str) -> Result<Option<KnowledgeEntry>, String>`
   - **Read:** `get_assembled(&self, gem_id: &str) -> Result<Option<String>, String>`
   - **Read:** `get_subfile(&self, gem_id: &str, filename: &str) -> Result<Option<String>, String>`
   - **Read:** `exists(&self, gem_id: &str) -> Result<bool, String>`
   - **Update:** `update_subfile(&self, gem_id: &str, filename: &str, content: &str) -> Result<(), String>`
   - **Update:** `reassemble(&self, gem_id: &str) -> Result<(), String>`
   - **Delete:** `delete(&self, gem_id: &str) -> Result<(), String>`
   - **Delete:** `delete_subfile(&self, gem_id: &str, filename: &str) -> Result<(), String>`
   - **Bulk:** `migrate_all(&self, gems: Vec<Gem>, event_emitter: &(dyn KnowledgeEventEmitter + Sync)) -> Result<MigrationResult, String>`
   - **Bulk:** `list_indexed(&self) -> Result<Vec<String>, String>`
   - **Availability:** `check_availability(&self) -> AvailabilityResult`
3. THE System SHALL define a `KnowledgeEntry` struct with fields: `gem_id` (String), `assembled` (String — full gem.md content), `subfiles` (Vec<KnowledgeSubfile>), `version` (u32), `last_assembled` (String — ISO 8601)
4. THE System SHALL define a `KnowledgeSubfile` struct with fields: `filename` (String), `exists` (bool), `size_bytes` (u64), `last_modified` (Option<String> — ISO 8601)
5. THE System SHALL define a `MigrationResult` struct with fields: `total` (usize), `created` (usize), `skipped` (usize), `failed` (usize), `errors` (Vec<(String, String)> — gem_id, error message)
6. ALL structs SHALL derive `Debug`, `Clone`, `Serialize`, `Deserialize` (except `MigrationResult` which needs `Serialize` only)
7. THE `KnowledgeStore` trait SHALL reuse `AvailabilityResult` from `src/intelligence/provider.rs`

---

## Requirement 2: KnowledgeEventEmitter Trait

**User Story:** As a user, I want to see progress when knowledge files are being generated or updated, so I know the system is working and can estimate completion time.

### Acceptance Criteria

1. THE System SHALL define a `KnowledgeEventEmitter` trait in `src/knowledge/store.rs` with `Send + Sync` bounds
2. THE trait SHALL define a single method: `fn emit_progress(&self, event: KnowledgeEvent)`
3. THE System SHALL define a `KnowledgeEvent` enum with `#[serde(tag = "type")]` and the following variants:
   - `SubfileUpdated { gem_id: String, filename: String, status: String }` — where status is `"writing"`, `"assembling"`, or `"done"`
   - `MigrationProgress { current: usize, total: usize, gem_id: String, gem_title: String, status: String }` — where status is `"generating"`, `"done"`, or `"failed"`
   - `MigrationComplete { result: MigrationResult }`
4. THE System SHALL implement a `TauriKnowledgeEventEmitter` struct that wraps `tauri::AppHandle` and emits events on the `"knowledge-progress"` channel
5. ALL knowledge file write operations (create, update_subfile, delete, migrate_all) SHALL emit appropriate events through the emitter

---

## Requirement 3: Knowledge Folder Structure

**User Story:** As a developer, I need a consistent folder structure per gem so that subfiles are predictable, the assembler knows what to look for, and external tools can navigate the knowledge directory.

### Acceptance Criteria

1. THE knowledge root directory SHALL be `{app_data_dir}/knowledge/` where `app_data_dir` is `~/Library/Application Support/com.jarvis.app/` on macOS
2. EACH gem SHALL have a folder at `knowledge/{gem_id}/` where `gem_id` is the gem's UUID
3. EACH gem folder SHALL contain the following files (present only if the gem has that data):

   | File | When Created | Content Source |
   |---|---|---|
   | `meta.json` | Always (on create) | Gem struct fields |
   | `content.md` | If `gem.content` is non-empty | `gem.content` |
   | `enrichment.md` | If `gem.ai_enrichment` is non-null | Formatted from `gem.ai_enrichment` JSON |
   | `transcript.md` | If `gem.transcript` is non-empty | `gem.transcript` with language header |
   | `copilot.md` | If co-pilot data exists | Co-pilot agent analysis |
   | `gem.md` | Always (assembled) | Combined from all above |

4. THE `meta.json` SHALL contain: `id`, `source_type`, `source_url`, `domain`, `title`, `author` (nullable), `captured_at`, `project_id` (nullable), `source_meta` (JSON object), `knowledge_version` (integer, starting at 1), `last_assembled` (ISO 8601)
5. THE `content.md` SHALL contain the raw extracted content with a heading derived from the gem title
6. THE `enrichment.md` SHALL contain sections: `## Summary` (from `ai_enrichment.summary`), `## Tags` (bulleted list from `ai_enrichment.tags`), `## Enrichment Metadata` (provider, enriched_at)
7. THE `transcript.md` SHALL contain: `## Transcript` heading, `Language: {code}` line, then the transcript text
8. THE `copilot.md` SHALL contain sections matching the co-pilot output: `## Rolling Summary`, `## Key Points`, `## Decisions`, `## Action Items`, `## Open Questions`, `## Key Concepts` — each section omitted if empty

---

## Requirement 4: gem.md Assembly

**User Story:** As a search engine or LLM agent, I need a single consolidated markdown document per gem so I can index or consume the gem's complete knowledge in one read operation.

### Acceptance Criteria

1. THE assembler SHALL produce `gem.md` by concatenating sections in this fixed order:
   1. Title heading (`# {title}`)
   2. Metadata block (Source, URL, Author, Captured, Tags, Project — as `- **Key:** value` lines)
   3. Summary section (from `enrichment.md`, if present)
   4. Content section (`## Content` + content.md body, if present)
   5. Transcript section (from `transcript.md`, if present)
   6. Co-Pilot Analysis section (`## Co-Pilot Analysis` + copilot.md body, if present)
2. SECTIONS for non-existent subfiles SHALL be omitted entirely — no empty headings
3. THE Tags line in the metadata block SHALL be extracted from `enrichment.md` if it exists, using the `## Tags` section
4. THE Project line in the metadata block SHALL use the `project_id` from `meta.json` if non-null
5. THE assembler SHALL be implemented as an `async fn assemble_gem_md(gem_folder: &Path, meta: &GemMeta) -> Result<String, String>` function
6. AFTER assembly, the `last_assembled` field in `meta.json` SHALL be updated to the current ISO 8601 timestamp
7. THE heading structure SHALL be strict and consistent across all gems to optimize QMD chunking (e.g., always `## Content`, never `## Article Body` or `## Video Description`)

---

## Requirement 5: LocalKnowledgeStore Implementation

**User Story:** As a developer, I need a filesystem-based implementation of `KnowledgeStore` that writes markdown files to disk, so gems have knowledge files on the local machine in v1.

### Acceptance Criteria

1. THE System SHALL implement `LocalKnowledgeStore` struct in `src/knowledge/local_store.rs` with fields: `base_path` (PathBuf — root knowledge directory), `gem_locks` (DashMap<String, Arc<Mutex<()>>> — per-gem write locks), `event_emitter` (Arc<dyn KnowledgeEventEmitter + Send + Sync>)
2. THE `create()` method SHALL:
   a. Create the directory `{base_path}/{gem_id}/` if it doesn't exist
   b. Write `meta.json` from the `Gem` struct fields
   c. Write `content.md` if `gem.content` is `Some` and non-empty
   d. Write `enrichment.md` if `gem.ai_enrichment` is `Some`
   e. Write `transcript.md` if `gem.transcript` is `Some` and non-empty
   f. Assemble and write `gem.md`
   g. Return a `KnowledgeEntry` with the assembled content and subfile listing
3. THE `create()` method SHALL be idempotent — if files already exist, they are overwritten
4. THE `get()` method SHALL return `Ok(None)` if the gem folder doesn't exist, and `Ok(Some(KnowledgeEntry))` with subfile metadata read from the filesystem if it does
5. THE `get_assembled()` method SHALL read and return the contents of `{base_path}/{gem_id}/gem.md`, or `Ok(None)` if the file doesn't exist
6. THE `get_subfile()` method SHALL read and return the contents of `{base_path}/{gem_id}/{filename}`, or `Ok(None)` if the file doesn't exist
7. THE `exists()` method SHALL return `true` if the directory `{base_path}/{gem_id}/` exists and contains a `gem.md` file
8. THE `update_subfile()` method SHALL acquire the per-gem mutex, write the subfile, call `reassemble()`, emit a `SubfileUpdated` event, then release the lock
9. THE `reassemble()` method SHALL read `meta.json`, call `assemble_gem_md()`, write the result to `gem.md`, and update `last_assembled` in `meta.json`
10. THE `delete()` method SHALL remove the entire `{base_path}/{gem_id}/` directory recursively
11. THE `delete_subfile()` method SHALL remove the specific file, then call `reassemble()` so `gem.md` no longer includes that section
12. THE `list_indexed()` method SHALL list all directories under `{base_path}/` and return their names (gem_ids)

---

## Requirement 6: Concurrent Write Safety

**User Story:** As a system, I need to safely handle overlapping writes to the same gem's knowledge files (e.g., enrichment and transcript completing at the same time), so that `gem.md` is never corrupted by racing reassemblies.

### Acceptance Criteria

1. THE `LocalKnowledgeStore` SHALL maintain a `DashMap<String, Arc<Mutex<()>>>` for per-gem write locking
2. EVERY write operation (`update_subfile`, `reassemble`, `delete_subfile`) SHALL acquire the mutex for the target `gem_id` before performing any file I/O
3. THE mutex SHALL be held for the entire write + reassemble cycle — write subfile, read all subfiles, write gem.md, update meta.json — then released
4. DIFFERENT gem_ids SHALL write in parallel without blocking each other — the mutex is per-gem, not global
5. THE `DashMap` entry SHALL be created on first access for a given gem_id (lazy initialization) using `entry().or_insert_with()`
6. THE `create()` method SHALL also acquire the per-gem mutex to prevent races with concurrent `update_subfile()` calls arriving before `create()` completes

---

## Requirement 7: Integration with Gem Lifecycle

**User Story:** As the Jarvis system, I need knowledge files to be automatically created, updated, and deleted in sync with gem lifecycle events, so the knowledge layer stays consistent with the database without manual intervention.

### Acceptance Criteria

1. WHEN `GemStore::save()` completes successfully, THE System SHALL call `knowledge_store.create(gem)` to generate initial knowledge files
2. WHEN the enrichment command completes for a gem, THE System SHALL call `knowledge_store.update_subfile(gem_id, "enrichment.md", formatted_enrichment)` where `formatted_enrichment` is the enrichment data formatted as markdown (Summary, Tags, Enrichment Metadata sections)
3. WHEN the transcript command completes for a gem, THE System SHALL call `knowledge_store.update_subfile(gem_id, "transcript.md", formatted_transcript)` where `formatted_transcript` includes the `## Transcript` heading, `Language:` line, and transcript text
4. WHEN the co-pilot agent saves analysis for a recording gem, THE System SHALL call `knowledge_store.update_subfile(gem_id, "copilot.md", formatted_copilot)` where `formatted_copilot` contains the co-pilot sections in markdown
5. WHEN `GemStore::delete()` completes successfully, THE System SHALL call `knowledge_store.delete(gem_id)` to remove all knowledge files
6. WHEN a gem is assigned to or unassigned from a project, THE System SHALL update `meta.json` (project_id) and call `knowledge_store.reassemble(gem_id)` to refresh the metadata header in `gem.md`
7. ALL integration calls SHALL use the injected `dyn KnowledgeStore` — never a concrete type. The knowledge store is registered in Tauri's managed state alongside `GemStore` and `IntelProvider`
8. FAILURES in knowledge file operations SHALL be logged but SHALL NOT block or fail the primary operation (e.g., a failed `create()` does not roll back `GemStore::save()`)

---

## Requirement 8: Migration of Existing Gems

**User Story:** As an existing Jarvis user upgrading to this version, I need all my existing gems to have knowledge files generated automatically on first launch, so I can immediately benefit from search indexing and agent features.

### Acceptance Criteria

1. ON app launch, THE System SHALL check if the `knowledge/` directory exists and contains a version marker file
2. IF no knowledge directory or no version marker exists, THE System SHALL run the migration process
3. THE migration SHALL:
   a. Read all gems from SQLite via `GemStore::list()` (paginated to avoid memory issues)
   b. For each gem, call `knowledge_store.create(gem)` to generate all knowledge files
   c. Emit `MigrationProgress` events for each gem processed
   d. Emit `MigrationComplete` event with the final `MigrationResult`
   e. Write a version marker file after successful completion
4. THE migration SHALL handle failures gracefully — if a single gem fails, log the error in `MigrationResult.errors` and continue with the next gem
5. THE migration SHALL process gems in parallel where possible (using tokio tasks) with a concurrency limit (e.g., 10 concurrent gem writes) to balance speed and I/O pressure
6. THE version marker SHALL record the `knowledge_version` number so future format changes can trigger re-migration

---

## Requirement 9: Co-Pilot Log Backfill

**User Story:** As an existing Jarvis user with recording gems, I need co-pilot agent logs from `agent_logs/` to be matched and copied into the appropriate gem's `copilot.md` during migration, so recording gems have their co-pilot analysis available in the knowledge layer.

### Acceptance Criteria

1. DURING migration, FOR EACH gem where `source_type` is a recording and `source_meta` contains a `recording_filename`, THE System SHALL search `agent_logs/` for matching co-pilot logs
2. THE matching algorithm SHALL scan agent log files for the `recording_filename` string in the file's header/first 10 lines
3. IF exactly one agent log matches, THE System SHALL copy its content to `copilot.md` in the gem's knowledge folder
4. IF multiple agent logs match, THE System SHALL select the one whose timestamp is closest to `gem.captured_at`
5. IF no agent log matches, THE System SHALL skip `copilot.md` for that gem — no error, no empty file
6. THE old `agent_logs/` directory SHALL NOT be deleted after migration — it is kept as a backup
7. FOR future recordings (post-upgrade), the co-pilot agent SHALL write directly to `knowledge/{gem_id}/copilot.md` via `knowledge_store.update_subfile()` — no backfill needed

---

## Requirement 10: Versioning and Format Evolution

**User Story:** As a developer, I need a versioning mechanism for knowledge files so that when the `gem.md` template changes in future releases, existing files can be detected and regenerated.

### Acceptance Criteria

1. THE `meta.json` for each gem SHALL include a `knowledge_version` field (integer, starting at 1)
2. THE `LocalKnowledgeStore` SHALL define a constant `CURRENT_KNOWLEDGE_VERSION: u32 = 1`
3. ON app launch (after initial migration check), THE System SHALL read the `knowledge_version` from a sample gem's `meta.json`
4. IF the stored version is less than `CURRENT_KNOWLEDGE_VERSION`, THE System SHALL trigger a format migration — reassembling `gem.md` for all gems using the new template
5. THE format migration SHALL reuse the `migrate_all()` infrastructure (progress events, error handling, concurrency limits)
6. THE format migration SHALL NOT regenerate subfiles — only re-run the assembler on existing subfiles to produce updated `gem.md` files with the new template

---

## Requirement 11: Tauri Command Exposure

**User Story:** As the frontend, I need Tauri commands to read knowledge files and trigger regeneration, so I can display knowledge file status and offer manual regeneration controls.

### Acceptance Criteria

1. THE System SHALL expose a `get_gem_knowledge` Tauri command that takes `gem_id: String` and returns `Option<KnowledgeEntry>` by calling `knowledge_store.get(gem_id)`
2. THE System SHALL expose a `get_gem_knowledge_assembled` Tauri command that takes `gem_id: String` and returns `Option<String>` by calling `knowledge_store.get_assembled(gem_id)`
3. THE System SHALL expose a `regenerate_gem_knowledge` Tauri command that takes `gem_id: String`, reads the gem from `GemStore`, and calls `knowledge_store.create(gem)` to force-regenerate all files
4. THE System SHALL expose a `check_knowledge_availability` Tauri command that returns `AvailabilityResult` by calling `knowledge_store.check_availability()`
5. ALL commands SHALL use `tauri::State<Arc<dyn KnowledgeStore>>` for dependency injection, following the existing pattern used by `GemStore` and `IntelProvider`
6. THE frontend SHALL listen to `"knowledge-progress"` events for migration progress and subfile update indicators

---

## Requirement 12: Module Structure and Registration

**User Story:** As a developer, I need the knowledge module to follow Jarvis's existing module patterns (like `gems/`, `intelligence/`) so the codebase remains consistent and the knowledge store is properly registered in the Tauri app state.

### Acceptance Criteria

1. THE System SHALL create a `src/knowledge/` module with the following files:
   - `mod.rs` — module root, re-exports public types
   - `store.rs` — `KnowledgeStore` trait, `KnowledgeEventEmitter` trait, all data types (`KnowledgeEntry`, `KnowledgeSubfile`, `MigrationResult`, `KnowledgeEvent`)
   - `local_store.rs` — `LocalKnowledgeStore` implementation
   - `assembler.rs` — `assemble_gem_md()` function and helper functions (`extract_tags`, `format_enrichment`, `format_transcript`, `format_copilot`)
   - `commands.rs` — Tauri command handlers
   - `migration.rs` — Migration logic (initial migration + co-pilot log backfill + version migration)
2. THE `mod.rs` SHALL re-export: `KnowledgeStore`, `KnowledgeEntry`, `KnowledgeSubfile`, `MigrationResult`, `KnowledgeEvent`, `KnowledgeEventEmitter`, `LocalKnowledgeStore`
3. THE `LocalKnowledgeStore` SHALL be constructed during app setup in `main.rs` and registered as `tauri::State<Arc<dyn KnowledgeStore>>` — same pattern as existing providers
4. THE knowledge module SHALL be added to `src/main.rs` module declarations
5. THE `Cargo.toml` SHALL add `dashmap` as a dependency for the per-gem lock map (if not already present)

---

## Technical Constraints

1. **Rust + Tauri 2.x**: All backend code is Rust. Knowledge store operations are async (tokio). Tauri commands bridge to the frontend.
2. **Trait-based architecture**: Consumers use `dyn KnowledgeStore` — never a concrete type. This is consistent with `GemStore`, `IntelProvider`, `SearchProvider`, `BrowserAdapter`.
3. **Database is source of truth**: Knowledge files are always regenerable from SQLite. File operations never modify the database. If files and DB disagree, DB wins.
4. **No new npm dependencies**: This is entirely a Rust backend feature. Frontend changes are limited to listening for Tauri events.
5. **DashMap for concurrency**: Per-gem locking uses `DashMap<String, Arc<Mutex<()>>>`. No global locks. Different gems write in parallel.
6. **Existing crate dependencies**: Uses `serde`, `serde_json`, `async-trait`, `tokio`, `uuid`, `chrono` (all already in the project). New dependency: `dashmap`.
7. **File I/O via tokio::fs**: All filesystem operations use `tokio::fs` for async I/O, not `std::fs`.
8. **Event-driven progress**: All operations emit events via `KnowledgeEventEmitter`. The Tauri implementation wraps `AppHandle::emit()`.
9. **No file watching**: Knowledge files are write-only from Jarvis's perspective. External edits are not detected. `reassemble()` always overwrites.
10. **Graceful degradation**: Knowledge file failures never block primary operations. A failed `create()` is logged but does not fail `GemStore::save()`.

## Out of Scope

1. `NoOpKnowledgeStore` implementation — defined in trait design but not implemented in v1
2. `CloudKnowledgeStore`, `HybridKnowledgeStore`, or any remote backend — future work
3. File watching for external edits — files are derived, not bidirectionally synced
4. Content truncation or size limits in `gem.md` — full content included in v1
5. Frontend UI for knowledge file management (beyond progress indicators) — no knowledge file browser
6. Subfile plugin/registry pattern for new data types — simple ordered list of known filenames for v1
7. Cleanup of `agent_logs/` after co-pilot log migration — kept as backup indefinitely
8. Knowledge file search — search is handled by `SearchProvider` (separate spec) which indexes `gem.md`
9. `SearchProvider` or QMD integration — separate spec, consumes knowledge files
10. Projects table or `project_id` on gems — separate spec, knowledge files support it via `meta.json` when available
