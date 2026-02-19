# Plan: Browser Tool — See All Tabs, Gist Any Page

## Context

JARVIS can now hear (transcription) and partially see (YouTube detection in Chrome). The current observer only watches the **active tab** and only handles YouTube. The user wants to evolve this into a full **Browser Tool** — JARVIS's first real tool that can see ALL open tabs and create gists from any page, not just YouTube.

The system is designed to be **browser-extensible** — starting with Chrome, but the architecture cleanly separates browser-specific tab retrieval from the universal extraction/gist pipeline. Adding Safari, Arc, or Firefox later means implementing one new `BrowserAdapter` trait, not rewriting the tool.

**Why this matters for AWS AIdeas**: This transforms JARVIS from a single-trick observer into a general-purpose knowledge extractor. The plugin architecture (extractor pattern + browser adapter pattern) means we can keep adding intelligence and browser support without changing the core.

## Design

### Core Concept

```
User opens Browser Tool in JARVIS
  → JARVIS reads ALL open tabs via browser adapter (Chrome first)
  → Displays a scrollable list with: title, URL, domain, source type badge
  → User taps a tab → "Prepare Gist" button
  → JARVIS picks the right extractor based on URL:
      youtube.com  → YouTubeExtractor (existing)
      *            → GenericExtractor (new — OG metadata + content extraction)
      [future]     → MediumExtractor, GitHubExtractor, ArxivExtractor...
  → Gist displayed as a card: title, source, summary, key content
```

### UX Flow

**Entry point**: Hamburger menu (☰) → "Browser" (alongside existing "YouTube")

**Tab List View**:
```
┌─────────────────────────────────────────┐
│ Browser                              ×  │
│─────────────────────────────────────────│
│ 🔄 Refresh                    12 tabs   │
│─────────────────────────────────────────│
│ ▸ youtube.com                    [YT]   │
│   How to Build a Tauri App              │
│                                         │
│ ▸ medium.com                   [Article]│
│   Understanding Rust Ownership          │
│                                         │
│ ▸ github.com                    [Code]  │
│   anthropics/claude-code                │
│                                         │
│ ▸ docs.rs                      [Docs]   │
│   reqwest - Rust HTTP client            │
│                                         │
│ ▸ stackoverflow.com              [Q&A]  │
│   How to parse HTML in Rust?            │
│                                         │
│ ▸ news.ycombinator.com          [News]  │
│   Show HN: I built an AI desktop app   │
│─────────────────────────────────────────│
```

**After selecting a tab → Gist card**:
```
┌─────────────────────────────────────────┐
│ Gist: Understanding Rust Ownership      │
│ Source: medium.com · Article · 8 min    │
│─────────────────────────────────────────│
│ Title: Understanding Rust Ownership     │
│ Author: Jane Doe                        │
│ Published: 2025-12-15                   │
│                                         │
│ Summary:                                │
│ This article explains Rust's ownership  │
│ system including borrowing, lifetimes,  │
│ and the borrow checker...               │
│                                         │
│ [Copy]  [Dismiss]                       │
└─────────────────────────────────────────┘
```

### Architecture

**Key insight**: This is NOT a background polling feature. It's an **on-demand tool** — user opens it, JARVIS snapshots all tabs, user interacts.

```
┌──────────────────────────────────────────────────────┐
│ Frontend (React)                                      │
│                                                       │
│  BrowserTool.tsx                                      │
│    ├── onMount/onRefresh → invoke('list_browser_tabs')│
│    ├── Tab list display with source type badges       │
│    └── onPrepareGist → invoke('prepare_tab_gist')     │
│         └── Backend picks extractor by URL            │
└──────────────────┬───────────────────────────────────┘
                   │ Tauri commands
┌──────────────────▼───────────────────────────────────┐
│ Backend (Rust)                                        │
│                                                       │
│  browser/adapters/mod.rs  ← browser-agnostic trait    │
│    ├── trait BrowserAdapter { list_tabs() }            │
│    └── chrome.rs  ← Chrome implementation (first)     │
│         └── AppleScript: every tab of every window    │
│                                                       │
│  browser/tabs.rs                                      │
│    ├── list_all_tabs() → picks active adapter         │
│    └── classify_url(url) → SourceType enum            │
│                                                       │
│  browser/extractors/mod.rs  ← URL-agnostic pipeline   │
│    ├── prepare_gist() → routes by SourceType          │
│    ├── GenericExtractor  (OG metadata + content)      │
│    └── YouTubeExtractor  (existing youtube.rs)        │
│                                                       │
│  browser/extractors/generic.rs                        │
│    ├── Fetch HTML with reqwest                        │
│    ├── Extract OG metadata (title, description, img)  │
│    ├── Extract article content via scraper CSS select  │
│    └── Return PageGist { title, author, summary... }  │
└──────────────────────────────────────────────────────┘
```

**Extensibility layers**:
- **New browser**: Add `adapters/safari.rs` implementing `BrowserAdapter` — tabs.rs picks it based on config
- **New extractor**: Add `extractors/github.rs` — mod.rs routes `SourceType::Code` to it
- Both can be added without touching existing code

## Implementation

### Backend Changes

**1. New: `src-tauri/src/browser/adapters/mod.rs`** — Browser adapter trait

```rust
pub mod chrome;

use serde::{Serialize, Deserialize};

/// Raw tab info from a browser — browser-agnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTab {
    pub url: String,
    pub title: String,
}

/// Trait for browser-specific tab retrieval
/// Implement this for each browser (Chrome, Safari, Arc, Firefox...)
pub trait BrowserAdapter {
    /// Human-readable browser name
    fn name(&self) -> &str;
    /// Check if this browser is running/available
    fn is_available(&self) -> bool;
    /// Get all open tabs across all windows
    async fn list_tabs(&self) -> Result<Vec<RawTab>, String>;
}
```

**2. New: `src-tauri/src/browser/adapters/chrome.rs`** — Chrome adapter

AppleScript to get ALL tabs from ALL windows:
```applescript
tell application "Google Chrome"
  set tabInfo to ""
  set windowCount to count of windows
  repeat with w from 1 to windowCount
    set tabCount to count of tabs in window w
    repeat with t from 1 to tabCount
      set tabURL to URL of tab t of window w
      set tabTitle to title of tab t of window w
      set tabInfo to tabInfo & tabURL & "|||" & tabTitle & "\n"
    end repeat
  end repeat
  return tabInfo
end tell
```

Implements `BrowserAdapter` for Chrome. Parses `|||`-delimited output into `Vec<RawTab>`.

**3. New: `src-tauri/src/browser/tabs.rs`** — Tab listing + classification

Structs:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTab {
    pub url: String,
    pub title: String,
    pub source_type: SourceType,  // classified from URL
    pub domain: String,           // extracted from URL (e.g. "medium.com")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    YouTube,
    Article,    // medium.com, substack.com, dev.to, blog-like
    Code,       // github.com, gitlab.com
    Docs,       // docs.rs, developer.*, *.readthedocs.io
    QA,         // stackoverflow.com, stackexchange
    News,       // news.ycombinator.com, reddit.com
    Research,   // arxiv.org, scholar.google.com
    Social,     // twitter/x.com, linkedin.com
    Other,      // everything else
}
```

Functions:
- `list_all_tabs() -> Result<Vec<BrowserTab>, String>` — uses ChromeAdapter (for now), enriches RawTabs with classification
- `classify_url(url: &str) -> SourceType` — domain matching
- `extract_domain(url: &str) -> String` — parse domain from URL

**4. New: `src-tauri/src/browser/extractors/mod.rs`** — Extractor router + PageGist type

```rust
pub mod generic;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageGist {
    pub url: String,
    pub title: String,
    pub source_type: SourceType,
    pub domain: String,
    pub author: Option<String>,
    pub description: Option<String>,   // OG description or meta description
    pub content_excerpt: Option<String>, // first ~500 chars of article body
    pub published_date: Option<String>,
    pub image_url: Option<String>,      // OG image
    pub extra: serde_json::Value,       // source-specific fields (e.g. duration for YT)
}

/// Route URL to the right extractor and produce a gist
pub async fn prepare_gist(url: &str, source_type: &SourceType) -> Result<PageGist, String> {
    match source_type {
        SourceType::YouTube => youtube_gist(url).await,  // wraps existing scrape_youtube_gist
        _ => generic::extract(url).await,                 // generic for everything else
    }
}
```

**5. New: `src-tauri/src/browser/extractors/generic.rs`** — Generic page extractor

Uses `reqwest` (already in deps) + `scraper` crate (new dep) for HTML parsing:
- Fetch page HTML
- Extract Open Graph metadata: `og:title`, `og:description`, `og:image`, `og:type`
- Extract `<meta name="author">`, `<meta name="description">`
- Extract article body using `scraper` DOM parsing (find `<article>`, `<main>`, or largest `<div>`)
- Truncate content to ~500 chars for the excerpt
- Return `PageGist`

**Why `scraper` over `readability-rust`**: The `scraper` crate (CSS selector-based DOM parsing) is lighter, more established (40M+ downloads), and sufficient for metadata + basic content extraction. `readability-rust` is heavier and we don't need full Reader Mode — just enough for a gist. We can upgrade later if needed.

**6. Modify: `src-tauri/Cargo.toml`** — Add scraper dependency
```toml
scraper = "0.22"  # CSS selector-based HTML parsing
```

**7. Modify: `src-tauri/src/browser/mod.rs`** — Add new modules
```rust
pub mod observer;
pub mod youtube;
pub mod tabs;        // NEW
pub mod adapters;    // NEW
pub mod extractors;  // NEW
```

**8. Modify: `src-tauri/src/commands.rs`** — 2 new commands
```rust
#[tauri::command]
pub async fn list_browser_tabs() -> Result<Vec<BrowserTab>, String> {
    crate::browser::tabs::list_all_tabs().await
}

#[tauri::command]
pub async fn prepare_tab_gist(url: String, source_type: String) -> Result<PageGist, String> {
    let st = serde_json::from_str(&format!("\"{}\"", source_type))
        .unwrap_or(SourceType::Other);
    crate::browser::extractors::prepare_gist(&url, &st).await
}
```

**9. Modify: `src-tauri/src/lib.rs`** — Register new commands
Add `commands::list_browser_tabs` and `commands::prepare_tab_gist` to invoke_handler.

### Frontend Changes

**10. New: `src/components/BrowserTool.tsx`** — Main component

```
BrowserTool
├── State: tabs[], selectedTab, gist, loading, error
├── onMount: invoke('list_browser_tabs') → populate tabs list
├── Refresh button: re-invoke list_browser_tabs
├── Tab list: scrollable, each tab shows title + domain + source badge
├── Tab click: select tab, show "Prepare Gist" button
├── Prepare Gist: invoke('prepare_tab_gist') → display gist card
├── Copy: copy gist to clipboard
└── Dismiss: clear gist, deselect tab
```

Source type badges (colored chips):
- `[YT]` red — YouTube
- `[Article]` blue — Medium, Substack, blogs
- `[Code]` green — GitHub, GitLab
- `[Docs]` purple — documentation sites
- `[Q&A]` orange — StackOverflow
- `[News]` teal — HN, Reddit
- `[Research]` indigo — arXiv
- `[Other]` gray — everything else

**11. Modify: `src/state/types.ts`** — Add new types
```typescript
export type SourceType = 'YouTube' | 'Article' | 'Code' | 'Docs' | 'QA' | 'News' | 'Research' | 'Social' | 'Other';

export interface BrowserTab {
  url: string;
  title: string;
  source_type: SourceType;
  domain: string;
}

export interface PageGist {
  url: string;
  title: string;
  source_type: SourceType;
  domain: string;
  author: string | null;
  description: string | null;
  content_excerpt: string | null;
  published_date: string | null;
  image_url: string | null;
  extra: Record<string, unknown>;
}
```

**12. Modify: `src/App.tsx`** — Add to hamburger menu
- Add `showBrowserTool` state
- Add "Browser" menu item
- Render `BrowserTool` in dialog-overlay

**13. Modify: `src/App.css`** — Browser tool styling
- `.tab-item` — list row with domain, title, badge
- `.source-badge` — colored chip for source type
- `.source-badge.youtube`, `.source-badge.article`, etc.
- `.page-gist` — gist card display (reuse `.gist-display` pattern)

### Files Summary

| Action | File | Description |
|--------|------|-------------|
| NEW | `src-tauri/src/browser/adapters/mod.rs` | BrowserAdapter trait |
| NEW | `src-tauri/src/browser/adapters/chrome.rs` | Chrome AppleScript implementation |
| NEW | `src-tauri/src/browser/tabs.rs` | Tab listing + URL classifier |
| NEW | `src-tauri/src/browser/extractors/mod.rs` | Extractor router + PageGist type |
| NEW | `src-tauri/src/browser/extractors/generic.rs` | Generic page gist extractor |
| NEW | `src/components/BrowserTool.tsx` | Frontend tab list + gist UI |
| MODIFY | `src-tauri/Cargo.toml` | Add `scraper` dep |
| MODIFY | `src-tauri/src/browser/mod.rs` | Add `tabs`, `adapters`, `extractors` modules |
| MODIFY | `src-tauri/src/commands.rs` | Add `list_browser_tabs` + `prepare_tab_gist` |
| MODIFY | `src-tauri/src/lib.rs` | Register new commands |
| MODIFY | `src/state/types.ts` | Add `BrowserTab`, `PageGist`, `SourceType` |
| MODIFY | `src/App.tsx` | Add Browser to hamburger menu |
| MODIFY | `src/App.css` | Tab list + gist card styles |

## Implementation Order

1. **Backend: `adapters/`** — BrowserAdapter trait + Chrome implementation (can test independently)
2. **Backend: `tabs.rs`** — Tab listing with classification (uses adapter)
3. **Backend: `extractors/generic.rs`** — Generic page extractor (can test with any URL)
4. **Backend: `extractors/mod.rs`** — Router that dispatches YouTube vs generic
5. **Backend: Wire up** — Cargo.toml, mod.rs, commands.rs, lib.rs
6. **Frontend: Types** — types.ts
7. **Frontend: `BrowserTool.tsx`** — Component with tab list + gist display
8. **Frontend: Integration** — App.tsx menu + App.css styles
9. **Test end-to-end**: Open Chrome with multiple tabs → JARVIS shows them all → gist a page

## Key Design Decisions

1. **On-demand, not polling** — Unlike the YouTube observer (background polling), the browser tool is invoked by the user. No wasted CPU when not in use.

2. **Two extensibility axes** — `BrowserAdapter` for new browsers, `extractors` for new site types. Both are additive (new files, no existing code changes).

3. **Unified `PageGist` type** — All extractors return the same `PageGist` struct. YouTube puts duration in `extra`. The frontend only needs one gist display component.

4. **`scraper` over `readability-rust`** — Lighter dependency, we only need metadata + excerpt, not full Reader Mode. Easy to swap later.

5. **Source classification by domain** — Simple string matching on URL domains. Fast, no network calls needed. Can be extended trivially.

6. **Existing YouTube extractor reused** — `scrape_youtube_gist()` already works. The router just wraps it into a `PageGist`.

7. **Browser-agnostic naming** — Types are `BrowserTab`, `BrowserAdapter`, `BrowserTool`. No "Chrome" in public API. Chrome is an implementation detail in `adapters/chrome.rs`.

## Verification

1. `cargo build` — compiles with new `scraper` dep
2. `cargo test` — classifier + adapter tests pass
3. Launch app → hamburger → "Browser" → list of all open Chrome tabs appears
4. Click any non-YouTube tab → "Prepare Gist" → gist card with title, author, excerpt
5. Click a YouTube tab → "Prepare Gist" → existing YouTube gist with channel, duration
6. Copy button works → clipboard contains formatted gist
7. Refresh button → re-fetches tab list
