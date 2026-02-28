// QmdResultProvider - semantic search provider wrapping QMD CLI

use std::path::PathBuf;
use async_trait::async_trait;
use tokio::process::Command;
use crate::intelligence::AvailabilityResult;
use super::provider::{SearchResultProvider, SearchResult, MatchType};

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
    /// Kept for future use (e.g., verifying gem files exist before indexing)
    #[allow(dead_code)]
    knowledge_path: PathBuf,
    /// Minimum relevance score (0.0–1.0) — results below this are discarded
    min_score: f64,
}

impl QmdResultProvider {
    pub fn new(qmd_path: PathBuf, knowledge_path: PathBuf, accuracy_pct: u8) -> Self {
        let min_score = (accuracy_pct.min(100) as f64) / 100.0;
        eprintln!("Search/QMD: min_score set to {:.0}%", min_score * 100.0);
        Self {
            qmd_path,
            knowledge_path,
            min_score,
        }
    }

    /// Try to find the qmd binary in common locations.
    ///
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

#[async_trait]
impl SearchResultProvider for QmdResultProvider {
    async fn check_availability(&self) -> AvailabilityResult {
        eprintln!("Search/QMD: Checking availability (binary: {})", self.qmd_path.display());

        // 1. Check binary exists
        if !self.qmd_path.exists() {
            eprintln!("Search/QMD: Binary not found at {}", self.qmd_path.display());
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
                eprintln!("Search/QMD: qmd --version failed: {}", e);
                return AvailabilityResult {
                    available: false,
                    reason: Some(format!("Failed to run qmd --version: {}", e)),
                };
            }
            Ok(output) if !output.status.success() => {
                eprintln!("Search/QMD: qmd --version returned non-zero exit code");
                return AvailabilityResult {
                    available: false,
                    reason: Some("qmd --version returned non-zero exit code".to_string()),
                };
            }
            Ok(ref output) => {
                let version = String::from_utf8_lossy(&output.stdout);
                eprintln!("Search/QMD: qmd version = {}", version.trim());
            }
        }

        // 3. Check collection exists via `qmd status --json`
        let status_result = Command::new(&self.qmd_path)
            .args(["status", "--json"])
            .output()
            .await;

        match status_result {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                eprintln!("Search/QMD: qmd status output: {}", stdout.trim());
                // Check that the output mentions the jarvis-gems collection
                // (exact JSON parsing depends on QMD's actual schema — TBD)
                if !stdout.contains("jarvis-gems") {
                    eprintln!("Search/QMD: jarvis-gems collection NOT found in status");
                    return AvailabilityResult {
                        available: false,
                        reason: Some(
                            "QMD jarvis-gems collection not found. Run setup first.".to_string()
                        ),
                    };
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("Search/QMD: qmd status failed (exit {}): {}", output.status, stderr.trim());
                return AvailabilityResult {
                    available: false,
                    reason: Some("Failed to check QMD status".to_string()),
                };
            }
            Err(e) => {
                eprintln!("Search/QMD: qmd status error: {}", e);
                return AvailabilityResult {
                    available: false,
                    reason: Some("Failed to check QMD status".to_string()),
                };
            }
        }

        eprintln!("Search/QMD: Available and ready");
        AvailabilityResult {
            available: true,
            reason: None,
        }
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
        // Cap query length — QMD's reranker crashes on very long inputs
        // (context size exceeded error). 200 chars is plenty for any real search.
        let query = if query.len() > 200 {
            eprintln!("Search/QMD: Query too long ({} chars), truncating to 200", query.len());
            &query[..200]
        } else {
            query
        };

        eprintln!("Search/QMD: query=\"{}\" limit={}", query, limit);
        // Request more results than limit since QMD returns multiple chunks per gem
        // and we deduplicate by gem_id (keeping highest score)
        let fetch_limit = limit * 3;
        // Run: qmd query "{query}" --json -n {fetch_limit}
        let output = Command::new(&self.qmd_path)
            .args([
                "query",
                query,
                "--json",
                "-n",
                &fetch_limit.to_string(),
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to run qmd query: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("Search/QMD: qmd query FAILED: {}", stderr.trim());
            // Return empty results instead of error — graceful degradation
            // QMD can fail on unusual inputs (long queries, special chars)
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        eprintln!("Search/QMD: qmd query raw output ({} bytes): {}", stdout.len(), &stdout[..stdout.len().min(500)]);

        // QMD outputs ANSI escape codes and progress text before JSON.
        // Find the JSON array start '[' and parse from there.
        let json_start = stdout.find('[');
        let json_end = stdout.rfind(']');
        let json_str = match (json_start, json_end) {
            (Some(start), Some(end)) if end >= start => &stdout[start..=end],
            _ => {
                eprintln!("Search/QMD: No JSON array found in output");
                return Ok(Vec::new());
            }
        };

        // Parse QMD JSON output
        // Actual shape: [{ "docid": "...", "score": 0.89, "file": "qmd://jarvis-gems/{gem_id}/enrichment.md", "title": "...", "snippet": "..." }, ...]
        let qmd_results: Vec<serde_json::Value> = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse QMD JSON output: {}", e))?;

        eprintln!("Search/QMD: Parsed {} raw results", qmd_results.len());

        // Deduplicate by gem_id — QMD returns multiple chunks per gem (gem.md, enrichment.md, transcript.md).
        // Keep the highest-scoring entry for each gem.
        let mut best_by_gem: std::collections::HashMap<String, (f64, String)> = std::collections::HashMap::new();

        for item in qmd_results {
            // Extract file URI and derive gem_id
            // Format: "qmd://jarvis-gems/{gem_id}/enrichment.md"
            let file_uri = item
                .get("file")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            let gem_id = extract_gem_id_from_uri(file_uri);

            if let Some(gem_id) = gem_id {
                let raw_score = item
                    .get("score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);

                // QMD uses "snippet" not "chunk"
                let snippet = item
                    .get("snippet")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                eprintln!("Search/QMD: Result gem_id={} score={:.3} file={}", gem_id, raw_score, file_uri);

                let entry = best_by_gem.entry(gem_id).or_insert((0.0, String::new()));
                if raw_score > entry.0 {
                    *entry = (raw_score, snippet);
                }
            } else {
                eprintln!("Search/QMD: Could not extract gem_id from URI: {}", file_uri);
            }
        }

        // Convert to SearchResult vec, filter low-confidence results, sort by score descending
        let min_score = self.min_score;
        let mut results: Vec<SearchResult> = best_by_gem
            .into_iter()
            .map(|(gem_id, (score, snippet))| SearchResult {
                gem_id,
                score: normalize_qmd_score(score),
                matched_chunk: snippet,
                match_type: MatchType::Hybrid,
            })
            .filter(|r| r.score >= min_score)
            .collect();
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        eprintln!("Search/QMD: Returning {} results (>={:.0}%) for query \"{}\"", results.len(), min_score * 100.0, query);
        Ok(results)
    }

    async fn index_gem(&self, _gem_id: &str) -> Result<(), String> {
        // Fire-and-forget: spawn qmd update && qmd embed
        // QMD detects changed files and re-indexes only those
        let gem_id = _gem_id.to_string();
        let qmd_path = self.qmd_path.clone();
        tokio::spawn(async move {
            eprintln!("Search/QMD: Indexing gem {} — running qmd update", gem_id);
            let update = Command::new(&qmd_path)
                .arg("update")
                .output()
                .await;

            match &update {
                Ok(output) if output.status.success() => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    eprintln!("Search/QMD: qmd update succeeded for gem {}: {}", gem_id, stdout.trim());
                    eprintln!("Search/QMD: Running qmd embed for gem {}", gem_id);
                    let embed = Command::new(&qmd_path)
                        .arg("embed")
                        .output()
                        .await;
                    match &embed {
                        Ok(out) if out.status.success() => {
                            let stdout = String::from_utf8_lossy(&out.stdout);
                            eprintln!("Search/QMD: qmd embed succeeded for gem {}: {}", gem_id, stdout.trim());
                        }
                        Ok(out) => {
                            let stderr = String::from_utf8_lossy(&out.stderr);
                            eprintln!("Search/QMD: qmd embed FAILED for gem {}: {}", gem_id, stderr.trim());
                        }
                        Err(e) => {
                            eprintln!("Search/QMD: qmd embed ERROR for gem {}: {}", gem_id, e);
                        }
                    }
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    eprintln!("Search/QMD: qmd update FAILED for gem {}: {}", gem_id, stderr.trim());
                }
                Err(e) => {
                    eprintln!("Search/QMD: qmd update ERROR for gem {}: {}", gem_id, e);
                }
            }
        });

        Ok(())
    }

    async fn remove_gem(&self, _gem_id: &str) -> Result<(), String> {
        // Fire-and-forget: spawn qmd update
        // QMD detects deleted files
        let gem_id = _gem_id.to_string();
        let qmd_path = self.qmd_path.clone();
        tokio::spawn(async move {
            eprintln!("Search/QMD: Removing gem {} — running qmd update", gem_id);
            let update = Command::new(&qmd_path)
                .arg("update")
                .output()
                .await;
            match &update {
                Ok(output) if output.status.success() => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    eprintln!("Search/QMD: qmd update (remove) succeeded for gem {}: {}", gem_id, stdout.trim());
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    eprintln!("Search/QMD: qmd update (remove) FAILED for gem {}: {}", gem_id, stderr.trim());
                }
                Err(e) => {
                    eprintln!("Search/QMD: qmd update (remove) ERROR for gem {}: {}", gem_id, e);
                }
            }
        });

        Ok(())
    }

    async fn reindex_all(&self) -> Result<usize, String> {
        eprintln!("Search/QMD: reindex_all — running qmd update");
        // Await: qmd update && qmd embed -f (force re-embed all)
        let update = Command::new(&self.qmd_path)
            .arg("update")
            .output()
            .await
            .map_err(|e| format!("qmd update failed: {}", e))?;

        if !update.status.success() {
            let stderr = String::from_utf8_lossy(&update.stderr);
            eprintln!("Search/QMD: reindex_all qmd update FAILED: {}", stderr.trim());
            return Err(format!("qmd update failed: {}", stderr));
        }
        let stdout = String::from_utf8_lossy(&update.stdout);
        eprintln!("Search/QMD: reindex_all qmd update succeeded: {}", stdout.trim());

        eprintln!("Search/QMD: reindex_all — running qmd embed -f");
        let embed = Command::new(&self.qmd_path)
            .args(["embed", "-f"])
            .output()
            .await
            .map_err(|e| format!("qmd embed failed: {}", e))?;

        if !embed.status.success() {
            let stderr = String::from_utf8_lossy(&embed.stderr);
            eprintln!("Search/QMD: reindex_all qmd embed FAILED: {}", stderr.trim());
            return Err(format!("qmd embed failed: {}", stderr));
        }
        let stdout = String::from_utf8_lossy(&embed.stdout);
        eprintln!("Search/QMD: reindex_all qmd embed succeeded: {}", stdout.trim());

        // TODO: Parse output for indexed document count
        // For now return 0 — refine after testing actual QMD CLI output
        Ok(0)
    }
}

/// Extract gem_id from a QMD result file URI.
///
/// QMD returns URIs like: "qmd://jarvis-gems/{gem_id}/enrichment.md"
/// We extract the gem_id (UUID) segment.
fn extract_gem_id_from_uri(file_uri: &str) -> Option<String> {
    // Strip the "qmd://jarvis-gems/" prefix
    if let Some(rest) = file_uri.strip_prefix("qmd://jarvis-gems/") {
        // rest = "{gem_id}/enrichment.md" — take first path segment
        return rest.split('/').next()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
    }

    // Fallback: try treating as filesystem path "{gem_id}/gem.md"
    let path = std::path::Path::new(file_uri);
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
