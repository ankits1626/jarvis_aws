# Requirements Document

## Introduction

This spec adds a `claude_extension` extractor to the Jarvis browser module. Unlike existing extractors (YouTube, ChatGPT, Gmail, Medium) that scrape web page DOM via JavaScript execution, this extractor reads the **Claude Chrome Extension side panel** using the macOS Accessibility API (AXUIElement). This is necessary because the side panel is a separate browser context — its DOM is inaccessible from AppleScript JavaScript execution or content scripts.

### Problem

When a user opens a web page (e.g., a Medium article, a GitHub repo, a Netflix tech blog post) and starts a conversation with the Claude Chrome Extension about that page, two valuable pieces of knowledge are created:

1. **The page content** — what the user was reading
2. **The Claude conversation** — the user's questions and Claude's analysis/explanations about that page

Today, Jarvis can capture the page content (via existing extractors) but has no way to capture the Claude conversation. The Claude Chrome Extension side panel conversations are:
- **Not synced** to claude.ai — they exist only in the extension's side panel
- **Not accessible** via AppleScript JavaScript execution — the side panel is a separate browsing context
- **Not stored** in any accessible local storage — conversations flow through Anthropic's API

However, through our prototype exploration (`exploration/ax_chrome_explorer.swift`), we confirmed that **macOS Accessibility APIs can read the side panel content**. Chrome exposes the Claude side panel as a separate `AXWebArea` element with title "Claude for Chrome" in its accessibility tree. All conversation text (user prompts, Claude responses, headings, formatted text) is fully readable.

### Solution

Add a `claude_extension` extractor that:
1. Uses macOS AXUIElement APIs to find Chrome's "Claude for Chrome" web area
2. Extracts the full conversation text (user prompts + Claude responses)
3. Pairs it with the active tab's URL and page title
4. Returns a `PageGist` with `source_type: Chat` and the conversation as content, with both page context and conversation metadata in `extra`

This extractor follows the same pattern as `chatgpt.rs`, `gmail.rs`, and `medium.rs` — it implements an `extract()` function that returns a `PageGist`. The key difference is the extraction mechanism: AXUIElement instead of `execute_js_in_tab`.

### Proof of Concept

The exploration script at `exploration/ax_chrome_explorer.swift` demonstrated:
- **WebArea #0**: Active tab content (e.g., "MediaFM: The Multimodal AI Foundation for Media Understanding at Netflix")
- **WebArea #1**: Claude side panel with title "Claude for Chrome"
- **841 text blocks** extracted from a single conversation
- **User prompt** visible as the first text block
- **Claude's full response** with headings, paragraphs, and formatted text
- **Input field marker**: `[input: Reply to Claude]` identifies the end of conversation
- **Footer marker**: `"Claude is AI and can make mistakes. Please double-check responses."` identifies Claude's content

## Glossary

- **Claude Chrome Extension**: Anthropic's official Chrome extension (ID: `fcoeoabgfenejglbffodgkkbkcdhcgfn`) that provides a side panel for conversing with Claude about any web page. Requires a paid Claude plan.
- **Side Panel**: Chrome's `chrome.sidePanel` API renders the Claude conversation UI in a persistent right-hand sidebar. This is a separate browsing context from the web page — its DOM is inaccessible from page-level JavaScript.
- **AXUIElement**: macOS Accessibility framework's core type for interacting with UI elements. Allows reading the accessibility tree of any application, including Chrome's side panel content.
- **AXWebArea**: The accessibility role for a web content area in Chrome. Each tab and each extension page (including the side panel) appears as a separate AXWebArea in Chrome's accessibility tree.
- **Accessibility Permission**: macOS requires explicit user consent (System Settings > Privacy & Security > Accessibility) before an app can use AXUIElement APIs to read other apps' UI. The Jarvis app must be granted this permission.
- **Text Block**: A single text element in the accessibility tree. Conversation text is extracted as a sequence of text blocks (AXStaticText, AXHeading, AXLink, AXTextField roles).
- **PageGist**: The unified gist type returned by all extractors (`browser/extractors/mod.rs`). Contains URL, title, source_type, content, author, and extra metadata.
- **Claude Extension Conversation**: A sequence of user prompts and Claude responses within the side panel, associated with whatever web page the user was viewing. These conversations are NOT synced to claude.ai.

## Requirements

### Requirement 1: macOS Accessibility API Integration

**User Story:** As a developer, I want a Rust module that can read Chrome's accessibility tree, so that I can extract text content from the Claude Chrome Extension side panel.

#### Acceptance Criteria

1. THE System SHALL provide a Rust module (`browser/accessibility.rs` or similar) that wraps macOS AXUIElement APIs for reading Chrome's accessibility tree
2. THE module SHALL find Chrome's process ID from running applications
3. THE module SHALL traverse Chrome's accessibility tree to find all AXWebArea elements
4. THE module SHALL extract text content from any AXWebArea by recursively reading AXStaticText, AXHeading, AXLink, and AXTextField elements
5. THE module SHALL use manual FFI with `core-foundation` and `core-graphics` crates for AXUIElement bindings (NOT the unmaintained `accessibility-sys` crate)
6. THE module SHALL handle the case where Chrome is not running (return descriptive error)
7. THE module SHALL handle the case where Accessibility permission is not granted (return descriptive error with instructions)
8. THE module SHALL be gated behind `#[cfg(target_os = "macos")]` since AXUIElement is macOS-only

### Requirement 2: Claude Side Panel Detection

**User Story:** As a user, I want Jarvis to detect when the Claude Chrome Extension side panel is open with a conversation, so that I can capture the conversation.

#### Acceptance Criteria

1. THE System SHALL identify the Claude side panel by finding an AXWebArea with title containing "Claude" (currently "Claude for Chrome")
2. THE System SHALL distinguish the Claude side panel AXWebArea from regular web page AXWebAreas (the side panel is a sibling, not a child, of the tab's web area)
3. THE System SHALL return a boolean indicating whether a Claude conversation is currently visible
4. THE System SHALL handle the case where the side panel is closed (no Claude AXWebArea found)
5. THE System SHALL handle the case where the side panel is open but empty (new conversation with no messages)

### Requirement 3: Conversation Text Extraction

**User Story:** As a user, I want Jarvis to extract the full conversation (my questions and Claude's responses) from the side panel, so that the knowledge is preserved in my gem store.

#### Acceptance Criteria

1. THE System SHALL extract all text blocks from the Claude side panel's AXWebArea
2. THE System SHALL reconstruct the conversation as structured text with clear separation between user messages and Claude responses
3. THE System SHALL preserve heading structure (AXHeading elements → `## heading`)
4. THE System SHALL preserve link text (AXLink elements → `[link: text]`)
5. THE System SHALL detect the conversation boundary: text blocks before the input field marker (`Reply to Claude` placeholder) are conversation content; text after is UI chrome
6. THE System SHALL handle conversations with multiple turns (multiple user prompts and Claude responses)
7. THE System SHALL truncate conversations longer than 50,000 characters (consistent with ChatGPT extractor limit) with a `[conversation truncated]` marker
8. THE System SHALL count the number of message turns in the conversation for metadata

### Requirement 4: Page Context Extraction

**User Story:** As a user, I want the gem to include what page I was reading when I had the Claude conversation, so that I have full context.

#### Acceptance Criteria

1. THE System SHALL extract the active tab's URL using the existing `ChromeAppleScriptAdapter` (AppleScript `URL of active tab`)
2. THE System SHALL extract the active tab's page title from the primary AXWebArea (non-Claude web area) title attribute
3. THE System SHALL include the page URL and title as metadata in the gem's `extra` field
4. THE System SHALL handle multi-window Chrome scenarios by reading from the frontmost window

### Requirement 5: PageGist Output Format

**User Story:** As a developer, I want the claude_extension extractor to return a standard PageGist, so that it integrates seamlessly with the existing gem save pipeline.

#### Acceptance Criteria

1. THE extractor SHALL return a `PageGist` struct matching the existing extractor interface
2. THE `source_type` SHALL be `SourceType::Chat` (same as ChatGPT, since both are AI chat conversations)
3. THE `title` SHALL be the Claude conversation's context — formatted as "Claude: {page_title}" (e.g., "Claude: MediaFM — The Multimodal AI Foundation for Netflix")
4. THE `url` SHALL be the active tab's URL (the page the conversation is about)
5. THE `domain` SHALL be extracted from the active tab's URL (not "claude.ai")
6. THE `content_excerpt` SHALL contain the full conversation text (user prompts + Claude responses, structured with `--- You ---` and `--- Claude ---` separators, matching the ChatGPT extractor pattern)
7. THE `author` SHALL be `Some("Claude Extension")` to distinguish from ChatGPT conversations
8. THE `description` SHALL be a brief summary: first user prompt truncated to 200 characters
9. THE `extra` field SHALL include:
   - `page_url`: the URL of the page the conversation is about
   - `page_title`: the title of the page
   - `message_count`: number of conversation turns
   - `extraction_method`: `"accessibility_api"` (to distinguish from DOM-based extractors)
   - `claude_extension_version`: title of the Claude AXWebArea (e.g., "Claude for Chrome") for future compatibility

### Requirement 6: Extractor Integration

**User Story:** As a user, I want to capture a Claude conversation as a gem through the existing browser tool UI, so that the workflow is familiar.

#### Acceptance Criteria

1. THE `claude_extension` module SHALL be added to `browser/extractors/mod.rs` as `pub mod claude_extension`
2. THE System SHALL expose a new Tauri command `capture_claude_conversation` that triggers the claude_extension extractor on demand (NOTE: `prepare_gist` will NOT be updated to auto-detect Claude conversations due to the performance cost of accessibility tree traversal on the hot path)
3. THE command SHALL return a `PageGist` on success, or a descriptive error if:
   - Chrome is not running
   - Accessibility permission is not granted
   - Claude side panel is not open
   - Side panel has no conversation (empty)
4. THE System SHALL NOT automatically trigger on URL changes (unlike YouTube detection) — Claude conversation capture is user-initiated only
5. THE command SHALL be registered in `lib.rs` invoke_handler alongside existing commands

### Requirement 7: Permission Handling

**User Story:** As a user, I want clear feedback when Accessibility permission is needed, so that I know how to grant it.

#### Acceptance Criteria

1. THE System SHALL check Accessibility permission status before attempting to read Chrome's accessibility tree
2. THE System SHALL expose a Tauri command `check_accessibility_permission` that returns whether the permission is granted
3. IF permission is not granted, THE System SHALL return an error message with instructions: "Accessibility permission required. Go to System Settings > Privacy & Security > Accessibility and add Jarvis."
4. THE System SHALL NOT prompt the macOS permission dialog automatically (use `kAXTrustedCheckOptionPrompt: false`) — the user should be guided by Jarvis's UI instead
5. THE permission check SHALL be lightweight and fast (no accessibility tree traversal needed — just `AXIsProcessTrusted()`)

### Requirement 8: Frontend Integration

**User Story:** As a user, I want a button in the Browser Tool to capture my current Claude conversation, so that saving it as a gem is one click.

#### Acceptance Criteria

1. THE Browser Tool panel SHALL display a "Capture Claude Conversation" button when Chrome is the active browser
2. THE button SHALL be disabled with a tooltip when:
   - Accessibility permission is not granted (tooltip: "Accessibility permission required")
   - Claude side panel is not detected (tooltip: "No Claude conversation found")
3. THE button SHALL show a loading state while extraction is in progress
4. ON successful extraction, THE System SHALL display the captured conversation as a GistCard (same as existing extractors) with:
   - Title showing "Claude: {page_title}"
   - Content preview showing the first user prompt
   - "Save Gem" button to persist
5. THE GistCard SHALL show the AI enrichment notice if IntelligenceKit is available (consistent with existing GistCards)
6. ON error, THE System SHALL display the error message inline (consistent with existing error handling in BrowserTool)

### Requirement 9: Error Handling and Edge Cases

**User Story:** As a user, I want the feature to handle edge cases gracefully without crashing or showing cryptic errors.

#### Acceptance Criteria

1. THE System SHALL handle Chrome not running: return error "Chrome is not running"
2. THE System SHALL handle Chrome running but no Claude side panel: return error "No Claude conversation found. Open the Claude Chrome Extension side panel first."
3. THE System SHALL handle Claude side panel open but empty (no messages): return error "Claude conversation is empty"
4. THE System SHALL handle Accessibility permission not granted: return error with grant instructions
5. THE System SHALL handle Chrome windows with no tabs (edge case): return error "No active tab found"
6. THE System SHALL NOT crash or panic on any accessibility API failure — all errors are returned as `Result::Err(String)`
7. THE System SHALL log accessibility API errors to stderr with `[ClaudeExtractor]` prefix for debugging
8. THE System SHALL handle the case where Chrome's accessibility tree is not fully populated (e.g., Chrome just launched): retry once after a short delay, then return error

### Requirement 10: Platform Constraints

**User Story:** As a developer, I want the feature to compile and degrade gracefully on non-macOS platforms.

#### Acceptance Criteria

1. ALL AXUIElement code SHALL be gated behind `#[cfg(target_os = "macos")]`
2. ON non-macOS platforms, the `capture_claude_conversation` command SHALL return error "Claude conversation capture is only available on macOS"
3. ON non-macOS platforms, the `check_accessibility_permission` command SHALL return `false`
4. THE feature SHALL NOT prevent compilation on Linux or Windows
5. THE `accessibility-sys` crate (or equivalent) SHALL be a macOS-only dependency in `Cargo.toml` using `[target.'cfg(target_os = "macos")'.dependencies]`

### Requirement 11: Backwards Compatibility

**User Story:** As a user, I want all existing features to continue working unchanged after this addition.

#### Acceptance Criteria

1. THE existing extractors (YouTube, ChatGPT, Gmail, Medium, Generic) SHALL NOT be modified
2. THE existing `prepare_gist` function SHALL continue to work for all existing source types
3. THE `capture_claude_conversation` command SHALL be independent of `prepare_tab_gist` — it's a separate extraction path
4. ALL existing tests SHALL continue to pass
5. THE gem save pipeline (including AI enrichment) SHALL work with Claude conversation gems the same as any other gem

## Technical Context

### Existing Architecture

The browser module (`src-tauri/src/browser/`) has:
- `adapters/chrome.rs` — ChromeAppleScriptAdapter with `execute_js_in_tab`, `get_tab_html`, `get_active_tab_url`
- `extractors/` — modular extractors (chatgpt.rs, gmail.rs, medium.rs, generic.rs) each with an `extract()` function
- `extractors/mod.rs` — PageGist struct and `prepare_gist` router
- `tabs.rs` — SourceType enum and `classify_url` function
- `observer.rs` — BrowserObserver that polls Chrome

### Accessibility API Exploration

The prototype at `exploration/ax_chrome_explorer.swift` confirmed:
- Chrome PID: discoverable via `NSWorkspace.shared.runningApplications`
- AXWebArea discovery: recursive tree traversal finds all web areas
- Claude side panel: appears as `AXWebArea` with title "Claude for Chrome"
- Text extraction: AXStaticText, AXHeading, AXLink roles contain all conversation text
- Conversation structure: user prompt is first text block, Claude response follows, input field placeholder "Reply to Claude" marks end
- Footer: "Claude is AI and can make mistakes. Please double-check responses." marks end of content

### Rust Crate Options for AXUIElement

1. **`accessibility-sys`** — Low-level FFI bindings to macOS Accessibility framework. Most direct.
2. **`accessibility`** — Higher-level wrapper around `accessibility-sys`. Provides `AXUIElement` type with methods like `attribute`, `children`, `role`.
3. **Raw FFI** — Use `core-foundation-sys` + manual `extern "C"` declarations for `AXUIElementCreateApplication`, `AXUIElementCopyAttributeValue`, etc.

Recommended: `accessibility` crate for cleaner Rust API, with `accessibility-sys` as fallback if the higher-level crate is insufficient.

### Key AXUIElement Constants

```
kAXRoleAttribute = "AXRole"
kAXTitleAttribute = "AXTitle"
kAXValueAttribute = "AXValue"
kAXChildrenAttribute = "AXChildren"
kAXDescriptionAttribute = "AXDescription"
kAXRoleDescriptionAttribute = "AXRoleDescription"
kAXPlaceholderValueAttribute = "AXPlaceholderValue"
```

### Key Roles for Text Extraction

| AX Role | Content | Example |
|---------|---------|---------|
| AXStaticText | Body text, paragraphs | "MediaFM is Netflix's first..." |
| AXHeading | Section headings | "## 1. Fundamentals" |
| AXLink | Hyperlinks | "[link: wav2vec2]" |
| AXTextField | Input fields | "[input: Reply to Claude]" |

### Conversation Structure (from prototype)

```
Block 0:   User prompt ("help me understand this...")
Block 1-5: Claude's plan steps ("2 steps", "Created a plan", "Extract page text", "Done")
Block 6+:  Claude's response (headings, paragraphs, formatted text)
...
Block N-2: Input field placeholder ("Reply to Claude")
Block N-1: Input field value
Block N:   Footer ("Claude is AI and can make mistakes...")
```
