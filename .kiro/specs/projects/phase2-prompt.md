# Phase 2 Implementation Prompt — SqliteProjectStore: Schema, CRUD, and Gem Associations

## What to Implement

Implement **Phase 2** from `.kiro/specs/projects/tasks.md` — Tasks 3, 4, and 5. This creates the `SqliteProjectStore` struct that implements the `ProjectStore` trait (defined in Phase 1) with full SQLite-backed logic: schema creation, CRUD operations, and gem association management.

After this phase, the entire `ProjectStore` trait is implemented and compiles. It will not yet be wired into `lib.rs` or exposed as Tauri commands — that's Phase 3.

## Context Files (Read These First)

1. **Design doc** (has complete Rust code for `sqlite_store.rs`):
   `.kiro/specs/projects/design.md` — Section: "sqlite_store.rs — SQLite Implementation"

2. **Phase 1 output** (the trait you're implementing):
   `jarvis-app/src-tauri/src/projects/store.rs` — `ProjectStore` trait and all data types

3. **Reference implementation** (follow this pattern for struct, constructor, schema):
   `jarvis-app/src-tauri/src/gems/sqlite_store.rs` — Shows `SqliteGemStore` pattern:
   - `conn: Arc<Mutex<Connection>>` field
   - `new()` constructor that calls `initialize_schema()`
   - `#[cfg(test)] new_in_memory()` for tests
   - `fn initialize_schema(&self) -> Result<(), String>` with `CREATE TABLE IF NOT EXISTS`
   - Lock acquisition pattern: `self.conn.lock().map_err(|e| format!(...))?`

4. **GemPreview column mapping** (how GemPreview is built from raw SQL rows):
   `jarvis-app/src-tauri/src/gems/sqlite_store.rs` — Look at `gem_to_preview()` method (around line 229) for how `tags`, `summary`, `enrichment_source` are extracted from `ai_enrichment` JSON. Your `get()` and `get_project_gems()` methods need to do the same extraction but from raw SQL row values (not a `Gem` struct).

5. **Requirements spec**:
   `.kiro/specs/projects/requirements.md` — Requirements 1 (schema) and 3 (SqliteProjectStore)

## Tasks

### Task 3: Schema and Constructor

Create `jarvis-app/src-tauri/src/projects/sqlite_store.rs` with:

**Struct:**
```rust
pub struct SqliteProjectStore {
    conn: Arc<Mutex<Connection>>,
}
```

**Constructor** — `pub fn new(conn: Arc<Mutex<Connection>>) -> Result<Self, String>`:
- Accepts an **existing** `Arc<Mutex<Connection>>` (unlike `SqliteGemStore::new()` which creates its own connection). This is intentional — `SqliteProjectStore` shares the same `gems.db` connection as `SqliteGemStore`. The wiring happens in Phase 3's `lib.rs` changes.
- Calls `self.initialize_schema()?`

**Test constructor** — `#[cfg(test)] pub fn new_in_memory() -> Result<Self, String>`:
- Creates an in-memory SQLite connection for unit tests
- **Important**: Must also create the `gems` table and `gems_fts` FTS5 virtual table in the in-memory DB, because `get_project_gems()` with search queries JOINs against `gems_fts`. Without these tables, tests for search-within-project will fail. Copy the minimal `CREATE TABLE gems (...)` and `CREATE VIRTUAL TABLE gems_fts USING fts5(...)` from `SqliteGemStore::initialize_schema()`.

**`initialize_schema()`** — create these tables:

```sql
-- Enable foreign keys (required for CASCADE)
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    objective TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS project_gems (
    project_id TEXT NOT NULL,
    gem_id TEXT NOT NULL,
    added_at TEXT NOT NULL,
    PRIMARY KEY (project_id, gem_id),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (gem_id) REFERENCES gems(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_project_gems_gem ON project_gems(gem_id);
CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status);
CREATE INDEX IF NOT EXISTS idx_projects_updated ON projects(updated_at DESC);
```

### Task 4: CRUD Methods

Implement `#[async_trait] impl ProjectStore for SqliteProjectStore`:

**`create()`**:
- Generate UUID v4 via `uuid::Uuid::new_v4().to_string()`
- Set `status = "active"`, timestamps = `chrono::Utc::now().to_rfc3339()`
- INSERT INTO projects
- Return the `Project` struct

**`list()`**:
- SELECT with LEFT JOIN on `project_gems` for `gem_count` (COUNT)
- GROUP BY `p.id`
- ORDER BY `p.updated_at DESC`
- Return `Vec<ProjectPreview>`

**`get()`**:
- Query project by id — return `Err("Project not found".to_string())` if `QueryReturnedNoRows`
- Query associated gems via `INNER JOIN project_gems ON gems.id = project_gems.gem_id WHERE project_gems.project_id = ?` ordered by `added_at DESC`
- Build each `GemPreview` from the row, extracting `tags`, `summary`, `enrichment_source` from the `ai_enrichment` TEXT column. The `ai_enrichment` column stores JSON as TEXT in SQLite. You need to:
  1. Read it as `Option<String>` from the row
  2. Parse it with `serde_json::from_str::<serde_json::Value>()`
  3. Extract `.tags` (array of strings), `.summary` (string), `.provider` + `.model` (combined as enrichment_source)
- Use a helper function `parse_ai_enrichment(json_str: Option<&str>) -> (Option<Vec<String>>, Option<String>, Option<String>)` for this extraction — see the design doc for the complete implementation.
- Return `ProjectDetail { project, gem_count, gems }`

**`update()`**:
- Build a dynamic UPDATE query — only SET columns where the `UpdateProject` field is `Some`
- Always update `updated_at` to current timestamp
- Use parameterized queries with incrementing `?N` placeholders
- Return `Err("Project not found")` if 0 rows affected
- Re-query and return the updated `Project`

**`delete()`**:
- `DELETE FROM projects WHERE id = ?1`
- CASCADE automatically cleans up `project_gems`
- Return `Ok(())`

### Task 5: Gem Association Methods

**`add_gems()`**:
- Loop through `gem_ids`, for each: `INSERT OR IGNORE INTO project_gems (project_id, gem_id, added_at) VALUES (?1, ?2, ?3)`
- `INSERT OR IGNORE` means already-associated gems are silently skipped
- Track count of rows actually inserted (result of each `conn.execute()` — 1 if inserted, 0 if ignored)
- Update project's `updated_at` timestamp
- Return the count

**`remove_gem()`**:
- `DELETE FROM project_gems WHERE project_id = ?1 AND gem_id = ?2`
- Update project's `updated_at` timestamp

**`get_project_gems()`**:
- Three cases:
  1. `query` is `None` — SELECT gems via JOIN, ORDER BY `added_at DESC`, LIMIT (default 100)
  2. `query` is `Some("")` (empty string) — same as None
  3. `query` is `Some("search term")` — add `INNER JOIN gems_fts ON gems_fts.rowid = g.rowid` and `WHERE gems_fts MATCH ?` to filter by FTS5 search, ORDER BY `rank`
- Build `GemPreview` from rows using `parse_ai_enrichment` helper (same as `get()`)
- The SELECT columns for gems should be: `g.id, g.source_type, g.source_url, g.domain, g.title, g.author, g.description, SUBSTR(g.content, 1, 200) as content_preview, g.captured_at, g.ai_enrichment, g.transcript_language`

**`get_gem_projects()`**:
- Query projects via `INNER JOIN project_gems ON p.id = pg.project_id WHERE pg.gem_id = ?1`
- Include subquery `(SELECT COUNT(*) FROM project_gems WHERE project_id = p.id) as gem_count` for each project
- ORDER BY `p.updated_at DESC`
- Return `Vec<ProjectPreview>`

### Update `mod.rs`

After creating `sqlite_store.rs`, update `jarvis-app/src-tauri/src/projects/mod.rs` to add:
- `pub mod sqlite_store;` declaration
- `pub use sqlite_store::SqliteProjectStore;` re-export

## Important Implementation Notes

1. **Imports needed** in `sqlite_store.rs`:
   ```rust
   use rusqlite::Connection;
   use std::sync::{Arc, Mutex};
   use uuid::Uuid;
   use crate::gems::GemPreview;
   use super::store::*;
   ```

2. **`parse_ai_enrichment` helper**: This function parses the `ai_enrichment` JSON TEXT column from SQLite into `(tags, summary, enrichment_source)`. The existing `SqliteGemStore` does similar logic inside `gem_to_preview()` but works with `serde_json::Value` (since `Gem.ai_enrichment` is `Option<serde_json::Value>`). Your version works with `Option<&str>` because you're reading the raw TEXT from SQLite. See the design doc for the exact implementation — it's a standalone `fn`, not a method.

3. **Lock pattern**: Every method acquires the connection lock the same way:
   ```rust
   let conn = self.conn.lock()
       .map_err(|e| format!("Failed to acquire lock: {}", e))?;
   ```

4. **`update()` dynamic query**: This is the most complex method. You need to build a parameterized SQL query at runtime because only `Some` fields are updated. Use `Vec<Box<dyn rusqlite::types::ToSql>>` for dynamic parameters. See the design doc for the complete pattern with incrementing `?N` placeholders.

5. **No changes to `lib.rs`**: Do NOT register `SqliteProjectStore` in `lib.rs` or add Tauri commands yet — that's Phase 3. The store just needs to compile.

6. **No changes to `Cargo.toml`**: All dependencies already exist (`rusqlite`, `uuid`, `chrono`, `serde_json`, `async-trait`).

## Verification

After implementing, run:
```bash
cd jarvis-app/src-tauri && cargo check
```

This must pass. Common compilation issues to watch for:
- Missing `use serde_json;` import (needed for `parse_ai_enrichment`)
- `chrono` usage — the crate is already a dependency. Use `chrono::Utc::now().to_rfc3339()`
- `uuid` usage — already a dependency. Use `uuid::Uuid::new_v4().to_string()`
- Forgetting to update `mod.rs` with `pub mod sqlite_store;`
- The `async_trait` macro — use `#[async_trait::async_trait]` or import `use async_trait::async_trait;` at the top

## What NOT to Do

- Do NOT create `commands.rs` — that's Phase 3
- Do NOT modify `lib.rs` beyond what Phase 1 already did — Phase 3 handles wiring
- Do NOT add `get_conn()` or `from_conn()` to `SqliteGemStore` — Phase 3 handles shared connection
- Do NOT modify `Cargo.toml`
- Do NOT add TypeScript types or frontend code

## After Implementation

Once `cargo check` passes:
1. Show me the complete `sqlite_store.rs` file and any changes to `mod.rs`
2. I'll review before we proceed to Phase 3

**If you have any confusion or questions — about SQL queries, the `parse_ai_enrichment` logic, how `GemPreview` columns map, or the dynamic update query pattern — please ask before guessing. It's better to clarify than to implement something wrong.**
