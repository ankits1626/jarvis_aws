# Phase 3: Save as Gem + Knowledge Store Change

## What You're Building

You are implementing Phase 3 of the Project Summary Checkpoints feature. This phase:

1. **Adds `composite_summary_of_all_gems.md` to `KNOWN_SUBFILES`** — so the knowledge viewer recognizes this file type
2. **Implements `save_summary_checkpoint`** — saves a generated summary as a new gem, adds it to the project, creates knowledge files, and writes the composite document as a subfile

**No Tauri commands or frontend work in this phase.** Backend only. The goal is to compile cleanly with `cargo check`.

## Context

Read these files before starting:

- **Design doc:** `.kiro/specs/project-summary-checkpoints/design.md` — see "Save as Gem" section
- **Tasks:** `.kiro/specs/project-summary-checkpoints/tasks.md` — Phase 3 has Tasks 7-8
- **The agent:** `jarvis-app/src-tauri/src/agents/project_agent.rs` — you'll add a method to the existing `impl ProjectResearchAgent` block
- **Knowledge local store:** `jarvis-app/src-tauri/src/knowledge/local_store.rs` — contains `KNOWN_SUBFILES` constant

## What Phases 1-2 Already Built

These are available for you to use:

```rust
// Phase 1
async fn build_composite_document(&self, project_id: &str) -> Result<(String, Vec<String>, usize), String>
fn chunk_by_gem_boundaries(gem_sections: &[String], max_chars: usize) -> Vec<String>

// Phase 2
pub async fn generate_summary_checkpoint(&self, project_id: &str) -> Result<ProjectSummaryResult, String>
pub async fn send_summary_question(&self, question: &str, summary: &str, composite_doc: &str) -> Result<String, String>
```

The agent already holds these fields (all `Arc`):
- `project_store: Arc<dyn ProjectStore>`
- `gem_store: Arc<dyn GemStore>`
- `knowledge_store: Arc<dyn KnowledgeStore>`
- `search_provider: Arc<dyn SearchResultProvider>`

## Tasks

### Task 7: Update `KNOWN_SUBFILES`

**File:** `jarvis-app/src-tauri/src/knowledge/local_store.rs`

The current constant (around line 15):
```rust
const KNOWN_SUBFILES: &[&str] = &[
    "meta.json",
    "content.md",
    "enrichment.md",
    "transcript.md",
    "copilot.md",
    "gem.md",
];
```

**Change:** Add `"composite_summary_of_all_gems.md"` before `"gem.md"`:
```rust
const KNOWN_SUBFILES: &[&str] = &[
    "meta.json",
    "content.md",
    "enrichment.md",
    "transcript.md",
    "copilot.md",
    "composite_summary_of_all_gems.md",
    "gem.md",
];
```

**Why before `gem.md`?** The assembler reads subfiles to build `gem.md`. Placing it before ensures the composite file is recognized during assembly. For non-summary gems, the file simply won't exist on disk and is skipped — no branching logic needed.

### Task 8: Implement `save_summary_checkpoint`

Add a public async method to `ProjectResearchAgent`:

```rust
pub async fn save_summary_checkpoint(
    &self,
    project_id: &str,
    summary_content: &str,
    composite_doc: &str,
) -> Result<Gem, String>
```

**You'll need these imports** at the top of `project_agent.rs` (add only what's not already there):
```rust
use uuid::Uuid;
use chrono::Utc;
use crate::gems::Gem;
```

Check if `Gem` is already imported — the agent uses `GemStore` but may not import `Gem` directly yet.

**Implementation — follow this exact order:**

**Step 1: Load project metadata**
```rust
eprintln!("Projects/Summary: Saving summary checkpoint for project {}", project_id);
let project_detail = self.project_store.get(project_id)
    .map_err(|e| format!("Failed to load project: {}", e))?;
let gem_count = project_detail.gems.len();
let project_title = project_detail.title.clone();
```

**Step 2: Build the Gem struct**
```rust
let gem = Gem {
    id: Uuid::new_v4().to_string(),
    source_type: "ProjectSummary".to_string(),
    source_url: format!(
        "jarvis://project/{}/summary/{}",
        project_id,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    ),
    title: format!("Summary: {} — {}", project_title, Utc::now().format("%B %d, %Y")),
    content: Some(summary_content.to_string()),
    domain: "jarvis".to_string(),
    description: Some(format!("Summary of {} gems from {}", gem_count, project_title)),
    author: None,
    captured_at: Utc::now().to_rfc3339(),
    source_meta: serde_json::json!({}),
    ai_enrichment: None,
    transcript: None,
    transcript_language: None,
};
```

**Step 3: Save gem to database**

The `gem_store.save()` takes an owned `Gem` (not a reference) and returns `Result<Gem, String>`. It performs an upsert by `source_url`:
```rust
let saved_gem = self.gem_store.save(gem).await
    .map_err(|e| format!("Failed to save summary gem: {}", e))?;
```

**Step 4: Add gem to project**
```rust
self.project_store.add_gems(project_id, &[saved_gem.id.clone()]).await
    .map_err(|e| format!("Failed to add summary gem to project: {}", e))?;
```

**Step 5: Create knowledge files**

This creates the standard knowledge directory (`content.md`, `enrichment.md`, `meta.json`, `gem.md`):
```rust
self.knowledge_store.create(&saved_gem).await
    .map_err(|e| format!("Failed to create knowledge files: {}", e))?;
```

**Step 6: Write composite file as subfile**

This writes `composite_summary_of_all_gems.md` and reassembles `gem.md` to include it:
```rust
self.knowledge_store.update_subfile(
    &saved_gem.id,
    "composite_summary_of_all_gems.md",
    composite_doc,
).await.map_err(|e| format!("Failed to write composite file: {}", e))?;
```

**Step 7: Index for search (fire-and-forget)**

Follow the existing pattern — log errors but don't fail the save:
```rust
if let Err(e) = self.search_provider.index_gem(&saved_gem.id).await {
    eprintln!("Projects/Summary: Failed to index summary gem {}: {}", saved_gem.id, e);
}
```

**Step 8: Log and return**
```rust
eprintln!(
    "Projects/Summary: Saved summary gem {} for project {} ({} chars)",
    saved_gem.id, project_id, summary_content.len()
);
Ok(saved_gem)
```

## Checkpoint

Run `cargo check`. Both changes should compile.

**What to verify:**
- `KNOWN_SUBFILES` has 7 entries now (was 6), with `composite_summary_of_all_gems.md` before `gem.md`
- `save_summary_checkpoint` compiles with correct types:
  - `gem_store.save()` takes `Gem` (owned, not `&Gem`)
  - `project_store.add_gems()` takes `&str` + `&[String]`
  - `knowledge_store.create()` takes `&Gem`
  - `knowledge_store.update_subfile()` takes `&str, &str, &str`
  - `search_provider.index_gem()` takes `&str`
- Return type is `Result<Gem, String>` — the saved gem is returned for the frontend to use
- All existing methods unchanged

## If You Have Questions

- **Ask me before guessing.** Especially about ownership — `gem_store.save()` consumes the `Gem` (takes owned value), so use the `saved_gem` returned by it for all subsequent steps.
- **Don't add Tauri commands yet.** That's Phase 4. This method just needs to compile.
- **Check existing imports** before adding new ones. The agent file already imports from several crate modules.
