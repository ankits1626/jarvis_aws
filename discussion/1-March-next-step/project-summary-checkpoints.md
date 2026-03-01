# Project Summary Checkpoints

**Date:** March 1, 2026
**Status:** Proposal

---

## Mental Model

```
Step 1: BUILD — Concatenate all gems + their knowledge .md files into one giant document
Step 2: CHUNK — Slide a window across the document, LLM summarizes each chunk
Step 3: REVIEW — Show the assembled summary to the user in Summary Chat
Step 4: INTERACT — User can save as gem, or ask questions about it
```

---

## Summary Chat vs Research Chat

These are two separate tabs in the project right panel. Different purpose, different UI, different state.

```
┌──────────┬────────────────────────┬─────────────────────────────────┐
│ LeftNav  │   Center Content       │        RightPanel                │
│          │                        │                                  │
│ Projects │  ProjectsContainer     │  [Research] [Summary] [Detail]   │
│          │  ┌──────┬────────────┐ │                                  │
│          │  │ List │ Gems       │ │   (one tab active at a time)     │
│          │  │      │            │ │                                  │
│          │  └──────┴────────────┘ │                                  │
└──────────┴────────────────────────┴─────────────────────────────────┘
```

| | Research Chat | Summary Chat |
|---|---|---|
| **Purpose** | Discover and collect gems | Distill what you've collected |
| **Trigger** | Auto-opens when project selected | User clicks "Summary" tab or button |
| **LLM work** | Topic suggestion, web search | Chunked summarization of all gems |
| **Output** | Web result cards, gem suggestions | One formatted summary document |
| **Save action** | Adds existing gems to project | Creates a new summary gem |
| **Chat context** | Project metadata + gem titles | Full composite doc + generated summary |
| **Component** | `ProjectResearchChat` (existing) | `ProjectSummaryChat` (new) |

---

## Step 1: Build the Giant Document

Each gem in the project has a knowledge directory with .md files:
- `content.md` — raw extracted content
- `enrichment.md` — AI tags + summary
- `transcript.md` — transcript (if audio gem)
- `copilot.md` — co-pilot analysis (if recording gem)
- `gem.md` — assembled master document

**Action:** For each gem (sorted chronologically by `captured_at`), concatenate their knowledge files into one giant markdown document.

```markdown
========================================
GEM: "Understanding ECS Task Definitions"
Source: Article — medium.com
Captured: Feb 15, 2026
========================================

[contents of gem.md or individual subfiles]

========================================
GEM: "AWS re:Invent 2025 - ECS Best Practices"
Source: Video — youtube.com
Captured: Feb 15, 2026
========================================

[contents of gem.md or individual subfiles]

... (all gems)
```

**Which .md to use per gem?**
- Simplest: just use `gem.md` (it's already the assembled master doc with everything)
- Fallback if no knowledge files: use `gem.title + gem.description + gem.content` directly from DB

---

## Step 2: Chunk + Summarize

The giant doc may be too large for a single LLM context window (Qwen3-8B has ~8K-32K context depending on model).

**Windowed approach:**
1. Split the giant doc into chunks of ~X tokens (e.g., 4000 tokens per chunk)
2. For each chunk, LLM call: "Summarize this section, extract key points and highlights"
3. Collect all chunk summaries
4. Optional final pass: LLM merges chunk summaries into one cohesive summary

```
Giant Doc (e.g., 30K tokens)
  ├── Chunk 1 (4K tokens) → LLM → Summary 1
  ├── Chunk 2 (4K tokens) → LLM → Summary 2
  ├── Chunk 3 (4K tokens) → LLM → Summary 3
  └── ...

Chunk summaries combined → Final LLM pass → Complete Summary
```

**Smart chunking:** Split on gem boundaries, not mid-sentence. Each chunk = 1 or more complete gems. This preserves context per gem.

---

## Step 3: Show for Review (Summary Chat)

The summary appears in the **Summary Chat** tab — a dedicated space separate from Research Chat.

```
┌─────────────────────────────────────┐
│  [Research] [Summary*] [Detail]      │
├─────────────────────────────────────┤
│                                      │
│  Summary of "ECS Migration" project  │
│  12 gems analyzed                    │
│                                      │
│  ┌─────────────────────────────────┐ │
│  │ ## Feb 15, 2026                 │ │
│  │ **"Understanding ECS..."**      │ │
│  │ - Key point 1                   │ │
│  │ - Key point 2                   │ │
│  │                                 │ │
│  │ ## Feb 18, 2026                 │ │
│  │ **"Fargate Pricing..."**        │ │
│  │ - Key point 1                   │ │
│  │ ...                             │ │
│  └─────────────────────────────────┘ │
│                                      │
│  [Save as Gem]                       │
│                                      │
│  ┌────────────────────────────────┐  │
│  │ Ask about the summary...       │  │
│  └────────────────────────────────┘  │
└─────────────────────────────────────┘
```

**Summary Chat states:**

1. **Empty** — No summary generated yet. Shows "Generate Summary" button.
2. **Generating** — Progress indicator: "Analyzing 12 gems... chunk 2 of 4"
3. **Review** — Summary displayed as formatted markdown. "Save as Gem" button + chat input for questions.
4. **Saved** — Summary saved. Badge/indicator shows "Saved as gem". Chat still available.

---

## Step 4: User Decides

**"Save as Gem"** button → Save the summary as a gem:
- `source_type`: `"ProjectSummary"`
- `source_url`: `jarvis://project/{id}/summary/{timestamp}`
- `title`: `"Summary: {project_title} — Mar 1, 2026"`
- `content`: the full summary markdown
- Auto-add to project, generate knowledge files, index for search

**Ask questions** → Chat input below the summary:
- "What were the main cost findings?"
- "Which gems talked about networking?"
- "Expand on the Fargate pricing points"
- Context = composite doc + generated summary (grounded in the actual source material)

**"Regenerate"** → Re-runs the pipeline. Useful after adding new gems.

User can ask questions first, then save when satisfied. Or save immediately. Or never save.

---

## Summary Gem Knowledge Directory

When saved, the summary gem gets its own knowledge folder just like any other gem. But it also includes a special file — the **composite source document** that was used to generate it.

```
knowledge/{summary_gem_id}/
├── composite_summary_of_all_gems.md   ← the giant concatenated input doc
├── content.md                          ← the LLM-generated summary (same as gem content)
├── enrichment.md                       ← auto-enriched tags/summary
├── gem.md                              ← assembled master document
└── meta.json
```

### `composite_summary_of_all_gems.md`

This is the full concatenated document that was fed to the LLM — all project gems and their knowledge files stitched together chronologically. It serves as:

1. **Provenance** — you can always see exactly what the LLM was working with
2. **Searchable archive** — FTS/semantic search indexes this file, so searching for any detail from any source gem also hits the summary gem
3. **Chat context** — when user asks questions about the summary, this file provides the full source material for grounded answers
4. **Reproducibility** — regenerating the summary from the same composite should yield similar results

This file is written at generation time (Step 1), before the LLM is even called. It persists regardless of whether the user saves the summary as a gem — but it only lands in the knowledge directory once saved.

---

## Implementation

### Backend

```
ProjectResearchAgent::generate_summary(project_id) -> Result<(String, String), String>
  1. Load project gems (sorted by captured_at)
  2. For each gem: load gem.md from KnowledgeStore (fallback to DB content)
  3. Concatenate into giant document (= composite_doc)
  4. Chunk by gem boundaries (fit within context window)
  5. LLM summarizes each chunk
  6. LLM merges chunk summaries into final summary
  7. Return (summary, composite_doc) — NOT saved yet

ProjectResearchAgent::save_summary_as_gem(project_id, summary_content, composite_doc) -> Result<Gem, String>
  1. Create gem with source_type "ProjectSummary"
  2. Save to GemStore
  3. Add to project
  4. Generate knowledge files (content.md, enrichment.md, gem.md)
  5. Write composite_summary_of_all_gems.md into the knowledge directory
  6. Index for search
  7. Return created gem
```

### Tauri Commands

```
generate_project_summary(project_id) → { summary: String, composite_doc: String }
save_project_summary(project_id, summary, composite_doc) → Gem
```

Two separate commands — generate is decoupled from save.

### Frontend

**New component:** `ProjectSummaryChat.tsx`
- Props: `projectId`, `projectTitle`, `onGemSaved`
- State: `summary`, `compositeDoc`, `generating`, `saved`, `chatMessages`
- Renders: summary preview + save button + chat input

**RightPanel change:** Add "Summary" tab alongside "Research" and "Detail"
- `[Research] [Summary] [Detail]`
- Summary tab renders `ProjectSummaryChat`
- Detail tab only shows when a gem is selected (existing behavior)

---

## Chunking Strategy

**Window size:** ~4000 tokens (~3000 words). Conservative for Qwen3-8B.

**Splitting logic:**
1. Walk through gems chronologically
2. Accumulate gems into current chunk until adding next gem would exceed window size
3. Start new chunk
4. Each chunk has complete gems — never split mid-gem

**If a single gem exceeds window size:** Truncate that gem's content to fit, with a note.

**Merge pass prompt:**
```
You have summaries of different sections of a research project.
Combine them into one cohesive summary. Preserve all key points.
Group by date. Keep the chronological structure.
```

---

## Edge Cases

- **0 gems** → "Add some gems first" in Summary tab
- **1 gem** → Skip chunking, single LLM call
- **All gems fit in one chunk** → Skip merge pass, single LLM call
- **Gem has no knowledge files and no content** → Skip, note in summary
- **LLM fails mid-chunk** → Return partial summary with error note
- **User never saves** → That's fine, summary was just for review
- **User switches projects** → Summary Chat resets (keyed by projectId)
- **New gems added after summary** → "Regenerate" re-runs with updated gem list
