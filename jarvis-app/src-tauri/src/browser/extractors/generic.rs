// Generic page extractor — fetches HTML and extracts OG metadata + content excerpt

use super::PageGist;
use crate::browser::tabs::SourceType;
use regex::Regex;
use std::sync::LazyLock;
use std::time::Duration;

// Module-level regex patterns for content extraction
pub(crate) static ARTICLE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?is)<article[^>]*>(.*?)</article>").unwrap()
});

pub(crate) static TAG_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<[^>]+>").unwrap()
});

/// Extract a gist from any web page using OG metadata and basic content extraction
pub async fn extract(url: &str, source_type: &SourceType, domain: &str) -> Result<PageGist, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let html = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch page: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    let title = extract_og_content(&html, "og:title")
        .or_else(|| extract_meta_content(&html, "title"))
        .or_else(|| extract_html_title(&html))
        .unwrap_or_else(|| "Unknown".to_string());

    let description = extract_og_content(&html, "og:description")
        .or_else(|| extract_meta_content(&html, "description"));

    let author = extract_meta_content(&html, "author")
        .or_else(|| extract_meta_content(&html, "article:author"));

    let published_date = extract_meta_content(&html, "article:published_time")
        .or_else(|| extract_meta_content(&html, "publishedDate"))
        .or_else(|| extract_meta_content(&html, "date"));

    let image_url = extract_og_content(&html, "og:image");

    let content_excerpt = extract_content_excerpt(&html);

    Ok(PageGist {
        url: url.to_string(),
        title,
        source_type: source_type.clone(),
        domain: domain.to_string(),
        author,
        description,
        content_excerpt,
        published_date,
        image_url,
        extra: serde_json::Value::Null,
    })
}

/// Extract content from <meta property="og:*" content="...">
pub(crate) fn extract_og_content(html: &str, property: &str) -> Option<String> {
    static OG_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<meta\s+(?:[^>]*?\s)?property="([^"]+)"\s+(?:[^>]*?\s)?content="([^"]*)"[^>]*>"#).unwrap()
    });
    // Also try reversed attribute order
    static OG_REGEX_REV: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<meta\s+(?:[^>]*?\s)?content="([^"]*)"\s+(?:[^>]*?\s)?property="([^"]+)"[^>]*>"#).unwrap()
    });

    for cap in OG_REGEX.captures_iter(html) {
        if cap.get(1).map_or("", |m| m.as_str()) == property {
            let content = cap.get(2).map_or("", |m| m.as_str()).to_string();
            if !content.is_empty() {
                return Some(decode_html_entities(&content));
            }
        }
    }

    for cap in OG_REGEX_REV.captures_iter(html) {
        if cap.get(2).map_or("", |m| m.as_str()) == property {
            let content = cap.get(1).map_or("", |m| m.as_str()).to_string();
            if !content.is_empty() {
                return Some(decode_html_entities(&content));
            }
        }
    }

    None
}

/// Extract content from <meta name="..." content="...">
pub(crate) fn extract_meta_content(html: &str, name: &str) -> Option<String> {
    static META_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<meta\s+(?:[^>]*?\s)?name="([^"]+)"\s+(?:[^>]*?\s)?content="([^"]*)"[^>]*>"#).unwrap()
    });
    static META_REGEX_REV: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<meta\s+(?:[^>]*?\s)?content="([^"]*)"\s+(?:[^>]*?\s)?name="([^"]+)"[^>]*>"#).unwrap()
    });

    for cap in META_REGEX.captures_iter(html) {
        if cap.get(1).map_or("", |m| m.as_str()) == name {
            let content = cap.get(2).map_or("", |m| m.as_str()).to_string();
            if !content.is_empty() {
                return Some(decode_html_entities(&content));
            }
        }
    }

    for cap in META_REGEX_REV.captures_iter(html) {
        if cap.get(2).map_or("", |m| m.as_str()) == name {
            let content = cap.get(1).map_or("", |m| m.as_str()).to_string();
            if !content.is_empty() {
                return Some(decode_html_entities(&content));
            }
        }
    }

    None
}

/// Extract title from <title>...</title>
pub(crate) fn extract_html_title(html: &str) -> Option<String> {
    static TITLE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"<title[^>]*>([^<]+)</title>").unwrap()
    });

    TITLE_REGEX
        .captures(html)
        .and_then(|caps| caps.get(1))
        .map(|m| decode_html_entities(m.as_str().trim()))
}

/// Extract a content excerpt from the page body.
/// Tries <article>, <main>, then falls back to <p> tags.
fn extract_content_excerpt(html: &str) -> Option<String> {
    // Try to find content in <article> or <main> tags
    static MAIN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?is)<main[^>]*>(.*?)</main>").unwrap()
    });

    let body_html = ARTICLE_REGEX
        .captures(html)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .or_else(|| {
            MAIN_REGEX
                .captures(html)
                .and_then(|caps| caps.get(1))
                .map(|m| m.as_str().to_string())
        });

    let source = body_html.as_deref().unwrap_or(html);

    // Extract text from <p> tags
    static P_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?is)<p[^>]*>(.*?)</p>").unwrap()
    });

    let mut text_parts: Vec<String> = Vec::new();
    let mut total_len = 0;

    for cap in P_REGEX.captures_iter(source) {
        if let Some(content) = cap.get(1) {
            let text = TAG_REGEX.replace_all(content.as_str(), "").to_string();
            let text = decode_html_entities(text.trim());
            if text.len() > 20 {
                // Skip very short paragraphs (likely nav items)
                total_len += text.len();
                text_parts.push(text);
                if total_len > 500 {
                    break;
                }
            }
        }
    }

    if text_parts.is_empty() {
        return None;
    }

    let mut excerpt = text_parts.join(" ");
    if excerpt.len() > 500 {
        // Truncate at word boundary
        if let Some(pos) = excerpt[..500].rfind(' ') {
            excerpt.truncate(pos);
        } else {
            excerpt.truncate(500);
        }
        excerpt.push_str("...");
    }

    Some(excerpt)
}

/// Decode common HTML entities
pub(crate) fn decode_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_og_title() {
        let html = r#"<html><head><meta property="og:title" content="Test Page"></head></html>"#;
        assert_eq!(extract_og_content(html, "og:title"), Some("Test Page".to_string()));
    }

    #[test]
    fn test_extract_og_title_reversed() {
        let html = r#"<html><head><meta content="Test Page" property="og:title"></head></html>"#;
        assert_eq!(extract_og_content(html, "og:title"), Some("Test Page".to_string()));
    }

    #[test]
    fn test_extract_og_missing() {
        let html = r#"<html><head><title>Test</title></head></html>"#;
        assert_eq!(extract_og_content(html, "og:title"), None);
    }

    #[test]
    fn test_extract_meta_author() {
        let html = r#"<html><head><meta name="author" content="Jane Doe"></head></html>"#;
        assert_eq!(extract_meta_content(html, "author"), Some("Jane Doe".to_string()));
    }

    #[test]
    fn test_extract_html_title() {
        let html = "<html><head><title>My Page Title</title></head></html>";
        assert_eq!(extract_html_title(html), Some("My Page Title".to_string()));
    }

    #[test]
    fn test_extract_content_excerpt() {
        let html = r#"<html><body><article><p>This is a paragraph with enough text to be meaningful and extracted by the content extractor.</p><p>Another paragraph that adds more context to the article content for the gist.</p></article></body></html>"#;
        let excerpt = extract_content_excerpt(html);
        assert!(excerpt.is_some());
        assert!(excerpt.unwrap().contains("This is a paragraph"));
    }

    #[test]
    fn test_decode_html_entities() {
        assert_eq!(decode_html_entities("Hello &amp; World"), "Hello & World");
        assert_eq!(decode_html_entities("&lt;tag&gt;"), "<tag>");
        assert_eq!(decode_html_entities("It&#39;s fine"), "It's fine");
    }

    #[test]
    fn test_extract_content_skips_short_paragraphs() {
        let html = r#"<html><body><p>Short</p><p>This is a longer paragraph that should be extracted because it has enough meaningful content.</p></body></html>"#;
        let excerpt = extract_content_excerpt(html);
        assert!(excerpt.is_some());
        let text = excerpt.unwrap();
        assert!(!text.contains("Short"));
        assert!(text.contains("longer paragraph"));
    }
}
