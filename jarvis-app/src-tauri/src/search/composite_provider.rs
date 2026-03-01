// CompositeSearchProvider - delegates gem search and web search to separate providers

use std::sync::Arc;
use async_trait::async_trait;
use crate::intelligence::AvailabilityResult;
use super::provider::{
    SearchResultProvider, SearchResult, WebSearchResult,
};

/// Composite search provider that delegates gem search to one provider
/// and web search to another.
///
/// Registered as the single Arc<dyn SearchResultProvider> in Tauri state.
/// All existing commands (search_gems, check_search_availability, etc.)
/// work unchanged — they call .search() which delegates to gem_provider.
/// New research commands call .web_search() which delegates to web_provider.
pub struct CompositeSearchProvider {
    gem_provider: Arc<dyn SearchResultProvider>,
    web_provider: Option<Arc<dyn SearchResultProvider>>,
}

impl CompositeSearchProvider {
    pub fn new(
        gem_provider: Arc<dyn SearchResultProvider>,
        web_provider: Option<Arc<dyn SearchResultProvider>>,
    ) -> Self {
        let web_status = if web_provider.is_some() { "enabled" } else { "disabled" };
        eprintln!("Search/Composite: Initialized (web search: {})", web_status);
        Self {
            gem_provider,
            web_provider,
        }
    }
}

#[async_trait]
impl SearchResultProvider for CompositeSearchProvider {
    async fn check_availability(&self) -> AvailabilityResult {
        self.gem_provider.check_availability().await
    }

    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, String> {
        self.gem_provider.search(query, limit).await
    }

    async fn index_gem(&self, gem_id: &str) -> Result<(), String> {
        self.gem_provider.index_gem(gem_id).await
    }

    async fn remove_gem(&self, gem_id: &str) -> Result<(), String> {
        self.gem_provider.remove_gem(gem_id).await
    }

    async fn reindex_all(&self) -> Result<usize, String> {
        self.gem_provider.reindex_all().await
    }

    async fn web_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<WebSearchResult>, String> {
        match &self.web_provider {
            Some(wp) => wp.web_search(query, limit).await,
            None => Ok(Vec::new()),
        }
    }

    fn supports_web_search(&self) -> bool {
        self.web_provider
            .as_ref()
            .map_or(false, |wp| wp.supports_web_search())
    }
}
