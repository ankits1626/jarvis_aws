# Phase 1 Implementation Prompt — Projects Backend: Trait, Types, and Module Structure

## What to Implement

Implement **Phase 1** from `.kiro/specs/projects/tasks.md` — Tasks 1 and 2. This creates the `projects` module skeleton with the `ProjectStore` trait and all data types. **No implementation logic yet** — just the trait definition, data structs, and module wiring.

## Context Files (Read These First)

Read these files before writing any code — they contain the exact code to write and the patterns to follow:

1. **Design doc** (has complete Rust code for `store.rs` and `mod.rs`):
   `.kiro/specs/projects/design.md` — Sections: "store.rs — Trait and Data Types" and "mod.rs — Module Root"

2. **Reference trait** (follow this pattern exactly):
   `jarvis-app/src-tauri/src/gems/store.rs` — Shows how `GemStore` trait is structured with `#[async_trait]`, `Send + Sync`, and data structs with derive macros

3. **Module declarations** (where to add `pub mod projects`):
   `jarvis-app/src-tauri/src/lib.rs` — Line 1-16 shows existing module declarations

4. **Requirements spec** (acceptance criteria to verify against):
   `.kiro/specs/projects/requirements.md` — Requirements 1, 2, and 5

## Tasks

### Task 1: Create `src/projects/store.rs`

Create the file `jarvis-app/src-tauri/src/projects/store.rs` with:

**Data structs** (all derive `Debug, Clone, Serialize, Deserialize`):
- `Project` — id, title, description (Option), objective (Option), status, created_at, updated_at
- `ProjectPreview` — id, title, description (Option), status, gem_count (usize), updated_at
- `ProjectDetail` — project (Project), gem_count (usize), gems (Vec<GemPreview>)
- `CreateProject` — title, description (Option), objective (Option). Only needs `Deserialize` (input struct)
- `UpdateProject` — title (Option), description (Option), objective (Option), status (Option). Only needs `Deserialize` (input struct)

**Trait** `ProjectStore` with `#[async_trait]` and `Send + Sync` bounds, 9 methods:
- `create`, `list`, `get`, `update`, `delete`
- `add_gems`, `remove_gem`
- `get_project_gems`, `get_gem_projects`

**Import**: `GemPreview` from `crate::gems::GemPreview` (used in `ProjectDetail` and `get_project_gems` return type).

The complete code is in the design doc — adapt it, don't copy blindly. Make sure the imports and derive macros match what this codebase actually uses.

### Task 2: Create `src/projects/mod.rs`

Create the file `jarvis-app/src-tauri/src/projects/mod.rs` with:
- Module declarations: `pub mod store;` (only store for now — `sqlite_store` and `commands` will be added in Phase 2/3)
- Re-exports: `ProjectStore`, `Project`, `ProjectPreview`, `ProjectDetail`, `CreateProject`, `UpdateProject`

**Note**: Do NOT declare `pub mod sqlite_store;` or `pub mod commands;` yet — those files don't exist and will cause compilation errors. Only declare modules that exist.

### Task 2 (continued): Add module to `lib.rs`

Add `pub mod projects;` to the module declarations in `jarvis-app/src-tauri/src/lib.rs` (after `pub mod knowledge;` or similar — alphabetical order preferred but not required).

## Verification

After implementing, run:
```bash
cd jarvis-app/src-tauri && cargo check
```

This must pass. If it doesn't, fix the errors before requesting review.

Common pitfalls:
- Missing `use async_trait::async_trait;` import in `store.rs`
- Missing `use serde::{Deserialize, Serialize};` import
- Declaring `pub mod sqlite_store;` in `mod.rs` when the file doesn't exist yet
- Wrong path for `GemPreview` import — it's `crate::gems::GemPreview` (check `gems/mod.rs` to confirm re-export)

## What NOT to Do

- Do NOT create `sqlite_store.rs` or `commands.rs` — those are Phase 2 and Phase 3
- Do NOT add any Tauri commands or register anything in `generate_handler![]`
- Do NOT modify `Cargo.toml` — all dependencies (`async-trait`, `serde`, etc.) already exist
- Do NOT add TypeScript types — that's Phase 4
- Do NOT add tests yet — that's Phase 9

## After Implementation

Once `cargo check` passes:
1. Show me the files you created and any modifications you made
2. I'll review before we proceed to Phase 2

**If you have any confusion or questions during implementation — about the trait design, field types, naming conventions, or how something works in the existing codebase — please ask before guessing. It's better to clarify than to implement something wrong.**
