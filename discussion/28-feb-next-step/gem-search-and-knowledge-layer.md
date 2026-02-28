# Gem Search & Knowledge Layer — Making Gems Truly Searchable

> **Depends on:** [Gem Knowledge Files](./gem-knowledge-files.md) — the filesystem layer that generates the .md files this search layer indexes.
>
> **Powers:** [Projects & Summarizer](./projects-and-summarizer.md) — the Setup Agent's gem matching (Phase 3) and the Summarizer Agent's content retrieval.

---

## The Gap

Jarvis currently has **SQLite FTS5** for full-text search across gem titles, descriptions, and content. This is keyword-based — it finds "Postgres migration" but not "database move" or "RDS transition."

For the project system to work well, we need two things:

1. **Semantic search** — find gems by meaning, not just exact words
2. **Rich, indexable content** — each gem needs a complete, searchable document, not just scattered DB columns

Without this, the Setup Agent's gem matching is reduced to keyword overlap, and the Summarizer can't intelligently weigh which gems matter most.

---

## Proposal: Gem Knowledge Files (.md)

Each gem gets a companion markdown file — a **materialized document** that consolidates everything known about that gem into one searchable, readable artifact.

### What Goes in the File

```markdown
# ECS vs EKS Comparison — Container Orchestration on AWS

- **Source:** YouTube
- **URL:** https://youtube.com/watch?v=abc123
- **Captured:** 2026-02-25
- **Tags:** AWS, ECS, EKS, Kubernetes, containers
- **Project:** AWS Migration Q1

## Summary
A 22-minute technical comparison of Amazon ECS and EKS for production
container workloads, covering cost, complexity, and operational overhead.

## Key Points
- ECS is simpler for teams without existing Kubernetes experience
- EKS provides multi-cloud portability via standard K8s APIs
- ECS Fargate pricing is ~30% cheaper for equivalent workloads
- EKS requires dedicated ops investment for cluster management

## Content / Transcript
[Full extracted content or transcript here]

## AI Enrichment
- One-line summary: "ECS wins on simplicity and cost; EKS wins on portability"
- Related topics: serverless containers, Fargate, managed Kubernetes

## Co-Pilot Notes (if from a recording)
- Decisions: [...]
- Action items: [...]
- Open questions: [...]
```

### Why Markdown Files?

| Benefit | Explanation |
|---|---|
| **Portable** | Readable without Jarvis — plain text, works anywhere |
| **Indexable** | External tools (grep, ripgrep, qmd) can search them |
| **Embeddable** | Easy to chunk and generate embeddings from structured sections |
| **Diffable** | Can be version-controlled with git if user wants |
| **Composable** | Summarizer agent can concatenate files, not query a DB |
| **Debuggable** | User can open a file and see exactly what Jarvis knows |

### Storage Location

```
~/Library/Application Support/com.jarvis.app/
├── gems.db                          # SQLite (structured data, FTS5, metadata)
├── knowledge/                       # Knowledge files
│   ├── 550e8400-e29b-.../gem.md     # One folder per gem (by UUID)
│   ├── 6ba7b810-9dad-.../gem.md
│   └── ...
```

Each gem gets a folder (not just a file) to allow future expansion — attachments, images, cached web snapshots, etc.

### Lifecycle

| Event | Action |
|---|---|
| Gem created | Generate `gem.md` from DB fields + extracted content |
| Gem enriched | Update `gem.md` with AI summary, tags, key points |
| Gem re-enriched | Regenerate `gem.md` |
| Gem assigned to project | Update the `Project:` field in `gem.md` |
| Gem deleted | Delete the knowledge folder |
| Co-pilot data saved | Append co-pilot sections to `gem.md` |

The .md file is a **derived artifact** — the DB remains the source of truth. If the file gets corrupted or lost, it can be regenerated from DB data. But it's the primary input for search and agent workflows.

---

## Search Architecture: Two Layers

### Layer 1 — Full-Text Search (already exists, enhance)

**What:** SQLite FTS5 — keyword matching with ranking.

**Current state:** Works on title, description, content columns in gems table.

**Enhancement:** Also index the `gem.md` content as a single FTS5 document, so searches hit the consolidated view including AI enrichment, co-pilot notes, etc.

```sql
-- Existing FTS5 table (or updated version)
CREATE VIRTUAL TABLE gems_fts USING fts5(
    title, description, content, enrichment_summary, tags,
    content=gems, content_rowid=rowid
);
```

**Good for:** Exact keyword search ("Postgres migration"), filtering ("show me all gems tagged AWS"), quick lookups.

### Layer 2 — Semantic Search (new)

**What:** Vector embeddings + similarity search. Find gems by meaning.

**How it works:**
1. For each gem, generate an embedding vector from the `gem.md` content
2. Store the vector alongside the gem
3. At query time, embed the query and find nearest neighbors

**Query examples that only semantic search can handle:**
- "What do I know about moving databases to the cloud?" → finds "Zero-downtime Postgres migration" even though "cloud" never appears in the gem
- "Anything about container costs?" → finds the ECS vs EKS video and the AWS pricing calculator browsing session
- Setup Agent asks: "Which gems relate to 'Migrate monolith to AWS ECS'?" → semantic match, not keyword

### Embedding Generation (Local-First)

Since Jarvis is local-first, embeddings must be generated on-device.

**Option A: MLX Embeddings (recommended)**

Use a small embedding model via the existing MLX sidecar:

| Model | Dimensions | Size | Speed |
|---|---|---|---|
| `all-MiniLM-L6-v2` | 384 | ~80MB | ~5ms/doc |
| `nomic-embed-text-v1.5` | 768 | ~270MB | ~10ms/doc |
| `bge-small-en-v1.5` | 384 | ~130MB | ~5ms/doc |

These are small enough to bundle or auto-download. Separate from the LLM — embedding models are tiny.

**Option B: Apple's NaturalLanguage framework**

macOS has built-in sentence embeddings via `NLEmbedding`. No external model needed, but:
- Fixed dimensions (512)
- English-centric
- Less control over quality
- Available from Rust via objc bindings

**Recommendation:** Start with Option A (MLX embedding model). It's higher quality, the MLX sidecar already exists, and it's one more model download — not a new architecture.

### Vector Storage

**Option A: sqlite-vec (recommended)**

A SQLite extension for vector similarity search. Fits perfectly:
- Jarvis already uses SQLite — no new database
- Stores vectors in a virtual table
- Supports cosine similarity, L2 distance
- Can be loaded as a SQLite extension in Rust via `rusqlite`

```sql
-- Load extension
SELECT load_extension('vec0');

-- Create vector table
CREATE VIRTUAL TABLE gem_embeddings USING vec0(
    gem_id TEXT PRIMARY KEY,
    embedding FLOAT[384]    -- dimension matches the model
);

-- Insert embedding
INSERT INTO gem_embeddings(gem_id, embedding)
VALUES ('550e8400-e29b-...', vec_f32('[0.12, -0.34, ...]'));

-- Semantic search: find 10 most similar gems to a query embedding
SELECT gem_id, distance
FROM gem_embeddings
WHERE embedding MATCH vec_f32('[0.05, -0.22, ...]')
ORDER BY distance
LIMIT 10;
```

**Option B: Qdrant**

Full-featured vector database. Overkill for local use — it's a server you'd need to run alongside Jarvis. Better suited for cloud deployments.

**Option C: In-memory brute force**

For < 1000 gems, you could just load all embeddings into memory and compute cosine similarity in Rust. No extension needed. Dead simple but doesn't scale.

**Recommendation:** sqlite-vec. Zero new infrastructure, lives inside the existing DB, and handles the scale Jarvis will see (hundreds to low thousands of gems).

---

## Combined Search: How the Layers Work Together

```
User query: "What do I know about cloud database options?"
                    │
                    ├──► Layer 1 (FTS5): keyword match
                    │    Results: gems containing "cloud", "database", "options"
                    │    Score: BM25 relevance
                    │
                    ├──► Layer 2 (Semantic): vector similarity
                    │    Results: gems about RDS, Aurora, Postgres migration,
                    │             pricing comparison — even without exact keywords
                    │    Score: cosine similarity
                    │
                    ▼
              Merge & Rank
              ┌─────────────────────────────────────┐
              │ Reciprocal Rank Fusion (RRF)         │
              │ or weighted score combination        │
              │                                      │
              │ FTS5 weight:     0.3                  │
              │ Semantic weight: 0.7                  │
              │ Recency boost:   +0.1 for < 7 days   │
              └─────────────────────────────────────┘
                    │
                    ▼
              Final ranked results
```

This is called **hybrid search** — combining keyword precision with semantic recall. The FTS5 layer catches exact matches the vector search might rank lower, and the vector layer catches conceptual matches FTS5 would completely miss.

---

## How This Powers the Project System

### Setup Agent (Phase 3 of projects-and-summarizer.md)

When the Setup Agent needs to match gems to a project profile:

```
Project profile:
  Objective: "Migrate monolith to AWS ECS with zero downtime"
  Topics: "AWS, ECS, Docker, Postgres, migration"

Step 1: Embed the objective → query vector
Step 2: Semantic search against gem_embeddings → ranked candidates
Step 3: FTS5 search for topic keywords → more candidates
Step 4: Merge with RRF → final ranked list with scores
Step 5: LLM generates one-line reason for top N matches
```

This is much richer than pure keyword matching — it would catch a gem about "blue-green deployment strategies" even if it never mentions "migration."

### Summarizer Agent (Phase 2 of projects-and-summarizer.md)

The summarizer can use semantic search to:
- Find the most important gems in a project (rank by relevance to objective)
- Identify themes across gems (cluster embeddings to find topic groups)
- Detect redundancy (two gems with very similar embeddings = overlapping content)

### General Gem Search (existing feature, enhanced)

The search bar in the Gems UI currently does FTS5. Adding semantic search means:
- Fuzzy/conceptual queries work ("stuff about costs" finds pricing-related gems)
- Search works across languages if the embedding model supports it
- Typo-tolerant (embeddings are robust to surface form variation)

---

## Embedding Lifecycle

| Event | Action |
|---|---|
| Gem created | Generate embedding from title + summary (fast, small text) |
| Gem enriched | Regenerate embedding from full `gem.md` content (richer) |
| Gem content updated | Regenerate embedding |
| Gem deleted | Delete embedding row |
| First app launch (migration) | Batch-embed all existing gems |

Embedding generation is fast (~5ms per gem) so it can happen synchronously during gem save, or batched in the background for migrations.

---

## Data Model Changes

### New: `gem_embeddings` (sqlite-vec virtual table)

```sql
CREATE VIRTUAL TABLE gem_embeddings USING vec0(
    gem_id TEXT PRIMARY KEY,
    embedding FLOAT[384]
);
```

### New Column on `gems` (optional, for caching)

```sql
ALTER TABLE gems ADD COLUMN knowledge_path TEXT;
-- path to the gem.md file, e.g., "knowledge/550e8400-.../gem.md"
```

### Embedding Model Config in Settings

```json
{
  "embedding_model": "all-MiniLM-L6-v2",
  "embedding_dimensions": 384,
  "search_semantic_weight": 0.7,
  "search_fts_weight": 0.3
}
```

---

## Phased Delivery (aligned with project phases)

### Step 1 — Knowledge Files + SearchProvider Trait (ship with Projects Phase 1)

- Define `SearchProvider` trait, `SearchResult`, `SearchOptions` types
- Implement `NoOpSearchProvider` (FTS5-only fallback)
- Generate `gem.md` on gem creation/enrichment
- Migration: batch-generate gem.md for all existing gems
- Expose `search_gems(query, options)` Tauri command that uses `SearchProvider`
- Existing FTS5 search migrated to go through the trait

### Step 2 — QMD Sidecar Integration (ship with Projects Phase 2)

- Implement `QmdSearchProvider` (CLI mode)
- Sidecar lifecycle: detect QMD, setup collection, manage process
- QMD setup prompt in Settings ("Install QMD for semantic search?")
- Auto-index: `qmd update && qmd embed` after gem save
- First-run migration: index all existing gem.md files
- Existing search bar upgraded to hybrid search via QMD

### Step 3 — Agent-Powered Search (ship with Projects Phase 3)

- Setup Agent uses `SearchProvider::search()` for gem matching during project creation
- `find_similar()` powers gem suggestions
- Upgrade to MCP HTTP mode if CLI latency is too high
- Push project descriptions as QMD collection contexts

### Step 4 — Search UI Enhancements (optional polish)

- "Similar gems" on gem detail view (`find_similar`)
- Search within a project scope (`SearchOptions::project_id`)
- Search result explanations (matched chunk display)

---

## QMD — Could Jarvis Use It?

[QMD](https://github.com/tobi/qmd) (by Tobi Luetke / Shopify) is a local-first CLI search engine for markdown knowledge bases. It's strikingly similar to what we're proposing here — worth evaluating seriously.

### What QMD Does

QMD combines **three search layers** — exactly the hybrid approach we described above, plus a reranking step:

```
Query → Query Expansion (LLM generates variations)
      → Parallel retrieval:
          ├── BM25 (SQLite FTS5) — keyword matching
          └── Vector search (sqlite-vec) — semantic similarity
      → Reciprocal Rank Fusion (merge results)
      → LLM Reranker (scores relevance yes/no)
      → Final ranked results
```

| Feature | QMD | Our Proposal |
|---|---|---|
| Full-text search | SQLite FTS5 | SQLite FTS5 |
| Vector search | sqlite-vec | sqlite-vec |
| Embedding model | `embedding-gemma-300M` (~300MB) | MiniLM/nomic-embed (~80-270MB) |
| Reranker | `qwen3-reranker-0.6b` (~640MB) | Not proposed (LLM reason generation instead) |
| Query expansion | Fine-tuned 1.7B model (~1.1GB) | Not proposed |
| Fusion strategy | RRF + position-aware blending | RRF + weighted scores |
| Chunking | Smart markdown-aware (900 tokens, 15% overlap) | Section-based (per gem.md template) |
| Runtime | Node.js (node-llama-cpp) | Rust + MLX sidecar |
| Storage | `~/.cache/qmd/index.sqlite` | Jarvis's `gems.db` |
| MCP server | Yes (stdio + HTTP) | No (native Tauri commands) |

### QMD's Extra Tricks We Don't Have

1. **Query expansion** — before searching, an LLM generates alternative phrasings of the query. "database costs" becomes ["database pricing", "cloud db expenses", "RDS Aurora cost comparison"]. This dramatically improves recall.
2. **LLM reranker** — after retrieval, a small model scores each result for relevance. Catches false positives from vector search.
3. **Collection contexts** — you can annotate directories with descriptions ("this folder contains meeting notes from the infra team") that help the search understand document relationships.
4. **Smart chunking** — respects markdown structure (headings, code blocks, paragraphs) instead of cutting at arbitrary token boundaries.

### Integration Decision: QMD as Sidecar Behind a Trait

Jarvis already uses this pattern successfully — `IntelProvider` trait abstracts MLX vs IntelligenceKit vs NoOp, so commands never know which backend is running. We do the same for search.

**The principle:** Jarvis talks to a `SearchProvider` trait. QMD is the first implementation. If we later want to replace it with a native Rust implementation, Qdrant, or some future tool — only the provider changes. Everything above the trait (commands, frontend, agents) stays untouched.

```
┌─────────────────────────────────────────────────────────┐
│  Jarvis Rust Backend                                    │
│                                                         │
│  Tauri Commands          Agents                         │
│  ┌──────────────┐       ┌──────────────┐                │
│  │ search_gems  │       │ Setup Agent  │                │
│  │ similar_gems │       │ Summarizer   │                │
│  └──────┬───────┘       └──────┬───────┘                │
│         │                      │                        │
│         ▼                      ▼                        │
│  ┌─────────────────────────────────────┐                │
│  │        SearchProvider trait          │                │
│  │                                     │                │
│  │  fn search(query, opts) → results   │                │
│  │  fn semantic_search(query) → results│                │
│  │  fn index_document(doc) → ok        │                │
│  │  fn remove_document(id) → ok        │                │
│  │  fn similar(doc_id, k) → results    │                │
│  │  fn check_availability() → status   │                │
│  └──────────────┬──────────────────────┘                │
│                 │                                       │
│        ┌────────┴────────┐                              │
│        ▼                 ▼                              │
│  ┌───────────┐    ┌────────────┐    ┌────────────┐     │
│  │ QmdSearch │    │ NativeSearch│   │ NoOpSearch │     │
│  │ Provider  │    │ Provider   │    │ Provider   │     │
│  │ (v1)      │    │ (future)   │    │ (fallback) │     │
│  └─────┬─────┘    └────────────┘    └────────────┘     │
│        │                                               │
└────────┼───────────────────────────────────────────────┘
         │ CLI / MCP
         ▼
   ┌───────────┐
   │    QMD    │
   │  sidecar  │
   │ (Node.js) │
   └───────────┘
```

### The `SearchProvider` Trait

Following the same pattern as `IntelProvider` ([provider.rs](../jarvis-app/src-tauri/src/intelligence/provider.rs)):

```rust
/// A single search result returned by any search provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub gem_id: String,
    pub score: f64,              // normalized 0.0–1.0
    pub matched_chunk: String,   // the text snippet that matched
    pub match_type: MatchType,   // how this result was found
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchType {
    Keyword,     // FTS/BM25 match
    Semantic,    // vector similarity
    Hybrid,      // both layers contributed
}

/// Search options — callers configure what they need
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub limit: usize,                    // max results (default 10)
    pub project_id: Option<String>,      // scope to a project
    pub min_score: Option<f64>,          // filter low-confidence results
    pub search_mode: SearchMode,         // keyword / semantic / hybrid
}

#[derive(Debug, Clone, Default)]
pub enum SearchMode {
    Keyword,
    Semantic,
    #[default]
    Hybrid,
}

/// Backend-agnostic search provider interface
///
/// Abstracts the search engine, enabling swappable implementations
/// (QMD, native sqlite-vec, external API) without modifying commands,
/// agents, or frontend code.
#[async_trait]
pub trait SearchProvider: Send + Sync {
    /// Check if the search backend is available and ready
    async fn check_availability(&self) -> AvailabilityResult;

    /// Index a gem's knowledge file for searching
    ///
    /// Called when a gem is created or updated. The provider is responsible
    /// for extracting text, generating embeddings, and storing them.
    async fn index_document(&self, gem_id: &str, content: &str) -> Result<(), String>;

    /// Remove a gem from the search index
    async fn remove_document(&self, gem_id: &str) -> Result<(), String>;

    /// Search across all indexed gems
    ///
    /// Returns ranked results. The provider decides how to combine
    /// keyword and semantic signals based on SearchMode.
    async fn search(
        &self,
        query: &str,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult>, String>;

    /// Find gems similar to a given gem
    ///
    /// Uses the gem's embedding to find nearest neighbors.
    /// Useful for "related gems" on gem detail view and
    /// deduplication detection.
    async fn find_similar(
        &self,
        gem_id: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, String>;

    /// Re-index all documents (migration / rebuild)
    ///
    /// Called on first setup or when switching providers.
    /// Default implementation iterates and calls index_document.
    async fn reindex_all(&self, documents: Vec<(String, String)>) -> Result<usize, String> {
        let mut count = 0;
        for (gem_id, content) in documents {
            self.index_document(&gem_id, &content).await?;
            count += 1;
        }
        Ok(count)
    }
}
```

### QMD Sidecar Implementation (`QmdSearchProvider`)

The first concrete implementation talks to QMD via CLI or MCP HTTP:

```rust
pub struct QmdSearchProvider {
    /// Path to the qmd binary
    qmd_path: PathBuf,
    /// Path to the knowledge/ directory QMD indexes
    collection_path: PathBuf,
    /// MCP HTTP port if running in server mode
    mcp_port: Option<u16>,
    /// Process handle for the QMD sidecar
    process: Option<Child>,
}
```

**Two interaction modes:**

| Mode | How | When to use |
|---|---|---|
| **CLI** | Shell out to `qmd query "..."`, parse JSON output | Simple, no long-running process. Good for development. |
| **MCP HTTP** | QMD runs as `qmd mcp --http --port 8377`, Jarvis calls HTTP endpoints | Better for production — no per-query startup cost, connection pooling. |

**Recommendation:** Start with CLI mode (simpler), switch to MCP HTTP when query latency matters.

```rust
#[async_trait]
impl SearchProvider for QmdSearchProvider {
    async fn check_availability(&self) -> AvailabilityResult {
        // Check: qmd binary exists? Node.js ≥22 installed? Collection indexed?
        // `qmd status --json` returns health info
    }

    async fn index_document(&self, gem_id: &str, content: &str) -> Result<(), String> {
        // 1. Write/update knowledge/{gem_id}/gem.md
        // 2. `qmd update` to re-index the collection
        // 3. `qmd embed` to generate/update embeddings
        // (or if MCP mode: call qmd_index endpoint)
    }

    async fn remove_document(&self, gem_id: &str) -> Result<(), String> {
        // 1. Delete knowledge/{gem_id}/gem.md
        // 2. `qmd update` to re-index
    }

    async fn search(&self, query: &str, options: SearchOptions) -> Result<Vec<SearchResult>, String> {
        // Map SearchMode to QMD command:
        //   Keyword  → `qmd search "query" --json --limit N`
        //   Semantic → `qmd vsearch "query" --json --limit N`
        //   Hybrid   → `qmd query "query" --json --limit N`
        //
        // Parse JSON output → map to Vec<SearchResult>
        // Map QMD's doc IDs back to gem_ids (filename = gem_id)
    }

    async fn find_similar(&self, gem_id: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
        // `qmd vsearch` with the gem's content as query
        // Or: get the gem's embedding, query directly
    }
}
```

### Sidecar Lifecycle Management

Same pattern as the MLX sidecar — Jarvis manages the process:

```
App launches
    │
    ├── Check: is QMD installed? (`qmd --version`)
    │   ├── No  → SearchProvider = NoOpSearchProvider (FTS5 only fallback)
    │   └── Yes → continue
    │
    ├── Check: is collection set up? (`qmd status`)
    │   ├── No  → `qmd collection add ~/Library/.../knowledge --name jarvis-gems`
    │   │         `qmd embed` (batch index existing gems)
    │   └── Yes → continue
    │
    ├── Start MCP server (if HTTP mode)
    │   `qmd mcp --http --port 8377`
    │   Store process handle for cleanup
    │
    └── SearchProvider = QmdSearchProvider { ready: true }

App quits
    │
    └── Kill QMD MCP process (if running)
```

### NoOp Fallback

If QMD isn't installed, search degrades gracefully — exactly like how IntelProvider falls back to NoOp:

```rust
pub struct NoOpSearchProvider;

#[async_trait]
impl SearchProvider for NoOpSearchProvider {
    async fn check_availability(&self) -> AvailabilityResult {
        AvailabilityResult {
            available: false,
            reason: Some("No search provider configured. Install QMD for semantic search.".into()),
        }
    }

    async fn search(&self, query: &str, options: SearchOptions) -> Result<Vec<SearchResult>, String> {
        // Fall back to existing FTS5 search on gems table
        // Keyword search still works — semantic doesn't
    }

    async fn find_similar(&self, _gem_id: &str, _limit: usize) -> Result<Vec<SearchResult>, String> {
        Err("Similar gem search requires a search provider (QMD).".into())
    }

    // index_document / remove_document are no-ops
}
```

### What This Enables Down the Line

The trait boundary means any of these future swaps are **localized changes** — one new file, one provider switch:

| Future Provider | Why you might switch |
|---|---|
| `NativeSearchProvider` | Build search in Rust with sqlite-vec + MLX embeddings. No Node.js dependency. |
| `OllamaSearchProvider` | If user already runs Ollama, use its embedding endpoint. |
| `CloudSearchProvider` | Optional cloud mode for users who want hosted search (Pinecone, Weaviate, etc.) |
| `QdrantSearchProvider` | If Jarvis goes multi-device, Qdrant handles distributed vector search. |

The frontend, Tauri commands, Setup Agent, and Summarizer Agent never change — they talk to `SearchProvider`, not to QMD.

### QMD Setup & Configuration

**Installation:** QMD requires Node.js ≥22. Two options for bundling:

| Approach | Trade-off |
|---|---|
| **User installs QMD globally** (`npm i -g qmd`) | Simplest. User manages Node.js. Jarvis detects and uses it. |
| **Jarvis bundles QMD** (vendor in node_modules) | Better UX but bloats app size. Need to ship Node.js or require it. |

**Recommendation:** Start with "user installs." Add a one-time setup prompt in Settings: "Install QMD for semantic search?" with a button that runs the install command. Similar to how MLX venv setup works today.

**Settings integration:**

```json
{
  "search_provider": "qmd",           // "qmd" | "native" | "none"
  "qmd_path": "/usr/local/bin/qmd",   // auto-detected or user-configured
  "qmd_mode": "cli",                  // "cli" | "mcp-http"
  "qmd_mcp_port": 8377,
  "search_mode": "hybrid"             // default search mode
}
```

### Ideas Worth Adopting from QMD

Even with QMD as a black box behind the trait, these patterns are worth understanding for when we eventually build native:

1. **Query expansion** — QMD generates search variations before retrieval. Huge recall boost. Our native version can do this via MLX with a simple prompt.
2. **Markdown-aware chunking** — splits at heading boundaries, keeps code blocks intact. When we generate gem.md with consistent heading structure, QMD chunks them well automatically.
3. **Position-aware RRF blending** — top-3 retrieval results weighted at 75%, declining to 40% by rank 11+. Prevents reranker from overriding strong retrieval signals.
4. **Collection contexts** — project descriptions (objective, topics) map naturally to QMD's context system. Feed them as collection metadata to improve search quality.

---

## Decided

1. **Integration approach** — QMD as sidecar behind a `SearchProvider` trait. Loose coupling. Swappable.
2. **Interaction mode** — Start with CLI (`qmd query --json`), upgrade to MCP HTTP when latency matters.
3. **Fallback** — `NoOpSearchProvider` degrades to FTS5-only if QMD isn't installed. App still works.
4. **Embeddings, chunking, reranking** — delegated to QMD. It handles its own models (~2GB). We don't duplicate this in MLX.
5. **Knowledge files** — gem.md generated by Jarvis, indexed by QMD. Jarvis owns the files, QMD owns the index.

## Open Questions

1. **Knowledge file format** — strict template (easier for QMD to chunk well) vs freeform markdown? Template recommended.
2. **QMD installation UX** — require user to install Node.js + QMD, or bundle it? Start with user-installs + setup prompt in Settings.
3. **CLI vs MCP timing** — when does CLI latency become a problem? Profile with ~100 gems to decide.
4. **Collection contexts** — should Jarvis push project descriptions into QMD as collection contexts? Would improve search quality.
5. **Index update frequency** — `qmd update && qmd embed` on every gem save, or batch periodically?
6. **Existing gems migration** — on first QMD setup, generate all gem.md files and run `qmd embed`. Could take minutes for large libraries.
7. **When to go native** — what triggers the move from QmdSearchProvider to NativeSearchProvider? Metrics to watch: query latency, install friction complaints, model download size concerns.
