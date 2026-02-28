# Searchable Gems — Design Document

## Overview

This design introduces a `SearchResultProvider` trait — a backend-agnostic interface for gem search. Tauri commands call the trait, receive results in a standard format (`Vec<SearchResult>`), and never know which backend is active. Two implementations ship: `FtsResultProvider` (wraps existing FTS5, always available) and `QmdResultProvider` (wraps QMD CLI for semantic search, opt-in via Settings).

The architecture follows the same trait-based pattern used by `IntelProvider` (AI), `KnowledgeStore` (knowledge files), `GemStore` (database), and `Chatable` (chat). Adding a future search backend is one struct implementing five methods.

### Design Goals

1. **Contract-driven**: The trait defines _what_ results look like. Each provider decides _how_ to get them. Commands never see implementation details.
2. **Always works**: FTS5 keyword search is always available, zero setup. Semantic search is opt-in.
3. **Automated setup**: Enabling semantic search is one button click in Settings — Jarvis handles all installation.
4. **Pluggable**: QMD today, Qdrant/Ollama/native-Rust tomorrow. One file changes.
5. **Non-blocking**: Search indexing never blocks gem save/enrich/delete operations.

### Key Design Decisions

- **Single trait `SearchResultProvider`**: No SearchManager, no SearchRouter, no two-trait hierarchy. Commands call the trait directly.
- **Provider selected in Settings, not per-search**: No search mode toggle in the UI. If semantic is enabled, all searches are semantic.
- **FtsResultProvider wraps GemStore::search()**: The existing FTS5 search stays untouched. The provider translates its output to `Vec<SearchResult>`.
- **QmdResultProvider shells out to CLI**: Uses `tokio::process::Command` to call the `qmd` binary. Same pattern as `MlxProvider` calling the Python sidecar.
- **Provider doesn't join metadata**: Returns `SearchResult` (gem_id + score + chunk). The Tauri command joins with `GemStore` for full gem data.
- **Fire-and-forget indexing**: `index_gem()` spawns QMD update without awaiting. Only `reindex_all()` awaits completion.
- **App restart for provider switch**: No hot-swap of `Arc<dyn SearchResultProvider>` at runtime in v1.

### Operational Flow

1. **App starts** → check `settings.search.semantic_search_enabled` → register `FtsResultProvider` or `QmdResultProvider`
2. **User searches** → `invoke('search_gems', { query })` → command calls `provider.search()` → joins with gem metadata → returns to frontend
3. **Gem saved** → `provider.index_gem(gem_id)` → FTS: no-op, QMD: spawn `qmd update && qmd embed`
4. **Gem deleted** → `provider.remove_gem(gem_id)` → FTS: no-op, QMD: spawn `qmd update`
5. **User enables semantic search** → `setup_semantic_search` command → install prerequisites → create collection → index → save setting → restart prompt

---

## Architecture

### Module Hierarchy

```
src/search/
├── mod.rs              — Module root, re-exports public types
├── provider.rs         — SearchResultProvider trait, SearchResult, MatchType, GemSearchResult
├── fts_provider.rs     — FtsResultProvider (wraps GemStore::search())
├── qmd_provider.rs     — QmdResultProvider (wraps QMD CLI)
└── commands.rs         — Tauri commands: search_gems, check_search_availability,
                          setup_semantic_search, rebuild_search_index
```

This follows the existing module pattern:
- `intelligence/` has `provider.rs` (trait) + `mlx_provider.rs` / `intelligencekit_provider.rs` (impls) + `mod.rs`
- `gems/` has `store.rs` (trait) + `sqlite_store.rs` (impl) + `mod.rs`
- `knowledge/` has `store.rs` (trait) + `local_store.rs` (impl) + `mod.rs`

### Dependency Graph

```
                     ┌─────────────┐
                     │   lib.rs    │
                     │   (setup)   │
                     └──────┬──────┘
                            │ reads settings, constructs & registers provider
                            ▼
              ┌───────────────────────────────┐
              │ Arc<dyn SearchResultProvider> │
              │    (Tauri managed state)      │
              └───────────┬───────────────────┘
                          │
          ┌───────────────┼───────────────┐
          ▼               ▼               ▼
  ┌──────────────┐ ┌────────────┐ ┌──────────────┐
  │ search/      │ │ commands.rs│ │ gems/        │
  │ commands.rs  │ │ (save,     │ │ store.rs     │
  │ (search,     │ │  enrich,   │ │ (GemStore    │
  │  setup,      │ │  delete    │ │  trait)      │
  │  rebuild)    │ │  hooks)    │ └──────────────┘
  └──────────────┘ └────────────┘

                ┌─────────────────┐
                │  Two Providers  │
                ├─────────────────┤
                │ FtsResultProvider    │──→ GemStore::search() (Arc<dyn GemStore>)
                │ QmdResultProvider    │──→ `qmd` CLI binary (tokio::process::Command)
                └─────────────────┘
```

### Data Flow — Search

```
GemsPanel.tsx (search bar)
  │
  │ invoke('search_gems', { query: "container orchestration", limit: 20 })
  │
  ▼
search/commands.rs :: search_gems()
  │
  ├── provider.search("container orchestration", 20).await
  │     │
  │     ├── [FtsResultProvider]
  │     │     └── gem_store.search("container orchestration", 20).await
  │     │           → Vec<GemPreview> → map to Vec<SearchResult>
  │     │
  │     └── [QmdResultProvider]
  │           └── Command::new("qmd").args(["query", "container orchestration", "--json", "-n", "20"])
  │                 → parse JSON → extract gem_ids from paths → Vec<SearchResult>
  │
  ├── For each SearchResult.gem_id:
  │     gem_store.get(gem_id) → join metadata
  │
  └── Return Vec<GemSearchResult> to frontend
```

### Data Flow — Lifecycle Integration

```
Gem Lifecycle Event
    │
    ├── save_gem command     ─── success ──→ provider.index_gem(gem_id)   [fire-and-forget]
    ├── enrich_gem command   ─── success ──→ provider.index_gem(gem_id)   [fire-and-forget]
    ├── transcribe_gem cmd   ─── success ──→ provider.index_gem(gem_id)   [fire-and-forget]
    └── delete_gem command   ─── success ──→ provider.remove_gem(gem_id)  [fire-and-forget]
                                                    │
                                                    ▼
                                            SearchResultProvider
                                                    │
                                        ┌───────────┴───────────┐
                                        ▼                       ▼
                                FtsResultProvider         QmdResultProvider
                                    Ok(())                spawn("qmd update
                                    (no-op)                && qmd embed")
```

### Data Flow — Setup

```
SettingsPanel.tsx → "Enable Semantic Search" button
  │
  │ invoke('setup_semantic_search')
  │
  ▼
search/commands.rs :: setup_semantic_search()
  │
  ├── Step 1: node --version → check >= 22
  ├── Step 2: brew list sqlite → install if missing
  ├── Step 3: qmd --version → npm install -g @tobilu/qmd if missing
  ├── Step 4: qmd collection list → create jarvis-gems if missing
  ├── Step 5: qmd update && qmd embed → index all gem.md files
  ├── Step 6: save semantic_search_enabled: true to settings
  │
  ├── Each step emits: emit("semantic-search-setup", { step, total: 6, description, status })
  │
  └── Return QmdSetupResult { success, node_version, qmd_version, docs_indexed }
```

---

## Modules and Interfaces

### `provider.rs` — Trait and Data Types

**File**: `src/search/provider.rs`

**Responsibilities**: Define the `SearchResultProvider` trait and all shared data types.

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::intelligence::AvailabilityResult;

/// How a search result was matched
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchType {
    /// FTS5 / BM25 keyword matching
    Keyword,
    /// Vector similarity (embedding-based)
    Semantic,
    /// Combined keyword + vector + reranking (e.g., QMD)
    Hybrid,
}

/// A single search result — the standard format every provider must return.
///
/// The trait consumer (Tauri commands) only sees this shape.
/// How it gets populated is the provider's business.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The gem UUID that matched
    pub gem_id: String,
    /// Relevance score, normalized to 0.0–1.0 (1.0 = best match)
    pub score: f64,
    /// Snippet of text that matched (empty if provider doesn't support snippets)
    pub matched_chunk: String,
    /// How this result was matched
    pub match_type: MatchType,
}

/// Enriched search result returned to the frontend.
///
/// Combines SearchResult metadata (score, chunk, match_type) with
/// gem metadata (title, source_type, etc.) from the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GemSearchResult {
    // From SearchResult
    pub score: f64,
    pub matched_chunk: String,
    pub match_type: MatchType,

    // From GemPreview (joined by gem_id)
    pub id: String,
    pub source_type: String,
    pub source_url: String,
    pub domain: String,
    pub title: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub captured_at: String,
    pub tags: Option<Vec<String>>,
    pub summary: Option<String>,
}

/// Result of the semantic search setup flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QmdSetupResult {
    pub success: bool,
    pub node_version: Option<String>,
    pub qmd_version: Option<String>,
    pub docs_indexed: Option<usize>,
    pub error: Option<String>,
}

/// Progress event emitted during setup
#[derive(Debug, Clone, Serialize)]
pub struct SetupProgressEvent {
    pub step: usize,
    pub total: usize,
    pub description: String,
    pub status: String, // "running", "done", "failed"
}

/// Backend-agnostic search result provider.
///
/// Tauri commands call this trait, never a concrete implementation.
/// Each provider fulfills the contract — returns results in the standard format.
///
/// Adding a new search backend = implement this trait + register in lib.rs.
///
/// Follows the same pattern as IntelProvider (AI), KnowledgeStore (knowledge files),
/// GemStore (database), Chatable (chat).
#[async_trait]
pub trait SearchResultProvider: Send + Sync {
    /// Check if the provider is available and ready to serve results
    async fn check_availability(&self) -> AvailabilityResult;

    /// Search gems by query string, return results in standard format
    ///
    /// Providers MUST return scores normalized to 0.0–1.0.
    /// Providers MUST return at most `limit` results.
    /// Providers SHOULD return results sorted by score descending.
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, String>;

    /// Notify the provider that a gem was created or updated
    ///
    /// FTS: no-op (triggers handle it). QMD: spawn `qmd update && qmd embed`.
    /// Implementations SHOULD be fire-and-forget (don't block on indexing).
    async fn index_gem(&self, gem_id: &str) -> Result<(), String>;

    /// Notify the provider that a gem was deleted
    ///
    /// FTS: no-op (triggers handle it). QMD: spawn `qmd update`.
    async fn remove_gem(&self, gem_id: &str) -> Result<(), String>;

    /// Rebuild the entire search index from scratch
    ///
    /// Unlike index_gem/remove_gem, this SHOULD await completion.
    /// Returns the number of documents indexed.
    async fn reindex_all(&self) -> Result<usize, String>;
}
```

### `fts_provider.rs` — FTS5 Keyword Search

**File**: `src/search/fts_provider.rs`

**Responsibilities**: Wrap the existing `GemStore::search()` FTS5 method. Translate `Vec<GemPreview>` to `Vec<SearchResult>`.

```rust
use std::sync::Arc;
use crate::gems::GemStore;
use crate::intelligence::AvailabilityResult;
use super::provider::*;

/// Default search provider — wraps existing SQLite FTS5 keyword search.
///
/// Always available, zero setup. Returns MatchType::Keyword.
/// FTS5 indexing is handled by SQLite triggers, so index_gem/remove_gem are no-ops.
pub struct FtsResultProvider {
    gem_store: Arc<dyn GemStore>,
}

impl FtsResultProvider {
    pub fn new(gem_store: Arc<dyn GemStore>) -> Self {
        Self { gem_store }
    }
}

#[async_trait::async_trait]
impl SearchResultProvider for FtsResultProvider {
    async fn check_availability(&self) -> AvailabilityResult {
        // FTS5 is always available — it's built into SQLite
        AvailabilityResult {
            available: true,
            reason: None,
        }
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
        let gems = self.gem_store.search(query, limit).await?;

        Ok(gems
            .into_iter()
            .enumerate()
            .map(|(i, gem)| SearchResult {
                gem_id: gem.id,
                score: (1.0 - (i as f64 * 0.05)).max(0.0),
                matched_chunk: String::new(), // FTS5 doesn't provide snippets
                match_type: MatchType::Keyword,
            })
            .collect())
    }

    async fn index_gem(&self, _gem_id: &str) -> Result<(), String> {
        // No-op: FTS5 triggers (gems_ai, gems_ad, gems_au) handle indexing
        Ok(())
    }

    async fn remove_gem(&self, _gem_id: &str) -> Result<(), String> {
        // No-op: FTS5 triggers handle deletion
        Ok(())
    }

    async fn reindex_all(&self) -> Result<usize, String> {
        // FTS5 index is maintained by SQLite triggers. Nothing to rebuild.
        // Could run `INSERT INTO gems_fts(gems_fts) VALUES('rebuild')` here
        // for a full FTS5 rebuild, but unnecessary for v1.
        Ok(0)
    }
}
```

### `qmd_provider.rs` — QMD Semantic Search

**File**: `src/search/qmd_provider.rs`

**Responsibilities**: Wrap the QMD CLI binary. Translate QMD's JSON output to `Vec<SearchResult>`.

```rust
use std::path::PathBuf;
use tokio::process::Command;
use crate::intelligence::AvailabilityResult;
use super::provider::*;

/// Opt-in semantic search provider — wraps QMD CLI.
///
/// QMD combines BM25 keyword matching, vector embeddings (Gemma 300M),
/// and LLM-based reranking (Qwen3 0.6B) for hybrid search.
///
/// Requires: Node.js 22+, Homebrew SQLite, @tobilu/qmd npm package,
/// ~1.9GB of search models in ~/.cache/qmd/models/.
///
/// Knowledge files at `knowledge/{gem_id}/gem.md` are the search corpus.
pub struct QmdResultProvider {
    /// Path to the qmd binary (e.g., /opt/homebrew/bin/qmd)
    qmd_path: PathBuf,
    /// Root knowledge directory (e.g., ~/Library/.../knowledge/)
    knowledge_path: PathBuf,
}

impl QmdResultProvider {
    /// Create a new QmdResultProvider.
    ///
    /// Does NOT verify that qmd is installed or available.
    /// Call check_availability() to verify.
    pub fn new(qmd_path: PathBuf, knowledge_path: PathBuf) -> Self {
        Self {
            qmd_path,
            knowledge_path,
        }
    }

    /// Try to find the qmd binary in common locations.
    ///
    /// Checks: provided path → /opt/homebrew/bin/qmd → `which qmd`
    pub async fn find_qmd_binary() -> Option<PathBuf> {
        // Check common Homebrew location
        let homebrew_path = PathBuf::from("/opt/homebrew/bin/qmd");
        if homebrew_path.exists() {
            return Some(homebrew_path);
        }

        // Check /usr/local/bin (Intel Mac)
        let usr_local_path = PathBuf::from("/usr/local/bin/qmd");
        if usr_local_path.exists() {
            return Some(usr_local_path);
        }

        // Try `which qmd`
        if let Ok(output) = Command::new("which")
            .arg("qmd")
            .output()
            .await
        {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
        }

        None
    }
}

#[async_trait::async_trait]
impl SearchResultProvider for QmdResultProvider {
    async fn check_availability(&self) -> AvailabilityResult {
        // 1. Check binary exists
        if !self.qmd_path.exists() {
            return AvailabilityResult {
                available: false,
                reason: Some(format!(
                    "QMD binary not found at {}",
                    self.qmd_path.display()
                )),
            };
        }

        // 2. Check qmd --version succeeds
        let version_result = Command::new(&self.qmd_path)
            .arg("--version")
            .output()
            .await;

        match version_result {
            Err(e) => {
                return AvailabilityResult {
                    available: false,
                    reason: Some(format!("Failed to run qmd --version: {}", e)),
                };
            }
            Ok(output) if !output.status.success() => {
                return AvailabilityResult {
                    available: false,
                    reason: Some("qmd --version returned non-zero exit code".to_string()),
                };
            }
            _ => {}
        }

        // 3. Check collection exists via `qmd status --json`
        let status_result = Command::new(&self.qmd_path)
            .args(["status", "--json"])
            .output()
            .await;

        match status_result {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Check that the output mentions the jarvis-gems collection
                // (exact JSON parsing depends on QMD's actual schema — TBD)
                if !stdout.contains("jarvis-gems") {
                    return AvailabilityResult {
                        available: false,
                        reason: Some(
                            "QMD jarvis-gems collection not found. Run setup first.".to_string()
                        ),
                    };
                }
            }
            _ => {
                return AvailabilityResult {
                    available: false,
                    reason: Some("Failed to check QMD status".to_string()),
                };
            }
        }

        AvailabilityResult {
            available: true,
            reason: None,
        }
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
        // Run: qmd query "{query}" --json -n {limit}
        let output = Command::new(&self.qmd_path)
            .args([
                "query",
                query,
                "--json",
                "-n",
                &limit.to_string(),
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to run qmd query: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("qmd query failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse QMD JSON output
        // Expected shape (TBD — needs verification against actual QMD output):
        // [{ "file": "path/to/gem.md", "score": 0.87, "chunk": "matched text..." }, ...]
        let qmd_results: Vec<serde_json::Value> = serde_json::from_str(&stdout)
            .map_err(|e| format!("Failed to parse QMD JSON output: {}", e))?;

        let mut results = Vec::new();

        for item in qmd_results {
            // Extract file path and derive gem_id
            let file_path = item
                .get("file")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            // Extract gem_id from path: knowledge/{gem_id}/gem.md → gem_id
            let gem_id = extract_gem_id_from_path(file_path, &self.knowledge_path);

            if let Some(gem_id) = gem_id {
                let raw_score = item
                    .get("score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);

                let matched_chunk = item
                    .get("chunk")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                results.push(SearchResult {
                    gem_id,
                    score: normalize_qmd_score(raw_score),
                    matched_chunk,
                    match_type: MatchType::Hybrid,
                });
            }
        }

        Ok(results)
    }

    async fn index_gem(&self, _gem_id: &str) -> Result<(), String> {
        // Fire-and-forget: spawn qmd update && qmd embed
        // QMD detects changed files and re-indexes only those
        let qmd_path = self.qmd_path.clone();
        tokio::spawn(async move {
            let update = Command::new(&qmd_path)
                .arg("update")
                .output()
                .await;

            if let Ok(output) = update {
                if output.status.success() {
                    let _ = Command::new(&qmd_path)
                        .arg("embed")
                        .output()
                        .await;
                }
            }
        });

        Ok(())
    }

    async fn remove_gem(&self, _gem_id: &str) -> Result<(), String> {
        // Fire-and-forget: spawn qmd update
        // QMD detects deleted files
        let qmd_path = self.qmd_path.clone();
        tokio::spawn(async move {
            let _ = Command::new(&qmd_path)
                .arg("update")
                .output()
                .await;
        });

        Ok(())
    }

    async fn reindex_all(&self) -> Result<usize, String> {
        // Await: qmd update && qmd embed -f (force re-embed all)
        let update = Command::new(&self.qmd_path)
            .arg("update")
            .output()
            .await
            .map_err(|e| format!("qmd update failed: {}", e))?;

        if !update.status.success() {
            let stderr = String::from_utf8_lossy(&update.stderr);
            return Err(format!("qmd update failed: {}", stderr));
        }

        let embed = Command::new(&self.qmd_path)
            .args(["embed", "-f"])
            .output()
            .await
            .map_err(|e| format!("qmd embed failed: {}", e))?;

        if !embed.status.success() {
            let stderr = String::from_utf8_lossy(&embed.stderr);
            return Err(format!("qmd embed failed: {}", stderr));
        }

        // Parse output for document count (TBD — depends on QMD output)
        // For now, return 0 and refine after testing QMD CLI output
        Ok(0)
    }
}

/// Extract gem_id from a QMD result file path.
///
/// QMD returns paths relative to the collection root (knowledge/).
/// Path format: `{gem_id}/gem.md` or absolute `{knowledge_path}/{gem_id}/gem.md`
fn extract_gem_id_from_path(file_path: &str, knowledge_path: &PathBuf) -> Option<String> {
    let path = std::path::Path::new(file_path);

    // Try stripping the knowledge_path prefix (absolute path)
    if let Ok(relative) = path.strip_prefix(knowledge_path) {
        return relative
            .components()
            .next()
            .and_then(|c| c.as_os_str().to_str())
            .map(|s| s.to_string());
    }

    // Try treating as relative path: {gem_id}/gem.md
    path.components()
        .next()
        .and_then(|c| c.as_os_str().to_str())
        .map(|s| s.to_string())
}

/// Normalize QMD score to 0.0–1.0 range.
///
/// QMD's actual score range is TBD. This function will be refined
/// after testing `qmd query --json` output.
fn normalize_qmd_score(raw_score: f64) -> f64 {
    // If QMD already returns 0-1, just clamp
    raw_score.clamp(0.0, 1.0)
}
```

### `commands.rs` — Tauri Command Handlers

**File**: `src/search/commands.rs`

**Responsibilities**: Expose search operations as Tauri commands. Join search results with gem metadata.

```rust
use std::sync::Arc;
use tauri::State;

use crate::gems::GemStore;
use crate::intelligence::AvailabilityResult;
use crate::settings::SettingsManager;
use super::provider::*;

/// Search gems via the active search result provider.
///
/// Delegates to whichever provider is registered (FTS5 or QMD).
/// Joins search results with gem metadata from the database.
#[tauri::command]
pub async fn search_gems(
    query: String,
    limit: Option<usize>,
    provider: State<'_, Arc<dyn SearchResultProvider>>,
    gem_store: State<'_, Arc<dyn GemStore>>,
) -> Result<Vec<GemSearchResult>, String> {
    let limit = limit.unwrap_or(20);

    // Handle empty query — delegate to gem_store.list() for consistency
    if query.trim().is_empty() {
        let gems = gem_store.list(limit, 0).await?;
        return Ok(gems
            .into_iter()
            .map(|gem| GemSearchResult {
                score: 1.0,
                matched_chunk: String::new(),
                match_type: MatchType::Keyword,
                id: gem.id,
                source_type: gem.source_type,
                source_url: gem.source_url,
                domain: gem.domain,
                title: gem.title,
                author: gem.author,
                description: gem.description,
                captured_at: gem.captured_at,
                tags: gem.tags,
                summary: gem.summary,
            })
            .collect());
    }

    // Search via provider
    let search_results = provider.search(&query, limit).await?;

    // Join each SearchResult with gem metadata from DB
    let mut enriched = Vec::new();
    for result in search_results {
        if let Ok(Some(gem)) = gem_store.get(&result.gem_id).await {
            // Extract tags and summary from ai_enrichment JSON
            let tags = gem.ai_enrichment
                .as_ref()
                .and_then(|e| e.get("tags"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                });

            let summary = gem.ai_enrichment
                .as_ref()
                .and_then(|e| e.get("summary"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            enriched.push(GemSearchResult {
                score: result.score,
                matched_chunk: result.matched_chunk,
                match_type: result.match_type,
                id: gem.id,
                source_type: gem.source_type,
                source_url: gem.source_url,
                domain: gem.domain,
                title: gem.title,
                author: gem.author,
                description: gem.description,
                captured_at: gem.captured_at,
                tags,
                summary,
            });
        }
        // Skip results where gem not found in DB (orphaned index entry)
    }

    Ok(enriched)
}

/// Check if the active search provider is available.
#[tauri::command]
pub async fn check_search_availability(
    provider: State<'_, Arc<dyn SearchResultProvider>>,
) -> Result<AvailabilityResult, String> {
    Ok(provider.check_availability().await)
}

/// Run the automated QMD semantic search setup flow.
///
/// Steps: check Node.js → install SQLite → install QMD → create collection → index → save setting
/// Emits progress events on "semantic-search-setup" channel.
#[tauri::command]
pub async fn setup_semantic_search(
    app_handle: tauri::AppHandle,
    settings_manager: State<'_, Arc<SettingsManager>>,
) -> Result<QmdSetupResult, String> {
    let total_steps = 6;

    // Helper to emit progress
    let emit_progress = |step: usize, description: &str, status: &str| {
        let _ = app_handle.emit("semantic-search-setup", SetupProgressEvent {
            step,
            total: total_steps,
            description: description.to_string(),
            status: status.to_string(),
        });
    };

    // Step 1: Check Node.js >= 22
    emit_progress(1, "Checking Node.js", "running");
    let node_version = check_node_version().await?;
    emit_progress(1, "Checking Node.js", "done");

    // Step 2: Check/Install SQLite
    emit_progress(2, "Checking SQLite", "running");
    check_or_install_sqlite().await?;
    emit_progress(2, "Checking SQLite", "done");

    // Step 3: Install QMD
    emit_progress(3, "Installing QMD", "running");
    let qmd_version = check_or_install_qmd().await?;
    emit_progress(3, "Installing QMD", "done");

    // Step 4: Create collection
    emit_progress(4, "Creating search collection", "running");
    let knowledge_path = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("knowledge");
    create_qmd_collection(&knowledge_path).await?;
    emit_progress(4, "Creating search collection", "done");

    // Step 5: Index & embed
    emit_progress(5, "Indexing gems (downloading models on first run, ~1.9GB)", "running");
    let docs_indexed = run_qmd_index().await?;
    emit_progress(5, "Indexing gems", "done");

    // Step 6: Save setting
    emit_progress(6, "Saving settings", "running");
    // Update settings to enable semantic search
    // settings_manager.update_search_settings(true)?;
    emit_progress(6, "Saving settings", "done");

    Ok(QmdSetupResult {
        success: true,
        node_version: Some(node_version),
        qmd_version: Some(qmd_version),
        docs_indexed: Some(docs_indexed),
        error: None,
    })
}

/// Rebuild the search index from scratch.
#[tauri::command]
pub async fn rebuild_search_index(
    provider: State<'_, Arc<dyn SearchResultProvider>>,
) -> Result<usize, String> {
    provider.reindex_all().await
}

// ── Setup helper functions ──────────────────────────────

async fn check_node_version() -> Result<String, String> {
    let output = tokio::process::Command::new("node")
        .arg("--version")
        .output()
        .await
        .map_err(|_| "Node.js not found. Install Node.js 22+ from https://nodejs.org/".to_string())?;

    if !output.status.success() {
        return Err("Node.js not found. Install Node.js 22+ from https://nodejs.org/".to_string());
    }

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Parse major version: "v24.1.0" → 24
    let major: u32 = version
        .strip_prefix('v')
        .and_then(|v| v.split('.').next())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    if major < 22 {
        return Err(format!(
            "Node.js {} is too old. Version 22+ required. Download from https://nodejs.org/",
            version
        ));
    }

    Ok(version)
}

async fn check_or_install_sqlite() -> Result<(), String> {
    let check = tokio::process::Command::new("brew")
        .args(["list", "sqlite"])
        .output()
        .await;

    match check {
        Ok(output) if output.status.success() => Ok(()),
        _ => {
            // Install sqlite via brew
            let install = tokio::process::Command::new("brew")
                .args(["install", "sqlite"])
                .output()
                .await
                .map_err(|e| format!("Failed to install sqlite via brew: {}", e))?;

            if install.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&install.stderr);
                Err(format!("brew install sqlite failed: {}", stderr))
            }
        }
    }
}

async fn check_or_install_qmd() -> Result<String, String> {
    // Check if already installed
    let check = tokio::process::Command::new("qmd")
        .arg("--version")
        .output()
        .await;

    if let Ok(output) = check {
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
    }

    // Install via npm
    let install = tokio::process::Command::new("npm")
        .args(["install", "-g", "@tobilu/qmd"])
        .output()
        .await
        .map_err(|e| format!("Failed to install QMD: {}", e))?;

    if !install.status.success() {
        let stderr = String::from_utf8_lossy(&install.stderr);
        return Err(format!("npm install -g @tobilu/qmd failed: {}", stderr));
    }

    // Verify installation
    let verify = tokio::process::Command::new("qmd")
        .arg("--version")
        .output()
        .await
        .map_err(|e| format!("QMD installed but not accessible: {}", e))?;

    Ok(String::from_utf8_lossy(&verify.stdout).trim().to_string())
}

async fn create_qmd_collection(knowledge_path: &std::path::Path) -> Result<(), String> {
    // Check if collection exists
    let list = tokio::process::Command::new("qmd")
        .args(["collection", "list"])
        .output()
        .await
        .map_err(|e| format!("Failed to list QMD collections: {}", e))?;

    let stdout = String::from_utf8_lossy(&list.stdout);
    if stdout.contains("jarvis-gems") {
        return Ok(()); // Already exists
    }

    // Create collection
    let path_str = knowledge_path.to_str()
        .ok_or("Knowledge path contains invalid UTF-8")?;

    let create = tokio::process::Command::new("qmd")
        .args([
            "collection",
            "add",
            path_str,
            "--name",
            "jarvis-gems",
            "--mask",
            "**/*.md",
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to create QMD collection: {}", e))?;

    if create.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&create.stderr);
        Err(format!("qmd collection add failed: {}", stderr))
    }
}

async fn run_qmd_index() -> Result<usize, String> {
    // qmd update
    let update = tokio::process::Command::new("qmd")
        .arg("update")
        .output()
        .await
        .map_err(|e| format!("qmd update failed: {}", e))?;

    if !update.status.success() {
        let stderr = String::from_utf8_lossy(&update.stderr);
        return Err(format!("qmd update failed: {}", stderr));
    }

    // qmd embed (downloads ~1.9GB of models on first run)
    let embed = tokio::process::Command::new("qmd")
        .arg("embed")
        .output()
        .await
        .map_err(|e| format!("qmd embed failed: {}", e))?;

    if !embed.status.success() {
        let stderr = String::from_utf8_lossy(&embed.stderr);
        return Err(format!("qmd embed failed: {}", stderr));
    }

    // TODO: Parse output for indexed document count
    Ok(0)
}
```

### `mod.rs` — Module Root

**File**: `src/search/mod.rs`

```rust
pub mod provider;
pub mod fts_provider;
pub mod qmd_provider;
pub mod commands;

pub use provider::{
    SearchResultProvider,
    SearchResult,
    MatchType,
    GemSearchResult,
    QmdSetupResult,
    SetupProgressEvent,
};
pub use fts_provider::FtsResultProvider;
pub use qmd_provider::QmdResultProvider;
```

---

## Settings Extension

### New `SearchSettings` Struct

**File**: `src/settings/manager.rs` — add to existing Settings struct:

```rust
pub struct Settings {
    pub transcription: TranscriptionSettings,
    #[serde(default)]
    pub browser: BrowserSettings,
    #[serde(default)]
    pub intelligence: IntelligenceSettings,
    #[serde(default)]
    pub copilot: CoPilotSettings,
    #[serde(default)]
    pub search: SearchSettings,         // ← NEW
}

/// Search-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSettings {
    /// Whether semantic search is enabled (QMD provider active)
    #[serde(default)]
    pub semantic_search_enabled: bool,
}

impl Default for SearchSettings {
    fn default() -> Self {
        Self {
            semantic_search_enabled: false,
        }
    }
}
```

Using `#[serde(default)]` ensures backward compatibility — existing settings files without a `search` field will get `SearchSettings::default()` (semantic disabled).

---

## Provider Registration

### In `lib.rs` — App Setup

```rust
// ── Search Provider Registration ──
// Following the same pattern as IntelProvider registration

let gem_store_for_search = gem_store.clone();
let settings = settings_manager.get_settings();

let search_provider: Arc<dyn search::SearchResultProvider> = if settings.search.semantic_search_enabled {
    // Try to create QMD provider
    match search::QmdResultProvider::find_qmd_binary().await {
        Some(qmd_path) => {
            let knowledge_path = app_data_dir.join("knowledge");
            let qmd = search::QmdResultProvider::new(qmd_path, knowledge_path);

            // Verify it's actually available
            let availability = qmd.check_availability().await;
            if availability.available {
                eprintln!("Search: Using QMD semantic search provider");
                Arc::new(qmd)
            } else {
                eprintln!(
                    "Search: QMD not available ({}), falling back to FTS5",
                    availability.reason.unwrap_or_default()
                );
                Arc::new(search::FtsResultProvider::new(gem_store_for_search))
            }
        }
        None => {
            eprintln!("Search: QMD binary not found, falling back to FTS5");
            Arc::new(search::FtsResultProvider::new(gem_store_for_search))
        }
    }
} else {
    eprintln!("Search: Using FTS5 keyword search (default)");
    Arc::new(search::FtsResultProvider::new(gem_store_for_search))
};

app.manage(search_provider);

// Register search commands in invoke_handler:
// search::commands::search_gems,
// search::commands::check_search_availability,
// search::commands::setup_semantic_search,
// search::commands::rebuild_search_index,
```

### Removing Old `search_gems` Command

The existing `search_gems` in `src/commands.rs` (line 461) is removed. The new `search/commands.rs::search_gems` replaces it. The frontend call stays the same: `invoke('search_gems', { query, limit })`.

### Lifecycle Hooks in `commands.rs`

Wire `provider.index_gem()` / `provider.remove_gem()` into existing gem commands:

```rust
// In save_gem command, after successful DB save + knowledge file creation:
if let Some(search_provider) = app_handle.try_state::<Arc<dyn SearchResultProvider>>() {
    if let Err(e) = search_provider.index_gem(&saved_gem.id).await {
        eprintln!("Search: Failed to index gem {}: {}", saved_gem.id, e);
    }
}

// In enrich_gem command, after enrichment + knowledge update:
if let Some(search_provider) = app_handle.try_state::<Arc<dyn SearchResultProvider>>() {
    if let Err(e) = search_provider.index_gem(&gem_id).await {
        eprintln!("Search: Failed to re-index gem {}: {}", gem_id, e);
    }
}

// In delete_gem command, after DB delete + knowledge delete:
if let Some(search_provider) = app_handle.try_state::<Arc<dyn SearchResultProvider>>() {
    if let Err(e) = search_provider.remove_gem(&gem_id).await {
        eprintln!("Search: Failed to remove gem {} from index: {}", gem_id, e);
    }
}
```

The pattern `try_state::<Arc<dyn SearchResultProvider>>()` ensures commands work even if the search provider isn't registered.

---

## Frontend Changes

### TypeScript Types

Add to `src/state/types.ts` (or alongside existing gem types):

```typescript
interface SearchResult {
  gem_id: string;
  score: number;        // 0.0–1.0
  matched_chunk: string;
  match_type: 'Keyword' | 'Semantic' | 'Hybrid';
}

interface GemSearchResult {
  // Search metadata
  score: number;
  matched_chunk: string;
  match_type: 'Keyword' | 'Semantic' | 'Hybrid';

  // Gem metadata
  id: string;
  source_type: string;
  source_url: string;
  domain: string;
  title: string;
  author: string | null;
  description: string | null;
  captured_at: string;
  tags: string[] | null;
  summary: string | null;
}

interface QmdSetupResult {
  success: boolean;
  node_version: string | null;
  qmd_version: string | null;
  docs_indexed: number | null;
  error: string | null;
}

interface SetupProgressEvent {
  step: number;
  total: number;
  description: string;
  status: 'running' | 'done' | 'failed';
}
```

### GemsPanel.tsx Changes

The search bar and debounce stay the same. The main change is the return type and optional score badge:

```typescript
// Before: invoke<GemPreview[]>('search_gems', { query })
// After:  invoke<GemSearchResult[]>('search_gems', { query, limit: 50 })

const fetchGems = useCallback(async (query: string, tag: string | null) => {
  try {
    let results: GemSearchResult[];
    if (tag) {
      // filter_by_tag returns GemPreview — wrap as GemSearchResult
      const gems = await invoke<GemPreview[]>('filter_gems_by_tag', { tag, limit: 50, offset: 0 });
      results = gems.map(g => ({ ...g, score: 1.0, matched_chunk: '', match_type: 'Keyword' as const }));
    } else {
      // search_gems returns GemSearchResult (with scores)
      results = await invoke<GemSearchResult[]>('search_gems', { query, limit: 50 });
    }
    setGems(results);
  } catch (e) {
    setError(String(e));
  }
}, []);
```

### Score Badge (Optional Enhancement)

When `match_type` is `Semantic` or `Hybrid`, show a small relevance badge on gem cards:

```tsx
{gem.match_type !== 'Keyword' && (
  <span className="relevance-badge">
    {Math.round(gem.score * 100)}%
  </span>
)}
```

CSS for the badge:

```css
.relevance-badge {
  display: inline-block;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 600;
  background: rgba(59, 130, 246, 0.15);
  color: #60a5fa;
  margin-right: 6px;
}
```

### SettingsPanel.tsx Changes

Add a "Semantic Search" section after the existing MLX/AI sections:

```tsx
{/* Semantic Search Section */}
<div className="settings-section">
  <h3>Semantic Search</h3>

  {!semanticSearchEnabled ? (
    <>
      <div className="settings-status">
        <span className="status-dot inactive" /> Not configured
      </div>
      <p className="settings-description">
        Semantic search finds gems by meaning, not just keywords.
        Powered by QMD (local, on-device).
        Requires Node.js 22+ and ~2GB for search models.
      </p>
      <button
        onClick={handleEnableSemanticSearch}
        className="action-button"
        disabled={setupInProgress}
      >
        Enable Semantic Search
      </button>

      {/* Setup progress */}
      {setupInProgress && setupSteps.map(step => (
        <div key={step.step} className="setup-step">
          <span className={`step-status ${step.status}`}>
            {step.status === 'running' ? '⏳' : step.status === 'done' ? '✅' : '❌'}
          </span>
          <span>{step.description}</span>
        </div>
      ))}
    </>
  ) : (
    <>
      <div className="settings-status">
        <span className="status-dot active" /> Ready
      </div>
      <div className="settings-info">
        <div>Provider: QMD</div>
        <div>Models: ~/.cache/qmd/models/</div>
      </div>
      <div className="settings-actions">
        <button onClick={handleRebuildIndex} className="action-button">
          Rebuild Index
        </button>
        <button onClick={handleDisableSemanticSearch} className="action-button secondary">
          Disable
        </button>
      </div>
      <p className="settings-note">
        Disabling requires an app restart. QMD stays installed.
      </p>
    </>
  )}
</div>
```

---

## Data Models

### SearchResult (provider output)

```json
{
  "gem_id": "550e8400-e29b-41d4-a716-446655440000",
  "score": 0.87,
  "matched_chunk": "ECS provides a simpler container orchestration layer...",
  "match_type": "Hybrid"
}
```

### GemSearchResult (command output to frontend)

```json
{
  "score": 0.87,
  "matched_chunk": "ECS provides a simpler container orchestration layer...",
  "match_type": "Hybrid",
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "source_type": "YouTube",
  "source_url": "https://youtube.com/watch?v=abc123",
  "domain": "youtube.com",
  "title": "ECS vs EKS — Container Orchestration Options",
  "author": "TechChannel",
  "description": "A comparison of ECS and EKS for production workloads",
  "captured_at": "2026-02-25T14:30:00Z",
  "tags": ["AWS", "ECS", "EKS", "containers"],
  "summary": "Comparison of ECS and EKS for container orchestration on AWS"
}
```

### QmdSetupResult (setup output)

```json
{
  "success": true,
  "node_version": "v24.1.0",
  "qmd_version": "1.2.3",
  "docs_indexed": 26,
  "error": null
}
```

---

## Correctness Properties

### Property 1: FTS Backward Compatibility

*For any* query that currently works with the existing `search_gems` command, calling the new `search_gems` command with `FtsResultProvider` active should return the same gems in the same order (scores may differ since they're derived, not from FTS5 rank directly).

**Validates: Requirements 2.3, 4.6**

### Property 2: Score Normalization

*For any* provider, all `SearchResult.score` values must be in the range [0.0, 1.0]. Values outside this range indicate a normalization bug.

**Validates: Requirements 1.3, 3.3d**

### Property 3: Empty Query Fallback

*For any* empty or whitespace-only query, `search_gems` should return the same results as `list_gems` — a paginated list of all gems, not an error.

**Validates: Existing behavior preservation**

### Property 4: Provider Independence

*For any* valid query, `FtsResultProvider.search()` and `QmdResultProvider.search()` should both return valid `Vec<SearchResult>` (possibly different results, but both structurally valid). Neither should crash or panic.

**Validates: Requirements 2, 3**

### Property 5: Fire-and-Forget Safety

*For any* call to `index_gem()` or `remove_gem()` on `QmdResultProvider`, the function must return `Ok(())` immediately, regardless of whether the spawned QMD process succeeds or fails.

**Validates: Requirement 3.4, 3.5**

### Property 6: Lifecycle Non-Blocking

*For any* failure in `search_provider.index_gem()`, the calling `save_gem` command must still succeed. Search indexing errors are logged but never propagated as command failures.

**Validates: Requirement 7.6**

### Property 7: Setup Idempotency

*For any* step in `setup_semantic_search`, if the tool is already installed (Node.js, SQLite, QMD) or the collection already exists, the step should succeed without reinstalling. Running setup twice should produce the same end state.

**Validates: Requirement 6**

### Property 8: Fallback Chain

*For any* configuration where `semantic_search_enabled` is `true` but QMD is unavailable (binary missing, collection not created), the system must fall back to `FtsResultProvider`. Search must never be completely broken.

**Validates: Requirements 5.4c, Frozen Decision 3**

---

## Error Handling

### Error Scenarios

1. **QMD binary not found**: `QmdResultProvider::check_availability()` returns `available: false`. At startup, `lib.rs` falls back to `FtsResultProvider`. Log warning.

2. **QMD query fails (stderr)**: `search()` returns `Err(stderr_message)`. Frontend shows error toast. User can retry or fall back to keyword search by disabling semantic search.

3. **QMD returns unexpected JSON**: `search()` returns `Err("Failed to parse QMD JSON output: ...")`. Needs investigation of actual QMD output format.

4. **Gem not found during join**: If `gem_store.get(gem_id)` returns `None` for a search result (orphaned index entry), the result is silently skipped. No error to the user.

5. **Setup step fails**: `setup_semantic_search` stops at the failed step, returns `QmdSetupResult { success: false, error: Some(description) }`. Settings are NOT changed. User can retry.

6. **Paths with spaces**: `Application Support` has a space. All `Command` args pass paths as single arguments (not shell-expanded), so spaces are handled correctly by `tokio::process::Command`.

7. **Index_gem fires but QMD not ready**: The spawned `qmd update` process may fail. Since it's fire-and-forget, the error is logged to stderr but doesn't affect the user. The next `reindex_all()` or manual "Rebuild Index" will catch up.

### Error Recovery Strategy

- **All errors return `Result<T, String>`**: Following the existing Jarvis pattern
- **Lifecycle hooks use `if let Err(e) = ... { eprintln!(...) }`**: Never propagate search errors to gem operations
- **Startup fallback**: If QMD is unavailable at startup despite `semantic_search_enabled: true`, fall back to FTS5 silently
- **Rebuild as recovery**: Any index corruption or staleness can be fixed via "Rebuild Index" button

---

## Testing Strategy

### Unit Tests

**`fts_provider.rs` tests**:
- `search`: Verify `GemPreview` → `SearchResult` mapping, score derivation, empty query
- `check_availability`: Always returns `true`
- `index_gem` / `remove_gem`: Always returns `Ok(())`

**`qmd_provider.rs` tests**:
- `extract_gem_id_from_path`: Absolute path, relative path, path with spaces, non-matching path
- `normalize_qmd_score`: Values in range, values out of range, zero, negative
- `find_qmd_binary`: Mock filesystem paths (harder to test — may need integration test)

**`provider.rs` tests**:
- `GemSearchResult` serialization/deserialization roundtrip
- `MatchType` serialization (verify enum variants serialize as expected)

**`commands.rs` setup helpers**:
- `check_node_version`: Parse various version strings (v24.1.0, v18.0.0, invalid)
- `extract_gem_id_from_path`: Various path formats

### Integration Tests

- **Full search flow**: Create gems → FtsResultProvider.search() → verify results contain correct gem_ids
- **Provider registration**: Mock settings → verify correct provider type selected
- **Lifecycle hooks**: Save gem → verify index_gem called → delete gem → verify remove_gem called
- **Setup flow**: Requires actual QMD installation — manual or CI-specific test

### Manual Testing Checklist

- [ ] Search with FTS (default) — verify results match existing behavior
- [ ] Enable semantic search in Settings — verify all 6 setup steps complete
- [ ] Search with QMD active — verify semantic results appear with score badges
- [ ] Search "container orchestration" → verify "ECS vs EKS" gem appears (semantic match)
- [ ] Save a new gem → verify QMD index updates (check `qmd status`)
- [ ] Delete a gem → verify QMD index updates
- [ ] Disable semantic search → restart → verify FTS5 is active again
- [ ] Rebuild Index button → verify `qmd update && qmd embed -f` runs
- [ ] Empty search query → verify all gems listed (no error)
- [ ] Start app with `semantic_search_enabled: true` but QMD uninstalled → verify fallback to FTS5

---

## Implementation Notes

### Performance Considerations

1. **FTS search latency**: <5ms for typical queries. No change from current behavior.
2. **QMD search latency**: ~100-500ms depending on index size and model inference. Acceptable for search bar with 300ms debounce.
3. **Index update latency**: `qmd update && qmd embed` takes ~2-5s for incremental updates. Fire-and-forget ensures no UI blocking.
4. **First-time model download**: ~1.9GB across 3 models. Takes 1-10 minutes depending on connection. Progress shown in setup UI.
5. **Memory**: QMD runs as a separate process. No memory impact on Jarvis when not searching. Models loaded on-demand by QMD.

### Open Questions (TBD After Testing QMD CLI)

1. **QMD JSON output schema**: Need to test `qmd query --json` to verify exact field names (`file`, `score`, `chunk`?). The `search()` implementation may need adjustment.
2. **QMD score range**: Are scores 0.0–1.0 or unbounded? `normalize_qmd_score()` needs calibration.
3. **QMD status --json schema**: Need to verify collection listing format for `check_availability()`.
4. **QMD collection root**: Does `qmd collection add` accept an absolute path with spaces? Need to test with `Application Support`.
5. **Incremental embed**: Does `qmd embed` (without `-f`) only process new/changed files? If so, `index_gem()` is efficient. If not, consider per-file embed commands.

---

## Summary

This design introduces a `SearchResultProvider` trait — the single abstraction for gem search. Tauri commands call the trait and receive `Vec<SearchResult>` in a standard format. `FtsResultProvider` wraps the existing FTS5 search (always available, zero setup). `QmdResultProvider` wraps QMD CLI for semantic search (opt-in, automated setup via Settings). The trait follows Jarvis's established pattern (`IntelProvider`, `KnowledgeStore`, `GemStore`, `Chatable`) and makes adding future search backends a one-file change. Provider selection is settings-driven with automatic fallback to FTS5 when QMD is unavailable. All lifecycle integration is fire-and-forget and non-blocking.
