# Implementation Plan: Medium Article Extractor

## Overview

This plan implements a specialized Medium article extractor for the JARVIS Browser Tool. The extractor understands Medium's HTML structure to produce richer gists than the generic extractor. Implementation follows the established pattern from the YouTube extractor: regex-based metadata extraction, graceful fallbacks, and integration into the existing router.

## Tasks

- [ ] 0. Extend Chrome adapter with DOM extraction capability
  - [ ] 0.1 Add `get_tab_html()` method to BrowserAdapter trait
    - Open `src-tauri/src/browser/adapters/mod.rs`
    - Add async method signature to `BrowserAdapter` trait: `async fn get_tab_html(&self, url: &str) -> Result<String, String>`
    - _Requirements: 2.1, Design - Chrome Adapter Extension_
  
  - [ ] 0.2 Implement `get_tab_html()` in Chrome adapter using secure two-step AppleScript
    - Open `src-tauri/src/browser/adapters/chrome.rs`
    - Implement `get_tab_html()` method using secure two-step approach:
      - Step 1: Call `list_tabs()` to get all tab URLs with window/tab indices
      - Step 2: Find matching URL in Rust (exact match)
      - Step 3: Execute AppleScript with numeric indices only (no URL interpolation): `tell application "Google Chrome" to execute tab {tab_index} of window {window_index} javascript "document.documentElement.outerHTML"`
    - Add size check: reject HTML > 5MB with error message
    - Handle errors: tab not found, Chrome not responding, AppleScript execution failure
    - _Requirements: 2.1, 2.2, 7.1, 7.2, 7.3, Design - Chrome Adapter Extension_
    - _Note: This secure approach prevents AppleScript injection attacks by never interpolating URLs into AppleScript strings_

- [x] 1. Expose generic extractor utilities for reuse
  - [x] 1.1 Move regex patterns to module level in `generic.rs`
    - Move `ARTICLE_REGEX` from inside `extract_content_excerpt()` (currently ~line 136) to module-level static
    - Move `TAG_REGEX` from inside `extract_content_excerpt()` (currently ~line 160) to module-level static
    - Update `extract_content_excerpt()` function body to reference the module-level `ARTICLE_REGEX` and `TAG_REGEX` instead of local statics
    - _Note: This refactor is required because Rust doesn't allow visibility modifiers on function-local statics_
  
  - [x] 1.2 Make utilities `pub(crate)` in `generic.rs`
    - Change helper functions from `fn` to `pub(crate) fn`: `extract_og_content`, `extract_meta_content`, `extract_html_title`, `decode_html_entities`
    - Change the moved regex patterns from `static` to `pub(crate) static`: `ARTICLE_REGEX`, `TAG_REGEX`
    - _Requirements: Design - Reusing Generic Extractor Utilities_

- [ ] 2. Create Medium extractor module with core extraction logic
  - [x] 2.1 Create `medium.rs` with module structure and imports
    - Create file `src-tauri/src/browser/extractors/medium.rs`
    - Import utilities from `generic.rs` and required dependencies
    - Import `ChromeAppleScriptAdapter` from `crate::browser::adapters::chrome`
    - _Requirements: 6.1_
    - _Note: No USER_AGENT constant needed - we extract from browser DOM, not HTTP_
  
  - [x] 2.2 Implement reading time extraction
    - Define `READING_TIME_REGEX` pattern using `LazyLock`
    - Implement `extract_reading_time(html: &str) -> Option<u32>` function
    - _Requirements: 4.1_
  
  - [ ]* 2.3 Write property test for reading time extraction
    - **Property 7: Reading Time Extraction**
    - **Validates: Requirements 4.1**
  
  - [x] 2.4 Implement main `extract()` function
    - Create async function with signature `pub async fn extract(url: &str, source_type: &SourceType, domain: &str) -> Result<PageGist, String>`
    - Get HTML from browser DOM: `let html = ChromeAppleScriptAdapter::new().get_tab_html(url).await?;`
    - Extract metadata: title, author, publication, date, image, description
    - Extract article body from `<article>` tags using `ARTICLE_REGEX`
    - Strip HTML tags using `TAG_REGEX`
    - Truncate content to ~500 chars at word boundary with "..."
    - Decode HTML entities in extracted text
    - Populate `PageGist` with Medium-specific `extra` fields (publication, reading_time_minutes)
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 3.1, 3.2, 3.4, 3.5, 3.6, 4.2, 4.3, 4.4, 5.1_
    - _Note: This replaces the previous reqwest HTTP fetching approach (lines 38-64) with DOM extraction via Chrome adapter_
  
  - [ ]* 2.5 Write unit tests for metadata extraction
    - Test title extraction from OG metadata and `<h1>` fallback
    - Test author extraction from meta tags
    - Test publication extraction from `og:site_name`
    - Test date extraction and formatting
    - Test missing metadata fallback behavior
    - _Requirements: 8.1, 8.2, 8.6_
  
  - [ ]* 2.6 Write property test for metadata extraction with fallbacks
    - **Property 2: Metadata Extraction with Fallbacks**
    - **Validates: Requirements 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9**
  
  - [ ]* 2.7 Write unit tests for content extraction
    - Test article body extraction from `<article>` tags
    - Test HTML tag stripping
    - Test content truncation at word boundary
    - Test HTML entity decoding
    - _Requirements: 8.3, 8.4, 8.5_
  
  - [ ]* 2.8 Write property tests for content processing
    - **Property 4: HTML Tag Stripping**
    - **Validates: Requirements 3.2**
    - **Property 5: Content Truncation at Word Boundary**
    - **Validates: Requirements 3.4, 3.5**
    - **Property 6: HTML Entity Decoding**
    - **Validates: Requirements 3.6**
    - **Property 12: Text Truncation Preserves Content**
    - **Validates: Requirements 3.4 (edge case)**
  
  - [ ]* 2.9 Write property test for extra field population
    - **Property 8: Extra Field Population**
    - **Validates: Requirements 4.2, 4.3, 4.4**
  
  - [ ]* 2.10 Write property test for PageGist structure
    - **Property 9: PageGist Structure Completeness**
    - **Validates: Requirements 5.1**

- [ ] 3. Integrate Medium extractor into router
  - [x] 3.1 Add module declaration to `mod.rs`
    - Add `pub mod medium;` to `src-tauri/src/browser/extractors/mod.rs`
    - _Requirements: 6.2_
  
  - [x] 3.2 Update router dispatch logic
    - Modify `prepare_gist()` function in `mod.rs`
    - Add Medium domain check: `_ if domain.contains("medium.com") => medium::extract(url, source_type, &domain).await`
    - Place check before generic fallback, after YouTube check
    - _Requirements: 1.1, 1.2, 1.4, 6.3, 6.4_
  
  - [ ]* 3.3 Write property test for Medium URL routing
    - **Property 1: Medium URL Routing**
    - **Validates: Requirements 1.1, 1.2, 1.4**

- [ ] 4. Implement error handling and fallbacks
  - [ ] 4.1 Add browser/tab error handling
    - Handle tab not found errors with descriptive error messages
    - Handle Chrome not responding errors
    - Handle AppleScript execution failures
    - Handle HTML size limit exceeded (> 5MB)
    - Return user-friendly error strings
    - _Requirements: 7.1, 7.3, 7.4_
    - _Note: This replaces HTTP error handling with browser/tab error handling for DOM extraction approach_
  
  - [x] 4.2 Add malformed HTML fallback
    - When Medium-specific extraction fails, fall back to OG metadata extraction
    - Ensure partial PageGist is returned rather than complete failure
    - _Requirements: 7.2_
  
  - [ ]* 4.3 Write property test for browser/tab error handling
    - **Property 10: Browser/Tab Error Handling**
    - **Validates: Requirements 7.1, 7.3**
  
  - [ ]* 4.4 Write property test for malformed HTML fallback
    - **Property 11: Malformed HTML Fallback**
    - **Validates: Requirements 7.2**
  
  - [ ]* 4.5 Write unit tests for error conditions
    - Test handling of missing article body
    - Test handling of paywalled content
    - Test handling of empty metadata fields
    - _Requirements: 8.6_

- [ ] 5. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- The Medium extractor reuses utilities from `generic.rs` to avoid code duplication
- Property tests use `proptest` crate (already in Cargo.toml) with minimum 100 iterations
- DOM extraction approach: extracts HTML from browser tab via AppleScript instead of HTTP fetching
- Security: Two-step AppleScript approach prevents injection attacks by using numeric indices only
- No new dependencies required - uses existing `regex`, `serde_json`, and Chrome adapter
- No frontend changes needed - existing `BrowserTool.tsx` renders `PageGist` data
