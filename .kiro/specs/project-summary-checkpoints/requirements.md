# Project Summary Checkpoints — LLM-Generated Project Summaries

## Introduction

When a project accumulates many gems (articles, videos, recordings, notes), there's no way to get a comprehensive, structured overview of everything that's been collected. The existing `get_project_summary` command produces a lightweight paragraph in the research chat — useful for a quick glance, but it doesn't cover all key points, isn't persistent, and isn't searchable.

This spec adds **Project Summary Checkpoints** — a dedicated Summary Chat tab in the project right panel where users can generate an LLM-powered summary covering all key points from every gem in the project. The summary is shown for review, the user can ask questions about it, and when satisfied they can save it as a gem — creating a searchable, reusable checkpoint of their research at that point in time.

The core approach: concatenate all project gems and their knowledge `.md` files into one giant document, chunk it by gem boundaries, summarize each chunk via the local LLM, merge into a final summary, and present for review.

**Reference:** Discussion doc at `discussion/1-March-next-step/project-summary-checkpoints.md`. Depends on: [Project Research Assistant spec](../project-research-assistant/requirements.md) (`ProjectResearchAgent`, `ProjectChatSource`, research chat), [Projects spec](../projects/requirements.md) (`ProjectStore`, `ProjectDetail`), [Intelligence spec](../intelligence-kit/requirements.md) (`IntelProvider::chat`), [Knowledge spec](../gem-knowledge-files/requirements.md) (`KnowledgeStore`, `gem.md` assembly).

## Glossary

- **Composite Document**: The giant markdown file created by concatenating all project gems' knowledge files (`gem.md`) in chronological order. This is the input to the LLM summarization pipeline.
- **Chunk**: A segment of the composite document sized to fit within the LLM's context window. Chunks are split on gem boundaries — never mid-gem.
- **Summary Checkpoint**: The LLM-generated summary document, saved as a gem. Multiple checkpoints over time show how research evolved.
- **Summary Chat**: A dedicated tab in the project right panel (`ProjectSummaryChat` component) — separate from the Research Chat tab. Used to generate, review, question, and save summaries.
- **`composite_summary_of_all_gems.md`**: A special file in the summary gem's knowledge directory that stores the full composite document used as LLM input. Provides provenance, searchability, and chat context.

## Frozen Design Decisions

These decisions were made during design discussion (2026-03-01):

1. **Summary Chat is separate from Research Chat.** They are different tabs in the project right panel. Research is for discovering/collecting. Summary is for distilling what's been collected.
2. **Generate and save are decoupled.** Two separate Tauri commands. The user reviews the summary before deciding to save. They can also ask questions first.
3. **Chunked summarization with gem-boundary splitting.** The composite document is split into chunks that respect gem boundaries. Each chunk is summarized independently, then a merge pass combines them.
4. **`gem.md` as the per-gem input.** Each gem's assembled knowledge file (`gem.md`) is the input — it already combines content, enrichment, transcript, and copilot data. Fallback to DB fields if no knowledge files exist.
5. **`composite_summary_of_all_gems.md` stored in knowledge directory.** The full input document is preserved alongside the summary output for provenance, search, and chat context.
6. **Summary saved as a gem with `source_type: "ProjectSummary"`.** This makes it a first-class citizen — searchable, has knowledge files, can be added to other projects.
7. **Single LLM call when everything fits.** If all gems fit within the context window, skip chunking and merge — just one LLM call.
8. **Chronological ordering by `captured_at`.** Gems are sorted oldest-first to tell the research progression story.

---

## Requirement 1: Build Composite Document from Project Gems

**User Story:** As the system, I need to assemble all project gems and their knowledge files into a single chronological document, so it can be fed to the LLM for summarization.

### Acceptance Criteria

1. THE System SHALL load all gems for a project via `ProjectStore::get(project_id)` and sort them by `captured_at` ascending (oldest first)
2. FOR each gem, THE System SHALL attempt to load `gem.md` from the `KnowledgeStore` via `get_assembled(gem_id)`
3. IF `gem.md` is not available for a gem, THE System SHALL fall back to assembling content from the gem's DB fields: `title`, `description`, `content`, `ai_enrichment.summary`, `transcript`
4. THE composite document SHALL be formatted as markdown with clear gem separators containing: gem title, source type, domain, and captured date
5. THE gems SHALL appear in the composite document in chronological order by `captured_at`
6. IF a gem has no content, no knowledge files, and no meaningful DB fields, THE System SHALL skip it and note the skip in the composite document
7. THE composite document SHALL include a header with: project title, project objective (if set), total gem count, and date range of gems

---

## Requirement 2: Chunked LLM Summarization

**User Story:** As the system, I need to summarize the composite document using a windowed approach, so projects of any size can be summarized without exceeding the LLM context window.

### Acceptance Criteria

1. THE System SHALL split the composite document into chunks that fit within the LLM context window (~4000 tokens per chunk)
2. CHUNKS SHALL be split on gem boundaries — a gem's content SHALL NOT be split across two chunks
3. IF a single gem's content exceeds the chunk size, THE System SHALL truncate that gem's content to fit, preserving the title and metadata
4. FOR each chunk, THE System SHALL call `IntelProvider::chat` with a summarization prompt asking for key points and highlights from the content in that chunk
5. AFTER all chunks are summarized, IF there were multiple chunks, THE System SHALL perform a merge pass: call `IntelProvider::chat` with all chunk summaries to produce one cohesive final summary
6. IF all gems fit within a single chunk, THE System SHALL skip chunking and the merge pass — use a single LLM call
7. THE final summary SHALL be formatted as markdown with: chronological grouping by date, key points per gem, and a synthesis section
8. ALL LLM calls SHALL go through the existing `IntelProvider::chat` method
9. ALL operations SHALL log via `eprintln!` with prefix `Projects/Summary:`

---

## Requirement 3: Generate Summary Tauri Command (Decoupled from Save)

**User Story:** As the frontend, I need a command that generates the project summary and returns it for review, without saving anything — so the user can review, ask questions, and decide whether to save.

### Acceptance Criteria

1. THE System SHALL expose a `generate_project_summary_checkpoint` Tauri command accepting `project_id: String`
2. THE command SHALL use `State<'_, Arc<TokioMutex<ProjectResearchAgent>>>` to access the agent
3. THE command SHALL return `Result<ProjectSummaryResult, String>` containing both the `summary` (String) and the `composite_doc` (String)
4. THE command SHALL NOT save anything to the database or filesystem
5. IF the project has 0 gems, THE command SHALL return an error with message: "This project has no gems yet. Add some resources first."
6. IF the LLM fails during any chunk, THE System SHALL return a partial summary with the chunks that succeeded, plus a note about the failure
7. THE `ProjectSummaryResult` struct SHALL be defined in `src/agents/project_agent.rs` with `Serialize`/`Deserialize` derives

---

## Requirement 4: Save Summary as Gem Tauri Command

**User Story:** As the frontend, I need a command that saves a reviewed summary as a gem and adds it to the project, so the user can persist their summary checkpoint after review.

### Acceptance Criteria

1. THE System SHALL expose a `save_project_summary_checkpoint` Tauri command accepting `project_id: String`, `summary_content: String`, and `composite_doc: String`
2. THE command SHALL create a new gem with:
   - `source_type`: `"ProjectSummary"`
   - `source_url`: `jarvis://project/{project_id}/summary/{unix_timestamp}` (unique per generation)
   - `title`: `"Summary: {project_title} — {formatted_date}"`
   - `content`: the `summary_content` parameter
   - `domain`: `"jarvis"`
   - `description`: `"Summary of {n} gems from {project_title}"`
3. THE command SHALL save the gem via `GemStore::save`
4. THE command SHALL add the gem to the project via `ProjectStore::add_gems(project_id, [gem_id])`
5. THE command SHALL generate knowledge files via `KnowledgeStore::create(gem)` — producing `content.md`, `enrichment.md`, `gem.md`, `meta.json`
6. THE command SHALL write `composite_summary_of_all_gems.md` into the gem's knowledge directory as an additional subfile
7. THE command SHALL index the gem for search via `SearchProvider::index_gem(gem_id)`
8. THE command SHALL return the created `Gem`
9. MULTIPLE saves for the same project SHALL create separate gems (different timestamps in the URL) — each is a distinct checkpoint

---

## Requirement 5: Summary Chat Component (Separate from Research Chat)

**User Story:** As a user viewing a project, I want a dedicated Summary tab where I can generate a summary, review it, ask questions about it, and save it — separate from the Research tab which is for discovery.

### Acceptance Criteria

1. THE System SHALL create a `ProjectSummaryChat` component in `src/components/ProjectSummaryChat.tsx`
2. THE component SHALL accept props: `projectId: string`, `projectTitle: string`, `onGemSaved?: () => void`
3. THE component SHALL have four states:
   a. **Empty** — No summary generated. Shows a "Generate Summary" button and brief description
   b. **Generating** — Progress indicator shown while LLM is working (e.g., "Analyzing {n} gems...")
   c. **Review** — Summary displayed as formatted markdown, with a "Save as Gem" button and a chat input for questions
   d. **Saved** — Summary has been saved. Shows confirmation (e.g., "Saved as gem"). Chat input remains for questions
4. THE "Generate Summary" button SHALL call `invoke('generate_project_summary_checkpoint', { projectId })`
5. THE summary SHALL be rendered as formatted markdown in a scrollable container
6. THE "Save as Gem" button SHALL call `invoke('save_project_summary_checkpoint', { projectId, summaryContent, compositeDoc })` and transition to the Saved state
7. THE chat input SHALL allow the user to ask questions about the summary. Questions SHALL be answered using the composite document + summary as context via `IntelProvider::chat`
8. A "Regenerate" button SHALL be available in the Review and Saved states to re-run the generation pipeline
9. WHEN the `projectId` prop changes, THE component SHALL reset to the Empty state
10. THE component SHALL call `onGemSaved` callback after successfully saving a summary gem

---

## Requirement 6: RightPanel Integration — Summary Tab

**User Story:** As a user viewing a project, I want a "Summary" tab in the right panel alongside "Research" and "Detail", so I can switch between discovering resources, distilling what I have, and viewing gem details.

### Acceptance Criteria

1. WHEN `activeNav === 'projects'` and a project is selected, THE RightPanel SHALL show three tabs: **Research**, **Summary**, **Detail**
2. THE "Research" tab SHALL render the existing `ProjectResearchChat` component
3. THE "Summary" tab SHALL render the new `ProjectSummaryChat` component
4. THE "Detail" tab SHALL render the existing `GemDetailPanel` (only enabled/visible when a gem is selected)
5. WHEN no gem is selected, THE RightPanel SHALL show only "Research" and "Summary" tabs
6. THE "Research" tab SHALL remain the default active tab when a project is first selected
7. EACH tab SHALL maintain its own state independently — switching between tabs SHALL NOT reset the other tab's state
8. THE `ProjectSummaryChat` component SHALL receive `projectId`, `projectTitle`, and an `onGemSaved` callback that refreshes the project gem list
9. SWITCHING projects SHALL reset both the Research and Summary tabs (via `key={projectId}` on both components)

---

## Requirement 7: Summary Chat CSS Styling

**User Story:** As a user, I want the Summary Chat to look consistent with the rest of Jarvis, following the dark theme and fitting naturally alongside the Research Chat and gem detail panels.

### Acceptance Criteria

1. THE Summary Chat layout SHALL be a flex column filling the available height
2. THE empty state SHALL be centered with the "Generate Summary" button prominently displayed
3. THE generating state SHALL show a centered spinner with descriptive text (e.g., "Analyzing 12 gems...")
4. THE summary preview SHALL be displayed in a scrollable container with markdown formatting, using monospace font for code blocks
5. THE "Save as Gem" button SHALL use accent color styling consistent with other action buttons
6. THE "Regenerate" button SHALL use secondary/outline styling
7. THE chat input area SHALL match the existing Research Chat input styling
8. THE saved confirmation SHALL be displayed as a subtle success indicator (not a modal)
9. ALL new CSS SHALL be added to `App.css` without modifying existing styles

---

## Requirement 8: `composite_summary_of_all_gems.md` as Knowledge Subfile

**User Story:** As the system, I need the composite source document preserved in the summary gem's knowledge directory, so it's searchable, provides provenance for the summary, and serves as context for Q&A.

### Acceptance Criteria

1. WHEN a summary gem is saved, THE System SHALL write `composite_summary_of_all_gems.md` into the gem's knowledge directory at `knowledge/{gem_id}/composite_summary_of_all_gems.md`
2. THE file SHALL contain the full composite document built in Requirement 1 — all project gems' knowledge files concatenated chronologically
3. THE file SHALL be listed in the knowledge viewer's file tree alongside `content.md`, `enrichment.md`, and `gem.md`
4. THE file SHALL be searchable via FTS/semantic search (indexed as part of the gem's knowledge files)
5. THE file SHALL be readable via the existing `get_gem_knowledge_subfile` Tauri command
6. THE `gem.md` assembler SHALL include a reference to `composite_summary_of_all_gems.md` in the assembled document (e.g., a "Source Material" section with a note that the composite file is available)

---

## Technical Constraints

1. **Existing LLM infrastructure.** All LLM calls use `IntelProvider::chat`. No new AI integration.
2. **Existing knowledge store.** `composite_summary_of_all_gems.md` is written via `KnowledgeStore` APIs. The assembler recognizes it as an additional subfile.
3. **Existing gem store.** Summary gems use the standard `GemStore::save` upsert path. The `source_url` with timestamp ensures uniqueness.
4. **Chunk size conservative.** ~4000 tokens per chunk is safe for Qwen3-8B. Can be made configurable later.
5. **Sequential chunk processing.** Chunks are summarized one at a time through `IntelProvider::chat`. The existing `IntelQueue` serializes LLM access anyway.
6. **No streaming.** Summary generation returns the complete result. Streaming partial results is a future enhancement.
7. **Agent extension.** New methods are added to the existing `ProjectResearchAgent` — no new agent struct.

## Out of Scope

1. **Diff between summary checkpoints** — comparing two summaries to show what changed.
2. **Streaming/progressive summary display** — showing partial results as chunks complete.
3. **Auto-scheduled summaries** — generating summaries on a timer or when new gems are added.
4. **Summary as chat context for Research Chat** — using the latest summary to improve research topic suggestions.
5. **Export to PDF** — summary gems can be viewed as markdown; PDF export is a separate feature.
6. **Configurable chunk size** — hardcoded to ~4000 tokens for v1.
7. **Summary of summaries** — meta-summaries across multiple projects.
8. **Custom summary prompts** — the summarization prompt is hardcoded. User-customizable prompts are a future enhancement.
