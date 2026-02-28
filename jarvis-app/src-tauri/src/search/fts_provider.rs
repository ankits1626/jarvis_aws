// FtsResultProvider - default search provider wrapping SQLite FTS5

use std::sync::Arc;
use async_trait::async_trait;
use crate::gems::GemStore;
use crate::intelligence::AvailabilityResult;
use super::provider::{SearchResultProvider, SearchResult, MatchType};

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

#[async_trait]
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
        Ok(0)
    }
}
