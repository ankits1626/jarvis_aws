# Jarvis Architecture Visualizer - Objectives

## What We're Building

An interactive React web application that lets you explore the Jarvis AWS desktop assistant architecture layer by layer — from a high-level birds-eye view down to individual module internals — **and learn the technologies that power it from scratch**. Modeled after the [LLM Visualizer](../llm-visualizer/) approach: progressive disclosure, color-coded systems, interactive exploration, and educational annotations at every level. Just like the LLM Visualizer taught transformers and model internals, this teaches the full Jarvis tech stack end to end.

---

## Why This Exists

Jarvis is a multi-layered system spanning Rust, Swift, TypeScript, SQLite, and multiple sidecar processes. Understanding how all the pieces fit together requires holding a lot of context simultaneously. A static diagram doesn't cut it — you need to be able to zoom in, click through data flows, and see how a user action ripples through the stack.

This visualizer serves as:
- **Learning platform** — go from zero to understanding the full stack fast
- **Onboarding tool** — new contributors understand the system in minutes
- **Architecture reference** — always up-to-date mental model of Jarvis
- **Design discussion aid** — explore "what if" changes visually
- **Spec companion** — connects the 13 Kiro specs to the actual codebase structure
- **Tech stack 101** — learn Rust, Tauri, Swift, React, SQLite, and spec-driven development interactively

---

## Core Principles (Borrowed from LLM Visualizer)

1. **Progressive disclosure** — start simple, reveal complexity on demand
2. **Color-coded systems** — each architectural layer gets a consistent color
3. **Interactive exploration** — click, hover, zoom — never just read
4. **Real data references** — show actual file paths, real struct names, real config
5. **Educational annotations** — every component explains *why* it exists, not just *what* it is

---

## Navigation Structure

### Part 1: The Big Picture
> "What is Jarvis?"

A single zoomable view with 3 zoom levels:
- **Level 1 — Black Box**: User speaks → Jarvis captures, transcribes, enriches → Knowledge base
- **Level 2 — Three Pillars**: Frontend (React) ↔ Backend (Rust/Tauri) ↔ Sidecars (Swift)
- **Level 3 — Full System Map**: All 6 subsystems with data flow arrows between them

### Part 2: Layer Explorer (6 Layers)
> "How does each layer work?"

Each layer is a self-contained interactive section:

#### Layer 1 — Frontend (React + TypeScript)
- **Color**: Blue/Cyan
- **Visualizations**:
  - Component tree (14 components) with parent-child relationships
  - State machine diagram: `idle → processing → recording → processing → idle`
  - Tauri IPC bridge — shows which commands each component calls
  - Hook system: `useRecording`, `useTauriCommand`, `useTauriEvent`
- **Key files**: `App.tsx`, `state/reducer.ts`, `state/types.ts`, `hooks/*.ts`

#### Layer 2 — Tauri Bridge (IPC + Commands)
- **Color**: Amber/Orange
- **Visualizations**:
  - Command catalog — all 37 Tauri commands grouped by subsystem
  - Event flow diagram — backend → frontend event emission
  - State management map — which Tauri state objects exist and their lock types (`Mutex`, `RwLock`, `Arc<dyn Trait>`)
  - Plugin registry — shell, global-shortcut, notification, opener
- **Key files**: `commands.rs`, `lib.rs`

#### Layer 3 — Audio Pipeline (Capture → Transcription)
- **Color**: Green/Emerald
- **Visualizations**:
  - Pipeline flow: `JarvisListen (Swift) → PCM file → FIFO → AudioRouter → TranscriptionManager`
  - Hybrid transcription breakdown: `Silero VAD → Vosk (partials) → Whisper (finals)`
  - Audio format specs: 16kHz, 16-bit, mono PCM
  - Real-time streaming diagram with timing annotations (Vosk ~instant, Whisper ~2-5s)
- **Key files**: `recording.rs`, `transcription/*.rs`, `jarvis-listen/`

#### Layer 4 — Browser & Content Extraction
- **Color**: Purple/Violet
- **Visualizations**:
  - Browser observer polling cycle (3s interval via AppleScript)
  - Extractor router — domain → extractor mapping (YouTube, Medium, Gmail, ChatGPT, Claude Extension)
  - `PageGist` struct anatomy — what each extractor produces
  - Capture-to-gem flow — tab detection → user clicks capture → extractor runs → gem saved
- **Key files**: `browser/*.rs`, `browser/extractors/*.rs`, `browser/adapters/*.rs`

#### Layer 5 — Intelligence / AI Layer
- **Color**: Rose/Red
- **Visualizations**:
  - Provider architecture — trait `IntelProvider` → `IntelligenceKitProvider` | `NoOpProvider`
  - Sidecar communication — NDJSON protocol request/response examples
  - Session lifecycle: `check-availability → open-session → message → close-session`
  - Graceful degradation flow — what happens when AI is unavailable
  - MLX provider (planned) — Python sidecar with Qwen/Llama models
- **Key files**: `intelligence/*.rs`, `intelligence-kit/`, `.kiro/specs/mlx-intelligence-provider/`

#### Layer 6 — Gems (Knowledge Base)
- **Color**: Yellow/Gold
- **Visualizations**:
  - Gem data model — all fields with types and optionality
  - SQLite schema — `gems` table + `gems_fts` FTS5 virtual table
  - CRUD operations — save (upsert by URL), list, search, filter-by-tag, delete
  - AI enrichment pipeline — gem saved → tags generated → summary generated → merged
- **Key files**: `gems/*.rs`, `~/.jarvis/gems.db`

### Part 3: Data Flows
> "How does data move through the system?"

Interactive animated flows the user can step through:

1. **Recording Flow** — User clicks record → JarvisListen spawned → PCM written → FIFO → transcription → events → UI update
2. **Browser Capture Flow** — Observer detects YouTube → notification → user opens panel → fetch gist → save as gem → AI enrichment
3. **Gem Lifecycle** — Content captured → gem created → AI enrichment → stored in SQLite → searchable via FTS5

### Part 4: Spec Map
> "What's planned next?"

Visual map connecting the 13 Kiro specs to codebase locations:
- Which specs are implemented (green), in-progress (yellow), planned (gray)
- Click a spec → see its requirements summary + which files it touches
- Shows evolution: `jarvis-browser-vision` → `jarvis-browser-vision-v2`
- Highlights the upcoming `mlx-intelligence-provider` spec

### Part 5: Tech Stack 101
> "Teach me the technologies — from zero to building Jarvis"

The missing piece: before you can truly understand the architecture, you need to understand the tools. Each 101 guide is an interactive, self-contained lesson — not a documentation dump. Think of each as a mini LLM-Visualizer for that technology. Every guide follows the same structure: **Why it exists → Core mental model → Key concepts (interactive) → How Jarvis uses it → Try it yourself**.

#### Guide 1 — Rust 101
- **Color**: Orange/Amber (matches the Rust brand)
- **Core mental model**: Ownership, borrowing, lifetimes — visualized as a "who holds the key" animation
- **Key concepts (interactive)**:
  - `Result<T, E>` and error handling — click through Ok/Err branches
  - `struct` + `impl` — build a Gem struct interactively, add methods
  - Traits — drag-and-drop: which types implement `IntelProvider`?
  - `Arc`, `Mutex`, `RwLock` — animated concurrent access visualization (who's reading, who's writing, who's blocked)
  - `async/await` + Tokio — event loop visualization with spawned tasks
  - Pattern matching — interactive match arm builder
  - Modules & crates — file tree → module tree mapping
- **How Jarvis uses it**: Real examples from `commands.rs`, `recording.rs`, `gems/store.rs`
- **Try it yourself**: Mini exercises — "Fix this borrow checker error", "Add a method to this struct"

#### Guide 2 — Tauri 101
- **Color**: Cyan/Teal
- **Core mental model**: Web frontend + Rust backend in one desktop app — animated split view
- **Key concepts (interactive)**:
  - Architecture diagram: Webview ↔ IPC bridge ↔ Rust core
  - `#[tauri::command]` — write a command, see it appear in the frontend
  - State management — `.manage()` → `State<'_, T>` extraction, visualized as a shared box
  - Events — backend emits, frontend listens — animated message bubbles
  - Plugins — puzzle pieces that snap into the app builder chain
  - Sidecar binaries — external processes bundled and spawned
  - `tauri.conf.json` — interactive config explorer (click a field → see what it does)
- **How Jarvis uses it**: Walk through `lib.rs` builder chain, show how `start_recording` command flows
- **Try it yourself**: "Trace this command from frontend to backend and back"

#### Guide 3 — Swift 101 (for macOS Sidecars)
- **Color**: Orange/Swift-brand
- **Core mental model**: Apple's language for Apple's APIs — system-level access
- **Key concepts (interactive)**:
  - Swift vs Rust comparison table — similar concepts, different syntax
  - `async/await` in Swift — side-by-side with Rust's async
  - ScreenCaptureKit — animated diagram of audio capture pipeline
  - Foundation Models framework — how on-device LLM works
  - `@Generable` / `@Guide` macros — structured output from LLMs, interactive example
  - Stdin/Stdout communication — NDJSON protocol step-through
  - Process lifecycle — spawn, communicate, terminate
- **How Jarvis uses it**: JarvisListen audio capture flow, IntelligenceKit session lifecycle
- **Try it yourself**: "Read this NDJSON exchange — what does the server respond?"

#### Guide 4 — React + TypeScript 101
- **Color**: Blue/Cyan (matches React brand)
- **Core mental model**: UI = f(state) — state changes, UI re-renders
- **Key concepts (interactive)**:
  - Components — build a component tree, see render order
  - `useState` vs `useReducer` — toggle between them for the same state
  - Custom hooks — extract logic, visualize the hook lifecycle
  - TypeScript types — hover over variables, see their types flow
  - Event handling — click a button in the demo, trace the event through the code
  - Conditional rendering — toggle state values, watch the UI change
- **How Jarvis uses it**: Walk through `App.tsx` → `RecordButton` → `useRecording` → Tauri command
- **Try it yourself**: "This component has a bug in its state transition — find and fix it"

#### Guide 5 — SQLite + FTS5 101
- **Color**: Yellow/Gold (matches Gems layer)
- **Core mental model**: A database that's just a file — no server, no setup
- **Key concepts (interactive)**:
  - Tables, rows, columns — visual table builder
  - SQL queries — type a query, see it execute against mock gem data
  - UNIQUE constraints — try inserting a duplicate URL, see the upsert
  - FTS5 full-text search — type a search term, see how the inverted index finds matches
  - Tokenization — how FTS5 breaks text into searchable tokens
  - Virtual tables — the `gems_fts` table shadows the `gems` table
- **How Jarvis uses it**: Walk through `sqlite_store.rs` — create table, insert gem, search
- **Try it yourself**: "Write a query to find all gems tagged 'rust' with 'async' in the title"

#### Guide 6 — macOS System APIs 101
- **Color**: Gray/Silver (system-level)
- **Core mental model**: The OS as a platform — not just an environment
- **Key concepts (interactive)**:
  - AppleScript / osascript — how Jarvis talks to Chrome
  - Accessibility API (AXUIElement) — DOM-like tree for native apps, interactive explorer
  - ScreenCaptureKit — permissions model, audio routing diagram
  - Notifications — NSUserNotification flow
  - FIFO / Named pipes — animated producer-consumer visualization
  - File system conventions — `~/.jarvis/` layout explorer
- **How Jarvis uses it**: Browser tab enumeration, Claude extension extraction, audio capture
- **Try it yourself**: "Trace the permission request flow for screen recording"

#### Guide 7 — Spec-Driven Development 101
- **Color**: Indigo/Blue
- **Core mental model**: Requirements first, code second — the spec is the source of truth
- **Key concepts (interactive)**:
  - The Kiro workflow — `requirements.md` → `design.md` → implementation → validation
  - Requirements anatomy — user stories, acceptance criteria, task breakdown
  - Design document anatomy — data models, API contracts, error handling, file structure
  - Spec evolution — click through `jarvis-browser-vision` → `v2`, see what changed and why
  - Traceability — click a requirement → see which files implement it
  - ADR (Architectural Decision Record) — interactive template builder
- **How Jarvis uses it**: Walk through all 13 specs, show the requirements → design → code pipeline
- **Try it yourself**: "Given this feature request, draft 3 user stories with acceptance criteria"

#### Guide 8 — The Sidecar Pattern 101
- **Color**: Teal/Cyan
- **Core mental model**: Your app spawns helper processes — they do specialized work and report back
- **Key concepts (interactive)**:
  - Why sidecars? — animated comparison: in-process vs sidecar vs microservice
  - Process lifecycle — spawn, communicate (stdin/stdout), monitor, terminate
  - NDJSON protocol — line-delimited JSON, interactive message builder
  - Error handling — what happens when the sidecar crashes? Animated failure scenarios
  - Bundling — how Tauri packages external binaries (`externalBin` in config)
  - Stdout corruption — the binary data problem (0x0A in PCM), and the file-based fix
- **How Jarvis uses it**: JarvisListen (audio) + IntelligenceKit (AI) — two sidecars, two patterns
- **Try it yourself**: "Design a NDJSON protocol for a hypothetical image-processing sidecar"

---

## Color System

| Layer | Primary Color | Tailwind Prefix | Meaning |
|-------|--------------|-----------------|---------|
| Frontend | Blue/Cyan | `blue-`, `cyan-` | UI, components, state |
| Tauri Bridge | Amber/Orange | `amber-`, `orange-` | IPC, commands, events |
| Audio Pipeline | Green/Emerald | `green-`, `emerald-` | Capture, transcription |
| Browser/Extractors | Purple/Violet | `purple-`, `violet-` | Observation, extraction |
| Intelligence/AI | Rose/Red | `rose-`, `red-` | AI enrichment, LLM |
| Gems/Storage | Yellow/Gold | `yellow-`, `amber-` | Persistence, search |

Dark theme background: `slate-950` / `slate-900`

---

## Tech Stack

Matching the LLM Visualizer for consistency:
- **React 19** + **TypeScript**
- **Vite** — build tool
- **Tailwind CSS 4** — styling
- **No external dependencies** — no routing library, no state management library, no charting library
- All visualizations built with plain React + CSS (flexbox, grid, transitions, keyframes)

---

## Interactive Elements

| Element | Description | Where Used |
|---------|-------------|------------|
| **Zoomable Overview** | 3 zoom levels with smooth transitions | Part 1 |
| **Clickable Nodes** | Click a component/module → expands to show internals | All layers |
| **Data Flow Arrows** | Animated arrows showing data movement | Part 3 flows |
| **Hover Tooltips** | File paths, type signatures, brief explanations | Everywhere |
| **Code Snippets** | Real Rust/TypeScript/Swift code excerpts | Layer details |
| **Toggle Switches** | Show/hide optional subsystems (e.g., AI when unavailable) | Layer 5 |
| **Step-Through Controls** | Next/Prev buttons for animated flows | Part 3 |
| **Collapsible Sidebar** | Navigate between parts and layers | Global |
| **Spec Status Badges** | Green/Yellow/Gray for implementation status | Part 4 |
| **Interactive Code Editor** | Editable code blocks with mock validation | Part 5 guides |
| **Quiz Blocks** | "Try it yourself" exercises with hints and reveal | Part 5 guides |
| **Concept Cards** | Flip-card style — term on front, explanation on back | Part 5 guides |
| **Comparison Tables** | Side-by-side Rust vs Swift, useState vs useReducer | Part 5 guides |
| **Progress Tracker** | Which guides completed, which sections visited | Part 5 sidebar |

---

## Scope Boundaries

### In Scope
- Architecture visualization of the current Jarvis codebase
- All 6 layers with interactive exploration
- 3 animated data flows
- Spec map showing the 13 Kiro specs
- 8 interactive tech-stack 101 guides with "try it yourself" exercises
- Real file paths, struct names, and config values
- Self-contained — runs with `npm run dev`, no backend needed

### Out of Scope
- Live connection to a running Jarvis instance
- Editing or generating code from the visualizer
- Performance profiling or runtime metrics
- Mobile responsiveness (desktop-first, like Jarvis itself)
- Deployment / hosting setup
- Running actual Rust/Swift compilers in the browser (exercises use mock validation)

---

## File Structure (Planned)

```
jarvis-architecture-visualizer/
├── index.html
├── package.json
├── vite.config.ts
├── tsconfig.json
├── src/
│   ├── main.tsx
│   ├── App.tsx                    # Sidebar nav + part routing
│   ├── index.css                  # Global styles, animations, Tailwind
│   ├── data/
│   │   ├── architecture.ts        # All architecture data (components, commands, structs)
│   │   ├── dataFlows.ts           # Step-by-step flow definitions
│   │   ├── specs.ts               # Kiro spec metadata + status
│   │   └── guides/               # Guide content data (concepts, examples, exercises)
│   │       ├── rustData.ts
│   │       ├── tauriData.ts
│   │       ├── swiftData.ts
│   │       ├── reactTSData.ts
│   │       ├── sqliteData.ts
│   │       ├── macosData.ts
│   │       ├── specDrivenData.ts
│   │       └── sidecarData.ts
│   ├── components/
│   │   ├── Sidebar.tsx            # Collapsible navigation
│   │   ├── BigPicture.tsx         # Part 1: Zoomable overview
│   │   ├── LayerExplorer.tsx      # Part 2: Layer container
│   │   ├── layers/
│   │   │   ├── FrontendLayer.tsx
│   │   │   ├── TauriBridgeLayer.tsx
│   │   │   ├── AudioPipelineLayer.tsx
│   │   │   ├── BrowserExtractorLayer.tsx
│   │   │   ├── IntelligenceLayer.tsx
│   │   │   └── GemsLayer.tsx
│   │   ├── DataFlowViewer.tsx     # Part 3: Animated flows
│   │   ├── SpecMap.tsx            # Part 4: Kiro spec visualization
│   │   ├── guides/               # Part 5: Tech Stack 101 guides
│   │   │   ├── GuideShell.tsx     # Shared guide layout (nav, sections, exercises)
│   │   │   ├── RustGuide.tsx
│   │   │   ├── TauriGuide.tsx
│   │   │   ├── SwiftGuide.tsx
│   │   │   ├── ReactTSGuide.tsx
│   │   │   ├── SQLiteGuide.tsx
│   │   │   ├── MacOSApisGuide.tsx
│   │   │   ├── SpecDrivenGuide.tsx
│   │   │   └── SidecarGuide.tsx
│   │   └── shared/
│   │       ├── FlowArrow.tsx      # Animated directional arrow
│   │       ├── CodeSnippet.tsx    # Syntax-highlighted code block
│   │       ├── NodeCard.tsx       # Clickable expandable node
│   │       ├── Tooltip.tsx        # Hover tooltip
│   │       ├── StatusBadge.tsx    # Green/Yellow/Gray badge
│   │       ├── InteractiveCode.tsx # Editable code block with mock validation
│   │       └── QuizBlock.tsx      # "Try it yourself" exercise component
```

---

## Success Criteria

1. A new developer can understand Jarvis's architecture in under 10 minutes by clicking through the visualizer
2. Every major module in the codebase is represented and clickable
3. The 3 data flows accurately trace real code paths
4. All 13 Kiro specs are visible and linked to their codebase locations
5. All 8 tech-stack 101 guides are navigable, with interactive examples and exercises
6. A developer unfamiliar with Rust/Tauri/Swift can learn the essentials through the guides alone
7. Each guide connects back to real Jarvis code — "here's the concept, here's where Jarvis uses it"
8. The visualizer runs standalone with `npm install && npm run dev`
9. Visual style matches the LLM Visualizer (dark theme, color-coded, smooth animations)
