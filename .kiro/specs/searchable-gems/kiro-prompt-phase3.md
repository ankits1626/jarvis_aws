# Kiro Prompt — Searchable Gems Phase 3: QmdResultProvider (Semantic Search)

## What You're Building

Implement `QmdResultProvider` — the **opt-in** semantic search provider that wraps the [QMD](https://github.com/tobi/qmd) CLI binary. QMD combines BM25 keyword matching, vector embeddings, and LLM-based reranking for hybrid search over markdown files.

This provider shells out to the `qmd` CLI via `tokio::process::Command` — same pattern as `MlxProvider` calling the Python sidecar and `VenvManager` running `pip install`. The key difference: `index_gem()` and `remove_gem()` are **fire-and-forget** (`tokio::spawn`, don't await), while `search()` and `reindex_all()` **await** the result.

## Spec Files

- **Requirements**: `.kiro/specs/searchable-gems/requirements.md` — Requirement 3
- **Design**: `.kiro/specs/searchable-gems/design.md` — Section `qmd_provider.rs` (full code)
- **Tasks**: `.kiro/specs/searchable-gems/tasks.md` — Phase 3, Task 4

## Context: What Already Exists

Phase 1 created the trait in `src/search/provider.rs`. Phase 2 created `FtsResultProvider`. Now you're implementing the second provider.

The codebase already uses `tokio::process::Command` extensively:

```rust
// Pattern from mlx_provider.rs — check version
let output = Command::new(python_path)
    .arg("--version")
    .output()
    .await
    .map_err(|e| format!("Failed to check Python version: {}", e))?;

if !output.status.success() {
    return Err(format!("Check failed: {}", String::from_utf8_lossy(&output.stderr)));
}
```

```rust
// Pattern from venv_manager.rs — run install
let output = Command::new(&pip_path)
    .args(["install", "-r", &requirements_path.to_string_lossy()])
    .output()
    .await
    .map_err(|e| format!("Failed to run pip install: {}", e))?;
```

## Exact Task

### Replace the contents of `jarvis-app/src-tauri/src/search/qmd_provider.rs`

**Imports:**
```rust
use std::path::PathBuf;
use async_trait::async_trait;
use tokio::process::Command;
use crate::intelligence::AvailabilityResult;
use super::provider::{SearchResultProvider, SearchResult, MatchType};
```

**Struct:**
```rust
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
```

**Constructor + static helper:**
```rust
impl QmdResultProvider {
    pub fn new(qmd_path: PathBuf, knowledge_path: PathBuf) -> Self {
        Self { qmd_path, knowledge_path }
    }

    /// Try to find the qmd binary in common locations.
    /// Checks: /opt/homebrew/bin/qmd → /usr/local/bin/qmd → `which qmd`
    pub async fn find_qmd_binary() -> Option<PathBuf> {
        // 1. Check /opt/homebrew/bin/qmd (Apple Silicon Mac)
        let homebrew_path = PathBuf::from("/opt/homebrew/bin/qmd");
        if homebrew_path.exists() {
            return Some(homebrew_path);
        }

        // 2. Check /usr/local/bin/qmd (Intel Mac)
        let usr_local_path = PathBuf::from("/usr/local/bin/qmd");
        if usr_local_path.exists() {
            return Some(usr_local_path);
        }

        // 3. Try `which qmd`
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
```

**Trait Implementation — method by method:**

### 1. `check_availability()`

Three sequential checks. If any fails, return `available: false` with a specific reason.

```
Step 1: Check self.qmd_path.exists()
        → false: return { available: false, reason: "QMD binary not found at {path}" }

Step 2: Run `{qmd_path} --version`, check output.status.success()
        → error or non-zero: return { available: false, reason: "Failed to run qmd --version: {e}" }

Step 3: Run `{qmd_path} status --json`, check stdout contains "jarvis-gems"
        → missing: return { available: false, reason: "QMD jarvis-gems collection not found. Run setup first." }
        → command fails: return { available: false, reason: "Failed to check QMD status" }

All pass: return { available: true, reason: None }
```

**Important:** All `Command::new()` calls use `&self.qmd_path`, NOT the bare string `"qmd"`. We never assume `qmd` is in `$PATH`.

### 2. `search()`

Run `qmd query "{query}" --json -n {limit}` and parse the JSON output.

```rust
async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
    let output = Command::new(&self.qmd_path)
        .args(["query", query, "--json", "-n", &limit.to_string()])
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
        let file_path = item.get("file")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        let gem_id = extract_gem_id_from_path(file_path, &self.knowledge_path);

        if let Some(gem_id) = gem_id {
            let raw_score = item.get("score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let matched_chunk = item.get("chunk")
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
```

**Note:** This needs `serde_json` in the imports. Add `use serde_json;` — it's already a dependency in `Cargo.toml`.

### 3. `index_gem()` — Fire-and-forget

Spawn `qmd update && qmd embed` as a background task. Return `Ok(())` immediately without awaiting the result.

```rust
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
```

**Key:** The `tokio::spawn` moves `qmd_path` into the closure. The outer function returns `Ok(())` before the spawned task finishes. If QMD fails, the error is silently dropped — this is intentional (fire-and-forget, documented in design).

### 4. `remove_gem()` — Fire-and-forget

Same pattern but only `qmd update` (QMD detects deleted files):

```rust
async fn remove_gem(&self, _gem_id: &str) -> Result<(), String> {
    let qmd_path = self.qmd_path.clone();
    tokio::spawn(async move {
        let _ = Command::new(&qmd_path)
            .arg("update")
            .output()
            .await;
    });

    Ok(())
}
```

### 5. `reindex_all()` — Await completion

Unlike the fire-and-forget methods, this one **awaits** both commands and returns the result:

```rust
async fn reindex_all(&self) -> Result<usize, String> {
    // qmd update (discover new/changed/deleted files)
    let update = Command::new(&self.qmd_path)
        .arg("update")
        .output()
        .await
        .map_err(|e| format!("qmd update failed: {}", e))?;

    if !update.status.success() {
        let stderr = String::from_utf8_lossy(&update.stderr);
        return Err(format!("qmd update failed: {}", stderr));
    }

    // qmd embed -f (force re-embed all documents)
    let embed = Command::new(&self.qmd_path)
        .args(["embed", "-f"])
        .output()
        .await
        .map_err(|e| format!("qmd embed failed: {}", e))?;

    if !embed.status.success() {
        let stderr = String::from_utf8_lossy(&embed.stderr);
        return Err(format!("qmd embed failed: {}", stderr));
    }

    // TODO: Parse output for indexed document count
    // For now return 0 — refine after testing actual QMD CLI output
    Ok(0)
}
```

### Two helper functions (module-private, outside the impl blocks)

Place these at the bottom of the file, after the `impl SearchResultProvider` block:

```rust
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
    raw_score.clamp(0.0, 1.0)
}
```

**These are `fn`, not `pub fn`** — they're internal helpers, not public API.

### Update `mod.rs` re-exports

Replace the Phase 3 comment in `src/search/mod.rs` with:
```rust
pub use qmd_provider::QmdResultProvider;
```

## Complete imports for `qmd_provider.rs`

```rust
use std::path::PathBuf;
use async_trait::async_trait;
use tokio::process::Command;
use crate::intelligence::AvailabilityResult;
use super::provider::{SearchResultProvider, SearchResult, MatchType};
```

**Note:** You'll also need `serde_json` for parsing QMD's JSON output in `search()`. Either:
- Add `use serde_json;` at the top (if the crate is already a dependency — check `Cargo.toml`), OR
- Use the fully qualified `serde_json::from_str` / `serde_json::Value` inline

`serde_json` IS already in `Cargo.toml` — it's used throughout the codebase. So just add the import.

## Gotchas

1. **Always use `&self.qmd_path`** for `Command::new()`. Never use bare `"qmd"`. The binary might not be in `$PATH`.

2. **`tokio::spawn` needs owned data.** In `index_gem()` and `remove_gem()`, clone `self.qmd_path` before the `move` closure: `let qmd_path = self.qmd_path.clone();`

3. **`PathBuf` in the struct, `&PathBuf` in helpers.** The helper `extract_gem_id_from_path` takes `&PathBuf` (borrowed). The struct field is owned `PathBuf`.

4. **`serde_json::Value`** — QMD's exact JSON schema is TBD. We use dynamic `Value` parsing with `.get("file")`, `.get("score")`, `.get("chunk")` and graceful `.unwrap_or_default()` fallbacks. This is intentional — it will be refined after testing QMD's actual output.

5. **`find_qmd_binary()` is `pub async fn`** (associated function, no `&self`). It's a static factory helper called during provider registration in `lib.rs`, not a trait method.

6. **Helper functions are NOT `pub`** — `extract_gem_id_from_path` and `normalize_qmd_score` are private to this module.

7. **`_gem_id` prefix** on `index_gem` and `remove_gem` parameters — the gem_id isn't used directly since QMD's `update` command scans the whole collection directory for changes.

## Verification

Run `cargo check` from `jarvis-app/src-tauri/`. Must pass with zero new errors.

**Expected outcome:**
- 1 file modified: `src/search/qmd_provider.rs` (placeholder → full implementation)
- 1 file modified: `src/search/mod.rs` (added `QmdResultProvider` re-export)
- `cargo check` passes

## If You're Unsure

- **Is `serde_json` available?** → Yes, it's in `Cargo.toml`. Used everywhere in the codebase.
- **Should I use `Command::new("qmd")` or `Command::new(&self.qmd_path)`?** → Always `&self.qmd_path`. Never bare `"qmd"`.
- **What if `qmd query` returns empty array?** → Fine. `search()` returns `Ok(vec![])`.
- **What is the actual QMD JSON schema?** → TBD. We assume `[{ "file": "...", "score": 0.87, "chunk": "..." }]` for now. The `// TODO` comments indicate where to refine after testing.
- **Does `tokio::spawn` need an error return?** → No. Fire-and-forget tasks silently drop errors. The outer function returns `Ok(())` immediately.
- **Should `find_qmd_binary` be on the trait?** → No. It's an associated function on `QmdResultProvider` (called as `QmdResultProvider::find_qmd_binary()`), not a trait method.
- **Anything else?** → Ask before guessing.

## When Done

Stop and ask for review. Show me:
1. The full `qmd_provider.rs` content
2. The updated `mod.rs`
3. `cargo check` output
4. Any questions or decisions you made

Do NOT proceed to Phase 4 until I review and approve.
