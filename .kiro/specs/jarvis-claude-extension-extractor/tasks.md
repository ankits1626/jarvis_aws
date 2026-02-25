# Implementation Plan: Claude Extension Extractor

## Overview

This feature adds a `claude_extension` extractor to the Jarvis browser module that captures conversations from the Claude Chrome Extension side panel using macOS Accessibility APIs. The implementation follows the established extractor pattern but introduces a new extraction mechanism: AXUIElement APIs instead of JavaScript execution. The feature includes manual FFI bindings, depth-based message separation, permission handling, and comprehensive property-based testing.

## Tasks

### Phase 1: Foundation - Accessibility API Setup

This phase establishes the macOS Accessibility API foundation, including FFI bindings, permission checks, Chrome process discovery, and tree traversal primitives.

- [x] 1. Set up macOS Accessibility API FFI bindings
  - [x] 1.1 Add core-foundation and core-graphics dependencies to Cargo.toml
    - Add `core-foundation = "0.9"` and `core-graphics = "0.23"` under `[target.'cfg(target_os = "macos")'.dependencies]`
    - _Requirements: 1.5, 10.5_
  
  - [x] 1.2 Create browser/accessibility.rs with FFI declarations
    - Define `AXUIElementRef`, `AXError` types
    - Declare extern "C" functions: `AXIsProcessTrusted`, `AXUIElementCreateApplication`, `AXUIElementCopyAttributeValue`, `AXUIElementCopyAttributeNames`
    - Define attribute constants: `kAXRoleAttribute`, `kAXTitleAttribute`, `kAXValueAttribute`, `kAXChildrenAttribute`, `kAXPlaceholderValueAttribute`
    - Gate all code behind `#[cfg(target_os = "macos")]`
    - _Requirements: 1.1, 1.5, 1.8_
  
  - [x] 1.3 Implement AccessibilityReader::check_permission()
    - Call `AXIsProcessTrusted()` with no prompt
    - Return boolean result
    - _Requirements: 1.7, 7.1, 7.4_
  
  - [x]* 1.4 Write unit test for permission check
    - Test that `check_permission()` returns a boolean without panicking
    - _Requirements: 1.7_

- [x] 2. Implement Chrome process discovery
  - [x] 2.1 Implement AccessibilityReader::find_chrome_pid()
    - Use NSWorkspace to find running applications
    - Filter by bundle ID "com.google.Chrome"
    - Return process ID or error "Chrome is not running"
    - _Requirements: 1.2, 1.6, 9.1_
  
  - [ ]* 2.2 Write property test for Chrome process discovery
    - **Property 1: Chrome Process Discovery**
    - **Validates: Requirements 1.2**
    - Generate: Mock system states with Chrome running/not running
    - Test: When Chrome is running, `find_chrome_pid()` returns positive PID
  
  - [ ]* 2.3 Write unit test for Chrome not running error
    - Test: `find_chrome_pid()` returns descriptive error when Chrome is not running
    - _Requirements: 1.6, 9.1_

- [x] 3. Implement accessibility tree traversal and text extraction
  - [x] 3.1 Implement AccessibilityReader::find_web_areas()
    - Create AXUIElement for Chrome application using PID
    - Recursively traverse accessibility tree to find all AXWebArea elements
    - Extract title attribute from each web area
    - Return Vec<WebArea> with title and element reference
    - Handle tree not populated: retry once after 500ms delay
    - _Requirements: 1.3, 9.8_
  
  - [x] 3.2 Implement AccessibilityReader::extract_text_content()
    - Recursively traverse web area's children
    - Track depth from web area root (0 = direct child)
    - Track parent role for each element
    - Extract text from AXStaticText (value), AXHeading (title with "## " prefix), AXLink (title with "[link: ]" wrapper), AXTextField (placeholder with "[input: ]" wrapper)
    - Return Vec<TextBlock> with text, role, depth, parent_role
    - _Requirements: 1.4, 3.1, 3.3, 3.4_
  
  - [ ]* 3.3 Write property test for web area discovery completeness
    - **Property 2: Web Area Discovery Completeness**
    - **Validates: Requirements 1.3**
    - Generate: Mock Chrome instances with varying numbers of tabs
    - Test: `find_web_areas()` returns non-empty list when tabs exist
  
  - [ ]* 3.4 Write property test for text content extraction completeness
    - **Property 3: Text Content Extraction Completeness**
    - **Validates: Requirements 1.4, 3.1**
    - Generate: Random accessibility trees with text elements at various depths
    - Test: All text values appear in output with correct depth and role
  
  - [ ]* 3.5 Write unit test for element formatting preservation
    - Test: AXHeading with text "Overview" produces "## Overview"
    - Test: AXLink with text "docs" produces "[link: docs]"
    - Test: AXTextField with placeholder "Reply to Claude" produces "[input: Reply to Claude]"
    - _Requirements: 3.3, 3.4_

**Phase 1 Checkpoint**: Run `cargo test` to verify all accessibility module tests pass. Ensure FFI bindings work correctly and tree traversal returns expected data structures.

---

### Phase 2: Extractor Core - Conversation Extraction Logic

This phase implements the core conversation extraction logic, including Claude panel detection, depth-based message separation, page context extraction, and PageGist construction.

- [x] 4. Implement Claude side panel detection
  - [x] 4.1 Create browser/extractors/claude_extension.rs and implement find_claude_web_area() helper function
    - Create new file browser/extractors/claude_extension.rs
    - Add necessary imports (PageGist, ChromeAppleScriptAdapter, BrowserAdapter, SourceType, AccessibilityReader, WebArea, TextBlock)
    - Implement find_claude_web_area() helper function that searches web areas for title containing "Claude"
    - Return the matching WebArea or error "No Claude conversation found. Open the Claude Chrome Extension side panel first."
    - Handle case where multiple web areas match (return first)
    - _Requirements: 2.1, 2.2, 2.4, 9.2_
  
  - [ ]* 4.2 Write property test for Claude panel identification
    - **Property 4: Claude Panel Identification**
    - **Validates: Requirements 2.1**
    - Generate: Sets of web areas with exactly one containing "Claude" in title
    - Test: `find_claude_web_area()` identifies the correct web area
  
  - [ ]* 4.3 Write property test for Claude panel detection boolean
    - **Property 5: Claude Panel Detection Boolean**
    - **Validates: Requirements 2.3**
    - Generate: Random sets of web areas with/without Claude panel
    - Test: Detection returns true iff Claude web area exists
  
  - [ ]* 4.4 Write unit test for no Claude panel error
    - Test: `find_claude_web_area()` returns descriptive error when no Claude web area exists
    - _Requirements: 2.4, 9.2_

- [x] 5. Implement conversation reconstruction with depth-based message separation
  - [x] 5.1 Add reconstruct_conversation() function to claude_extension.rs
    - Implement in browser/extractors/claude_extension.rs (file created in Task 4.1)
    - Filter text blocks to exclude those after "[input: Reply to Claude]" marker
    - Detect message boundaries using depth changes (depth decrease from >6 to <6) and plan indicators
    - Track current author starting with "You", alternating to "Claude" at each boundary
    - Format messages with "--- You ---" and "--- Claude ---" separators
    - Preserve heading and link formatting from TextBlock
    - Truncate conversation at 50,000 characters with "[conversation truncated]" marker
    - Count message turns for metadata
    - Extract first user prompt (first 200 chars) for description
    - Return ConversationData struct with full_text, message_count, first_prompt
    - _Requirements: 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8_
  
  - [x] 5.2 Implement is_plan_indicator() helper function
    - Check if text contains "steps", "created a plan", "done", or "extract page text" (case-insensitive)
    - Used to detect start of Claude's response
    - _Requirements: 3.2_
  
  - [ ]* 5.3 Write property test for conversation structure preservation
    - **Property 6: Conversation Structure Preservation**
    - **Validates: Requirements 3.2**
    - Generate: Random sequences of text blocks with varying depths simulating user/Claude patterns
    - Test: Output contains correct number of "--- You ---" and "--- Claude ---" separators based on depth boundaries
  
  - [ ]* 5.4 Write property test for element formatting preservation
    - **Property 7: Element Formatting Preservation**
    - **Validates: Requirements 3.3, 3.4**
    - Generate: Random mix of headings and links
    - Test: All headings formatted as "## text", all links as "[link: text]"
  
  - [ ]* 5.5 Write property test for conversation length truncation
    - **Property 10: Conversation Length Truncation**
    - **Validates: Requirements 3.7**
    - Generate: Random conversation text of varying lengths
    - Test: Output length ≤ 50,000 + truncation marker length
  
  - [ ]* 5.6 Write property test for message count accuracy
    - **Property 11: Message Count Accuracy**
    - **Validates: Requirements 3.8**
    - Generate: Random conversations with N turns
    - Test: message_count metadata equals N
  
  - [ ]* 5.7 Write unit test for conversation boundary detection
    - Test: Text blocks with "[input: Reply to Claude]" marker are excluded from conversation
    - _Requirements: 3.5_
  
  - [ ]* 5.8 Write unit test for multi-turn conversation with depth changes
    - Test: Conversation with 3 user prompts and 3 Claude responses (simulated with depth patterns) is correctly separated
    - _Requirements: 3.6_
  
  - [ ]* 5.9 Write unit test for empty conversation error
    - Test: `reconstruct_conversation()` returns error "Claude conversation is empty" when text blocks are empty
    - _Requirements: 2.5, 9.3_

- [x] 6. Implement page context extraction
  - [x] 6.1 Add extract_page_title() helper function to claude_extension.rs
    - Implement in browser/extractors/claude_extension.rs (file created in Task 4.1)
    - Find non-Claude web area (first web area without "Claude" in title)
    - Extract title attribute
    - Return title or error "No active tab found"
    - _Requirements: 4.2, 4.4, 9.5_
  
  - [ ]* 6.2 Write unit test for active tab title extraction
    - Test: `extract_page_title()` returns title from non-Claude web area
    - _Requirements: 4.2_

- [x] 7. Implement PageGist construction
  - [x] 7.1 Add build_page_gist() function to claude_extension.rs
    - Implement in browser/extractors/claude_extension.rs (file created in Task 4.1)
    - Set source_type to SourceType::Chat
    - Format title as "Claude: " + page_title
    - Set url to page_url
    - Extract domain from page_url using existing extract_domain() function
    - Set author to Some("Claude Extension")
    - Set description to first_prompt (from ConversationData)
    - Set content_excerpt to full_text (from ConversationData)
    - Build extra JSON with page_url, page_title, message_count, extraction_method: "accessibility_api", claude_extension_version
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9_
  
  - [ ]* 7.2 Write property test for PageGist title formatting
    - **Property 14: PageGist Title Formatting**
    - **Validates: Requirements 5.3**
    - Generate: Random page titles (including edge cases: empty, very long, special chars)
    - Test: PageGist title = "Claude: " + page_title
  
  - [ ]* 7.3 Write property test for domain extraction from URL
    - **Property 15: Domain Extraction from URL**
    - **Validates: Requirements 5.5**
    - Generate: Random URLs (various schemes, subdomains, paths)
    - Test: Domain extraction matches existing extract_domain() function
  
  - [ ]* 7.4 Write property test for metadata completeness
    - **Property 17: Metadata Completeness**
    - **Validates: Requirements 4.3, 5.9**
    - Generate: Random conversation data
    - Test: All required keys present in extra field with correct values
  
  - [ ]* 7.5 Write unit test for PageGist construction
    - Test: All PageGist fields are populated correctly for a sample conversation
    - Test: Description field contains first user prompt truncated to 200 characters (Property 16)
    - _Requirements: 5.1-5.9_

**Phase 2 Checkpoint**: Run `cargo test` to verify all conversation reconstruction and PageGist construction tests pass. Ensure depth-based message separation works correctly with sample data.

---

### Phase 3: Integration - Backend Wiring

This phase wires the extractor into the Tauri backend, including the main orchestration function, module exports, and Tauri command handlers.

- [x] 8. Implement main extractor orchestration
  - [x] 8.1 Add extract() orchestrator function to claude_extension.rs
    - Implement in browser/extractors/claude_extension.rs (file created in Task 4.1)
    - Check accessibility permission, return error if not granted
    - Call find_chrome_pid(), propagate error if Chrome not running
    - Call find_web_areas(), handle retry logic for tree not populated
    - Call find_claude_web_area(), propagate error if not found
    - Call extract_text_content() on Claude web area
    - Call reconstruct_conversation() with text blocks
    - Use ChromeAppleScriptAdapter to get active tab URL
    - Call extract_page_title() to get page title from web areas
    - Call build_page_gist() to construct final result
    - Log all errors to stderr with "[ClaudeExtractor]" prefix
    - Return Result<PageGist, String>
    - _Requirements: 1.1, 1.6, 1.7, 2.1, 3.1, 3.2, 4.1, 4.2, 5.1, 9.6, 9.7_
  
  - [x] 8.2 Add non-macOS stub implementation
    - Implement extract() for non-macOS platforms that returns error "Claude conversation capture is only available on macOS"
    - Gate behind `#[cfg(not(target_os = "macos"))]`
    - _Requirements: 10.1, 10.2, 10.3, 10.4_
  
  - [ ]* 8.3 Write property test for error handling without panics
    - **Property 20: Error Handling Without Panics**
    - **Validates: Requirements 9.6**
    - Generate: Random error conditions (simulated failures)
    - Test: All code paths return Result::Err, never panic
    - Test: Permission check executes before any tree traversal (Property 19)
  
  - [ ]* 8.4 Write unit test for non-macOS platform error
    - Test: extract() returns platform error on non-macOS
    - _Requirements: 10.1, 10.2_

- [x] 9. Integrate extractor into browser module
  - [x] 9.1 Add claude_extension module to browser/extractors/mod.rs
    - Add `pub mod claude_extension;` declaration
    - _Requirements: 6.1_
  
  - [x] 9.2 Export accessibility module from browser module
    - If browser/mod.rs exists: add `pub mod accessibility;` declaration (macOS-only)
    - If browser module uses different structure: add accessibility module export to appropriate location
    - _Requirements: 1.1_

- [x] 10. Implement Tauri commands
  - [x] 10.1 Add capture_claude_conversation command to commands.rs
    - Call claude_extension::extract().await
    - Return Result<PageGist, String>
    - _Requirements: 6.2, 6.3_
  
  - [x] 10.2 Add check_accessibility_permission command to commands.rs
    - Call AccessibilityReader::check_permission() on macOS
    - Return false on non-macOS
    - _Requirements: 7.2, 7.5_
  
  - [x] 10.3 Register commands in lib.rs invoke_handler
    - Add capture_claude_conversation and check_accessibility_permission to generate_handler! macro
    - _Requirements: 6.5_
  
  - [ ]* 10.4 Write integration test for Tauri command registration
    - Test: Commands are callable from frontend (mock invocation)
    - _Requirements: 6.5_

**Phase 3 Checkpoint**: Run `cargo test --package jarvis-app` to verify all backend integration tests pass. Ensure Tauri commands are properly registered and callable.

---

### Phase 4: Frontend - UI Integration

This phase adds the frontend UI components for triggering Claude conversation capture, including permission checks, loading states, and error handling.

- [x] 11. Implement frontend integration in BrowserTool component
  - [x] 11.1 Add state for Claude conversation capture
    - Add claudePermission state (boolean)
    - Add capturingClaude state (boolean)
    - _Requirements: 8.1_
  
  - [x] 11.2 Add permission check on component mount
    - Call check_accessibility_permission command on mount (macOS only)
    - Update claudePermission state with result
    - _Requirements: 8.2_
  
  - [x] 11.3 Implement handleCaptureClaude function
    - Set capturingClaude to true
    - Call capture_claude_conversation command
    - On success: set currentGist state to display GistCard
    - On error: set error state to display error message
    - Finally: set capturingClaude to false
    - _Requirements: 8.1, 8.4, 8.6_
  
  - [x] 11.4 Add "Capture Claude Conversation" button to BrowserTool UI
    - Show button when Chrome is active browser
    - Disable when claudePermission is false or capturingClaude is true
    - Show tooltip "Accessibility permission required" when disabled due to permission
    - Show tooltip "No Claude conversation found" when disabled due to no conversation (handled by error display)
    - Show "Capturing..." text when capturingClaude is true
    - Call handleCaptureClaude on click
    - _Requirements: 8.1, 8.2, 8.3_
  
  - [ ]* 11.5 Write unit test for button visibility
    - Test: Button appears when Chrome is active
    - _Requirements: 8.1_
  
  - [ ]* 11.6 Write unit test for button disabled state
    - Test: Button is disabled when permission is not granted
    - _Requirements: 8.2_
  
  - [ ]* 11.7 Write unit test for loading state
    - Test: Button shows "Capturing..." during async operation
    - _Requirements: 8.3_
  
  - [ ]* 11.8 Write unit test for success flow
    - Test: Successful capture displays GistCard with conversation
    - _Requirements: 8.4_
  
  - [ ]* 11.9 Write unit test for error display
    - Test: Error message appears inline when capture fails
    - _Requirements: 8.6_

**Phase 4 Checkpoint**: Run `npm test -- --run` to verify all frontend component tests pass. Manually test the UI flow in the browser to ensure button states and error messages work correctly.

---

### Phase 5: Validation - End-to-End Testing

This phase performs comprehensive end-to-end testing, including backwards compatibility verification and manual testing with the real Claude Chrome Extension.

- [ ] 12. Final integration and testing
  - [ ] 12.1 Run all backend tests
    - Execute `cargo test --package jarvis-app` to verify all unit and property tests pass
    - Ensure minimum 100 iterations for property tests
    - _Requirements: All_
  
  - [ ] 12.2 Run all frontend tests
    - Execute `npm test -- --run` to verify all React component tests pass
    - _Requirements: 8.1-8.6_
  
  - [ ] 12.3 Verify backwards compatibility
    - Ensure existing extractors (YouTube, ChatGPT, Gmail, Medium, Generic) still work
    - Ensure prepare_gist function continues to work for all existing source types
    - Ensure gem save pipeline works with Claude conversation gems
    - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5_

- [ ] 13. Manual testing and documentation
  - [ ] 13.1 Perform manual testing checklist
    - Test accessibility permission flow: deny permission, verify error message
    - Test accessibility permission flow: grant permission, verify capture works
    - Test Chrome not running: verify error message
    - Test Claude panel not open: verify error message
    - Test successful capture: verify conversation appears in gem with correct formatting
    - Test multi-turn conversation: verify message separation is correct
    - Test conversation with headings and links: verify formatting is preserved
    - Test very long conversation: verify truncation at 50,000 characters
    - _Requirements: All_
  
  - [ ] 13.2 Verify conversation capture with real Claude Chrome Extension
    - Open Claude Chrome Extension side panel in Chrome
    - Have a multi-turn conversation with Claude
    - Click "Capture Claude Conversation" button in Jarvis
    - Verify captured conversation matches what's visible in the side panel
    - Verify message boundaries are correct (user vs Claude)
    - Verify page context (URL and title) is correct
    - _Requirements: All_

**Phase 5 Checkpoint**: All tests pass, manual testing confirms the feature works end-to-end with the real Claude Chrome Extension, and backwards compatibility is verified.

- [ ] 1. Set up macOS Accessibility API FFI bindings
  - [ ] 1.1 Add core-foundation and core-graphics dependencies to Cargo.toml
    - Add `core-foundation = "0.9"` and `core-graphics = "0.23"` under `[target.'cfg(target_os = "macos")'.dependencies]`
    - _Requirements: 1.5, 10.5_
  
  - [ ] 1.2 Create browser/accessibility.rs with FFI declarations
    - Define `AXUIElementRef`, `AXError` types
    - Declare extern "C" functions: `AXIsProcessTrusted`, `AXUIElementCreateApplication`, `AXUIElementCopyAttributeValue`, `AXUIElementCopyAttributeNames`
    - Define attribute constants: `kAXRoleAttribute`, `kAXTitleAttribute`, `kAXValueAttribute`, `kAXChildrenAttribute`, `kAXPlaceholderValueAttribute`
    - Gate all code behind `#[cfg(target_os = "macos")]`
    - _Requirements: 1.1, 1.5, 1.8_
  
  - [ ] 1.3 Implement AccessibilityReader::check_permission()
    - Call `AXIsProcessTrusted()` with no prompt
    - Return boolean result
    - _Requirements: 1.7, 7.1, 7.4_
  
  - [ ]* 1.4 Write unit test for permission check
    - Test that `check_permission()` returns a boolean without panicking
    - _Requirements: 1.7_

- [ ] 2. Implement Chrome process discovery
  - [ ] 2.1 Implement AccessibilityReader::find_chrome_pid()
    - Use NSWorkspace to find running applications
    - Filter by bundle ID "com.google.Chrome"
    - Return process ID or error "Chrome is not running"
    - _Requirements: 1.2, 1.6, 9.1_
  
  - [ ]* 2.2 Write property test for Chrome process discovery
    - **Property 1: Chrome Process Discovery**
    - **Validates: Requirements 1.2**
    - Generate: Mock system states with Chrome running/not running
    - Test: When Chrome is running, `find_chrome_pid()` returns positive PID
  
  - [ ]* 2.3 Write unit test for Chrome not running error
    - Test: `find_chrome_pid()` returns descriptive error when Chrome is not running
    - _Requirements: 1.6, 9.1_

- [ ] 3. Implement accessibility tree traversal and text extraction
  - [ ] 3.1 Implement AccessibilityReader::find_web_areas()
    - Create AXUIElement for Chrome application using PID
    - Recursively traverse accessibility tree to find all AXWebArea elements
    - Extract title attribute from each web area
    - Return Vec<WebArea> with title and element reference
    - Handle tree not populated: retry once after 500ms delay
    - _Requirements: 1.3, 9.8_
  
  - [ ] 3.2 Implement AccessibilityReader::extract_text_content()
    - Recursively traverse web area's children
    - Track depth from web area root (0 = direct child)
    - Track parent role for each element
    - Extract text from AXStaticText (value), AXHeading (title with "## " prefix), AXLink (title with "[link: ]" wrapper), AXTextField (placeholder with "[input: ]" wrapper)
    - Return Vec<TextBlock> with text, role, depth, parent_role
    - _Requirements: 1.4, 3.1, 3.3, 3.4_
  
  - [ ]* 3.3 Write property test for web area discovery completeness
    - **Property 2: Web Area Discovery Completeness**
    - **Validates: Requirements 1.3**
    - Generate: Mock Chrome instances with varying numbers of tabs
    - Test: `find_web_areas()` returns non-empty list when tabs exist
  
  - [ ]* 3.4 Write property test for text content extraction completeness
    - **Property 3: Text Content Extraction Completeness**
    - **Validates: Requirements 1.4, 3.1**
    - Generate: Random accessibility trees with text elements at various depths
    - Test: All text values appear in output with correct depth and role
  
  - [ ]* 3.5 Write unit test for element formatting preservation
    - Test: AXHeading with text "Overview" produces "## Overview"
    - Test: AXLink with text "docs" produces "[link: docs]"
    - Test: AXTextField with placeholder "Reply to Claude" produces "[input: Reply to Claude]"
    - _Requirements: 3.3, 3.4_

- [ ] 4. Checkpoint - Ensure accessibility module tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 5. Implement Claude side panel detection
  - [ ] 5.1 Create browser/extractors/claude_extension.rs and implement find_claude_web_area() helper function
    - Create new file browser/extractors/claude_extension.rs
    - Add necessary imports (PageGist, ChromeAppleScriptAdapter, BrowserAdapter, SourceType, AccessibilityReader, WebArea, TextBlock)
    - Implement find_claude_web_area() helper function that searches web areas for title containing "Claude"
    - Return the matching WebArea or error "No Claude conversation found. Open the Claude Chrome Extension side panel first."
    - Handle case where multiple web areas match (return first)
    - _Requirements: 2.1, 2.2, 2.4, 9.2_
  
  - [ ]* 5.2 Write property test for Claude panel identification
    - **Property 4: Claude Panel Identification**
    - **Validates: Requirements 2.1**
    - Generate: Sets of web areas with exactly one containing "Claude" in title
    - Test: `find_claude_web_area()` identifies the correct web area
  
  - [ ]* 5.3 Write property test for Claude panel detection boolean
    - **Property 5: Claude Panel Detection Boolean**
    - **Validates: Requirements 2.3**
    - Generate: Random sets of web areas with/without Claude panel
    - Test: Detection returns true iff Claude web area exists
  
  - [ ]* 5.4 Write unit test for no Claude panel error
    - Test: `find_claude_web_area()` returns descriptive error when no Claude web area exists
    - _Requirements: 2.4, 9.2_

- [ ] 6. Implement conversation reconstruction with depth-based message separation
  - [ ] 6.1 Add reconstruct_conversation() function to claude_extension.rs
    - Implement in browser/extractors/claude_extension.rs (file created in Task 5.1)
    - Filter text blocks to exclude those after "[input: Reply to Claude]" marker
    - Detect message boundaries using depth changes (depth decrease from >6 to <6) and plan indicators
    - Track current author starting with "You", alternating to "Claude" at each boundary
    - Format messages with "--- You ---" and "--- Claude ---" separators
    - Preserve heading and link formatting from TextBlock
    - Truncate conversation at 50,000 characters with "[conversation truncated]" marker
    - Count message turns for metadata
    - Extract first user prompt (first 200 chars) for description
    - Return ConversationData struct with full_text, message_count, first_prompt
    - _Requirements: 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8_
  
  - [ ] 6.2 Implement is_plan_indicator() helper function
    - Check if text contains "steps", "created a plan", "done", or "extract page text" (case-insensitive)
    - Used to detect start of Claude's response
    - _Requirements: 3.2_
  
  - [ ]* 6.3 Write property test for conversation structure preservation
    - **Property 6: Conversation Structure Preservation**
    - **Validates: Requirements 3.2**
    - Generate: Random sequences of text blocks with varying depths simulating user/Claude patterns
    - Test: Output contains correct number of "--- You ---" and "--- Claude ---" separators based on depth boundaries
  
  - [ ]* 6.4 Write property test for element formatting preservation
    - **Property 7: Element Formatting Preservation**
    - **Validates: Requirements 3.3, 3.4**
    - Generate: Random mix of headings and links
    - Test: All headings formatted as "## text", all links as "[link: text]"
  
  - [ ]* 6.5 Write property test for conversation length truncation
    - **Property 10: Conversation Length Truncation**
    - **Validates: Requirements 3.7**
    - Generate: Random conversation text of varying lengths
    - Test: Output length ≤ 50,000 + truncation marker length
  
  - [ ]* 6.6 Write property test for message count accuracy
    - **Property 11: Message Count Accuracy**
    - **Validates: Requirements 3.8**
    - Generate: Random conversations with N turns
    - Test: message_count metadata equals N
  
  - [ ]* 6.7 Write unit test for conversation boundary detection
    - Test: Text blocks with "[input: Reply to Claude]" marker are excluded from conversation
    - _Requirements: 3.5_
  
  - [ ]* 6.8 Write unit test for multi-turn conversation with depth changes
    - Test: Conversation with 3 user prompts and 3 Claude responses (simulated with depth patterns) is correctly separated
    - _Requirements: 3.6_
  
  - [ ]* 6.9 Write unit test for empty conversation error
    - Test: `reconstruct_conversation()` returns error "Claude conversation is empty" when text blocks are empty
    - _Requirements: 2.5, 9.3_

- [ ] 7. Implement page context extraction
  - [ ] 7.1 Add extract_page_title() helper function to claude_extension.rs
    - Implement in browser/extractors/claude_extension.rs (file created in Task 5.1)
    - Find non-Claude web area (first web area without "Claude" in title)
    - Extract title attribute
    - Return title or error "No active tab found"
    - _Requirements: 4.2, 4.4, 9.5_
  
  - [ ]* 7.2 Write unit test for active tab title extraction
    - Test: `extract_page_title()` returns title from non-Claude web area
    - _Requirements: 4.2_

- [ ] 8. Implement PageGist construction
  - [ ] 8.1 Add build_page_gist() function to claude_extension.rs
    - Implement in browser/extractors/claude_extension.rs (file created in Task 5.1)
    - Set source_type to SourceType::Chat
    - Format title as "Claude: " + page_title
    - Set url to page_url
    - Extract domain from page_url using existing extract_domain() function
    - Set author to Some("Claude Extension")
    - Set description to first_prompt (from ConversationData)
    - Set content_excerpt to full_text (from ConversationData)
    - Build extra JSON with page_url, page_title, message_count, extraction_method: "accessibility_api", claude_extension_version
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9_
  
  - [ ]* 8.2 Write property test for PageGist title formatting
    - **Property 14: PageGist Title Formatting**
    - **Validates: Requirements 5.3**
    - Generate: Random page titles (including edge cases: empty, very long, special chars)
    - Test: PageGist title = "Claude: " + page_title
  
  - [ ]* 8.3 Write property test for domain extraction from URL
    - **Property 15: Domain Extraction from URL**
    - **Validates: Requirements 5.5**
    - Generate: Random URLs (various schemes, subdomains, paths)
    - Test: Domain extraction matches existing extract_domain() function
  
  - [ ]* 8.4 Write property test for metadata completeness
    - **Property 17: Metadata Completeness**
    - **Validates: Requirements 4.3, 5.9**
    - Generate: Random conversation data
    - Test: All required keys present in extra field with correct values
  
  - [ ]* 8.5 Write unit test for PageGist construction
    - Test: All PageGist fields are populated correctly for a sample conversation
    - Test: Description field contains first user prompt truncated to 200 characters (Property 16)
    - _Requirements: 5.1-5.9_

- [ ] 9. Checkpoint - Ensure conversation reconstruction tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 10. Implement main extractor orchestration
  - [x] 10.1 Add extract() orchestrator function to claude_extension.rs
    - Implement in browser/extractors/claude_extension.rs (file created in Task 5.1)
    - Check accessibility permission, return error if not granted
    - Call find_chrome_pid(), propagate error if Chrome not running
    - Call find_web_areas(), handle retry logic for tree not populated
    - Call find_claude_web_area(), propagate error if not found
    - Call extract_text_content() on Claude web area
    - Call reconstruct_conversation() with text blocks
    - Use ChromeAppleScriptAdapter to get active tab URL
    - Call extract_page_title() to get page title from web areas
    - Call build_page_gist() to construct final result
    - Log all errors to stderr with "[ClaudeExtractor]" prefix
    - Return Result<PageGist, String>
    - _Requirements: 1.1, 1.6, 1.7, 2.1, 3.1, 3.2, 4.1, 4.2, 5.1, 9.6, 9.7_
  
  - [x] 10.2 Add non-macOS stub implementation
    - Implement extract() for non-macOS platforms that returns error "Claude conversation capture is only available on macOS"
    - Gate behind `#[cfg(not(target_os = "macos"))]`
    - _Requirements: 10.1, 10.2, 10.3, 10.4_
  
  - [ ]* 10.3 Write property test for error handling without panics
    - **Property 20: Error Handling Without Panics**
    - **Validates: Requirements 9.6**
    - Generate: Random error conditions (simulated failures)
    - Test: All code paths return Result::Err, never panic
    - Test: Permission check executes before any tree traversal (Property 19)
  
  - [ ]* 10.4 Write unit test for non-macOS platform error
    - Test: extract() returns platform error on non-macOS
    - _Requirements: 10.1, 10.2_

- [ ] 11. Integrate extractor into browser module
  - [x] 11.1 Add claude_extension module to browser/extractors/mod.rs
    - Add `pub mod claude_extension;` declaration
    - _Requirements: 6.1_
  
  - [x] 11.2 Export accessibility module from browser module
    - If browser/mod.rs exists: add `pub mod accessibility;` declaration (macOS-only)
    - If browser module uses different structure: add accessibility module export to appropriate location
    - _Requirements: 1.1_

- [ ] 12. Implement Tauri commands
  - [x] 12.1 Add capture_claude_conversation command to commands.rs
    - Call claude_extension::extract().await
    - Return Result<PageGist, String>
    - _Requirements: 6.2, 6.3_
  
  - [x] 12.2 Add check_accessibility_permission command to commands.rs
    - Call AccessibilityReader::check_permission() on macOS
    - Return false on non-macOS
    - _Requirements: 7.2, 7.5_
  
  - [x] 12.3 Register commands in lib.rs invoke_handler
    - Add capture_claude_conversation and check_accessibility_permission to generate_handler! macro
    - _Requirements: 6.5_
  
  - [ ]* 12.4 Write integration test for Tauri command registration
    - Test: Commands are callable from frontend (mock invocation)
    - _Requirements: 6.5_

- [ ] 13. Checkpoint - Ensure backend integration tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 14. Implement frontend integration in BrowserTool component
  - [x] 14.1 Add state for Claude conversation capture
    - Add claudePermission state (boolean)
    - Add capturingClaude state (boolean)
    - _Requirements: 8.1_
  
  - [x] 14.2 Add permission check on component mount
    - Call check_accessibility_permission command on mount (macOS only)
    - Update claudePermission state with result
    - _Requirements: 8.2_
  
  - [x] 14.3 Implement handleCaptureClaude function
    - Set capturingClaude to true
    - Call capture_claude_conversation command
    - On success: set currentGist state to display GistCard
    - On error: set error state to display error message
    - Finally: set capturingClaude to false
    - _Requirements: 8.1, 8.4, 8.6_
  
  - [x] 14.4 Add "Capture Claude Conversation" button to BrowserTool UI
    - Show button when Chrome is active browser
    - Disable when claudePermission is false or capturingClaude is true
    - Show tooltip "Accessibility permission required" when disabled due to permission
    - Show tooltip "No Claude conversation found" when disabled due to no conversation (handled by error display)
    - Show "Capturing..." text when capturingClaude is true
    - Call handleCaptureClaude on click
    - _Requirements: 8.1, 8.2, 8.3_
  
  - [ ]* 14.5 Write unit test for button visibility
    - Test: Button appears when Chrome is active
    - _Requirements: 8.1_
  
  - [ ]* 14.6 Write unit test for button disabled state
    - Test: Button is disabled when permission is not granted
    - _Requirements: 8.2_
  
  - [ ]* 14.7 Write unit test for loading state
    - Test: Button shows "Capturing..." during async operation
    - _Requirements: 8.3_
  
  - [ ]* 14.8 Write unit test for success flow
    - Test: Successful capture displays GistCard with conversation
    - _Requirements: 8.4_
  
  - [ ]* 14.9 Write unit test for error display
    - Test: Error message appears inline when capture fails
    - _Requirements: 8.6_

- [ ] 15. Final integration and testing
  - [ ] 15.1 Run all backend tests
    - Execute `cargo test --package jarvis-app` to verify all unit and property tests pass
    - Ensure minimum 100 iterations for property tests
    - _Requirements: All_
  
  - [ ] 15.2 Run all frontend tests
    - Execute `npm test -- --run` to verify all React component tests pass
    - _Requirements: 8.1-8.6_
  
  - [ ] 15.3 Verify backwards compatibility
    - Ensure existing extractors (YouTube, ChatGPT, Gmail, Medium, Generic) still work
    - Ensure prepare_gist function continues to work for all existing source types
    - Ensure gem save pipeline works with Claude conversation gems
    - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5_

- [ ] 16. Final checkpoint - Manual testing and documentation
  - Ensure all tests pass, ask the user if questions arise.
  - Perform manual testing checklist from design document
  - Verify accessibility permission flow works end-to-end
  - Verify conversation capture works with real Claude Chrome Extension

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation at logical breaks
- Property tests validate universal correctness properties (minimum 100 iterations)
- Unit tests validate specific examples and edge cases
- All macOS-specific code is gated behind `#[cfg(target_os = "macos")]`
- Manual FFI is used instead of unmaintained accessibility-sys crate
- Depth-based message separation is the core algorithm for conversation reconstruction
- The feature integrates seamlessly with existing gem save pipeline
