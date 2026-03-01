# Phase 2: Summary Generation + Q&A Methods

## What You're Building

You are implementing Phase 2 of the Project Summary Checkpoints feature. This phase adds two public methods to `ProjectResearchAgent`:

1. **`generate_summary_checkpoint`** — orchestrates the full pipeline: build composite doc → chunk → summarize each chunk → merge → return result for review
2. **`send_summary_question`** — stateless Q&A: user asks a question about the summary, LLM answers using summary + composite doc as context

**No Tauri commands or frontend work in this phase.** Backend only. The goal is to compile cleanly with `cargo check`.

## Context

Read these files before starting:

- **Design doc:** `.kiro/specs/project-summary-checkpoints/design.md` — see "Backend: New Methods on ProjectResearchAgent" section for the full flow
- **Tasks:** `.kiro/specs/project-summary-checkpoints/tasks.md` — Phase 2 has Tasks 5-6
- **The agent (with Phase 1 changes):** `jarvis-app/src-tauri/src/agents/project_agent.rs` — you'll be adding methods to the existing `impl ProjectResearchAgent` block

## What Phase 1 Already Built

These are available for you to use:

```rust
// Returns (full_composite_doc, individual_gem_sections, gems_analyzed)
async fn build_composite_document(&self, project_id: &str) -> Result<(String, Vec<String>, usize), String>

// Groups gem sections into chunks that don't exceed max_chars
fn chunk_by_gem_boundaries(gem_sections: &[String], max_chars: usize) -> Vec<String>

// Prompt constants already defined:
// CHECKPOINT_SUMMARY_PROMPT, CHUNK_SUMMARY_PROMPT, MERGE_SUMMARY_PROMPT, SUMMARY_QA_PROMPT
```

The LLM is called via `self.intel_provider.chat()` which takes `&[(String, String)]` — pairs of (role, content). Use it like:
```rust
self.intel_provider.chat(&[
    ("system".to_string(), SOME_PROMPT.to_string()),
    ("user".to_string(), some_content),
]).await?;
```

## Tasks

### Task 5: Implement `generate_summary_checkpoint`

Add a public async method to `ProjectResearchAgent`:

```rust
pub async fn generate_summary_checkpoint(
    &self,
    project_id: &str,
) -> Result<ProjectSummaryResult, String>
```

**Implementation:**

1. Log: `eprintln!("Projects/Summary: Generating summary checkpoint for project {}", project_id)`
2. Call `self.build_composite_document(project_id).await?` — get `(composite_doc, gem_sections, gems_analyzed)`
3. Call `Self::chunk_by_gem_boundaries(&gem_sections, 16000)` — get `chunks: Vec<String>`
4. Branch on chunk count:

**Single chunk (chunks.len() == 1):**
```rust
let summary = self.intel_provider.chat(&[
    ("system".to_string(), CHECKPOINT_SUMMARY_PROMPT.to_string()),
    ("user".to_string(), chunks[0].clone()),
]).await?;
```

**Multiple chunks:**
```rust
let mut chunk_summaries: Vec<String> = Vec::new();
for (i, chunk) in chunks.iter().enumerate() {
    eprintln!("Projects/Summary: Summarizing chunk {} of {}", i + 1, chunks.len());
    match self.intel_provider.chat(&[
        ("system".to_string(), CHUNK_SUMMARY_PROMPT.to_string()),
        ("user".to_string(), chunk.clone()),
    ]).await {
        Ok(chunk_summary) => chunk_summaries.push(chunk_summary),
        Err(e) => {
            eprintln!("Projects/Summary: Chunk {} failed: {}", i + 1, e);
            // Continue with remaining chunks — don't fail entirely
        }
    }
}

// Merge pass
let merged_input = chunk_summaries.join("\n\n---\n\n");
let summary = self.intel_provider.chat(&[
    ("system".to_string(), MERGE_SUMMARY_PROMPT.to_string()),
    ("user".to_string(), merged_input),
]).await?;
```

5. If `chunk_summaries` is empty after all failures, return an error: `"All chunks failed during summarization"`
6. Log: `eprintln!("Projects/Summary: Generated summary ({} chars) from {} gems in {} chunks", summary.len(), gems_analyzed, chunks.len())`
7. Return:
```rust
Ok(ProjectSummaryResult {
    summary,
    composite_doc,
    gems_analyzed,
    chunks_used: chunks.len(),
})
```

### Task 6: Implement `send_summary_question`

Add a public async method to `ProjectResearchAgent`:

```rust
pub async fn send_summary_question(
    &self,
    question: &str,
    summary: &str,
    composite_doc: &str,
) -> Result<String, String>
```

**Implementation:**

1. Log: `eprintln!("Projects/Summary: Answering question ({} chars)", question.len())`
2. Truncate composite_doc if too long:
```rust
let max_context = 10000;
let truncated_composite = if composite_doc.len() > max_context {
    &composite_doc[..max_context]
} else {
    composite_doc
};
```
3. Build context string:
```rust
let context = format!(
    "## Generated Summary\n\n{}\n\n## Source Material\n\n{}",
    summary, truncated_composite
);
```
4. Call LLM:
```rust
let user_message = format!("{}\n\nQuestion: {}", context, question);
let answer = self.intel_provider.chat(&[
    ("system".to_string(), SUMMARY_QA_PROMPT.to_string()),
    ("user".to_string(), user_message),
]).await?;
```
5. Log: `eprintln!("Projects/Summary: Answer generated ({} chars)", answer.len())`
6. Return `Ok(answer)`

## Checkpoint

Run `cargo check`. Both methods should compile. You should see the Phase 1 warnings about unused code resolved for the prompt constants (they're now used by `generate_summary_checkpoint` and `send_summary_question`).

**What to verify:**
- `generate_summary_checkpoint` uses `build_composite_document` and `chunk_by_gem_boundaries` from Phase 1
- Single-chunk path uses `CHECKPOINT_SUMMARY_PROMPT`
- Multi-chunk path uses `CHUNK_SUMMARY_PROMPT` per chunk + `MERGE_SUMMARY_PROMPT` for merge
- LLM failure on individual chunks is handled gracefully (log + continue)
- `send_summary_question` truncates long composite docs
- All existing methods unchanged

## If You Have Questions

- **Ask me before guessing.** Especially about how `intel_provider.chat` error handling works or whether you need `.await?` vs `match`.
- **Don't add Tauri commands yet.** That's Phase 4. These methods just need to compile.
- **The `?` operator is fine** for the merge pass and single-chunk path — if those fail, the whole operation should fail. Only individual chunk failures should be gracefully handled.
