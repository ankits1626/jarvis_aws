// Medium article extractor — uses DOM extraction via Chrome adapter
// Extracts metadata + full article text directly from the browser DOM,
// bypassing paywalls and getting authenticated content.

use super::PageGist;
use crate::browser::adapters::chrome::ChromeAppleScriptAdapter;
use crate::browser::adapters::BrowserAdapter;
use crate::browser::tabs::SourceType;
use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

/// Regex pattern for extracting reading time from Medium articles
static READING_TIME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\d+)\s*min(?:ute)?s?\s*read").unwrap()
});

/// Extract reading time from text (e.g. "8 min read")
fn extract_reading_time(text: &str) -> Option<u32> {
    READING_TIME_REGEX
        .captures(text)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
}

/// Data extracted from Medium DOM via JavaScript
#[derive(Deserialize)]
struct MediumDomData {
    title: Option<String>,
    description: Option<String>,
    image: Option<String>,
    publication: Option<String>,
    author: Option<String>,
    published_date: Option<String>,
    article_text: Option<String>,
}

/// JavaScript that extracts all Medium metadata + article text in one call.
/// Uses DOM APIs only (no regex), avoiding AppleScript escaping issues.
const EXTRACT_JS: &str = r#"(function(){
  var metas = document.getElementsByTagName('meta');
  var d = {};
  for (var i = 0; i < metas.length; i++) {
    var m = metas[i];
    var p = m.getAttribute('property') || '';
    var n = m.getAttribute('name') || '';
    var c = m.getAttribute('content') || '';
    if (p === 'og:title') d.title = c;
    if (p === 'og:description') d.description = c;
    if (p === 'og:image') d.image = c;
    if (p === 'og:site_name') d.publication = c;
    if (n === 'author') d.author = c;
    if (p === 'article:published_time') d.published_date = c;
    if (n === 'description' && !d.description) d.description = c;
  }
  if (!d.title) {
    var h1 = document.querySelector('h1');
    d.title = h1 ? h1.innerText : document.title;
  }
  if (!d.author) {
    var authorEl = document.querySelector('a[rel=author]');
    if (authorEl) d.author = authorEl.innerText;
  }
  if (!d.published_date) {
    var timeEl = document.querySelector('time[datetime]');
    if (timeEl) d.published_date = timeEl.getAttribute('datetime');
  }
  var article = document.querySelector('article');
  d.article_text = article ? article.innerText : '';
  return JSON.stringify(d);
})()"#;

/// Extract a gist from a Medium article page via DOM extraction
pub async fn extract(
    url: &str,
    source_type: &SourceType,
    domain: &str,
) -> Result<PageGist, String> {
    let adapter = ChromeAppleScriptAdapter;

    // Single JS call extracts all metadata + full article text from the DOM
    let json_str = adapter.execute_js_in_tab(url, EXTRACT_JS).await?;

    let data: MediumDomData = serde_json::from_str(json_str.trim())
        .map_err(|e| format!("Failed to parse Medium page data: {}", e))?;

    let title = data.title.unwrap_or_else(|| "Unknown".to_string());
    let article_text = data.article_text.unwrap_or_default();

    // Extract reading time from the article text (done in Rust to avoid regex in JS/AppleScript)
    let reading_time = extract_reading_time(&article_text);

    // Format date to human-readable
    let published_date = data.published_date.map(|date_str| {
        if let Some(date_part) = date_str.split('T').next() {
            if let Ok(parsed) = chrono::NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
                return parsed.format("%B %d, %Y").to_string();
            }
        }
        date_str
    });

    // Full article content (no truncation — user wants the complete article)
    let content_excerpt = if article_text.is_empty() {
        None
    } else {
        Some(article_text)
    };

    // Build extra JSON with Medium-specific fields
    let mut extra = serde_json::Map::new();
    if let Some(pub_name) = data.publication.as_ref() {
        extra.insert(
            "publication".to_string(),
            serde_json::Value::String(pub_name.clone()),
        );
    }
    if let Some(time) = reading_time {
        extra.insert(
            "reading_time_minutes".to_string(),
            serde_json::Value::Number(time.into()),
        );
    }

    Ok(PageGist {
        url: url.to_string(),
        title,
        source_type: source_type.clone(),
        domain: domain.to_string(),
        author: data.author,
        description: data.description,
        content_excerpt,
        published_date,
        image_url: data.image,
        extra: serde_json::Value::Object(extra),
    })
}
