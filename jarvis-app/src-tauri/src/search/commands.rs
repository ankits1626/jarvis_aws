use std::sync::{Arc, RwLock};
use tauri::{Emitter, Manager, State};

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
    eprintln!("Search: search_gems called — query=\"{}\" limit={}", query, limit);

    // Handle empty query — delegate to gem_store.list() for consistency
    if query.trim().is_empty() {
        let gems = gem_store.list(limit, 0).await?;
        eprintln!("Search: Empty query — returning {} gems from list", gems.len());
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

    // Search via provider (QMD semantic or FTS5)
    let search_results = provider.search(&query, limit).await?;
    eprintln!("Search: Provider returned {} raw results for \"{}\"", search_results.len(), query);

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
        } else {
            eprintln!("Search: Gem {} not found in DB (orphaned index entry)", result.gem_id);
        }
    }

    // Fallback: if semantic provider returned 0 results, try FTS5 keyword search
    if enriched.is_empty() {
        eprintln!("Search: Semantic returned 0 results, falling back to FTS5 for \"{}\"", query);
        if let Ok(fts_results) = gem_store.search(&query, limit).await {
            eprintln!("Search: FTS5 fallback returned {} results for \"{}\"", fts_results.len(), query);
            for gem in fts_results {
                enriched.push(GemSearchResult {
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
                });
            }
        }
    }

    eprintln!("Search: Returning {} enriched results for \"{}\"", enriched.len(), query);
    Ok(enriched)
}

/// Check if the active search provider is available.
#[tauri::command]
pub async fn check_search_availability(
    provider: State<'_, Arc<dyn SearchResultProvider>>,
) -> Result<AvailabilityResult, String> {
    eprintln!("Search: check_search_availability called");
    let result = provider.check_availability().await;
    eprintln!("Search: availability = {} (reason: {:?})", result.available, result.reason);
    Ok(result)
}

/// Run the automated QMD semantic search setup flow.
///
/// Steps: check Node.js → install SQLite → install QMD → create collection → index → save setting
/// Emits progress events on "semantic-search-setup" channel.
#[tauri::command]
pub async fn setup_semantic_search(
    app_handle: tauri::AppHandle,
    settings_manager: State<'_, Arc<RwLock<SettingsManager>>>,
) -> Result<QmdSetupResult, String> {
    let total_steps = 7;

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
    let node_version = match check_node_version().await {
        Ok(v) => v,
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
    emit_progress(1, "Checking Node.js", "done");

    // Step 2: Check/Install SQLite
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

    // Step 3: Install QMD
    emit_progress(3, "Installing QMD", "running");
    let qmd_version = match check_or_install_qmd().await {
        Ok(v) => v,
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
    emit_progress(3, "Installing QMD", "done");

    // Step 3b: Patch QMD reranker context size (2048 → 4096)
    // QMD's character-based chunking at query time can produce chunks that exceed
    // the reranker's 2048-token context window, causing crashes on certain queries.
    // Qwen3-Reranker-0.6B supports up to 32K context, so 4096 is safe.
    if let Err(e) = patch_qmd_rerank_context_size().await {
        eprintln!("Search/Setup: QMD reranker patch failed (non-fatal): {}", e);
    }

    // Step 4: Create collection
    emit_progress(4, "Creating search collection", "running");
    let knowledge_path = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("knowledge");
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

    // Step 5: Index & embed
    emit_progress(5, "Indexing gems (downloading models on first run, ~1.9GB)", "running");
    let docs_indexed = match run_qmd_index().await {
        Ok(count) => count,
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
    emit_progress(5, "Indexing gems", "done");

    // Step 6: Warm up query expansion model (~1.2GB download on first run)
    // QMD downloads the query expansion model lazily on first `qmd query`.
    // We run a dummy query here so the user doesn't wait on first real search.
    emit_progress(6, "Downloading query model (~1.2GB, first time only)", "running");
    if let Err(e) = warm_up_qmd_query_model().await {
        // Non-fatal — model will download on first real query instead
        eprintln!("Search/Setup: Query model warm-up failed (non-fatal): {}", e);
    }
    emit_progress(6, "Downloading query model", "done");

    // Step 7: Save setting
    emit_progress(7, "Saving settings", "running");
    {
        let manager = settings_manager.read()
            .map_err(|e| format!("Failed to acquire settings read lock: {}", e))?;
        let mut settings = manager.get();
        settings.search.semantic_search_enabled = true;
        manager.update(settings)?;
    }
    emit_progress(7, "Saving settings", "done");

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
    eprintln!("Search: rebuild_search_index called");
    let result = provider.reindex_all().await;
    match &result {
        Ok(count) => eprintln!("Search: rebuild_search_index completed — {} docs indexed", count),
        Err(e) => eprintln!("Search: rebuild_search_index FAILED: {}", e),
    }
    result
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

/// Run a dummy QMD query to trigger download of the query expansion model.
///
/// QMD downloads 3 models: embedding + reranking (during `qmd embed`)
/// and query expansion (~1.2GB, during first `qmd query`).
/// This function ensures all models are ready before the user's first search.
async fn warm_up_qmd_query_model() -> Result<(), String> {
    eprintln!("Search/Setup: Running warm-up query to download query expansion model...");
    let output = tokio::process::Command::new("qmd")
        .args(["query", "test", "--json", "-n", "1"])
        .output()
        .await
        .map_err(|e| format!("qmd warm-up query failed: {}", e))?;

    if output.status.success() {
        eprintln!("Search/Setup: Warm-up query succeeded — all models ready");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Search/Setup: Warm-up query failed: {}", stderr.trim());
        Err(format!("qmd warm-up query failed: {}", stderr))
    }
}

/// Patch QMD's reranker context size from 2048 to 4096.
///
/// QMD uses character-based chunking at query time (3600 chars ≈ 900 tokens estimate),
/// but markdown with URLs/short words can produce chunks of 1200-1800 actual tokens.
/// Combined with ~200 tokens of template overhead, this exceeds the 2048-token context.
/// Qwen3-Reranker-0.6B supports up to 32K context, so 4096 is well within range.
async fn patch_qmd_rerank_context_size() -> Result<(), String> {
    // Find QMD's installation path via `npm root -g`
    let output = tokio::process::Command::new("npm")
        .args(["root", "-g"])
        .output()
        .await
        .map_err(|e| format!("Failed to find npm global root: {}", e))?;

    if !output.status.success() {
        return Err("npm root -g failed".to_string());
    }

    let npm_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let llm_js_path = std::path::PathBuf::from(&npm_root)
        .join("@tobilu/qmd/dist/llm.js");

    if !llm_js_path.exists() {
        return Err(format!("QMD llm.js not found at {}", llm_js_path.display()));
    }

    let content = tokio::fs::read_to_string(&llm_js_path)
        .await
        .map_err(|e| format!("Failed to read llm.js: {}", e))?;

    if content.contains("RERANK_CONTEXT_SIZE = 4096") {
        eprintln!("Search/Setup: QMD reranker already patched to 4096");
        return Ok(());
    }

    if !content.contains("RERANK_CONTEXT_SIZE = 2048") {
        return Err("RERANK_CONTEXT_SIZE = 2048 not found in llm.js — QMD version may have changed".to_string());
    }

    let patched = content.replace(
        "RERANK_CONTEXT_SIZE = 2048",
        "RERANK_CONTEXT_SIZE = 4096",
    );

    tokio::fs::write(&llm_js_path, patched)
        .await
        .map_err(|e| format!("Failed to write patched llm.js: {}", e))?;

    eprintln!("Search/Setup: Patched QMD reranker context size 2048 → 4096 at {}", llm_js_path.display());
    Ok(())
}
