# Prompt: Implement Gem Knowledge Files — Phase 1

## Your Task

Implement **Phase 1: Core Types and Assembler** of the gem-knowledge-files spec. This phase is pure data types and formatting logic — no filesystem I/O, no Tauri integration yet.

**After completing Phase 1, stop and ask me to review before moving to Phase 2.**

If you have any confusion or questions during implementation — about naming, types, existing patterns, how something fits together — feel free to ask rather than guessing. I'd rather answer a question than fix a wrong assumption later.

---

## Context

Jarvis is a Tauri 2.x desktop app (Rust backend + React frontend). We're adding a **knowledge file system** that generates one markdown folder per gem for search indexing and agent consumption.

**Spec:** `.kiro/specs/gem-knowledge-files/requirements.md` (Requirements 1–4 cover Phase 1)
**Design doc:** `discussion/28-feb-next-step/gem-knowledge-files.md`
**Tasks:** `.kiro/specs/gem-knowledge-files/tasks.md` (Tasks 1–4)

---

## What Phase 1 Produces

4 new files inside `jarvis-app/src-tauri/src/knowledge/`:

```
src/knowledge/
├── mod.rs          ← module declarations + re-exports
├── store.rs        ← KnowledgeStore trait, KnowledgeEventEmitter trait, all data types
├── assembler.rs    ← format_content(), format_enrichment(), format_transcript(),
│                     format_copilot(), extract_tags(), extract_summary(),
│                     assemble_gem_md()
└── (no local_store.rs yet — that's Phase 2)
```

Plus: `pub mod knowledge;` added to `lib.rs` and `dashmap = "6"` in `Cargo.toml`.

---

## Existing Patterns to Follow

### Trait pattern (follow IntelProvider exactly)

```rust
// src/intelligence/provider.rs — this is the pattern
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityResult {
    pub available: bool,
    pub reason: Option<String>,
}

#[async_trait]
pub trait IntelProvider: Send + Sync {
    async fn check_availability(&self) -> AvailabilityResult;
    async fn generate_tags(&self, content: &str) -> Result<Vec<String>, String>;
    // ...
}
```

### Gem struct (what you'll consume)

```rust
// src/gems/store.rs
pub struct Gem {
    pub id: String,                              // UUID v4
    pub source_type: String,                     // "YouTube", "Article", "Email", "Chat", "Recording"
    pub source_url: String,
    pub domain: String,
    pub title: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub source_meta: serde_json::Value,          // JSON blob
    pub captured_at: String,                     // ISO 8601
    pub ai_enrichment: Option<serde_json::Value>, // {"tags": [...], "summary": "...", "provider": "...", "enriched_at": "..."}
    pub transcript: Option<String>,
    pub transcript_language: Option<String>,      // "en", "zh", etc.
}
```

### CoPilotCycleResult (for format_copilot)

```rust
// src/intelligence/provider.rs
pub struct CoPilotCycleResult {
    pub new_content: String,
    pub updated_summary: String,
    pub key_points: Vec<String>,
    pub decisions: Vec<String>,
    pub action_items: Vec<String>,
    pub open_questions: Vec<String>,
    pub suggested_questions: Vec<CoPilotQuestion>,
    pub key_concepts: Vec<CoPilotConcept>,
}
```

### Import style

```rust
use crate::intelligence::provider::AvailabilityResult;
use crate::gems::store::Gem;
```

---

## Task 1: Module Structure + Trait Definitions

### 1.1 — Create `src/knowledge/mod.rs`

```rust
pub mod store;
pub mod assembler;

pub use store::{
    KnowledgeStore, KnowledgeEntry, KnowledgeSubfile,
    MigrationResult, KnowledgeEvent, KnowledgeEventEmitter, GemMeta,
};
```

Note: `LocalKnowledgeStore` re-export comes in Phase 2.

### 1.2 — Create `src/knowledge/store.rs`

Define these types and traits:

**`GemMeta`** — machine-readable metadata for `meta.json`:
- `id: String`
- `source_type: String`
- `source_url: String`
- `domain: String`
- `title: String`
- `author: Option<String>`
- `captured_at: String`
- `project_id: Option<String>`
- `source_meta: serde_json::Value`
- `knowledge_version: u32`
- `last_assembled: String`

**`KnowledgeEntry`** — returned by `get()`:
- `gem_id: String`
- `assembled: String` (full gem.md content)
- `subfiles: Vec<KnowledgeSubfile>`
- `version: u32`
- `last_assembled: String` (ISO 8601)

**`KnowledgeSubfile`**:
- `filename: String`
- `exists: bool`
- `size_bytes: u64`
- `last_modified: Option<String>`

**`MigrationResult`**:
- `total: usize`
- `created: usize`
- `skipped: usize`
- `failed: usize`
- `errors: Vec<(String, String)>` (gem_id, error)

All structs: `#[derive(Debug, Clone, Serialize, Deserialize)]`
Exception: `MigrationResult` only needs `Serialize` (+ Debug, Clone).

**`KnowledgeStore` trait** — full CRUD contract (see requirements.md Req 1 for exact signatures). Reuse `AvailabilityResult` from `crate::intelligence::provider`.

### 1.3 — Add `pub mod knowledge;` to `lib.rs`

Place it alongside existing module declarations.

### 1.4 — Add `dashmap = "6"` to `Cargo.toml`

Check if it's already there first.

---

## Task 2: KnowledgeEventEmitter

In `store.rs`:

**`KnowledgeEvent` enum** with `#[serde(tag = "type")]`:
- `SubfileUpdated { gem_id, filename, status }` — status: "writing" | "assembling" | "done"
- `MigrationProgress { current, total, gem_id, gem_title, status }` — status: "generating" | "done" | "failed"
- `MigrationComplete { result: MigrationResult }`

**`KnowledgeEventEmitter` trait** (`Send + Sync`):
- `fn emit_progress(&self, event: KnowledgeEvent)`

**`TauriKnowledgeEventEmitter` struct**:
- Wraps `tauri::AppHandle`
- Emits on `"knowledge-progress"` channel

---

## Task 3: Assembler Formatting Functions

Create `src/knowledge/assembler.rs` with these public functions:

### `format_content(title: &str, content: &str) -> String`
```markdown
# {title}

{content}
```

### `format_enrichment(enrichment: &serde_json::Value) -> String`
```markdown
## Summary
{enrichment.summary — skip section if missing}

## Tags
- tag1
- tag2
{skip section if no tags}

## Enrichment Metadata
- Provider: {enrichment.provider}
- Enriched: {enrichment.enriched_at}
```
Handle missing fields gracefully — skip sections that don't exist.

### `format_transcript(transcript: &str, language: &str) -> String`
```markdown
## Transcript
Language: {language}

{transcript text}
```

### `format_copilot(copilot_data: &serde_json::Value) -> String`
The co-pilot data in `gem.source_meta` (for recording gems) contains the accumulated analysis. Format each non-empty section:
```markdown
## Rolling Summary
{summary}

## Key Points
- point1
- point2

## Decisions
- decision1

## Action Items
- item1

## Open Questions
- question1

## Key Concepts
- **{term}**: {context}
```
Omit any section that's empty. If you're unsure about the exact shape of co-pilot data in `source_meta`, **ask me** — don't guess.

---

## Task 4: Assembly + Extraction Helpers

### `extract_tags(enrichment_md: &str) -> Vec<String>`
Parse the `## Tags` section from an enrichment.md string. Return the bulleted items as strings.

### `extract_summary(enrichment_md: &str) -> Option<String>`
Parse the `## Summary` section — text between `## Summary` heading and next `##` heading.

### `assemble_gem_md(gem_folder: &Path, meta: &GemMeta) -> Result<String, String>`
This is async — it reads subfiles from disk. Fixed section order:

1. `# {title}`
2. Metadata lines: `- **Source:** ...`, `- **URL:** ...`, `- **Author:** ...`, `- **Captured:** ...`, `- **Tags:** ...`, `- **Project:** ...`
3. Summary (from enrichment.md)
4. `## Content` + content.md body
5. Transcript section (from transcript.md)
6. `## Co-Pilot Analysis` + copilot.md body

Omit sections for non-existent subfiles. Use a helper `read_subfile(folder: &Path, filename: &str) -> Result<String, std::io::Error>` that reads file content (tokio::fs::read_to_string).

---

## Validation Checklist (before asking for review)

- [ ] `cargo build` succeeds with no errors
- [ ] All types in `store.rs` compile (`KnowledgeStore` trait, all structs, enums)
- [ ] `TauriKnowledgeEventEmitter` compiles against `tauri::AppHandle`
- [ ] All assembler functions produce correct markdown format
- [ ] `extract_tags` roundtrips: `format_enrichment(json) → extract_tags(result)` returns same tags
- [ ] `assemble_gem_md` produces sections in the right order
- [ ] Module re-exports work: `use crate::knowledge::KnowledgeStore` compiles from other modules

---

## Important Notes

- This is a **Rust-only** phase. No frontend changes.
- Use `tokio::fs` for async file reads in `assemble_gem_md`, not `std::fs`.
- The `KnowledgeStore` trait methods won't have implementations yet — that's Phase 2. The trait just needs to compile.
- Follow existing code style: no unnecessary comments on obvious things, doc comments only on public trait methods.
- Keep imports organized: std → external crates → crate-internal.
