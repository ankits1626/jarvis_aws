# Browser Vision — YouTube Detection + Gist

**Date**: Feb 19, 2026
**Goal**: JARVIS observes your Chrome browser. When you open a YouTube video, it prompts via native macOS notification. If you say yes, it scrapes the page and shows a gist in a dedicated YouTube section.

---

## How It Works

```
Chrome Browser (user opens YouTube video)
       │
       │  AppleScript polls active tab URL every 3s
       ▼
┌─────────────────────┐
│  BrowserObserver     │  ← background thread in Rust
│  (osascript polling) │
└──────────┬──────────┘
           │
           │  YouTube URL detected
           ▼
┌─────────────────────┐
│  macOS Notification  │  "YouTube Video Detected"
│  (tauri-plugin-      │  "Open JarvisApp to prepare a gist"
│   notification)      │
└──────────┬──────────┘
           │
           │  User opens app / clicks "Prepare Gist"
           ▼
┌─────────────────────┐
│  YouTube Scraper     │  GET youtube.com page → parse HTML
│  (reqwest + regex)   │  extract: title, channel, description, duration
└──────────┬──────────┘
           │
           ▼
┌─────────────────────────────────────┐
│  YouTube Section (hamburger menu)    │
│                                      │
│  Gist of https://youtube.com/...     │
│  Title: How to Build AI Agents       │
│  Channel: TechWithTim                │
│  Duration: 15:32                     │
│  Description: This video covers...   │
│                                      │
│  [Copy] [Dismiss]                    │
└──────────────────────────────────────┘
```

---

## Backend (Rust) — New Files

### 1. `src-tauri/src/browser/mod.rs`

Module declaration. Exposes `observer` and `youtube` submodules.

### 2. `src-tauri/src/browser/observer.rs` — BrowserObserver

**Struct fields:**
- `app_handle: AppHandle`
- `stop_tx: Option<watch::Sender<bool>>` — stop signal for background task
- `is_running: bool`
- `last_url: String` — debounce (skip if URL unchanged)

**`start()` method:**
- Spawns a `tokio::spawn` background task
- Every 3 seconds, runs AppleScript via `std::process::Command`:
  ```
  osascript -e 'tell application "Google Chrome" to return URL of active tab of front window'
  ```
- Compares returned URL to `last_url` — if unchanged, skip
- If URL matches `youtube.com/watch?v=` regex:
  - Extract `video_id` from URL
  - Emit Tauri event: `"youtube-video-detected"` with `{ url, video_id }`
  - Send native macOS notification via `tauri_plugin_notification::NotificationExt`:
    - Title: "YouTube Video Detected"
    - Body: "Open JarvisApp to prepare a gist"
- Uses `tokio::select!` with biased stop signal (same pattern as TranscriptionManager)

**`stop()` method:**
- Sends stop signal via watch channel
- Resets `is_running` and `last_url`

**Edge cases:**
- Chrome not running → `osascript` returns error → catch gracefully, no crash
- No window open → same handling
- Non-YouTube URL → update `last_url` but don't emit event
- Same YouTube video → debounce via `last_url` comparison

**Managed state:** `Arc<tokio::sync::Mutex<BrowserObserver>>` (async Mutex, same as TranscriptionManager)

### 3. `src-tauri/src/browser/youtube.rs` — YouTube Page Scraper

**Function:** `fetch_youtube_gist(url: &str) -> Result<YouTubeGist, String>`

**How it works (no API key needed):**
1. `reqwest::get(url)` — fetch the full YouTube page HTML
2. Parse with regex:
   - `<title>` tag → video title (strip " - YouTube" suffix)
   - `"shortDescription":"..."` from `ytInitialPlayerResponse` → description
   - `"ownerChannelName":"..."` → channel name
   - `"lengthSeconds":"..."` → duration in seconds
3. Return `YouTubeGist` struct

**`YouTubeGist` struct (Serde serializable):**
```
url: String
video_id: String
title: String
channel: String
description: String
duration_seconds: u64
```

**Why no API key:** YouTube embeds all this metadata in the page HTML inside `ytInitialPlayerResponse` JSON. The Python `youtube-transcript-api` library uses this same approach.

---

## Backend — Modified Files

### 4. `src-tauri/Cargo.toml`

Add dependencies:
```toml
tauri-plugin-notification = "2"
regex = "1"   # currently in dev-dependencies, move to dependencies
```

`reqwest` already present with `stream`, `blocking`, `json` features — sufficient for YouTube fetch.

### 5. `src-tauri/src/lib.rs`

- Add `pub mod browser;`
- Add `.plugin(tauri_plugin_notification::init())` to builder chain
- In `setup()`: create `BrowserObserver::new(app.handle().clone())`, manage as `Arc<tokio::sync::Mutex<BrowserObserver>>`
- Add 3 new commands to `invoke_handler`: `start_browser_observer`, `stop_browser_observer`, `fetch_youtube_gist`

### 6. `src-tauri/src/commands.rs`

3 new commands:

| Command | Args | Returns | What it does |
|---------|------|---------|-------------|
| `start_browser_observer` | none | `Result<(), String>` | Acquires BrowserObserver lock, calls `start()` |
| `stop_browser_observer` | none | `Result<(), String>` | Acquires BrowserObserver lock, calls `stop()` |
| `fetch_youtube_gist` | `url: String` | `Result<YouTubeGist, String>` | Calls `youtube::fetch_youtube_gist(&url)` |

All follow existing pattern: `Result<T, String>` return type, state access via `State<'_>`.

### 7. `src-tauri/capabilities/default.json`

Add `"notification:default"` to permissions array. This grants notification send/request permissions.

---

## Frontend (React/TypeScript) — New Files

### 8. `src/components/YouTubeSection.tsx`

Settings-panel style overlay (reuses `dialog-overlay` + `settings-panel` CSS classes).

**State:**
- `observerRunning: boolean`
- `detectedVideos: YouTubeGist[]`
- `loadingGist: string | null` (video_id being fetched)

**Behavior:**
- Toggle button: Start/Stop Observer → calls `start_browser_observer` / `stop_browser_observer`
- Listens to `youtube-video-detected` event → adds to `detectedVideos` list
- Each detected video shows as a card with "Prepare Gist" button
- Click "Prepare Gist" → calls `fetch_youtube_gist` command → displays result

**Display format:**
```
┌─ YouTube ─────────────────────── [×] ─┐
│                                        │
│  Observer: [● Running] [Stop]          │
│                                        │
│  ┌─ Detected Video ────────────────┐   │
│  │ Gist of https://youtu.be/...    │   │
│  │                                  │   │
│  │ Title: How to Build AI Agents   │   │
│  │ Channel: TechWithTim            │   │
│  │ Duration: 15:32                 │   │
│  │                                  │   │
│  │ Description:                    │   │
│  │ This video covers building AI   │   │
│  │ agents using AWS Bedrock and    │   │
│  │ Strands SDK. We'll walk through │   │
│  │ setting up the environment...   │   │
│  │                                  │   │
│  │ [Copy Gist] [Dismiss]          │   │
│  └──────────────────────────────────┘   │
│                                        │
└────────────────────────────────────────┘
```

---

## Frontend — Modified Files

### 9. `src/App.tsx`

- Add `showYouTube` state (boolean)
- Add `youtubeNotification` state (boolean — badge indicator)
- Add hamburger menu button (☰) next to settings gear in header
- Hamburger dropdown with "YouTube" option (shows badge dot when video detected)
- Listen to `youtube-video-detected` event → set badge indicator
- Render `YouTubeSection` in `dialog-overlay` when `showYouTube` is true

**Header layout change:**
```
┌─────────────────────────────────────┐
│  JarvisApp              [☰] [⚙️]   │
└─────────────────────────────────────┘
```

Hamburger menu dropdown:
```
┌──────────────┐
│ 📺 YouTube ● │  ← red dot when video detected
│              │
│ (more items  │  ← future: Knowledge, Actions
│  later)      │
└──────────────┘
```

### 10. `src/components/index.ts`

Add export: `export { YouTubeSection } from './YouTubeSection';`

### 11. `src/state/types.ts`

Add types:
```typescript
interface YouTubeGist {
  url: string;
  video_id: string;
  title: string;
  channel: string;
  description: string;
  duration_seconds: number;
}

interface YouTubeDetectedEvent {
  url: string;
  video_id: string;
}
```

### 12. `src/App.css`

New styles for:
- `.hamburger-button` — same size/style as settings button, no rotation on hover
- `.hamburger-menu` — absolute positioned dropdown, white bg, shadow, border-radius
- `.hamburger-item` — menu item with icon + label + optional badge dot
- `.notification-badge` — small red dot indicator
- `.youtube-card` — card for displaying gist data
- `.observer-status` — running/stopped indicator

---

## Tauri Events (New)

| Event | Payload | Emitted by | Listened by |
|-------|---------|------------|-------------|
| `youtube-video-detected` | `{ url: String, video_id: String }` | BrowserObserver | YouTubeSection.tsx, App.tsx (badge) |

---

## Implementation Order

1. Backend: `browser/mod.rs` + `browser/youtube.rs` (scraper function)
2. Backend: `browser/observer.rs` (Chrome polling + notification)
3. Backend: `Cargo.toml` deps, `lib.rs` wiring, `commands.rs`, `capabilities/default.json`
4. Frontend: Types in `types.ts`
5. Frontend: `YouTubeSection.tsx` component
6. Frontend: Hamburger menu in `App.tsx` + CSS in `App.css`
7. Build + test end-to-end

---

## Patterns Reused from Existing Code

| Pattern | Source | Used for |
|---------|--------|----------|
| `tokio::spawn` + `tokio::select!` with stop signal | `transcription/manager.rs` | BrowserObserver polling loop |
| `Arc<tokio::sync::Mutex<T>>` managed state | TranscriptionManager in `lib.rs` | BrowserObserver state |
| `app_handle.emit("event", &payload)` | Recording/transcription events | YouTube detection events |
| `Result<T, String>` command returns | All existing commands | New browser commands |
| `dialog-overlay` + `settings-panel` CSS | Settings component | YouTubeSection overlay |
| `useTauriEvent` hook / `listen()` | useRecording.ts / Settings.tsx | YouTube event listeners |

---

## Dependencies Summary

| Crate | Version | Why | New? |
|-------|---------|-----|------|
| `tauri-plugin-notification` | 2 | Native macOS notifications | Yes |
| `regex` | 1 | YouTube URL matching + HTML parsing | Move from dev-deps |
| `reqwest` | 0.12 | Fetch YouTube page HTML | Already present |
| `serde` | 1 | Serialize YouTubeGist | Already present |

---

## Verification Checklist

- [ ] `cargo build` compiles without errors
- [ ] App launches, hamburger menu visible in header
- [ ] Click hamburger → YouTube option shown
- [ ] Click YouTube → YouTubeSection panel opens
- [ ] Click "Start Observer" → observer status shows running
- [ ] Open a YouTube video in Chrome
- [ ] macOS notification appears: "YouTube Video Detected"
- [ ] YouTube section shows detected video with URL
- [ ] Click "Prepare Gist" → loading state → gist displayed
- [ ] Gist shows: title, channel, duration, description
- [ ] Click "Stop Observer" → polling stops
- [ ] Click "Dismiss" → removes gist card
