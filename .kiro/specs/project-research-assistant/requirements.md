# Project Research Assistant — Agent-Based Research & Summarization

## Introduction

When a user creates a project in Jarvis, they start with an empty container and must manually hunt for relevant resources. There's no help discovering external materials (papers, articles, videos) or surfacing internal gems that relate to the project's goal. As projects grow, there's also no way to get a quick summary of accumulated knowledge.

This spec adds a **Project Research Agent** — a persistent, reusable agent that can be invoked on any project at any time. It supports three actions: (1) **Research** — LLM-generated search topics fed into web search + semantic gem suggestions, (2) **Summarize** — LLM-generated summary of all project gems, and (3) **Chat** — conversational Q&A over project content via the existing `Chatbot` + `Chatable` pattern.

The agent follows the same patterns as `CoPilotAgent` (lifecycle management) and `RecordingChatSource` + `Chatbot` (chat capability). It extends the existing `SearchResultProvider` trait with a `web_search` default method and introduces a `CompositeSearchProvider` for delegation.

**Reference:** Discussion doc at `discussion/29-feb-next-step/project-creation-research-assistant.md`. Depends on: [Projects spec](../projects/requirements.md) (project CRUD, `ProjectStore` trait, `ProjectDetail`), [Searchable Gems spec](../searchable-gems/requirements.md) (`SearchResultProvider` trait, FTS/QMD providers), [Intelligence spec](../intelligence-kit/requirements.md) (`IntelProvider` trait, `chat` method).

## Glossary

- **WebSearchResult**: A search result from the internet — title, URL, snippet, source type (Paper/Article/Video/Other), domain, and optional publish date. Returned by `SearchResultProvider::web_search`.
- **WebSourceType**: Classification enum for web results: `Paper` (arxiv, scholar), `Article` (medium, dev.to, blogs), `Video` (youtube, vimeo), `Other`.
- **ProjectResearchResults**: The combined output of both research pipelines — `web_results: Vec<WebSearchResult>`, `suggested_gems: Vec<GemSearchResult>`, `topics_generated: Vec<String>`.
- **TavilyProvider**: A `SearchResultProvider` implementation that calls the Tavily API for web search. No-ops for gem-related methods (`search`, `index_gem`, `remove_gem`, `reindex_all`).
- **CompositeSearchProvider**: A `SearchResultProvider` that wraps a gem provider (QMD or FTS) and an optional web provider (Tavily). Delegates `.search()` to the gem provider and `.web_search()` to the web provider. Registered as the single `Arc<dyn SearchResultProvider>` in Tauri state.
- **ProjectChatSource**: Implements `Chatable` for a project — provides project gem content as context for the `Chatbot` engine. Follows the same pattern as `RecordingChatSource`.
- **ProjectResearchAgent**: A persistent agent (like `CoPilotAgent`) that manages research, summarization, and chat for a project. Registered in Tauri state as `Arc<TokioMutex<ProjectResearchAgent>>`.
- **ProjectResearchPanel**: Frontend component that shows loading state, then renders web results and gem suggestions.

## Frozen Design Decisions

These decisions were made during design discussion (2026-03-01):

1. **Extend `SearchResultProvider`, don't create a new trait.** The `web_search` and `supports_web_search` methods are added to the existing trait with default no-op implementations. FTS and QMD providers are unchanged. This keeps one search interface across the codebase.
2. **CompositeSearchProvider for delegation.** Since Tauri state can't hold two instances of the same trait type, a composite wraps the gem provider and web provider behind a single `Arc<dyn SearchResultProvider>`.
3. **Tavily API key already in settings.** `SearchSettings.tavily_api_key: Option<String>` exists with a password input in the Settings UI. No new settings infrastructure needed.
4. **LLM topic generation via existing `IntelProvider::chat`.** No new AI integration — reuse the active intelligence provider (IntelligenceKit or MLX) to generate search topics from project metadata.
5. **Graceful degradation.** If web search is unavailable (no API key, provider down), gem suggestions still work. The UI shows a note instead of failing entirely.
6. **Logging via `eprintln!`.** Uses the existing file-based logging system with `Projects/Research:` prefix for the agent and `Search/Tavily:` prefix for the web provider.
7. **`reqwest` for HTTP.** Tavily API calls use the `reqwest` crate (already in `Cargo.toml`).
8. **Agent-based architecture.** The Research Assistant is a persistent agent (`ProjectResearchAgent`) that can be invoked independently on any project at any time — not just on project creation. It supports research, summarize, and chat actions.
9. **`Chatable` for project chat.** `ProjectChatSource` implements the existing `Chatable` trait, enabling the generic `Chatbot` engine to chat about project content. Follows the `RecordingChatSource` pattern exactly.
10. **Agent in Tauri state.** `ProjectResearchAgent` is registered as `Arc<TokioMutex<ProjectResearchAgent>>` in Tauri managed state, following the same pattern as `CoPilotAgent` (which uses `Arc<TokioMutex<CoPilotAgent>>`).

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

## Requirement 6: ProjectResearchAgent

**User Story:** As the Jarvis system, I need a persistent agent that manages research, summarization, and chat for projects, so these capabilities are available independently and on demand — not just at project creation.

### Acceptance Criteria

1. THE System SHALL create `src/agents/project_agent.rs` containing the `ProjectResearchAgent` struct
2. THE `ProjectResearchAgent` SHALL hold references to all required providers: `Arc<dyn ProjectStore>`, `Arc<dyn GemStore>`, `Arc<dyn IntelProvider>`, `Arc<dyn SearchResultProvider>`
3. THE agent SHALL hold an internal `Chatbot` instance for managing chat sessions
4. THE agent SHALL expose a `research` method that:
   a. Accepts a `project_id: String`
   b. Loads the project via `ProjectStore::get`
   c. Generates search topics via `IntelProvider::chat` (3-5 specific queries from project metadata)
   d. If `search_provider.supports_web_search()`, calls `search_provider.web_search(topic, 5)` for each topic
   e. Deduplicates web results by URL
   f. Calls `search_provider.search(project.title, 20)` for gem suggestions
   g. Enriches gem results with `GemPreview` data (same pattern as `search_gems` command)
   h. Returns `ProjectResearchResults { web_results, suggested_gems, topics_generated }`
5. THE agent SHALL expose a `summarize` method that:
   a. Accepts a `project_id: String`
   b. Loads the project and its associated gems
   c. Assembles all gem content (titles, descriptions, summaries from `ai_enrichment`)
   d. Calls `IntelProvider::chat` with a summarization prompt
   e. Returns the summary as a `String`
6. THE agent SHALL expose chat lifecycle methods:
   a. `start_chat(project_id)` — creates a `ProjectChatSource`, starts a `Chatbot` session, returns session_id
   b. `send_chat_message(session_id, message)` — delegates to `Chatbot::send_message` via the `ProjectChatSource` and `IntelQueue`
   c. `get_chat_history(session_id)` — delegates to `Chatbot::get_history`
   d. `end_chat(session_id)` — delegates to `Chatbot::end_session`
7. ALL operations SHALL log via `eprintln!` with prefix `Projects/Research:`
8. THE `ProjectResearchAgent` SHALL be re-exported from `src/agents/mod.rs`
9. IF web search is not supported, THE agent SHALL skip web search and return empty `web_results` — it SHALL NOT fail
10. IF any individual web search call fails, THE agent SHALL log the error and continue with remaining topics

---

## Requirement 7: Agent Tauri Commands

**User Story:** As the frontend, I need Tauri commands that expose the agent's research, summarize, and chat capabilities, so I can invoke them from the UI.

### Acceptance Criteria

1. THE System SHALL expose a `get_project_research` Tauri command accepting `project_id: String`
   - SHALL use `State<'_, Arc<TokioMutex<ProjectResearchAgent>>>`
   - SHALL return `Result<ProjectResearchResults, String>`
   - SHALL delegate to `agent.research(project_id)`
2. THE System SHALL expose a `get_project_summary` Tauri command accepting `project_id: String`
   - SHALL use `State<'_, Arc<TokioMutex<ProjectResearchAgent>>>`
   - SHALL return `Result<String, String>`
   - SHALL delegate to `agent.summarize(project_id)`
3. THE System SHALL expose a `start_project_chat` Tauri command accepting `project_id: String`
   - SHALL return `Result<String, String>` (the session_id)
   - SHALL delegate to `agent.start_chat(project_id)`
4. THE System SHALL expose a `send_project_chat_message` Tauri command accepting `session_id: String` and `message: String`
   - SHALL return `Result<String, String>` (the assistant's response)
   - SHALL delegate to `agent.send_chat_message(session_id, message)`
5. THE System SHALL expose a `get_project_chat_history` Tauri command accepting `session_id: String`
   - SHALL return `Result<Vec<ChatMessage>, String>`
   - SHALL delegate to `agent.get_chat_history(session_id)`
6. THE System SHALL expose an `end_project_chat` Tauri command accepting `session_id: String`
   - SHALL delegate to `agent.end_chat(session_id)`
7. ALL commands SHALL be registered in `lib.rs` in the `generate_handler!` macro
8. THE `ProjectResearchAgent` SHALL be registered in Tauri state as `Arc<TokioMutex<ProjectResearchAgent>>` during app setup in `lib.rs`

---

## Requirement 8: ProjectResearchResults TypeScript Type

**User Story:** As the frontend, I need TypeScript types for the research results and agent interactions, so I can type-safely render web results, gem suggestions, and chat messages.

### Acceptance Criteria

1. THE System SHALL add the following interfaces to `src/state/types.ts`:
   - `WebSearchResult` with fields: `title: string`, `url: string`, `snippet: string`, `source_type: 'Paper' | 'Article' | 'Video' | 'Other'`, `domain: string`, `published_date: string | null`
   - `ProjectResearchResults` with fields: `web_results: WebSearchResult[]`, `suggested_gems: GemSearchResult[]`, `topics_generated: string[]`
2. ALL new types SHALL be exported from `types.ts`
3. THE types SHALL match the Rust struct field names exactly (snake_case)

---

## Requirement 9: ProjectResearchPanel Component

**User Story:** As a user who just created a project, I want to see relevant web resources and matching gems from my library, so I have a starting point for my research without manual searching.

### Acceptance Criteria

1. THE System SHALL create a `ProjectResearchPanel` component rendered inside `ProjectGemList`
2. THE component SHALL activate when:
   a. A project was just created (0 gems and 0 prior research), OR
   b. The user clicks a "Research" button in the project toolbar
3. ON activation, THE component SHALL call `invoke<ProjectResearchResults>('get_project_research', { projectId })`
4. WHILE loading, THE component SHALL render a spinner with text "Researching your project..." and subtext "Finding relevant articles, papers, videos, and gems..."
5. ON success, THE component SHALL render two collapsible sections:
   a. **"Suggested from the web"** — `WebSearchResult` cards showing title, snippet, domain badge, and source type icon. Clicking a card opens the URL in the system browser via `shell.open`
   b. **"From your gem library"** — `GemSearchResult` cards with an "Add to Project" button. Clicking "Add" calls `invoke('add_gems_to_project', { projectId, gemIds: [gemId] })` and updates the card to show "Added" state
6. IF `web_results` is empty, THE web section SHALL show: "No web resources found. Try refining your project description."
7. IF `suggested_gems` is empty, THE gem section SHALL show: "No matching gems in your library yet."
8. IF web search is not configured (empty `web_results` and `supports_web_search` would be false), THE component SHALL show gem suggestions only with a subtle note: "Web search not configured."
9. ON error, THE component SHALL show an inline error: "Research failed: {error}. You can still add gems manually."

---

## Requirement 10: Research & Summarize Buttons in Project Toolbar

**User Story:** As a user viewing an existing project, I want to re-run the research assistant and generate project summaries on demand, so I can discover new resources and get overviews as my project evolves.

### Acceptance Criteria

1. THE `ProjectGemList` toolbar SHALL include a "Research" button alongside the existing search input and "+ Add Gems" button
2. CLICKING the "Research" button SHALL trigger the `ProjectResearchPanel` component (same as auto-trigger on project creation)
3. THE "Research" button SHALL be disabled while research is loading
4. THE `ProjectGemList` toolbar SHALL include a "Summarize" button
5. CLICKING the "Summarize" button SHALL call `invoke<string>('get_project_summary', { projectId })`
6. THE summary result SHALL be displayed inline in a dismissible panel above the gem list
7. WHILE summarization is loading, the "Summarize" button SHALL be disabled with a loading indicator
8. RESULTS from research/summary runs SHALL be displayed inline above the existing gem list, dismissible by the user

---

## Requirement 11: CSS Styling for Research Panel

**User Story:** As a user, I want the research results to look consistent with the rest of Jarvis, following the dark theme.

### Acceptance Criteria

1. THE loading spinner SHALL be centered vertically and horizontally within the `ProjectGemList` area
2. WEB result cards SHALL display:
   - Source type icon/badge (distinct for Paper, Article, Video, Other)
   - Domain name in muted text
   - Title as primary text
   - Snippet truncated to 3 lines
   - Hover effect matching existing card patterns
3. GEM suggestion cards SHALL reuse existing `GemCard` styling with an added "Add to Project" action button
4. THE "Added" state on gem cards SHALL show a checkmark or "Added" label with muted styling
5. COLLAPSIBLE section headers SHALL match existing panel heading styles
6. THE summary panel SHALL use blockquote styling consistent with the dark theme
7. ALL new CSS SHALL be added to `App.css`

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

## Out of Scope

1. **Web result caching/persistence** — results are ephemeral. Caching is a future optimization.
2. **Hot-reload of Tavily API key** — changing the key in settings requires an app restart for the provider to pick it up.
3. **Parallel web search execution** — topics are searched sequentially. Parallel calls are a future optimization.
4. **Auto-ingestion of web results as gems** — users must manually open links. Saving web content as gems is a separate feature.
5. **Custom topic count configuration** — hardcoded to 3-5 topics from the LLM prompt.
6. **Source-type-specific LLM prompting** — topics are general; source classification is done by domain after search.
7. **Alternative web search providers** — only Tavily is implemented. Brave/SerpAPI/Searx are future additions that just implement `web_search` on a new `SearchResultProvider`.
8. **Project chat UI** — the chat interface (chat panel, message bubbles, input) is a future requirement. This spec only defines the backend `ProjectChatSource` + agent chat methods and Tauri commands.
