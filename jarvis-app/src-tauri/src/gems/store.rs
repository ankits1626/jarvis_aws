use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Backend-agnostic gem representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gem {
    /// Unique identifier (UUID v4)
    pub id: String,
    
    /// Source classification (YouTube, Article, Email, Chat, etc.)
    pub source_type: String,
    
    /// Original URL (unique constraint)
    pub source_url: String,
    
    /// Domain extracted from URL (e.g., "youtube.com", "medium.com")
    pub domain: String,
    
    /// Page/video/article title
    pub title: String,
    
    /// Author/channel name (optional)
    pub author: Option<String>,
    
    /// Short description or summary (optional)
    pub description: Option<String>,
    
    /// Full extracted content (optional)
    pub content: Option<String>,
    
    /// Source-specific metadata (JSON, e.g., video duration, email thread ID)
    pub source_meta: serde_json::Value,
    
    /// ISO 8601 timestamp when gem was captured
    pub captured_at: String,
    
    /// AI-generated enrichment metadata (JSON blob)
    /// Structure: {"tags": ["tag1", ...], "summary": "...", "provider": "intelligencekit", "enriched_at": "ISO 8601"}
    /// NULL when no enrichment has been applied
    pub ai_enrichment: Option<serde_json::Value>,
}

/// Lightweight gem for list/search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GemPreview {
    pub id: String,
    pub source_type: String,
    pub source_url: String,
    pub domain: String,
    pub title: String,
    pub author: Option<String>,
    pub description: Option<String>,
    
    /// Content truncated to 200 characters
    pub content_preview: Option<String>,
    
    pub captured_at: String,
    
    /// AI-generated topic tags (extracted from ai_enrichment.tags)
    pub tags: Option<Vec<String>>,
    
    /// AI-generated summary (extracted from ai_enrichment.summary)
    pub summary: Option<String>,
}

/// Storage interface for gems - implementations are swappable
#[async_trait]
pub trait GemStore: Send + Sync {
    /// Save or update a gem (upsert by source_url)
    async fn save(&self, gem: Gem) -> Result<Gem, String>;
    
    /// Get a gem by ID
    async fn get(&self, id: &str) -> Result<Option<Gem>, String>;
    
    /// List gems with pagination (ordered by captured_at DESC)
    async fn list(&self, limit: usize, offset: usize) -> Result<Vec<GemPreview>, String>;
    
    /// Search gems by keyword (FTS on title, description, content)
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<GemPreview>, String>;
    
    /// Filter gems by tag (exact match on ai_enrichment.tags array)
    async fn filter_by_tag(&self, tag: &str, limit: usize, offset: usize) -> Result<Vec<GemPreview>, String>;
    
    /// Delete a gem by ID
    async fn delete(&self, id: &str) -> Result<(), String>;
}
