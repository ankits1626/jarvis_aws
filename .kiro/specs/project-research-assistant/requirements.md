# Project Research Assistant — Conversational Research Agent

## Introduction

When a user creates a project in Jarvis, they start with an empty container and must manually hunt for relevant resources. There's no help discovering external materials (papers, articles, videos) or surfacing internal gems that relate to the project's goal. As projects grow, there's also no way to get a quick summary of accumulated knowledge.

This spec adds a **Conversational Research Agent** — a chat-first assistant that lives in the RightPanel when viewing a project. The agent suggests research topics, the user refines them through natural dialogue, and research results flow back as rich messages in the chat. It supports three actions: (1) **Research** — two-phase flow: suggest topics → user curates → execute web search + gem suggestions, (2) **Summarize** — LLM-generated summary of all project gems, and (3) **Chat** — conversational Q&A over project content via the existing `Chatbot` + `Chatable` pattern.

The agent follows the same patterns as `CoPilotAgent` (lifecycle management) and `RecordingChatSource` + `Chatbot` (chat capability). It extends the existing `SearchResultProvider` trait with a `web_search` default method and introduces a `CompositeSearchProvider` for delegation.

**Reference:** Discussion doc at `discussion/29-feb-next-step/project-creation-research-assistant.md`. Change request at `change_request.md`. Depends on: [Projects spec](../projects/requirements.md) (project CRUD, `ProjectStore` trait, `ProjectDetail`), [Searchable Gems spec](../searchable-gems/requirements.md) (`SearchResultProvider` trait, FTS/QMD providers), [Intelligence spec](../intelligence-kit/requirements.md) (`IntelProvider` trait, `chat` method).

## Glossary

- **WebSearchResult**: A search result from the internet — title, URL, snippet, source type (Paper/Article/Video/Other), domain, and optional publish date. Returned by `SearchResultProvider::web_search`.
- **WebSourceType**: Classification enum for web results: `Paper` (arxiv, scholar), `Article` (medium, dev.to, blogs), `Video` (youtube, vimeo), `Other`.
- **ProjectResearchResults**: The combined output of the research pipeline — `web_results: Vec<WebSearchResult>`, `suggested_gems: Vec<GemSearchResult>`, `topics_searched: Vec<String>`.
- **TavilyProvider**: A `SearchResultProvider` implementation that calls the Tavily API for web search. No-ops for gem-related methods (`search`, `index_gem`, `remove_gem`, `reindex_all`).
- **CompositeSearchProvider**: A `SearchResultProvider` that wraps a gem provider (QMD or FTS) and an optional web provider (Tavily). Delegates `.search()` to the gem provider and `.web_search()` to the web provider. Registered as the single `Arc<dyn SearchResultProvider>` in Tauri state.
- **ProjectChatSource**: Implements `Chatable` for a project — provides project gem content as context for the `Chatbot` engine. Follows the same pattern as `RecordingChatSource`.
- **ProjectResearchAgent**: A persistent agent (like `CoPilotAgent`) that manages two-phase research, summarization, and chat for a project. Registered in Tauri state as `Arc<TokioMutex<ProjectResearchAgent>>`.
- **ProjectResearchChat**: Frontend component — a chat interface in the RightPanel with rich message rendering (topic chips, web result cards, gem suggestion cards).

## Frozen Design Decisions

These decisions were made during design discussion (2026-03-01):

1. **Extend `SearchResultProvider`, don't create a new trait.** The `web_search` and `supports_web_search` methods are added to the existing trait with default no-op implementations. FTS and QMD providers are unchanged. This keeps one search interface across the codebase.
2. **CompositeSearchProvider for delegation.** Since Tauri state can't hold two instances of the same trait type, a composite wraps the gem provider and web provider behind a single `Arc<dyn SearchResultProvider>`.
3. **Tavily API key already in settings.** `SearchSettings.tavily_api_key: Option<String>` exists with a password input in the Settings UI. No new settings infrastructure needed.
4. **LLM topic generation via existing `IntelProvider::chat`.** No new AI integration — reuse the active intelligence provider (IntelligenceKit or MLX) to generate search topics from project metadata.
5. **Graceful degradation.** If web search is unavailable (no API key, provider down), gem suggestions still work. The UI shows a note instead of failing entirely.
6. **Logging via `eprintln!`.** Uses the existing file-based logging system with `Projects/Research:` prefix for the agent and `Search/Tavily:` prefix for the web provider.
7. **`reqwest` for HTTP.** Tavily API calls use the `reqwest` crate (already in `Cargo.toml`).
8. **Chat-first architecture.** The Research Agent lives in the RightPanel as a conversational collaborator. Research is a conversation, not a form submission.
9. **Two-phase research flow.** `suggest_topics()` generates the opening message, `run_research(topics)` executes on user-curated topics. The user controls what gets searched.
10. **`Chatable` for project chat.** `ProjectChatSource` implements the existing `Chatable` trait, enabling the generic `Chatbot` engine to chat about project content. Follows the `RecordingChatSource` pattern exactly.
11. **Agent in Tauri state.** `ProjectResearchAgent` is registered as `Arc<TokioMutex<ProjectResearchAgent>>` in Tauri managed state, following the same pattern as `CoPilotAgent` (which uses `Arc<TokioMutex<CoPilotAgent>>`).
12. **Frontend intent detection (v1).** Keywords like "search"/"go ahead" trigger research, "summarize" triggers summary, everything else adds topics. Upgradeable to LLM tool-calling in v2.

---

## Requirement 1: Extend SearchResultProvider with Web Search

**User Story:** As a developer, I need the existing search trait to support web search alongside gem search, so both capabilities flow through one interface without creating a parallel trait.

### Acceptance Criteria

1. THE System SHALL add a `WebSearchResult` struct to `src/search/provider.rs` with fields: `title` (String), `url` (String), `snippet` (String), `source_type` (WebSourceType), `domain` (String), `published_date` (Option<String>)
2. THE System SHALL add a `WebSourceType` enum to `src/search/provider.rs` with variants: `Paper`, `Article`, `Video`, `Other`
3. BOTH `WebSearchResult` and `WebSourceType` SHALL derive `Debug`, `Clone`, `Serialize`, `Deserialize`
4. THE `SearchResultProvider` trait SHALL be extended with a `web_search` method: `async fn web_search(&self, query: &str, limit: usize) -> Result<Vec<WebSearchResult>, String>` with a default implementation that returns `Ok(Vec::new())`
5. THE `SearchResultProvider` trait SHALL be extended with a `supports_web_search` method: `fn supports_web_search(&self) -> bool` with a default implementation that returns `false`
6. THE existing `FtsResultProvider` and `QmdResultProvider` SHALL NOT require any changes — they inherit the default implementations
7. THE new types SHALL be re-exported from `src/search/mod.rs`

---

## Requirement 2: TavilyProvider Implementation

**User Story:** As the Jarvis system, I need a web search provider backed by the Tavily API, so projects can discover relevant external resources (papers, articles, videos).

### Acceptance Criteria

1. THE System SHALL create `src/search/tavily_provider.rs` implementing `SearchResultProvider`
2. THE `TavilyProvider` struct SHALL hold an API key (String) and an HTTP client (`reqwest::Client`)
3. THE `web_search` method SHALL call the Tavily Search API (`POST https://api.tavily.com/search`) with the query and limit
4. THE `web_search` method SHALL parse the Tavily response into `Vec<WebSearchResult>`, classifying `source_type` by domain:
   - `youtube.com`, `vimeo.com` -> `Video`
   - `arxiv.org`, `scholar.google.com`, `semanticscholar.org` -> `Paper`
   - `medium.com`, `dev.to`, `*.substack.com`, blog-like domains -> `Article`
   - Everything else -> `Other`
5. THE `supports_web_search` method SHALL return `true`
6. THE `check_availability` method SHALL return `available: true` when the API key is non-empty
7. THE gem-related methods (`search`, `index_gem`, `remove_gem`, `reindex_all`) SHALL be no-ops returning empty/zero results
8. ALL operations SHALL log via `eprintln!` with prefix `Search/Tavily:`
9. THE `TavilyProvider` SHALL be re-exported from `src/search/mod.rs`

---

## Requirement 3: CompositeSearchProvider

**User Story:** As the Jarvis system, I need a single search provider that delegates gem search and web search to separate backends, so one `Arc<dyn SearchResultProvider>` serves all search needs.

### Acceptance Criteria

1. THE System SHALL create `src/search/composite_provider.rs` implementing `SearchResultProvider`
2. THE `CompositeSearchProvider` struct SHALL hold a `gem_provider: Arc<dyn SearchResultProvider>` (required) and a `web_provider: Option<Arc<dyn SearchResultProvider>>` (optional)
3. THE `search`, `index_gem`, `remove_gem`, `reindex_all` methods SHALL delegate to `gem_provider`
4. THE `check_availability` method SHALL delegate to `gem_provider`
5. THE `web_search` method SHALL delegate to `web_provider` if present, or return `Ok(Vec::new())` if absent
6. THE `supports_web_search` method SHALL return `true` only if `web_provider` is present and its `supports_web_search` returns `true`
7. THE `CompositeSearchProvider` SHALL be re-exported from `src/search/mod.rs`

---

## Requirement 4: Provider Registration in lib.rs

**User Story:** As the Jarvis system, I need the composite search provider assembled and registered at startup using existing settings, so both gem search and web search are available through Tauri state.

### Acceptance Criteria

1. THE System SHALL read `settings.search.tavily_api_key` from the existing `SettingsManager` during app setup
2. IF the Tavily API key is present and non-empty, THE System SHALL create a `TavilyProvider` with that key
3. THE System SHALL create a `CompositeSearchProvider` wrapping the existing gem provider (QMD or FTS) and the optional Tavily provider
4. THE `CompositeSearchProvider` SHALL be registered as `Arc<dyn SearchResultProvider>` in Tauri managed state — replacing the current direct registration of the gem provider
5. ALL existing Tauri commands (`search_gems`, `check_search_availability`, `rebuild_search_index`) SHALL continue to work unchanged
6. THE `Cargo.toml` SHALL include `reqwest` with TLS support as a dependency (already present)

---

## Requirement 5: ProjectChatSource — Chatable for Projects

**User Story:** As the Jarvis system, I need projects to be chatbot-compatible, so users can have conversational Q&A about a project's collected gems using the existing `Chatbot` engine.

### Acceptance Criteria

1. THE System SHALL create `src/agents/project_chat.rs` implementing `Chatable` for projects
2. THE `ProjectChatSource` struct SHALL hold a `project_id: String`, `project_title: String`, a `project_store: Arc<dyn ProjectStore>`, and a `gem_store: Arc<dyn GemStore>`
3. THE `get_context` method SHALL:
   a. Load the project via `project_store.get(project_id)` to get associated gems
   b. For each gem, load full gem data via `gem_store.get(gem_id)`
   c. Assemble context as: project title/description/objective + gem titles + gem summaries (from `ai_enrichment`)
   d. Return the assembled context string
4. THE `label` method SHALL return `"Project: {project_title}"`
5. THE `session_dir` method SHALL return a project-specific directory: `{app_data}/projects/{project_id}/chat_sessions/`
6. THE `needs_preparation` method SHALL return `false` (project context is assembled on the fly, no expensive generation needed)
7. THE `ProjectChatSource` SHALL be re-exported from `src/agents/mod.rs`

---

## Requirement 6: ProjectResearchAgent — Two-Phase Research

**User Story:** As the Jarvis system, I need a persistent agent that manages two-phase research (suggest topics → execute on user-curated topics), summarization, and chat for projects, so these capabilities are available as a conversational collaborator.

### Acceptance Criteria

1. THE System SHALL create `src/agents/project_agent.rs` containing the `ProjectResearchAgent` struct
2. THE `ProjectResearchAgent` SHALL hold references to all required providers: `Arc<dyn ProjectStore>`, `Arc<dyn GemStore>`, `Arc<dyn IntelProvider>`, `Arc<dyn SearchResultProvider>`, `Arc<IntelQueue>`
3. THE agent SHALL hold an internal `Chatbot` instance and a `HashMap<String, ProjectChatSource>` for chat sessions
4. THE agent SHALL expose a `suggest_topics` method that:
   a. Accepts a `project_id: &str`
   b. Loads the project via `ProjectStore::get`
   c. Generates search topics via `IntelProvider::chat` (3-5 specific queries from project metadata)
   d. Parses the JSON array response (strips markdown code fences if present)
   e. Returns `Result<Vec<String>, String>`
5. THE agent SHALL expose a `run_research` method that:
   a. Accepts `project_id: &str` and `topics: Vec<String>` (user-curated)
   b. If `search_provider.supports_web_search()`, calls `search_provider.web_search(topic, 5)` for each topic
   c. Deduplicates web results by URL
   d. Calls `search_provider.search(project.title, 20)` for gem suggestions
   e. Enriches gem results with full gem data (same pattern as `search_gems` command)
   f. Returns `ProjectResearchResults { web_results, suggested_gems, topics_searched }`
6. THE agent SHALL expose a `summarize` method that:
   a. Accepts a `project_id: &str`
   b. Loads the project and its associated gems
   c. Returns early with friendly message if 0 gems
   d. Assembles all gem content (titles, descriptions, summaries from `ai_enrichment`)
   e. Calls `IntelProvider::chat` with a summarization prompt
   f. Returns the summary as a `String`
7. THE agent SHALL expose chat lifecycle methods:
   a. `start_chat(project_id)` — creates a `ProjectChatSource`, starts a `Chatbot` session, returns session_id
   b. `send_chat_message(session_id, message)` — delegates to `Chatbot::send_message` via the `ProjectChatSource` and `IntelQueue`
   c. `get_chat_history(session_id)` — delegates to `Chatbot::get_history`
   d. `end_chat(session_id)` — delegates to `Chatbot::end_session`
8. ALL operations SHALL log via `eprintln!` with prefix `Projects/Research:`
9. THE `ProjectResearchAgent` SHALL be re-exported from `src/agents/mod.rs`
10. IF web search is not supported, THE agent SHALL skip web search and return empty `web_results` — it SHALL NOT fail
11. IF any individual web search call fails, THE agent SHALL log the error and continue with remaining topics

---

## Requirement 7: Agent Tauri Commands

**User Story:** As the frontend, I need Tauri commands that expose the agent's research, summarize, and chat capabilities, so I can invoke them from the UI.

### Acceptance Criteria

1. THE System SHALL expose a `suggest_project_topics` Tauri command accepting `project_id: String`
   - SHALL use `State<'_, Arc<TokioMutex<ProjectResearchAgent>>>`
   - SHALL return `Result<Vec<String>, String>`
   - SHALL delegate to `agent.suggest_topics(project_id)`
2. THE System SHALL expose a `run_project_research` Tauri command accepting `project_id: String` and `topics: Vec<String>`
   - SHALL use `State<'_, Arc<TokioMutex<ProjectResearchAgent>>>`
   - SHALL return `Result<ProjectResearchResults, String>`
   - SHALL delegate to `agent.run_research(project_id, topics)`
3. THE System SHALL expose a `get_project_summary` Tauri command accepting `project_id: String`
   - SHALL use `State<'_, Arc<TokioMutex<ProjectResearchAgent>>>`
   - SHALL return `Result<String, String>`
   - SHALL delegate to `agent.summarize(project_id)`
4. THE System SHALL expose a `start_project_chat` Tauri command accepting `project_id: String`
   - SHALL return `Result<String, String>` (the session_id)
   - SHALL delegate to `agent.start_chat(project_id)`
5. THE System SHALL expose a `send_project_chat_message` Tauri command accepting `session_id: String` and `message: String`
   - SHALL return `Result<String, String>` (the assistant's response)
   - SHALL delegate to `agent.send_chat_message(session_id, message)`
6. THE System SHALL expose a `get_project_chat_history` Tauri command accepting `session_id: String`
   - SHALL return `Result<Vec<ChatMessage>, String>`
   - SHALL delegate to `agent.get_chat_history(session_id)`
7. THE System SHALL expose an `end_project_chat` Tauri command accepting `session_id: String`
   - SHALL delegate to `agent.end_chat(session_id)`
8. ALL commands SHALL be registered in `lib.rs` in the `generate_handler!` macro
9. THE `ProjectResearchAgent` SHALL be registered in Tauri state as `Arc<TokioMutex<ProjectResearchAgent>>` during app setup in `lib.rs`
10. THE `intel_queue` SHALL be wrapped in `Arc` before registration so the agent can hold a reference

---

## Requirement 8: ProjectResearchResults TypeScript Type

**User Story:** As the frontend, I need TypeScript types for the research results and agent interactions, so I can type-safely render web results, gem suggestions, and chat messages.

### Acceptance Criteria

1. THE System SHALL add the following interfaces to `src/state/types.ts`:
   - `WebSearchResult` with fields: `title: string`, `url: string`, `snippet: string`, `source_type: 'Paper' | 'Article' | 'Video' | 'Other'`, `domain: string`, `published_date: string | null`
   - `ProjectResearchResults` with fields: `web_results: WebSearchResult[]`, `suggested_gems: GemSearchResult[]`, `topics_searched: string[]`
2. ALL new types SHALL be exported from `types.ts`
3. THE types SHALL match the Rust struct field names exactly (snake_case)

---

## Requirement 9: ProjectResearchChat Component — Conversational Research UI

**User Story:** As a user who opens a project, I want to interact with a research assistant through a chat interface where it suggests topics, I refine them through dialogue, and research results appear as rich cards — so research feels like a collaboration, not a form submission.

### Acceptance Criteria

1. THE System SHALL create a `ProjectResearchChat` component rendered in the RightPanel when a project is selected
2. ON mount, THE component SHALL call `invoke('suggest_project_topics', { projectId })` and display the agent's opening message with numbered topic chips
3. EACH topic chip SHALL have a remove button to drop that topic
4. THE user SHALL be able to type new topics which get added to the topic list
5. WHEN the user says "search" / "go ahead" / "find", THE component SHALL call `invoke('run_project_research', { projectId, topics })` with the current topic list
6. RESEARCH results SHALL render inline in the chat as:
   a. **Web result cards** — source type badge, domain, title, snippet. Clicking opens URL via `shell.open`
   b. **Gem suggestion cards** — source badge, title, "Add to Project" button. Clicking "Add" calls `invoke('add_gems_to_project', ...)` and updates to "Added" state
7. WHEN the user says "summarize" / "summary", THE component SHALL call `invoke('get_project_summary', { projectId })` and display the result
8. THE chat SHALL show a loading spinner ("Analyzing your project...") while initial topics are being generated
9. THE chat SHALL auto-scroll to the latest message
10. WHILE loading, THE input SHALL be disabled and a "Thinking..." indicator shown
11. IF topic suggestion fails, THE component SHALL show an error message and allow the user to type their own topics
12. IF research fails, THE component SHALL show an error message in the chat

---

## Requirement 10: RightPanel and App Integration for Projects

**User Story:** As a user viewing a project, I want the research chat to appear in the right panel, and when I select a gem, I want to toggle between the research chat and gem detail — so the research assistant is always accessible.

### Acceptance Criteria

1. WHEN `activeNav === 'projects'` and a project is selected, THE RightPanel SHALL show the `ProjectResearchChat` component
2. WHEN a project is selected AND a gem is selected, THE RightPanel SHALL show tabs: "Research" (default) and "Detail"
3. THE "Research" tab SHALL render `ProjectResearchChat`
4. THE "Detail" tab SHALL render the existing `GemDetailPanel`
5. WHEN no project is selected, THE RightPanel SHALL show a placeholder: "Select a project to start researching"
6. THE `selectedProjectId` and `selectedProjectTitle` state SHALL be lifted from `ProjectsContainer` to `App.tsx` so both `ProjectsContainer` and `RightPanel` can access them
7. `ProjectsContainer` SHALL emit `onProjectSelect(id, title)` when a project is clicked or created
8. SWITCHING projects SHALL remount `ProjectResearchChat` to reset the conversation

---

## Requirement 11: CSS Styling for Research Chat

**User Story:** As a user, I want the research chat interface to look consistent with the rest of Jarvis, following the dark theme and matching existing chat and card patterns.

### Acceptance Criteria

1. THE research chat layout SHALL be a flex column filling the available height
2. TOPIC chips SHALL display as bordered rows with topic text and a remove button, with hover effect on the remove button
3. WEB result cards SHALL display:
   - Source type badge (distinct colors for Paper/purple, Article/blue, Video/red, Other/gray)
   - Domain name in muted text
   - Title as primary text
   - Snippet truncated to 2 lines
   - Hover effect with accent border
4. GEM suggestion cards SHALL display as flex rows with source badge, title (truncated), and "Add" / "Added" button
5. THE "Added" state SHALL show muted styling with no hover effect
6. SYSTEM messages (topic add/remove) SHALL be small, centered, muted, and italic
7. THE loading state SHALL be centered with a spinner and muted text
8. ALL new CSS SHALL be added to `App.css`

---

## Technical Constraints

1. **One search interface**: All search goes through `SearchResultProvider`. No new traits for search.
2. **Existing settings**: Tavily API key is read from `SearchSettings.tavily_api_key` — already has a Settings UI field.
3. **Existing LLM**: Topic generation and summarization use `IntelProvider::chat` — no new AI integration.
4. **Existing chat engine**: Project chat uses the `Chatbot` + `Chatable` pattern — `ProjectChatSource` implements `Chatable`, `Chatbot` handles sessions and LLM prompting.
5. **Graceful degradation**: Web search failure never blocks gem suggestions. Missing API key means web section is simply empty.
6. **reqwest dependency**: Already in `Cargo.toml`. Needed for Tavily HTTP calls.
7. **No web result persistence**: Web results are ephemeral — fetched on demand, not stored in the database. Re-running research re-fetches.
8. **Sequential web search**: Topic searches run sequentially to respect API rate limits. Parallel execution is a future optimization.
9. **Agent in Tauri state**: `ProjectResearchAgent` follows the `CoPilotAgent` registration pattern — `Arc<TokioMutex<ProjectResearchAgent>>` in managed state.
10. **IntelQueue Arc wrapping**: `intel_queue` must be wrapped in `Arc` before registration so the agent can hold a reference.

## Out of Scope

1. **Web result caching/persistence** — results are ephemeral. Caching is a future optimization.
2. **Hot-reload of Tavily API key** — changing the key in settings requires an app restart for the provider to pick it up.
3. **Parallel web search execution** — topics are searched sequentially. Parallel calls are a future optimization.
4. **Auto-ingestion of web results as gems** — users must manually open links. Saving web content as gems is a separate feature.
5. **Custom topic count configuration** — hardcoded to 3-5 topics from the LLM prompt.
6. **Source-type-specific LLM prompting** — topics are general; source classification is done by domain after search.
7. **Alternative web search providers** — only Tavily is implemented. Brave/SerpAPI/Searx are future additions that just implement `web_search` on a new `SearchResultProvider`.
8. **LLM tool-calling for intent detection** — v1 uses keyword matching. v2 can upgrade to structured tool-calling for more robust intent detection.
9. **Chat persistence across app restarts** — research chat is ephemeral in v1. Can be persisted via existing Chatbot session logs in v2.
