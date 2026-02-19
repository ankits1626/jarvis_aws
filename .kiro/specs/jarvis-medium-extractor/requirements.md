# Requirements Document

## Introduction

The Medium Extractor is a specialized extractor for the JARVIS Browser Tool that produces richer gists from Medium.com articles compared to the generic extractor. When a user selects a Medium tab in the Browser Tool and clicks "Prepare Gist", the system routes to this extractor instead of the generic one. The Medium extractor understands Medium's specific HTML structure — extracting the article title, author name, publication name, published date, reading time, clap count, and a clean article body excerpt stripped of Medium's UI chrome. The extractor returns a `PageGist` (the unified gist type used by all extractors) with Medium-specific fields stored in the `extra` JSON field. No new frontend component is needed — the existing `BrowserTool.tsx` gist card already renders `PageGist` data. No new Tauri commands are needed — the existing `prepare_tab_gist` command routes through `extractors/mod.rs` which dispatches based on `SourceType`.

## Glossary

- **Browser_Tool**: The on-demand JARVIS feature that lists all open Chrome tabs and lets the user select one to create a gist
- **Extractor**: A Rust module that fetches a URL and produces a `PageGist` struct. Each extractor specializes in a specific type of website (YouTube, Medium, generic, etc.)
- **PageGist**: The unified gist struct returned by all extractors, containing: url, title, source_type, domain, author, description, content_excerpt, published_date, image_url, and extra (JSON for source-specific fields)
- **SourceType**: An enum classifying URLs by domain (YouTube, Article, Code, Docs, QA, News, Research, Social, Other). Medium falls under `Article`.
- **Generic_Extractor**: The fallback extractor (`extractors/generic.rs`) that works on any URL using OG metadata and `<p>` tag extraction
- **Medium_Extractor**: The new extractor (`extractors/medium.rs`) that understands Medium's HTML structure to produce richer gists
- **Extractor_Router**: The `prepare_gist()` function in `extractors/mod.rs` that dispatches to the right extractor based on URL domain and SourceType
- **OG_Metadata**: Open Graph `<meta>` tags (og:title, og:description, og:image) used by social media platforms for link previews
- **Extra_Fields**: The `extra: serde_json::Value` field in PageGist used for source-specific metadata (e.g. reading_time, clap_count for Medium; duration_seconds for YouTube)

## Context: Existing Architecture

The Browser Tool extractor pipeline is already built:

```
User clicks "Prepare Gist" on a tab
  → Frontend calls invoke('prepare_tab_gist', { url, sourceType })
  → Backend: commands::prepare_tab_gist()
  → Backend: extractors::prepare_gist(url, source_type)
  → Router matches on source_type / domain:
      YouTube → youtube_gist()       (existing)
      Medium  → medium::extract()    (NEW — this spec)
      *       → generic::extract()   (existing fallback)
  → Returns PageGist
  → Frontend renders gist card
```

Key files in the existing codebase:

| File | Purpose |
|------|---------|
| `src-tauri/src/browser/extractors/mod.rs` | Router + PageGist struct definition |
| `src-tauri/src/browser/extractors/generic.rs` | Generic extractor (OG metadata + `<p>` tags) |
| `src-tauri/src/browser/tabs.rs` | SourceType enum + classify_url() + extract_domain() |
| `src-tauri/src/browser/youtube.rs` | YouTube scraper (reference for extractor pattern) |
| `src/components/BrowserTool.tsx` | Frontend component (renders PageGist — no changes needed) |
| `src/state/types.ts` | TypeScript PageGist interface (no changes needed) |

## Requirements

### Requirement 1: Medium Article Detection

**User Story:** As a JARVIS user, I want Medium articles to be identified and routed to the specialized Medium extractor, so that I get richer gists than the generic extractor provides.

#### Acceptance Criteria

1. WHEN a URL's domain is `medium.com` or a known Medium custom domain, THE Extractor_Router SHALL dispatch to the Medium_Extractor instead of the Generic_Extractor
2. THE Extractor_Router SHALL detect Medium URLs by checking if the domain contains `medium.com`
3. THE Extractor_Router SHALL also detect Medium custom domains by checking for the `<meta property="al:android:app_name" content="Medium">` tag in the fetched HTML as a fallback (NOT required for MVP — can be added later)
4. THE classification in `tabs.rs` already maps `medium.com` to `SourceType::Article` — this SHALL NOT change. The router in `extractors/mod.rs` SHALL add a domain check within the `Article` match arm to dispatch Medium URLs specifically

### Requirement 2: Medium Article Metadata Extraction

**User Story:** As a JARVIS user, I want JARVIS to extract the article's title, author, publication, and date from a Medium page, so that the gist includes proper attribution.

#### Acceptance Criteria

1. THE Medium_Extractor SHALL obtain the Medium article page HTML from the browser DOM via the Chrome adapter's `get_tab_html(url)` method
2. THE Chrome adapter SHALL use AppleScript to execute JavaScript in the matching Chrome tab to retrieve `document.documentElement.outerHTML`
3. THE Medium_Extractor SHALL extract the article title from `<meta property="og:title" content="...">` or the `<h1>` tag
4. THE Medium_Extractor SHALL extract the author name from `<meta name="author" content="...">` or the `<a rel="author">` tag
5. THE Medium_Extractor SHALL extract the publication name from `<meta property="og:site_name" content="...">` (e.g. "Towards Data Science", "Better Programming")
6. THE Medium_Extractor SHALL extract the published date from `<meta property="article:published_time" content="...">` and format it as a human-readable date (e.g. "2025-12-15")
7. THE Medium_Extractor SHALL extract the OG image URL from `<meta property="og:image" content="...">`
8. THE Medium_Extractor SHALL extract the OG description from `<meta property="og:description" content="...">`
9. WHEN any metadata field cannot be extracted, THE Medium_Extractor SHALL fall back to the value from OG metadata or use `None`/default rather than failing

### Requirement 3: Medium Article Content Extraction

**User Story:** As a JARVIS user, I want the gist to include a clean excerpt of the article body, so that I can understand what the article covers without opening it.

#### Acceptance Criteria

1. THE Medium_Extractor SHALL extract article body text from `<article>` tags in the HTML
2. THE Medium_Extractor SHALL strip all HTML tags from the extracted body to produce plain text
3. THE Medium_Extractor SHALL remove Medium UI elements (navigation, footer, related articles, sidebar) by focusing extraction on the `<article>` element
4. THE Medium_Extractor SHALL truncate the content excerpt to approximately 500 characters, breaking at a word boundary
5. THE Medium_Extractor SHALL append "..." to truncated excerpts
6. THE Medium_Extractor SHALL decode HTML entities in the extracted text (e.g. `&amp;` → `&`, `&#39;` → `'`)

### Requirement 4: Medium-Specific Extra Fields

**User Story:** As a JARVIS user, I want the gist to include Medium-specific metadata like reading time and publication, so that I get more context than a generic page extraction would provide.

#### Acceptance Criteria

1. THE Medium_Extractor SHALL extract the estimated reading time from the page HTML (Medium includes this in the page, typically near the author byline or in a `<span>` with "min read")
2. THE Medium_Extractor SHALL populate the `PageGist.extra` JSON field with Medium-specific data:
   ```json
   {
     "publication": "Towards Data Science",
     "reading_time_minutes": 8
   }
   ```
3. WHEN reading time cannot be extracted, THE Medium_Extractor SHALL omit the `reading_time_minutes` field from extra (not set it to 0)
4. WHEN publication name is not present (personal blog, not in a publication), THE Medium_Extractor SHALL omit the `publication` field from extra

### Requirement 5: PageGist Return Format

**User Story:** As a system architect, I want the Medium extractor to return the same `PageGist` struct as all other extractors, so that the frontend doesn't need any changes.

#### Acceptance Criteria

1. THE Medium_Extractor SHALL return a `PageGist` with:
   - `url`: the original Medium article URL
   - `title`: extracted article title
   - `source_type`: `SourceType::Article`
   - `domain`: extracted domain (e.g. "medium.com")
   - `author`: extracted author name or `None`
   - `description`: OG description or `None`
   - `content_excerpt`: first ~500 chars of article body or `None`
   - `published_date`: formatted date string or `None`
   - `image_url`: OG image URL or `None`
   - `extra`: JSON with `publication` and `reading_time_minutes`
2. THE Medium_Extractor function signature SHALL be `pub async fn extract(url: &str, source_type: &SourceType, domain: &str) -> Result<PageGist, String>` (same as generic extractor)

### Requirement 6: Router Integration

**User Story:** As a developer, I want the Medium extractor to be wired into the existing router with minimal code changes, so that it's clean and easy to add more extractors in the future.

#### Acceptance Criteria

1. THE developer SHALL create a new file `src-tauri/src/browser/extractors/medium.rs` containing the Medium_Extractor
2. THE developer SHALL add `pub mod medium;` to `src-tauri/src/browser/extractors/mod.rs`
3. THE developer SHALL modify the `prepare_gist()` function in `extractors/mod.rs` to check if the URL domain contains "medium.com" and dispatch to `medium::extract()` before falling through to `generic::extract()`
4. THE routing logic SHALL be:
   ```rust
   match source_type {
       SourceType::YouTube => youtube_gist(url, &domain).await,
       _ if domain.contains("medium.com") => medium::extract(url, source_type, &domain).await,
       _ => generic::extract(url, source_type, &domain).await,
   }
   ```
5. NO changes SHALL be required to `commands.rs`, `lib.rs`, `tabs.rs`, `BrowserTool.tsx`, or `types.ts`
6. NO new dependencies SHALL be added to `Cargo.toml` — the extractor SHALL use `reqwest` (already present) and `regex` (already present)

### Requirement 7: Error Handling

**User Story:** As a user, I want the Medium extractor to handle errors gracefully, so that a closed tab or unavailable article doesn't crash the gist preparation.

#### Acceptance Criteria

1. WHEN the Chrome tab for the URL cannot be found (tab was closed), THE Medium_Extractor SHALL return a descriptive error message
2. WHEN the Medium page HTML cannot be parsed (unexpected structure), THE Medium_Extractor SHALL fall back to the Generic_Extractor's behavior (OG metadata + `<p>` tag extraction) rather than failing entirely
3. WHEN the AppleScript execution fails (Chrome not responding, permissions issue), THE Medium_Extractor SHALL return an error message suggesting the user check Chrome permissions
4. ALL error messages SHALL be user-friendly strings, not raw Rust error types

### Requirement 8: Unit Tests

**User Story:** As a developer, I want unit tests for the Medium extractor's parsing logic, so that changes to Medium's HTML structure are caught early.

#### Acceptance Criteria

1. THE developer SHALL write unit tests for title extraction from Medium HTML
2. THE developer SHALL write unit tests for author extraction from Medium HTML
3. THE developer SHALL write unit tests for content excerpt extraction and truncation
4. THE developer SHALL write unit tests for reading time extraction
5. THE developer SHALL write unit tests for HTML entity decoding in extracted text
6. THE developer SHALL write unit tests for handling missing/malformed metadata fields (fallback behavior)
7. ALL tests SHALL use inline HTML strings (not live network requests) for deterministic results
8. Tests SHALL be placed in a `#[cfg(test)] mod tests` block within `medium.rs`

## Files Summary

| Action | File | Description |
|--------|------|-------------|
| NEW | `src-tauri/src/browser/extractors/medium.rs` | Medium article extractor |
| MODIFY | `src-tauri/src/browser/extractors/mod.rs` | Add `pub mod medium;` + update router match arm |
| MODIFY | `src-tauri/src/browser/adapters/mod.rs` | Add `get_tab_html(url)` method to BrowserAdapter trait |
| MODIFY | `src-tauri/src/browser/adapters/chrome.rs` | Implement `get_tab_html()` using AppleScript + JavaScript execution |

## Implementation Notes

- The generic extractor (`generic.rs`) already has helper functions for OG metadata extraction, HTML title extraction, content excerpt extraction, and HTML entity decoding. The Medium extractor can reuse these or implement its own Medium-specific versions.
- Medium articles sometimes use custom domains (e.g. `betterprogramming.pub`, `towardsdatascience.com`). For MVP, only `medium.com` domain matching is required. Custom domain detection can be added later by checking for `<meta property="al:android:app_name" content="Medium">` in the HTML.
- **DOM Extraction Benefits**: By extracting from the browser DOM instead of HTTP fetching, we bypass Medium's paywall (user is already authenticated), get the actual rendered content (including lazy-loaded content if user scrolled), and avoid network errors/timeouts.
- **User Workflow**: User should scroll through the full Medium article first (Medium lazy-loads content), then click "Prepare Gist" to ensure all content is in the DOM.
- The Chrome adapter (`adapters/chrome.rs`) needs a new `get_tab_html(url: &str)` method that finds the matching tab and executes JavaScript to retrieve `document.documentElement.outerHTML` via AppleScript.
