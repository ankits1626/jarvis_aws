# Requirements Document

## Introduction

The JARVIS Browser Vision module extends the JARVIS desktop application with passive browser observation capabilities. While JARVIS already listens to audio via the transcription pipeline (Listen → Transcribe → Display), Browser Vision adds a parallel "sight" channel by observing the user's Chrome browser activity. The first implementation focuses on YouTube video detection: when the user navigates to a YouTube video, JARVIS sends a native macOS notification offering to prepare a gist. If accepted, JARVIS scrapes the YouTube page for metadata (title, channel, description, duration) and displays it in a dedicated YouTube section accessible via a hamburger menu. This module uses macOS AppleScript to query Chrome's active tab URL (no browser extension required) and `reqwest` to fetch YouTube page HTML (no API key required). The architecture is designed for future extension to detect other content types (articles, documentation, etc.).

## Glossary

- **Browser_Observer**: A background polling service that queries Chrome's active tab URL every 3 seconds via macOS AppleScript
- **YouTube_Scraper**: A component that fetches a YouTube video page via HTTP and extracts metadata (title, channel, description, duration) from the embedded `ytInitialPlayerResponse` JSON
- **YouTube_Gist**: A structured summary of a YouTube video containing URL, video ID, title, channel name, description text, and duration
- **AppleScript**: macOS scripting language that can query and control applications; used here to read Chrome's active tab URL without a browser extension
- **osascript**: The macOS command-line tool that executes AppleScript commands
- **ytInitialPlayerResponse**: A JSON object embedded in YouTube page HTML that contains video metadata including caption tracks, title, channel, and description
- **Video_ID**: The unique identifier for a YouTube video, extracted from the URL query parameter `v` (e.g., `dQw4w9WgXcQ`)
- **Native_Notification**: A macOS system notification displayed via the Notification Center using `tauri-plugin-notification`
- **YouTube_Section**: A dedicated UI panel in the JARVIS application for displaying detected YouTube videos and their gists
- **Hamburger_Menu**: A navigation menu (☰) in the application header that provides access to the YouTube section and future feature sections
- **Observer_Poll_Interval**: The time between consecutive Chrome URL checks (default: 3 seconds)
- **URL_Debounce**: Logic that prevents duplicate detections by comparing the current URL against the last observed URL

## Requirements

### Requirement 1: Chrome Browser URL Polling

**User Story:** As a JARVIS user, I want the application to passively monitor my Chrome browser's active tab, so that it can detect when I navigate to content it can help me with.

#### Acceptance Criteria

1. WHEN the Browser_Observer is started, THE Browser_Observer SHALL spawn a tokio background task that polls Chrome's active tab URL
2. THE Browser_Observer SHALL execute the AppleScript command `tell application "Google Chrome" to return URL of active tab of front window` via `std::process::Command::new("osascript")`
3. THE Browser_Observer SHALL poll Chrome every 3 seconds (Observer_Poll_Interval)
4. WHEN Chrome is not running, THE Browser_Observer SHALL handle the osascript error gracefully and continue polling without emitting errors to the frontend
5. WHEN Chrome has no open windows, THE Browser_Observer SHALL handle the AppleScript error gracefully and continue polling
6. WHEN the returned URL is identical to the previously observed URL, THE Browser_Observer SHALL skip processing (URL_Debounce)
7. WHEN the returned URL differs from the previously observed URL, THE Browser_Observer SHALL update its internal `last_url` state and proceed with URL classification
8. THE Browser_Observer SHALL use `tokio::select!` with a biased stop signal to allow clean shutdown (same pattern as Transcription_Manager)
9. WHEN the Browser_Observer is stopped, THE Browser_Observer SHALL send a stop signal via `tokio::sync::watch` channel and reset its internal state

### Requirement 2: YouTube Video Detection

**User Story:** As a JARVIS user, I want the application to automatically recognize when I open a YouTube video, so that it can offer to prepare a summary without me manually copying URLs.

#### Acceptance Criteria

1. WHEN a new URL is observed, THE Browser_Observer SHALL check if the URL matches the pattern `youtube.com/watch?v=` or `youtu.be/`
2. WHEN a YouTube video URL is detected, THE Browser_Observer SHALL extract the Video_ID from the URL query parameter `v`
3. WHEN a YouTube video URL is detected, THE Browser_Observer SHALL emit a `"youtube-video-detected"` Tauri event with payload `{ url: String, video_id: String }`
4. THE Browser_Observer SHALL use regex for URL pattern matching to handle variations (http/https, www prefix, additional query parameters)
5. WHEN the same YouTube video URL is detected consecutively, THE Browser_Observer SHALL NOT emit duplicate events (enforced by URL_Debounce in Requirement 1)
6. WHEN a non-YouTube URL is observed, THE Browser_Observer SHALL update `last_url` but SHALL NOT emit any event

### Requirement 3: Native macOS Notification

**User Story:** As a JARVIS user, I want to receive a desktop notification when a YouTube video is detected, so that I'm aware JARVIS can help even when the app window is not in focus.

#### Acceptance Criteria

1. WHEN a YouTube video is detected, THE Browser_Observer SHALL send a native macOS notification using `tauri_plugin_notification::NotificationExt`
2. THE notification SHALL have the title "YouTube Video Detected"
3. THE notification SHALL have the body "Open JarvisApp to prepare a gist"
4. THE application SHALL register the `tauri_plugin_notification` plugin in the Tauri builder chain via `.plugin(tauri_plugin_notification::init())`
5. THE application SHALL include `"notification:default"` in the capabilities permissions (`src-tauri/capabilities/default.json`)
6. WHEN the notification system is unavailable or permission is denied, THE Browser_Observer SHALL log a warning and continue operating without notifications

### Requirement 4: YouTube Page Scraping

**User Story:** As a JARVIS user, I want to get a quick summary of a YouTube video's content without watching the entire video, so that I can decide if it's worth my time.

#### Acceptance Criteria

1. WHEN the user requests a gist for a YouTube video, THE YouTube_Scraper SHALL fetch the YouTube page HTML using `reqwest::get(url)`
2. THE YouTube_Scraper SHALL extract the video title from the `<title>` HTML tag and strip the " - YouTube" suffix
3. THE YouTube_Scraper SHALL extract the video description from the `"shortDescription":"..."` field in the `ytInitialPlayerResponse` JSON embedded in the page HTML
4. THE YouTube_Scraper SHALL extract the channel name from the `"ownerChannelName":"..."` field in the `ytInitialPlayerResponse` JSON
5. THE YouTube_Scraper SHALL extract the video duration from the `"lengthSeconds":"..."` field in the `ytInitialPlayerResponse` JSON
6. THE YouTube_Scraper SHALL return a YouTube_Gist struct containing: url, video_id, title, channel, description, and duration_seconds
7. THE YouTube_Scraper SHALL use regex to parse the `ytInitialPlayerResponse` JSON from the page HTML
8. WHEN the YouTube page cannot be fetched (network error, 404, etc.), THE YouTube_Scraper SHALL return a descriptive error message
9. WHEN a specific field cannot be extracted from the page HTML, THE YouTube_Scraper SHALL use a fallback value ("Unknown" for strings, 0 for duration) rather than failing the entire operation
10. THE YouTube_Scraper SHALL NOT require any API key or authentication token

### Requirement 5: Tauri Commands

**User Story:** As a frontend developer, I want to control the browser observer and fetch YouTube gists through Tauri commands, so that I can build a reactive UI for the feature.

#### Acceptance Criteria

1. THE System SHALL expose a `start_browser_observer` Tauri command that starts the Browser_Observer polling loop
2. THE System SHALL expose a `stop_browser_observer` Tauri command that stops the Browser_Observer polling loop
3. THE System SHALL expose a `fetch_youtube_gist` Tauri command that accepts a `url: String` parameter and returns a `Result<YouTubeGist, String>`
4. THE System SHALL expose a `get_observer_status` Tauri command that returns whether the Browser_Observer is currently running
5. ALL commands SHALL follow the existing pattern of returning `Result<T, String>` for error handling
6. THE `start_browser_observer` command SHALL return an error if the observer is already running
7. THE `stop_browser_observer` command SHALL return an error if the observer is not running
8. THE Browser_Observer SHALL be managed as `Arc<tokio::sync::Mutex<BrowserObserver>>` in Tauri state (same pattern as Transcription_Manager)

### Requirement 6: Browser Observer State Management

**User Story:** As a system architect, I want the browser observer to follow the same state management patterns as existing managers, so that the codebase remains consistent and maintainable.

#### Acceptance Criteria

1. THE Browser_Observer SHALL be initialized in the Tauri `setup()` closure alongside other managers
2. THE Browser_Observer SHALL store an `AppHandle` for emitting events and sending notifications
3. THE Browser_Observer SHALL store a `stop_tx: Option<tokio::sync::watch::Sender<bool>>` for signaling the background task to stop
4. THE Browser_Observer SHALL store an `is_running: bool` flag to track observer state
5. THE Browser_Observer SHALL store a `last_url: String` to implement URL_Debounce
6. THE `browser` module SHALL be declared in `lib.rs` as `pub mod browser;`
7. THE browser module SHALL be organized as `src/browser/mod.rs`, `src/browser/observer.rs`, and `src/browser/youtube.rs`

### Requirement 7: YouTube Section UI

**User Story:** As a JARVIS user, I want a dedicated section in the app to see detected YouTube videos and their gists, so that I can review video summaries at a glance.

#### Acceptance Criteria

1. THE YouTube_Section SHALL be rendered as a modal overlay using the existing `dialog-overlay` + `settings-panel` CSS pattern (same as Settings component)
2. THE YouTube_Section SHALL display an observer status indicator showing whether the Browser_Observer is running or stopped
3. THE YouTube_Section SHALL provide a toggle button to start and stop the Browser_Observer
4. WHEN a `youtube-video-detected` event is received, THE YouTube_Section SHALL add the detected video to a list displayed in the panel
5. EACH detected video SHALL display a card showing the YouTube URL and a "Prepare Gist" button
6. WHEN the user clicks "Prepare Gist", THE YouTube_Section SHALL call the `fetch_youtube_gist` command and display a loading state
7. WHEN the gist is fetched successfully, THE YouTube_Section SHALL display the gist in the format: "Gist of <YouTube URL>" followed by title, channel, duration, and description
8. THE YouTube_Section SHALL provide a "Dismiss" button on each gist card to remove it from the list
9. THE YouTube_Section SHALL provide a "Copy" button on each gist card to copy the gist text to the clipboard
10. THE YouTube_Section SHALL have a close button (×) in the header to dismiss the panel
11. WHEN multiple YouTube videos are detected, THE YouTube_Section SHALL display them as a scrollable list with the most recent at the top

### Requirement 8: Hamburger Menu Navigation

**User Story:** As a JARVIS user, I want a navigation menu to access the YouTube section and future feature sections, so that the app header stays clean as more features are added.

#### Acceptance Criteria

1. THE application SHALL display a hamburger menu button (☰) in the header next to the existing settings gear button (⚙️)
2. WHEN the hamburger button is clicked, THE application SHALL display a dropdown menu with available feature sections
3. THE dropdown menu SHALL include a "YouTube" option with a video icon
4. WHEN a `youtube-video-detected` event is received, THE hamburger button SHALL display a notification badge (small red dot) to indicate new activity
5. WHEN the user opens the YouTube_Section, THE notification badge SHALL be cleared
6. WHEN the user clicks outside the dropdown menu, THE dropdown SHALL close
7. THE dropdown menu SHALL be positioned below the hamburger button, aligned to the right edge
8. THE hamburger button SHALL follow the same styling pattern as the existing settings button (circular, border, hover effect)

### Requirement 9: Frontend Types and Events

**User Story:** As a frontend developer, I want typed interfaces for YouTube data and events, so that the TypeScript code is type-safe and consistent with the Rust backend.

#### Acceptance Criteria

1. THE frontend SHALL define a `YouTubeGist` interface matching the Rust struct with fields: `url`, `video_id`, `title`, `channel`, `description`, `duration_seconds`
2. THE frontend SHALL define a `YouTubeDetectedEvent` interface with fields: `url`, `video_id`
3. THE types SHALL be defined in `src/state/types.ts` alongside existing type definitions
4. THE `YouTubeSection` component SHALL be exported from `src/components/index.ts`
5. THE frontend SHALL listen for `youtube-video-detected` events using the existing `useTauriEvent` hook or `listen()` from `@tauri-apps/api/event`

### Requirement 10: Dependencies and Configuration

**User Story:** As a developer, I want minimal new dependencies, so that the build stays fast and the binary size doesn't grow unnecessarily.

#### Acceptance Criteria

1. THE System SHALL add `tauri-plugin-notification = "2"` to `src-tauri/Cargo.toml` dependencies
2. THE System SHALL move `regex = "1"` from `[dev-dependencies]` to `[dependencies]` in `src-tauri/Cargo.toml`
3. THE System SHALL NOT add any new frontend npm packages (uses existing Tauri API)
4. THE System SHALL reuse the existing `reqwest` dependency (already present with `stream`, `blocking`, `json` features) for YouTube page fetching
5. THE System SHALL add `"notification:default"` to the permissions array in `src-tauri/capabilities/default.json`

### Requirement 11: Error Handling

**User Story:** As a user, I want the browser observer to handle errors gracefully, so that Chrome being closed or YouTube changing its page format doesn't crash the application.

#### Acceptance Criteria

1. WHEN Chrome is not running, THE Browser_Observer SHALL silently continue polling without emitting error events
2. WHEN Chrome has no windows or tabs, THE Browser_Observer SHALL silently continue polling
3. WHEN the YouTube page HTML cannot be fetched, THE YouTube_Scraper SHALL return an error message that the frontend displays to the user
4. WHEN a regex pattern fails to match expected fields in the YouTube HTML, THE YouTube_Scraper SHALL use fallback values ("Unknown" for title/channel, empty string for description, 0 for duration)
5. WHEN the notification plugin fails to send a notification, THE Browser_Observer SHALL log the error and continue operating
6. WHEN the observer background task panics, THE System SHALL catch the error and set `is_running` to false
7. ALL error messages returned from commands SHALL be user-friendly strings (not raw Rust error types)

### Requirement 12: Performance

**User Story:** As a user, I want the browser observer to run in the background without affecting recording or transcription performance.

#### Acceptance Criteria

1. THE Browser_Observer SHALL run its polling loop in a separate tokio task that does not block the main thread
2. THE AppleScript execution SHALL timeout after 2 seconds to prevent hanging if Chrome is unresponsive
3. THE Browser_Observer polling interval of 3 seconds SHALL be sufficient to detect navigation without excessive CPU usage
4. THE YouTube page fetch (reqwest) SHALL timeout after 10 seconds to prevent blocking on slow connections
5. THE Browser_Observer SHALL NOT interfere with the recording or transcription pipelines (separate tokio task, no shared locks)
6. WHEN the observer is stopped, THE background task SHALL terminate within one poll interval (3 seconds)
