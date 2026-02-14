# JARVIS — Session Context for Continuation

## Project: JARVIS — AWS 10,000 AIdeas Competition

**Competition:** AWS 10,000 AIdeas ($250K prizes, Workplace Efficiency track, deadline Mar 13, 2026). I'm a semi-finalist.

**What JARVIS does:** Real-time AI assistant that listens to live conversations (including WhatsApp calls with AirPods) and surfaces relevant context.

**Repo:** `/Users/ankit/code/learn/jarvis_aws/jarvis_aws/`

---

## Workflow

I use **Kiro IDE** to generate specs and code. **Claude** reviews specs and provides feedback as "Kiro fix instructions." Claude does NOT edit code directly — only reviews. I tell Kiro to fix, then ask Claude to review again.

I do NOT know Rust, Tauri, or many of the technologies being used. That's why the specs include Requirement 13 (coaching 101 guides for each technology, delivered BEFORE it's used).

---

## Current State

### Module 1: JarvisListen (Swift CLI) — DONE
- **Location:** `jarvis-listen/`
- macOS audio capture tool using ScreenCaptureKit (macOS 15+)
- Captures system audio + microphone, outputs interleaved PCM to stdout
- All 16 main tasks complete, some test sub-tasks unchecked (12.3, 12.4, 12.5, 15.1-15.3)
- Key files: `main.swift`, `AudioCapture.swift`, `PCMConverter.swift`, `AudioCaptureProvider.swift`
- Specs: `.kiro/specs/jarvis-listen/tasks.md`

**PREREQUISITE NEEDED:** JarvisListen currently only writes PCM to stdout. Must add `--output <filepath>` flag so the Tauri app can use it (Tauri's shell plugin corrupts binary stdout by splitting on newline bytes `0x0A` that naturally occur in PCM data). This needs a jarvis-listen spec update before jarvis-app can function.

### Module 2: Jarvis Desktop App (Tauri v2 + React) — SPECS APPROVED
- **Specs location:** `.kiro/specs/jarvis-app/`
- `requirements.md` — **APPROVED (10/10)** after 5+ review iterations
- `design.md` — **APPROVED (10/10)** after 2 review iterations with web-search validation
- `tasks.md` — **NOT YET GENERATED** (next step: Kiro generates this)

**Key design decisions in approved specs:**
1. Sidecar pattern — bundle JarvisListen CLI, spawn with `--mono --sample-rate 16000 --output <filepath>`
2. Direct file writing via `--output` flag (avoids binary stdout corruption through Tauri shell plugin)
3. Tauri v2 capabilities (NOT v1 allowlist) in `src-tauri/capabilities/default.json`
4. `tauri-plugin-shell = "2"` and `tauri-plugin-global-shortcut = "2"` in Cargo.toml
5. `std::sync::Mutex` for Tauri state (not tokio::sync::Mutex)
6. `created_at` as `u64` Unix timestamp (not SystemTime) for serde compatibility
7. `.manage()` calls for RecordingManager and FileManager in main setup
8. Event-driven state: click -> "processing" -> backend confirms via event -> "recording"
9. REMOVE_RECORDING action in AppAction union for deletion
10. `binaries/JarvisListen` in externalBin config (Tauri auto-appends target triple)

### Module 3: Transcribe — NOT STARTED
- Plan: local whisper.cpp for dev (avoid AWS credits), AWS backend for production

### Module 4: Full Pipeline — NOT STARTED
- Wire: UI -> capture -> transcribe -> display

---

## Critical Issues Discovered During Reviews

1. **Binary stdout corruption** — Tauri shell plugin splits stdout on `\n` bytes (0x0A), corrupting raw PCM data. Solution: `--output <filepath>` flag.
2. **Tauri v2 config** — Uses capabilities/permissions system, NOT v1 allowlist. Capabilities go in `src-tauri/capabilities/default.json`.
3. **Global shortcuts** — Require `tauri-plugin-global-shortcut` plugin (separate from core in Tauri v2).

---

## Next Steps (in order)
1. Kiro generates `tasks.md` for jarvis-app spec
2. Update jarvis-listen spec to add `--output <filepath>` flag
3. Implement `--output` in JarvisListen
4. Build the Tauri desktop app (following tasks.md)
5. Build Transcribe module
6. Wire full pipeline
