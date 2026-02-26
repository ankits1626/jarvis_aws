# Jarvis Architecture Visualizer — Build Phases

## Overview

8 phases, each self-contained and demo-able. Each phase builds on the previous one.

```
Phase 0 ──→ Phase 1 ──→ Phase 2 ──→ Phase 3 ──→ Phase 4 ──→ Phase 5 ──→ Phase 6 ──→ Phase 7
Scaffold    Big Picture  Layers      Flows       Spec Map    Guide FW    Guide Content Polish
```

---

## Phase Summary

| Phase | Name | Tasks | Key Output |
|-------|------|-------|------------|
| **0** | [Scaffolding](phase-0-scaffolding.md) | 6 | Running app shell with dark theme + sidebar navigation |
| **1** | [Big Picture](phase-1-big-picture.md) | 5 | 3-level zoomable architecture overview |
| **2** | [Layer Explorer](phase-2-layer-explorer.md) | 8 | 6 interactive layer pages with real Jarvis data |
| **3** | [Data Flows](phase-3-data-flows.md) | 6 | 3 step-through animated data flow walkthroughs |
| **4** | [Spec Map](phase-4-spec-map.md) | 6 | 13 Kiro specs in filterable grid with relationships |
| **5** | [Guides Foundation](phase-5-guides-foundation.md) | 8 | Reusable guide framework (code editor, quizzes, cards) |
| **6** | [Guides Content](phase-6-guides-content.md) | 8 | All 8 tech-stack 101 guides fully written |
| **7** | [Polish](phase-7-polish.md) | 8 | Home page, cross-links, animations, keyboard nav |

**Total: 55 tasks across 8 phases**

---

## Dependency Chain

```
Phase 0 (scaffold) — required by everything
  │
  ├── Phase 1 (big picture) — standalone after Phase 0
  │     └── needs FlowArrow from shared components
  │
  ├── Phase 2 (layers) — standalone after Phase 0
  │     └── needs NodeCard, CodeSnippet, Tooltip
  │
  ├── Phase 3 (data flows) — best after Phase 2 (reuses module knowledge)
  │     └── needs FlowArrow, DataFlow data types
  │
  ├── Phase 4 (spec map) — standalone after Phase 0
  │     └── needs StatusBadge
  │
  ├── Phase 5 (guide framework) — standalone after Phase 0
  │     └── builds InteractiveCode, QuizBlock, ConceptCard, etc.
  │
  ├── Phase 6 (guide content) — requires Phase 5
  │     └── populates all 8 guide data files
  │
  └── Phase 7 (polish) — requires all above
        └── cross-links, home page, animation pass
```

### Parallelization Opportunities

These can be built in parallel after Phase 0:
- Phase 1 + Phase 2 (different parts, shared `FlowArrow` only)
- Phase 4 + Phase 5 (completely independent)

Phase 6 must wait for Phase 5.
Phase 7 must wait for everything.

---

## What's Demo-able After Each Phase

| After Phase | You Can Show |
|-------------|-------------|
| 0 | "Here's the app shell — dark theme, sidebar, navigation works" |
| 1 | "Zoom through the 3-level architecture overview" |
| 2 | "Click into any of the 6 layers and explore every module" |
| 3 | "Step through how recording works, end to end" |
| 4 | "See all 13 specs, their status, and how they relate" |
| 5 | "The guide framework — flip cards, quizzes, interactive code" |
| 6 | "Learn Rust, Tauri, Swift, React — all 8 guides complete" |
| 7 | "The finished product — polished, cross-linked, keyboard-navigable" |

---

## Files Created Per Phase

| Phase | New Files | Modified Files |
|-------|-----------|----------------|
| 0 | ~12 | 0 |
| 1 | 2 | 1 |
| 2 | 9 | 1 |
| 3 | 2 | 1 |
| 4 | 2 | 1 |
| 5 | 8 | 0 |
| 6 | 16 | 0 |
| 7 | 1 | ~8 |

**Total: ~50 new files, ~12 modifications**
