# Phase 1: The Big Picture (Part 1)

> First real content. The 30-second overview of Jarvis.

---

## Goal

A zoomable, animated overview of Jarvis with 3 zoom levels. User starts at the simplest view and can zoom in to see more detail. Each zoom level builds on the previous one — boxes expand, new connections appear, labels get more specific.

---

## Tasks

### 1.1 — Architecture data file
- Create `src/data/architecture.ts`
- Define types: `SystemNode`, `Connection`, `ZoomLevel`
- Populate Level 1 data:
  - 3 nodes: "User Input" → "Jarvis" → "Knowledge Base"
  - 2 connections with labels
- Populate Level 2 data:
  - "Jarvis" expands into 3 pillars: Frontend, Backend, Sidecars
  - Show IPC arrows between them
- Populate Level 3 data:
  - 6 subsystem nodes: Frontend, Tauri Bridge, Audio Pipeline, Browser/Extractors, Intelligence, Gems
  - All inter-system connections with labels (e.g., "Tauri events", "FIFO pipe", "NDJSON")

### 1.2 — BigPicture component
- Build `BigPicture.tsx`
- Zoom control: 3 dots/buttons at top indicating current level (1, 2, 3)
- Keyboard support: arrow keys or scroll to zoom in/out
- Smooth CSS transitions between zoom levels (scale + opacity)

### 1.3 — Node rendering
- Each node is a rounded box with:
  - Layer-appropriate color (border + subtle background)
  - Icon (emoji or simple SVG)
  - Title
  - One-line description
- Nodes at Level 3 are clickable → navigates to that layer in Part 2

### 1.4 — Connection rendering
- Implement `FlowArrow.tsx` shared component
- Arrows between nodes with:
  - Animated dashed line (CSS animation)
  - Label on the arrow (e.g., "IPC commands", "PCM audio")
  - Direction indicator (arrowhead)
- Arrows appear/disappear with zoom level transitions

### 1.5 — Annotations
- Each zoom level has a brief explanation text above the diagram:
  - Level 1: "Jarvis is a desktop assistant that captures what you see and hear, and turns it into searchable knowledge."
  - Level 2: "Three layers work together: a React frontend you interact with, a Rust backend that orchestrates everything, and Swift sidecars for macOS-specific work."
  - Level 3: "Six subsystems handle different responsibilities. Click any to explore."

---

## Expected Output

What you see at **Level 1**:
```
[ User Input ] ──→ [ Jarvis ] ──→ [ Knowledge Base ]
     🎤                ⚡               📚
  "speaks,          "captures,        "gems stored,
   browses"        transcribes,       searchable,
                    enriches"         AI-enriched"
```

What you see at **Level 2** (Jarvis box expands):
```
[ User Input ] ──→ [ Frontend ] ←──IPC──→ [ Backend ] ←──spawn──→ [ Sidecars ]
                      React              Rust/Tauri            Swift
```

What you see at **Level 3** (full map with 6 subsystems and all connections)

### Interactions
- Click zoom dots to switch levels
- Hover a node → tooltip with key file paths
- Click a Level 3 node → navigates to Part 2 layer
- Smooth 300ms transition between levels

### Files Created/Modified
```
src/data/architecture.ts          (NEW)
src/components/BigPicture.tsx      (REPLACE placeholder)
src/components/shared/FlowArrow.tsx (IMPLEMENT)
```

---

## Definition of Done
- [ ] All 3 zoom levels render correctly
- [ ] Transitions between levels are smooth (no flicker, no layout shift)
- [ ] Level 3 nodes are clickable and navigate to correct Part 2 layer
- [ ] FlowArrow component shows animated directional arrows with labels
- [ ] Annotation text updates per zoom level
- [ ] Works with keyboard (arrow keys)
