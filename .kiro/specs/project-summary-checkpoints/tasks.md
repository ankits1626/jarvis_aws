# Project Summary Checkpoints — Implementation Tasks

## Phase 1: Agent Struct Change + Composite Document Builder (Requirements 1, 2)

### Task 1: Add `knowledge_store` to `ProjectResearchAgent`

- [x] 1.1 Add `knowledge_store: Arc<dyn KnowledgeStore>` field to `ProjectResearchAgent` struct in `src/agents/project_agent.rs`
- [x] 1.2 Add `use crate::knowledge::KnowledgeStore;` import to `project_agent.rs`
- [x] 1.3 Update `ProjectResearchAgent::new()` to accept `knowledge_store: Arc<dyn KnowledgeStore>` parameter and store it
- [x] 1.4 Update `lib.rs` to pass `knowledge_store_arc.clone()` when constructing `ProjectResearchAgent::new()` — clone before `app.manage` consumes it
- [x] 1.5 Verify `cargo check` passes — existing methods unchanged, just a new field

### Task 2: Define `ProjectSummaryResult` and LLM Prompts

- [x] 2.1 Add `ProjectSummaryResult` struct to `src/agents/project_agent.rs`: `summary: String`, `composite_doc: String`, `gems_analyzed: usize`, `chunks_used: usize` — derive `Debug`, `Clone`, `Serialize`, `Deserialize`
- [x] 2.2 Define `CHECKPOINT_SUMMARY_PROMPT` constant — instructs LLM to generate comprehensive summary grouped by date with 3-5 key points per resource and a synthesis section
- [x] 2.3 Define `CHUNK_SUMMARY_PROMPT` constant — instructs LLM to extract key points from a section of resources
- [x] 2.4 Define `MERGE_SUMMARY_PROMPT` constant — instructs LLM to combine chunk summaries into one cohesive document preserving chronological order
- [x] 2.5 Define `SUMMARY_QA_PROMPT` constant — instructs LLM to answer questions grounded in the summary and source material

### Task 3: Implement `build_composite_document`

- [x] 3.1 Add private async method `build_composite_document(&self, project_id: &str) -> Result<(String, usize), String>` that returns `(composite_doc, gems_analyzed)`
- [x] 3.2 Load project via `self.project_store.get(project_id)` — return error if 0 gems
- [x] 3.3 Sort `detail.gems` by `captured_at` ascending (chronological)
- [x] 3.4 Build composite header: project title, objective (if set), gem count, date range (first and last `captured_at`)
- [x] 3.5 For each gem: call `self.knowledge_store.get_assembled(&gem.id)` to get `gem.md` content
- [x] 3.6 If `get_assembled` returns `None`: fall back to `self.gem_store.get(&gem.id)` and assemble from DB fields (title + description + content preview + ai_enrichment summary + transcript)
- [x] 3.7 If gem has no content at all: skip gem, add note in composite doc ("Gem X had no content to analyze")
- [x] 3.8 Wrap each gem's content with separator header: `========` line, gem number, title, source_type, domain, captured_at, `========` line
- [x] 3.9 Concatenate all gem sections with the project header into one markdown string
- [x] 3.10 Log via `eprintln!("Projects/Summary: Built composite document: {} chars, {} gems", ...)`

### Task 4: Implement `chunk_by_gem_boundaries`

- [x] 4.1 Add private method `chunk_by_gem_boundaries(gem_sections: Vec<String>, max_chars: usize) -> Vec<String>` — takes individual gem sections, groups them into chunks
- [x] 4.2 Walk through gem sections in order — accumulate into current chunk until adding the next section would exceed `max_chars` (default: 16000 chars ≈ 4000 tokens)
- [x] 4.3 If current chunk is non-empty and adding next gem exceeds limit: push current chunk, start new chunk with the next gem
- [x] 4.4 If a single gem section exceeds `max_chars`: truncate its content to fit, preserving the separator header and a truncation note
- [x] 4.5 Push final chunk if non-empty
- [x] 4.6 Log via `eprintln!("Projects/Summary: Split into {} chunks", chunks.len())`

**Checkpoint**: `cargo check` passes. Composite builder and chunker compile. No Tauri commands yet.

---

## Phase 2: Summary Generation + Q&A Methods (Requirements 2, 3)

### Task 5: Implement `generate_summary_checkpoint`

- [x] 5.1 Add public async method `generate_summary_checkpoint(&self, project_id: &str) -> Result<ProjectSummaryResult, String>`
- [x] 5.2 Call `self.build_composite_document(project_id)` — returns `(composite_doc, gems_analyzed)`
- [x] 5.3 Split composite doc into per-gem sections by splitting on the `========` separator pattern
- [x] 5.4 Call `self.chunk_by_gem_boundaries(gem_sections, 16000)` — returns `Vec<String>` of chunks
- [x] 5.5 If 1 chunk: single LLM call — `self.intel_provider.chat(&[("system", CHECKPOINT_SUMMARY_PROMPT), ("user", chunk)])` → final summary
- [x] 5.6 If N chunks: for each chunk, `self.intel_provider.chat(&[("system", CHUNK_SUMMARY_PROMPT), ("user", chunk)])` → chunk_summary. Collect all chunk summaries.
- [x] 5.7 If N chunks: merge pass — `self.intel_provider.chat(&[("system", MERGE_SUMMARY_PROMPT), ("user", all_chunk_summaries_joined)])` → final summary
- [x] 5.8 If LLM fails on a chunk: log error, skip that chunk, continue with remaining. Add note to final summary about partial failure.
- [x] 5.9 Return `ProjectSummaryResult { summary, composite_doc, gems_analyzed, chunks_used: chunks.len() }`
- [x] 5.10 Log via `eprintln!("Projects/Summary: Generated summary ({} chars) from {} gems in {} chunks", ...)`

### Task 6: Implement `send_summary_question`

- [x] 6.1 Add public async method `send_summary_question(&self, question: &str, summary: &str, composite_doc: &str) -> Result<String, String>`
- [x] 6.2 Truncate `composite_doc` to ~10000 chars if longer (preserve beginning, which has the oldest/most foundational gems)
- [x] 6.3 Build context string: `"## Summary\n{summary}\n\n## Source Material\n{truncated_composite_doc}"`
- [x] 6.4 Call `self.intel_provider.chat(&[("system", SUMMARY_QA_PROMPT), ("user", format!("{context}\n\nQuestion: {question}"))])`
- [x] 6.5 Return LLM response
- [x] 6.6 Log via `eprintln!("Projects/Summary: Answered question ({} chars)", ...)`

**Checkpoint**: `cargo check` passes. `generate_summary_checkpoint` and `send_summary_question` compile. No Tauri commands yet.

---

## Phase 3: Save as Gem + Knowledge Store Change (Requirements 4, 8)

### Task 7: Update `KNOWN_SUBFILES` in Knowledge Store

- [x] 7.1 Add `"composite_summary_of_all_gems.md"` to `KNOWN_SUBFILES` array in `src/knowledge/local_store.rs` — insert before `"gem.md"` (last position before the assembled output)
- [x] 7.2 Verify existing gems still load correctly — the file won't exist for non-summary gems, so it's skipped during `get()`
- [x] 7.3 Verify `update_subfile()` works for the new filename — it should write the file and trigger `gem.md` reassembly

### Task 8: Implement `save_summary_checkpoint`

- [x] 8.1 Add public async method `save_summary_checkpoint(&self, project_id: &str, summary_content: &str, composite_doc: &str) -> Result<Gem, String>`
- [x] 8.2 Load project via `self.project_store.get(project_id)` — get title and gem count
- [x] 8.3 Build `Gem` struct:
  - `id`: `uuid::Uuid::new_v4().to_string()`
  - `source_type`: `"ProjectSummary"`
  - `source_url`: `format!("jarvis://project/{}/summary/{}", project_id, SystemTime::now() unix secs)`
  - `title`: `format!("Summary: {} — {}", project_title, formatted_date)`
  - `content`: `Some(summary_content.to_string())`
  - `domain`: `"jarvis".to_string()`
  - `description`: `Some(format!("Summary of {} gems from {}", gem_count, project_title))`
  - `author`: `None`
  - `captured_at`: current ISO 8601 timestamp
  - `source_meta`: `serde_json::json!({})`
  - `ai_enrichment`: `None`
  - `transcript`: `None`
  - `transcript_language`: `None`
- [x] 8.4 Save gem via `self.gem_store.save(&gem)`
- [x] 8.5 Add gem to project via `self.project_store.add_gems(project_id, &[gem.id.clone()])`
- [x] 8.6 Generate knowledge files via `self.knowledge_store.create(&gem)` — produces `content.md`, `enrichment.md`, `gem.md`, `meta.json`
- [x] 8.7 Write composite file via `self.knowledge_store.update_subfile(&gem.id, "composite_summary_of_all_gems.md", composite_doc)`
- [x] 8.8 Index for search via `self.search_provider.index_gem(&gem.id)` (fire-and-forget, match existing pattern)
- [x] 8.9 Log via `eprintln!("Projects/Summary: Saved summary gem {} for project {}", gem.id, project_id)`
- [x] 8.10 Return the created `Gem`

**Checkpoint**: `cargo check` passes. Full backend: generate, save, and Q&A methods compile. Knowledge store recognizes composite file.

---

## Phase 4: Tauri Commands + Registration (Requirements 3, 4)

### Task 9: Add Summary Tauri Commands to `projects/commands.rs`

- [x] 9.1 Add import for `ProjectSummaryResult` from `crate::agents::project_agent`
- [x] 9.2 Add import for `Gem` from `crate::gems::store` (if not already imported)
- [x] 9.3 Implement `generate_project_summary_checkpoint` command: accepts `project_id: String`, locks agent, delegates to `agent.generate_summary_checkpoint()` → returns `Result<ProjectSummaryResult, String>`
- [x] 9.4 Implement `save_project_summary_checkpoint` command: accepts `project_id: String`, `summary_content: String`, `composite_doc: String`, locks agent, delegates to `agent.save_summary_checkpoint()` → returns `Result<Gem, String>`
- [x] 9.5 Implement `send_summary_question` command: accepts `question: String`, `summary: String`, `composite_doc: String`, locks agent, delegates to `agent.send_summary_question()` → returns `Result<String, String>`

### Task 10: Register Commands in `lib.rs`

- [x] 10.1 Add 3 new commands to `generate_handler![]` in `lib.rs`:
  - `projects::commands::generate_project_summary_checkpoint`
  - `projects::commands::save_project_summary_checkpoint`
  - `projects::commands::send_summary_question`
- [x] 10.2 Verify no other `lib.rs` changes needed — agent already registered, knowledge_store already passed (from Task 1.4)

**Checkpoint**: `cargo build` succeeds. All 3 summary commands registered and invocable from frontend. Backend fully functional.

---

## Phase 5: Frontend — TypeScript Types + ProjectSummaryChat Component (Requirements 5, 7)

### Task 11: Add TypeScript Types

- [x] 11.1 Add `ProjectSummaryResult` interface to `src/state/types.ts`: `summary: string`, `composite_doc: string`, `gems_analyzed: number`, `chunks_used: number`
- [x] 11.2 Export `ProjectSummaryResult` from `types.ts`

### Task 12: Create `ProjectSummaryChat` Component

- [x] 12.1 Create `src/components/ProjectSummaryChat.tsx` with props: `projectId: string`, `projectTitle: string`, `onGemSaved?: () => void`
- [x] 12.2 Implement state: `state: SummaryState` (`'empty' | 'generating' | 'review' | 'saved'`), `summaryResult: ProjectSummaryResult | null`, `saved: boolean`, `chatMessages: {role: string, content: string}[]`, `input: string`, `loading: boolean`, `error: string | null`
- [x] 12.3 Implement `handleGenerate`: sets state to `'generating'`, calls `invoke('generate_project_summary_checkpoint', { projectId })`, on success sets `summaryResult` and state to `'review'`, on error sets state to `'empty'` with error message
- [x] 12.4 Implement `handleSave`: calls `invoke('save_project_summary_checkpoint', { projectId, summaryContent, compositeDoc })`, on success sets `saved: true`, state to `'saved'`, calls `onGemSaved?.()`
- [x] 12.5 Implement `handleAskQuestion`: appends user message to `chatMessages`, calls `invoke('send_summary_question', { question, summary, compositeDoc })`, appends assistant response
- [x] 12.6 Implement `handleKeyPress`: Enter key triggers `handleAskQuestion`
- [x] 12.7 Render **empty state**: centered container with description text and "Generate Summary" button
- [x] 12.8 Render **generating state**: centered spinner with "Analyzing {n} gems..." text
- [x] 12.9 Render **review/saved state**: scrollable summary preview (`<pre>` with `summary-content` class), meta line (gems analyzed, chunks used), action buttons (Save as Gem / Saved badge + Regenerate), Q&A section with chat messages and input
- [x] 12.10 Render error state: error message with "Try Again" option in empty state
- [x] 12.11 Auto-scroll Q&A messages via `messagesEndRef`
- [x] 12.12 Reset state when `projectId` changes (via `useEffect` with `projectId` dep, or handled by `key={projectId}` in parent)

### Task 13: Add Summary Chat CSS

- [x] 13.1 Add `.summary-chat`: flex column, height 100%
- [x] 13.2 Add `.summary-chat-empty`: centered flex column with gap, padding, text-align center
- [x] 13.3 Add `.summary-chat-empty p`: secondary color, 13px, max-width 280px
- [x] 13.4 Add `.summary-chat-generating`: centered flex column, spinner + muted text
- [x] 13.5 Add `.summary-preview`: flex 1, overflow-y auto, padding 16px
- [x] 13.6 Add `.summary-content`: monospace font, 12px, line-height 1.6, pre-wrap
- [x] 13.7 Add `.summary-meta`: 11px, muted color, border-top, margin-top
- [x] 13.8 Add `.summary-actions`: flex row, gap 8px, padding, border-top
- [x] 13.9 Add `.summary-saved-badge`: 12px, success color (green)
- [x] 13.10 Add `.summary-qa`: border-top, padding, max-height 200px, overflow-y auto
- [x] 13.11 Add `.summary-qa .chat-message`: reuse existing chat message styling where possible
- [x] 13.12 Verify dark theme consistency — uses existing design tokens throughout

**Checkpoint**: Frontend builds. `ProjectSummaryChat` renders all four states. Styled consistently with dark theme.

---

## Phase 6: RightPanel Integration — Summary Tab (Requirement 6)

### Task 14: Add Summary Tab to RightPanel

- [x] 14.1 Import `ProjectSummaryChat` in `RightPanel.tsx`
- [x] 14.2 In the `activeNav === 'projects'` block (project selected case): add "Summary" tab button between "Research" and "Detail" — `activeTab === 'summary'`
- [x] 14.3 Add `activeTab === 'summary'` rendering branch: renders `<ProjectSummaryChat key={selectedProjectId} projectId={selectedProjectId} projectTitle={selectedProjectTitle} onGemSaved={onProjectGemsChanged} />`
- [x] 14.4 In the project-selected-no-gem case: show [Research] [Summary] tabs (two tabs, no Detail)
- [x] 14.5 In the project-selected-with-gem case: show [Research] [Summary] [Detail] tabs (three tabs + knowledge file tabs)
- [x] 14.6 Ensure `key={selectedProjectId}` on `ProjectSummaryChat` for remount on project change
- [x] 14.7 Ensure switching between Research, Summary, and Detail tabs does NOT reset the other tabs' state (each component maintains own state, only remounted on project change)
- [x] 14.8 Default active tab remains `'chat'` (Research) when a project is first selected

**Checkpoint**: Frontend builds. Opening a project shows [Research] [Summary] tabs. Clicking Summary shows empty state. Generate → Review → Save → Saved flow works. Switching tabs preserves state. Switching projects resets.

---

## Phase 7: Checkpoint Persistence — Load Latest on Mount (Requirement 9)

### Task 18: Backend — `get_latest_summary_checkpoint` Method

- [x] 18.1 Add public async method `get_latest_summary_checkpoint(&self, project_id: &str) -> Result<Option<ProjectSummaryResult>, String>` to `ProjectResearchAgent`
- [x] 18.2 Load project via `self.project_store.get(project_id)` — get `detail.gems`
- [x] 18.3 Filter gems where `source_type == "ProjectSummary"`, sort by `captured_at` descending, take the first (latest)
- [x] 18.4 If no summary gems found: return `Ok(None)`
- [x] 18.5 Load full gem via `self.gem_store.get(&gem_preview.id)` — get `content` field as the summary text
- [x] 18.6 Load composite doc via `self.knowledge_store.get_subfile(&gem.id, "composite_summary_of_all_gems.md")` — fall back to empty string if not found
- [x] 18.7 Calculate `gems_analyzed` by counting `"========"` separator pairs in composite doc (each gem has 2 separator lines)
- [x] 18.8 Return `Ok(Some(ProjectSummaryResult { summary, composite_doc, gems_analyzed, chunks_used: 0 }))`
- [x] 18.9 Log via `eprintln!("Projects/Summary: Loaded latest checkpoint for project {} ({} chars)", ...)`

### Task 19: Tauri Command — `get_latest_project_summary_checkpoint`

- [x] 19.1 Add `get_latest_project_summary_checkpoint` command to `projects/commands.rs`: accepts `project_id: String`, locks agent, delegates to `agent.get_latest_summary_checkpoint()` → returns `Result<Option<ProjectSummaryResult>, String>`
- [x] 19.2 Register in `generate_handler![]` in `lib.rs`

### Task 20: Frontend — Load Checkpoint on Mount

- [x] 20.1 In `ProjectSummaryChat.tsx`: add `initializing` state (`useState<boolean>(true)`)
- [x] 20.2 Add `useEffect` on mount (triggered by `projectId`): call `invoke('get_latest_project_summary_checkpoint', { projectId })`
- [x] 20.3 If result is not null: set `summaryResult`, set `state` to `'saved'`, set `saved` to `true`
- [x] 20.4 If result is null: keep `state` as `'empty'`
- [x] 20.5 Set `initializing` to `false` after fetch completes (success or error)
- [x] 20.6 While `initializing` is true: render a loading placeholder (spinner or brief "Loading..." text) instead of empty state
- [x] 20.7 When `chunks_used` is 0 (loaded checkpoint): hide "chunks" from meta line or show only gems analyzed

**Checkpoint**: Opening Summary tab loads the latest saved checkpoint. First visit shows empty state only if no checkpoint exists. Regenerate creates a new version. All saved versions remain accessible as gems.

---

## Phase 8: Testing and Verification

### Task 15: Backend Unit Tests

- [ ] 15.1 Test `build_composite_document` with 0 gems → returns error
- [ ] 15.2 Test `build_composite_document` with 3 gems → returns composite doc with all 3 gems in chronological order
- [ ] 15.3 Test `build_composite_document` fallback → gem without knowledge files uses DB fields
- [ ] 15.4 Test `chunk_by_gem_boundaries` with small gems → all in one chunk
- [ ] 15.5 Test `chunk_by_gem_boundaries` with large gems → splits into multiple chunks
- [ ] 15.6 Test `chunk_by_gem_boundaries` with single oversized gem → truncates
- [ ] 15.7 Test `generate_summary_checkpoint` single chunk path → one LLM call, no merge
- [ ] 15.8 Test `generate_summary_checkpoint` multi-chunk path → N+1 LLM calls (N chunks + 1 merge)
- [ ] 15.9 Test `save_summary_checkpoint` → gem created with correct fields, added to project
- [ ] 15.10 Test `save_summary_checkpoint` → knowledge files created + `composite_summary_of_all_gems.md` written
- [ ] 15.11 Test `send_summary_question` → returns LLM response using summary + composite context

### Task 16: Knowledge Store Tests

- [ ] 16.1 Test `KNOWN_SUBFILES` includes `composite_summary_of_all_gems.md`
- [ ] 16.2 Test `get()` for a non-summary gem → `composite_summary_of_all_gems.md` not in subfiles list (file doesn't exist)
- [ ] 16.3 Test `get()` for a summary gem with composite file → file listed in subfiles
- [ ] 16.4 Test `get_subfile()` for `composite_summary_of_all_gems.md` → returns content
- [ ] 16.5 Test `update_subfile()` for `composite_summary_of_all_gems.md` → writes file + reassembles `gem.md`

### Task 17: End-to-End Verification

- [ ] 17.1 Open project with gems → [Research] [Summary] tabs visible in RightPanel
- [ ] 17.2 Click Summary tab on project with no saved summary → empty state with "Generate Summary" button
- [ ] 17.2a Click Summary tab on project with saved summary → latest checkpoint loaded automatically
- [ ] 17.3 Click "Generate Summary" → spinner with "Analyzing N gems..." text
- [ ] 17.4 Summary appears as formatted markdown with key points per gem
- [ ] 17.5 Meta line shows correct gems analyzed count and chunks used
- [ ] 17.6 Type question in Q&A input → press Enter → answer appears in chat
- [ ] 17.7 Ask follow-up question → another answer appears (stateless, each is independent)
- [ ] 17.8 Click "Save as Gem" → button changes to "Saved" badge, gem appears in project gem list
- [ ] 17.9 Click the saved summary gem in gem list → Detail tab shows summary content
- [ ] 17.10 Open summary gem's knowledge files → `composite_summary_of_all_gems.md` listed in file tree
- [ ] 17.11 Click `composite_summary_of_all_gems.md` → full concatenated source document visible in viewer
- [ ] 17.12 Click "Regenerate" → new summary replaces old, Q&A chat clears, "Saved" resets to "Save as Gem"
- [ ] 17.13 Switch to different project → Summary tab loads that project's latest checkpoint (or empty if none)
- [ ] 17.14 Switch between Research and Summary tabs → each tab's state preserved
- [ ] 17.15 Project with 0 gems → "Generate Summary" shows friendly error message
- [ ] 17.16 Search for summary gem in GemsPanel search → found via FTS
- [ ] 17.17 Existing functionality unchanged — Research Chat, gem detail, knowledge viewer all work
- [ ] 17.18 App builds and starts without errors: `cargo build` + `npm run build`
