# Gem Knowledge Files — The Filesystem Layer

> **Prerequisite for:** [Projects & Summarizer](./projects-and-summarizer.md) and [Search & Knowledge Layer](./gem-search-and-knowledge-layer.md)
>
> Both the project system and the search layer depend on gems having rich, consolidated, file-based representations. This document defines that foundation.

---

## Why Files, Not Just a Database?

Today, everything about a gem lives inside SQLite columns — `title`, `content`, `ai_enrichment` (JSON blob), `transcript`. This works for the app, but it has limits:

| Problem | Impact |
|---|---|
| **Not indexable by external tools** | QMD, grep, ripgrep, Spotlight can't search gems |
| **Not composable** | To feed a gem to an LLM, you have to assemble it from 6+ columns |
| **Not debuggable** | To see what Jarvis knows, you open a DB viewer, not a file |
| **Not portable** | Can't drag a gem into another tool, email it, or version-control it |
| **Data scattered** | Gem content in SQLite, agent logs in `agent_logs/`, recordings in `recordings/` — related data in 3 places |

The knowledge file system fixes this: **one folder per gem, everything in one place.**

---

## The Structure

```
~/Library/Application Support/com.jarvis.app/
├── gems.db                              # SQLite — source of truth for structured data
├── knowledge/                           # Knowledge filesystem — derived, regenerable
│   ├── {gem-id-1}/
│   │   ├── gem.md                       # Consolidated gem document (primary)
│   │   ├── enrichment.md               # AI-generated enrichment (tags, summary)
│   │   ├── transcript.md               # Full transcript (if recording)
│   │   ├── copilot.md                  # Co-pilot analysis (if recording with copilot)
│   │   ├── content.md                  # Raw extracted content (article body, etc.)
│   │   └── meta.json                   # Machine-readable metadata
│   │
│   ├── {gem-id-2}/
│   │   ├── gem.md
│   │   ├── enrichment.md
│   │   └── meta.json
│   │
│   └── ...
```

### Why Subfiles, Not Just One Big gem.md?

Two use cases pull in different directions:

1. **Search / LLM context** wants one consolidated document → `gem.md`
2. **Incremental updates** want granular files → `enrichment.md`, `transcript.md`, etc.

The answer: **both.** Individual subfiles are generated as data arrives. `gem.md` is assembled from them.

```
gem.md = meta header + content.md + enrichment.md + transcript.md + copilot.md
```

This means:
- When enrichment runs, only `enrichment.md` is rewritten, then `gem.md` is reassembled
- When a transcript is generated, only `transcript.md` is created, then `gem.md` is reassembled
- External tools (QMD) index `gem.md` — one file, one document per gem
- Developers can inspect individual subfiles for debugging

---

## File Contents

### `meta.json` — Machine-Readable Metadata

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "source_type": "YouTube",
  "source_url": "https://youtube.com/watch?v=abc123",
  "domain": "youtube.com",
  "title": "ECS vs EKS Comparison",
  "author": "TechChannel",
  "captured_at": "2026-02-25T14:30:00Z",
  "project_id": null,
  "source_meta": {
    "duration": "22:15",
    "channel": "TechChannel",
    "recording_filename": null
  },
  "knowledge_version": 1,
  "last_assembled": "2026-02-25T15:00:00Z"
}
```

This is the structured representation — used by the knowledge file generator to assemble `gem.md`, and by any tool that needs machine-parseable gem data.

### `content.md` — Raw Extracted Content

Whatever was captured from the source. Varies by source type:

| Source Type | Content |
|---|---|
| YouTube | Video description + any available subtitles/captions |
| Article/Medium | Readability-extracted body text |
| Gmail/Email | Email body |
| ChatGPT/Claude | Conversation messages |
| Recording | *(empty — transcript is separate)* |
| Browser tab | Generic page extraction |

```markdown
# ECS vs EKS Comparison

## Video Description
In this video we compare Amazon ECS and Amazon EKS for production
container workloads...

## Captions
[00:00] Today we're going to look at two container orchestration...
[00:45] Let's start with ECS. Amazon Elastic Container Service...
```

### `enrichment.md` — AI-Generated Analysis

Created when `IntelProvider::generate_tags()` and `summarize()` run:

```markdown
## Summary
A 22-minute comparison of ECS and EKS covering cost, complexity, and
operational overhead for production container workloads on AWS.

## Tags
- AWS
- ECS
- EKS
- Kubernetes
- Containers

## Enrichment Metadata
- Provider: mlx / Qwen3-8B-4bit
- Enriched: 2026-02-25T15:00:00Z
```

### `transcript.md` — Full Transcript (Recordings Only)

Created when a recording gem gets a high-quality transcript via MLX Omni:

```markdown
## Transcript
Language: en

[00:00:00] So let's kick off the infrastructure planning session.
[00:00:05] We've been looking at ECS versus EKS for the past week.
[00:00:12] I think ECS is the right call for us given the team's experience.
...
```

### `copilot.md` — Co-Pilot Analysis (Recordings Only)

Created when co-pilot data is saved with a recording gem. Consolidated from the co-pilot agent's rolling analysis:

```markdown
## Rolling Summary
The team discussed container orchestration options for the AWS migration,
settling on ECS over EKS due to team experience constraints.

## Key Points
- ECS chosen over EKS for simplicity
- Fargate pricing estimated at $2,400/mo
- Timeline target: March completion

## Decisions
- Use ECS Fargate (not EC2 launch type)
- Skip CloudEndure — workloads already containerized

## Action Items
- Ankit: Evaluate RDS vs Aurora pricing by Friday
- Team: Set up staging VPC by next week

## Open Questions
- VPC peering vs Transit Gateway — needs architect input
- Blue-green deployment strategy for database cutover

## Key Concepts
- ECS Fargate (mentioned 12 times)
- Zero-downtime migration (mentioned 8 times)
- Aurora Serverless (mentioned 5 times)
```

### `gem.md` — The Assembled Document

This is what QMD indexes and what agents consume. Assembled from all subfiles:

```markdown
# ECS vs EKS Comparison

- **Source:** YouTube
- **URL:** https://youtube.com/watch?v=abc123
- **Author:** TechChannel
- **Captured:** 2026-02-25
- **Tags:** AWS, ECS, EKS, Kubernetes, Containers
- **Project:** AWS Migration Q1

## Summary
A 22-minute comparison of ECS and EKS covering cost, complexity, and
operational overhead for production container workloads on AWS.

## Content
In this video we compare Amazon ECS and Amazon EKS for production
container workloads...
[Full content from content.md]

## Transcript
[Full transcript from transcript.md, if present]

## Co-Pilot Analysis
[Decisions, action items, etc. from copilot.md, if present]
```

---

## Generation Pipeline

### When Files Are Created/Updated

| Event | What Happens |
|---|---|
| **Gem created** (save to library) | Create folder → write `meta.json` + `content.md` → assemble `gem.md` |
| **Gem enriched** (AI tags/summary) | Write `enrichment.md` → reassemble `gem.md` |
| **Transcript generated** | Write `transcript.md` → reassemble `gem.md` |
| **Co-pilot data saved** | Write `copilot.md` → reassemble `gem.md` |
| **Gem assigned to project** | Update `meta.json` (project_id) → reassemble `gem.md` (header) |
| **Gem re-enriched** | Overwrite `enrichment.md` → reassemble `gem.md` |
| **Gem deleted** | Delete entire folder |
| **Gem content updated** (URL re-capture) | Overwrite `content.md` → reassemble `gem.md` |

### The Assembler

A single function that reads subfiles and produces `gem.md`:

```rust
/// Assemble gem.md from subfiles in a gem's knowledge folder
pub async fn assemble_gem_md(gem_folder: &Path, meta: &GemMeta) -> Result<String, String> {
    let mut doc = String::new();

    // Header from meta.json
    doc.push_str(&format!("# {}\n\n", meta.title));
    doc.push_str(&format!("- **Source:** {}\n", meta.source_type));
    doc.push_str(&format!("- **URL:** {}\n", meta.source_url));
    if let Some(author) = &meta.author {
        doc.push_str(&format!("- **Author:** {}\n", author));
    }
    doc.push_str(&format!("- **Captured:** {}\n", meta.captured_at));

    // Tags from enrichment (if exists)
    if let Ok(enrichment) = read_subfile(gem_folder, "enrichment.md").await {
        let tags = extract_tags(&enrichment);
        if !tags.is_empty() {
            doc.push_str(&format!("- **Tags:** {}\n", tags.join(", ")));
        }
    }

    // Project (if assigned)
    if let Some(project_id) = &meta.project_id {
        doc.push_str(&format!("- **Project:** {}\n", project_id));
    }

    doc.push_str("\n");

    // Enrichment summary
    if let Ok(enrichment) = read_subfile(gem_folder, "enrichment.md").await {
        doc.push_str(&enrichment);
        doc.push_str("\n\n");
    }

    // Content
    if let Ok(content) = read_subfile(gem_folder, "content.md").await {
        doc.push_str("## Content\n\n");
        doc.push_str(&content);
        doc.push_str("\n\n");
    }

    // Transcript
    if let Ok(transcript) = read_subfile(gem_folder, "transcript.md").await {
        doc.push_str(&transcript);
        doc.push_str("\n\n");
    }

    // Co-pilot
    if let Ok(copilot) = read_subfile(gem_folder, "copilot.md").await {
        doc.push_str("## Co-Pilot Analysis\n\n");
        doc.push_str(&copilot);
        doc.push_str("\n\n");
    }

    Ok(doc)
}
```

### The `KnowledgeStore` Trait — CRUD Contract

Following the same pattern as `IntelProvider`, `SearchProvider`, `GemStore`, and `BrowserAdapter` — the knowledge layer is a **trait**, not a concrete class. Every consumer talks to the contract, never to the implementation.

```
┌─────────────────────────────────────────────────────────┐
│  Jarvis Rust Backend                                    │
│                                                         │
│  GemStore::save()    Enrichment    Transcript   CoPilot │
│       │                  │             │           │    │
│       ▼                  ▼             ▼           ▼    │
│  ┌─────────────────────────────────────────────────┐    │
│  │            KnowledgeStore trait                  │    │
│  │                                                 │    │
│  │  C  create(gem)         → KnowledgeEntry        │    │
│  │  R  get(gem_id)         → KnowledgeEntry        │    │
│  │  R  get_subfile(id, f)  → String                │    │
│  │  U  update(id, file, c) → ()                    │    │
│  │  D  delete(gem_id)      → ()                    │    │
│  │                                                 │    │
│  │  +  get_assembled(id)   → String (gem.md)       │    │
│  │  +  exists(gem_id)      → bool                  │    │
│  │  +  migrate_all(gems)   → MigrationResult       │    │
│  └──────────────┬──────────────────────────────────┘    │
│                 │                                       │
│        ┌────────┼────────┐                              │
│        ▼        ▼        ▼                              │
│  ┌──────────┐ ┌───────────┐ ┌────────────────┐         │
│  │  Local   │ │  Cloud    │ │  NoOp          │         │
│  │ Knowledge│ │ Knowledge │ │ Knowledge      │         │
│  │  Store   │ │  Store    │ │  Store         │         │
│  │ (v1)     │ │ (future)  │ │ (fallback)     │         │
│  └──────────┘ └───────────┘ └────────────────┘         │
│   filesystem    S3/GCS/       returns empty,            │
│   + markdown    API-backed    app still works           │
└─────────────────────────────────────────────────────────┘
```

#### The Trait

```rust
/// What a knowledge entry looks like — provider-agnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub gem_id: String,
    pub assembled: String,              // the full gem.md content
    pub subfiles: Vec<KnowledgeSubfile>,
    pub version: u32,
    pub last_assembled: String,         // ISO 8601
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSubfile {
    pub filename: String,               // "content.md", "enrichment.md", etc.
    pub exists: bool,
    pub size_bytes: u64,
    pub last_modified: Option<String>,  // ISO 8601
}

pub struct MigrationResult {
    pub total: usize,
    pub created: usize,
    pub skipped: usize,
    pub failed: usize,
    pub errors: Vec<(String, String)>,  // (gem_id, error)
}

/// Backend-agnostic knowledge store interface
///
/// CRUD contract for gem knowledge files. Implementations can store
/// files locally (filesystem), remotely (S3, GCS), or in any other
/// backend. Consumers never know which.
///
/// Analogous to GemStore (DB), SearchProvider (search), IntelProvider (AI).
#[async_trait]
pub trait KnowledgeStore: Send + Sync {
    /// Check if the knowledge store is available
    async fn check_availability(&self) -> AvailabilityResult;

    // ── CREATE ──────────────────────────────────────────

    /// Create knowledge entry for a new gem
    ///
    /// Generates all subfiles from the gem data and assembles gem.md.
    /// Idempotent — if files already exist, they are overwritten.
    async fn create(&self, gem: &Gem) -> Result<KnowledgeEntry, String>;

    // ── READ ────────────────────────────────────────────

    /// Get the full knowledge entry for a gem (metadata + subfile listing)
    async fn get(&self, gem_id: &str) -> Result<Option<KnowledgeEntry>, String>;

    /// Get the assembled gem.md content (what search indexes, what agents consume)
    async fn get_assembled(&self, gem_id: &str) -> Result<Option<String>, String>;

    /// Get a specific subfile's content
    async fn get_subfile(
        &self,
        gem_id: &str,
        filename: &str,     // "enrichment.md", "transcript.md", etc.
    ) -> Result<Option<String>, String>;

    /// Check if a gem has knowledge files
    async fn exists(&self, gem_id: &str) -> Result<bool, String>;

    // ── UPDATE ──────────────────────────────────────────

    /// Update a specific subfile and reassemble gem.md
    ///
    /// This is the primary write operation. Enrichment writes "enrichment.md",
    /// transcription writes "transcript.md", etc. After writing the subfile,
    /// the implementation reassembles gem.md automatically.
    ///
    /// Implementations MUST handle concurrent calls for the same gem_id safely.
    async fn update_subfile(
        &self,
        gem_id: &str,
        filename: &str,
        content: &str,
    ) -> Result<(), String>;

    /// Force reassemble gem.md from existing subfiles
    ///
    /// Useful after format version bumps or manual subfile edits.
    async fn reassemble(&self, gem_id: &str) -> Result<(), String>;

    // ── DELETE ──────────────────────────────────────────

    /// Delete all knowledge files for a gem
    async fn delete(&self, gem_id: &str) -> Result<(), String>;

    /// Delete a specific subfile and reassemble gem.md
    async fn delete_subfile(
        &self,
        gem_id: &str,
        filename: &str,
    ) -> Result<(), String>;

    // ── BULK ────────────────────────────────────────────

    /// Generate knowledge files for all gems (migration / rebuild)
    ///
    /// Emits progress events via the event_emitter.
    /// Implementations should handle this efficiently (batch I/O, parallelism).
    async fn migrate_all(
        &self,
        gems: Vec<Gem>,
        event_emitter: &(dyn KnowledgeEventEmitter + Sync),
    ) -> Result<MigrationResult, String>;

    /// List all gem_ids that have knowledge files
    async fn list_indexed(&self) -> Result<Vec<String>, String>;
}
```

#### Why CRUD?

Every operation on knowledge files maps cleanly to CRUD:

| Operation | CRUD | Trait Method | Who Calls It |
|---|---|---|---|
| Gem saved to library | **C**reate | `create(gem)` | `GemStore::save()` |
| View gem's knowledge | **R**ead | `get_assembled(gem_id)` | Summarizer, Export, UI |
| Read a specific part | **R**ead | `get_subfile(gem_id, "transcript.md")` | Agents needing specific data |
| Enrichment completes | **U**pdate | `update_subfile(id, "enrichment.md", content)` | Enrichment command |
| Transcript generated | **U**pdate | `update_subfile(id, "transcript.md", content)` | Transcript command |
| Co-pilot data saved | **U**pdate | `update_subfile(id, "copilot.md", content)` | Co-pilot agent |
| Gem assigned to project | **U**pdate | `update_subfile(id, "meta", ...)` + `reassemble` | Project assignment |
| Gem deleted | **D**elete | `delete(gem_id)` | `GemStore::delete()` |

Callers never know if "update enrichment" writes a file to disk, uploads to S3, or posts to an API. They just call `update_subfile()`.

### `LocalKnowledgeStore` — First Implementation (Filesystem)

The filesystem-based implementation that writes markdown files to disk:

```rust
pub struct LocalKnowledgeStore {
    /// Root knowledge directory
    /// ~/Library/Application Support/com.jarvis.app/knowledge/
    base_path: PathBuf,
    /// Per-gem locks to serialize concurrent writes
    gem_locks: DashMap<String, Arc<Mutex<()>>>,
    /// Event emitter for progress notifications
    event_emitter: Arc<dyn KnowledgeEventEmitter + Send + Sync>,
}
```

This is the only implementation for v1. It writes to the local filesystem as described in the "Structure" section above. But because it implements `KnowledgeStore`, swapping it out is a one-line change.

### Future Implementations

| Implementation | Backend | Use Case |
|---|---|---|
| `LocalKnowledgeStore` | Filesystem (`knowledge/`) | Default. Local-first. |
| `CloudKnowledgeStore` | S3 / GCS / Azure Blob | Multi-device sync. Gems accessible from any machine. |
| `ApiKnowledgeStore` | REST API | Team/enterprise. Central knowledge server. |
| `HybridKnowledgeStore` | Local + Cloud sync | Local speed with cloud backup. Write locally, sync async. |
| `NoOpKnowledgeStore` | None | Graceful fallback. App works without knowledge files. |

The `HybridKnowledgeStore` is particularly interesting — it could wrap `LocalKnowledgeStore` and async-replicate to cloud:

```rust
pub struct HybridKnowledgeStore {
    local: LocalKnowledgeStore,
    remote: Box<dyn KnowledgeStore>,
    sync_queue: mpsc::Sender<SyncJob>,  // background sync
}

// Writes go to local immediately, then queue for remote sync
// Reads always hit local (fast), with remote as fallback
```

### Trait Consistency Across Jarvis

With this, Jarvis has a clean provider pattern across all layers:

```
┌────────────────────────────────────────────┐
│  Layer            │  Trait                  │
├────────────────────────────────────────────┤
│  Database         │  GemStore              │
│  Intelligence     │  IntelProvider         │
│  Knowledge Files  │  KnowledgeStore   ← NEW│
│  Search           │  SearchProvider        │
│  Browser          │  BrowserAdapter        │
│  Transcription    │  TranscriptionProvider │
└────────────────────────────────────────────┘

Every layer: trait → concrete impl → swappable
Every layer: has a NoOp fallback → graceful degradation
```
```

### Concurrent Write Safety

Enrichment, transcription, and co-pilot can finish at overlapping times for the same gem. Each triggers `update_subfile()` → `reassemble()`. Without coordination, two reassemblies could race and produce a corrupted `gem.md`.

**Solution:** Per-gem mutex via `DashMap<String, Arc<Mutex<()>>>`.

```
Enrichment finishes → update_subfile("enrichment.md")
                         │
                         ├── acquire lock for gem_id
                         ├── write enrichment.md
                         ├── reassemble gem.md
                         └── release lock
                                                    Transcript finishes → update_subfile("transcript.md")
                                                         │
                                                         ├── acquire lock for gem_id (waits if held)
                                                         ├── write transcript.md
                                                         ├── reassemble gem.md (now includes enrichment too)
                                                         └── release lock
```

This is lightweight — `DashMap` is lock-free for reads, and the per-gem mutex only serializes writes to the *same* gem. Different gems write in parallel.

### Progress Events — Keeping the User Informed

All file operations emit Tauri events so the frontend can show what's happening:

```rust
/// Event trait for knowledge file operations
pub trait KnowledgeEventEmitter: Send + Sync {
    fn emit_progress(&self, event: KnowledgeEvent);
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum KnowledgeEvent {
    /// A subfile was updated for a gem
    SubfileUpdated {
        gem_id: String,
        filename: String,      // "enrichment.md", "transcript.md"
        status: String,        // "writing", "assembling", "done"
    },
    /// Migration progress
    MigrationProgress {
        current: usize,
        total: usize,
        gem_id: String,
        gem_title: String,
        status: String,        // "generating", "done", "failed"
    },
    /// Migration complete
    MigrationComplete {
        result: MigrationResult,
    },
}
```

**Frontend UX for migration (first launch after upgrade):**

```
┌──────────────────────────────────────────┐
│  Setting up Knowledge Files              │
│                                          │
│  ████████████░░░░░░░░  47/100 gems       │
│                                          │
│  Currently: "ECS vs EKS Comparison"      │
│                                          │
│  This is a one-time setup. Your gems     │
│  are being indexed for smart search.     │
└──────────────────────────────────────────┘
```

**Frontend UX for ongoing updates (subtle, non-blocking):**

When a gem's knowledge files update in the background (e.g., enrichment just finished), show a brief indicator on the gem card or detail view — a small sync icon that appears and fades. No modal, no blocking. Just awareness.
```

---

## Integration Points — Where This Plugs In

### Existing Flows (modify)

| Current Code | Change |
|---|---|
| `GemStore::save()` | After DB save → call `knowledge_store.create(gem)` |
| Enrichment command | After enriching gem → call `knowledge_store.update_subfile(id, "enrichment.md", ...)` |
| Transcript command | After transcript → call `knowledge_store.update_subfile(id, "transcript.md", ...)` |
| Co-pilot save | After saving copilot data → call `knowledge_store.update_subfile(id, "copilot.md", ...)` |
| `GemStore::delete()` | After DB delete → call `knowledge_store.delete(id)` |

All callers use `knowledge_store` (the injected `dyn KnowledgeStore`) — never a concrete type.

### New Consumers

| Consumer | How It Uses Knowledge Files |
|---|---|
| **QMD / SearchProvider** | Indexes `gem.md` files in the `knowledge/` directory |
| **Summarizer Agent** | Reads `gem.md` files for project gems, feeds to LLM |
| **Setup Agent** | Uses search results (powered by `gem.md` index) for gem matching |
| **Export** | Can export a gem's folder as-is — it's already a portable package |

---

## Source of Truth: DB vs Files

**The database remains the source of truth.** Knowledge files are derived artifacts.

```
SQLite (gems.db)                    Knowledge Files (knowledge/)
┌──────────────────┐                ┌──────────────────────────┐
│ Source of truth   │  ──generates─► │ Derived, regenerable     │
│ Structured data   │                │ Optimized for search     │
│ Queried by app    │                │ Consumed by QMD, agents  │
│ Transactional     │                │ Human-readable           │
└──────────────────┘                └──────────────────────────┘
```

**If knowledge files are lost or corrupted:**
- Run `knowledge_store.migrate_all(gems)` to regenerate everything from DB
- No data loss — files are just a different view of DB data

**If DB and files disagree:**
- DB wins. Regenerate the file.
- `knowledge_store.reassemble(gem_id)` can be called on any gem at any time to sync.

---

## Migration Strategy (Existing Gems)

Existing installations have gems in SQLite with no knowledge files. On first launch after upgrade:

```
App launches
    │
    ├── Check: does knowledge/ directory exist?
    │   └── No → first time. Run migration.
    │
    ├── Migration:
    │   1. Read all gems from SQLite
    │   2. For each gem:
    │      a. Create knowledge/{gem_id}/
    │      b. Write meta.json from gem fields
    │      c. Write content.md from gem.content
    │      d. Write enrichment.md from gem.ai_enrichment (if exists)
    │      e. Write transcript.md from gem.transcript (if exists)
    │      f. Assemble gem.md
    │   3. Log migration result (created/skipped/failed)
    │
    ├── Mark migration complete (write version marker)
    │
    └── Continue startup
```

**Performance estimate:** ~50ms per gem (mostly file I/O). 100 gems = ~5 seconds. 1000 gems = ~50 seconds. Show a progress indicator for large libraries.

### Co-Pilot Log Migration

Today, co-pilot agent logs live in a separate `agent_logs/` directory as timestamped markdown files. These aren't linked to gems by ID — only by recording filename embedded in the log header and timestamp proximity.

**Simple approach for v1:**

```
For each recording gem:
  1. Extract recording_filename from gem.source_meta
  2. Scan agent_logs/ for files containing that filename in their header
  3. If exactly one match → copy to copilot.md
  4. If multiple matches → take the one closest in timestamp to gem.captured_at
  5. If no match → skip (copilot.md won't exist for this gem)
```

| Case | Action | Extensibility |
|---|---|---|
| Exact filename match in log header | Copy to `copilot.md` | Reliable, covers most cases |
| Multiple matches | Closest timestamp wins | Could add smarter matching later |
| No match | Skip gracefully | User can manually place a copilot.md later |
| Future recordings | Co-pilot writes directly to `knowledge/{gem_id}/copilot.md` | No migration needed |

The old `agent_logs/` directory is **not deleted** after migration — it stays as a backup. A future cleanup command can remove it once the user confirms everything migrated correctly.

---

## Versioning

Knowledge files have a format version (`knowledge_version` in meta.json). If the format changes in future releases:

```
App launches
    │
    ├── Read knowledge_version from any gem's meta.json
    │
    ├── Version matches current? → continue
    │
    └── Version outdated? → run format migration
        (regenerate all gem.md files with new template)
```

This ensures the assembled `gem.md` format can evolve (e.g., adding new sections) without breaking existing files.

---

## Decided

1. **Architecture** — `KnowledgeStore` trait with CRUD contract. `LocalKnowledgeStore` (filesystem) is v1. Swappable for cloud, API, or hybrid — same pattern as `IntelProvider`, `SearchProvider`, `GemStore`.
2. **File size limits** — include full content in `gem.md` for v1. QMD chunks large files anyway. The assembler is designed so a truncation strategy can be added later without changing the trait contract or any consumer.
3. **Concurrent writes** — per-gem mutex via `DashMap` in `LocalKnowledgeStore`. The trait contract requires implementations to handle concurrent `update_subfile()` calls safely — how they do it is their problem.
4. **External edits** — files are derived from DB, not the other way around. No file watching in v1. `reassemble()` always overwrites `gem.md` from subfiles.
5. **Agent log backfill** — simple filename match in log header, timestamp tiebreaker. Old `agent_logs/` directory kept as backup. See "Co-Pilot Log Migration" above.
6. **Storage overhead** — accepted. Content lives in both DB and files. If storage becomes an issue later, the content column in SQLite could be dropped in favor of reading from `content.md` — the `KnowledgeStore` abstraction makes this a localized change.
7. **Progress visibility** — all file operations emit Tauri events via `KnowledgeEventEmitter`. Migration shows a progress bar. Ongoing updates show a subtle sync indicator on gem cards.

## Open Questions

1. **gem.md template strictness** — how rigid should the heading structure be? Stricter = better QMD chunking. Looser = more natural reading. Leaning strict.
2. **Subfile extensibility** — when a new data type arrives (e.g., screenshots, cached web pages), it gets its own subfile and a new section in `gem.md`. Does the assembler need a plugin/registry pattern or is a simple ordered list of known filenames enough for now?
3. **Regeneration trigger** — should there be a UI button to "Regenerate knowledge files" for a single gem or all gems? Useful for debugging and after format version bumps.
