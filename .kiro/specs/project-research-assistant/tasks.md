# Project Research Assistant — Implementation Tasks

## Phase 1: Search Infrastructure — Trait Extension, Tavily, Composite (Requirements 1, 2, 3)

### Task 1: Extend `SearchResultProvider` with Web Search Types and Methods

- [ ] 1.1 Add `WebSourceType` enum to `src/search/provider.rs` with variants: `Paper`, `Article`, `Video`, `Other` — derive `Debug`, `Clone`, `Serialize`, `Deserialize`
- [ ] 1.2 Add `WebSearchResult` struct to `src/search/provider.rs` with fields: `title` (String), `url` (String), `snippet` (String), `source_type` (WebSourceType), `domain` (String), `published_date` (Option<String>) — derive `Debug`, `Clone`, `Serialize`, `Deserialize`
- [ ] 1.3 Add `web_search` default method to `SearchResultProvider` trait: `async fn web_search(&self, _query: &str, _limit: usize) -> Result<Vec<WebSearchResult>, String>` returning `Ok(Vec::new())`
- [ ] 1.4 Add `supports_web_search` default method to `SearchResultProvider` trait: `fn supports_web_search(&self) -> bool` returning `false`
- [ ] 1.5 Verify `FtsResultProvider` and `QmdResultProvider` compile without changes (they inherit the defaults)
- [ ] 1.6 Re-export `WebSearchResult` and `WebSourceType` from `src/search/mod.rs`

### Task 2: Create `TavilyProvider` — Web Search via Tavily API

- [ ] 2.1 Create `src/search/tavily_provider.rs` with `TavilyProvider` struct holding `api_key: String` and `client: reqwest::Client`
- [ ] 2.2 Implement `TavilyProvider::new(api_key: String)` constructor with `eprintln!` log
- [ ] 2.3 Define private Tavily API request/response structs: `TavilySearchRequest`, `TavilySearchResponse`, `TavilyResult`
- [ ] 2.4 Implement `classify_source_type(url: &str) -> WebSourceType` helper — classify by domain (youtube→Video, arxiv→Paper, medium→Article, else→Other)
- [ ] 2.5 Implement `extract_domain(url: &str) -> String` helper — strip protocol, www prefix, path
- [ ] 2.6 Implement `SearchResultProvider` for `TavilyProvider`:
  - `check_availability` → true if API key non-empty
  - `search`, `index_gem`, `remove_gem`, `reindex_all` → no-ops
  - `web_search` → POST to `https://api.tavily.com/search`, parse response, classify source types, extract domains
  - `supports_web_search` → true
- [ ] 2.7 Add `pub mod tavily_provider;` to `src/search/mod.rs` and re-export `TavilyProvider`

### Task 3: Create `CompositeSearchProvider` — Delegating Wrapper

- [ ] 3.1 Create `src/search/composite_provider.rs` with `CompositeSearchProvider` struct holding `gem_provider: Arc<dyn SearchResultProvider>` and `web_provider: Option<Arc<dyn SearchResultProvider>>`
- [ ] 3.2 Implement `CompositeSearchProvider::new()` constructor with `eprintln!` log
- [ ] 3.3 Implement `SearchResultProvider` for `CompositeSearchProvider`:
  - `check_availability`, `search`, `index_gem`, `remove_gem`, `reindex_all` → delegate to `gem_provider`
  - `web_search` → delegate to `web_provider` if present, else return empty
  - `supports_web_search` → true only if `web_provider` is present and supports it
- [ ] 3.4 Add `pub mod composite_provider;` to `src/search/mod.rs` and re-export `CompositeSearchProvider`

**Checkpoint**: `cargo check` passes. All search types and providers defined. Existing FTS/QMD providers unchanged.

---

## Phase 2: Provider Registration — lib.rs Wiring (Requirement 4)

### Task 4: Register CompositeSearchProvider in `lib.rs`

- [ ] 4.1 Read `settings.search.tavily_api_key` from `SettingsManager` during app setup (alongside existing `semantic_search_enabled` and `semantic_search_accuracy`)
- [ ] 4.2 Build optional `TavilyProvider` if API key is present and non-empty
- [ ] 4.3 Wrap existing gem provider (QMD or FTS) + optional Tavily provider in `CompositeSearchProvider`
- [ ] 4.4 Register `CompositeSearchProvider` as `Arc<dyn SearchResultProvider>` in Tauri state — replacing current direct gem provider registration
- [ ] 4.5 Verify existing commands still work: `search_gems`, `check_search_availability`, `rebuild_search_index` — all delegate through composite transparently

**Checkpoint**: `cargo build` succeeds. App starts with CompositeSearchProvider. Existing search unchanged. Tavily enabled when API key present.

---

## Phase 3: Agent Backend — ProjectChatSource and ProjectResearchAgent (Requirements 5, 6)

### Task 5: Create `ProjectChatSource` — Chatable for Projects

- [ ] 5.1 Create `src/agents/project_chat.rs` with `ProjectChatSource` struct holding `project_id`, `project_title`, `project_store: Arc<dyn ProjectStore>`, `gem_store: Arc<dyn GemStore>`
- [ ] 5.2 Implement `ProjectChatSource::new()` constructor
- [ ] 5.3 Implement `Chatable` trait:
  - `get_context()` → load project + gems, assemble context string (project metadata + gem titles/descriptions/summaries from `ai_enrichment`)
  - `label()` → `"Project: {project_title}"`
  - `session_dir()` → `{app_data}/projects/{project_id}/chat_sessions/`
  - `needs_preparation()` → `false` (context assembled on the fly)
- [ ] 5.4 Add `pub mod project_chat;` to `src/agents/mod.rs`

### Task 6: Create `ProjectResearchAgent` — Research Action

- [ ] 6.1 Create `src/agents/project_agent.rs` with `ProjectResearchAgent` struct holding `project_store`, `gem_store`, `intel_provider`, `search_provider`, `intel_queue`, `chatbot: Chatbot`, `chat_sources: HashMap<String, ProjectChatSource>`
- [ ] 6.2 Define `ProjectResearchResults` struct: `web_results: Vec<WebSearchResult>`, `suggested_gems: Vec<GemSearchResult>`, `topics_generated: Vec<String>` — derive `Debug`, `Clone`, `Serialize`, `Deserialize`
- [ ] 6.3 Define `TOPIC_GENERATION_PROMPT` constant — instructs LLM to return JSON array of 3-5 search queries
- [ ] 6.4 Implement `ProjectResearchAgent::new()` constructor
- [ ] 6.5 Implement `research(&self, project_id: &str) -> Result<ProjectResearchResults, String>`:
  - Load project via `project_store.get()`
  - Build context from title/description/objective
  - Generate topics via `intel_provider.chat()` with `TOPIC_GENERATION_PROMPT`
  - Parse JSON array response (strip markdown code fences)
  - For each topic: `search_provider.web_search(topic, 5)` if `supports_web_search()` — log errors, continue on failure
  - Deduplicate web results by URL
  - `search_provider.search(project.title, 20)` for gem suggestions
  - Enrich gem results with `GemPreview` data (same pattern as `search_gems` command)
  - Return `ProjectResearchResults`

### Task 7: Add Summarize and Chat Actions to ProjectResearchAgent

- [ ] 7.1 Define `SUMMARIZE_PROMPT` constant — instructs LLM to write executive summary covering goals, themes, findings, gaps
- [ ] 7.2 Implement `summarize(&self, project_id: &str) -> Result<String, String>`:
  - Load project + gems
  - Return early with friendly message if 0 gems
  - Assemble context from all gem content (titles, descriptions, summaries, tags from `ai_enrichment`)
  - Call `intel_provider.chat()` with `SUMMARIZE_PROMPT`
  - Return summary string
- [ ] 7.3 Implement `start_chat(&mut self, project_id: &str) -> Result<String, String>`:
  - Load project to get title
  - Create `ProjectChatSource`
  - Call `chatbot.start_session(&source)` to get `session_id`
  - Store source in `chat_sources` HashMap
  - Return `session_id`
- [ ] 7.4 Implement `send_chat_message(&mut self, session_id: &str, message: &str) -> Result<String, String>`:
  - Look up `ProjectChatSource` in `chat_sources`
  - Delegate to `chatbot.send_message(session_id, message, source, &self.intel_queue)`
- [ ] 7.5 Implement `get_chat_history(&self, session_id: &str) -> Result<Vec<ChatMessage>, String>`:
  - Delegate to `chatbot.get_history(session_id)`
- [ ] 7.6 Implement `end_chat(&mut self, session_id: &str)`:
  - Delegate to `chatbot.end_session(session_id)`
  - Remove source from `chat_sources`
- [ ] 7.7 Add `pub mod project_agent;` to `src/agents/mod.rs`

**Checkpoint**: `cargo check` passes. Full agent with research, summarize, and chat methods compiles. No Tauri commands yet.

---

## Phase 4: Agent Tauri Commands and Registration (Requirement 7)

### Task 8: Add Agent Tauri Commands to `projects/commands.rs`

- [ ] 8.1 Add imports: `TokioMutex`, `ProjectResearchAgent`, `ProjectResearchResults`, `ChatMessage`
- [ ] 8.2 Implement `get_project_research` command: accepts `project_id`, locks agent mutex, delegates to `agent.research()`
- [ ] 8.3 Implement `get_project_summary` command: accepts `project_id`, locks agent mutex, delegates to `agent.summarize()`
- [ ] 8.4 Implement `start_project_chat` command: accepts `project_id`, locks agent (mutable), delegates to `agent.start_chat()`
- [ ] 8.5 Implement `send_project_chat_message` command: accepts `session_id`, `message`, locks agent (mutable), delegates to `agent.send_chat_message()`
- [ ] 8.6 Implement `get_project_chat_history` command: accepts `session_id`, locks agent, delegates to `agent.get_chat_history()`
- [ ] 8.7 Implement `end_project_chat` command: accepts `session_id`, locks agent (mutable), delegates to `agent.end_chat()`

### Task 9: Register Agent and Commands in `lib.rs`

- [ ] 9.1 Create `ProjectResearchAgent::new()` with all provider Arcs: `project_store_arc`, `gem_store_arc`, `intel_provider_arc`, `search_provider` (composite), `intel_queue_arc`
- [ ] 9.2 Register agent in Tauri state: `app.manage(Arc::new(tokio::sync::Mutex::new(project_agent)))`
- [ ] 9.3 Add all 6 agent commands to `generate_handler![]`:
  - `projects::commands::get_project_research`
  - `projects::commands::get_project_summary`
  - `projects::commands::start_project_chat`
  - `projects::commands::send_project_chat_message`
  - `projects::commands::get_project_chat_history`
  - `projects::commands::end_project_chat`

**Checkpoint**: `cargo build` succeeds. All 6 agent commands registered. Backend fully functional: research, summarize, and chat all invocable.

---

## Phase 5: Frontend — TypeScript Types and ProjectResearchPanel (Requirements 8, 9)

### Task 10: Add TypeScript Types

- [ ] 10.1 Add `WebSearchResult` interface to `src/state/types.ts`: `title`, `url`, `snippet`, `source_type` (union: `'Paper' | 'Article' | 'Video' | 'Other'`), `domain`, `published_date` (string | null)
- [ ] 10.2 Add `ProjectResearchResults` interface: `web_results: WebSearchResult[]`, `suggested_gems: GemSearchResult[]`, `topics_generated: string[]`
- [ ] 10.3 Export all new types from `types.ts`

### Task 11: Create `ProjectResearchPanel` Component

- [ ] 11.1 Create `ProjectResearchPanel` component with props: `projectId: string`, `onGemsAdded: () => void`
- [ ] 11.2 Implement loading state: `invoke<ProjectResearchResults>('get_project_research', { projectId })` on mount
- [ ] 11.3 Render loading spinner with "Researching your project..." text and subtext
- [ ] 11.4 Render error state: "Research failed: {error}. You can still add gems manually."
- [ ] 11.5 Render "Suggested from the web" section:
  - Web result cards with source type badge, domain, title, snippet (truncated 3 lines)
  - Click opens URL via `shell.open`
  - Empty state: "No web resources found" or "Web search not configured"
- [ ] 11.6 Render "From your gem library" section:
  - Gem cards with "Add to Project" button
  - Click "Add" → `invoke('add_gems_to_project', { projectId, gemIds: [gemId] })` → update to "Added" state
  - Empty state: "No matching gems in your library yet"
- [ ] 11.7 Track added gem IDs in `Set<string>` to show "Added" state

**Checkpoint**: Frontend builds. Research panel renders with web results and gem suggestions.

---

## Phase 6: Frontend — Research & Summarize Buttons in ProjectGemList (Requirement 10)

### Task 12: Add Research Button to Project Toolbar

- [ ] 12.1 Add `showResearch: boolean` state to `ProjectGemList`
- [ ] 12.2 Add "Research" button to toolbar alongside existing search input and "+ Add Gems"
- [ ] 12.3 Auto-trigger research on new project creation (0 gems detected)
- [ ] 12.4 Disable "Research" button while research is loading
- [ ] 12.5 Render `ProjectResearchPanel` inline above gem list when `showResearch` is true
- [ ] 12.6 Wire `onGemsAdded` to refresh project detail and project list

### Task 13: Add Summarize Button to Project Toolbar

- [ ] 13.1 Add `projectSummary: string | null` and `summarizing: boolean` state
- [ ] 13.2 Add "Summarize" button to toolbar
- [ ] 13.3 On click: `invoke<string>('get_project_summary', { projectId })` → set summary state
- [ ] 13.4 Disable button and show "Summarizing..." while loading
- [ ] 13.5 Render summary in dismissible panel above gem list (with "Dismiss" button)
- [ ] 13.6 Handle error state inline

**Checkpoint**: Frontend builds. Research and Summarize buttons work. Results display inline above gem list.

---

## Phase 7: CSS Styling (Requirement 11)

### Task 14: Add CSS for Research Panel

- [ ] 14.1 Add `.research-panel` styles: padding, border-bottom
- [ ] 14.2 Add `.research-loading` styles: centered flex column, spinner animation (`@keyframes spin`)
- [ ] 14.3 Add `.research-spinner` styles: border-based spinner with accent color
- [ ] 14.4 Add `.research-error` styles: error color, padding
- [ ] 14.5 Add `.research-section` and `.research-section-title` styles: uppercase label, muted color
- [ ] 14.6 Add `.research-empty` styles: muted, italic

### Task 15: Add CSS for Web Result and Gem Suggestion Cards

- [ ] 15.1 Add `.web-result-card` styles: padding, border, border-radius, cursor pointer, hover effect with accent border
- [ ] 15.2 Add `.source-type-badge` base styles and color variants: `.source-paper` (purple), `.source-article` (blue), `.source-video` (red), `.source-other` (gray)
- [ ] 15.3 Add `.web-result-domain`, `.web-result-title`, `.web-result-snippet` styles (snippet with `-webkit-line-clamp: 3`)
- [ ] 15.4 Add `.research-gem-card` styles: flex layout with gem card + add button
- [ ] 15.5 Add `.research-add-gem` styles: outline button with accent color, `.added` state with muted color

### Task 16: Add CSS for Project Summary Panel

- [ ] 16.1 Add `.project-summary-panel` styles: padding, border, border-radius, card background
- [ ] 16.2 Add `.summary-header` styles: flex with justify-content space-between, dismiss button
- [ ] 16.3 Add `.summary-content` styles: font-size, line-height, pre-wrap for markdown

**Checkpoint**: All UI styled consistently with dark theme. Research panel, web cards, gem cards, summary panel all look polished.

---

## Phase 8: Testing and Polish

### Task 17: Verify search infrastructure

- [ ] 17.1 Test `classify_source_type`: youtube.com → Video, arxiv.org → Paper, medium.com → Article, random.com → Other
- [ ] 17.2 Test `extract_domain`: strips protocol, www prefix, path correctly
- [ ] 17.3 Test `CompositeSearchProvider`: `.search()` delegates to gem_provider, `.web_search()` delegates to web_provider or returns empty
- [ ] 17.4 Test `supports_web_search`: false when no web_provider, true when present
- [ ] 17.5 Verify existing `search_gems` command still works through CompositeSearchProvider
- [ ] 17.6 Verify existing `rebuild_search_index` still works through CompositeSearchProvider

### Task 18: Verify agent actions

- [ ] 18.1 Test `research()`: returns web results + gem suggestions when Tavily key present
- [ ] 18.2 Test `research()` graceful degradation: returns gem suggestions only when Tavily key absent
- [ ] 18.3 Test `research()` topic parsing: handles markdown code fences around JSON array
- [ ] 18.4 Test `research()` resilience: individual web search failure doesn't fail entire command
- [ ] 18.5 Test `summarize()`: returns summary for project with gems
- [ ] 18.6 Test `summarize()`: returns friendly message for project with 0 gems
- [ ] 18.7 Test `start_chat()` + `send_chat_message()` + `end_chat()` lifecycle
- [ ] 18.8 Test `get_chat_history()` returns messages from session

### Task 19: End-to-end verification

- [ ] 19.1 Create project → verify research panel auto-shows with spinner
- [ ] 19.2 Verify LLM generates reasonable topics for the project title
- [ ] 19.3 Verify web results show with source type badges (Paper, Article, Video, Other)
- [ ] 19.4 Click web result → opens in system browser
- [ ] 19.5 Verify gem suggestions appear with "Add" buttons
- [ ] 19.6 Click "Add" on a gem → button changes to "Added", gem appears in project
- [ ] 19.7 Click "Research" button on existing project → research panel shows
- [ ] 19.8 Click "Summarize" button on project with gems → summary panel appears
- [ ] 19.9 Click "Summarize" on empty project → shows "no gems yet" message
- [ ] 19.10 Remove Tavily API key from settings → restart → web section shows "Web search not configured"
- [ ] 19.11 Existing gem search (GemsPanel search bar) still works unchanged
- [ ] 19.12 App builds and starts without errors
