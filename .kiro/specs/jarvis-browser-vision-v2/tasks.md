# Implementation Plan: JARVIS Browser Vision v2

## Overview

This implementation transforms the browser observation system from manual to automatic, adding proactive background monitoring, intelligent deduplication, quick metadata via oEmbed API, and seamless notification-to-app navigation. The implementation builds on existing v1 infrastructure while adding auto-start logic, video-ID deduplication, oEmbed integration, notification click handling, and settings control.

## Tasks

- [ ] 1. Backend - Settings Infrastructure
  - [x] 1.1 Add BrowserSettings struct to settings/manager.rs
    - Add `BrowserSettings` struct with `observer_enabled: bool` field
    - Implement `Default` trait returning `observer_enabled: true`
    - Add `#[serde(default)]` annotation to `browser` field in `Settings` struct
    - Update `Settings::default()` to include `BrowserSettings::default()`
    - Export `BrowserSettings` from settings module (add to mod.rs or re-export location)
    - Ensure `BrowserSettings` is public so commands.rs can use it as return type
    - _Requirements: 1.3, 6.2, 6.3_
  
  - [ ]* 1.2 Write property test for settings persistence
    - **Property 10: Settings persistence round-trip**
    - **Validates: Requirements 6.7**
  
  - [x] 1.3 Add browser settings commands to commands.rs
    - Implement `update_browser_settings` command with observer state control
    - Implement `get_browser_settings` command
    - Add both commands to `invoke_handler` in lib.rs
    - _Requirements: 6.4, 6.5, 6.6_
  
  - [ ]* 1.4 Write property test for settings toggle controls observer
    - **Property 21: Settings toggle controls observer state**
    - **Validates: Requirements 6.4, 6.5**

- [ ] 2. Backend - Observer Enhancements
  - [x] 2.1 Add video-ID deduplication to browser/observer.rs
    - Add local `HashSet<String>` for `seen_video_ids` in polling loop (same scope as `last_url`)
    - Extract video ID from detected YouTube URLs
    - Check video ID against seen set before emitting events
    - Add video ID to seen set after first detection
    - _Requirements: 2.1, 2.2, 2.3, 2.4_
  
  - [ ]* 2.2 Write property test for video-ID deduplication
    - **Property 3: Video-ID deduplication prevents duplicate notifications**
    - **Validates: Requirements 2.2, 2.3, 2.4**
  
  - [ ]* 2.3 Write property test for deduplication set clearing
    - **Property 4: Observer restart clears deduplication set**
    - **Validates: Requirements 2.5**
  
  - [x] 2.4 Add oEmbed API integration
    - Create `QuickMetadata` struct with `title`, `author_name`, `thumbnail_url` fields
    - Implement `fetch_oembed_metadata` function using reqwest with `.query()` method
    - Set 3-second timeout on oEmbed requests
    - Handle success and failure cases with appropriate error logging
    - **LOCATION**: Prefer browser/youtube.rs (alongside scrape_youtube_gist) for better separation of concerns, but browser/observer.rs is acceptable
    - If placed in youtube.rs, ensure it's pub(crate) and imported in observer.rs
    - _Requirements: 3.1, 3.2, 3.3_
  
  - [ ]* 2.5 Write property test for oEmbed timeout
    - **Property 5: oEmbed fetch includes title in notification**
    - **Validates: Requirements 3.4**
  
  - [x] 2.6 Update YouTubeDetectedEvent struct in browser/observer.rs
    - **BREAKING CHANGE**: Update the Rust struct definition to add new fields
    - Add optional `title: Option<String>` field to struct
    - Add optional `author_name: Option<String>` field to struct
    - Update event emission logic to populate metadata when available
    - Emit single event per video detection after oEmbed completes (or times out)
    - **NOTE**: This struct change must be coordinated with Task 6.1 (TypeScript interface update)
    - _Requirements: 4.4, 8.1, 8.2_
  
  - [ ]* 2.7 Write property test for single event emission
    - **Property 7: Single event emission per video detection**
    - **Validates: Requirements 3.6**
  
  - [x] 2.8 Implement smart notifications in browser/observer.rs
    - Create `send_notification_with_title` function for successful oEmbed
    - Create `send_notification_generic` function for failed oEmbed
    - Use notification body format: "You're watching: {title}. Want me to keep a gist?"
    - Use generic body: "New YouTube video detected. Open JarvisApp to prepare a gist."
    - Send notification immediately after oEmbed completes within the same tokio::spawn from task 2.6
    - _Requirements: 4.1, 4.2, 4.3_
  
  - [ ]* 2.9 Write property test for notification before scrape
    - **Property 8: Notification sent before full scrape**
    - **Validates: Requirements 10.4**
  
  - [ ]* 2.10 Write property test for lightweight oEmbed only
    - **Property 9: Observer only performs lightweight oEmbed fetch**
    - **Validates: Requirements 10.5**

- [ ] 3. Backend - Auto-Start Integration
  - [x] 3.1 Add observer auto-start to lib.rs setup closure
    - Load settings in setup closure
    - Check `settings.browser.observer_enabled` flag
    - Auto-start observer if enabled, following ShortcutManager pattern
    - Log errors without crashing if observer fails to start
    - _Requirements: 1.1, 1.2, 1.4_
  
  - [ ]* 3.2 Write property test for settings-based auto-start
    - **Property 1: Settings-based auto-start control**
    - **Validates: Requirements 1.2**
  
  - [ ]* 3.3 Write property test for observer error handling
    - **Property 2: Observer error handling during startup**
    - **Validates: Requirements 1.4**

- [x] 4. Checkpoint - Backend validation
  - Ensure all backend tests pass, verify observer auto-starts correctly, ask the user if questions arise.

- [ ] 5. Frontend - Dependencies
  - [x] 5.1 Install notification plugin npm package
    - Run `npm install @tauri-apps/plugin-notification` in jarvis-app directory
    - Verify package.json includes the dependency
    - Run type check to verify import works: `npm run check` or `tsc --noEmit`
    - Ensure no TypeScript errors related to the notification plugin
    - _Requirements: 5.1_

- [ ] 6. Frontend - Type Definitions and YouTubeSection Updates
  - [x] 6.1 Update YouTubeDetectedEvent interface in src/state/types.ts
    - Add optional `title?: string` field to YouTubeDetectedEvent interface
    - Add optional `author_name?: string` field to YouTubeDetectedEvent interface
    - This is the canonical type definition used throughout the app
    - _Requirements: 8.1_
  
  - [x] 6.2 Add DetectedVideo interface to YouTubeSection.tsx
    - Create DetectedVideo interface extending YouTubeDetectedEvent
    - Add optional `gist?: YouTubeGist` field
    - Add optional `loading?: boolean` field
    - Add optional `error?: string` field
    - Import YouTubeDetectedEvent and YouTubeGist from '../state/types'
    - _Requirements: 8.1_
  
  - [x] 6.3 Update event listener to handle enhanced payload
    - Update `youtube-video-detected` event handler to extract title and author_name
    - Store metadata in DetectedVideo state
    - Add new videos at top of list (most recent first)
    - _Requirements: 7.2, 8.3, 8.4_
  
  - [ ]* 6.4 Write property test for video list ordering
    - **Property 11: Video list displays most recent first**
    - **Validates: Requirements 7.2**
  
  - [x] 6.5 Update VideoCard component to display title when available
    - Display `video.title` if present, otherwise display `video.url`
    - Display `video.author_name` if present
    - _Requirements: 7.3, 7.4_
  
  - [ ]* 6.6 Write property test for title display
    - **Property 12: Video card displays title when available**
    - **Validates: Requirements 7.3, 8.3**
  
  - [ ]* 6.7 Write property test for URL fallback
    - **Property 13: Video card displays URL when title unavailable**
    - **Validates: Requirements 7.4, 8.4**
  
  - [x] 6.8 Remove observer start/stop toggle from YouTubeSection.tsx
    - Remove `isRunning` state variable
    - Remove `handleToggleObserver` function
    - Remove useEffect that loads observer status on mount
    - Remove observer status UI (toggle button/switch)
    - Remove any calls to `start_browser_observer`, `stop_browser_observer`, or `get_observer_status` commands
    - Observer is now managed exclusively via Settings panel
    - _Requirements: 7.1_
  
  - [ ]* 6.9 Write property test for gist metadata display
    - **Property 14: Gist display includes all metadata fields**
    - **Validates: Requirements 7.7**

- [ ] 7. Frontend - App Integration
  - [x] 7.1 Add notification click handler to App.tsx
    - Import `onAction` from `@tauri-apps/plugin-notification`
    - Add useEffect to listen for notification actions
    - Set `showYouTube` to true when notification is clicked
    - Clear notification badge when YouTube section opens
    - _Requirements: 5.1, 5.2, 5.3_
  
  - [x] 7.2 Update dialog overlay pattern in App.tsx
    - Ensure YouTubeSection renders as child of dialog-overlay div
    - Add click handler to overlay that only closes on overlay click (not children)
    - Use `e.target === e.currentTarget` check
    - _Requirements: 5.2_
  
  - [x] 7.3 Update notification badge logic in App.tsx
    - Listen for `youtube-video-detected` events
    - Set `youtubeNotification` badge when event received and YouTube section closed
    - Clear badge when YouTube section opens
    - _Requirements: 7.9_

- [ ] 8. Frontend - Settings Panel
  - [x] 8.1 Add browser settings section to Settings.tsx
    - Add `BrowserSettings` interface with `observer_enabled: bool`
    - Load browser settings on mount using `get_browser_settings` command
    - Add "Browser" section with observer toggle checkbox
    - Add descriptive text: "Automatically detect YouTube videos in Chrome and offer to prepare gists"
    - _Requirements: 6.3, 6.4_
  
  - [x] 8.2 Implement observer toggle handler in Settings.tsx
    - Call `update_browser_settings` command when toggle changes
    - Update local state on success
    - Log errors on failure
    - _Requirements: 6.4, 6.5_

- [x] 9. Checkpoint - Frontend validation
  - Ensure all frontend components render correctly, verify notification clicks open YouTube section, ask the user if questions arise.

- [ ] 10. Integration Testing
  - [ ]* 10.1 Write property test for v1 command compatibility
    - **Property 15: v1 commands remain functional**
    - **Validates: Requirements 9.1, 9.2, 9.3**
  
  - [ ]* 10.2 Verify existing Chrome polling tests still pass
    - Run existing v1 tests for Chrome polling mechanism
    - Verify AppleScript execution, 3s interval, and 2s timeout unchanged
    - **Property 16: Chrome polling mechanism unchanged**
    - **Validates: Requirements 9.4**
    - **NOTE**: This is a regression check - existing tests cover this behavior
  
  - [ ]* 10.3 Verify existing URL detection tests still pass
    - Run existing v1 tests for YouTube URL detection regex
    - Verify all v1-supported URL formats still detected correctly
    - **Property 17: YouTube URL detection regex unchanged**
    - **Validates: Requirements 9.5**
    - **NOTE**: This is a regression check - existing tests cover this behavior
  
  - [ ]* 10.4 Verify existing scraper tests still pass
    - Run existing v1 tests for scrape_youtube_gist function
    - Verify same result structure and behavior as v1
    - **Property 18: Full YouTube scraper unchanged**
    - **Validates: Requirements 9.6**
    - **NOTE**: This is a regression check - existing tests cover this behavior
  
  - [ ]* 10.5 Write property test for subsystem isolation
    - **Property 19: Other subsystems unaffected**
    - **Validates: Requirements 9.8**
  
  - [ ]* 10.6 Write property test for frontend deduplication logic
    - **Property 20: Frontend adds new videos without deduplication logic**
    - **Validates: Requirements 8.3, 8.4**

- [x] 11. Final checkpoint - End-to-end validation
  - Ensure all tests pass, verify complete workflow from auto-start to notification to gist preparation, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation at backend, frontend, and integration levels
- Property tests validate universal correctness properties from the design document
- The implementation maintains full backward compatibility with v1 commands and infrastructure
- oEmbed API provides fast notifications (~200-600ms) while full scraping happens only on user request
- Video-ID deduplication is session-scoped (cleared on observer restart)
- Settings persistence uses existing SettingsManager infrastructure with `#[serde(default)]` for backward compatibility
