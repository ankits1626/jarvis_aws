# Phase 6: Tech Stack 101 — Guide Content (8 Guides)

> The content phase. Populate all 8 guides with real educational content.

---

## Goal

Write the actual content for all 8 tech-stack guides. Each guide is a data file that feeds into the `GuideShell` framework from Phase 5. Every guide follows: **Why it exists → Core mental model → Key concepts (interactive) → How Jarvis uses it → Try it yourself**.

---

## Tasks

### 6.1 — Rust 101 Guide
- Create `src/data/guides/rustData.ts`
- **Sections**:
  1. "Why Rust?" — text + comparison card (Rust vs C++ vs Go)
  2. "Ownership & Borrowing" — concept cards for ownership, borrow, lifetime + diagram showing move vs borrow
  3. "Structs & Implementations" — interactive code: build a `Gem` struct, add a method
  4. "Error Handling" — `Result<T, E>` flow diagram, click through Ok/Err branches, quiz
  5. "Traits" — concept cards, diagram showing `IntelProvider` trait with 2 implementations
  6. "Concurrency" — `Arc`, `Mutex`, `RwLock` animated diagram, comparison table
  7. "Async & Tokio" — event loop diagram, concept cards for `async`, `await`, `spawn`
  8. "Modules & Crates" — Jarvis file tree → module tree mapping diagram
- **Exercises**: Fix borrow checker error, implement a trait method, match on Result
- **Jarvis connections**: 5 real examples from `commands.rs`, `recording.rs`, `gems/store.rs`, `intelligence/provider.rs`, `transcription/manager.rs`

### 6.2 — Tauri 101 Guide
- Create `src/data/guides/tauriData.ts`
- **Sections**:
  1. "What is Tauri?" — text + comparison (Tauri vs Electron)
  2. "Architecture" — diagram: Webview ↔ IPC ↔ Rust, concept cards
  3. "Commands" — interactive code: write a `#[tauri::command]`, see the invoke call
  4. "State Management" — diagram: `.manage()` → `State<'_, T>`, comparison of lock types
  5. "Events" — animated message bubble diagram, concept cards for emit/listen
  6. "Plugins" — puzzle-piece diagram of plugin chain, quiz on which plugin does what
  7. "Configuration" — interactive `tauri.conf.json` explorer (click fields)
  8. "Sidecars" — `externalBin` config, spawn lifecycle diagram
- **Exercises**: Trace a command end-to-end, identify state threading for a new command
- **Jarvis connections**: `lib.rs` builder chain, `commands.rs` examples, `tauri.conf.json`

### 6.3 — Swift 101 Guide
- Create `src/data/guides/swiftData.ts`
- **Sections**:
  1. "Why Swift for Sidecars?" — text + comparison table (Swift vs Rust for Apple APIs)
  2. "Swift Basics" — concept cards: optionals, structs, enums, protocols
  3. "Async/Await in Swift" — side-by-side comparison with Rust async
  4. "ScreenCaptureKit" — diagram of audio capture pipeline, permission flow
  5. "Foundation Models" — on-device LLM diagram, `@Generable`/`@Guide` concept cards
  6. "Stdin/Stdout Communication" — NDJSON protocol step-through interactive
  7. "Process Lifecycle" — spawn → communicate → terminate diagram
- **Exercises**: Read NDJSON exchange and predict response, design a capture pipeline
- **Jarvis connections**: `jarvis-listen/` source, `intelligence-kit/` source

### 6.4 — React + TypeScript 101 Guide
- Create `src/data/guides/reactTSData.ts`
- **Sections**:
  1. "UI = f(state)" — core mental model diagram
  2. "Components" — build a tree interactively, concept cards for props/children
  3. "State: useState vs useReducer" — comparison table, interactive toggle demo
  4. "Custom Hooks" — extract logic pattern, lifecycle diagram
  5. "TypeScript Essentials" — type annotations, interfaces, generics concept cards
  6. "Event Handling" — click trace diagram, quiz
  7. "Conditional Rendering" — toggle states, watch UI change
- **Exercises**: Fix a state transition bug, type a function signature, build a custom hook
- **Jarvis connections**: `App.tsx`, `RecordButton.tsx`, `useRecording.ts`, `state/reducer.ts`

### 6.5 — SQLite + FTS5 101 Guide
- Create `src/data/guides/sqliteData.ts`
- **Sections**:
  1. "Database in a File" — concept card, comparison (SQLite vs Postgres vs Mongo)
  2. "Tables & Schema" — interactive table builder
  3. "CRUD Operations" — interactive SQL: type query, see results against mock data
  4. "Constraints & Upsert" — UNIQUE constraint demo, `INSERT OR REPLACE` walkthrough
  5. "FTS5 Full-Text Search" — inverted index diagram, tokenization walkthrough
  6. "Virtual Tables" — concept: `gems_fts` mirrors `gems`, sync diagram
- **Exercises**: Write a search query, explain an upsert scenario, design a new FTS table
- **Jarvis connections**: `gems/sqlite_store.rs`, schema creation code, search implementation

### 6.6 — macOS System APIs 101 Guide
- Create `src/data/guides/macosData.ts`
- **Sections**:
  1. "The OS as a Platform" — concept cards for app sandbox, permissions, system services
  2. "AppleScript & osascript" — command builder, example scripts for Chrome interaction
  3. "Accessibility API" — DOM-like tree diagram for native apps, AXUIElement concept cards
  4. "ScreenCaptureKit" — permission flow diagram, audio routing
  5. "Named Pipes (FIFO)" — animated producer-consumer, concept cards
  6. "File System Conventions" — `~/.jarvis/` directory explorer
- **Exercises**: Trace permission request flow, predict AppleScript output
- **Jarvis connections**: `browser/adapters/chrome.rs`, `browser/accessibility.rs`, `recording.rs`

### 6.7 — Spec-Driven Development 101 Guide
- Create `src/data/guides/specDrivenData.ts`
- **Sections**:
  1. "Requirements First, Code Second" — concept cards, benefits diagram
  2. "The Kiro Workflow" — `requirements.md` → `design.md` → code → validate pipeline
  3. "Anatomy of a Requirement" — user story template, acceptance criteria, task breakdown
  4. "Anatomy of a Design Document" — data models, API contracts, error handling
  5. "Spec Evolution" — interactive: `jarvis-browser-vision` → `v2` diff view
  6. "ADRs" — template builder, concept cards for context/decision/consequences
  7. "Traceability" — click a requirement → see which files implement it
- **Exercises**: Draft user stories for a feature, write an ADR
- **Jarvis connections**: All 13 Kiro specs with direct references

### 6.8 — The Sidecar Pattern 101 Guide
- Create `src/data/guides/sidecarData.ts`
- **Sections**:
  1. "Why Sidecars?" — comparison diagram: in-process vs sidecar vs microservice
  2. "Process Lifecycle" — spawn → stdin/stdout → monitor → terminate diagram
  3. "NDJSON Protocol" — format explanation, interactive message builder
  4. "Error Handling" — crash scenarios: what if sidecar dies? what if it hangs?
  5. "Bundling with Tauri" — `externalBin` config, build pipeline
  6. "The Stdout Corruption Story" — before/after: binary 0x0A in PCM vs file-based fix
- **Exercises**: Design a NDJSON protocol, identify sidecar failure modes
- **Jarvis connections**: `recording.rs` (JarvisListen), `intelligence/intelligencekit_provider.rs` (IntelligenceKit)

---

## Expected Output

8 complete guides, each with:
- 5-8 sections of mixed content (text, diagrams, code, quizzes, exercises)
- 2-3 interactive exercises with hints and solutions
- 3-5 "How Jarvis uses it" connections to real code
- Concept cards for key terminology
- At least one comparison table per guide
- At least one quiz per guide

### Files Created
```
src/data/guides/rustData.ts
src/data/guides/tauriData.ts
src/data/guides/swiftData.ts
src/data/guides/reactTSData.ts
src/data/guides/sqliteData.ts
src/data/guides/macosData.ts
src/data/guides/specDrivenData.ts
src/data/guides/sidecarData.ts
```

### Guide Components (thin wrappers around GuideShell)
```
src/components/guides/RustGuide.tsx
src/components/guides/TauriGuide.tsx
src/components/guides/SwiftGuide.tsx
src/components/guides/ReactTSGuide.tsx
src/components/guides/SQLiteGuide.tsx
src/components/guides/MacOSApisGuide.tsx
src/components/guides/SpecDrivenGuide.tsx
src/components/guides/SidecarGuide.tsx
```

---

## Definition of Done
- [ ] All 8 guides render in GuideShell with correct content
- [ ] Each guide has working interactive exercises (edit, run, validate)
- [ ] Each guide has at least 1 quiz with feedback
- [ ] Concept cards flip correctly in all guides
- [ ] Comparison tables render for all applicable guides
- [ ] "How Jarvis uses it" section shows real file paths and descriptions
- [ ] Progress tracking works across all guides
- [ ] Content is technically accurate (matches Jarvis codebase reality)
- [ ] No placeholder or "TODO" text remains
