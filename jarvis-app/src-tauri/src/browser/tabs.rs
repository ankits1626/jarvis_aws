// Tab listing and URL classification

use super::adapters::chrome::ChromeAppleScriptAdapter;
use super::adapters::BrowserAdapter;
use serde::{Deserialize, Serialize};

/// Source type classification based on URL domain
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SourceType {
    YouTube,
    Article,
    Code,
    Docs,
    Email,
    Chat,
    QA,
    News,
    Research,
    Social,
    Other,
}

/// Enriched browser tab with classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTab {
    pub url: String,
    pub title: String,
    pub source_type: SourceType,
    pub domain: String,
}

/// Extract domain from a URL (e.g. "https://www.github.com/repo" -> "github.com")
pub fn extract_domain(url: &str) -> String {
    url.split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .strip_prefix("www.")
        .unwrap_or(
            url.split("://")
                .nth(1)
                .unwrap_or(url)
                .split('/')
                .next()
                .unwrap_or("")
                .split(':')
                .next()
                .unwrap_or(""),
        )
        .to_string()
}

/// Classify a URL into a SourceType based on its domain
pub fn classify_url(url: &str) -> SourceType {
    let domain = extract_domain(url);
    let domain = domain.as_str();

    // YouTube
    if domain.contains("youtube.com") || domain.contains("youtu.be") {
        return SourceType::YouTube;
    }

    // Email
    if domain.contains("mail.google.com") {
        return SourceType::Email;
    }

    // Chat (AI conversations)
    if domain.contains("chatgpt.com") || domain.contains("chat.openai.com") {
        return SourceType::Chat;
    }

    // Code hosting
    if domain.contains("github.com")
        || domain.contains("gitlab.com")
        || domain.contains("bitbucket.org")
        || domain.contains("codeberg.org")
    {
        return SourceType::Code;
    }

    // Q&A
    if domain.contains("stackoverflow.com")
        || domain.contains("stackexchange.com")
        || domain.contains("superuser.com")
        || domain.contains("serverfault.com")
        || domain.contains("askubuntu.com")
    {
        return SourceType::QA;
    }

    // Documentation
    if domain.contains("docs.rs")
        || domain.contains("doc.rust-lang.org")
        || domain.contains("developer.mozilla.org")
        || domain.contains("developer.apple.com")
        || domain.contains("learn.microsoft.com")
        || domain.contains("readthedocs.io")
        || domain.contains("docs.python.org")
        || domain.contains("docs.google.com")
    {
        return SourceType::Docs;
    }

    // Research / Academic
    if domain.contains("arxiv.org")
        || domain.contains("scholar.google.com")
        || domain.contains("semanticscholar.org")
        || domain.contains("researchgate.net")
        || domain.contains("pubmed.ncbi.nlm.nih.gov")
    {
        return SourceType::Research;
    }

    // News / Forums
    if domain.contains("news.ycombinator.com")
        || domain.contains("reddit.com")
        || domain.contains("lobste.rs")
        || domain.contains("slashdot.org")
    {
        return SourceType::News;
    }

    // Articles / Blogs
    if domain.contains("medium.com")
        || domain.contains("substack.com")
        || domain.contains("dev.to")
        || domain.contains("hashnode.dev")
        || domain.contains("blog.")
        || domain.contains("wordpress.com")
    {
        return SourceType::Article;
    }

    // Social
    if domain.contains("twitter.com")
        || domain.contains("x.com")
        || domain.contains("linkedin.com")
        || domain.contains("mastodon")
        || domain.contains("threads.net")
    {
        return SourceType::Social;
    }

    SourceType::Other
}

/// List all open browser tabs with classification.
/// Currently uses Chrome AppleScript adapter (macOS only).
pub async fn list_all_tabs() -> Result<Vec<BrowserTab>, String> {
    let adapter = ChromeAppleScriptAdapter;

    if !adapter.is_available() {
        return Err("Chrome is not running".to_string());
    }

    let raw_tabs = adapter.list_tabs().await?;

    let tabs = raw_tabs
        .into_iter()
        .map(|raw| {
            let source_type = classify_url(&raw.url);
            let domain = extract_domain(&raw.url);
            BrowserTab {
                url: raw.url,
                title: raw.title,
                source_type,
                domain,
            }
        })
        .collect();

    Ok(tabs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain_https() {
        assert_eq!(extract_domain("https://www.github.com/repo"), "github.com");
    }

    #[test]
    fn test_extract_domain_http() {
        assert_eq!(extract_domain("http://example.com/page"), "example.com");
    }

    #[test]
    fn test_extract_domain_with_port() {
        assert_eq!(
            extract_domain("http://localhost:3000/api"),
            "localhost"
        );
    }

    #[test]
    fn test_extract_domain_no_www() {
        assert_eq!(
            extract_domain("https://github.com/user/repo"),
            "github.com"
        );
    }

    #[test]
    fn test_classify_youtube() {
        assert_eq!(
            classify_url("https://www.youtube.com/watch?v=abc"),
            SourceType::YouTube
        );
        assert_eq!(
            classify_url("https://youtu.be/abc"),
            SourceType::YouTube
        );
    }

    #[test]
    fn test_classify_code() {
        assert_eq!(
            classify_url("https://github.com/user/repo"),
            SourceType::Code
        );
        assert_eq!(
            classify_url("https://gitlab.com/user/repo"),
            SourceType::Code
        );
    }

    #[test]
    fn test_classify_qa() {
        assert_eq!(
            classify_url("https://stackoverflow.com/questions/123"),
            SourceType::QA
        );
    }

    #[test]
    fn test_classify_docs() {
        assert_eq!(
            classify_url("https://docs.rs/reqwest/latest/reqwest/"),
            SourceType::Docs
        );
        assert_eq!(
            classify_url("https://developer.mozilla.org/en-US/docs/Web"),
            SourceType::Docs
        );
    }

    #[test]
    fn test_classify_research() {
        assert_eq!(
            classify_url("https://arxiv.org/abs/2301.01234"),
            SourceType::Research
        );
    }

    #[test]
    fn test_classify_news() {
        assert_eq!(
            classify_url("https://news.ycombinator.com/item?id=123"),
            SourceType::News
        );
        assert_eq!(
            classify_url("https://www.reddit.com/r/rust/"),
            SourceType::News
        );
    }

    #[test]
    fn test_classify_article() {
        assert_eq!(
            classify_url("https://medium.com/@user/article"),
            SourceType::Article
        );
        assert_eq!(
            classify_url("https://dev.to/user/article"),
            SourceType::Article
        );
    }

    #[test]
    fn test_classify_social() {
        assert_eq!(
            classify_url("https://twitter.com/user/status/123"),
            SourceType::Social
        );
        assert_eq!(
            classify_url("https://x.com/user/status/123"),
            SourceType::Social
        );
    }

    #[test]
    fn test_classify_chat() {
        assert_eq!(
            classify_url("https://chatgpt.com/c/abc123"),
            SourceType::Chat
        );
        assert_eq!(
            classify_url("https://chat.openai.com/c/abc123"),
            SourceType::Chat
        );
    }

    #[test]
    fn test_classify_email() {
        assert_eq!(
            classify_url("https://mail.google.com/mail/u/0/#inbox/FMfcgzQXJWDRS"),
            SourceType::Email
        );
    }

    #[test]
    fn test_classify_other() {
        assert_eq!(
            classify_url("https://www.google.com/search?q=rust"),
            SourceType::Other
        );
    }
}
