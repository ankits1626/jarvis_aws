# Project Research Assistant — Implementation Tasks

## Phase 1: Search Infrastructure — Trait Extension, Tavily, Composite (Requirements 1, 2, 3) ✅

### Task 1: Extend `SearchResultProvider` with Web Search Types and Methods

- [x] 1.1 Add `WebSourceType` enum to `src/search/provider.rs` with variants: `Paper`, `Article`, `Video`, `Other` — derive `Debug`, `Clone`, `Serialize`, `Deserialize`
- [x] 1.2 Add `WebSearchResult` struct to `src/search/provider.rs` with fields: `title` (String), `url` (String), `snippet` (String), `source_type` (WebSourceType), `domain` (String), `published_date` (Option<String>) — derive `Debug`, `Clone`, `Serialize`, `Deserialize`
- [x] 1.3 Add `web_search` default method to `SearchResultProvider` trait: `async fn web_search(&self, _query: &str, _limit: usize) -> Result<Vec<WebSearchResult>, String>` returning `Ok(Vec::new())`
- [x] 1.4 Add `supports_web_search` default method to `SearchResultProvider` trait: `fn supports_web_search(&self) -> bool` returning `false`
- [x] 1.5 Verify `FtsResultProvider` and `QmdResultProvider` compile without changes (they inherit the defaults)
- [x] 1.6 Re-export `WebSearchResult` and `WebSourceType` from `src/search/mod.rs`

### Task 2: Create `TavilyProvider` — Web Search via Tavily API

- [x] 2.1 Create `src/search/tavily_provider.rs` with `TavilyProvider` struct holding `api_key: String` and `client: reqwest::Client`
- [x] 2.2 Implement `TavilyProvider::new(api_key: String)` constructor with `eprintln!` log
- [x] 2.3 Define private Tavily API request/response structs: `TavilySearchRequest`, `TavilySearchResponse`, `TavilyResult`
- [x] 2.4 Implement `classify_source_type(url: &str) -> WebSourceType` helper — classify by domain (youtube→Video, arxiv→Paper, medium→Article, else→Other)
- [x] 2.5 Implement `extract_domain(url: &str) -> String` helper — strip protocol, www prefix, path
- [x] 2.6 Implement `SearchResultProvider` for `TavilyProvider`:
  - `check_availability` → true if API key non-empty
  - `search`, `index_gem`, `remove_gem`, `reindex_all` → no-ops
  - `web_search` → POST to `https://api.tavily.com/search`, parse response, classify source types, extract domains
  - `supports_web_search` → true
- [x] 2.7 Add `pub mod tavily_provider;` to `src/search/mod.rs` and re-export `TavilyProvider`

### Task 3: Create `CompositeSearchProvider` — Delegating Wrapper

- [x] 3.1 Create `src/search/composite_provider.rs` with `CompositeSearchProvider` struct holding `gem_provider: Arc<dyn SearchResultProvider>` and `web_provider: Option<Arc<dyn SearchResultProvider>>`
- [x] 3.2 Implement `CompositeSearchProvider::new()` constructor with `eprintln!` log
- [x] 3.3 Implement `SearchResultProvider` for `CompositeSearchProvider`:
  - `check_availability`, `search`, `index_gem`, `remove_gem`, `reindex_all` → delegate to `gem_provider`
  - `web_search` → delegate to `web_provider` if present, else return empty
  - `supports_web_search` → true only if `web_provider` is present and supports it
- [x] 3.4 Add `pub mod composite_provider;` to `src/search/mod.rs` and re-export `CompositeSearchProvider`

**Checkpoint**: ✅ `cargo check` passes. All search types and providers defined. Existing FTS/QMD providers unchanged.

---

## Phase 2: Provider Registration — lib.rs Wiring (Requirement 4) ✅

### Task 4: Register CompositeSearchProvider in `lib.rs`

- [x] 4.1 Read `settings.search.tavily_api_key` from `SettingsManager` during app setup (alongside existing `semantic_search_enabled` and `semantic_search_accuracy`)
- [x] 4.2 Build optional `TavilyProvider` if API key is present and non-empty
- [x] 4.3 Wrap existing gem provider (QMD or FTS) + optional Tavily provider in `CompositeSearchProvider`
- [x] 4.4 Register `CompositeSearchProvider` as `Arc<dyn SearchResultProvider>` in Tauri state — replacing current direct gem provider registration
- [x] 4.5 Verify existing commands still work: `search_gems`, `check_search_availability`, `rebuild_search_index` — all delegate through composite transparently

**Checkpoint**: ✅ `cargo build` succeeds. App starts with CompositeSearchProvider. Existing search unchanged. Tavily enabled when API key present.

---

## Phase 3: Agent Backend — ProjectChatSource and ProjectResearchAgent (Requirements 5, 6) ✅

### Task 5: Create `ProjectChatSource` — Chatable for Projects

- [x] 5.1 Create `src/agents/project_chat.rs` with `ProjectChatSource` struct holding `project_id`, `project_title`, `project_store: Arc<dyn ProjectStore>`, `gem_store: Arc<dyn GemStore>`
- [x] 5.2 Implement `ProjectChatSource::new()` constructor
- [x] 5.3 Implement `Chatable` trait:
  - `get_context()` → load project + gems, assemble context string (project metadata + gem titles/descriptions/summaries from `ai_enrichment`)
  - `label()` → `"Project: {project_title}"`
  - `session_dir()` → `{app_data}/projects/{project_id}/chat_sessions/`
  - `needs_preparation()` → `false` (context assembled on the fly)
- [x] 5.4 Add `pub mod project_chat;` to `src/agents/mod.rs`

### Task 6: Create `ProjectResearchAgent` — Two-Phase Research + Chat

- [x] 6.1 Create `src/agents/project_agent.rs` with `ProjectResearchAgent` struct holding `project_store`, `gem_store`, `intel_provider`, `search_provider`, `intel_queue: Arc<IntelQueue>`, `chatbot: Chatbot`, `chat_sources: HashMap<String, ProjectChatSource>`
- [x] 6.2 Define `ProjectResearchResults` struct: `web_results: Vec<WebSearchResult>`, `suggested_gems: Vec<GemSearchResult>`, `topics_searched: Vec<String>` — derive `Debug`, `Clone`, `Serialize`, `Deserialize`
- [x] 6.3 Define `TOPIC_GENERATION_PROMPT` constant — instructs LLM to return JSON array of 3-5 search queries
- [x] 6.4 Define `SUMMARIZE_PROMPT` constant — instructs LLM to write executive summary covering goals, themes, findings, gaps
- [x] 6.5 Implement `ProjectResearchAgent::new()` constructor
- [x] 6.6 Implement `suggest_topics(&self, project_id: &str) -> Result<Vec<String>, String>`:
  - Load project via `project_store.get()`
  - Build context from title/description/objective
  - Generate topics via `intel_provider.chat()` with `TOPIC_GENERATION_PROMPT`
  - Parse JSON array response (strip markdown code fences)
  - Return `Vec<String>` of 3-5 topics
- [x] 6.7 Implement `run_research(&self, project_id: &str, topics: Vec<String>) -> Result<ProjectResearchResults, String>`:
  - For each topic: `search_provider.web_search(topic, 5)` if `supports_web_search()` — log errors, continue on failure
  - Deduplicate web results by URL
  - `search_provider.search(project.title, 20)` for gem suggestions
  - Enrich gem results with full gem data (same pattern as `search_gems` command)
  - Return `ProjectResearchResults { web_results, suggested_gems, topics_searched: topics }`

### Task 7: Add Summarize and Chat to ProjectResearchAgent

- [x] 7.1 Implement `summarize(&self, project_id: &str) -> Result<String, String>`:
  - Load project + gems
  - Return early with friendly message if 0 gems
  - Assemble context from all gem content (titles, descriptions, summaries, tags from `ai_enrichment`)
  - Call `intel_provider.chat()` with `SUMMARIZE_PROMPT`
  - Return summary string
- [x] 7.2 Implement `start_chat(&mut self, project_id: &str) -> Result<String, String>`:
  - Load project to get title
  - Create `ProjectChatSource`
  - Call `chatbot.start_session(&source)` to get `session_id`
  - Store source in `chat_sources` HashMap
  - Return `session_id`
- [x] 7.3 Implement `send_chat_message(&mut self, session_id: &str, message: &str) -> Result<String, String>`:
  - Look up `ProjectChatSource` in `chat_sources`
  - Delegate to `chatbot.send_message(session_id, message, source, &self.intel_queue)`
- [x] 7.4 Implement `get_chat_history(&self, session_id: &str) -> Result<Vec<ChatMessage>, String>`
- [x] 7.5 Implement `end_chat(&mut self, session_id: &str)` — delegate to chatbot + remove source
- [x] 7.6 Add `pub mod project_agent;` to `src/agents/mod.rs`

**Checkpoint**: ✅ `cargo check` passes. Full agent with suggest_topics, run_research, summarize, and chat methods compiles. No Tauri commands yet.

---

## Phase 4: Tauri Commands and Agent Registration (Requirement 7) ✅

### Task 8: Add Agent Tauri Commands to `projects/commands.rs`

- [x] 8.1 Add imports: `TokioMutex`, `ProjectResearchAgent`, `ProjectResearchResults`, `ChatMessage`
- [x] 8.2 Implement `suggest_project_topics` command: accepts `project_id`, locks agent, delegates to `agent.suggest_topics()` → returns `Vec<String>`
- [x] 8.3 Implement `run_project_research` command: accepts `project_id` + `topics: Vec<String>`, locks agent, delegates to `agent.run_research()` → returns `ProjectResearchResults`
- [x] 8.4 Implement `get_project_summary` command: accepts `project_id`, locks agent, delegates to `agent.summarize()`
- [x] 8.5 Implement `start_project_chat` command: accepts `project_id`, locks agent (mutable), delegates to `agent.start_chat()`
- [x] 8.6 Implement `send_project_chat_message` command: accepts `session_id`, `message`, locks agent (mutable), delegates to `agent.send_chat_message()`
- [x] 8.7 Implement `get_project_chat_history` command: accepts `session_id`, locks agent, delegates to `agent.get_chat_history()`
- [x] 8.8 Implement `end_project_chat` command: accepts `session_id`, locks agent (mutable), delegates to `agent.end_chat()`

### Task 9: Register Agent and Commands in `lib.rs`

- [x] 9.1 Wrap `intel_queue` in `Arc` before registration: `let intel_queue_arc = Arc::new(intel_queue); app.manage(intel_queue_arc.clone());`
- [x] 9.2 Clone `search_provider` before `app.manage` consumes it: `let search_provider_for_agent = search_provider.clone();`
- [x] 9.3 Create `ProjectResearchAgent::new()` with all provider Arcs: `project_store_arc`, `gem_store_arc`, `intel_provider`, `search_provider_for_agent`, `intel_queue_arc`
- [x] 9.4 Register agent in Tauri state: `app.manage(Arc::new(tokio::sync::Mutex::new(project_agent)))`
- [x] 9.5 Add all 7 agent commands to `generate_handler![]`:
  - `projects::commands::suggest_project_topics`
  - `projects::commands::run_project_research`
  - `projects::commands::get_project_summary`
  - `projects::commands::start_project_chat`
  - `projects::commands::send_project_chat_message`
  - `projects::commands::get_project_chat_history`
  - `projects::commands::end_project_chat`
- [x] 9.6 Fix `project_store_arc` consumed without `.clone()` — changed to `app.manage(project_store_arc.clone())`
- [x] 9.7 Fix `intel_provider` consumed without `.clone()` — changed to `app.manage(intel_provider.clone())`
- [x] 9.8 Update 2 existing `State<'_, IntelQueue>` → `State<'_, Arc<IntelQueue>>` in `commands.rs`

**Checkpoint**: ✅ `cargo build` succeeds. All 7 agent commands registered. Backend fully functional: suggest_topics, run_research, summarize, and chat all invocable from frontend.

---

## Phase 5: Frontend — TypeScript Types and ProjectResearchChat Component (Requirements 8, 9) ✅

### Task 10: Add TypeScript Types

- [x] 10.1 Add `WebSearchResult` interface to `src/state/types.ts`: `title`, `url`, `snippet`, `source_type` (union: `'Paper' | 'Article' | 'Video' | 'Other'`), `domain`, `published_date` (string | null)
- [x] 10.2 Add `ProjectResearchResults` interface: `web_results: WebSearchResult[]`, `suggested_gems: GemSearchResult[]`, `topics_searched: string[]`
- [x] 10.3 Export all new types from `types.ts`

### Task 11: Create `ProjectResearchChat` Component

- [x] 11.1 Create `src/components/ProjectResearchChat.tsx` with props: `projectId: string`, `projectTitle: string`, `onGemsAdded?: () => void`
- [x] 11.2 Implement state: `messages: ChatMessage[]`, `input`, `loading`, `topics: string[]`, `addedGemIds: Set<string>`, `initializing`
- [x] 11.3 Implement `useEffect` for auto-suggesting topics on mount: call `invoke('suggest_project_topics', { projectId })`, render as agent's opening message with topic chips
- [x] 11.4 Implement `handleRunResearch`: calls `invoke('run_project_research', { projectId, topics })`, renders results as rich cards in chat
- [x] 11.5 Implement `handleSendMessage` with keyword-based intent detection (v1):
  - "search" / "go ahead" / "find" → triggers `handleRunResearch` with current topics
  - "summarize" / "summary" → calls `invoke('get_project_summary', { projectId })`
  - Default → adds message as a new topic to the list
- [x] 11.6 Implement `handleRemoveTopic(index)`: removes topic from state, adds system message
- [x] 11.7 Implement `handleAddGem(gemId)`: calls `invoke('add_gems_to_project', ...)`, tracks added gems
- [x] 11.8 Render topic suggestion messages with numbered chips, remove buttons, and "Search (N topics)" button
- [x] 11.9 Render web result cards inline in chat: source type badge, domain, title, snippet — click opens URL via `shell.open`
- [x] 11.10 Render gem suggestion cards inline in chat: source badge, title, "Add" / "Added" button
- [x] 11.11 Render system messages centered and muted (for topic add/remove events)
- [x] 11.12 Show loading spinner during initialization ("Analyzing your project...")
- [x] 11.13 Auto-scroll to latest message via `messagesEndRef`

**Checkpoint**: ✅ Frontend builds. Research chat component renders with topic chips, web cards, and gem cards.

---

## Phase 6: Frontend Integration — RightPanel, App.tsx, ProjectsContainer (Requirement 10)

### Task 12: Wire `ProjectResearchChat` into RightPanel

- [x] 12.1 Add new props to `RightPanel`: `selectedProjectId?: string | null`, `selectedProjectTitle?: string | null`, `onProjectGemsChanged?: () => void`
- [x] 12.2 Import `ProjectResearchChat` in `RightPanel.tsx`
- [x] 12.3 Replace `activeNav === 'projects'` block with three states:
  - No project selected → placeholder "Select a project to start researching"
  - Project selected + gem selected → tabs: Research (default) | Detail
  - Project selected, no gem → `ProjectResearchChat` full-height
- [x] 12.4 "Research" tab renders `ProjectResearchChat`, "Detail" tab renders existing `GemDetailPanel`
- [x] 12.5 Added `useEffect` to default to Research tab when entering projects view
- [x] 12.6 `key={selectedProjectId}` on all `<ProjectResearchChat>` instances for remount on project change

### Task 13: Lift Project State to App.tsx

- [x] 13.1 Add `selectedProjectId`, `selectedProjectTitle`, `projectGemsRefreshKey` state to `App.tsx`
- [x] 13.2 Add `handleProjectSelect` callback with `useCallback`
- [x] 13.3 Add `handleProjectGemsChanged` callback with `useCallback`
- [x] 13.4 Pass `selectedProjectId`, `selectedProjectTitle`, `onProjectGemsChanged` to `RightPanel`
- [x] 13.5 Pass `onProjectSelect` and `refreshTrigger` to `ProjectsContainer`
- [x] 13.6 Reset `selectedProjectId` and `selectedProjectTitle` in `handleNavChange`

### Task 14: Update ProjectsContainer to Emit Project Selection

- [x] 14.1 Accept `onProjectSelect` and `refreshTrigger` props in `ProjectsContainer`
- [x] 14.2 Call `onProjectSelect(id, title)` when a project is clicked in ProjectList
- [x] 14.3 Call `onProjectSelect(projectId, projectTitle)` after creating a new project
- [x] 14.4 Call `onProjectSelect(null, null)` on project deletion
- [x] 14.5 `CreateProjectForm.onCreated` passes both `id` and `title`
- [x] 14.6 `ProjectGemList` accepts `refreshTrigger` prop, added to useEffect deps

**Checkpoint**: ✅ Frontend builds. Opening a project shows research chat in RightPanel. Selecting a gem shows Research | Detail tabs. Switching projects resets the chat. Adding gems via research refreshes the gem list.

---

## Phase 7: CSS Styling (Requirement 11) ✅

### Task 15: Add Research Chat CSS

- [x] 15.1 Add `.research-chat` layout: flex column, height 100%
- [x] 15.2 Add `.research-chat-loading`: centered flex column with spinner, muted text
- [x] 15.3 Add `.research-chat-messages`: flex 1, overflow-y auto, padding, custom scrollbar

### Task 16: Add Topic Chip CSS

- [x] 16.1 Add `.research-topics-list`: flex column with gap
- [x] 16.2 Add `.research-topic-chip`: flex row, background, border, rounded, space-between
- [x] 16.3 Add `.topic-remove`: no background button, muted color, hover red (var(--error))
- [x] 16.4 Add `.research-go-button`: align self end, margin top

### Task 17: Add Web Result Card CSS

- [x] 17.1 Add `.web-result-card`: padding, border, rounded, cursor pointer, hover accent border, transitions
- [x] 17.2 Add `.web-result-header`: flex with gap for badge + domain
- [x] 17.3 Add `.source-type-badge` base + variants: `.source-paper` (purple), `.source-article` (blue), `.source-video` (red), `.source-other` (gray)
- [x] 17.4 Add `.web-result-domain`, `.web-result-title`, `.web-result-snippet` (snippet with `-webkit-line-clamp: 2`)

### Task 18: Add Gem Suggestion Card CSS

- [x] 18.1 Add `.research-gem-card`: flex row, space-between, border, rounded
- [x] 18.2 Add `.research-gem-card .gem-info` and `.gem-title`: flex with ellipsis overflow
- [x] 18.3 Add `.research-add-gem`: outline button with accent color, `.added` state muted

### Task 19: Add System Message CSS

- [x] 19.1 Add `.chat-system-msg`: small, centered, muted, italic
- [x] 19.2 Add `.research-section` and `.research-section-title`: section headers (uppercase, muted)

**Checkpoint**: ✅ All research chat UI styled consistently with dark theme. Design tokens used throughout. No existing CSS modified. Topic chips, web cards, gem cards all polished.

---

## Phase 8: Testing and Polish

### Task 20: Verify search infrastructure (Phases 1-2)

- [x] 20.1 Test `classify_source_type`: youtube.com → Video, arxiv.org → Paper, medium.com → Article, random.com → Other
- [x] 20.2 Test `extract_domain`: strips protocol, www prefix, path correctly
- [x] 20.3 Test `CompositeSearchProvider`: `.search()` delegates to gem_provider, `.web_search()` delegates to web_provider or returns empty
- [x] 20.4 Verify existing `search_gems` and `rebuild_search_index` commands work through CompositeSearchProvider

### Task 21: Verify agent actions (Phases 3-4)

- [x] 21.1 Test `suggest_topics()`: returns 3-5 topic strings for a valid project
- [x] 21.2 Test `suggest_topics()`: handles markdown code fences around JSON array
- [x] 21.3 Test `run_research()`: returns web results + gem suggestions for given topics
- [x] 21.4 Test `run_research()` with empty topics: returns empty results
- [x] 21.5 Test `run_research()` graceful degradation: individual web search failure doesn't fail entire command
- [x] 21.6 Test `run_research()` without Tavily key: returns gem suggestions only, empty web_results
- [x] 21.7 Test `run_research()` deduplication: duplicate URLs across topics are removed
- [x] 21.8 Test `summarize()`: returns summary for project with gems
- [x] 21.9 Test `summarize()`: returns friendly message for project with 0 gems
- [x] 21.10 Test `start_chat()` + `send_chat_message()` + `end_chat()` lifecycle
- [x] 21.11 Test `get_chat_history()` returns messages from session

### Task 22: End-to-end verification

- [x] 22.1 Open project → RightPanel shows research chat with loading spinner → topics appear as chips
- [x] 22.2 Remove a topic chip → chip disappears, system message shown
- [x] 22.3 Type a custom topic → added to topic list with confirmation message
- [x] 22.4 Click "Search (N topics)" → loading → web result cards + gem suggestion cards appear in chat
- [x] 22.5 Click a web result card → opens URL in system browser
- [x] 22.6 Click "Add" on a gem card → button changes to "Added", gem appears in project gem list
- [x] 22.7 Type "summarize" → summary appears as chat message
- [x] 22.8 Select a gem in the project → "Research" + "Detail" tabs appear in RightPanel
- [x] 22.9 Switch between Research and Detail tabs → state preserved
- [x] 22.10 Switch to different project → research chat resets with new topics for that project
- [x] 22.11 Remove Tavily API key → restart → web section empty, gems still work
- [x] 22.12 Existing gem search (GemsPanel search bar) still works unchanged
- [x] 22.13 App builds and starts without errors
