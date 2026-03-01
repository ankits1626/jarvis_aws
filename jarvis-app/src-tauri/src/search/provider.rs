// SearchResultProvider trait - backend-agnostic search interface

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

/// Classification of a web search result by content type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebSourceType {
    /// Academic papers (arxiv, scholar, semantic scholar)
    Paper,
    /// Blog posts and articles (medium, dev.to, substack)
    Article,
    /// Video content (youtube, vimeo)
    Video,
    /// Everything else
    Other,
}

/// A search result from the web (not a gem).
///
/// Returned by SearchResultProvider::web_search for providers that support it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub source_type: WebSourceType,
    pub domain: String,
    pub published_date: Option<String>,
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

    /// Search the web for external resources (papers, articles, videos).
    ///
    /// Default: returns empty vec (provider does not support web search).
    /// Override in providers that have web search capability (e.g., Tavily).
    async fn web_search(
        &self,
        _query: &str,
        _limit: usize,
    ) -> Result<Vec<WebSearchResult>, String> {
        Ok(Vec::new())
    }

    /// Check if this provider supports web search.
    ///
    /// Default: false. Override in web-capable providers.
    fn supports_web_search(&self) -> bool {
        false
    }
}
