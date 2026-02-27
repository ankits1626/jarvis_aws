# Three-Panel Layout Redesign

## Current State

Today, App.tsx is a single vertical column with everything stacked:

```
+-----------------------------------------------+
|  Header (title + hamburger + settings)         |
+-----------------------------------------------+
|  Status: Ready to record                       |
|  [ Start Recording ]                           |
+-----------------------------------------------+
|  Recordings (3)                                |
|  ┌──────────────────────────────────┐          |
|  │ rec_001.pcm  💎  📝  🗑️        │          |
|  │ Feb 27 • 2:30 • 1.2MB           │          |
|  ├──────────────────────────────────┤          |
|  │ rec_002.pcm      📝  🗑️        │          |
|  │ Feb 26 • 1:15 • 600KB           │          |
|  │  ┌─ Transcript (en) ─────────┐  │          |
|  │  │ "Hello, this is a test..." │  │          |
|  │  │ [ Save as Gem ]            │  │          |
|  │  └────────────────────────────┘  │          |
|  └──────────────────────────────────┘          |
+-----------------------------------------------+
|  Audio Player: rec_001.pcm                     |
|  [=====>----------] 1:23 / 2:30               |
+-----------------------------------------------+
|  Live Transcript                               |
|  "Real-time whisper output..."                 |
+-----------------------------------------------+
|                                                |
|  (YouTube, Browser, Gems, Settings all open    |
|   as full-screen dialog overlays)              |
|                                                |
+-----------------------------------------------+
```

### Problems
- Everything in one column = scrolling hell
- YouTube, Browser, Gems, Settings are modal overlays (blocking)
- Transcript inline in recording row = cluttered list
- Audio player jammed between recordings and live transcript
- No persistent navigation — hamburger menu for everything

---

## Proposed: Three-Panel Layout

```
+------+-----------------------------+-----------------------------+
| LEFT |          CENTER             |           RIGHT             |
| NAV  |       (main content)        |      (context panel)        |
+------+-----------------------------+-----------------------------+
|      |                             |                             |
| 🎙️  |  (changes based on          |  (changes based on          |
| Rec  |   selected nav item)        |   user action / context)    |
|      |                             |                             |
| 📼  |                             |                             |
| List |                             |                             |
|      |                             |                             |
| 💎  |                             |                             |
| Gems |                             |                             |
|      |                             |                             |
| 📹  |                             |                             |
| YT   |                             |                             |
|      |                             |                             |
| 🌐  |                             |                             |
| Web  |                             |                             |
|      |                             |                             |
| ⚙️  |                             |                             |
| Set  |                             |                             |
|      |                             |                             |
+------+-----------------------------+-----------------------------+
```

### Left Panel: Navigation (collapsible sidebar)
- Icon-based nav, expands on hover or toggle to show labels
- Always visible (not a hamburger menu)
- Active item highlighted
- Collapse/expand toggle at bottom

### Center Panel: Main Content (driven by nav selection)
- Changes entirely based on which nav item is selected

### Right Panel: Context Panel (driven by user actions)
- Contextual detail/output panel
- Can be empty, collapsed, or populated

---

## Panel Content by Nav Selection

### Nav: Record (🎙️)

```
+------+-----------------------------+-----------------------------+
| 🎙️◀ |  RECORD                     |  LIVE TRANSCRIPT            |
| 📼  |                             |                             |
| 💎  |  Status: Ready to record    |  (empty when idle)          |
| 📹  |                             |                             |
| 🌐  |  [ ⏺ Start Recording ]     |  When recording:            |
| ⚙️  |                             |  "Real-time whisper         |
|      |  Elapsed: --:--             |   output appears here       |
|      |                             |   as the user speaks..."    |
|      |                             |                             |
|      |                             |  [ Save as Gem ]            |
+------+-----------------------------+-----------------------------+
```

- Center: Record button, status, elapsed timer
- Right: Live Whisper transcript (streams during recording)
- Right shows "Save as Gem" button after recording stops

### Nav: Recordings (📼)

```
+------+-----------------------------+-----------------------------+
| 🎙️  |  RECORDINGS (3)             |  (nothing selected)         |
| 📼◀ |                             |                             |
| 💎  |  ┌────────────────────────┐ |  Select a recording to      |
| 📹  |  │ rec_001.pcm 💎    🗑️ │ |  play or transcribe         |
| 🌐  |  │ Feb 27 • 2:30 • 1.2MB │ |                             |
| ⚙️  |  ├────────────────────────┤ |                             |
|      |  │ rec_002.pcm       🗑️ │ |                             |
|      |  │ Feb 26 • 1:15 • 600KB │ |                             |
|      |  ├────────────────────────┤ |                             |
|      |  │ rec_003.pcm       🗑️ │ |                             |
|      |  │ Feb 25 • 0:45 • 300KB │ |                             |
|      |  └────────────────────────┘ |                             |
+------+-----------------------------+-----------------------------+
```

**When user clicks a recording:**

```
+------+-----------------------------+-----------------------------+
| 🎙️  |  RECORDINGS (3)             |  rec_001.pcm                |
| 📼◀ |                             |                             |
| 💎  |  ┌────────────────────────┐ |  Feb 27, 2025 14:30         |
| 📹  |  │ rec_001.pcm 💎 ► 🗑️ │ |  Duration: 2:30             |
| 🌐  |  ├────────────────────────┤ |  Size: 1.2 MB               |
| ⚙️  |  │ rec_002.pcm       🗑️ │ |                             |
|      |  │ Feb 26 • 1:15 • 600KB │ |  PLAYER                     |
|      |  ├────────────────────────┤ |  [======>--------] 1:23     |
|      |  │ rec_003.pcm       🗑️ │ |  [⏮] [⏯] [⏭]              |
|      |  │ Feb 25 • 0:45 • 300KB │ |                             |
|      |  └────────────────────────┘ |  ACTIONS                    |
|      |                             |  [ 📝 Transcribe ]          |
|      |                             |  [ 💎 Has Gem ]             |
+------+-----------------------------+-----------------------------+
```

- Center: Clean recording list (no inline transcripts)
- Right: Selected recording details + audio player + action buttons

**When user clicks Transcribe:**

```
+------+-----------------------------+-----------------------------+
| 🎙️  |  RECORDINGS (3)             |  rec_002.pcm                |
| 📼◀ |                             |                             |
| 💎  |  ┌────────────────────────┐ |  PLAYER                     |
| 📹  |  │ rec_001.pcm 💎    🗑️ │ |  [======>--------] 0:45     |
| 🌐  |  ├────────────────────────┤ |                             |
| ⚙️  |  │ rec_002.pcm  ⏳   🗑️ │ |  TRANSCRIPT (Hindi)         |
|      |  │ Feb 26 • 1:15 • 600KB │ |  ┌────────────────────────┐ |
|      |  ├────────────────────────┤ |  │ "यह एक परीक्षण है।     │ |
|      |  │ rec_003.pcm       🗑️ │ |  │ मैं हिंदी में बोल       │ |
|      |  │ Feb 25 • 0:45 • 300KB │ |  │ रहा हूँ..."             │ |
|      |  └────────────────────────┘ |  └────────────────────────┘ |
|      |                             |                             |
|      |                             |  [ Save as Gem ]            |
+------+-----------------------------+-----------------------------+
```

- Center: Recording list stays clean (only ⏳ spinner on active row)
- Right: Player + transcript + Save button all in context panel

### Nav: Gems (💎)

```
+------+-----------------------------+-----------------------------+
| 🎙️  |  GEMS (12)                  |  (no gem selected)          |
| 📼  |                             |                             |
| 💎◀ |  Search: [___________]      |  Select a gem to view       |
| 📹  |  Tags: [all▼]              |  details                    |
| 🌐  |                             |                             |
| ⚙️  |  ┌────────────────────────┐ |                             |
|      |  │ Audio Transcript -     │ |                             |
|      |  │ 2025-02-27             │ |                             |
|      |  │ 🏷️ meeting, hindi      │ |                             |
|      |  │ Hindi                  │ |                             |
|      |  ├────────────────────────┤ |                             |
|      |  │ YouTube: How to...     │ |                             |
|      |  │ 🏷️ tutorial, coding    │ |                             |
|      |  └────────────────────────┘ |                             |
+------+-----------------------------+-----------------------------+
```

**When user clicks a gem:**

```
+------+-----------------------------+-----------------------------+
| 🎙️  |  GEMS (12)                  |  Audio Transcript           |
| 📼  |                             |  2025-02-27 14:30           |
| 💎◀ |  Search: [___________]      |                             |
| 📹  |  Tags: [all▼]              |  Tags: meeting, hindi       |
| 🌐  |                             |  Summary: A discussion      |
| ⚙️  |  ┌────────────────────────┐ |  about project planning...  |
|      |  │ Audio Transcript - ◀   │ |                             |
|      |  │ 2025-02-27             │ |  TRANSCRIPT (Hindi)         |
|      |  │ 🏷️ meeting, hindi      │ |  ┌────────────────────────┐ |
|      |  ├────────────────────────┤ |  │ Full transcript text    │ |
|      |  │ YouTube: How to...     │ |  │ displayed here with     │ |
|      |  │ 🏷️ tutorial, coding    │ |  │ scrolling...            │ |
|      |  └────────────────────────┘ |  └────────────────────────┘ |
|      |                             |                             |
|      |                             |  [ 🎙️ Transcribe ] [Enrich]|
|      |                             |  [ 🗑️ Delete ]             |
+------+-----------------------------+-----------------------------+
```

- Center: Gem list with search/filter
- Right: Full gem detail view (summary, tags, transcript, actions)

### Nav: YouTube (📹)

```
+------+-----------------------------+-----------------------------+
| 🎙️  |  YOUTUBE                    |  (no video selected)        |
| 📼  |                             |                             |
| 💎  |  URL: [________________]    |  Paste a YouTube URL to     |
| 📹◀ |  [ Extract ]               |  extract content            |
| 🌐  |                             |                             |
| ⚙️  |  Recent Extractions:        |                             |
|      |  ┌────────────────────────┐ |                             |
|      |  │ How to build...        │ |                             |
|      |  │ 2025-02-27             │ |                             |
|      |  └────────────────────────┘ |                             |
+------+-----------------------------+-----------------------------+
```

### Nav: Browser (🌐)

```
+------+-----------------------------+-----------------------------+
| 🎙️  |  BROWSER TOOL               |  Extracted Content          |
| 📼  |                             |                             |
| 💎  |  URL: [________________]    |  (shows extracted page      |
| 📹  |  [ Extract ]               |   content after extraction) |
| 🌐◀ |                             |                             |
| ⚙️  |                             |  [ Save as Gem ]            |
+------+-----------------------------+-----------------------------+
```

### Nav: Settings (⚙️)

```
+------+-----------------------------+-----------------------------+
| 🎙️  |  SETTINGS                   |  (right panel hidden        |
| 📼  |                             |   or shows help text)       |
| 💎  |  Intelligence Provider       |                             |
| 📹  |  [MLX ▼]                   |  Current Model:             |
| 🌐  |                             |  mlx-community/...          |
| ⚙️◀ |  Active Model               |                             |
|      |  [model-name ▼]            |  Capabilities:              |
|      |                             |  ✅ Text                    |
|      |  Audio Settings             |  ✅ Audio                   |
|      |  Sample Rate: [16000]       |  ❌ Vision                  |
|      |  Channels: [1]              |                             |
+------+-----------------------------+-----------------------------+
```

---

## Collapsible Left Panel States

### Expanded (default or on hover/toggle)

```
+------------+
| 🎙️ Record  |
| 📼 List    |
| 💎 Gems    |
| 📹 YouTube |
| 🌐 Browser |
|            |
|            |
| ⚙️ Settings|
| [◀ Collapse]
+------------+
  ~140px
```

### Collapsed (icon-only)

```
+----+
| 🎙️ |
| 📼 |
| 💎 |
| 📹 |
| 🌐 |
|    |
|    |
| ⚙️ |
| [▶] |
+----+
 ~48px
```

---

## Right Panel Behavior Summary

| Left Nav    | Center Content           | Right Panel Content                   |
|-------------|--------------------------|---------------------------------------|
| Record      | Record button + status   | Live transcript (during recording)    |
| Recordings  | Recording list           | Player + transcript + actions         |
| Gems        | Gem list + search        | Gem detail + transcript + actions     |
| YouTube     | URL input + history      | Extracted content                     |
| Browser     | URL input                | Extracted page content                |
| Settings    | Settings form            | Model info / help (or hidden)         |

### Right Panel Rules
- Empty state: shows placeholder text ("Select a recording...")
- Can be collapsed/hidden for settings
- Scrollable independently from center
- Transcript always in right panel (never inline in lists)

---

## Key Benefits Over Current Layout

1. **No more modal overlays** — YouTube, Browser, Gems, Settings are all nav items, not blocking dialogs
2. **Clean lists** — Transcripts and players move to right panel, recordings/gems lists stay compact
3. **Persistent navigation** — Left sidebar always available, no hamburger hunting
4. **Context-aware right panel** — Shows relevant detail without cluttering the main view
5. **Better use of horizontal space** — Desktop apps have wide screens, use them
6. **Consistent pattern** — List on left/center, detail on right (master-detail) is a well-understood UX pattern

---

## Implementation Considerations

### Component Refactoring
- Extract `RecordingDetailPanel` (right panel for recordings)
- Extract `GemDetailPanel` (right panel for gems)
- Extract `LeftNav` component
- Move live transcript from inline to right panel
- Move audio player from inline to right panel
- Existing `GemsPanel`, `YouTubeSection`, `BrowserTool`, `Settings` become center-panel views (remove overlay wrappers)

### State Management
- New state: `activeNav: 'record' | 'recordings' | 'gems' | 'youtube' | 'browser' | 'settings'`
- New state: `selectedGemId: string | null` (for gem detail in right panel)
- Existing `selectedRecording` already works for recording detail
- Remove: `showSettings`, `showYouTube`, `showBrowserTool`, `showGems`, `showHamburgerMenu`

### CSS Layout
- CSS Grid or Flexbox for three-column layout
- Left panel: `min-width: 48px; max-width: 140px; transition: width`
- Center: `flex: 1; min-width: 300px`
- Right: `flex: 1; min-width: 0; max-width: 50%` (can collapse to 0)
- Responsive: On small windows, right panel could stack below center

### Migration Path
1. Add left nav + routing (keep existing content in center)
2. Move each overlay (Settings, YouTube, Browser, Gems) to a nav route
3. Add right panel structure
4. Move player + transcript to right panel
5. Remove hamburger menu + overlay code
6. Polish CSS transitions and responsive behavior
