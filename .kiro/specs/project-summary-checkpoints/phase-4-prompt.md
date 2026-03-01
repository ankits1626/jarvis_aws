# Phase 4: Tauri Commands + Registration

## What You're Building

You are implementing Phase 4 of the Project Summary Checkpoints feature. This phase exposes the backend methods from Phases 2-3 to the frontend by adding three Tauri commands and registering them.

**No frontend work in this phase.** Backend only. The goal is to compile cleanly with `cargo check`.

## Context

Read these files before starting:

- **Design doc:** `.kiro/specs/project-summary-checkpoints/design.md` — see "Tauri Commands" section
- **Tasks:** `.kiro/specs/project-summary-checkpoints/tasks.md` — Phase 4 has Tasks 9-10
- **Existing project commands:** `jarvis-app/src-tauri/src/projects/commands.rs` — follow the same patterns exactly
- **lib.rs:** `jarvis-app/src-tauri/src/lib.rs` — find the `generate_handler![]` macro to register commands

## What Phases 1-3 Already Built

These public methods exist on `ProjectResearchAgent`:

```rust
pub async fn generate_summary_checkpoint(&self, project_id: &str) -> Result<ProjectSummaryResult, String>
pub async fn save_summary_checkpoint(&self, project_id: &str, summary_content: &str, composite_doc: &str) -> Result<Gem, String>
pub async fn send_summary_question(&self, question: &str, summary: &str, composite_doc: &str) -> Result<String, String>
```

Types already defined and serializable:
- `ProjectSummaryResult` — in `crate::agents::project_agent`, derives `Serialize`/`Deserialize`
- `Gem` — in `crate::gems`, derives `Serialize`/`Deserialize`

## Existing Command Pattern

Every project agent command in `commands.rs` follows this exact pattern:

```rust
#[tauri::command]
pub async fn some_command(
    arg1: String,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<SomeReturnType, String> {
    let agent = agent.lock().await;
    agent.some_method(&arg1).await
}
```

The existing imports at the top of `commands.rs`:
```rust
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex as TokioMutex;
use crate::gems::GemPreview;
use crate::agents::project_agent::{ProjectResearchAgent, ProjectResearchResults};
use crate::agents::chatbot::ChatMessage;
use super::store::*;
```

## Tasks

### Task 9: Add Three Summary Commands to `projects/commands.rs`

**Step 1: Update imports**

Add `ProjectSummaryResult` to the existing `project_agent` import, and add `Gem`:
```rust
use crate::agents::project_agent::{ProjectResearchAgent, ProjectResearchResults, ProjectSummaryResult};
use crate::gems::{GemPreview, Gem};
```

**Step 2: Add the three commands**

Add these at the end of the file, after the existing commands:

```rust
#[tauri::command]
pub async fn generate_project_summary_checkpoint(
    project_id: String,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<ProjectSummaryResult, String> {
    let agent = agent.lock().await;
    agent.generate_summary_checkpoint(&project_id).await
}

#[tauri::command]
pub async fn save_project_summary_checkpoint(
    project_id: String,
    summary_content: String,
    composite_doc: String,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<Gem, String> {
    let agent = agent.lock().await;
    agent.save_summary_checkpoint(&project_id, &summary_content, &composite_doc).await
}

#[tauri::command]
pub async fn send_summary_question(
    question: String,
    summary: String,
    composite_doc: String,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<String, String> {
    let agent = agent.lock().await;
    agent.send_summary_question(&question, &summary, &composite_doc).await
}
```

**Notes:**
- All three use **immutable** lock (`let agent`, not `let mut agent`) — these methods take `&self`
- Arguments are `String` (owned), converted to `&str` when calling the agent methods
- The `agent` parameter uses `State<'_, Arc<TokioMutex<ProjectResearchAgent>>>` — same as all other agent commands

### Task 10: Register Commands in `lib.rs`

Find the `generate_handler![]` macro in `lib.rs`. The project commands are grouped together. Add the three new commands alongside the existing project commands:

```rust
projects::commands::generate_project_summary_checkpoint,
projects::commands::save_project_summary_checkpoint,
projects::commands::send_summary_question,
```

Place them after the existing `get_project_summary` command (or at the end of the project commands group) — the order in the macro doesn't matter functionally, but grouping by feature keeps it organized.

## Checkpoint

Run `cargo check`. All three commands should compile.

**What to verify:**
- `generate_project_summary_checkpoint` returns `Result<ProjectSummaryResult, String>`
- `save_project_summary_checkpoint` returns `Result<Gem, String>`
- `send_summary_question` returns `Result<String, String>`
- All three are registered in `generate_handler![]`
- No existing commands modified
- The `Gem` import doesn't conflict with existing `GemPreview` import (they're in the same `crate::gems` module — combine into one `use` statement)

## If You Have Questions

- **Ask me before guessing.** Especially about import paths — `Gem` and `GemPreview` are both in `crate::gems` (re-exported from `crate::gems::store`).
- **Don't add frontend code yet.** That's Phase 5. These commands just need to compile and be registered.
- **Follow the existing pattern exactly.** The three commands are thin wrappers — no business logic in the command functions.
