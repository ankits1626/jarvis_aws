# Jarvis — Feature Roadmap

**Goal:** 4 features to make story.md demo-able
**Deadline:** March 13, 2026 (~12 working days)

---

## Feature 1: Gem Knowledge Files

Generate a `knowledge/{gem_id}/gem.md` for every gem — a portable, readable document that search and synthesis can consume.

**Components:**
- `generate_knowledge_file(gem, base_path)` — assembles gem data into markdown
- Called on gem save + after enrichment completes
- `regenerate_all_knowledge_files()` — one-time backfill for existing gems

**gem.md format:**
```markdown
# {title}
- Source: {source_type} | URL: {source_url} | Captured: {date}
- Tags: {comma-separated}

## Summary
{ai_enrichment.summary}

## Content
{content, truncated to 10K chars}

## Transcript
{if exists}
```

**Effort:** 1 day

---

## Feature 2: Projects

Group gems into projects with metadata. Enable synthesis across project gems.

**Components:**

| Component | Description |
|-----------|-------------|
| `projects` table | id, name, description, objective, deadline, status, timestamps |
| `project_gems` table | junction table linking projects ↔ gems |
| CRUD commands | create, list, get, update, delete project + add/remove gems |
| `synthesize_project(id)` | Load all project gems → LLM synthesis → save as new "synthesis" gem |
| Frontend | Projects nav, list view, detail view, create dialog, "Add to Project" on gems |

**Tauri Commands:**
- `create_project(name, description, objective, deadline)`
- `list_projects()` / `get_project(id)` / `update_project(id, ...)` / `delete_project(id)`
- `add_gem_to_project(project_id, gem_id)` / `remove_gem_from_project(project_id, gem_id)`
- `synthesize_project(project_id)` → creates synthesis Gem

**Effort:** 3 days (DB + commands: 1d, UI: 1.5d, synthesis: 0.5d)

---

## Feature 3: Semantic Search for Gems

Find relevant gems by meaning, not just keywords. Two-step approach — no embeddings, no new dependencies.

**Components:**

| Step | What | How |
|------|------|-----|
| FTS5 candidate retrieval | Get top 20 keyword matches | Existing FTS5 index, enhanced to search gem.md content |
| LLM relevance scoring | Rate each candidate 0–100 | Send gem summary + project context to LLM via IntelQueue |

**Tauri Commands:**
- `find_relevant_gems(project_id)` → `Vec<{gem, score, reason}>`

**Frontend:**
- "Find Related Gems" panel in project creation/detail
- Gem cards with relevance score badge + "Add" button

**Effort:** 1–2 days

---

## Feature 4: Web Search Recommendations (Tavily)

Suggest research topics and links based on project context + existing gem tags/summaries.

**Components:**

| Step | What | How |
|------|------|-----|
| Context assembly | Gather project + gem signals | Project description + objective + gem tags + gem summaries |
| Topic generation | LLM suggests 5 research topics | "Given this context, suggest 5 topics with search queries" |
| Web search | Tavily API per topic | `POST api.tavily.com/search` — free tier: 1K searches/mo |

**Tauri Commands:**
- `suggest_research(project_id)` → `Vec<{topic, query, results: [{title, url, snippet}]}>`

**Frontend:**
- "Suggested Research" panel in project view
- Topic headings with link cards, "Open in Chrome" buttons

**Config:** Tavily API key in settings/.env

**Effort:** 1–2 days

---

## Build Sequence

```
Day 1:     Gem Knowledge Files — gem.md generation + backfill
Day 2–3:   Projects — DB, CRUD commands, basic UI
Day 4:     Projects UI — gem assignment, project detail, synthesis
Day 5:     Semantic Search — FTS5 enhancement + LLM scoring
Day 6:     Tavily — API integration + topic suggestions + UI
Day 7:     Integration test — run full story.md flow end-to-end
Day 8:     Bug fixes + polish
Day 9–10:  Record demo video
Day 11–12: Write + publish article
Day 13:    Buffer
```

---

## Dependencies

```
Feature 1 (Knowledge Files)
    ├──► Feature 3 (Semantic Search) — searches gem.md
    └──► Feature 2 (Projects) ←── Feature 4 (Tavily) needs project context
              └──► Synthesis (sub-feature)
```

Build order: **1 → 2 → 3 → 4**

---

## Summary

| # | Feature | Effort | Story Acts |
|---|---------|--------|------------|
| 1 | Gem Knowledge Files | 1 day | Foundation for search + synthesis |
| 2 | Projects + Synthesis | 3 days | Act 2, Act 4 |
| 3 | Semantic Search | 1–2 days | Act 2 (gem recommendations) |
| 4 | Tavily Web Search | 1–2 days | Act 3 (research suggestions) |
| | **Total dev** | **6–8 days** | |
| | **Demo + Article** | **5 days** | |
