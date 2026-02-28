# Searchable Gems — SearchResultProvider Trait + QMD Semantic Search

## The Idea

Add a **`SearchResultProvider` trait** to Jarvis — a backend-agnostic interface that any search provider must implement. Tauri commands ask for results in a standard format (`Vec<SearchResult>`), and the provider fulfills that contract. The commands never know which backend is active.

Ship two implementations:

1. **`FtsResultProvider`** (default) — wraps existing SQLite FTS5 keyword search. Always available, zero setup.
2. **`QmdResultProvider`** (opt-in) — wraps [QMD](https://github.com/tobi/qmd) CLI for semantic search over `gem.md` knowledge files. User enables it in Settings, Jarvis installs everything automatically.

---

## Why a Trait

- **QMD today, something else tomorrow.** Swap to Qdrant, Ollama embeddings, or a native Rust implementation — just add a new struct that implements `SearchResultProvider`. Zero changes to commands or frontend.
- **Same pattern as everything else.** `IntelProvider`, `KnowledgeStore`, `Chatable` — Jarvis's architecture is trait-based. Search should follow.
- **Clean fallback.** FTS5 is always there. If QMD breaks or user disables it, search keeps working.
- **Contract-driven.** The trait defines _what_ results look like. Each provider decides _how_ to get them.

---

## Architecture

```
GemsPanel.tsx
  │
  │ Search bar input (same UI regardless of provider)
  │
  ▼
invoke('search_gems', { query, limit })
  │
  ▼
Tauri command: search_gems
  │
  ▼
SearchResultProvider trait (Arc<dyn SearchResultProvider>)
  │  → Asks: "give me Vec<SearchResult> for this query"
  │  → Provider fulfills the contract
  │
  ├── FtsResultProvider (default)
  │   └── SQLite FTS5 MATCH query → Vec<SearchResult>
  │
  ├── QmdResultProvider (opt-in, after setup)
  │   └── QMD CLI `qmd query --json` → parse → Vec<SearchResult>
  │
  └── FutureProvider (Qdrant, Ollama, etc.)
      └── Implement trait → Vec<SearchResult>
```

---

## The Trait

```rust
/// A single search result — the standard format every provider must return.
///
/// The trait consumer (Tauri commands) only sees this shape.
/// How it gets populated is the provider's business.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub gem_id: String,
    pub score: f64,              // 0.0–1.0 normalized
    pub matched_chunk: String,   // snippet that matched (empty if provider doesn't support)
    pub match_type: MatchType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchType {
    Keyword,   // FTS5/BM25
    Semantic,  // vector similarity
    Hybrid,    // both (e.g., QMD combines BM25 + vectors + reranker)
}

/// Backend-agnostic search result provider.
///
/// Tauri commands call this trait, never a concrete implementation.
/// Each provider fulfills the contract — returns results in the standard format.
///
/// Adding a new search backend = implement this trait + register in lib.rs.
#[async_trait]
pub trait SearchResultProvider: Send + Sync {
    /// Check if the provider is available and ready to serve results
    async fn check_availability(&self) -> AvailabilityResult;

    /// Search gems by query string, return results in standard format
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, String>;

    /// Index a gem's content (called after gem save/enrich)
    async fn index_gem(&self, gem_id: &str) -> Result<(), String>;

    /// Remove a gem from the index (called after gem delete)
    async fn remove_gem(&self, gem_id: &str) -> Result<(), String>;

    /// Rebuild the entire index from scratch
    async fn reindex_all(&self) -> Result<usize, String>;
}
```

---

## Two Providers

### 1. FtsResultProvider (Default — Always Available)

Wraps the existing `SqliteGemStore::search()` method. No new dependencies. Translates FTS5 results into the standard `SearchResult` format.

```rust
pub struct FtsResultProvider {
    gem_store: Arc<dyn GemStore>,
}

#[async_trait]
impl SearchResultProvider for FtsResultProvider {
    async fn check_availability(&self) -> AvailabilityResult {
        AvailabilityResult { available: true, reason: None }
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
        let gems = self.gem_store.search(query, limit as i64).await?;
        Ok(gems.into_iter().enumerate().map(|(i, gem)| SearchResult {
            gem_id: gem.id,
            score: 1.0 - (i as f64 * 0.05), // approximate from FTS rank order
            matched_chunk: String::new(),     // FTS doesn't provide snippets
            match_type: MatchType::Keyword,
        }).collect())
    }

    async fn index_gem(&self, _gem_id: &str) -> Result<(), String> {
        Ok(()) // FTS5 triggers handle indexing automatically
    }

    async fn remove_gem(&self, _gem_id: &str) -> Result<(), String> {
        Ok(()) // FTS5 triggers handle deletion automatically
    }

    async fn reindex_all(&self) -> Result<usize, String> {
        // Run FTS5 rebuild
        Ok(0)
    }
}
```

### 2. QmdResultProvider (Opt-In — After Setup)

Wraps QMD CLI. Translates QMD's JSON output into the standard `SearchResult` format. QMD combines BM25 + vector + reranker internally.

```rust
pub struct QmdResultProvider {
    qmd_path: PathBuf,            // path to qmd binary
    knowledge_path: PathBuf,      // ~/Library/.../knowledge/
}

#[async_trait]
impl SearchResultProvider for QmdResultProvider {
    async fn check_availability(&self) -> AvailabilityResult {
        // Run `qmd --version` and `qmd status --json`
        // Check: binary exists, collection exists, index populated
        // Return AvailabilityResult with reason if not ready
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
        // 1. Run `qmd query "{query}" --json -n {limit}`
        // 2. Parse QMD's JSON output (whatever schema QMD returns)
        // 3. Extract gem_id from file paths (knowledge/{gem_id}/gem.md → gem_id)
        // 4. Normalize scores to 0.0–1.0
        // 5. Return Vec<SearchResult> in the standard format
    }

    async fn index_gem(&self, _gem_id: &str) -> Result<(), String> {
        // Run `qmd update && qmd embed` (fire-and-forget)
        // QMD detects changed files and re-indexes only those
    }

    async fn remove_gem(&self, _gem_id: &str) -> Result<(), String> {
        // Run `qmd update` (QMD detects deleted files)
    }

    async fn reindex_all(&self) -> Result<usize, String> {
        // Run `qmd update && qmd embed -f` (force re-embed all)
    }
}
```

### 3. Adding a Future Provider

To add a new search backend (Qdrant, Ollama, etc.):

```rust
pub struct QdrantResultProvider { /* connection details */ }

#[async_trait]
impl SearchResultProvider for QdrantResultProvider {
    // Implement 5 methods, return Vec<SearchResult> in the same format
    // Register in lib.rs → done
}
```

No changes to Tauri commands, no changes to frontend. The trait contract guarantees compatibility.

---

## Provider Registration (lib.rs)

Same pattern as `IntelProvider` and `KnowledgeStore`:

```rust
// In lib.rs setup:

// Default: FTS keyword search (always available)
let search_result_provider: Arc<dyn SearchResultProvider> = Arc::new(
    FtsResultProvider::new(gem_store.clone())
);

// If user has enabled semantic search in settings AND QMD is available:
if settings.semantic_search_enabled {
    if let Ok(qmd) = QmdResultProvider::new(knowledge_path.clone()) {
        search_result_provider = Arc::new(qmd);
    }
}

app.manage(search_result_provider);
```

---

## Tauri Commands

Commands interact with the trait only. They ask for results, they get results. They don't know how.

```rust
#[tauri::command]
pub async fn search_gems(
    query: String,
    limit: Option<usize>,
    provider: State<'_, Arc<dyn SearchResultProvider>>,
    gem_store: State<'_, Arc<dyn GemStore>>,
) -> Result<Vec<GemSearchResult>, String> {
    let results = provider.search(&query, limit.unwrap_or(20)).await?;
    // Join search results with gem metadata from DB (title, source_type, etc.)
    // Return enriched results to frontend
}

#[tauri::command]
pub async fn check_search_availability(
    provider: State<'_, Arc<dyn SearchResultProvider>>,
) -> Result<AvailabilityResult, String> {
    Ok(provider.check_availability().await)
}

#[tauri::command]
pub async fn setup_semantic_search() -> Result<QmdSetupResult, String> {
    // Full automated setup flow (see below)
    // This is the ONLY place that knows about QMD specifically
}

#[tauri::command]
pub async fn rebuild_search_index(
    provider: State<'_, Arc<dyn SearchResultProvider>>,
) -> Result<usize, String> {
    provider.reindex_all().await
}
```

---

## Automated Setup Flow (Settings Page)

When user clicks **"Enable Semantic Search"** in Settings, Jarvis runs the full setup automatically:

```
User clicks "Enable Semantic Search"
  │
  ├── Step 1: Check Node.js >= 22
  │   ├── Run `node --version`
  │   ├── Found v22+? → Continue
  │   └── Missing/old? → Show "Install Node.js 22+" with download link. Stop.
  │
  ├── Step 2: Check/Install Homebrew SQLite
  │   ├── Run `brew list sqlite`
  │   ├── Installed? → Continue
  │   └── Missing? → Run `brew install sqlite`, show progress → Continue
  │
  ├── Step 3: Install QMD
  │   ├── Run `qmd --version`
  │   ├── Installed? → Continue
  │   └── Missing? → Run `npm install -g @tobilu/qmd`, show progress → Continue
  │
  ├── Step 4: Create collection
  │   ├── Run `qmd collection list` — check for "jarvis-gems"
  │   ├── Exists? → Continue
  │   └── Missing? → Run `qmd collection add {knowledge_path} --name jarvis-gems --mask "**/*.md"`
  │
  ├── Step 5: Index & embed
  │   ├── Run `qmd update` (scan files)
  │   ├── Run `qmd embed` (generate vectors — downloads ~1.9GB of models on first run)
  │   ├── Emit progress events to frontend
  │   └── Done
  │
  ├── Step 6: Switch provider
  │   ├── Save `semantic_search_enabled: true` to settings
  │   ├── Replace Arc<dyn SearchResultProvider> with QmdResultProvider
  │   └── Active
  │
  └── Return QmdSetupResult { success: true, node_version, qmd_version, docs_indexed }
```

### QMD Models (Auto-Downloaded)

| Model | Purpose | Size |
|-------|---------|------|
| `embedding-gemma-300M-Q8_0` | Vector embeddings | ~300MB |
| `qwen3-reranker-0.6b-q8_0` | Re-ranking results | ~640MB |
| `qmd-query-expansion-1.7B-q4_k_m` | Query expansion | ~1.1GB |

Downloaded to `~/.cache/qmd/models/` on first `qmd embed`. User doesn't manage these.

---

## Settings UI

Add a **"Semantic Search"** section in Settings (same pattern as MLX Virtual Environment):

```
┌─ Semantic Search ────────────────────────────────────┐
│                                                       │
│  Status: ○ Not configured                            │
│                                                       │
│  Semantic search finds gems by meaning, not just     │
│  keywords. Powered by QMD (local, on-device).        │
│  Requires Node.js 22+ and ~2GB for search models.    │
│                                                       │
│  [Enable Semantic Search]                            │
│                                                       │
│  ─ ─ ─ ─ ─  after setup completes  ─ ─ ─ ─ ─       │
│                                                       │
│  Status: ● Ready (26 gems indexed)                   │
│  Provider: QMD v1.2.3                                │
│  Models: ~/.cache/qmd/models/ (1.9 GB)               │
│                                                       │
│  [Rebuild Index]  [Disable]                          │
│                                                       │
└───────────────────────────────────────────────────────┘
```

**"Enable"** triggers `setup_semantic_search` → step-by-step progress → switches provider.
**"Disable"** switches back to `FtsResultProvider`, saves setting. QMD stays installed (no cleanup).
**"Rebuild Index"** calls `rebuild_search_index` → `provider.reindex_all()`.

---

## Frontend (GemsPanel.tsx)

### What Changes

The search bar stays the same. The only frontend change is **how results are displayed** when semantic search is active:

```
invoke('search_gems', { query, limit })
  → always calls the same command
  → backend routes to active provider (FTS or QMD)
  → results come back with score + match_type
  → if match_type is Semantic/Hybrid: show score badge on gem cards
```

**No search mode toggle needed.** The provider is selected in Settings, not per-search. The search bar just works — if semantic is enabled, searches are semantic. If not, they're keyword.

### Score Badge (Optional Enhancement)

When semantic search is active, show a small relevance percentage on each result card:

```
┌──────────────────────────────────────────┐
│  87%  ECS vs EKS — Container Options     │
│       YouTube · aws.com · Feb 25         │
├──────────────────────────────────────────┤
│  74%  RDS Migration Strategy             │
│       Article · medium.com · Feb 20      │
└──────────────────────────────────────────┘
```

---

## Lifecycle Integration

### When to Update the Index

| Event | Action |
|-------|--------|
| Gem saved | `provider.index_gem(gem_id)` |
| Gem enriched | Same — knowledge files regenerated, triggers re-index |
| Gem deleted | `provider.remove_gem(gem_id)` |
| App launch | If semantic enabled: `provider.reindex_all()` on startup (catch up) |
| Manual | "Rebuild Index" button in Settings |

Each provider decides what these calls mean:
- **FtsResultProvider**: `index_gem` and `remove_gem` are no-ops (FTS5 triggers handle it).
- **QmdResultProvider**: shells out to `qmd update && qmd embed` (async, fire-and-forget).
- **FutureProvider**: whatever that backend needs.

The Tauri commands just call the trait. They don't care about the implementation.

---

## What Changes in Existing Code

### New Files

| File | What |
|------|------|
| `src-tauri/src/search/mod.rs` | Module declaration |
| `src-tauri/src/search/provider.rs` | `SearchResultProvider` trait, `SearchResult`, `MatchType` |
| `src-tauri/src/search/fts_provider.rs` | `FtsResultProvider` — wraps existing FTS5, fulfills trait contract |
| `src-tauri/src/search/qmd_provider.rs` | `QmdResultProvider` — wraps QMD CLI, fulfills trait contract |
| `src-tauri/src/search/commands.rs` | Tauri commands: `search_gems`, `check_search_availability`, `setup_semantic_search`, `rebuild_search_index` |

### Modified Files

| File | Change |
|------|--------|
| `src-tauri/src/lib.rs` | Register search module, manage `Arc<dyn SearchResultProvider>`, register commands |
| `src-tauri/src/commands.rs` | Wire `index_gem` / `remove_gem` calls into save/enrich/delete flows |
| `src/components/GemsPanel.tsx` | Call `search_gems` (replace existing invoke), optional score badge |
| `src/components/SettingsPanel.tsx` | Add "Semantic Search" section |
| `src/App.css` | Styles for settings section + score badge |

### Files NOT Changed

- `sqlite_store.rs` — FTS5 stays as-is (FtsResultProvider wraps it)
- `knowledge/` module — already generates gem.md files
- `GemDetailPanel.tsx`, `RightPanel.tsx` — no changes

---

## What We Skip (For Now)

| Skipped | Why |
|---------|-----|
| MCP HTTP mode | CLI mode is fine. Optimize later if latency matters. |
| `find_similar(gem_id)` | Nice-to-have for "related gems". Not needed for search bar. |
| Search within project scope | Projects feature doesn't exist yet. |
| Hybrid score weighting config | Just use QMD's defaults. |
| Hot-swapping provider at runtime | Requires app restart after enabling/disabling in Settings. Fine for now. |

---

## Effort Estimate

| Phase | Work | Time |
|-------|------|------|
| SearchResultProvider trait + types | `provider.rs` with trait, SearchResult, MatchType | 0.25 day |
| FtsResultProvider | Wrap existing FTS5, return SearchResult format | 0.25 day |
| QmdResultProvider | CLI wrapper, JSON parsing, gem_id extraction | 0.5 day |
| Setup flow | Node.js/SQLite/QMD check + install + collection + index | 0.5 day |
| Tauri commands + lib.rs wiring | search_gems, check_availability, setup, rebuild | 0.5 day |
| Lifecycle hooks | Wire index_gem/remove_gem into save/enrich/delete | 0.25 day |
| Settings UI | Semantic Search section with enable/disable/rebuild | 0.5 day |
| GemsPanel + score badge | Call new search_gems, optional score display | 0.25 day |
| Testing: end-to-end | Install QMD, setup, verify search works | 0.25 day |
| **Total** | | **~3 days** |

---

## Open Questions

1. **QMD JSON output schema** — need to test `qmd query --json` to verify exact field names and score ranges. Assumed format may need adjustment.
2. **Index update latency** — how long does `qmd update && qmd embed` take for ~100 gems? If > 2s, must be truly fire-and-forget (spawn process, don't await).
3. **Path with spaces** — macOS `Application Support` has a space. Need to verify QMD handles quoted paths correctly.
4. **Score normalization** — QMD scores may not be 0.0–1.0. Need to normalize for consistent badge display.
5. **Provider hot-swap** — can we swap the `Arc<dyn SearchResultProvider>` at runtime (after setup completes) without restarting? If not, require restart.
6. **Existing `search_gems` command** — the current one in `commands.rs` calls `gem_store.search()` directly. Replace it with the trait-based version, or add a new command and deprecate the old one?
