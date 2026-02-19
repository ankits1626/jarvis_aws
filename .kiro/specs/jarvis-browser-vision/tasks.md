# Implementation Plan: JARVIS Browser Vision Module

## Overview

This plan implements passive browser observation for the JARVIS desktop application. The Browser Vision module monitors Chrome's active tab URL via AppleScript, detects YouTube videos, sends native macOS notifications, and scrapes video metadata to display gists in a dedicated UI section. Implementation is split into 4 phases: backend observer, YouTube scraper, Tauri commands, and frontend UI.

## Prerequisites

Before starting, verify:
- macOS system with Google Chrome installed
- AppleScript can query Chrome: `osascript -e 'tell application "Google Chrome" to return URL of active tab of front window'`
- `tauri-plugin-notification` is compatible with Tauri v2

## Tasks

- [x] 1. Create browser module structure
  - Create `jarvis-app/src-tauri/src/browser/mod.rs`
  - Create `jarvis-app/src-tauri/src/browser/observer.rs`
  - Create `jarvis-app/src-tauri/src/browser/youtube.rs`
  - Add `pub mod browser;` to `src-tauri/src/lib.rs`
  - Define module exports in `mod.rs`
  - _Requirements: 6.6, 6.7_

- [x] 2. Implement BrowserObserver struct and state management
  - Define `BrowserObserver` struct in `observer.rs` with fields:
    - `app_handle: AppHandle`
    - `stop_tx: Option<tokio::sync::watch::Sender<bool>>`
    - `is_running: bool`
    - `last_url: String`
  - Implement `BrowserObserver::new(app_handle: AppHandle)`
  - Implement `is_running()` getter
  - _Requirements: 6.2, 6.3, 6.4, 6.5_

- [x] 3. Implement Chrome URL polling
  - Implement `poll_chrome_url()` async method that executes AppleScript via `tokio::process::Command`
  - Use command: `osascript -e 'tell application "Google Chrome" to return URL of active tab of front window'`
  - Add 2-second timeout using `tokio::time::timeout`
  - Return `Result<String, String>` with URL or error
  - Handle errors gracefully (Chrome not running, no windows, timeout)
  - _Requirements: 1.2, 1.4, 1.5, 12.2_

- [x] 3.1 Write unit tests for Chrome polling
  - Test `poll_chrome_url()` handles timeout gracefully
  - Test error messages are descriptive
  - Mock AppleScript execution for testing

- [x] 4. Implement observer start/stop lifecycle
  - Implement `start()` method:
    - Check if already running, return error if true
    - Create `tokio::sync::watch` channel for stop signal
    - Spawn tokio background task with polling loop
    - Set `is_running = true`
  - Implement `stop()` method:
    - Check if not running, return error if true
    - Send stop signal via watch channel
    - Set `is_running = false`, clear `last_url`
  - Use `tokio::select!` with biased stop signal in polling loop
  - Poll every 3 seconds (Observer_Poll_Interval)
  - _Requirements: 1.1, 1.3, 1.8, 1.9_


- [x] 4.1 Write unit tests for observer lifecycle
  - Test `start()` returns error when already running
  - Test `stop()` returns error when not running
  - Test observer stops within one poll interval (3 seconds)
  - Test `is_running` state transitions correctly

- [x] 5. Implement URL debouncing and classification
  - In polling loop, compare new URL with `last_url`
  - Skip processing if URLs are identical (URL_Debounce)
  - Update `last_url` when URL changes
  - Call `classify_url()` for changed URLs
  - _Requirements: 1.6, 1.7_

- [x] 6. Implement YouTube URL detection
  - Implement `detect_youtube(url: &str) -> Option<(String, String)>` function
  - Use `LazyLock<Regex>` for compiled regex pattern (initialized once)
  - Match patterns: `youtube.com/watch?v=` and `youtu.be/`
  - Handle http/https, www prefix, additional query parameters
  - Extract 11-character video ID from query parameter `v`
  - Return `Some((url.to_string(), video_id))` if YouTube video detected, `None` otherwise
  - _Requirements: 2.1, 2.2, 2.4_

- [x] 6.1 Write unit tests for YouTube detection
  - Test various YouTube URL formats (youtube.com, youtu.be, with/without www)
  - Test video ID extraction from different URL patterns
  - Test non-YouTube URLs return None
  - Test malformed URLs are handled gracefully

- [x] 7. Implement YouTube detection event emission
  - Define `YouTubeDetectedEvent` struct in `observer.rs`:
    - Fields: `url: String`, `video_id: String`
    - Add `#[derive(Serialize, Deserialize, Clone)]`
  - When YouTube video detected, emit `youtube-video-detected` Tauri event
  - Event payload: `YouTubeDetectedEvent { url, video_id }`
  - Use `app_handle.emit()` to send event
  - Ensure no duplicate events for same URL (enforced by URL_Debounce)
  - _Requirements: 2.3, 2.5, 2.6_

- [x] 8. Implement native macOS notification
  - Register plugin in `lib.rs`: `.plugin(tauri_plugin_notification::init())`
  - When YouTube detected, send notification using `NotificationExt`
  - Notification title: "YouTube Video Detected"
  - Notification body: "Open JarvisApp to prepare a gist"
  - Handle notification failures gracefully (log warning, continue)
  - _Requirements: 3.1, 3.2, 3.3, 3.6, 10.1_

- [x] 8.1 Update Cargo.toml dependencies and capabilities
  - Add `tauri-plugin-notification = "2"` to `[dependencies]` in `src-tauri/Cargo.toml`
  - Move `regex = "1"` from `[dev-dependencies]` to `[dependencies]`
  - Add `proptest = "1.0"` to `[dev-dependencies]` (for property-based tests)
  - Verify `reqwest` has `stream`, `blocking`, `json` features
  - Add `"notification:default"` to permissions in `src-tauri/capabilities/default.json`
  - _Requirements: 3.5, 10.1, 10.2, 10.4, 10.5_

- [x] 9. Checkpoint — Observer backend compiles and runs
  - Run `cargo build` in `jarvis-app/src-tauri`
  - Run `cargo test` — all tests pass
  - Manually test: start observer, open Chrome, navigate to YouTube
  - Verify notification appears
  - Verify no errors when Chrome is closed
  - Ask user if questions arise

- [x] 10. Implement YouTube scraper struct
  - Define `YouTubeGist` struct in `youtube.rs`:
    - `url: String`
    - `video_id: String`
    - `title: String`
    - `channel: String`
    - `description: String`
    - `duration_seconds: u32`
  - Add `#[derive(Serialize, Deserialize, Clone)]`
  - _Requirements: 4.6_

- [x] 11. Implement YouTube page fetching
  - Implement `scrape_youtube_gist(url: &str) -> Result<YouTubeGist, String>`
  - Use `reqwest::get(url)` with 10-second timeout
  - Fetch page HTML using `.text().await`
  - Handle network errors with descriptive messages
  - _Requirements: 4.1, 4.8, 10.4, 12.4_

- [x] 12. Implement metadata extraction from HTML
  - Extract title from `<title>` tag, strip " - YouTube" suffix
  - Use string search + brace-counting to extract `ytInitialPlayerResponse` JSON from HTML:
    - Find start marker "var ytInitialPlayerResponse = "
    - Count braces to find matching closing brace (handle escaped quotes in strings)
  - Implement `unescape_json(s: &str) -> String` helper to handle escaped characters (\n, \t, \", \\, etc.)
  - Parse JSON and extract using regex with `LazyLock`:
    - `shortDescription` → description (call `unescape_json` on result)
    - `ownerChannelName` → channel (call `unescape_json` on result)
    - `lengthSeconds` → duration_seconds (parse as u32)
  - Use fallback values for missing fields:
    - "Unknown" for title/channel
    - Empty string for description
    - 0 for duration
  - _Requirements: 4.2, 4.3, 4.4, 4.5, 4.7, 4.9, 4.10_

- [x] 12.1 Write unit tests for metadata extraction
  - Test title extraction and suffix stripping
  - Test JSON parsing from HTML with brace-counting
  - Test `unescape_json` handles \n, \t, \", \\, \/ correctly
  - Test fallback values when fields are missing
  - Test error handling for malformed HTML

- [x] 13. Implement Tauri commands
  - Add `start_browser_observer` command in `commands.rs`
  - Add `stop_browser_observer` command
  - Add `fetch_youtube_gist` command accepting `url: String`
  - Add `get_observer_status` command returning `bool`
  - All commands access `BrowserObserver` from Tauri state: `State<Arc<tokio::sync::Mutex<BrowserObserver>>>`
  - Return `Result<T, String>` for error handling
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7_

- [x] 14. Initialize BrowserObserver in Tauri setup
  - In `lib.rs` setup closure, create `BrowserObserver::new(app.handle().clone())`
  - Wrap in `Arc<tokio::sync::Mutex<BrowserObserver>>`
  - Add to managed state: `app.manage(observer)`
  - Register commands in `invoke_handler`
  - _Requirements: 6.1, 6.8_

- [x] 15. Checkpoint — Commands and state management integration
  - Run `cargo build` in `jarvis-app/src-tauri` — compiles without errors
  - Run `cargo test` — all unit tests pass
  - Verify BrowserObserver is properly initialized in Tauri state
  - Test command registration in `invoke_handler`
  - Manually test command invocations via Tauri dev tools or frontend:
    - `get_observer_status` returns false initially
    - `start_browser_observer` succeeds and returns Ok(())
    - `get_observer_status` now returns true
    - `stop_browser_observer` succeeds and returns Ok(())
    - `get_observer_status` returns false again
    - `start_browser_observer` twice returns error on second call
    - `stop_browser_observer` when not running returns error
  - Ask user if questions arise

- [x] 16. Write property-based tests
  - Create property test file or add to existing test module
  - Implement property tests using `proptest!` macro:
    - **Property 7**: YouTube URL detection and extraction
      - Generate random 11-character video IDs
      - Test both youtube.com and youtu.be formats
      - Verify video_id extraction is correct
    - **Property 9**: Non-YouTube URLs don't emit events
      - Generate random non-YouTube domains and paths
      - Verify `detect_youtube` returns None
    - **Property 14**: Scraper uses fallback values for missing fields
      - Generate JSON with/without optional fields
      - Verify fallback values are used when fields missing
  - Each test should run minimum 100 iterations
  - Add comment tags referencing design document properties
  - _Validates: Properties 7, 9, 14_

- [ ] 17. Checkpoint — Backend commands work end-to-end
  - Run `cargo build` and `cargo test` — all unit and property tests pass
  - Test commands via Tauri dev tools:
    - Call `start_browser_observer` → verify starts
    - Navigate to YouTube in Chrome → verify event emitted
    - Call `fetch_youtube_gist` with URL → verify gist returned
    - Call `stop_browser_observer` → verify stops
  - Ask user if questions arise

- [x] 18. Add frontend types
  - Update `src/state/types.ts`:
    - Add `YouTubeGist` interface matching Rust struct
    - Add `YouTubeDetectedEvent` interface: `{ url: string, video_id: string }`
  - _Requirements: 9.1, 9.2, 9.3_

- [x] 19. Create YouTubeSection component
  - Create `src/components/YouTubeSection.tsx`
  - Define component props: `{ onClose: () => void }`
  - Add state for:
    - `isRunning: boolean` (observer status)
    - `videos: DetectedVideo[]` (list of detected videos)
  - Define `DetectedVideo` interface with gist, loading, error fields
  - _Requirements: 7.1, 7.2, 7.3_

- [x] 20. Implement observer status and toggle
  - Load observer status on mount via `get_observer_status` command
  - Display status indicator: "Observer: Running" or "Observer: Stopped"
  - Add toggle button to start/stop observer
  - Call `start_browser_observer` or `stop_browser_observer` on click
  - Update `isRunning` state after command succeeds
  - _Requirements: 7.2, 7.3_

- [x] 21. Listen for youtube-video-detected events
  - Use `useTauriEvent` hook or `listen()` from `@tauri-apps/api/event`
  - On event, add detected video to `videos` array
  - Most recent videos at the top of the list
  - _Requirements: 7.4, 7.11, 9.5_

- [x] 22. Implement VideoCard sub-component
  - Create `VideoCard` component with props:
    - `video: DetectedVideo`
    - `onPrepareGist: () => void`
    - `onDismiss: () => void`
    - `onCopy: () => void`
  - Display video URL
  - Show "Prepare Gist" button when no gist loaded
  - Show loading state while fetching
  - Show error state if fetch fails
  - When gist loaded, display title, channel, duration, description
  - Add "Copy" and "Dismiss" buttons for gist
  - _Requirements: 7.5, 7.6, 7.7, 7.8, 7.9_

- [x] 23. Implement gist preparation
  - Add `handlePrepareGist(index: number)` function
  - Set loading state for video at index
  - Call `fetch_youtube_gist` command with video URL
  - On success, update video with gist data
  - On error, update video with error message
  - _Requirements: 7.6, 7.7_

- [x] 24. Implement gist actions
  - Add `handleDismiss(index: number)` to remove video from list
  - Add `formatGist(gist: YouTubeGist)` to format gist as text
  - Add `handleCopy(gist: YouTubeGist)` to copy formatted gist to clipboard
  - Use `navigator.clipboard.writeText()`
  - _Requirements: 7.8, 7.9_

- [x] 25. Add YouTubeSection to component exports
  - Export `YouTubeSection` from `src/components/index.ts`
  - _Requirements: 9.4_

- [x] 26. Implement hamburger menu in App.tsx
  - Add hamburger button (☰) next to settings button in header
  - Add state for `showHamburgerMenu: boolean`
  - Add state for `youtubeNotification: boolean` (badge indicator)
  - Render dropdown menu when hamburger clicked
  - Include "YouTube" option with video icon
  - Close menu when clicking outside using `onClick` handler on dropdown wrapper/overlay
  - _Requirements: 8.1, 8.2, 8.3, 8.6, 8.7_

- [x] 27. Implement notification badge
  - Listen for `youtube-video-detected` events in App.tsx
  - Set `youtubeNotification = true` when event received and YouTube section not open
  - Display red dot badge on hamburger button when `youtubeNotification = true`
  - Clear badge when YouTube section is opened
  - _Requirements: 8.4, 8.5_

- [x] 28. Integrate YouTubeSection with App
  - Add state for `showYouTube: boolean`
  - Render `YouTubeSection` as modal overlay when `showYouTube = true`
  - Pass `onClose` handler to close section
  - Use existing `dialog-overlay` and `settings-panel` CSS classes
  - _Requirements: 7.1, 7.10_

- [x] 29. Add CSS styles for YouTube section
  - Add styles for `.youtube-section`, `.youtube-header`, `.youtube-content`
  - Add styles for `.observer-status`, `.videos-list`, `.video-card`
  - Add styles for `.gist-display`, `.gist-field`, `.gist-description`
  - Add styles for `.gist-actions`, `.prepare-gist-button`, `.copy-button`, `.dismiss-button`
  - Match existing settings panel styling patterns
  - _Requirements: 7.1_

- [x] 30. Add CSS styles for hamburger menu
  - Add styles for `.hamburger-button`, `.hamburger-menu`, `.hamburger-menu-item`
  - Add styles for `.notification-badge` (small red dot)
  - Match existing settings button styling
  - Position dropdown below button, aligned right
  - _Requirements: 8.7, 8.8_

- [x] 31. Final build and verification
  - Run `cargo build` in `jarvis-app/src-tauri` — compiles without errors
  - Run `cargo test` — all tests pass
  - Run `npm run build` in `jarvis-app` — frontend compiles
  - Launch app: `make dev`
  - Test full workflow:
    - Start observer from YouTube section
    - Open Chrome, navigate to YouTube video
    - Verify notification appears
    - Verify hamburger badge appears
    - Open YouTube section, verify video detected
    - Click "Prepare Gist", verify gist displays
    - Click "Copy", verify clipboard contains gist
    - Click "Dismiss", verify video removed
    - Stop observer, verify polling stops
  - Test error cases:
    - Close Chrome while observer running → no errors
    - Invalid YouTube URL → descriptive error
    - Network timeout → descriptive error
  - Verify observer doesn't interfere with recording/transcription
  - _Requirements: 11.1-11.7, 12.1-12.6_

## Files Changed/Created

### New Files
- `jarvis-app/src-tauri/src/browser/mod.rs` — Module exports
- `jarvis-app/src-tauri/src/browser/observer.rs` — BrowserObserver implementation
- `jarvis-app/src-tauri/src/browser/youtube.rs` — YouTube scraper and gist struct
- `jarvis-app/src/components/YouTubeSection.tsx` — YouTube UI section

### Modified Files
- `jarvis-app/src-tauri/src/lib.rs` — Add browser module, initialize observer, register commands
- `jarvis-app/src-tauri/src/commands.rs` — Add browser observer commands
- `jarvis-app/src-tauri/Cargo.toml` — Add notification plugin, move regex to dependencies
- `jarvis-app/src-tauri/capabilities/default.json` — Add notification permissions
- `jarvis-app/src/state/types.ts` — Add YouTubeGist and event types
- `jarvis-app/src/components/index.ts` — Export YouTubeSection
- `jarvis-app/src/App.tsx` — Add hamburger menu, YouTube section integration
- `jarvis-app/src/App.css` — Add YouTube section and hamburger menu styles

## Notes

- Observer runs in separate tokio task, doesn't block main thread or other subsystems
- AppleScript polling is macOS-specific, no cross-platform support needed for MVP
- YouTube scraping uses embedded JSON, no API key required
- Notification failures are non-fatal, observer continues operating
- URL debouncing prevents duplicate processing and event emission
- Frontend uses existing modal overlay pattern for consistency
- Hamburger menu is extensible for future feature sections
- All error messages are user-friendly strings, not raw Rust errors
