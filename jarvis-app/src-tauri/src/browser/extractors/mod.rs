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

/// Merge a page/article gist with a Claude conversation gist into a single gist.
/// The page gist provides primary metadata (url, title, author, dates).
/// The Claude conversation is appended to the content.
pub fn merge_gists(page_gist: PageGist, claude_gist: PageGist) -> PageGist {
    let mut content_parts: Vec<String> = Vec::new();

    if let Some(ref excerpt) = page_gist.content_excerpt {
        if !excerpt.trim().is_empty() {
            content_parts.push(format!("--- Page Content ---\n{}", excerpt));
        }
    }

    if let Some(ref conversation) = claude_gist.content_excerpt {
        if !conversation.trim().is_empty() {
            content_parts.push(format!("--- Claude Conversation ---\n{}", conversation));
        }
    }

    let merged_content = if content_parts.is_empty() {
        None
    } else {
        Some(content_parts.join("\n\n"))
    };

    // Nest extras under page/claude keys to avoid collisions
    let mut merged_extra = serde_json::Map::new();
    if !page_gist.extra.is_null() {
        merged_extra.insert("page".to_string(), page_gist.extra);
    }
    if !claude_gist.extra.is_null() {
        merged_extra.insert("claude".to_string(), claude_gist.extra);
    }
    merged_extra.insert("has_claude_conversation".to_string(), serde_json::Value::Bool(true));

    PageGist {
        url: page_gist.url,
        title: page_gist.title,
        source_type: page_gist.source_type,
        domain: page_gist.domain,
        author: page_gist.author,
        description: page_gist.description.or(claude_gist.description),
        published_date: page_gist.published_date,
        image_url: page_gist.image_url,
        content_excerpt: merged_content,
        extra: serde_json::Value::Object(merged_extra),
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
