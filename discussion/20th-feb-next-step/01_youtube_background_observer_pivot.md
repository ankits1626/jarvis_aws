# Pivot: YouTube Background Observer — Proactive Gist Suggestions

## Current Flow (What We Built)

1. User opens JARVIS app
2. User clicks hamburger menu -> YouTube
3. User clicks "Start" to start the browser observer
4. Observer polls Chrome every 3s
5. When YouTube video detected -> generic notification "YouTube Video Detected"
6. User opens JARVIS -> YouTube section -> clicks "Prepare Gist" on the detected video
7. Gist is fetched and displayed

**Problem**: Too many manual steps. User has to proactively open the YouTube section and start the observer. The notification is generic and doesn't tell the user what video was detected.

## New Flow (What We Want)

1. User launches JARVIS — observer starts automatically in the background
2. User browses normally in Chrome
3. When user opens a YouTube video, JARVIS detects it and:
   - Quickly fetches the video title from the YouTube page
   - Shows a native macOS notification:
     - Title: "New YouTube Video"
     - Body: "You're watching: [Video Title]. Would you like me to keep a gist?"
4. User clicks the notification -> JARVIS app comes to foreground with the YouTube section open, showing the detected video with a "Prepare Gist" button ready to go
5. User clicks "Prepare Gist" -> full gist is fetched and displayed

**Key shift**: JARVIS is a background assistant that proactively offers help. The user doesn't need to configure or start anything.

## What Changes

### Backend (`observer.rs`)

- **Auto-start**: Observer starts in `setup()` during app init — no manual toggle needed
- **Smarter classify_url**: When a YouTube URL is detected, do a quick fetch of the page title BEFORE sending the notification
- **Richer notification**: Include the video title in the notification body
- **Notification action**: Clicking the notification should bring JARVIS to foreground and open the YouTube section

### Backend (`lib.rs`)

- Start BrowserObserver automatically in `setup()` after creating it
- Remove or keep start/stop commands (keep for power users, or remove entirely?)

### Frontend (`App.tsx`)

- Remove manual observer Start/Stop toggle from YouTube section (or move to Settings as advanced option)
- When app comes to foreground via notification click, auto-open YouTube section
- Notification badge on hamburger menu still useful as a fallback indicator

### Frontend (`YouTubeSection.tsx`)

- Remove observer toggle UI (Start/Stop button)
- Focus on displaying detected videos and gist preparation
- Maybe auto-prepare gist when user opens the section (since they clicked "yes" on the notification)

## Open Questions

1. **Notification click action**: Does `tauri-plugin-notification` v2 support click actions that bring the app to foreground and trigger a specific view? If not, the notification is just informational and the user opens the app manually.

2. **Quick title fetch**: Should we do a lightweight fetch (just get `<title>` tag) for the notification, or use the full scraper? The full scraper takes a few seconds — might delay the notification.

3. **Auto-gist**: When the user responds to the notification, should JARVIS auto-prepare the gist? Or still require a manual "Prepare Gist" click?

4. **Multiple videos**: If the user opens 5 YouTube videos in quick succession, do we notify for each one? Or debounce/batch?

5. **Observer always-on**: Should there be any way to disable the observer, or is it always running? (Maybe a toggle in Settings for users who don't want this feature)
