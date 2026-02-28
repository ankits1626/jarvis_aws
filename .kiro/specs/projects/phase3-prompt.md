# Phase 3 Implementation Prompt — Tauri Commands + lib.rs Wiring

## What to Implement

Implement **Phase 3** from `.kiro/specs/projects/tasks.md` — Tasks 6 and 7. This creates the 9 Tauri command handlers in `commands.rs`, wires the shared database connection so `SqliteProjectStore` shares `gems.db` with `SqliteGemStore`, registers the project store as Tauri managed state, and registers all commands in the `generate_handler!` macro.

After this phase, the entire backend is functional and callable from the frontend via `invoke()`.

## Context Files (Read These First)

1. **Design doc** (has complete Rust code for `commands.rs` and `lib.rs` wiring):
   `.kiro/specs/projects/design.md` — Sections: "commands.rs — Tauri Command Handlers" and "Provider Registration — In lib.rs"

2. **Existing command pattern** (follow this for Tauri command signatures):
   `jarvis-app/src-tauri/src/knowledge/commands.rs` — Shows the pattern: `State<'_, Arc<dyn TraitName>>`, `#[tauri::command]`, `pub async fn`, `Result<T, String>` return type

3. **lib.rs** (where to wire everything):
   `jarvis-app/src-tauri/src/lib.rs` — Read the entire file. Key areas:
   - Lines 51-55: `SqliteGemStore::new()` and `gem_store_arc` creation — **you will modify this**
   - Lines 326-396: `generate_handler![]` macro — **you will add 9 commands here**

4. **SqliteGemStore** (you need to add a `from_conn()` constructor or `get_conn()` method):
   `jarvis-app/src-tauri/src/gems/sqlite_store.rs` — Read lines 1-45 to see the current `new()` and `new_in_memory()` constructors

5. **Phase 2 output** (the store you're wiring):
   `jarvis-app/src-tauri/src/projects/sqlite_store.rs` — `SqliteProjectStore::new(conn)` accepts `Arc<Mutex<Connection>>`

6. **Requirements spec**:
   `.kiro/specs/projects/requirements.md` — Requirements 4 (commands) and 5 (module structure + registration)

## Tasks

### Task 6: Create `src/projects/commands.rs`

Create `jarvis-app/src-tauri/src/projects/commands.rs` with 9 Tauri commands. Each command:
- Is `pub async fn` with `#[tauri::command]` attribute
- Uses `State<'_, Arc<dyn ProjectStore>>` for the project store
- Returns `Result<T, String>` where T is the appropriate type
- Delegates to the `ProjectStore` trait method

**Commands:**

```
#[tauri::command] create_project(title: String, description: Option<String>, objective: Option<String>, project_store: State<'_, Arc<dyn ProjectStore>>) -> Result<Project, String>

#[tauri::command] list_projects(project_store: State<'_, Arc<dyn ProjectStore>>) -> Result<Vec<ProjectPreview>, String>

#[tauri::command] get_project(id: String, project_store: State<'_, Arc<dyn ProjectStore>>) -> Result<ProjectDetail, String>

#[tauri::command] update_project(id: String, title: Option<String>, description: Option<String>, objective: Option<String>, status: Option<String>, project_store: State<'_, Arc<dyn ProjectStore>>) -> Result<Project, String>

#[tauri::command] delete_project(id: String, project_store: State<'_, Arc<dyn ProjectStore>>) -> Result<(), String>

#[tauri::command] add_gems_to_project(project_id: String, gem_ids: Vec<String>, project_store: State<'_, Arc<dyn ProjectStore>>) -> Result<usize, String>

#[tauri::command] remove_gem_from_project(project_id: String, gem_id: String, project_store: State<'_, Arc<dyn ProjectStore>>) -> Result<(), String>

#[tauri::command] get_project_gems(project_id: String, query: Option<String>, limit: Option<usize>, project_store: State<'_, Arc<dyn ProjectStore>>) -> Result<Vec<GemPreview>, String>

#[tauri::command] get_gem_projects(gem_id: String, project_store: State<'_, Arc<dyn ProjectStore>>) -> Result<Vec<ProjectPreview>, String>
```

**Imports needed:**
```rust
use std::sync::Arc;
use tauri::State;
use crate::gems::GemPreview;
use super::store::*;
```

**Note on `get_project_gems`**: The trait method takes `query: Option<&str>` but the Tauri command takes `query: Option<String>`. You need to convert: `query.as_deref()` when calling the trait method.

**Note on `update_project`**: The command takes individual optional fields and constructs `UpdateProject { title, description, objective, status }` to pass to the trait method.

### Task 7: Wire shared DB connection and register commands in lib.rs

This is the trickiest task. You need to:

#### 7a. Add `get_conn()` method to `SqliteGemStore`

In `jarvis-app/src-tauri/src/gems/sqlite_store.rs`, add a public method that exposes the internal database connection:

```rust
/// Get a clone of the underlying database connection Arc.
/// Used by other stores (e.g., SqliteProjectStore) that share the same gems.db.
pub fn get_conn(&self) -> Arc<Mutex<Connection>> {
    self.conn.clone()
}
```

Add this method inside the `impl SqliteGemStore { ... }` block, after `new_in_memory()` and before `initialize_schema()`.

**Why `get_conn()` instead of refactoring `new()`:** The existing `SqliteGemStore::new()` constructor creates its own connection, opens the DB, and calls `initialize_schema()`. Changing this would affect all existing code paths and tests. Adding `get_conn()` is the minimal, non-breaking change. The project store calls `gem_store.get_conn()` to get the same `Arc<Mutex<Connection>>`, ensuring both stores share the exact same database connection.

#### 7b. Update `lib.rs` to create and register `SqliteProjectStore`

In `jarvis-app/src-tauri/src/lib.rs`, add the following **after** the gem store initialization block (after line 55: `app.manage(gem_store_arc.clone());`):

```rust
// Initialize ProjectStore (shares gems.db connection with SqliteGemStore)
let project_store = projects::SqliteProjectStore::new(gem_store.get_conn())
    .map_err(|e| format!("Failed to initialize project store: {}", e))?;
let project_store_arc = Arc::new(project_store) as Arc<dyn projects::ProjectStore>;
app.manage(project_store_arc);
```

**Important**: `gem_store.get_conn()` must be called **before** `gem_store` is moved into the `Arc`. The current code does:
```rust
let gem_store = SqliteGemStore::new()?;          // line 52
let gem_store_arc = Arc::new(gem_store) as Arc<dyn GemStore>;  // line 54
```

You need to extract the connection **between** these two lines:
```rust
let gem_store = SqliteGemStore::new()?;
let shared_conn = gem_store.get_conn();  // <-- get conn BEFORE Arc wrapping
let gem_store_arc = Arc::new(gem_store) as Arc<dyn GemStore>;
app.manage(gem_store_arc.clone());

// Initialize ProjectStore with same connection
let project_store = projects::SqliteProjectStore::new(shared_conn)
    .map_err(|e| format!("Failed to initialize project store: {}", e))?;
let project_store_arc = Arc::new(project_store) as Arc<dyn projects::ProjectStore>;
app.manage(project_store_arc);
```

Also add the import at the top of `lib.rs`:
```rust
use projects::{ProjectStore, SqliteProjectStore};
```

#### 7c. Register commands in `generate_handler![]`

Add 9 commands to the `generate_handler![]` macro in `lib.rs`. Add them as a block after the `knowledge::commands::*` entries (around line 395):

```rust
projects::commands::create_project,
projects::commands::list_projects,
projects::commands::get_project,
projects::commands::update_project,
projects::commands::delete_project,
projects::commands::add_gems_to_project,
projects::commands::remove_gem_from_project,
projects::commands::get_project_gems,
projects::commands::get_gem_projects,
```

#### 7d. Update `projects/mod.rs`

Add `pub mod commands;` to the module declarations in `jarvis-app/src-tauri/src/projects/mod.rs`. The file should now have:
```rust
pub mod store;
pub mod sqlite_store;
pub mod commands;
```

## Verification

After implementing, run:
```bash
cd jarvis-app/src-tauri && cargo check
```

Then run a full build to ensure everything links:
```bash
cd jarvis-app/src-tauri && cargo build
```

Both must pass.

**Common issues:**
- `get_conn()` not accessible — make sure it's `pub` and in the `impl SqliteGemStore` block (not in the `impl GemStore for SqliteGemStore` block)
- Calling `gem_store.get_conn()` after `Arc::new(gem_store)` — the original `gem_store` is moved. Call `get_conn()` before the Arc wrapping
- Missing import `use projects::{ProjectStore, SqliteProjectStore};` in lib.rs — without this the type annotations won't resolve
- Unused import warnings — `GemStore` import in `commands.rs` is not needed if you're not using it directly. Only import what you use: `use crate::gems::GemPreview;`
- `query.as_deref()` — needed in `get_project_gems` command to convert `Option<String>` to `Option<&str>`

## What NOT to Do

- Do NOT refactor `SqliteGemStore::new()` to take an external connection — just add `get_conn()` to expose the existing one
- Do NOT modify `SqliteProjectStore` — it's complete from Phase 2
- Do NOT modify `store.rs` — it's complete from Phase 1
- Do NOT add TypeScript types or frontend code — that's Phase 4
- Do NOT modify `Cargo.toml`

## After Implementation

Once `cargo build` passes:
1. Show me the files you created (`commands.rs`) and modified (`lib.rs`, `sqlite_store.rs`, `mod.rs`)
2. I'll review before we proceed to Phase 4 (frontend)

**If you have any confusion or questions — about the `get_conn()` placement, the ordering in lib.rs setup, how State<> works in Tauri commands, or anything else — please ask before guessing. It's better to clarify than to implement something wrong.**
