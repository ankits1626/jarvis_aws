# Design Document: Medium Article Extractor

## Overview

The Medium Article Extractor is a specialized extractor module for the JARVIS Browser Tool that produces richer gists from Medium.com articles compared to the generic extractor. When a user selects a Medium tab and clicks "Prepare Gist", the system routes to this extractor which understands Medium's specific HTML structure.

The extractor uses DOM extraction instead of HTTP fetching: it retrieves the HTML directly from the user's authenticated Chrome tab via AppleScript + JavaScript execution. This bypasses Medium's paywall (user is already logged in), captures lazy-loaded content (if user scrolled), and avoids network errors. The extractor extracts metadata using regex patterns and returns a `PageGist` struct with Medium-specific fields stored in the `extra` JSON field. The implementation integrates seamlessly into the existing extractor router without requiring frontend changes.

### Key Design Decisions

1. **DOM extraction over HTTP fetching**: Extract HTML from the browser DOM via AppleScript instead of re-fetching with reqwest. This bypasses paywalls, captures authenticated content, and gets the actual rendered DOM (including lazy-loaded content).

2. **Regex-based extraction**: Following the YouTube extractor pattern, we use regex to extract metadata from HTML rather than a full HTML parser. This keeps dependencies minimal and performance high.

3. **Graceful degradation**: When Medium-specific extraction fails, fall back to OG metadata extraction (same as generic extractor). This ensures the feature degrades gracefully rather than failing completely.

4. **Minimal routing changes**: The router modification is a single match arm addition, keeping the architecture clean and extensible for future extractors.

5. **Reuse existing utilities**: Leverage helper functions from `generic.rs` for OG metadata extraction and HTML entity decoding to avoid code duplication.

6. **Browser adapter extension**: Add `get_tab_html(url)` method to the `BrowserAdapter` trait, implemented via AppleScript JavaScript execution in Chrome.

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                      Browser Tool UI                         │
│                   (BrowserTool.tsx)                          │
└────────────────────────┬────────────────────────────────────┘
                         │ invoke('prepare_tab_gist')
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                   Tauri Command Layer                        │
│                   (commands.rs)                              │
└────────────────────────┬────────────────────────────────────┘
                         │ prepare_tab_gist(url, source_type)
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                   Extractor Router                           │
│              (extractors/mod.rs)                             │
│                                                              │
│  match source_type {                                         │
│    YouTube => youtube_gist()                                 │
│    _ if domain.contains("medium.com") => medium::extract()   │
│    _ => generic::extract()                                   │
│  }                                                           │
└──────────┬──────────────────────┬────────────────┬──────────┘
           │                      │                │
           ▼                      ▼                ▼
    ┌──────────────┐      ┌──────────────┐  ┌──────────────┐
    │   YouTube    │      │    Medium    │  │   Generic    │
    │  Extractor   │      │  Extractor   │  │  Extractor   │
    │ (youtube.rs) │      │ (medium.rs)  │  │ (generic.rs) │
    └──────────────┘      └──────┬───────┘  └──────────────┘
                                 │
                                 ▼
                    ┌────────────────────────┐
                    │   Chrome Adapter       │
                    │ (adapters/chrome.rs)   │
                    │                        │
                    │  get_tab_html(url)     │
                    └────────┬───────────────┘
                             │
                             ▼
                    ┌────────────────────────┐
                    │   AppleScript          │
                    │   + JavaScript         │
                    │                        │
                    │  Execute in Chrome tab │
                    │  Return outerHTML      │
                    └────────────────────────┘
                             │
                             ▼ Returns PageGist
           ┌─────────────────────────────┐
           │       PageGist Struct       │
           │  - url                      │
           │  - title                    │
           │  - source_type              │
           │  - domain                   │
           │  - author                   │
           │  - description              │
           │  - content_excerpt          │
           │  - published_date           │
           │  - image_url                │
           │  - extra (JSON)             │
           └─────────────────────────────┘
```

### Data Flow

1. User scrolls through Medium article (to load lazy-loaded content)
2. User clicks "Prepare Gist" on the Medium tab
3. Frontend calls `invoke('prepare_tab_gist', { url, sourceType: 'Article' })`
4. Backend `commands::prepare_tab_gist()` calls `extractors::prepare_gist()`
5. Router checks domain: `medium.com` → dispatch to `medium::extract()`
6. Medium extractor:
   - Calls `ChromeAppleScriptAdapter::get_tab_html(url)` to get DOM HTML
   - Chrome adapter finds matching tab via AppleScript
   - Executes JavaScript `document.documentElement.outerHTML` in that tab
   - Returns the full rendered HTML (including lazy-loaded content)
   - Extracts metadata using regex patterns
   - Extracts article body from `<article>` tags
   - Truncates content to ~500 chars
   - Populates `PageGist` with Medium-specific `extra` fields
7. Returns `PageGist` to frontend
8. Frontend renders gist card (no changes needed)

## Components and Interfaces

### Medium Extractor Module (`medium.rs`)

```rust
/// Extract a gist from a Medium article page
pub async fn extract(
    url: &str,
    source_type: &SourceType,
    domain: &str
) -> Result<PageGist, String>
```

**Responsibilities:**
- Get Medium article HTML from browser DOM via Chrome adapter
- Extract title, author, publication, date, reading time
- Extract clean article body excerpt
- Handle errors gracefully with fallbacks
- Return unified `PageGist` struct

### Chrome Adapter Extension (`adapters/chrome.rs`)

```rust
/// Get HTML content from a specific Chrome tab by URL
pub async fn get_tab_html(&self, url: &str) -> Result<String, String>
```

**Responsibilities:**
- Find the Chrome tab matching the given URL
- Execute JavaScript in that tab to retrieve `document.documentElement.outerHTML`
- Return the full rendered HTML (including lazy-loaded content)
- Handle errors (tab not found, Chrome not responding, permissions issues)

### Internal Helper Functions

```rust
/// Extract reading time from Medium HTML (e.g. "8 min read")
fn extract_reading_time(html: &str) -> Option<u32>
```

**Note**: Article body extraction, HTML tag stripping, and text truncation logic already exists in `generic.rs` and will be reused via `pub(crate)` functions.

### Router Integration (`extractors/mod.rs`)

**Modified function:**
```rust
pub async fn prepare_gist(
    url: &str,
    source_type: &SourceType
) -> Result<PageGist, String> {
    let domain = super::tabs::extract_domain(url);

    match source_type {
        SourceType::YouTube => youtube_gist(url, &domain).await,
        _ if domain.contains("medium.com") => medium::extract(url, source_type, &domain).await,
        _ => generic::extract(url, source_type, &domain).await,
    }
}
```

**Changes:**
- Add `pub mod medium;` declaration
- Add Medium domain check in match arm before generic fallback

## Data Models

### PageGist Structure (Existing)

```rust
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
    pub extra: serde_json::Value,
}
```

### Medium-Specific Extra Fields

```json
{
  "publication": "Towards Data Science",
  "reading_time_minutes": 8
}
```

**Field descriptions:**
- `publication`: The Medium publication name (e.g. "Better Programming", "UX Collective"). Omitted if article is on personal blog.
- `reading_time_minutes`: Estimated reading time extracted from page. Omitted if not found.

### Medium HTML Structure (Research)

Based on Medium's current HTML structure (as of 2024-2025):

**Title extraction sources (priority order):**
1. `<meta property="og:title" content="...">`
2. `<h1>` tag within `<article>`
3. `<title>` tag

**Author extraction sources:**
1. `<meta name="author" content="...">`
2. `<a rel="author" ...>` tag text content
3. `<meta property="article:author" content="...">`

**Publication extraction:**
1. `<meta property="og:site_name" content="...">`
2. `<meta property="al:ios:app_name" content="...">`

**Published date extraction:**
1. `<meta property="article:published_time" content="...">`
2. `<time>` tag with `datetime` attribute

**Reading time extraction:**
- Look for patterns like "8 min read", "5 minute read" in HTML
- Common locations: near author byline, in `<span>` tags
- Regex pattern: `(\d+)\s*min(?:ute)?s?\s*read`

**Article body extraction:**
- Extract from `<article>` tag
- Strip all HTML tags to get plain text
- Remove Medium UI elements (navigation, footer, related articles)
- Focus on paragraph content within article

**Image extraction:**
1. `<meta property="og:image" content="...">`

**Description extraction:**
1. `<meta property="og:description" content="...">`
2. `<meta name="description" content="...">`

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*


### Property Reflection

After analyzing all acceptance criteria, I identified the following consolidation opportunities:

1. **Metadata extraction properties (2.3-2.8)** can be consolidated into a single comprehensive property about metadata extraction with fallbacks, since they all follow the same pattern: try primary source, fall back to OG metadata.

2. **Missing field properties (4.3-4.4)** can be combined into one property about optional field handling in the extra JSON.

3. **Error handling properties (7.1, 7.3)** are essentially the same - both test that failed HTTP requests return errors rather than panicking.

4. **Truncation properties (3.4, 3.5)** can be combined since appending "..." is part of the truncation behavior.

5. **Routing properties (1.1, 1.2, 1.4)** can be consolidated into one property about Medium URL detection and routing.

This reduces 20+ testable criteria to 12 focused, non-redundant properties.

### Property 1: Medium URL Routing

*For any* URL containing "medium.com" in its domain and classified as `SourceType::Article`, the router should dispatch to the Medium extractor instead of the generic extractor.

**Validates: Requirements 1.1, 1.2, 1.4**

### Property 2: Metadata Extraction with Fallbacks

*For any* Medium article HTML, when extracting metadata fields (title, author, publication, date, image, description), the extractor should try Medium-specific sources first, then fall back to OG metadata, and use `None` if neither is available, without failing.

**Validates: Requirements 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9**

### Property 3: Article Body Extraction

*For any* Medium HTML containing `<article>` tags, extracting the article body should successfully extract text content from within those tags while excluding content outside the article element.

**Validates: Requirements 3.1**

### Property 4: HTML Tag Stripping

*For any* HTML content, stripping HTML tags should remove all tag markers (anything between `<` and `>`) while preserving the text content between tags.

**Validates: Requirements 3.2**

### Property 5: Content Truncation at Word Boundary

*For any* text longer than 500 characters, truncating it should produce text of approximately 500 characters (±50 chars) that ends at a word boundary (space character) and is appended with "...".

**Validates: Requirements 3.4, 3.5**

### Property 6: HTML Entity Decoding

*For any* text containing HTML entities (e.g., `&amp;`, `&lt;`, `&#39;`), decoding should convert them to their corresponding characters (e.g., `&`, `<`, `'`).

**Validates: Requirements 3.6**

### Property 7: Reading Time Extraction

*For any* Medium HTML containing reading time indicators (patterns like "8 min read", "5 minute read"), the extractor should successfully extract the numeric reading time value.

**Validates: Requirements 4.1**

### Property 8: Extra Field Population

*For any* successfully extracted Medium metadata, the `PageGist.extra` field should be valid JSON containing only the fields that were successfully extracted (publication and/or reading_time_minutes), omitting fields that couldn't be extracted.

**Validates: Requirements 4.2, 4.3, 4.4**

### Property 9: PageGist Structure Completeness

*For any* successful Medium extraction, the returned `PageGist` should have all required fields populated: url (original URL), title (non-empty), source_type (Article), domain (extracted domain), and extra (valid JSON).

**Validates: Requirements 5.1**

### Property 10: HTTP Error Handling

*For any* HTTP request that fails (non-200 status, timeout, network error), the extractor should return a `Result::Err` with a descriptive error message rather than panicking.

**Validates: Requirements 7.1, 7.3**

### Property 11: Malformed HTML Fallback

*For any* HTML that lacks Medium-specific structure (no `<article>` tags, missing metadata), the extractor should fall back to generic OG metadata extraction and still return a valid `PageGist` rather than failing.

**Validates: Requirements 7.2**

### Property 12: Text Truncation Preserves Content

*For any* text shorter than or equal to 500 characters, truncation should return the original text unchanged (no "..." appended).

**Validates: Requirements 3.4** (edge case)

## Error Handling

### Error Categories

1. **Browser/Tab Errors**
   - Chrome tab not found for the given URL (tab was closed)
   - Chrome not running or not responding
   - AppleScript execution failure (permissions issue, Chrome busy)
   - **Handling**: Return `Result::Err` with user-friendly message like "Chrome tab not found for this URL - the tab may have been closed"

2. **Parsing Errors**
   - Missing expected HTML structure (no `<article>` tags)
   - Malformed HTML that breaks regex patterns
   - Missing all metadata sources (no OG tags, no Medium-specific tags)
   - **Handling**: Fall back to generic extractor behavior (OG metadata + `<p>` tag extraction)

3. **Content Errors**
   - Empty article body (user didn't scroll to load content)
   - Incomplete lazy-loaded content
   - **Handling**: Extract whatever is available, return partial `PageGist` with available fields

### Error Messages

All error messages should be user-friendly strings suitable for display in the UI:

- ✅ "Chrome tab not found for this URL - the tab may have been closed"
- ✅ "Failed to extract HTML from Chrome tab - check Chrome permissions"
- ✅ "Unable to extract article content - try scrolling through the article first"
- ❌ "AppleScript execution failed: error -1728" (too technical)
- ❌ "NoneError" (not descriptive)

### Fallback Strategy

```
Medium Extraction Attempt
    ↓
Get HTML from Chrome tab
    ↓ Success              ↓ Failure
    │                   Return error
    ↓
Medium-specific metadata found?
    ↓ Yes                    ↓ No
Use Medium extraction    Fall back to OG metadata
    ↓                        ↓
Article body found?      <p> tags found?
    ↓ Yes    ↓ No          ↓ Yes    ↓ No
Extract   Use OG desc   Extract   Return minimal gist
```

## Testing Strategy

### Dual Testing Approach

The Medium extractor will be tested using both unit tests and property-based tests:

- **Unit tests**: Verify specific examples, edge cases, and error conditions with known HTML samples
- **Property tests**: Verify universal properties across randomized inputs to catch edge cases

Both approaches are complementary and necessary for comprehensive coverage. Unit tests catch concrete bugs with specific examples, while property tests verify general correctness across many inputs.

### Unit Testing

Unit tests will focus on:

1. **Specific examples**: Known Medium article HTML structures
2. **Edge cases**: Empty fields, missing metadata, malformed HTML
3. **Error conditions**: Network failures, non-200 status codes
4. **Integration points**: Router dispatch logic, PageGist construction

Example unit tests:

```rust
#[test]
fn test_extract_title_from_og_metadata() {
    let html = r#"<meta property="og:title" content="Test Article">"#;
    let title = extract_title(html);
    assert_eq!(title, Some("Test Article".to_string()));
}

#[test]
fn test_extract_reading_time() {
    let html = r#"<span>8 min read</span>"#;
    let time = extract_reading_time(html);
    assert_eq!(time, Some(8));
}

#[test]
fn test_truncate_at_word_boundary() {
    let text = "This is a very long article ".repeat(50);
    let truncated = truncate_excerpt(&text, 500);
    assert!(truncated.len() <= 550); // ~500 + "..."
    assert!(truncated.ends_with("..."));
    assert!(!truncated[..truncated.len()-3].ends_with(char::is_whitespace));
}

#[test]
fn test_strip_html_tags() {
    let html = "<p>Hello <strong>world</strong></p>";
    let text = strip_html_tags(html);
    assert_eq!(text, "Hello world");
}

#[test]
fn test_missing_metadata_uses_fallback() {
    let html = r#"<html><head><title>Test</title></head></html>"#;
    // Should not panic, should use fallbacks
    let title = extract_og_content(html, "og:title");
    assert!(title.is_none()); // No OG metadata present
    let author = extract_meta_content(html, "author");
    assert!(author.is_none()); // No author metadata present
}
```

### Property-Based Testing

Property tests will use the `proptest` crate (already in dependencies) to verify universal properties across randomized inputs. Each property test will run a minimum of 100 iterations.

**Property Test Configuration:**
- Library: `proptest` (already in Cargo.toml)
- Iterations: 100 minimum per test
- Tag format: `// Feature: jarvis-medium-extractor, Property {number}: {property_text}`

Example property tests:

```rust
use proptest::prelude::*;

// Feature: jarvis-medium-extractor, Property 4: HTML Tag Stripping
// For any HTML content, stripping HTML tags should remove all tag markers
proptest! {
    #[test]
    fn prop_strip_html_tags_removes_all_tags(
        text in "[a-zA-Z0-9 ]{10,100}",
        tag in "(p|div|span|strong|em|a)"
    ) {
        let html = format!("<{}>{}</{}>", tag, text, tag);
        let stripped = strip_html_tags(&html);
        
        prop_assert!(!stripped.contains('<'));
        prop_assert!(!stripped.contains('>'));
        prop_assert!(stripped.contains(&text));
    }
}

// Feature: jarvis-medium-extractor, Property 5: Content Truncation at Word Boundary
// For any text longer than 500 characters, truncation should produce text
// ending at a word boundary with "..."
proptest! {
    #[test]
    fn prop_truncate_at_word_boundary(
        words in prop::collection::vec("[a-z]{3,10}", 100..200)
    ) {
        let text = words.join(" ");
        prop_assume!(text.len() > 500);
        
        let truncated = truncate_excerpt(&text, 500);
        
        prop_assert!(truncated.len() <= 550);
        prop_assert!(truncated.ends_with("..."));
        
        // Check that it ends at a word boundary (before "...")
        let without_ellipsis = &truncated[..truncated.len()-3];
        if let Some(last_char) = without_ellipsis.chars().last() {
            prop_assert!(!last_char.is_whitespace());
        }
    }
}

// Feature: jarvis-medium-extractor, Property 6: HTML Entity Decoding
// For any text containing HTML entities, decoding should convert them correctly
proptest! {
    #[test]
    fn prop_decode_html_entities(
        text1 in "[a-zA-Z ]{5,20}",
        text2 in "[a-zA-Z ]{5,20}"
    ) {
        let encoded = format!("{}&amp;{}", text1, text2);
        let decoded = decode_html_entities(&encoded);
        
        prop_assert!(decoded.contains('&'));
        prop_assert!(!decoded.contains("&amp;"));
        prop_assert!(decoded.contains(&text1));
        prop_assert!(decoded.contains(&text2));
    }
}

// Feature: jarvis-medium-extractor, Property 12: Text Truncation Preserves Content
// For any text shorter than 500 characters, truncation should return it unchanged
proptest! {
    #[test]
    fn prop_short_text_not_truncated(
        text in "[a-zA-Z0-9 ]{10,400}"
    ) {
        prop_assume!(text.len() <= 500);
        
        let result = truncate_excerpt(&text, 500);
        
        prop_assert_eq!(result, text);
        prop_assert!(!result.ends_with("..."));
    }
}
```

### Test Organization

All tests will be placed in a `#[cfg(test)] mod tests` block within `medium.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // Unit tests
    #[test]
    fn test_extract_title() { /* ... */ }
    
    // Property tests
    mod property_tests {
        use super::*;
        use proptest::prelude::*;
        
        proptest! {
            #[test]
            fn prop_strip_html_tags(/* ... */) { /* ... */ }
        }
    }
}
```

### Testing Medium-Specific Behavior

Since we can't make live network requests in tests, we'll use inline HTML strings representing typical Medium article structures:

```rust
const SAMPLE_MEDIUM_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <meta property="og:title" content="Understanding Rust Lifetimes">
    <meta property="og:description" content="A deep dive into Rust's lifetime system">
    <meta property="og:image" content="https://miro.medium.com/max/1200/1*abc.jpg">
    <meta property="og:site_name" content="Better Programming">
    <meta name="author" content="Jane Developer">
    <meta property="article:published_time" content="2025-01-15T10:30:00Z">
</head>
<body>
    <article>
        <h1>Understanding Rust Lifetimes</h1>
        <span>8 min read</span>
        <p>Rust's lifetime system is one of its most powerful features...</p>
        <p>In this article, we'll explore how lifetimes work...</p>
    </article>
</body>
</html>
"#;
```

## Implementation Notes

### Reusing Generic Extractor Utilities

The generic extractor (`generic.rs`) already provides helper functions and regex patterns that should be reused:

**Functions to make `pub(crate)`:**
- `extract_og_content(html, property)` - Extract OG metadata
- `extract_meta_content(html, name)` - Extract meta tags
- `extract_html_title(html)` - Extract `<title>` tag
- `decode_html_entities(s)` - Decode HTML entities

**Regex patterns to extract and make `pub(crate)`:**
- `ARTICLE_REGEX` - Currently defined inside `extract_content_excerpt()`, needs to be moved to module level and made `pub(crate)` - Matches `(?is)<article[^>]*>(.*?)</article>`
- `TAG_REGEX` - Currently defined inside `extract_content_excerpt()`, needs to be moved to module level and made `pub(crate)` - Matches `<[^>]+>` for stripping HTML tags

**Note**: These regex patterns are currently function-local statics inside `extract_content_excerpt()`. They must be moved to module-level statics before they can be made `pub(crate)` (Rust doesn't allow visibility modifiers on function-local statics).

**Implementation approach for Medium extractor:**

The Medium extractor will need to extract article body text and strip HTML tags. Rather than duplicating the logic from `extract_content_excerpt()` in `generic.rs`, the implementation should:

1. Use `ARTICLE_REGEX` (made `pub(crate)`) to extract `<article>` content
2. Use `TAG_REGEX` (made `pub(crate)`) to strip HTML tags
3. Implement its own truncation logic (simpler than `extract_content_excerpt` since Medium articles don't need the fallback to `<main>` or multiple `<p>` tags)

This approach reuses the regex patterns while allowing Medium-specific extraction flow.

The Medium extractor should import these utilities:

```rust
use super::generic::{
    extract_og_content,
    extract_meta_content,
    decode_html_entities,
    ARTICLE_REGEX,
    TAG_REGEX,
};
```

### Regex Patterns

Following the YouTube extractor pattern, use `LazyLock` for compiled regex patterns:

```rust
use std::sync::LazyLock;
use regex::Regex;

static READING_TIME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\d+)\s*min(?:ute)?s?\s*read").unwrap()
});
```

**Note**: `ARTICLE_REGEX` and `TAG_REGEX` will be imported from `generic.rs` (made `pub(crate)`) rather than redefined, to avoid duplication.

### DOM Extraction via Chrome Adapter

The Medium extractor uses DOM extraction instead of HTTP fetching to bypass paywalls and get authenticated content:

**Chrome Adapter Extension (`adapters/chrome.rs`):**

Add a new method to retrieve HTML from a specific Chrome tab using a **secure two-step approach**:

**Step 1: Get all tabs with their window/tab indices (reuse existing `list_tabs()` logic)**
**Step 2: Find matching tab in Rust, then execute JavaScript using numeric indices (no string interpolation)**

```rust
pub async fn get_tab_html(&self, url: &str) -> Result<String, String> {
    // Step 1: Get all tabs with their positions
    // This AppleScript returns: windowIndex|||tabIndex|||URL (one per line)
    let list_script = r#"
tell application "Google Chrome"
    set tabInfo to ""
    set windowCount to count of windows
    repeat with w from 1 to windowCount
        set tabCount to count of tabs in window w
        repeat with t from 1 to tabCount
            set tabURL to URL of tab t of window w
            set tabInfo to tabInfo & w & "|||" & t & "|||" & tabURL & linefeed
        end repeat
    end repeat
    return tabInfo
end tell
"#;

    let output = Command::new("osascript")
        .arg("-e")
        .arg(list_script)
        .output()
        .map_err(|e| format!("Failed to execute AppleScript: {}", e))?;

    if !output.status.success() {
        return Err("Failed to list Chrome tabs".to_string());
    }

    // Step 2: Find matching tab in Rust (safe string comparison)
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut window_idx = None;
    let mut tab_idx = None;

    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(3, "|||").collect();
        if parts.len() == 3 {
            let tab_url = parts[2].trim();
            if tab_url == url {
                window_idx = parts[0].parse::<usize>().ok();
                tab_idx = parts[1].parse::<usize>().ok();
                break;
            }
        }
    }

    let (w, t) = match (window_idx, tab_idx) {
        (Some(w), Some(t)) => (w, t),
        _ => return Err("Chrome tab not found for this URL - the tab may have been closed".to_string()),
    };

    // Step 3: Execute JavaScript on the specific tab using numeric indices
    // SAFE: No string interpolation of user-controlled data
    let js_script = format!(r#"
tell application "Google Chrome"
    execute tab {} of window {} javascript "document.documentElement.outerHTML"
end tell
"#, t, w);

    let output = Command::new("osascript")
        .arg("-e")
        .arg(&js_script)
        .output()
        .map_err(|e| format!("Failed to execute JavaScript: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to extract HTML from tab: {}", stderr));
    }

    let html = String::from_utf8_lossy(&output.stdout).to_string();
    
    // Sanity check: HTML size should be reasonable (typical Medium article: 500KB-2MB)
    const MAX_HTML_SIZE: usize = 5 * 1024 * 1024; // 5MB
    if html.len() > MAX_HTML_SIZE {
        return Err(format!(
            "HTML output too large ({} bytes) - this may not be a normal article page",
            html.len()
        ));
    }

    Ok(html)
}
```

**Performance Notes:**
- **Typical Medium article HTML**: 500KB - 2MB (includes embedded scripts, styles, lazy-loaded content)
- **AppleScript stdout limit**: Can handle multi-MB output without issues
- **Size check**: Reject HTML > 5MB as a sanity check (likely not a normal article page)
- **Memory usage**: HTML is processed in-memory, then only metadata + ~500 char excerpt is kept
- **Blocking I/O**: Uses `std::process::Command` (blocking) in async function, consistent with existing `list_tabs()` implementation. AppleScript execution is fast enough (~100-500ms) that this is acceptable for MVP.

**Security Notes:**
- **No URL string interpolation in AppleScript** - URLs are compared in Rust after retrieval
- **Numeric indices only** - Window and tab indices are integers, not user-controlled strings
- **Two-step approach** - List all tabs first, find match in Rust, then execute JS with safe numeric indices
- **Prevents injection attacks** - URLs with quotes, newlines, or AppleScript syntax cannot break the script

**BrowserAdapter Trait Extension (`adapters/mod.rs`):**

Add the method to the trait:

```rust
pub trait BrowserAdapter {
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;
    fn list_tabs(&self) -> impl std::future::Future<Output = Result<Vec<RawTab>, String>> + Send;
    
    // NEW: Get HTML from a specific tab by URL
    fn get_tab_html(&self, url: &str) -> impl std::future::Future<Output = Result<String, String>> + Send;
}
```

**Medium Extractor Usage:**

```rust
pub async fn extract(url: &str, source_type: &SourceType, domain: &str) -> Result<PageGist, String> {
    let adapter = ChromeAppleScriptAdapter;
    
    // Get HTML from browser DOM instead of HTTP fetch
    let html = adapter.get_tab_html(url).await?;
    
    // Extract metadata and content from HTML...
}
```

### User-Agent String (Not Needed)

Since we're extracting from the browser DOM, no User-Agent header is needed. The browser already handled authentication and rendering.

### Handling Paywalled Articles

By extracting from the browser DOM, we bypass Medium's paywall entirely:

1. User is already authenticated in Chrome
2. User has scrolled through the article (lazy-loaded content is in DOM)
3. Extractor gets the full rendered HTML with all content visible to the user
4. No paywall preview limitations

**User Workflow:**
1. User opens Medium article in Chrome (logs in if needed)
2. User scrolls through the full article to load lazy-loaded content
3. User clicks "Prepare Gist" in JARVIS
4. Extractor retrieves the full DOM HTML with all content

### Future Enhancements (Out of Scope for MVP)

1. **Custom domain detection**: Detect Medium custom domains (e.g., `betterprogramming.pub`, `towardsdatascience.com`) by checking for `<meta property="al:android:app_name" content="Medium">` in the HTML

2. **Clap count extraction**: Extract the number of claps/likes from the page

3. **Tag extraction**: Extract article tags/topics from Medium's tag system

4. **Response count**: Extract number of responses/comments

5. **Member-only indicator**: Detect and flag member-only articles

These can be added incrementally by extending the `extra` JSON field without breaking existing functionality.

## File Changes Summary

### New Files

**`src-tauri/src/browser/extractors/medium.rs`**
- Medium article extractor implementation
- Helper functions for metadata extraction
- Unit tests and property tests

### Modified Files

**`src-tauri/src/browser/extractors/mod.rs`**
- Add `pub mod medium;` declaration
- Modify `prepare_gist()` to check for Medium domain and dispatch accordingly

**`src-tauri/src/browser/adapters/mod.rs`**
- Add `get_tab_html(url)` method to `BrowserAdapter` trait

**`src-tauri/src/browser/adapters/chrome.rs`**
- Implement `get_tab_html()` using AppleScript + JavaScript execution
- Find matching tab by URL
- Execute `document.documentElement.outerHTML` in that tab
- Return full rendered HTML

**`src-tauri/src/browser/extractors/generic.rs`**
- Change visibility of helper functions from `fn` to `pub(crate) fn`:
  - `extract_og_content`
  - `extract_meta_content`
  - `extract_html_title`
  - `decode_html_entities`
- Move regex patterns from inside `extract_content_excerpt()` to module level:
  - `ARTICLE_REGEX` (currently at line ~136 inside function)
  - `TAG_REGEX` (currently at line ~160 inside function)
- Change visibility of the moved regex patterns to `pub(crate) static`
- Update `extract_content_excerpt()` to reference the module-level patterns
- This allows the Medium extractor to reuse these utilities and avoid code duplication

### No Changes Required

- `src-tauri/src/commands.rs` - Tauri commands unchanged
- `src-tauri/src/browser/tabs.rs` - URL classification unchanged
- `src/components/BrowserTool.tsx` - Frontend unchanged
- `src/state/types.ts` - TypeScript types unchanged
- `Cargo.toml` - No new dependencies needed

## Dependencies

All required dependencies are already present in `Cargo.toml`:

- `reqwest` - HTTP client for fetching pages
- `regex` - Pattern matching for metadata extraction
- `serde` / `serde_json` - JSON serialization for `extra` field
- `proptest` - Property-based testing (dev dependency)

No new dependencies need to be added.
