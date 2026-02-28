# Kiro Prompt — Searchable Gems Phase 4+5: Settings Extension + Tauri Commands

## What You're Building

Two tightly-coupled pieces:

1. **Phase 4** — Add `SearchSettings` sub-struct to the `Settings` struct (1 new field: `semantic_search_enabled`)
2. **Phase 5** — Create `src/search/commands.rs` with 4 Tauri commands: `search_gems`, `check_search_availability`, `setup_semantic_search`, `rebuild_search_index`

These are combined because the commands reference `SearchSettings` and the settings change is trivial.

## Spec Files

- **Requirements**: `.kiro/specs/searchable-gems/requirements.md` — Requirements 4 (commands), 5 (settings), 6 (setup flow)
- **Design**: `.kiro/specs/searchable-gems/design.md` — Sections `commands.rs`, `Settings Extension`
- **Tasks**: `.kiro/specs/searchable-gems/tasks.md` — Phase 4 (Task 5) and Phase 5 (Tasks 6-7)

## Context: What Already Exists

**Settings pattern** — `src/settings/manager.rs` has:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub transcription: TranscriptionSettings,
    #[serde(default)]
    pub browser: BrowserSettings,
    #[serde(default)]
    pub intelligence: IntelligenceSettings,
    #[serde(default)]
    pub copilot: CoPilotSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            transcription: TranscriptionSettings::default(),
            browser: BrowserSettings::default(),
            intelligence: IntelligenceSettings::default(),
            copilot: CoPilotSettings::default(),
        }
    }
}
```

**SettingsManager** has `get()` and `update()`:
```rust
pub fn get(&self) -> Settings {
    self.current_settings.read().expect("...").clone()
}

pub fn update(&self, settings: Settings) -> Result<(), String> {
    Self::validate(&settings)?;
    self.save_to_file(&settings)?;
    *self.current_settings.write()...? = settings;
    Ok(())
}
```

**SettingsManager in Tauri commands** is accessed as:
```rust
settings_manager: State<'_, Arc<RwLock<SettingsManager>>>,
```

**Tauri command pattern** from `knowledge/commands.rs`:
```rust
use std::sync::Arc;
use tauri::State;
use crate::gems::GemStore;
use crate::knowledge::store::{KnowledgeEntry, KnowledgeStore};

#[tauri::command]
pub async fn get_gem_knowledge(
    gem_id: String,
    knowledge_store: State<'_, Arc<dyn KnowledgeStore>>,
) -> Result<Option<KnowledgeEntry>, String> {
    knowledge_store.get(&gem_id).await
}
```

**Existing `search_gems` command** in `commands.rs` (line ~461, will be REMOVED in Phase 6):
```rust
#[tauri::command]
pub async fn search_gems(
    query: String,
    limit: Option<usize>,
    gem_store: State<'_, Arc<dyn GemStore>>,
) -> Result<Vec<GemPreview>, String> {
    gem_store.search(&query, limit.unwrap_or(50)).await
}
```

**`delete_gem` pattern** — shows how to access optional state with `try_state`:
```rust
#[tauri::command]
pub async fn delete_gem(
    app_handle: tauri::AppHandle,
    id: String,
    gem_store: State<'_, Arc<dyn GemStore>>,
) -> Result<(), String> {
    gem_store.delete(&id).await?;

    // Delete knowledge files
    if let Some(ks) = app_handle.try_state::<Arc<dyn crate::knowledge::KnowledgeStore>>() {
        if let Err(e) = ks.delete(&id).await {
            eprintln!("Knowledge file deletion failed for gem {}: {}", id, e);
        }
    }

    Ok(())
}
```

**Gem struct** (from `gems/store.rs`) — the full struct you'll join search results with:
```rust
pub struct Gem {
    pub id: String,
    pub source_type: String,
    pub source_url: String,
    pub domain: String,
    pub title: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub source_meta: serde_json::Value,
    pub captured_at: String,
    pub ai_enrichment: Option<serde_json::Value>,  // {"tags": [...], "summary": "...", ...}
    pub transcript: Option<String>,
    pub transcript_language: Option<String>,
}
```

---

## Part A: Phase 4 — Settings Extension

### Modify `jarvis-app/src-tauri/src/settings/manager.rs`

**Step 1: Add `SearchSettings` struct** (place it near the other sub-settings structs, e.g., after `CoPilotSettings`):

```rust
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

**Step 2: Add `search` field to `Settings` struct:**

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
```

**Step 3: Add `search` to `Default for Settings`:**

```rust
impl Default for Settings {
    fn default() -> Self {
        Self {
            transcription: TranscriptionSettings::default(),
            browser: BrowserSettings::default(),
            intelligence: IntelligenceSettings::default(),
            copilot: CoPilotSettings::default(),
            search: SearchSettings::default(),    // ← NEW
        }
    }
}
```

**Step 4: Add `SearchSettings` to `settings/mod.rs` re-exports:**

The existing re-export line is:
```rust
pub use manager::{BrowserSettings, CoPilotSettings, IntelligenceSettings, Settings, SettingsManager, TranscriptionSettings};
```

Add `SearchSettings` to it:
```rust
pub use manager::{BrowserSettings, CoPilotSettings, IntelligenceSettings, SearchSettings, Settings, SettingsManager, TranscriptionSettings};
```

**Key:** The `#[serde(default)]` attribute on the `search` field ensures backward compatibility — existing `settings.json` files without a `search` key will deserialize successfully with `SearchSettings::default()` (semantic search disabled).

---

## Part B: Phase 5 — Tauri Commands

### Replace the contents of `jarvis-app/src-tauri/src/search/commands.rs`

**Imports:**
```rust
use std::sync::Arc;
use std::sync::RwLock;
use tauri::State;
use crate::gems::GemStore;
use crate::intelligence::AvailabilityResult;
use crate::settings::SettingsManager;
use super::provider::{SearchResultProvider, SearchResult, MatchType, GemSearchResult, QmdSetupResult, SetupProgressEvent};
```

### Command 1: `search_gems`

This is the most important command — it replaces the existing one in `commands.rs`.

```rust
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

    // Handle empty query — return all gems (consistent with existing behavior)
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
```

**Important details:**
- Empty query fallback uses `gem_store.list(limit, 0)` — `list` returns `Vec<GemPreview>`, which has `tags` and `summary` already extracted. No need to parse `ai_enrichment` JSON for list results.
- Non-empty query uses `gem_store.get(gem_id)` — `get` returns `Option<Gem>`. The full `Gem` struct has `ai_enrichment` as `Option<serde_json::Value>`, so we must extract `tags` and `summary` from the JSON.
- Orphaned results (gem in search index but deleted from DB) are silently skipped.

### Command 2: `check_search_availability`

```rust
/// Check if the active search provider is available.
#[tauri::command]
pub async fn check_search_availability(
    provider: State<'_, Arc<dyn SearchResultProvider>>,
) -> Result<AvailabilityResult, String> {
    Ok(provider.check_availability().await)
}
```

### Command 3: `rebuild_search_index`

```rust
/// Rebuild the search index from scratch.
#[tauri::command]
pub async fn rebuild_search_index(
    provider: State<'_, Arc<dyn SearchResultProvider>>,
) -> Result<usize, String> {
    provider.reindex_all().await
}
```

### Command 4: `setup_semantic_search`

This is the 6-step automated setup flow. It needs `app_handle` for emitting progress events and `settings_manager` for saving the setting.

```rust
/// Run the automated QMD semantic search setup flow.
///
/// Steps: check Node.js → install SQLite → install QMD → create collection → index → save setting
/// Emits progress events on "semantic-search-setup" channel.
#[tauri::command]
pub async fn setup_semantic_search(
    app_handle: tauri::AppHandle,
    settings_manager: State<'_, Arc<RwLock<SettingsManager>>>,
) -> Result<QmdSetupResult, String> {
    use tauri::Emitter;

    let total_steps = 6;

    // Helper closure to emit progress events
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
    let node_version = match check_node_version().await {
        Ok(v) => {
            emit_progress(1, "Checking Node.js", "done");
            v
        }
        Err(e) => {
            emit_progress(1, "Checking Node.js", "failed");
            return Ok(QmdSetupResult {
                success: false,
                node_version: None,
                qmd_version: None,
                docs_indexed: None,
                error: Some(e),
            });
        }
    };

    // Step 2: Check/Install SQLite via Homebrew
    emit_progress(2, "Checking SQLite", "running");
    if let Err(e) = check_or_install_sqlite().await {
        emit_progress(2, "Checking SQLite", "failed");
        return Ok(QmdSetupResult {
            success: false,
            node_version: Some(node_version),
            qmd_version: None,
            docs_indexed: None,
            error: Some(e),
        });
    }
    emit_progress(2, "Checking SQLite", "done");

    // Step 3: Install QMD via npm
    emit_progress(3, "Installing QMD", "running");
    let qmd_version = match check_or_install_qmd().await {
        Ok(v) => {
            emit_progress(3, "Installing QMD", "done");
            v
        }
        Err(e) => {
            emit_progress(3, "Installing QMD", "failed");
            return Ok(QmdSetupResult {
                success: false,
                node_version: Some(node_version),
                qmd_version: None,
                docs_indexed: None,
                error: Some(e),
            });
        }
    };

    // Step 4: Create QMD collection
    emit_progress(4, "Creating search collection", "running");
    let app_data_dir = app_handle.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let knowledge_path = app_data_dir.join("knowledge");
    if let Err(e) = create_qmd_collection(&knowledge_path).await {
        emit_progress(4, "Creating search collection", "failed");
        return Ok(QmdSetupResult {
            success: false,
            node_version: Some(node_version),
            qmd_version: Some(qmd_version),
            docs_indexed: None,
            error: Some(e),
        });
    }
    emit_progress(4, "Creating search collection", "done");

    // Step 5: Index & embed (downloads ~1.9GB of models on first run)
    emit_progress(5, "Indexing gems (downloading models on first run, ~1.9GB)", "running");
    let docs_indexed = match run_qmd_index().await {
        Ok(n) => {
            emit_progress(5, "Indexing gems", "done");
            n
        }
        Err(e) => {
            emit_progress(5, "Indexing gems", "failed");
            return Ok(QmdSetupResult {
                success: false,
                node_version: Some(node_version),
                qmd_version: Some(qmd_version),
                docs_indexed: None,
                error: Some(e),
            });
        }
    };

    // Step 6: Save setting
    emit_progress(6, "Saving settings", "running");
    {
        let sm = settings_manager.read()
            .map_err(|e| format!("Failed to read settings: {}", e))?;
        let mut settings = sm.get();
        settings.search.semantic_search_enabled = true;
        sm.update(settings)
            .map_err(|e| format!("Failed to save settings: {}", e))?;
    }
    emit_progress(6, "Saving settings", "done");

    Ok(QmdSetupResult {
        success: true,
        node_version: Some(node_version),
        qmd_version: Some(qmd_version),
        docs_indexed: Some(docs_indexed),
        error: None,
    })
}
```

**Important:** The `emit_progress` closure captures `app_handle` by reference. You need `use tauri::Emitter;` for the `.emit()` method to be available. Place it inside the function body (not at the top of the file) to keep the import scoped — OR add `use tauri::Emitter;` to the file-level imports. Either works. The design doc places it inside the function.

**Important:** `app_handle.path().app_data_dir()` — in Tauri 2.x, `.path()` returns a `PathResolver`. You need `use tauri::Manager;` for this to work. Add it to the file-level imports.

### Setup helper functions

Place these at the bottom of the file, after all the `#[tauri::command]` functions:

```rust
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

**Note:** The setup helpers use bare `"qmd"` / `"node"` / `"npm"` / `"brew"` for `Command::new()` — this is intentional. During setup, QMD hasn't been located yet (we're installing it). After setup, `QmdResultProvider` will use the discovered `qmd_path`. The helpers run `qmd` from `$PATH` because `npm install -g` puts it there.

---

## Complete file-level imports for `commands.rs`

```rust
use std::sync::Arc;
use std::sync::RwLock;
use tauri::{State, Manager, Emitter};
use crate::gems::GemStore;
use crate::intelligence::AvailabilityResult;
use crate::settings::SettingsManager;
use super::provider::{SearchResultProvider, MatchType, GemSearchResult, QmdSetupResult, SetupProgressEvent};
```

**Notes on imports:**
- `Manager` — needed for `app_handle.path()` in Tauri 2.x
- `Emitter` — needed for `app_handle.emit()` in Tauri 2.x
- `SearchResult` is NOT imported at file level — it's only used internally by `search_gems` (the provider returns it, the command destructures it). If the compiler complains it's unused, remove it. If you need it for the type in the for loop, add it.
- `RwLock` — used for `State<'_, Arc<RwLock<SettingsManager>>>`

**If `Manager` or `Emitter` traits cause issues:** Check how the existing codebase imports them. Search for `use tauri::Emitter` or `use tauri::Manager` in `commands.rs` or `lib.rs`. The exact import path may vary by Tauri 2.x version. If the crate uses `tauri::Manager` already, match that pattern.

---

## Gotchas

1. **`Arc<RwLock<SettingsManager>>`** — SettingsManager is wrapped in `Arc<RwLock<>>`, not bare `Arc<>`. This is different from `GemStore` (which is `Arc<dyn GemStore>`). The command signature must be `State<'_, Arc<RwLock<SettingsManager>>>`.

2. **Reading settings in the setup command** — Use `settings_manager.read().map_err(...)?.get()` to get a clone of the current settings, mutate, then `sm.update(settings)` to persist. The read lock is released after the block.

3. **Empty query → `gem_store.list()`** — When query is empty, we call `gem_store.list(limit, 0)` which returns `Vec<GemPreview>`. `GemPreview` already has `tags` and `summary` extracted (no JSON parsing needed). Just map fields directly.

4. **Non-empty query → `gem_store.get()`** — For search results, we call `gem_store.get(gem_id)` which returns `Option<Gem>`. The `Gem` struct has `ai_enrichment: Option<serde_json::Value>`, so we must extract `tags` and `summary` from the JSON blob.

5. **Setup returns `Ok(QmdSetupResult { success: false, ... })` on failure** — NOT `Err(...)`. The command succeeds (no Tauri error), but the result signals failure. This lets the frontend show step-by-step progress rather than a generic error toast.

6. **`app_handle.path().app_data_dir()`** — Returns the Tauri app data directory (e.g., `~/Library/Application Support/com.jarvis.app/`). The knowledge files are at `{app_data_dir}/knowledge/`.

7. **Setup helpers use bare command names** — `"qmd"`, `"node"`, `"npm"`, `"brew"` — because they're installing tools to `$PATH`. This is intentional and different from `QmdResultProvider` which uses `&self.qmd_path`.

8. **`use tauri::Emitter`** — Required for `.emit()` in Tauri 2.x. If unsure, check how the existing codebase calls `app_handle.emit()` — search for `emit(` in `commands.rs` or `lib.rs` and match the import pattern.

---

## Verification

Run `cargo check` from `jarvis-app/src-tauri/`. Must pass with zero new errors.

**Expected outcome:**
- 2 files modified: `src/settings/manager.rs` (added `SearchSettings`), `src/settings/mod.rs` (added re-export)
- 1 file modified: `src/search/commands.rs` (placeholder → full implementation with 4 commands + 5 helpers)
- `cargo check` passes

**Do NOT yet:**
- Remove the old `search_gems` from `commands.rs` (that's Phase 6)
- Register the new commands in `lib.rs` (that's Phase 6)
- Wire lifecycle hooks (that's Phase 6)

The new `search_gems` in `search/commands.rs` will temporarily "shadow" the old one — this is fine because neither is registered in `generate_handler!` yet. The old one stays until Phase 6 removes it.

## If You're Unsure

- **How is SettingsManager accessed in commands?** → `State<'_, Arc<RwLock<SettingsManager>>>`. Use `settings_manager.read().map_err(...)?.get()` to read, `.update(settings)` to write.
- **Does `Gem` have `tags` and `summary` as direct fields?** → No. `Gem` has `ai_enrichment: Option<serde_json::Value>`. Tags and summary must be extracted from the JSON. `GemPreview` does have them as direct fields.
- **Should setup return `Err` or `Ok(failure_result)` on step failure?** → `Ok(QmdSetupResult { success: false, error: Some(...) })`. Not `Err`.
- **What's `app_handle.path().app_data_dir()` return type?** → `Result<PathBuf, tauri::Error>`. Map the error to `String`.
- **Do I need `use tauri::Manager`?** → Yes, for `app_handle.path()`. Check how existing code imports it.
- **Anything else?** → Ask before guessing.

## When Done

Stop and ask for review. Show me:
1. The changes to `settings/manager.rs` (the diff or new code sections)
2. The updated `settings/mod.rs` re-export line
3. The full `search/commands.rs` content
4. `cargo check` output
5. Any decisions you made (especially around import paths for `Emitter`/`Manager`)

Do NOT proceed to Phase 6 until I review and approve.
