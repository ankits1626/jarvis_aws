# Phase 3: Data Flows (Part 3)

> Animated step-by-step walkthroughs of how data moves through Jarvis.

---

## Goal

Three interactive, animated data flow diagrams that trace real code paths from user action to final result. User can step forward/backward through each flow, seeing which module is active at each step, what data is passed, and what code is executing.

---

## Tasks

### 3.1 — Data flow data file
- Create `src/data/dataFlows.ts`
- Define types: `FlowStep`, `DataFlow`, `ActiveModule`
- Each `FlowStep` contains:
  - `id`: step number
  - `title`: short description ("User clicks Record")
  - `description`: 2-3 sentence explanation
  - `activeModules`: which system modules are highlighted
  - `dataLabel`: what data is being passed (e.g., "PCM audio bytes")
  - `codeSnippet`: optional real code excerpt
  - `layer`: which architecture layer this step belongs to (for color coding)
- Populate all 3 flows with accurate step data

### 3.2 — DataFlowViewer component
- Build `DataFlowViewer.tsx`
- Flow selector: 3 tabs at the top to switch between flows
- Step-through controls:
  - Previous / Next buttons
  - Step indicator dots (1 of N)
  - Auto-play toggle with speed control (slow/medium/fast)
  - Keyboard: left/right arrows to step
- Current step number and title displayed prominently

### 3.3 — Flow visualization layout
- Horizontal module strip at top showing all modules involved in the flow
  - Each module is a colored box (layer color)
  - Active module(s) at current step are highlighted (glow + scale up)
  - Inactive modules are dimmed
- Below the strip: step detail card
  - Title, description, which modules are active
  - Data label showing what's being passed between modules
  - Optional code snippet
- Animated arrow connecting active modules at each step

### 3.4 — Flow 1: Recording Flow (10 steps)
Steps:
1. **User clicks Record** — RecordButton component, dispatch `START_RECORDING` action
2. **Frontend sends command** — `invoke('start_recording')` via Tauri IPC
3. **Backend receives command** — `commands.rs::start_recording()`, acquires RecordingManager mutex
4. **JarvisListen spawned** — `recording.rs` spawns Swift sidecar with `--output` flag
5. **Audio capture begins** — JarvisListen captures system audio via ScreenCaptureKit
6. **PCM written to file** — 16kHz, 16-bit, mono PCM to `~/.jarvis/recordings/`
7. **FIFO pipe created** — AudioRouter creates named pipe, tails PCM file into FIFO
8. **TranscriptionManager receives audio** — Audio chunks via mpsc channel
9. **Hybrid transcription** — VAD detects speech → Vosk (instant partial) → Whisper (final)
10. **Events emitted to frontend** — `transcription-update` events → TranscriptDisplay updates in real-time

### 3.5 — Flow 2: Browser Capture Flow (8 steps)
Steps:
1. **BrowserObserver polls Chrome** — AppleScript query every 3s for active tab URL
2. **YouTube URL detected** — URL pattern match, video ID extracted
3. **Event emitted** — `youtube-video-detected` event sent to frontend
4. **User opens YouTube section** — Badge notification, user clicks to open panel
5. **Fetch Gist triggered** — User clicks "Fetch", backend scrapes YouTube page
6. **PageGist returned** — Metadata extracted: title, channel, duration, description
7. **User saves as Gem** — Click "Save", backend creates Gem from PageGist
8. **AI enrichment** — IntelligenceKit generates tags + summary, merged into gem, saved to SQLite

### 3.6 — Flow 3: Gem Lifecycle (7 steps)
Steps:
1. **Content captured** — Any extractor produces a `PageGist`
2. **Gem created** — `PageGist` converted to `Gem` struct with UUID, timestamp
3. **AI availability check** — IntelProvider.check_availability() called
4. **Enrichment (if available)** — Open session → generate tags → summarize → close session
5. **Upsert to SQLite** — `INSERT OR REPLACE` by `source_url` uniqueness
6. **FTS5 index updated** — `gems_fts` virtual table auto-synced via triggers
7. **Gem searchable** — User can search by text, filter by tags in GemsPanel

---

## Expected Output

What you see:
- 3 tab buttons at top: "Recording", "Browser Capture", "Gem Lifecycle"
- Module strip: colored boxes for each involved module, active ones glowing
- Step detail card with title, explanation, code snippet
- Animated arrows showing data direction between modules
- Step dots at bottom showing progress (e.g., "Step 4 of 10")
- Auto-play mode: steps advance every 2s/1s/0.5s depending on speed

### Interactions
- Click Next/Prev to step through
- Click any step dot to jump to that step
- Toggle auto-play for hands-free walkthrough
- Keyboard: left/right arrows, space to toggle play

### Files Created/Modified
```
src/data/dataFlows.ts                  (NEW)
src/components/DataFlowViewer.tsx       (REPLACE placeholder)
src/components/shared/FlowArrow.tsx     (EXTEND — support step-based highlighting)
```

---

## Definition of Done
- [ ] All 3 flows render with correct steps and accurate data
- [ ] Step-through controls work (prev/next/jump/keyboard)
- [ ] Active modules highlight correctly at each step
- [ ] Auto-play works at all 3 speeds
- [ ] Code snippets appear for steps that have them
- [ ] Flow arrows animate direction between active modules
- [ ] Tab switching preserves step position (or resets — either is fine)
- [ ] Responsive — flows don't overflow on reasonable screen sizes (1200px+)
