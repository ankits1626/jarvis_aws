# Requirements Document

## Introduction

The JARVIS Browser Vision v2 module evolves the browser observation system from a manual, user-initiated flow into a proactive background assistant. In v1, the user had to manually open the YouTube section and start the observer. In v2, JARVIS automatically observes Chrome in the background from the moment the app launches. When the user navigates to a YouTube video, JARVIS fetches the video title via YouTube's oEmbed API (~200-600ms) and shows a native macOS notification: "You're watching: [Title]. Would you like a gist?" Clicking the notification brings JARVIS to the foreground with the YouTube section open. The user can then click "Prepare Gist" to fetch the full summary. This module builds on the existing v1 infrastructure (AppleScript polling, reqwest scraping, tauri-plugin-notification) and adds: auto-start observer, oEmbed quick metadata, video-ID deduplication, notification click handling, and a settings toggle.

## Glossary

- **Browser_Observer**: A background polling service that queries Chrome's active tab URL every 3 seconds via macOS AppleScript. In v2, it auto-starts on app launch.
- **oEmbed_API**: YouTube's lightweight metadata endpoint (`youtube.com/oembed?url=...&format=json`) that returns title, author, and thumbnail in ~1-2 KB. Used for fast notification content.
- **YouTube_Scraper**: The existing component that fetches a full YouTube page via HTTP and extracts metadata (title, channel, description, duration) from `ytInitialPlayerResponse` JSON. Used for detailed gists.
- **YouTube_Gist**: A structured summary of a YouTube video containing URL, video ID, title, channel name, description text, and duration.
- **Quick_Metadata**: A lightweight struct containing title, author, and thumbnail URL, fetched via oEmbed_API for notification display.
- **Video_ID_Dedup**: A mechanism using a HashSet of seen video IDs to prevent duplicate notifications for the same video within a session.
- **Notification_Click_Handler**: Logic that responds to the user clicking a notification by bringing JARVIS to the foreground and opening the YouTube section.
- **Observer_Settings**: User-configurable settings for the browser observer, including an enabled/disabled toggle, persisted to `~/.jarvis/settings.json`.

## Requirements

### Requirement 1: Auto-Start Observer on App Launch

**User Story:** As a JARVIS user, I want the browser observer to start automatically when I launch the app, so that I don't have to manually enable it every time.

#### Acceptance Criteria

1. WHEN the application starts, THE System SHALL auto-start the Browser_Observer in the `setup()` closure, following the same pattern as ShortcutManager
2. THE System SHALL check Observer_Settings before auto-starting; IF `observer_enabled` is `false`, THE System SHALL NOT start the observer
3. THE Observer_Settings SHALL default to `observer_enabled: true` for new installations
4. WHEN the observer fails to start (e.g., system error), THE System SHALL log the error and continue app startup without crashing
5. THE System SHALL NOT require the user to open the YouTube section or click any button to begin observing

### Requirement 2: Video-ID Based Deduplication

**User Story:** As a JARVIS user, I want to receive only one notification per YouTube video, so that I'm not spammed when I navigate back to a video I already saw.

#### Acceptance Criteria

1. THE Browser_Observer SHALL maintain a `HashSet<String>` of seen video IDs within the polling loop
2. WHEN a YouTube URL is detected, THE Browser_Observer SHALL extract the video ID and check it against the seen set
3. IF the video ID is already in the seen set, THE Browser_Observer SHALL skip notification and event emission
4. IF the video ID is NOT in the seen set, THE Browser_Observer SHALL add it and proceed with notification
5. THE seen set SHALL be cleared when the observer is stopped and restarted
6. THE seen set SHALL persist for the lifetime of the observer session (not across app restarts)
7. THE URL comparison (`last_url`) SHALL remain as a first-pass optimization to avoid regex on every poll

### Requirement 3: Quick Metadata via oEmbed API

**User Story:** As a JARVIS user, I want the notification to include the video title, so that I know which video JARVIS detected without switching to the app.

#### Acceptance Criteria

1. WHEN a new YouTube video is detected, THE Browser_Observer SHALL fetch Quick_Metadata from `https://www.youtube.com/oembed?url={VIDEO_URL}&format=json`
2. THE oEmbed request SHALL timeout after 3 seconds to keep notification latency low
3. THE Quick_Metadata struct SHALL contain: `title: String`, `author_name: String`, `thumbnail_url: String`
4. WHEN the oEmbed fetch succeeds, THE Browser_Observer SHALL include the video title in the notification body
5. WHEN the oEmbed fetch fails (network error, private video, etc.), THE Browser_Observer SHALL fall back to a generic notification without the title: "New YouTube video detected. Open JarvisApp to prepare a gist."
6. THE oEmbed fetch SHALL NOT block or delay the `youtube-video-detected` event emission; the event SHALL be emitted regardless of oEmbed success

### Requirement 4: Smart Notification with Video Title

**User Story:** As a JARVIS user, I want a friendly notification that tells me what video I'm watching and offers to help, so that JARVIS feels like a proactive assistant.

#### Acceptance Criteria

1. WHEN a YouTube video is detected AND oEmbed succeeds, THE notification SHALL have:
   - Title: "YouTube Video Detected"
   - Body: "You're watching: {title}. Want me to keep a gist?"
2. WHEN a YouTube video is detected AND oEmbed fails, THE notification SHALL have:
   - Title: "YouTube Video Detected"
   - Body: "New YouTube video detected. Open JarvisApp to prepare a gist."
3. THE notification SHALL be sent via `tauri_plugin_notification::NotificationExt` (existing infrastructure)
4. THE `youtube-video-detected` event payload SHALL be extended to include optional Quick_Metadata: `{ url, video_id, title?, author_name? }`

### Requirement 5: Notification Click — Bring App to Foreground

**User Story:** As a JARVIS user, I want to click the notification to open JARVIS and see the detected video, so that I can quickly prepare a gist without searching for the app.

#### Acceptance Criteria

1. WHEN the user clicks a notification, THE System SHALL bring the JARVIS main window to the foreground using `window.set_focus()` and `window.unminimize()`
2. WHEN the app is brought to foreground via notification click, THE System SHALL emit an event to the frontend to auto-open the YouTube section
3. THE frontend SHALL listen for the "open-youtube-section" event and set `showYouTube` to `true`
4. IF the notification click handler is unavailable on the platform, THE System SHALL log a warning and degrade gracefully (notification still shows, just no click action)

### Requirement 6: Observer Settings Toggle

**User Story:** As a JARVIS user, I want to disable the browser observer if I don't want JARVIS watching my browsing, so that I'm in control of my privacy.

#### Acceptance Criteria

1. THE Settings struct SHALL include a `browser` field containing `BrowserSettings { observer_enabled: bool }`
2. THE BrowserSettings SHALL default to `observer_enabled: true`
3. THE Settings UI SHALL display a toggle for "Browser Observer" in the Settings panel
4. WHEN the user toggles the observer OFF, THE System SHALL stop the Browser_Observer and persist the setting
5. WHEN the user toggles the observer ON, THE System SHALL start the Browser_Observer and persist the setting
6. THE setting SHALL be persisted to `~/.jarvis/settings.json` using the existing SettingsManager infrastructure
7. THE setting SHALL survive app restarts (read on startup, respected by auto-start logic)

### Requirement 7: Simplified YouTube Section UI

**User Story:** As a JARVIS user, I want the YouTube section to focus on showing detected videos and gists, without requiring me to manage the observer manually.

#### Acceptance Criteria

1. THE YouTube_Section SHALL NOT display an observer Start/Stop toggle (observer is managed via Settings)
2. THE YouTube_Section SHALL display a list of detected videos, most recent at top
3. EACH video card SHALL display the video title (from oEmbed Quick_Metadata) and URL
4. WHEN Quick_Metadata is not available for a video, THE card SHALL display only the URL
5. EACH video card SHALL have a "Prepare Gist" button that calls the existing `fetch_youtube_gist` command
6. WHEN "Prepare Gist" is clicked, THE card SHALL show a loading indicator
7. WHEN the gist is ready, THE card SHALL display: title, channel, duration, and description
8. EACH gist card SHALL have "Copy" and "Dismiss" buttons (existing functionality)
9. THE YouTube_Section SHALL still be accessible via the hamburger menu

### Requirement 8: Extended Event Payload

**User Story:** As a frontend developer, I want the youtube-video-detected event to include metadata when available, so that the UI can display video info immediately without an extra fetch.

#### Acceptance Criteria

1. THE `youtube-video-detected` event payload SHALL be: `{ url: String, video_id: String, title: Option<String>, author_name: Option<String> }`
2. THE frontend `YouTubeDetectedEvent` TypeScript interface SHALL be updated to include optional `title` and `author_name` fields
3. WHEN the frontend receives an event with a title, THE YouTube_Section SHALL display the title in the video card immediately
4. WHEN the frontend receives an event without a title, THE YouTube_Section SHALL display only the URL

### Requirement 9: Existing Functionality Preservation

**User Story:** As a developer, I want the v2 changes to build on v1 without breaking existing features.

#### Acceptance Criteria

1. THE `start_browser_observer` and `stop_browser_observer` Tauri commands SHALL remain available for programmatic control
2. THE `fetch_youtube_gist` command SHALL remain unchanged
3. THE `get_observer_status` command SHALL remain unchanged
4. THE Chrome polling mechanism (AppleScript + 3s interval + 2s timeout) SHALL remain unchanged
5. THE YouTube URL detection regex SHALL remain unchanged
6. THE full YouTube scraper (`scrape_youtube_gist`) SHALL remain unchanged
7. THE hamburger menu and notification badge SHALL remain unchanged
8. THE recording and transcription pipelines SHALL NOT be affected

### Requirement 10: Performance

**User Story:** As a user, I want JARVIS to detect videos and show notifications quickly without slowing down my system.

#### Acceptance Criteria

1. THE oEmbed API fetch SHALL complete within 3 seconds (timeout) to keep notification latency under 4 seconds total (3s poll + 3s fetch worst case, ~3.5s typical)
2. THE oEmbed fetch SHALL run in the same tokio task as the observer, using `tokio::spawn` for the HTTP request to avoid blocking the poll loop
3. THE Video_ID_Dedup HashSet SHALL have O(1) lookup performance
4. THE notification SHALL be sent immediately after oEmbed completes (or fails), not after the full scrape
5. THE observer SHALL NOT perform the full YouTube scrape automatically; it SHALL only do the lightweight oEmbed fetch
