// Extractor router — dispatches to the right extractor based on SourceType

pub mod chatgpt;
pub mod claude_extension;
pub mod generic;
pub mod gmail;
pub mod medium;

use super::tabs::SourceType;
use super::youtube::scrape_youtube_gist;
use serde::{Deserialize, Serialize};

/// Unified gist type returned by all extractors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageGist {
    pub url: String,
    pub title: String,
    pub source_type: SourceType,
    pub domain: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub content_excerpt: Option<String>,
    pub published_date: Option<String>,
    pub image_url: Option<String>,
    /// Source-specific extra fields (e.g. duration for YouTube)
    pub extra: serde_json::Value,
}

/// Route URL to the right extractor and produce a gist
pub async fn prepare_gist(url: &str, source_type: &SourceType) -> Result<PageGist, String> {
    let domain = super::tabs::extract_domain(url);

    match source_type {
        SourceType::YouTube => youtube_gist(url, &domain).await,
        SourceType::Email => gmail::extract(url, source_type, &domain).await,
        SourceType::Chat => chatgpt::extract(url, source_type, &domain).await,
        _ if domain.contains("medium.com") => medium::extract(url, source_type, &domain).await,
        _ => generic::extract(url, source_type, &domain).await,
    }
}

/// Wrap existing YouTube scraper into PageGist format
async fn youtube_gist(url: &str, domain: &str) -> Result<PageGist, String> {
    let yt = scrape_youtube_gist(url).await?;

    Ok(PageGist {
        url: yt.url,
        title: yt.title,
        source_type: SourceType::YouTube,
        domain: domain.to_string(),
        author: Some(yt.channel.clone()),
        description: Some(yt.description),
        content_excerpt: None,
        published_date: None,
        image_url: None,
        extra: serde_json::json!({
            "video_id": yt.video_id,
            "channel": yt.channel,
            "duration_seconds": yt.duration_seconds,
        }),
    })
}
