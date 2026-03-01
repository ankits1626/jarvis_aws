// TavilyProvider - web search via Tavily Search API

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::intelligence::AvailabilityResult;
use super::provider::{
    SearchResultProvider, SearchResult, WebSearchResult, WebSourceType,
};

/// Web search provider backed by the Tavily Search API.
///
/// Implements SearchResultProvider::web_search. All gem-related methods
/// (search, index_gem, remove_gem, reindex_all) are no-ops.
pub struct TavilyProvider {
    api_key: String,
    client: Client,
}

impl TavilyProvider {
    pub fn new(api_key: String) -> Self {
        eprintln!("Search/Tavily: Initialized with API key ({}...)", &api_key[..8.min(api_key.len())]);
        Self {
            api_key,
            client: Client::new(),
        }
    }
}

// ── Tavily API request/response shapes ──

#[derive(Serialize)]
struct TavilySearchRequest {
    query: String,
    max_results: usize,
    search_depth: String,
    api_key: String,
}

#[derive(Deserialize)]
struct TavilySearchResponse {
    results: Vec<TavilyResult>,
}

#[derive(Deserialize)]
struct TavilyResult {
    title: String,
    url: String,
    content: String,
    #[serde(default)]
    published_date: Option<String>,
}

// ── Domain classification ──

/// Classify a URL's domain into a WebSourceType.
fn classify_source_type(url: &str) -> WebSourceType {
    let url_lower = url.to_lowercase();
    if url_lower.contains("youtube.com") || url_lower.contains("youtu.be") || url_lower.contains("vimeo.com") {
        WebSourceType::Video
    } else if url_lower.contains("arxiv.org") || url_lower.contains("scholar.google") || url_lower.contains("semanticscholar.org") || url_lower.contains("ieee.org") || url_lower.contains("acm.org") {
        WebSourceType::Paper
    } else if url_lower.contains("medium.com") || url_lower.contains("dev.to") || url_lower.contains("substack.com") || url_lower.contains("hashnode") || url_lower.contains("blog") {
        WebSourceType::Article
    } else {
        WebSourceType::Other
    }
}

/// Extract domain from a URL (e.g., "https://medium.com/foo" -> "medium.com").
fn extract_domain(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or(url)
        .trim_start_matches("www.")
        .to_string()
}

#[async_trait]
impl SearchResultProvider for TavilyProvider {
    async fn check_availability(&self) -> AvailabilityResult {
        if self.api_key.is_empty() {
            return AvailabilityResult {
                available: false,
                reason: Some("Tavily API key is empty".to_string()),
            };
        }
        AvailabilityResult {
            available: true,
            reason: None,
        }
    }

    // Gem search — not applicable for Tavily
    async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<SearchResult>, String> {
        Ok(Vec::new())
    }

    async fn index_gem(&self, _gem_id: &str) -> Result<(), String> {
        Ok(())
    }

    async fn remove_gem(&self, _gem_id: &str) -> Result<(), String> {
        Ok(())
    }

    async fn reindex_all(&self) -> Result<usize, String> {
        Ok(0)
    }

    // Web search — the real implementation
    async fn web_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<WebSearchResult>, String> {
        eprintln!("Search/Tavily: web_search query=\"{}\" limit={}", query, limit);

        let request = TavilySearchRequest {
            query: query.to_string(),
            max_results: limit,
            search_depth: "basic".to_string(),
            api_key: self.api_key.clone(),
        };

        let response = self.client
            .post("https://api.tavily.com/search")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Tavily API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            eprintln!("Search/Tavily: API error {} — {}", status, body);
            return Err(format!("Tavily API returned {}: {}", status, body));
        }

        let tavily_response: TavilySearchResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Tavily response: {}", e))?;

        let results: Vec<WebSearchResult> = tavily_response
            .results
            .into_iter()
            .map(|r| {
                let source_type = classify_source_type(&r.url);
                let domain = extract_domain(&r.url);
                WebSearchResult {
                    title: r.title,
                    url: r.url,
                    snippet: r.content,
                    source_type,
                    domain,
                    published_date: r.published_date,
                }
            })
            .collect();

        eprintln!("Search/Tavily: Returning {} results for \"{}\"", results.len(), query);
        Ok(results)
    }

    fn supports_web_search(&self) -> bool {
        true
    }
}
