# Phase 1: Agent Struct Change + Composite Document Builder

## What You're Building

You are implementing Phase 1 of the Project Summary Checkpoints feature. This phase adds a `knowledge_store` field to the existing `ProjectResearchAgent`, defines new types/prompts, and implements two private helper methods: `build_composite_document` and `chunk_by_gem_boundaries`.

**No Tauri commands or frontend work in this phase.** Backend only. The goal is to compile cleanly with `cargo check`.

## Context

Read these files before starting:

- **Design doc:** `.kiro/specs/project-summary-checkpoints/design.md` — full architecture, struct changes, method signatures, prompt definitions
- **Tasks:** `.kiro/specs/project-summary-checkpoints/tasks.md` — Phase 1 has Tasks 1-4 with specific subtasks
- **Existing agent:** `jarvis-app/src-tauri/src/agents/project_agent.rs` — the `ProjectResearchAgent` struct you'll modify
- **Existing lib.rs:** `jarvis-app/src-tauri/src/lib.rs` — where the agent is constructed and registered
- **Knowledge store trait:** `jarvis-app/src-tauri/src/knowledge/store.rs` — the `KnowledgeStore` trait with `get_assembled()` method
- **Knowledge local store:** `jarvis-app/src-tauri/src/knowledge/local_store.rs` — the `KNOWN_SUBFILES` constant and `LocalKnowledgeStore` implementation

## Tasks (do these in order)

### Task 1: Add `knowledge_store` to `ProjectResearchAgent`

1. In `project_agent.rs`: add `knowledge_store: Arc<dyn KnowledgeStore>` field to the struct
2. Add `use crate::knowledge::KnowledgeStore;` import (check what the knowledge module exports — look at `src/knowledge/mod.rs`)
3. Update `new()` to accept and store `knowledge_store` parameter
4. In `lib.rs`: find where `ProjectResearchAgent::new()` is called. Add `knowledge_store_arc.clone()` as a parameter. Make sure to clone it BEFORE `app.manage(knowledge_store_arc)` consumes it — follow the same pattern used for `search_provider` and `project_store_arc` (they clone before manage)
5. Run `cargo check` to verify nothing broke

### Task 2: Define `ProjectSummaryResult` and LLM Prompts

In `project_agent.rs`:

1. Add the `ProjectSummaryResult` struct (near the existing `ProjectResearchResults`):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummaryResult {
    pub summary: String,
    pub composite_doc: String,
    pub gems_analyzed: usize,
    pub chunks_used: usize,
}
```

2. Define these prompt constants (near the existing `TOPIC_GENERATION_PROMPT` and `SUMMARIZE_PROMPT`):

**`CHECKPOINT_SUMMARY_PROMPT`** — used when everything fits in one chunk:
```
You are a research analyst. Given a project and its collected resources (listed chronologically), generate a comprehensive summary covering all key points.

Format:
- Group findings by date
- Under each resource: 3-5 bullet points with the most important highlights
- End with a synthesis of cross-cutting themes

Rules:
- Be specific — cite actual facts, numbers, and insights
- Every resource should have key points extracted
- Use markdown formatting
- Keep each bullet to one concise sentence
```

**`CHUNK_SUMMARY_PROMPT`** — used per-chunk when multiple chunks needed:
```
You are summarizing a section of a research project. Extract the key points and highlights from each resource below. Be specific and preserve important details.

Format: For each resource, list 3-5 key bullet points.
```

**`MERGE_SUMMARY_PROMPT`** — used to combine chunk summaries:
```
You have summaries from different sections of a research project. Combine them into one cohesive summary document.

Rules:
- Preserve all key points from each section
- Maintain chronological order by date
- Add a brief synthesis of cross-cutting themes at the end
- Use markdown formatting
```

**`SUMMARY_QA_PROMPT`** — used for Q&A (you'll use this in Phase 2, but define it now):
```
You are answering questions about a project summary. Use the summary and source material provided to give specific, grounded answers. If the answer isn't in the provided material, say so.
```

### Task 3: Implement `build_composite_document`

Add a private async method to `ProjectResearchAgent`:

```rust
async fn build_composite_document(&self, project_id: &str) -> Result<(String, Vec<String>, usize), String>
```

Returns: `(full_composite_doc, individual_gem_sections, gems_analyzed)`

**Why return `Vec<String>` of individual sections?** The chunker (Task 4) needs the individual gem sections to group them without splitting mid-gem. The `full_composite_doc` is for the frontend (stored in `ProjectSummaryResult.composite_doc`).

**Implementation:**

1. `self.project_store.get(project_id)` — load project with gems
2. Return error if `detail.gems.is_empty()`
3. Sort `detail.gems` by `captured_at` ascending — use `.sort_by(|a, b| a.captured_at.cmp(&b.captured_at))` (these are ISO 8601 strings, lexicographic sort works)
4. Build a header string:
```
# Project: {title}
**Objective:** {objective or "Not specified"}
**Gems:** {count} | **Date range:** {first.captured_at} — {last.captured_at}

---
```
5. For each gem, build a section:
   - Try `self.knowledge_store.get_assembled(&gem.id).await`
   - If `Ok(Some(content))`: use that content
   - If `Ok(None)` or `Err(_)`: fall back to DB fields via `self.gem_store.get(&gem.id).await`:
     - Assemble from: title, description (if Some), content (first 2000 chars if Some), ai_enrichment summary (if present), transcript (first 2000 chars if Some)
   - If absolutely nothing available: use `"(No content available for this gem)"`
   - Wrap with separator:
```
========================================
GEM {n}: "{title}"
Source: {source_type} | Domain: {domain} | Captured: {captured_at}
========================================

{content}
```
6. Collect gem sections into a `Vec<String>`
7. Build `full_composite_doc` = header + all sections joined with `"\n\n"`
8. Log: `eprintln!("Projects/Summary: Built composite document: {} chars, {} gems", composite_doc.len(), gems_analyzed)`
9. Return `(full_composite_doc, gem_sections, gems_analyzed)`

**Important:** The `GemPreview` struct (from `detail.gems`) has limited fields. You need the full `Gem` for fallback content. Check what `GemPreview` has — it likely has `id`, `title`, `source_type`, `domain`, `captured_at` but maybe not `content`. The `get_assembled` path avoids needing the full gem. Only load the full gem on fallback.

### Task 4: Implement `chunk_by_gem_boundaries`

Add a private method (not async — pure logic):

```rust
fn chunk_by_gem_boundaries(gem_sections: &[String], max_chars: usize) -> Vec<String>
```

**Implementation:**

1. Walk through `gem_sections` in order
2. Maintain `current_chunk: String` and `chunks: Vec<String>`
3. For each section:
   - If `current_chunk.len() + section.len() <= max_chars`: append section to current_chunk (with `"\n\n"` separator)
   - Else if `current_chunk.is_empty()`: this single gem exceeds the limit — truncate the section to `max_chars` chars and push as its own chunk. Add a `"\n\n[Content truncated — original was {len} characters]"` note.
   - Else: push `current_chunk` to `chunks`, start new `current_chunk` with this section
4. Push final `current_chunk` if non-empty
5. Log: `eprintln!("Projects/Summary: Split into {} chunks from {} gem sections", chunks.len(), gem_sections.len())`
6. Return `chunks`

**Default `max_chars`:** 16000 (≈4000 tokens at 4 chars/token). This will be passed as a parameter — don't hardcode it in the method signature, but the caller will use 16000.

## Checkpoint

After completing all 4 tasks, run `cargo check`. Everything should compile. No new Tauri commands yet — that's Phase 4.

**What to verify:**
- `ProjectResearchAgent` struct has `knowledge_store` field
- `new()` accepts and stores it
- `lib.rs` passes it correctly
- `ProjectSummaryResult` struct defined with correct derives
- 4 prompt constants defined
- `build_composite_document` compiles (you can't test it end-to-end yet without Tauri commands, but the types should check out)
- `chunk_by_gem_boundaries` compiles
- All existing functionality unchanged — the agent's existing methods (`suggest_topics`, `run_research`, `summarize`, chat methods) should be untouched

## If You Have Questions

If something is unclear or you encounter unexpected code structure:

- **Ask me before guessing.** Especially about: how `knowledge_store` is exported from the knowledge module, how `lib.rs` orders its setup code, or what fields `GemPreview` has.
- **Check the existing patterns.** The agent already holds `project_store`, `gem_store`, `intel_provider`, `search_provider`, `intel_queue`. Adding `knowledge_store` follows the same pattern exactly.
- **Don't modify existing methods.** This phase only ADDS new code. The only existing code you change is: (1) the struct definition (add field), (2) `new()` (add parameter), (3) `lib.rs` (add argument to constructor).
