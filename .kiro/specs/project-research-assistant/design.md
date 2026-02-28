# Project Research Assistant — Design Document

## Overview

This design adds a **Project Research Agent** — a persistent, reusable agent that manages research (web search + gem suggestions), summarization, and chat for any project. It follows established Jarvis patterns:

- **Search infrastructure**: Extends `SearchResultProvider` with `web_search` default method. `TavilyProvider` for web search, `CompositeSearchProvider` for delegation. Same trait, no new interfaces.
- **Agent pattern**: `ProjectResearchAgent` follows `CoPilotAgent` — a struct in Tauri state with action methods. Registered as `Arc<TokioMutex<ProjectResearchAgent>>`.
- **Chat pattern**: `ProjectChatSource` implements `Chatable` (like `RecordingChatSource`), enabling the generic `Chatbot` engine to chat about project content.

### Design Goals

1. **No new search traits**: Everything flows through `SearchResultProvider`. Existing providers unchanged.
2. **Pluggable web search**: Tavily today, swap to Brave/SerpAPI later by implementing one method.
3. **Reusable agent**: Research + Summarize + Chat available on any project at any time — not just at creation.
4. **Existing patterns**: `Chatable` for chat, `Chatbot` for session management, `CoPilotAgent`-style state registration.
5. **Graceful degradation**: No Tavily key? Gem suggestions still work. API down? Skip web results, don't fail.

### Key Design Decisions

- **Default methods, not a new trait**: `web_search` and `supports_web_search` are default methods on `SearchResultProvider`. FTS/QMD inherit no-ops. `TavilyProvider` overrides them.
- **CompositeSearchProvider**: Wraps gem provider + optional web provider. Registered as the single `Arc<dyn SearchResultProvider>`. All existing commands work unchanged.
- **Agent holds all providers**: `ProjectResearchAgent` holds `Arc` references to `ProjectStore`, `GemStore`, `IntelProvider`, `SearchResultProvider`, and an `IntelQueue`. All actions can be invoked independently.
- **`ProjectChatSource` implements `Chatable`**: Assembles project gem content as context. `Chatbot` handles sessions, history, LLM prompting, log files. No chat-specific logic in the agent.
- **Sequential web searches**: One Tavily call per topic, sequentially. Keeps rate limits simple.
- **reqwest already in Cargo.toml**: `reqwest = { version = "0.12", features = ["stream", "blocking", "multipart", "json"] }` — no dependency changes needed.

### Operational Flow — Research

```
User clicks "Research" → invoke('get_project_research', { projectId })
  → ProjectResearchAgent::research(project_id)
    → 1. ProjectStore::get(project_id) — load project metadata
    → 2. IntelProvider::chat(TOPIC_PROMPT, context) — generate 3-5 topics
    → 3. For each topic: SearchResultProvider::web_search(topic, 5) — via Composite → Tavily
    → 4. Deduplicate web results by URL
    → 5. SearchResultProvider::search(project.title, 20) — gem suggestions via Composite → QMD/FTS
    → 6. Enrich gems with GemPreview data
    → Return ProjectResearchResults { web_results, suggested_gems, topics_generated }
```

### Operational Flow — Summarize

```
User clicks "Summarize" → invoke('get_project_summary', { projectId })
  → ProjectResearchAgent::summarize(project_id)
    → 1. ProjectStore::get(project_id) — load project + gem list
    → 2. For each gem: GemStore::get(gem_id) — load titles, descriptions, summaries
    → 3. Assemble context: project metadata + all gem content
    → 4. IntelProvider::chat(SUMMARY_PROMPT, context) — generate summary
    → Return summary String
```

### Operational Flow — Chat

```
User starts chat → invoke('start_project_chat', { projectId })
  → ProjectResearchAgent::start_chat(project_id)
    → Creates ProjectChatSource (implements Chatable)
    → Chatbot::start_session(source) → returns session_id

User sends message → invoke('send_project_chat_message', { sessionId, message })
  → ProjectResearchAgent::send_chat_message(session_id, message)
    → Chatbot::send_message(session_id, message, source, intel_queue)
      → source.get_context() — assembles project gem content
      → IntelQueue::submit(Chat { system + history + user }) → LLM response
    → Return assistant response

User ends chat → invoke('end_project_chat', { sessionId })
  → ProjectResearchAgent::end_chat(session_id)
    → Chatbot::end_session(session_id)
```

---

## Architecture

### Module Hierarchy

```
src/search/
├── provider.rs            — SearchResultProvider trait (MODIFIED: +web_search, +WebSearchResult)
├── fts_provider.rs        — FtsResultProvider (UNCHANGED)
├── qmd_provider.rs        — QmdResultProvider (UNCHANGED)
├── tavily_provider.rs     — TavilyProvider (NEW: web search via Tavily API)
├── composite_provider.rs  — CompositeSearchProvider (NEW: delegates gem + web search)
├── commands.rs            — Tauri search commands (UNCHANGED)
└── mod.rs                 — Module root (MODIFIED: re-exports new types + providers)

src/agents/
├── chatable.rs            — Chatable trait (UNCHANGED)
├── chatbot.rs             — Chatbot engine (UNCHANGED)
├── recording_chat.rs      — RecordingChatSource (UNCHANGED)
├── copilot.rs             — CoPilotAgent (UNCHANGED)
├── project_chat.rs        — ProjectChatSource (NEW: Chatable for projects)
├── project_agent.rs       — ProjectResearchAgent (NEW: research + summarize + chat)
└── mod.rs                 — Module root (MODIFIED: +project_chat, +project_agent)

src/projects/
├── commands.rs            — (MODIFIED: +agent Tauri commands)
└── ...                    — (everything else unchanged)
```

### Dependency Graph

```
                     ┌─────────────────┐
                     │     lib.rs      │
                     │     (setup)     │
                     └────────┬────────┘
                              │ creates & registers:
                              │ 1. CompositeSearchProvider → Arc<dyn SearchResultProvider>
                              │ 2. ProjectResearchAgent → Arc<TokioMutex<ProjectResearchAgent>>
                              ▼
         ┌───────────────────────────────────────┐
         │        ProjectResearchAgent           │
         │  ┌──────────────────────────────────┐ │
         │  │ project_store  gem_store         │ │
         │  │ intel_provider search_provider   │ │
         │  │ intel_queue    chatbot           │ │
         │  └──────────────────────────────────┘ │
         │                                       │
         │  .research(id) → web+gem results     │
         │  .summarize(id) → summary string     │
         │  .start_chat(id) → session_id        │
         │  .send_chat_message(sid, msg) → resp │
         └───────────────────────────────────────┘
                     │                     │
          ┌──────────┘                     └──────────┐
          ▼                                           ▼
┌──────────────────────┐                   ┌────────────────────┐
│ CompositeSearchProv. │                   │ ProjectChatSource  │
│                      │                   │ (implements        │
│ .search() → QMD/FTS │                   │  Chatable)         │
│ .web_search() → Tav │                   │                    │
└──────────────────────┘                   │ .get_context()     │
                                           │ .label()           │
                                           │ .session_dir()     │
                                           └────────────────────┘
                                                    │
                                                    ▼
                                           ┌────────────────────┐
                                           │     Chatbot        │
                                           │ (existing engine)  │
                                           │                    │
                                           │ .start_session()   │
                                           │ .send_message()    │
                                           │ .get_history()     │
                                           └────────────────────┘
```

---

## Modules and Interfaces

### `provider.rs` — Trait Extension (MODIFIED)

**File**: `src/search/provider.rs`

**Changes**: Add `WebSearchResult`, `WebSourceType` types and two default methods to `SearchResultProvider`.

```rust
// ── NEW types (add after existing GemSearchResult) ──

/// A search result from the web (not a gem).
///
/// Returned by SearchResultProvider::web_search for providers that support it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub source_type: WebSourceType,
    pub domain: String,
    pub published_date: Option<String>,
}

/// Classification of a web search result by content type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebSourceType {
    /// Academic papers (arxiv, scholar, semantic scholar)
    Paper,
    /// Blog posts and articles (medium, dev.to, substack)
    Article,
    /// Video content (youtube, vimeo)
    Video,
    /// Everything else
    Other,
}
```

```rust
// ── MODIFIED trait (add these two methods after reindex_all) ──

#[async_trait]
pub trait SearchResultProvider: Send + Sync {
    // --- Existing methods (UNCHANGED) ---
    async fn check_availability(&self) -> AvailabilityResult;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, String>;
    async fn index_gem(&self, gem_id: &str) -> Result<(), String>;
    async fn remove_gem(&self, gem_id: &str) -> Result<(), String>;
    async fn reindex_all(&self) -> Result<usize, String>;

    // --- NEW default methods ---

    /// Search the web for external resources (papers, articles, videos).
    ///
    /// Default: returns empty vec (provider does not support web search).
    /// Override in providers that have web search capability (e.g., Tavily).
    async fn web_search(
        &self,
        _query: &str,
        _limit: usize,
    ) -> Result<Vec<WebSearchResult>, String> {
        Ok(Vec::new())
    }

    /// Check if this provider supports web search.
    ///
    /// Default: false. Override in web-capable providers.
    fn supports_web_search(&self) -> bool {
        false
    }
}
```

**Impact on existing providers**: None. `FtsResultProvider` and `QmdResultProvider` inherit the defaults — `web_search` returns empty, `supports_web_search` returns false. No code changes needed.

### `tavily_provider.rs` — Tavily Web Search (NEW)

**File**: `src/search/tavily_provider.rs`

**Responsibilities**: Implement web search via the Tavily Search API. No-op for gem-related methods.

```rust
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::intelligence::AvailabilityResult;
use super::provider::{
    SearchResultProvider, SearchResult, WebSearchResult, WebSourceType,
};

/// Web search provider backed by the Tavily Search API.
///
/// Implements SearchResultProvider::web_search. All gem-related methods
/// (search, index_gem, remove_gem, reindex_all) are no-ops.
pub struct TavilyProvider {
    api_key: String,
    client: Client,
}

impl TavilyProvider {
    pub fn new(api_key: String) -> Self {
        eprintln!("Search/Tavily: Initialized with API key ({}...)", &api_key[..8.min(api_key.len())]);
        Self {
            api_key,
            client: Client::new(),
        }
    }
}

// ── Tavily API request/response shapes ──

#[derive(Serialize)]
struct TavilySearchRequest {
    query: String,
    max_results: usize,
    search_depth: String,
    api_key: String,
}

#[derive(Deserialize)]
struct TavilySearchResponse {
    results: Vec<TavilyResult>,
}

#[derive(Deserialize)]
struct TavilyResult {
    title: String,
    url: String,
    content: String,
    #[serde(default)]
    published_date: Option<String>,
}

// ── Domain classification ──

/// Classify a URL's domain into a WebSourceType.
fn classify_source_type(url: &str) -> WebSourceType {
    let url_lower = url.to_lowercase();
    if url_lower.contains("youtube.com") || url_lower.contains("youtu.be") || url_lower.contains("vimeo.com") {
        WebSourceType::Video
    } else if url_lower.contains("arxiv.org") || url_lower.contains("scholar.google") || url_lower.contains("semanticscholar.org") || url_lower.contains("ieee.org") || url_lower.contains("acm.org") {
        WebSourceType::Paper
    } else if url_lower.contains("medium.com") || url_lower.contains("dev.to") || url_lower.contains("substack.com") || url_lower.contains("hashnode") || url_lower.contains("blog") {
        WebSourceType::Article
    } else {
        WebSourceType::Other
    }
}

/// Extract domain from a URL (e.g., "https://medium.com/foo" -> "medium.com").
fn extract_domain(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or(url)
        .trim_start_matches("www.")
        .to_string()
}

#[async_trait]
impl SearchResultProvider for TavilyProvider {
    async fn check_availability(&self) -> AvailabilityResult {
        if self.api_key.is_empty() {
            return AvailabilityResult {
                available: false,
                reason: Some("Tavily API key is empty".to_string()),
            };
        }
        AvailabilityResult {
            available: true,
            reason: None,
        }
    }

    // Gem search — not applicable for Tavily
    async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<SearchResult>, String> {
        Ok(Vec::new())
    }

    async fn index_gem(&self, _gem_id: &str) -> Result<(), String> {
        Ok(())
    }

    async fn remove_gem(&self, _gem_id: &str) -> Result<(), String> {
        Ok(())
    }

    async fn reindex_all(&self) -> Result<usize, String> {
        Ok(0)
    }

    // Web search — the real implementation
    async fn web_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<WebSearchResult>, String> {
        eprintln!("Search/Tavily: web_search query=\"{}\" limit={}", query, limit);

        let request = TavilySearchRequest {
            query: query.to_string(),
            max_results: limit,
            search_depth: "basic".to_string(),
            api_key: self.api_key.clone(),
        };

        let response = self.client
            .post("https://api.tavily.com/search")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Tavily API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            eprintln!("Search/Tavily: API error {} — {}", status, body);
            return Err(format!("Tavily API returned {}: {}", status, body));
        }

        let tavily_response: TavilySearchResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Tavily response: {}", e))?;

        let results: Vec<WebSearchResult> = tavily_response
            .results
            .into_iter()
            .map(|r| {
                let source_type = classify_source_type(&r.url);
                let domain = extract_domain(&r.url);
                WebSearchResult {
                    title: r.title,
                    url: r.url,
                    snippet: r.content,
                    source_type,
                    domain,
                    published_date: r.published_date,
                }
            })
            .collect();

        eprintln!("Search/Tavily: Returning {} results for \"{}\"", results.len(), query);
        Ok(results)
    }

    fn supports_web_search(&self) -> bool {
        true
    }
}
```

### `composite_provider.rs` — Delegating Wrapper (NEW)

**File**: `src/search/composite_provider.rs`

**Responsibilities**: Wrap a gem provider and an optional web provider behind a single `SearchResultProvider` implementation.

```rust
use std::sync::Arc;
use async_trait::async_trait;
use crate::intelligence::AvailabilityResult;
use super::provider::{
    SearchResultProvider, SearchResult, WebSearchResult,
};

/// Composite search provider that delegates gem search to one provider
/// and web search to another.
///
/// Registered as the single Arc<dyn SearchResultProvider> in Tauri state.
/// All existing commands (search_gems, check_search_availability, etc.)
/// work unchanged — they call .search() which delegates to gem_provider.
/// New research commands call .web_search() which delegates to web_provider.
pub struct CompositeSearchProvider {
    gem_provider: Arc<dyn SearchResultProvider>,
    web_provider: Option<Arc<dyn SearchResultProvider>>,
}

impl CompositeSearchProvider {
    pub fn new(
        gem_provider: Arc<dyn SearchResultProvider>,
        web_provider: Option<Arc<dyn SearchResultProvider>>,
    ) -> Self {
        let web_status = if web_provider.is_some() { "enabled" } else { "disabled" };
        eprintln!("Search/Composite: Initialized (web search: {})", web_status);
        Self {
            gem_provider,
            web_provider,
        }
    }
}

#[async_trait]
impl SearchResultProvider for CompositeSearchProvider {
    async fn check_availability(&self) -> AvailabilityResult {
        self.gem_provider.check_availability().await
    }

    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, String> {
        self.gem_provider.search(query, limit).await
    }

    async fn index_gem(&self, gem_id: &str) -> Result<(), String> {
        self.gem_provider.index_gem(gem_id).await
    }

    async fn remove_gem(&self, gem_id: &str) -> Result<(), String> {
        self.gem_provider.remove_gem(gem_id).await
    }

    async fn reindex_all(&self) -> Result<usize, String> {
        self.gem_provider.reindex_all().await
    }

    async fn web_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<WebSearchResult>, String> {
        match &self.web_provider {
            Some(wp) => wp.web_search(query, limit).await,
            None => Ok(Vec::new()),
        }
    }

    fn supports_web_search(&self) -> bool {
        self.web_provider
            .as_ref()
            .map_or(false, |wp| wp.supports_web_search())
    }
}
```

### `mod.rs` — Search Module Root (MODIFIED)

**File**: `src/search/mod.rs`

```rust
pub mod provider;
pub mod fts_provider;
pub mod qmd_provider;
pub mod tavily_provider;
pub mod composite_provider;
pub mod commands;

pub use provider::{
    SearchResultProvider,
    SearchResult,
    MatchType,
    GemSearchResult,
    WebSearchResult,
    WebSourceType,
    QmdSetupResult,
    SetupProgressEvent,
};
pub use fts_provider::FtsResultProvider;
pub use qmd_provider::QmdResultProvider;
pub use tavily_provider::TavilyProvider;
pub use composite_provider::CompositeSearchProvider;
```

---

## Agent Module

### `project_chat.rs` — ProjectChatSource (NEW)

**File**: `src/agents/project_chat.rs`

**Responsibilities**: Implement `Chatable` for projects. Assembles project gem content as context for the `Chatbot` engine. Follows the `RecordingChatSource` pattern.

```rust
// ProjectChatSource — Project Conforms to Chatable
//
// This module makes projects chatbot-compatible by implementing the Chatable trait.
// It assembles project metadata + gem content as context for the Chatbot engine.

use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

use super::chatable::Chatable;
use crate::gems::GemStore;
use crate::intelligence::queue::IntelQueue;
use crate::projects::ProjectStore;

/// A project that can be chatted with.
///
/// Assembles project gem content as context. The Chatbot engine calls
/// get_context() on every message to get fresh context.
pub struct ProjectChatSource {
    project_id: String,
    project_title: String,
    project_store: Arc<dyn ProjectStore>,
    gem_store: Arc<dyn GemStore>,
}

impl ProjectChatSource {
    pub fn new(
        project_id: String,
        project_title: String,
        project_store: Arc<dyn ProjectStore>,
        gem_store: Arc<dyn GemStore>,
    ) -> Self {
        Self {
            project_id,
            project_title,
            project_store,
            gem_store,
        }
    }
}

#[async_trait]
impl Chatable for ProjectChatSource {
    async fn get_context(&self, _intel_queue: &IntelQueue) -> Result<String, String> {
        // Load project with its associated gems
        let detail = self.project_store.get(&self.project_id).await?;
        let project = &detail.project;

        // Start building context with project metadata
        let mut context_parts: Vec<String> = Vec::new();
        context_parts.push(format!("# Project: {}", project.title));

        if let Some(ref desc) = project.description {
            context_parts.push(format!("**Description:** {}", desc));
        }
        if let Some(ref obj) = project.objective {
            context_parts.push(format!("**Objective:** {}", obj));
        }

        context_parts.push(format!("\n## Gems ({} total)\n", detail.gems.len()));

        // Assemble gem content
        for gem_preview in &detail.gems {
            let mut gem_section = format!("### {}", gem_preview.title);

            // Try to load full gem for ai_enrichment (summary)
            if let Ok(Some(full_gem)) = self.gem_store.get(&gem_preview.id).await {
                if let Some(ref desc) = full_gem.description {
                    gem_section.push_str(&format!("\n{}", desc));
                }

                // Extract summary from ai_enrichment
                if let Some(ref enrichment) = full_gem.ai_enrichment {
                    if let Some(summary) = enrichment.get("summary").and_then(|v| v.as_str()) {
                        gem_section.push_str(&format!("\n**Summary:** {}", summary));
                    }
                }
            }

            context_parts.push(gem_section);
        }

        Ok(context_parts.join("\n\n"))
    }

    fn label(&self) -> String {
        format!("Project: {}", self.project_title)
    }

    fn session_dir(&self) -> PathBuf {
        let app_data = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("com.jarvis.app")
            .join("projects")
            .join(&self.project_id)
            .join("chat_sessions");
        app_data
    }

    async fn needs_preparation(&self) -> bool {
        // Project context is assembled on the fly from gems — no expensive generation needed
        false
    }
}
```

### `project_agent.rs` — ProjectResearchAgent (NEW)

**File**: `src/agents/project_agent.rs`

**Responsibilities**: Persistent agent that manages research, summarization, and chat for projects. Holds all required providers and a `Chatbot` instance.

```rust
// ProjectResearchAgent — Research, Summarize, and Chat for Projects
//
// This agent manages all project intelligence capabilities:
// - Research: LLM topic generation + web search + gem suggestions
// - Summarize: LLM summary of all project gems
// - Chat: Conversational Q&A over project content via Chatbot + ProjectChatSource

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

use super::chatbot::{Chatbot, ChatMessage};
use super::project_chat::ProjectChatSource;
use crate::gems::GemStore;
use crate::intelligence::provider::IntelProvider;
use crate::intelligence::queue::IntelQueue;
use crate::projects::ProjectStore;
use crate::search::{
    SearchResultProvider, GemSearchResult, WebSearchResult,
};

/// Combined results from both research pipelines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectResearchResults {
    /// External resources found via web search
    pub web_results: Vec<WebSearchResult>,
    /// Existing gems relevant to the project
    pub suggested_gems: Vec<GemSearchResult>,
    /// The search topics the LLM generated
    pub topics_generated: Vec<String>,
}

// ── LLM Prompts ──

const TOPIC_GENERATION_PROMPT: &str = r#"You are a research assistant. Given a project description, suggest 3-5 specific search queries that would find useful resources (academic papers, technical articles, YouTube tutorials).

Rules:
- Return ONLY a JSON array of strings, no other text
- Each query should be specific enough to return targeted results
- Avoid generic queries like "how to learn X" — be precise
- Include a mix of conceptual and practical queries

Example output: ["ECS to Fargate migration networking changes", "Fargate task definition best practices 2025", "AWS Fargate vs ECS EC2 cost comparison"]"#;

const SUMMARIZE_PROMPT: &str = r#"You are a project analyst. Given a project and its collected resources (gems), write a concise executive summary covering:

1. **Project goal** — what this project aims to achieve
2. **Key themes** — the main topics and patterns across the collected resources
3. **Notable findings** — the most important insights from the resources
4. **Gaps** — areas that seem under-researched based on the project objective

Rules:
- Be concise but thorough (aim for 200-400 words)
- Reference specific resources when making claims
- Use markdown formatting
- If there are few or no resources, acknowledge this and suggest next steps"#;

/// Persistent agent for project research, summarization, and chat.
///
/// Registered in Tauri state as Arc<TokioMutex<ProjectResearchAgent>>.
/// All actions can be invoked independently on any project at any time.
pub struct ProjectResearchAgent {
    project_store: Arc<dyn ProjectStore>,
    gem_store: Arc<dyn GemStore>,
    intel_provider: Arc<dyn IntelProvider>,
    search_provider: Arc<dyn SearchResultProvider>,
    intel_queue: Arc<IntelQueue>,
    chatbot: Chatbot,
    /// Maps session_id -> ProjectChatSource for active chat sessions
    chat_sources: HashMap<String, ProjectChatSource>,
}

impl ProjectResearchAgent {
    pub fn new(
        project_store: Arc<dyn ProjectStore>,
        gem_store: Arc<dyn GemStore>,
        intel_provider: Arc<dyn IntelProvider>,
        search_provider: Arc<dyn SearchResultProvider>,
        intel_queue: Arc<IntelQueue>,
    ) -> Self {
        eprintln!("Projects/Research: Agent initialized");
        Self {
            project_store,
            gem_store,
            intel_provider,
            search_provider,
            intel_queue,
            chatbot: Chatbot::new(),
            chat_sources: HashMap::new(),
        }
    }

    // ────────────────────────────────────────────
    // Research
    // ────────────────────────────────────────────

    /// Run research pipelines for a project: LLM topic generation + web search + gem suggestions.
    ///
    /// Returns combined results from both pipelines. Gracefully degrades if web search
    /// is unavailable — gem suggestions still work.
    pub async fn research(&self, project_id: &str) -> Result<ProjectResearchResults, String> {
        eprintln!("Projects/Research: Starting research for project {}", project_id);

        // 1. Load project metadata
        let detail = self.project_store.get(project_id).await?;
        let project = &detail.project;

        // 2. Build context string for LLM
        let mut context_parts = vec![format!("Project: {}", project.title)];
        if let Some(ref desc) = project.description {
            context_parts.push(format!("Description: {}", desc));
        }
        if let Some(ref obj) = project.objective {
            context_parts.push(format!("Objective: {}", obj));
        }
        let context = context_parts.join("\n");
        eprintln!("Projects/Research: Generating topics for '{}'", project.title);

        // 3. LLM generates search topics
        let topics_raw = self.intel_provider.chat(&[
            ("system".to_string(), TOPIC_GENERATION_PROMPT.to_string()),
            ("user".to_string(), context),
        ]).await?;

        // Parse JSON array from LLM response (strip markdown code fences if present)
        let topics_cleaned = topics_raw
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let topics: Vec<String> = serde_json::from_str(topics_cleaned)
            .map_err(|e| format!("Failed to parse LLM topics: {} — raw: {}", e, topics_raw))?;
        eprintln!("Projects/Research: {} topics generated: {:?}", topics.len(), topics);

        // 4. Web search for each topic
        let mut web_results: Vec<WebSearchResult> = Vec::new();
        if self.search_provider.supports_web_search() {
            for topic in &topics {
                eprintln!("Projects/Research: web_search for '{}'", topic);
                match self.search_provider.web_search(topic, 5).await {
                    Ok(results) => {
                        eprintln!("Projects/Research: {} results for '{}'", results.len(), topic);
                        web_results.extend(results);
                    }
                    Err(e) => {
                        eprintln!("Projects/Research: web_search failed for '{}': {}", topic, e);
                        // Continue with remaining topics
                    }
                }
            }
            // Deduplicate by URL
            web_results.sort_by(|a, b| a.url.cmp(&b.url));
            web_results.dedup_by(|a, b| a.url == b.url);
            eprintln!("Projects/Research: {} web results after dedup", web_results.len());
        } else {
            eprintln!("Projects/Research: Web search not available, skipping");
        }

        // 5. Gem search
        let gem_results = self.search_provider.search(&project.title, 20).await?;
        eprintln!("Projects/Research: {} raw gem search results", gem_results.len());

        // Enrich with GemPreview data (same pattern as search_gems command)
        let mut suggested_gems: Vec<GemSearchResult> = Vec::new();
        for result in gem_results {
            if let Ok(Some(gem)) = self.gem_store.get(&result.gem_id).await {
                let tags = gem.ai_enrichment
                    .as_ref()
                    .and_then(|e| e.get("tags"))
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

                let summary = gem.ai_enrichment
                    .as_ref()
                    .and_then(|e| e.get("summary"))
                    .and_then(|v| v.as_str())
                    .map(String::from);

                suggested_gems.push(GemSearchResult {
                    score: result.score,
                    matched_chunk: result.matched_chunk,
                    match_type: result.match_type,
                    id: gem.id,
                    source_type: gem.source_type,
                    source_url: gem.source_url,
                    domain: gem.domain,
                    title: gem.title,
                    author: gem.author,
                    description: gem.description,
                    captured_at: gem.captured_at,
                    tags,
                    summary,
                });
            }
        }
        eprintln!("Projects/Research: {} gems suggested", suggested_gems.len());

        Ok(ProjectResearchResults {
            web_results,
            suggested_gems,
            topics_generated: topics,
        })
    }

    // ────────────────────────────────────────────
    // Summarize
    // ────────────────────────────────────────────

    /// Generate a summary of all gems in a project.
    ///
    /// Assembles all gem content (titles, descriptions, summaries) and asks the LLM
    /// to produce an executive summary covering themes, findings, and gaps.
    pub async fn summarize(&self, project_id: &str) -> Result<String, String> {
        eprintln!("Projects/Research: Starting summarization for project {}", project_id);

        // 1. Load project + gems
        let detail = self.project_store.get(project_id).await?;
        let project = &detail.project;

        if detail.gems.is_empty() {
            return Ok("This project has no gems yet. Add some resources to generate a summary.".to_string());
        }

        // 2. Assemble context from all gems
        let mut context_parts: Vec<String> = Vec::new();
        context_parts.push(format!("Project: {}", project.title));
        if let Some(ref desc) = project.description {
            context_parts.push(format!("Description: {}", desc));
        }
        if let Some(ref obj) = project.objective {
            context_parts.push(format!("Objective: {}", obj));
        }
        context_parts.push(format!("\nResources ({} gems):", detail.gems.len()));

        for gem_preview in &detail.gems {
            let mut gem_text = format!("\n--- {} ---", gem_preview.title);

            if let Ok(Some(full_gem)) = self.gem_store.get(&gem_preview.id).await {
                if let Some(ref desc) = full_gem.description {
                    gem_text.push_str(&format!("\n{}", desc));
                }
                if let Some(ref enrichment) = full_gem.ai_enrichment {
                    if let Some(summary) = enrichment.get("summary").and_then(|v| v.as_str()) {
                        gem_text.push_str(&format!("\nSummary: {}", summary));
                    }
                    if let Some(tags) = enrichment.get("tags").and_then(|v| v.as_array()) {
                        let tag_strs: Vec<&str> = tags.iter().filter_map(|v| v.as_str()).collect();
                        if !tag_strs.is_empty() {
                            gem_text.push_str(&format!("\nTags: {}", tag_strs.join(", ")));
                        }
                    }
                }
            }

            context_parts.push(gem_text);
        }

        let context = context_parts.join("\n");
        eprintln!("Projects/Research: Summarizing {} gems for '{}'", detail.gems.len(), project.title);

        // 3. Ask LLM to summarize
        let summary = self.intel_provider.chat(&[
            ("system".to_string(), SUMMARIZE_PROMPT.to_string()),
            ("user".to_string(), context),
        ]).await?;

        eprintln!("Projects/Research: Summary generated ({} chars)", summary.len());
        Ok(summary)
    }

    // ────────────────────────────────────────────
    // Chat
    // ────────────────────────────────────────────

    /// Start a chat session for a project.
    ///
    /// Creates a ProjectChatSource and starts a Chatbot session.
    /// Returns the session_id for subsequent send_chat_message calls.
    pub async fn start_chat(&mut self, project_id: &str) -> Result<String, String> {
        eprintln!("Projects/Research: Starting chat for project {}", project_id);

        // Load project to get title
        let detail = self.project_store.get(project_id).await?;
        let project_title = detail.project.title.clone();

        // Create chat source
        let source = ProjectChatSource::new(
            project_id.to_string(),
            project_title,
            Arc::clone(&self.project_store),
            Arc::clone(&self.gem_store),
        );

        // Start session via Chatbot
        let session_id = self.chatbot.start_session(&source).await?;

        // Store source for use in send_chat_message
        self.chat_sources.insert(session_id.clone(), source);

        eprintln!("Projects/Research: Chat session started: {}", session_id);
        Ok(session_id)
    }

    /// Send a message in a project chat session.
    ///
    /// Delegates to Chatbot::send_message with the ProjectChatSource as context.
    pub async fn send_chat_message(
        &mut self,
        session_id: &str,
        message: &str,
    ) -> Result<String, String> {
        let source = self.chat_sources.get(session_id)
            .ok_or_else(|| format!("Chat source not found for session {}", session_id))?;

        self.chatbot.send_message(
            session_id,
            message,
            source,
            &self.intel_queue,
        ).await
    }

    /// Get message history for a project chat session.
    pub fn get_chat_history(&self, session_id: &str) -> Result<Vec<ChatMessage>, String> {
        self.chatbot.get_history(session_id)
    }

    /// End a project chat session.
    pub fn end_chat(&mut self, session_id: &str) {
        self.chatbot.end_session(session_id);
        self.chat_sources.remove(session_id);
        eprintln!("Projects/Research: Chat session ended: {}", session_id);
    }
}
```

### `mod.rs` — Agents Module Root (MODIFIED)

**File**: `src/agents/mod.rs`

```rust
pub mod copilot;
pub mod chatable;
pub mod chatbot;
pub mod recording_chat;
pub mod project_chat;
pub mod project_agent;
```

---

## Tauri Commands

### `commands.rs` — Project Agent Commands (MODIFIED)

**File**: `src/projects/commands.rs` (add to existing file)

**Responsibilities**: Thin Tauri command wrappers that delegate to `ProjectResearchAgent`.

```rust
// ── Add these imports to existing commands.rs ──

use tokio::sync::Mutex as TokioMutex;
use crate::agents::project_agent::{ProjectResearchAgent, ProjectResearchResults};
use crate::agents::chatbot::ChatMessage;

// ── New commands ──

/// Run research pipelines for a project.
#[tauri::command]
pub async fn get_project_research(
    project_id: String,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<ProjectResearchResults, String> {
    let agent = agent.lock().await;
    agent.research(&project_id).await
}

/// Generate a summary of all gems in a project.
#[tauri::command]
pub async fn get_project_summary(
    project_id: String,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<String, String> {
    let agent = agent.lock().await;
    agent.summarize(&project_id).await
}

/// Start a chat session for a project.
#[tauri::command]
pub async fn start_project_chat(
    project_id: String,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<String, String> {
    let mut agent = agent.lock().await;
    agent.start_chat(&project_id).await
}

/// Send a message in a project chat session.
#[tauri::command]
pub async fn send_project_chat_message(
    session_id: String,
    message: String,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<String, String> {
    let mut agent = agent.lock().await;
    agent.send_chat_message(&session_id, &message).await
}

/// Get chat history for a project chat session.
#[tauri::command]
pub async fn get_project_chat_history(
    session_id: String,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<Vec<ChatMessage>, String> {
    let agent = agent.lock().await;
    agent.get_chat_history(&session_id)
}

/// End a project chat session.
#[tauri::command]
pub async fn end_project_chat(
    session_id: String,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<(), String> {
    let mut agent = agent.lock().await;
    agent.end_chat(&session_id);
    Ok(())
}
```

---

## Provider & Agent Registration

### In `lib.rs` — Modified Setup

Replace the current search provider initialization block with:

```rust
// ── Search Provider Setup ──

// Read search settings
let (search_enabled, search_accuracy, tavily_api_key) = {
    let manager = app.state::<Arc<RwLock<SettingsManager>>>();
    let settings = manager.read().expect("Failed to acquire settings read lock").get();
    (
        settings.search.semantic_search_enabled,
        settings.search.semantic_search_accuracy,
        settings.search.tavily_api_key.clone(),
    )
};

// Build gem search provider (existing logic, unchanged)
let gem_provider: Arc<dyn SearchResultProvider> = if search_enabled {
    match tauri::async_runtime::block_on(QmdResultProvider::find_qmd_binary()) {
        Some(qmd_path) => {
            let knowledge_path = app.path().app_data_dir()
                .expect("Failed to get app data dir")
                .join("knowledge");
            let qmd = QmdResultProvider::new(qmd_path.clone(), knowledge_path, search_accuracy);

            let availability = tauri::async_runtime::block_on(qmd.check_availability());
            if availability.available {
                eprintln!("Search: Using QMD semantic search provider ({})", qmd_path.display());
                Arc::new(qmd)
            } else {
                eprintln!("Search: QMD unavailable ({}), falling back to FTS5",
                    availability.reason.unwrap_or_else(|| "unknown".to_string()));
                Arc::new(FtsResultProvider::new(gem_store_arc.clone()))
            }
        }
        None => {
            eprintln!("Search: QMD binary not found, falling back to FTS5");
            Arc::new(FtsResultProvider::new(gem_store_arc.clone()))
        }
    }
} else {
    eprintln!("Search: Using FTS5 keyword search provider (default)");
    Arc::new(FtsResultProvider::new(gem_store_arc.clone()))
};

// Build web search provider from existing settings (NEW)
let web_provider: Option<Arc<dyn SearchResultProvider>> = tavily_api_key
    .as_ref()
    .filter(|k| !k.is_empty())
    .map(|api_key| {
        eprintln!("Search: Tavily web search enabled");
        Arc::new(search::TavilyProvider::new(api_key.clone())) as Arc<dyn SearchResultProvider>
    });

if web_provider.is_none() {
    eprintln!("Search: Tavily web search disabled (no API key in settings)");
}

// Wrap in composite (NEW)
let search_provider: Arc<dyn SearchResultProvider> = Arc::new(
    search::CompositeSearchProvider::new(gem_provider, web_provider)
);
app.manage(search_provider.clone());
```

Add after the search provider setup:

```rust
// ── Project Research Agent Setup (NEW) ──

let project_agent = agents::project_agent::ProjectResearchAgent::new(
    project_store_arc.clone(),    // Arc<dyn ProjectStore>
    gem_store_arc.clone(),        // Arc<dyn GemStore>
    intel_provider_arc.clone(),   // Arc<dyn IntelProvider>
    search_provider.clone(),      // Arc<dyn SearchResultProvider>
    intel_queue_arc.clone(),      // Arc<IntelQueue>
);
app.manage(Arc::new(tokio::sync::Mutex::new(project_agent)));
```

### Register Commands in `generate_handler!`

```rust
// Add to generate_handler![] in lib.rs:
projects::commands::get_project_research,
projects::commands::get_project_summary,
projects::commands::start_project_chat,
projects::commands::send_project_chat_message,
projects::commands::get_project_chat_history,
projects::commands::end_project_chat,
```

---

## Frontend Changes

### TypeScript Types

Add to `src/state/types.ts`:

```typescript
/** A search result from the web (not a gem) */
export interface WebSearchResult {
  title: string;
  url: string;
  snippet: string;
  source_type: 'Paper' | 'Article' | 'Video' | 'Other';
  domain: string;
  published_date: string | null;
}

/** Combined results from the project research pipelines */
export interface ProjectResearchResults {
  web_results: WebSearchResult[];
  suggested_gems: GemSearchResult[];
  topics_generated: string[];
}
```

### ProjectResearchPanel Component

```tsx
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-shell';
import type { ProjectResearchResults, WebSearchResult, GemSearchResult } from '../state/types';

interface ProjectResearchPanelProps {
  projectId: string;
  onGemsAdded: () => void;
}

export function ProjectResearchPanel({ projectId, onGemsAdded }: ProjectResearchPanelProps) {
  const [loading, setLoading] = useState(true);
  const [results, setResults] = useState<ProjectResearchResults | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [addedGemIds, setAddedGemIds] = useState<Set<string>>(new Set());

  useEffect(() => {
    setLoading(true);
    setError(null);
    invoke<ProjectResearchResults>('get_project_research', { projectId })
      .then(setResults)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [projectId]);

  const handleOpenUrl = (url: string) => {
    open(url);
  };

  const handleAddGem = async (gemId: string) => {
    try {
      await invoke('add_gems_to_project', { projectId, gemIds: [gemId] });
      setAddedGemIds(prev => new Set(prev).add(gemId));
      onGemsAdded();
    } catch (err) {
      console.error('Failed to add gem to project:', err);
    }
  };

  if (loading) {
    return (
      <div className="research-loading">
        <div className="research-spinner" />
        <p>Researching your project...</p>
        <p className="research-subtext">
          Finding relevant articles, papers, videos, and gems...
        </p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="research-error">
        <p>Research failed: {error}</p>
        <p className="research-subtext">You can still add gems manually.</p>
      </div>
    );
  }

  if (!results) return null;

  return (
    <div className="research-panel">
      {/* Web Results Section */}
      <div className="research-section">
        <h4 className="research-section-title">Suggested from the web</h4>
        {results.web_results.length === 0 ? (
          <p className="research-empty">
            {results.topics_generated.length > 0
              ? 'No web resources found. Try refining your project description.'
              : 'Web search not configured.'}
          </p>
        ) : (
          <div className="research-web-results">
            {results.web_results.map((result, i) => (
              <div
                key={i}
                className="web-result-card"
                onClick={() => handleOpenUrl(result.url)}
              >
                <div className="web-result-header">
                  <span className={`source-type-badge source-${result.source_type.toLowerCase()}`}>
                    {result.source_type}
                  </span>
                  <span className="web-result-domain">{result.domain}</span>
                </div>
                <div className="web-result-title">{result.title}</div>
                <div className="web-result-snippet">{result.snippet}</div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Gem Suggestions Section */}
      <div className="research-section">
        <h4 className="research-section-title">From your gem library</h4>
        {results.suggested_gems.length === 0 ? (
          <p className="research-empty">No matching gems in your library yet.</p>
        ) : (
          <div className="research-gem-results">
            {results.suggested_gems.map((gem) => (
              <div key={gem.id} className="research-gem-card">
                <div className="gem-card">
                  <div className="gem-card-header">
                    <span className={`source-badge ${gem.source_type.toLowerCase()}`}>
                      {gem.source_type}
                    </span>
                    <span className="gem-date">
                      {new Date(gem.captured_at).toLocaleDateString()}
                    </span>
                  </div>
                  <div className="gem-title">{gem.title}</div>
                  {gem.description && (
                    <div className="gem-description">{gem.description}</div>
                  )}
                </div>
                <button
                  className={`research-add-gem ${addedGemIds.has(gem.id) ? 'added' : ''}`}
                  onClick={() => handleAddGem(gem.id)}
                  disabled={addedGemIds.has(gem.id)}
                >
                  {addedGemIds.has(gem.id) ? 'Added' : '+ Add'}
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
```

### ProjectGemList Integration

Add to the existing `ProjectGemList` component:

```tsx
// In ProjectGemList state
const [showResearch, setShowResearch] = useState(false);
const [projectSummary, setProjectSummary] = useState<string | null>(null);
const [summarizing, setSummarizing] = useState(false);

// Detect newly created project (0 gems) — auto-show research
useEffect(() => {
  if (detail && detail.gem_count === 0) {
    setShowResearch(true);
  }
}, [detail?.project.id]);

// Summarize handler
const handleSummarize = async () => {
  setSummarizing(true);
  try {
    const summary = await invoke<string>('get_project_summary', { projectId });
    setProjectSummary(summary);
  } catch (e) {
    console.error('Summarization failed:', e);
  } finally {
    setSummarizing(false);
  }
};

// In the toolbar (alongside search input and "+ Add Gems")
<button
  className="action-button"
  onClick={() => setShowResearch(true)}
  disabled={showResearch}
>
  Research
</button>
<button
  className="action-button"
  onClick={handleSummarize}
  disabled={summarizing}
>
  {summarizing ? 'Summarizing...' : 'Summarize'}
</button>

// In the gem list area, above the gems
{projectSummary && (
  <div className="project-summary-panel">
    <div className="summary-header">
      <h4>Project Summary</h4>
      <button onClick={() => setProjectSummary(null)}>Dismiss</button>
    </div>
    <div className="summary-content">{projectSummary}</div>
  </div>
)}
{showResearch && (
  <ProjectResearchPanel
    projectId={projectId}
    onGemsAdded={() => {
      loadProject(projectId);
      onProjectsChanged();
    }}
  />
)}
```

---

## CSS Additions

Add to `App.css`:

```css
/* Research Panel */
.research-panel {
  padding: 16px;
  border-bottom: 1px solid var(--border-color, #333);
}

.research-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 48px 16px;
  color: var(--text-secondary, #aaa);
}

.research-spinner {
  width: 32px;
  height: 32px;
  border: 3px solid var(--border-color, #333);
  border-top-color: var(--accent-color, #3b82f6);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin-bottom: 16px;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.research-subtext {
  font-size: 12px;
  color: var(--text-muted, #666);
  margin-top: 4px;
}

.research-error {
  padding: 16px;
  color: var(--error-color, #ef4444);
  font-size: 13px;
}

/* Research Sections */
.research-section {
  margin-bottom: 20px;
}

.research-section-title {
  font-size: 13px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted, #888);
  margin: 0 0 12px 0;
}

.research-empty {
  font-size: 13px;
  color: var(--text-muted, #666);
  font-style: italic;
}

/* Web Result Cards */
.web-result-card {
  padding: 12px;
  border-radius: 6px;
  cursor: pointer;
  margin-bottom: 8px;
  border: 1px solid var(--border-color, #333);
}

.web-result-card:hover {
  background: var(--hover-bg, rgba(255, 255, 255, 0.05));
  border-color: var(--accent-color, #3b82f6);
}

.web-result-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.source-type-badge {
  display: inline-block;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
}

.source-paper { background: rgba(168, 85, 247, 0.15); color: #c084fc; }
.source-article { background: rgba(59, 130, 246, 0.15); color: #60a5fa; }
.source-video { background: rgba(239, 68, 68, 0.15); color: #f87171; }
.source-other { background: rgba(107, 114, 128, 0.15); color: #9ca3af; }

.web-result-domain {
  font-size: 11px;
  color: var(--text-muted, #888);
}

.web-result-title {
  font-size: 14px;
  font-weight: 500;
  margin-bottom: 4px;
}

.web-result-snippet {
  font-size: 12px;
  color: var(--text-secondary, #aaa);
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

/* Gem Suggestion Cards */
.research-gem-card {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.research-gem-card .gem-card {
  flex: 1;
}

.research-add-gem {
  flex-shrink: 0;
  padding: 6px 12px;
  border-radius: 4px;
  border: 1px solid var(--accent-color, #3b82f6);
  background: transparent;
  color: var(--accent-color, #3b82f6);
  font-size: 12px;
  cursor: pointer;
}

.research-add-gem:hover {
  background: rgba(59, 130, 246, 0.1);
}

.research-add-gem.added {
  border-color: var(--text-muted, #666);
  color: var(--text-muted, #666);
  cursor: default;
}

/* Project Summary Panel */
.project-summary-panel {
  padding: 16px;
  margin-bottom: 16px;
  border: 1px solid var(--border-color, #333);
  border-radius: 6px;
  background: var(--card-bg, rgba(255, 255, 255, 0.02));
}

.summary-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.summary-header h4 {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted, #888);
}

.summary-header button {
  background: none;
  border: none;
  color: var(--text-muted, #666);
  cursor: pointer;
  font-size: 12px;
}

.summary-content {
  font-size: 13px;
  line-height: 1.6;
  color: var(--text-secondary, #ccc);
  white-space: pre-wrap;
}
```

---

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| No Tavily API key | `web_provider` is `None`. `CompositeSearchProvider` returns empty for `web_search`. Gem suggestions still work. |
| Tavily API rate limited | Individual topic search returns error, logged, skipped. Remaining topics continue. |
| Tavily API key invalid | HTTP 401 from Tavily. Logged as error. `web_results` empty in response. |
| LLM returns non-JSON | `serde_json::from_str` fails. Command returns error. Frontend shows error message. |
| LLM returns markdown-wrapped JSON | Code fence stripping handles `` ```json [...] ``` `` wrapping. |
| LLM unavailable (IntelProvider not ready) | `chat()` returns error. Command returns error. Frontend shows error message. |
| No gems in library | `search_provider.search()` returns empty. `suggested_gems` empty. Web results still show. |
| Project has no description or objective | Context is just the title. LLM generates topics from title alone. |
| Duplicate URLs across topics | `dedup_by` on sorted URLs removes duplicates before returning. |
| App started without Tavily key, key added later | Requires restart. `CompositeSearchProvider` is built at startup. Hot-reload is out of scope. |
| Summarize with 0 gems | Returns a friendly message suggesting the user add gems first. |
| Chat session with 0 gems | Context will be project metadata only. LLM can still answer questions about the project goal. |
| Multiple concurrent chats on different projects | `Chatbot` manages multiple sessions via `HashMap<session_id, ChatSession>`. Each `ProjectChatSource` is independent. |
| Chat session after gems added/removed | `get_context()` is called fresh on every message. New gems appear, removed gems disappear. |

---

## Testing Strategy

### Unit Tests

**`tavily_provider.rs` tests**:
- `classify_source_type`: youtube.com -> Video, arxiv.org -> Paper, medium.com -> Article, random.com -> Other
- `extract_domain`: strips protocol, www prefix, path
- `supports_web_search`: returns true
- `check_availability`: returns true when key non-empty, false when empty

**`composite_provider.rs` tests**:
- `search` delegates to gem_provider
- `web_search` delegates to web_provider when present
- `web_search` returns empty when web_provider is None
- `supports_web_search` returns false when web_provider is None
- `index_gem`, `remove_gem`, `reindex_all` delegate to gem_provider

**`project_chat.rs` tests**:
- `label()` returns "Project: {title}"
- `needs_preparation()` returns false
- `get_context()` assembles project metadata + gem content

**`project_agent.rs` tests**:
- `research()` returns combined web + gem results
- `research()` handles missing web provider gracefully
- `summarize()` returns summary string
- `summarize()` handles empty project (0 gems)
- `start_chat()` + `send_chat_message()` + `end_chat()` lifecycle

### Integration Tests

- Create project -> run research -> verify both `web_results` and `suggested_gems` are populated
- Run research without Tavily key -> verify `web_results` empty, `suggested_gems` still work
- Add gems to project -> run summarize -> verify summary references the gems
- Start project chat -> send message -> verify response references project content
- Verify `search_gems` command still works identically through `CompositeSearchProvider`
- Verify `rebuild_search_index` still works through `CompositeSearchProvider`

### Manual Testing Checklist

- [ ] Create project with title only -> research panel auto-shows with spinner
- [ ] Verify LLM generates reasonable topics for the project title
- [ ] Verify web results show with source type badges (Paper, Article, Video)
- [ ] Click web result -> opens in system browser
- [ ] Verify gem suggestions appear with "Add" buttons
- [ ] Click "Add" on a gem -> button changes to "Added", gem appears in project
- [ ] Click "Research" button on existing project -> research panel shows
- [ ] Click "Summarize" button on project with gems -> summary panel appears
- [ ] Click "Summarize" on empty project -> shows "no gems yet" message
- [ ] Remove Tavily API key from settings -> restart -> web section shows "Web search not configured"
- [ ] Existing gem search (GemsPanel search bar) still works unchanged

---

## Summary

This design adds a **Project Research Agent** as a persistent, reusable agent that manages research (web search + gem suggestions), summarization, and chat for any project. Key architecture:

- **Search layer**: `SearchResultProvider` extended with `web_search`/`supports_web_search` default methods. `TavilyProvider` for web search, `CompositeSearchProvider` wrapping gem + web providers behind the existing `Arc<dyn SearchResultProvider>`.
- **Agent layer**: `ProjectResearchAgent` holds all providers + a `Chatbot` instance. Exposes `research()`, `summarize()`, and chat lifecycle methods. Registered as `Arc<TokioMutex<ProjectResearchAgent>>` in Tauri state.
- **Chat layer**: `ProjectChatSource` implements `Chatable` (assembles project gem content as context). `Chatbot` handles sessions, history, and LLM prompting unchanged.
- **Command layer**: Six thin Tauri commands delegating to the agent: `get_project_research`, `get_project_summary`, `start_project_chat`, `send_project_chat_message`, `get_project_chat_history`, `end_project_chat`.

No new traits for search. No new dependencies. No new settings infrastructure. All existing commands work unchanged through the `CompositeSearchProvider`. The agent is independent of project creation — it can be invoked on any project at any time.
