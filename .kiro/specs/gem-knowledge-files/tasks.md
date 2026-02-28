# Implementation Plan: Gem Knowledge Files — Per-Gem Filesystem Layer

## Overview

This implementation adds a per-gem knowledge file system that generates markdown subfiles (`meta.json`, `content.md`, `enrichment.md`, `transcript.md`, `copilot.md`) and an assembled `gem.md` for each gem. The system sits behind a `KnowledgeStore` trait (CRUD contract) with `LocalKnowledgeStore` as the v1 filesystem implementation.

Knowledge files are derived artifacts from the SQLite database — always regenerable. They integrate with every gem lifecycle event (save, enrich, transcribe, co-pilot, delete) and provide search-ready, agent-consumable markdown documents. The architecture follows the same trait-based provider pattern used by `GemStore`, `IntelProvider`, and `SearchProvider`.

## Implementation Phases

### Phase 1: Core Types and Assembler (Tasks 1-4)

**Goal:** Define the `KnowledgeStore` trait, all data types, and implement the assembler module. No filesystem I/O yet — pure data formatting and assembly logic.

**Tasks:**
- Task 1: Create knowledge module structure and trait definitions
- Task 2: Implement KnowledgeEventEmitter trait and TauriKnowledgeEventEmitter
- Task 3: Implement assembler formatting functions
- Task 4: Implement assemble_gem_md and extraction helpers

**Validation:** All assembler formatting functions produce correct markdown. Tag extraction roundtrips through format_enrichment → extract_tags. Assembly produces correct section ordering.

---

### Phase 2: LocalKnowledgeStore Implementation (Tasks 5-8)

**Goal:** Implement the filesystem-based knowledge store with CRUD operations and per-gem concurrent write safety.

**Tasks:**
- Task 5: Implement LocalKnowledgeStore struct and helper methods
- Task 6: Implement Create and Read operations
- Task 7: Implement Update, Delete, and Bulk operations
- Task 8: Checkpoint — Verify store operations against a temp directory

**Validation:** All CRUD operations work against a temp directory. Concurrent writes to the same gem are serialized. Different gems write in parallel.

---

### Phase 3: Migration Logic (Tasks 9-11)

**Goal:** Implement initial migration (generate knowledge files for all existing gems), co-pilot log backfill, and version-based format migration.

**Tasks:**
- Task 9: Implement run_migration and check_and_run_migration
- Task 10: Implement co-pilot log backfill
- Task 11: Checkpoint — Verify migration with sample gems

**Validation:** Migration generates files for all gems, emits progress events, and writes version marker. Co-pilot logs are matched and copied correctly.

---

### Phase 4: Tauri Integration (Tasks 12-16)

**Goal:** Register the knowledge store in Tauri managed state, expose commands, and wire into existing gem lifecycle events.

**Tasks:**
- Task 12: Implement Tauri command handlers
- Task 13: Register LocalKnowledgeStore in lib.rs
- Task 14: Wire knowledge store into existing gem commands (save, enrich, delete)
- Task 15: Wire knowledge store into transcript and co-pilot flows
- Task 16: Checkpoint — End-to-end testing with the running app

**Validation:** Knowledge files are created/updated/deleted in sync with gem lifecycle. Commands return correct data. Migration runs on first launch.

---

### Phase 5: Testing and Polish (Tasks 17-19)

**Goal:** Comprehensive testing, property-based tests, and final polish.

**Tasks:**
- Task 17: Write unit tests for assembler
- Task 18: Write unit tests for local_store and migration
- Task 19: Final checkpoint — All tests pass, manual testing complete

**Validation:** All unit, integration, and property-based tests pass. Manual testing checklist complete.

---

## Tasks

### Phase 1: Core Types and Assembler

- [ ] 1. Create knowledge module structure and trait definitions
  - [ ] 1.1 Create `src/knowledge/` directory and `mod.rs`
    - Create `jarvis-app/src-tauri/src/knowledge/mod.rs` with module declarations and re-exports
    - Re-export: `KnowledgeStore`, `KnowledgeEntry`, `KnowledgeSubfile`, `MigrationResult`, `KnowledgeEvent`, `KnowledgeEventEmitter`, `GemMeta`, `LocalKnowledgeStore`
    - _Requirements: 12.1, 12.2_

  - [ ] 1.2 Create `store.rs` with trait and data type definitions
    - Define `KnowledgeStore` trait with `#[async_trait]`, `Send + Sync` bounds
    - Define all CRUD methods: `create`, `get`, `get_assembled`, `get_subfile`, `exists`, `update_subfile`, `reassemble`, `delete`, `delete_subfile`, `migrate_all`, `list_indexed`, `check_availability`
    - Define `KnowledgeEntry` struct (gem_id, assembled, subfiles, version, last_assembled)
    - Define `KnowledgeSubfile` struct (filename, exists, size_bytes, last_modified)
    - Define `MigrationResult` struct (total, created, skipped, failed, errors)
    - Define `GemMeta` struct (id, source_type, source_url, domain, title, author, captured_at, project_id, source_meta, knowledge_version, last_assembled)
    - All structs derive `Debug`, `Clone`, `Serialize`, `Deserialize` (except `MigrationResult` — `Serialize` only)
    - Reuse `AvailabilityResult` from `src/intelligence/provider.rs`
    - _Requirements: 1.1–1.7_

  - [ ] 1.3 Add `knowledge` module to `lib.rs` module declarations
    - Add `pub mod knowledge;` to `src/lib.rs`
    - _Requirements: 12.4_

  - [ ] 1.4 Add `dashmap` dependency to `Cargo.toml`
    - Add `dashmap = "6"` to `[dependencies]`
    - _Requirements: 12.5_

- [ ] 2. Implement KnowledgeEventEmitter trait and TauriKnowledgeEventEmitter
  - [ ] 2.1 Define `KnowledgeEventEmitter` trait in `store.rs`
    - Define trait with `Send + Sync` bounds
    - Define `fn emit_progress(&self, event: KnowledgeEvent)` method
    - _Requirements: 2.1, 2.2_

  - [ ] 2.2 Define `KnowledgeEvent` enum in `store.rs`
    - Add `#[serde(tag = "type")]` attribute
    - Define `SubfileUpdated { gem_id, filename, status }` variant (status: "writing", "assembling", "done")
    - Define `MigrationProgress { current, total, gem_id, gem_title, status }` variant (status: "generating", "done", "failed")
    - Define `MigrationComplete { result: MigrationResult }` variant
    - _Requirements: 2.3_

  - [ ] 2.3 Implement `TauriKnowledgeEventEmitter` struct
    - Wrap `tauri::AppHandle`
    - Implement `KnowledgeEventEmitter` trait — emit on `"knowledge-progress"` channel via `app_handle.emit()`
    - Place in `store.rs` or alongside setup in `lib.rs`
    - _Requirements: 2.4_

- [ ] 3. Implement assembler formatting functions
  - [ ] 3.1 Create `assembler.rs` with `format_content()` function
    - Accept `title: &str` and `content: &str`
    - Return formatted markdown with `# {title}` heading followed by content body
    - _Requirements: 3.5_

  - [ ] 3.2 Implement `format_enrichment()` function
    - Accept `enrichment: &serde_json::Value`
    - Extract and format `## Summary` section from `enrichment.summary`
    - Extract and format `## Tags` section as bulleted list from `enrichment.tags`
    - Format `## Enrichment Metadata` with provider and enriched_at
    - Handle missing fields gracefully (skip sections if absent)
    - _Requirements: 3.6_

  - [ ] 3.3 Implement `format_transcript()` function
    - Accept `transcript: &str` and `language: &str`
    - Return `## Transcript` heading, `Language: {code}` line, and transcript text
    - _Requirements: 3.7_

  - [ ] 3.4 Implement `format_copilot()` function
    - Accept structured co-pilot data (summary, key_points, decisions, action_items, open_questions, key_concepts)
    - Format each non-empty section: `## Rolling Summary`, `## Key Points`, `## Decisions`, `## Action Items`, `## Open Questions`, `## Key Concepts`
    - Omit empty sections entirely
    - _Requirements: 3.8_

- [ ] 4. Implement assemble_gem_md and extraction helpers
  - [ ] 4.1 Implement `extract_tags()` helper
    - Parse `## Tags` section from enrichment.md content
    - Return `Vec<String>` of tag values from bulleted list items
    - Handle: no tags section, empty tags, tags with whitespace
    - _Requirements: 4.3_

  - [ ] 4.2 Implement `extract_summary()` helper
    - Parse `## Summary` section from enrichment.md content
    - Return text between `## Summary` heading and next `##` heading
    - Handle: no summary section, empty summary
    - _Requirements: 4.1_

  - [ ] 4.3 Implement `assemble_gem_md()` async function
    - Accept `gem_folder: &Path` and `meta: &GemMeta`
    - Produce `gem.md` by concatenating sections in fixed order:
      1. Title heading (`# {title}`)
      2. Metadata block (Source, URL, Author, Captured, Tags, Project)
      3. Summary (from enrichment.md, if present)
      4. Content (from content.md, if present)
      5. Transcript (from transcript.md, if present)
      6. Co-Pilot Analysis (from copilot.md, if present)
    - Omit sections for non-existent subfiles entirely
    - Read subfiles from disk via helper `read_subfile()`
    - _Requirements: 4.1–4.5, 4.7_

  - [ ]* 4.4 Write unit tests for assembler functions
    - Test `format_content`: empty content, special characters, long content
    - Test `format_enrichment`: full JSON, missing fields, empty tags
    - Test `format_transcript`: different languages, empty transcript
    - Test `format_copilot`: all sections filled, some empty, all empty
    - Test `extract_tags`: valid tags, no tags section, empty, whitespace
    - Test `extract_summary`: valid summary, no section, empty
    - **Property 5: Tag Extraction Roundtrip** — format_enrichment → extract_tags returns same tags
    - **Property 6: Assembly Section Order** — sections always in fixed order
    - _Requirements: 4.1, 4.3_

### Phase 2: LocalKnowledgeStore Implementation

- [ ] 5. Implement LocalKnowledgeStore struct and helper methods
  - [ ] 5.1 Create `local_store.rs` with struct definition
    - Define `LocalKnowledgeStore` struct with: `base_path` (PathBuf), `gem_locks` (DashMap<String, Arc<Mutex<()>>>), `event_emitter` (Arc<dyn KnowledgeEventEmitter + Send + Sync>)
    - Define `CURRENT_KNOWLEDGE_VERSION: u32 = 1` constant
    - Define `KNOWN_SUBFILES` array with assembly order
    - _Requirements: 5.1, 10.2_

  - [ ] 5.2 Implement `new()` constructor
    - Accept `base_path: PathBuf` and `event_emitter: Arc<dyn KnowledgeEventEmitter + Send + Sync>`
    - Initialize empty `DashMap`
    - _Requirements: 5.1_

  - [ ] 5.3 Implement helper methods
    - `gem_folder(&self, gem_id: &str) -> PathBuf` — returns `{base_path}/{gem_id}`
    - `get_lock(&self, gem_id: &str) -> Arc<Mutex<()>>` — lazy per-gem lock via `DashMap::entry().or_insert_with()`
    - `gem_to_meta(gem: &Gem) -> GemMeta` — builds `GemMeta` from a `Gem` struct
    - `read_subfile_metadata(&self, gem_id: &str) -> Vec<KnowledgeSubfile>` — reads filesystem metadata for all known subfiles
    - `read_meta(&self, gem_id: &str) -> Result<GemMeta, String>` — reads and parses `meta.json`
    - _Requirements: 5.1, 6.5_

- [ ] 6. Implement Create and Read operations
  - [ ] 6.1 Implement `write_all_subfiles()` internal method
    - Create gem directory if it doesn't exist
    - Write `meta.json` from `gem_to_meta(gem)`
    - Write `content.md` if `gem.content` is Some and non-empty (via `format_content()`)
    - Write `enrichment.md` if `gem.ai_enrichment` is Some (via `format_enrichment()`)
    - Write `transcript.md` if `gem.transcript` is Some and non-empty (via `format_transcript()`)
    - Call `assemble_gem_md()` and write `gem.md`
    - Update `last_assembled` in `meta.json`
    - Return `KnowledgeEntry`
    - _Requirements: 5.2_

  - [ ] 6.2 Implement `KnowledgeStore::create()` method
    - Acquire per-gem mutex
    - Emit `SubfileUpdated` event (status: "writing")
    - Call `write_all_subfiles()`
    - Emit `SubfileUpdated` event (status: "done")
    - Release lock
    - Idempotent — overwrites existing files
    - _Requirements: 5.2, 5.3, 6.6_

  - [ ] 6.3 Implement `KnowledgeStore::check_availability()`
    - Try to create `base_path` directory
    - Return `AvailabilityResult { available: true }` on success, with error reason on failure
    - _Requirements: 1.7_

  - [ ] 6.4 Implement Read methods: `get()`, `get_assembled()`, `get_subfile()`, `exists()`
    - `get()`: Return `Ok(None)` if folder/gem.md doesn't exist; read assembled + subfile metadata + meta if it does
    - `get_assembled()`: Read `gem.md` content, `Ok(None)` if not found
    - `get_subfile()`: Read specific file content, `Ok(None)` if not found
    - `exists()`: Check if directory exists AND contains `gem.md`
    - _Requirements: 5.4–5.7_

- [ ] 7. Implement Update, Delete, and Bulk operations
  - [ ] 7.1 Implement `KnowledgeStore::update_subfile()`
    - Acquire per-gem mutex
    - Create folder if missing (defensive)
    - Emit `SubfileUpdated` event (status: "writing")
    - Write the subfile content
    - Emit `SubfileUpdated` event (status: "assembling")
    - Read `meta.json`, call `assemble_gem_md()`, write `gem.md`
    - Update `last_assembled` in `meta.json`
    - Emit `SubfileUpdated` event (status: "done")
    - Release lock
    - _Requirements: 5.8, 6.2, 6.3_

  - [ ] 7.2 Implement `KnowledgeStore::reassemble()`
    - Acquire per-gem mutex
    - Read `meta.json`, call `assemble_gem_md()`, write `gem.md`
    - Update `last_assembled` in `meta.json`
    - _Requirements: 5.9_

  - [ ] 7.3 Implement `KnowledgeStore::delete()` and `delete_subfile()`
    - `delete()`: Remove entire `{base_path}/{gem_id}/` directory, clean up DashMap lock entry
    - `delete_subfile()`: Acquire lock, remove specific file, reassemble `gem.md`, release lock
    - _Requirements: 5.10, 5.11_

  - [ ] 7.4 Implement `KnowledgeStore::list_indexed()`
    - List all directories under `base_path`
    - Skip hidden files (e.g., `.version`)
    - Return directory names as gem_ids
    - _Requirements: 5.12_

- [ ] 8. Checkpoint — Verify store operations against a temp directory
  - Ensure `create()`, `get()`, `update_subfile()`, `reassemble()`, `delete()` all work correctly
  - Verify idempotent create (call `create()` twice, same result)
  - Verify concurrent write safety (two `update_subfile()` on same gem)
  - Verify parallel gem independence (writes to different gems don't block)
  - **Property 1: Idempotent Create**
  - **Property 3: Concurrent Write Safety**
  - **Property 4: Parallel Gem Independence**
  - **Property 8: Delete Cleanup**
  - _Requirements: 5.3, 6.1–6.4_

### Phase 3: Migration Logic

- [ ] 9. Implement run_migration and check_and_run_migration
  - [ ] 9.1 Create `migration.rs` with `run_migration()` function
    - Accept store reference, Vec<Gem>, and event_emitter
    - Process gems sequentially (v1 — future optimization: semaphore-bounded parallelism)
    - Emit `MigrationProgress` event for each gem (status: "generating", then "done" or "failed")
    - Accumulate results in `MigrationResult`
    - Emit `MigrationComplete` event at end
    - Handle failures per-gem — log error, continue to next gem
    - _Requirements: 8.3, 8.4_

  - [ ] 9.2 Implement `check_and_run_migration()` function
    - Check for `knowledge/.version` marker file
    - If no marker: run full migration, write version marker
    - If version outdated: reassemble all `gem.md` (don't regenerate subfiles)
    - If version current: skip (log "up to date")
    - _Requirements: 8.1, 8.2, 8.5, 8.6, 10.1–10.6_

- [ ] 10. Implement co-pilot log backfill
  - [ ] 10.1 Implement `backfill_copilot_logs()` function
    - Scan `agent_logs/` for all `.md` files
    - Read first 10 lines of each log file as header
    - For each recording gem with `recording_filename` in `source_meta`:
      - Search log headers for matching filename
      - If exactly one match: copy content to `copilot.md` via `update_subfile()`
      - If multiple matches: pick first match (v1 simplicity)
      - If no match: skip (no error)
    - Skip gems that already have `copilot.md`
    - Return count of backfilled logs
    - _Requirements: 9.1–9.7_

- [ ] 11. Checkpoint — Verify migration with sample gems
  - Test `run_migration()` with empty gem list, multiple gems, gem that fails
  - Test `check_and_run_migration()`: no version marker, current version, outdated version
  - Test `backfill_copilot_logs()`: no agent_logs dir, matching logs, no matches
  - **Property 7: Migration Completeness** — created + skipped + failed = total
  - _Requirements: 8.3–8.6, 9.1–9.6_

### Phase 4: Tauri Integration

- [ ] 12. Implement Tauri command handlers
  - [ ] 12.1 Create `commands.rs` with `get_gem_knowledge` command
    - Accept `gem_id: String` and `State<Arc<dyn KnowledgeStore>>`
    - Call `knowledge_store.get(gem_id)` and return result
    - _Requirements: 11.1_

  - [ ] 12.2 Implement `get_gem_knowledge_assembled` command
    - Accept `gem_id: String` and `State<Arc<dyn KnowledgeStore>>`
    - Call `knowledge_store.get_assembled(gem_id)` and return result
    - _Requirements: 11.2_

  - [ ] 12.3 Implement `regenerate_gem_knowledge` command
    - Accept `gem_id: String`, `State<Arc<dyn KnowledgeStore>>`, `State<Arc<dyn GemStore>>`
    - Read gem from DB via `gem_store.get(gem_id)`
    - Call `knowledge_store.create(gem)` to force-regenerate
    - Return `KnowledgeEntry`
    - _Requirements: 11.3_

  - [ ] 12.4 Implement `check_knowledge_availability` command
    - Accept `State<Arc<dyn KnowledgeStore>>`
    - Call `knowledge_store.check_availability()` and return result
    - _Requirements: 11.4_

- [ ] 13. Register LocalKnowledgeStore in lib.rs
  - [ ] 13.1 Construct LocalKnowledgeStore in setup closure
    - Resolve `app_data_dir` via `app.path().app_data_dir()`
    - Build `knowledge_path = app_data_dir.join("knowledge")`
    - Create `TauriKnowledgeEventEmitter` with `app.handle().clone()`
    - Create `LocalKnowledgeStore::new(knowledge_path, event_emitter)`
    - Register as `Arc<dyn KnowledgeStore>` in Tauri managed state
    - _Requirements: 12.3_

  - [ ] 13.2 Wire migration check into app startup
    - After knowledge store registration, spawn async migration check
    - Load gems from `GemStore` for migration
    - Call `check_and_run_migration()`
    - Handle errors gracefully (log and continue — app still works without knowledge files)
    - _Requirements: 8.1, 8.2_

  - [ ] 13.3 Register knowledge commands in invoke_handler
    - Add `get_gem_knowledge`, `get_gem_knowledge_assembled`, `regenerate_gem_knowledge`, `check_knowledge_availability` to `tauri::generate_handler![]`
    - _Requirements: 11.5_

- [ ] 14. Wire knowledge store into existing gem commands (save, enrich, delete)
  - [ ] 14.1 Add knowledge file creation to `save_gem` command
    - After successful `gem_store.save()`, call `knowledge_store.create(gem)`
    - Use `app_handle.try_state::<Arc<dyn KnowledgeStore>>()` for graceful degradation
    - Log errors but do not fail the save operation
    - _Requirements: 7.1, 7.8_

  - [ ] 14.2 Add knowledge file update to `enrich_gem` command
    - After successful enrichment, format enrichment data via `knowledge::assembler::format_enrichment()`
    - Call `knowledge_store.update_subfile(gem_id, "enrichment.md", formatted)`
    - Use `try_state` pattern, log errors, do not fail enrichment
    - _Requirements: 7.2, 7.8_

  - [ ] 14.3 Add knowledge file deletion to `delete_gem` command
    - After successful `gem_store.delete()`, call `knowledge_store.delete(gem_id)`
    - Use `try_state` pattern, log errors, do not fail the delete
    - _Requirements: 7.5, 7.8_

- [ ] 15. Wire knowledge store into transcript and co-pilot flows
  - [ ] 15.1 Add knowledge file update to `transcribe_gem` / `transcribe_recording` commands
    - After successful transcription, format transcript via `knowledge::assembler::format_transcript()`
    - Call `knowledge_store.update_subfile(gem_id, "transcript.md", formatted)`
    - Use `try_state` pattern, log errors
    - _Requirements: 7.3, 7.8_

  - [ ] 15.2 Add knowledge file update to co-pilot agent save flow
    - After co-pilot analysis is saved for a recording gem, format via `knowledge::assembler::format_copilot()`
    - Call `knowledge_store.update_subfile(gem_id, "copilot.md", formatted)`
    - Use `try_state` pattern, log errors
    - _Requirements: 7.4, 7.8_

  - [ ] 15.3 Add knowledge file update for `save_recording_gem` command
    - After saving a recording as a gem, call `knowledge_store.create(gem)`
    - If co-pilot data is present in source_meta, also write `copilot.md`
    - Use `try_state` pattern, log errors
    - _Requirements: 7.1, 7.4, 7.8_

- [ ] 16. Checkpoint — End-to-end testing with the running app
  - Test create gem via UI → verify knowledge folder appears
  - Test enrich gem → verify `enrichment.md` updated, `gem.md` reassembled
  - Test transcribe recording → verify `transcript.md` updated
  - Test delete gem → verify knowledge folder removed
  - Test fresh install migration → verify all folders created
  - Test `regenerate_gem_knowledge` command → verify files regenerated
  - _Requirements: All_

### Phase 5: Testing and Polish

- [ ]* 17. Write unit tests for assembler
  - [ ]* 17.1 Test formatting functions
    - `format_content`: empty, special chars, long content
    - `format_enrichment`: full JSON, missing fields, empty tags, null provider
    - `format_transcript`: different languages, empty transcript
    - `format_copilot`: all sections, some empty, all empty
    - _Requirements: 3.5–3.8_

  - [ ]* 17.2 Test extraction and assembly
    - `extract_tags`: valid, no section, empty, whitespace
    - `extract_summary`: valid, no section, empty
    - `assemble_gem_md`: all subfiles present, only meta+content, recording with all, no subfiles
    - **Property 2: Reassembly Consistency** — adding/removing subfile changes gem.md accordingly
    - **Property 5: Tag Extraction Roundtrip** — format_enrichment → extract_tags = same tags
    - **Property 6: Assembly Section Order** — always Title → Metadata → Summary → Content → Transcript → Co-Pilot
    - _Requirements: 4.1–4.3_

- [ ]* 18. Write unit tests for local_store and migration
  - [ ]* 18.1 Test LocalKnowledgeStore CRUD
    - `create`: normal gem, minimal gem, full gem with all optional fields
    - `get` / `get_assembled` / `get_subfile`: existing gem, non-existent, missing subfile
    - `exists`: existing, non-existent, folder without gem.md
    - `update_subfile`: normal update, update before create
    - `reassemble`: after subfile added, after subfile removed
    - `delete`: existing, non-existent (no-op)
    - `delete_subfile`: remove enrichment, verify gem.md updated
    - `list_indexed`: empty dir, multiple gems, hidden files ignored
    - **Property 1: Idempotent Create** — create twice, same assembled content
    - **Property 8: Delete Cleanup** — delete then exists = false, get = None
    - **Property 9: Meta Timestamp Update** — last_assembled within last few seconds
    - _Requirements: 5.1–5.12_

  - [ ]* 18.2 Test concurrent write safety
    - Spawn multiple `update_subfile()` tasks for same gem_id with different filenames
    - Verify all subfiles exist and gem.md contains all sections
    - Verify no data corruption
    - **Property 3: Concurrent Write Safety**
    - **Property 4: Parallel Gem Independence**
    - _Requirements: 6.1–6.4_

  - [ ]* 18.3 Test migration
    - `run_migration`: empty list, multiple gems, gem that fails
    - `check_and_run_migration`: no version marker, current version, outdated version
    - `backfill_copilot_logs`: no agent_logs dir, matching logs, no matches, multiple matches
    - **Property 7: Migration Completeness** — created + skipped + failed = total
    - _Requirements: 8.1–8.6, 9.1–9.6_

  - [ ]* 18.4 Write property-based tests with proptest
    - Idempotent create (Property 1)
    - Tag extraction roundtrip (Property 5)
    - Migration completeness (Property 7)
    - _Requirements: All_

- [ ] 19. Final checkpoint — All tests pass, manual testing complete
  - Run `cargo test` — all tests pass
  - Run `cargo build` — no warnings
  - Manual testing checklist:
    - [ ] Create gem → knowledge folder appears with meta.json, content.md, gem.md
    - [ ] Enrich gem → enrichment.md appears, gem.md includes Summary and Tags
    - [ ] Transcribe recording → transcript.md appears, gem.md includes Transcript section
    - [ ] Delete gem → entire knowledge folder removed
    - [ ] Fresh install migration → all folders created, progress events emitted
    - [ ] Open gem.md in text editor → readable, well-formatted markdown
    - [ ] `regenerate_gem_knowledge` → files regenerated correctly
  - _Requirements: All_

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design document
- The `try_state` pattern in integration tasks ensures graceful degradation — app works even if knowledge store isn't registered
- All file I/O uses `tokio::fs` for non-blocking operations
- The implementation follows a provider-agnostic architecture for future extensibility
