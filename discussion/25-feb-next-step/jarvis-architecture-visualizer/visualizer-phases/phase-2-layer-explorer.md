# Phase 2: Layer Explorer (Part 2)

> The heart of the visualizer. Six interactive deep-dives.

---

## Goal

Six self-contained, color-coded layer pages. Each one lets you explore that subsystem's components, data structures, and internal flows. Uses real file paths, real struct names, and real config values from the Jarvis codebase.

---

## Tasks

### 2.1 — Shared components
- Implement `NodeCard.tsx` — expandable card with title, color border, collapsible content
  - Collapsed: icon + title + one-line description
  - Expanded: full detail with code snippets, file paths, child nodes
- Implement `CodeSnippet.tsx` — syntax-highlighted code block
  - Language indicator (rust, typescript, swift, sql)
  - Monospace font, line numbers optional
  - Copy button
  - No external syntax highlighting library — use Tailwind color classes for keywords
- Implement `Tooltip.tsx` — hover tooltip
  - Appears on mouse enter, disappears on leave
  - Positioned above the element, arrow pointing down
  - Shows file path, type info, or brief explanation

### 2.2 — LayerExplorer container
- Build `LayerExplorer.tsx`
- Takes `activeLayer` prop
- Renders the correct layer component
- Layer header: colored bar with layer name + icon + description
- "Key files" section at the bottom of each layer with clickable file paths

### 2.3 — Layer 1: Frontend (Blue/Cyan)
- Build `FrontendLayer.tsx`
- **Component Tree Visualization**:
  - `App` at root → branches to 14 child components
  - Each component is a `NodeCard` — click to expand
  - Expanded view shows: props, which Tauri commands it calls, which events it listens to
- **State Machine Diagram**:
  - Visual states: `idle`, `processing`, `recording`
  - Arrows between states with action labels
  - Current state highlighted (interactive — click to simulate transitions)
- **Hook System**:
  - 3 hook cards: `useRecording`, `useTauriCommand`, `useTauriEvent`
  - Each shows: parameters, return value, which components use it
- Data source: component/hook metadata in `architecture.ts`

### 2.4 — Layer 2: Tauri Bridge (Amber/Orange)
- Build `TauriBridgeLayer.tsx`
- **Command Catalog**:
  - 37 commands grouped into categories: Recording, Transcription, Browser, Gems, Settings, Intelligence
  - Each command card shows: name, parameters, return type, which frontend component calls it
  - Expandable with real code snippet from `commands.rs`
- **Event Flow Diagram**:
  - List of events emitted: `recording-started`, `transcription-update`, `youtube-video-detected`, `model-download-progress`
  - Each event → which backend module emits it → which frontend component listens
  - Animated "pulse" showing event direction (backend → frontend)
- **State Management Map**:
  - Visual boxes for each Tauri state: `FileManager`, `GemStore`, `IntelProvider`, `SettingsManager`, etc.
  - Lock type badge on each: `Mutex`, `RwLock`, `Arc<dyn Trait>`, direct
  - Hover to see which commands access each state

### 2.5 — Layer 3: Audio Pipeline (Green/Emerald)
- Build `AudioPipelineLayer.tsx`
- **Pipeline Flow**:
  - Horizontal pipeline: `JarvisListen` → `PCM File` → `FIFO` → `AudioRouter` → `TranscriptionManager`
  - Each stage is a `NodeCard` — click to see internals
  - Animated data flow (green pulses moving left to right)
- **Hybrid Transcription Breakdown**:
  - Three-branch diagram: VAD → Vosk (fast, gray text) / Whisper (slow, final text)
  - Timing annotations: "~instant", "~2-5s"
  - Audio format specs box: 16kHz, 16-bit, mono, PCM
- **Sidecar Detail**:
  - JarvisListen card: Swift, ScreenCaptureKit, `--output` flag fix
  - Shows the stdout corruption bug and solution (with before/after)

### 2.6 — Layer 4: Browser & Extractors (Purple/Violet)
- Build `BrowserExtractorLayer.tsx`
- **Observer Cycle**:
  - Circular animation: poll Chrome → classify URL → emit event → wait 3s → repeat
  - Shows AppleScript under the hood
- **Extractor Router**:
  - Domain → Extractor mapping table
  - Click an extractor → see what it extracts and the `PageGist` output
  - 6 extractors: YouTube, Medium, Gmail, ChatGPT, Claude Extension, Generic
- **PageGist Anatomy**:
  - Interactive struct viewer — all fields with types
  - Toggle between extractors to see which fields each one populates
  - `extra: serde_json::Value` shown with example JSON per extractor

### 2.7 — Layer 5: Intelligence / AI (Rose/Red)
- Build `IntelligenceLayer.tsx`
- **Provider Architecture**:
  - Trait `IntelProvider` box at top
  - Two implementations branching below: `IntelligenceKitProvider`, `NoOpProvider`
  - Planned: `MLXProvider` (grayed out, with spec link)
- **NDJSON Protocol**:
  - Step-through: type a command, see the JSON request, see the JSON response
  - 4 commands: check-availability, open-session, message, close-session
  - Real JSON examples from the spec
- **Graceful Degradation**:
  - Toggle: "AI available" on/off
  - Shows what changes in the UI and data flow when switched off
  - NoOp path highlighted

### 2.8 — Layer 6: Gems / Knowledge Base (Yellow/Gold)
- Build `GemsLayer.tsx`
- **Gem Data Model**:
  - Interactive struct viewer — all 12 fields
  - Required vs optional visual indicator
  - `ai_enrichment` JSON example (tags + summary)
- **SQLite Schema**:
  - Two tables side by side: `gems` (main) + `gems_fts` (FTS5 virtual)
  - Show the CREATE TABLE statements
  - Animated: insert a gem → see it appear in both tables
- **CRUD Operations**:
  - Visual cards for each operation: Save (upsert), List, Search, Filter by Tag, Delete
  - Click each → see the SQL query and Rust code snippet
- **Enrichment Pipeline**:
  - Flow: gem saved → check AI available → open session → generate tags → summarize → merge → save
  - Branches at "check AI available" — success path vs NoOp path

---

## Expected Output

Each layer page shows:
- Colored header bar with layer name, icon, one-line description
- 2-4 interactive visualization sections
- Real code snippets from the Jarvis codebase
- Expandable `NodeCard` components for every major module
- "Key files" footer with real file paths

### Files Created/Modified
```
src/data/architecture.ts               (EXTEND with all layer data)
src/components/LayerExplorer.tsx         (NEW)
src/components/layers/FrontendLayer.tsx  (NEW)
src/components/layers/TauriBridgeLayer.tsx (NEW)
src/components/layers/AudioPipelineLayer.tsx (NEW)
src/components/layers/BrowserExtractorLayer.tsx (NEW)
src/components/layers/IntelligenceLayer.tsx (NEW)
src/components/layers/GemsLayer.tsx      (NEW)
src/components/shared/NodeCard.tsx       (IMPLEMENT)
src/components/shared/CodeSnippet.tsx    (IMPLEMENT)
src/components/shared/Tooltip.tsx        (IMPLEMENT)
```

---

## Definition of Done
- [ ] All 6 layers render with correct color themes
- [ ] NodeCard expand/collapse works smoothly for every module
- [ ] Code snippets show real Jarvis code with language indicators
- [ ] Tooltips appear on hover with file paths / type info
- [ ] Clicking a command/event/struct card reveals its details
- [ ] State machine in Frontend layer is interactive (click to transition)
- [ ] NDJSON step-through in Intelligence layer works
- [ ] SQLite insert animation in Gems layer works
- [ ] No broken layouts — each layer scrolls independently
