# Project Research Assistant — Design Document

## Overview

This design adds a **Conversational Research Agent** — a chat-first assistant that lives in the right panel when viewing a project. The user's first interaction with any project is a conversation: the agent suggests research topics, the user refines them through natural dialogue, and research results flow back as rich messages in the chat.

### Design Goals

1. **Chat-first**: Research is a conversation, not a form submission. The agent lives in the RightPanel as a persistent collaborator.
2. **No new search traits**: Everything flows through `SearchResultProvider`. Existing providers unchanged.
3. **Reuse Chatbot engine**: `ProjectChatSource` implements `Chatable`, the generic `Chatbot` handles sessions. The research chat is a specialized chat with a richer system prompt.
4. **Pluggable web search**: Tavily today, swap to Brave/SerpAPI later by implementing one method.
5. **Graceful degradation**: No Tavily key? Gem suggestions still work. API down? Skip web results, don't fail.

### Key Design Decisions

- **Default methods, not a new trait**: `web_search` and `supports_web_search` are default methods on `SearchResultProvider`. FTS/QMD inherit no-ops. `TavilyProvider` overrides them.
- **CompositeSearchProvider**: Wraps gem provider + optional web provider. Registered as the single `Arc<dyn SearchResultProvider>`. All existing commands work unchanged.
- **Two-phase research flow**: `suggest_topics()` generates the opening message, `run_research(topics)` executes on user-curated topics. The user controls what gets searched.
- **RightPanel integration**: When `activeNav === 'projects'`, the RightPanel shows a "Research" tab (default) alongside the existing "Detail" tab (when a gem is selected). Follows the same tab pattern as recordings (Details | Chat).
- **Rich chat messages**: Agent responses containing research results include structured JSON blocks that the frontend parses and renders as web result cards and gem suggestion cards inline in the chat.
- **Sequential web searches**: One Tavily call per topic, sequentially. Keeps rate limits simple.
- **reqwest already in Cargo.toml**: No dependency changes needed.

### User Experience Flow

```
1. User creates or opens a project
   → RightPanel shows "Research" tab with chat interface
   → Agent auto-sends opening message:
     "I see your project is about {title}. Here are some research topics I'd suggest:
      1. {topic1}
      2. {topic2}
      3. {topic3}
      Want me to search for these? You can add your own topics or ask me to modify these."

2. User responds naturally:
   "Also search for kubernetes security scanning tools"
   "Drop topic 2"
   "Go ahead and search"

3. Agent executes research on user-curated topics
   → Response contains structured result blocks
   → Frontend renders web result cards (clickable → opens URL)
   → Frontend renders gem suggestion cards (with "Add to Project" button)

4. Ongoing conversation:
   "Summarize what we've found"     → runs summarization pipeline
   "Search for more about topic X"  → runs additional web search
   "What does gem Y say about Z?"   → answers from project context
```

### Layout

```
┌──────────┬───────────────────────────┬──────────────────────────┐
│ LeftNav  │     Center Content        │     RightPanel           │
│          │                           │                          │
│ ...      │  ProjectsContainer        │  Tab: [Research][Detail] │
│ Projects │  ┌──────────┬────────────┐│                          │
│ ...      │  │ Project  │ Project    ││  Agent: I see your       │
│          │  │ List     │ GemList    ││  project is about...     │
│          │  │          │            ││                          │
│          │  │ • Proj A │ gems...    ││  1. topic one            │
│          │  │ • Proj B │            ││  2. topic two            │
│          │  │          │            ││  3. topic three          │
│          │  │          │            ││                          │
│          │  │          │            ││  User: Go ahead          │
│          │  │          │            ││                          │
│          │  │          │            ││  Agent: Found results... │
│          │  │          │            ││  [web cards]             │
│          │  │          │            ││  [gem cards]             │
│          │  │          │            ││                          │
│          │  │          │            ││  ┌────────────────────┐  │
│          │  │          │            ││  │ Type a message.. ↵ │  │
│          │  └──────────┴────────────┘│  └────────────────────┘  │
└──────────┴───────────────────────────┴──────────────────────────┘
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
├── project_agent.rs       — ProjectResearchAgent (NEW: suggest_topics + run_research + summarize + chat)
└── mod.rs                 — Module root (MODIFIED: +project_chat, +project_agent)

src/projects/
├── commands.rs            — (MODIFIED: +agent Tauri commands)
└── ...                    — (everything else unchanged)

Frontend:
├── components/
│   ├── ProjectResearchChat.tsx  — NEW: Chat interface with rich message rendering
│   ├── RightPanel.tsx           — MODIFIED: "Research" tab for projects
│   └── ProjectsContainer.tsx    — MODIFIED: passes project state to RightPanel
├── state/types.ts               — MODIFIED: +WebSearchResult, +ProjectResearchResults
└── App.css                      — MODIFIED: +research chat styles
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
         │  .suggest_topics(id) → topics         │
         │  .run_research(id, topics) → results  │
         │  .summarize(id) → summary string      │
         │  .start_chat(id) → session_id         │
         │  .send_chat_message(sid, msg) → resp  │
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

### Operational Flow — Two-Phase Research

```
Phase A: Topic Suggestion (auto on chat open)
──────────────────────────────────────────────
Frontend calls invoke('suggest_project_topics', { projectId })
  → ProjectResearchAgent::suggest_topics(project_id)
    → 1. ProjectStore::get(project_id) — load project metadata
    → 2. IntelProvider::chat(TOPIC_PROMPT, context) — generate 3-5 topics
    → 3. Parse JSON array response (strip markdown code fences)
    → Return Vec<String> of topics
  → Frontend renders topics as agent's opening message

Phase B: Execute Research (on user confirmation)
─────────────────────────────────────────────────
Frontend calls invoke('run_project_research', { projectId, topics })
  → ProjectResearchAgent::run_research(project_id, topics)
    → 1. For each topic: SearchResultProvider::web_search(topic, 5) — via Composite → Tavily
    → 2. Deduplicate web results by URL
    → 3. SearchResultProvider::search(project.title, 20) — gem suggestions via Composite → QMD/FTS
    → 4. Enrich gems with GemPreview data
    → Return ProjectResearchResults { web_results, suggested_gems, topics_searched }
  → Frontend renders results as rich cards in chat
```

### Operational Flow — Summarize

```
User says "summarize" or clicks Summarize button
  → invoke('get_project_summary', { projectId })
  → ProjectResearchAgent::summarize(project_id)
    → 1. ProjectStore::get(project_id) — load project + gem list
    → 2. For each gem: GemStore::get(gem_id) — load titles, descriptions, summaries
    → 3. Assemble context: project metadata + all gem content
    → 4. IntelProvider::chat(SUMMARY_PROMPT, context) — generate summary
    → Return summary String
```

### Operational Flow — Chat (Q&A over project content)

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

## Search Infrastructure (Phases 1-2 — ALREADY BUILT)

Phases 1-2 are complete and compiled. The following modules exist and work:

### `provider.rs` — Trait Extension (BUILT)

**File**: `src/search/provider.rs`

Added `WebSourceType` enum, `WebSearchResult` struct, and two default methods (`web_search`, `supports_web_search`) to `SearchResultProvider`. FTS/QMD providers unchanged — they inherit defaults.

### `tavily_provider.rs` — Tavily Web Search (BUILT)

**File**: `src/search/tavily_provider.rs`

`TavilyProvider` implementing `SearchResultProvider::web_search` via Tavily Search API. Classifies results by domain (Video/Paper/Article/Other). No-ops for gem methods.

### `composite_provider.rs` — Delegating Wrapper (BUILT)

**File**: `src/search/composite_provider.rs`

`CompositeSearchProvider` wrapping gem provider + optional web provider behind single `Arc<dyn SearchResultProvider>`.

### `lib.rs` — Provider Registration (BUILT)

Reads `tavily_api_key` from settings, builds optional `TavilyProvider`, wraps in `CompositeSearchProvider`, registered in Tauri state.

---

## Agent Module (Phase 3 — TO BUILD)

### `project_chat.rs` — ProjectChatSource (NEW)

**File**: `src/agents/project_chat.rs`

**Responsibilities**: Implement `Chatable` for projects. Assembles project gem content as context for the `Chatbot` engine. Follows the `RecordingChatSource` pattern.

```rust
// ProjectChatSource — Project Conforms to Chatable

use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

use super::chatable::Chatable;
use crate::gems::GemStore;
use crate::intelligence::queue::IntelQueue;
use crate::projects::ProjectStore;

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
        Self { project_id, project_title, project_store, gem_store }
    }
}

#[async_trait]
impl Chatable for ProjectChatSource {
    async fn get_context(&self, _intel_queue: &IntelQueue) -> Result<String, String> {
        let detail = self.project_store.get(&self.project_id).await?;
        let project = &detail.project;

        let mut context_parts: Vec<String> = Vec::new();
        context_parts.push(format!("# Project: {}", project.title));

        if let Some(ref desc) = project.description {
            context_parts.push(format!("**Description:** {}", desc));
        }
        if let Some(ref obj) = project.objective {
            context_parts.push(format!("**Objective:** {}", obj));
        }

        context_parts.push(format!("\n## Gems ({} total)\n", detail.gems.len()));

        for gem_preview in &detail.gems {
            let mut gem_section = format!("### {}", gem_preview.title);

            if let Ok(Some(full_gem)) = self.gem_store.get(&gem_preview.id).await {
                if let Some(ref desc) = full_gem.description {
                    gem_section.push_str(&format!("\n{}", desc));
                }
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
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("com.jarvis.app")
            .join("projects")
            .join(&self.project_id)
            .join("chat_sessions")
    }

    async fn needs_preparation(&self) -> bool {
        false
    }
}
```

### `project_agent.rs` — ProjectResearchAgent (NEW)

**File**: `src/agents/project_agent.rs`

**Responsibilities**: Persistent agent with two-phase research (suggest → execute), summarization, and chat. The key difference from the original design: `research()` is split into `suggest_topics()` and `run_research(topics)`, giving the user control over what gets searched.

```rust
// ProjectResearchAgent — Research, Summarize, and Chat for Projects

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
    pub web_results: Vec<WebSearchResult>,
    pub suggested_gems: Vec<GemSearchResult>,
    /// The topics that were actually searched (user-curated)
    pub topics_searched: Vec<String>,
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

pub struct ProjectResearchAgent {
    project_store: Arc<dyn ProjectStore>,
    gem_store: Arc<dyn GemStore>,
    intel_provider: Arc<dyn IntelProvider>,
    search_provider: Arc<dyn SearchResultProvider>,
    intel_queue: Arc<IntelQueue>,
    chatbot: Chatbot,
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
    // Phase A: Topic Suggestion
    // ────────────────────────────────────────────

    /// Generate research topic suggestions for a project.
    ///
    /// Called when the research chat opens. Returns 3-5 topic strings that the
    /// frontend renders as the agent's opening message. The user can then
    /// refine these before triggering the actual search.
    pub async fn suggest_topics(&self, project_id: &str) -> Result<Vec<String>, String> {
        eprintln!("Projects/Research: Suggesting topics for project {}", project_id);

        let detail = self.project_store.get(project_id).await?;
        let project = &detail.project;

        // Build context string for LLM
        let mut context_parts = vec![format!("Project: {}", project.title)];
        if let Some(ref desc) = project.description {
            context_parts.push(format!("Description: {}", desc));
        }
        if let Some(ref obj) = project.objective {
            context_parts.push(format!("Objective: {}", obj));
        }
        let context = context_parts.join("\n");

        // LLM generates topics
        let topics_raw = self.intel_provider.chat(&[
            ("system".to_string(), TOPIC_GENERATION_PROMPT.to_string()),
            ("user".to_string(), context),
        ]).await?;

        // Parse JSON array (strip markdown code fences if present)
        let topics_cleaned = topics_raw
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let topics: Vec<String> = serde_json::from_str(topics_cleaned)
            .map_err(|e| format!("Failed to parse LLM topics: {} — raw: {}", e, topics_raw))?;

        eprintln!("Projects/Research: {} topics suggested: {:?}", topics.len(), topics);
        Ok(topics)
    }

    // ────────────────────────────────────────────
    // Phase B: Execute Research
    // ────────────────────────────────────────────

    /// Execute research on user-curated topics.
    ///
    /// Runs web search for each topic (if Tavily available), deduplicates,
    /// then searches for relevant gems. Returns combined results.
    pub async fn run_research(
        &self,
        project_id: &str,
        topics: Vec<String>,
    ) -> Result<ProjectResearchResults, String> {
        eprintln!("Projects/Research: Running research for project {} with {} topics", project_id, topics.len());

        let detail = self.project_store.get(project_id).await?;
        let project = &detail.project;

        // Web search for each topic
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

        // Gem search — find existing gems relevant to the project
        let gem_results = self.search_provider.search(&project.title, 20).await?;
        eprintln!("Projects/Research: {} raw gem search results", gem_results.len());

        // Enrich with full gem data
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
            topics_searched: topics,
        })
    }

    // ────────────────────────────────────────────
    // Summarize
    // ────────────────────────────────────────────

    pub async fn summarize(&self, project_id: &str) -> Result<String, String> {
        eprintln!("Projects/Research: Starting summarization for project {}", project_id);

        let detail = self.project_store.get(project_id).await?;
        let project = &detail.project;

        if detail.gems.is_empty() {
            return Ok("This project has no gems yet. Add some resources to generate a summary.".to_string());
        }

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

    pub async fn start_chat(&mut self, project_id: &str) -> Result<String, String> {
        eprintln!("Projects/Research: Starting chat for project {}", project_id);

        let detail = self.project_store.get(project_id).await?;
        let project_title = detail.project.title.clone();

        let source = ProjectChatSource::new(
            project_id.to_string(),
            project_title,
            Arc::clone(&self.project_store),
            Arc::clone(&self.gem_store),
        );

        let session_id = self.chatbot.start_session(&source).await?;
        self.chat_sources.insert(session_id.clone(), source);

        eprintln!("Projects/Research: Chat session started: {}", session_id);
        Ok(session_id)
    }

    pub async fn send_chat_message(
        &mut self,
        session_id: &str,
        message: &str,
    ) -> Result<String, String> {
        let source = self.chat_sources.get(session_id)
            .ok_or_else(|| format!("Chat source not found for session {}", session_id))?;

        self.chatbot.send_message(session_id, message, source, &self.intel_queue).await
    }

    pub fn get_chat_history(&self, session_id: &str) -> Result<Vec<ChatMessage>, String> {
        self.chatbot.get_history(session_id)
    }

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

## Tauri Commands (Phase 4)

### `commands.rs` — Project Agent Commands (MODIFIED)

**File**: `src/projects/commands.rs` (add to existing file)

```rust
// ── Add these imports to existing commands.rs ──

use tokio::sync::Mutex as TokioMutex;
use crate::agents::project_agent::{ProjectResearchAgent, ProjectResearchResults};
use crate::agents::chatbot::ChatMessage;

// ── New commands ──

/// Suggest research topics for a project (Phase A of research flow).
#[tauri::command]
pub async fn suggest_project_topics(
    project_id: String,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<Vec<String>, String> {
    let agent = agent.lock().await;
    agent.suggest_topics(&project_id).await
}

/// Execute research on user-curated topics (Phase B of research flow).
#[tauri::command]
pub async fn run_project_research(
    project_id: String,
    topics: Vec<String>,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<ProjectResearchResults, String> {
    let agent = agent.lock().await;
    agent.run_research(&project_id, topics).await
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

### Provider & Agent Registration in `lib.rs`

Add after existing search provider setup (line ~350):

```rust
// ── Project Research Agent Setup (NEW) ──
// Clone search_provider BEFORE app.manage consumes it
let search_provider_for_agent = search_provider.clone();
app.manage(search_provider);

let project_agent = agents::project_agent::ProjectResearchAgent::new(
    project_store_arc.clone(),
    gem_store_arc.clone(),
    intel_provider.clone(),       // Arc<dyn IntelProvider>
    search_provider_for_agent,    // Arc<dyn SearchResultProvider>
    intel_queue_arc.clone(),      // Arc<IntelQueue>
);
app.manage(Arc::new(tokio::sync::Mutex::new(project_agent)));
```

**Note on `intel_queue`:** Currently `intel_queue` is created as a bare `IntelQueue` and registered via `app.manage(intel_queue)`. For the agent to hold an `Arc<IntelQueue>`, we need to wrap it in `Arc` before registration. Change:
```rust
// Before (current):
let intel_queue = tauri::async_runtime::block_on(async {
    intelligence::IntelQueue::new(intel_provider.clone())
});
app.manage(intel_queue);

// After:
let intel_queue = tauri::async_runtime::block_on(async {
    intelligence::IntelQueue::new(intel_provider.clone())
});
let intel_queue_arc = Arc::new(intel_queue);
app.manage(intel_queue_arc.clone());
```

### Register Commands in `generate_handler!`

```rust
// Add to generate_handler![] in lib.rs:
projects::commands::suggest_project_topics,
projects::commands::run_project_research,
projects::commands::get_project_summary,
projects::commands::start_project_chat,
projects::commands::send_project_chat_message,
projects::commands::get_project_chat_history,
projects::commands::end_project_chat,
```

---

## Frontend Changes (Phases 5-7)

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
  topics_searched: string[];
}
```

### ProjectResearchChat Component (NEW)

**File**: `src/components/ProjectResearchChat.tsx`

This is the main new component — a chat interface with rich message rendering for research results. It follows the existing `ChatPanel.tsx` pattern but adds:
1. Auto-suggests topics on mount (agent's opening message)
2. Renders web result cards and gem suggestion cards inline in chat
3. Handles "Run Research" action with user-curated topics

**Message types in the chat:**
- **Text messages** — standard user/assistant text bubbles (same CSS as `ChatPanel`)
- **Topic suggestion messages** — assistant bubble with numbered topic chips and remove buttons + "Search" button
- **Research result messages** — assistant bubble with embedded web cards and gem cards
- **System messages** — small centered text for actions like "Removed topic: X"

```tsx
import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-shell';
import type { ProjectResearchResults, WebSearchResult, GemSearchResult } from '../state/types';

interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  researchResults?: ProjectResearchResults;
  suggestedTopics?: string[];
}

interface ProjectResearchChatProps {
  projectId: string;
  projectTitle: string;
  onGemsAdded?: () => void;
}

export function ProjectResearchChat({
  projectId,
  projectTitle,
  onGemsAdded,
}: ProjectResearchChatProps) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const [topics, setTopics] = useState<string[]>([]);
  const [addedGemIds, setAddedGemIds] = useState<Set<string>>(new Set());
  const [initializing, setInitializing] = useState(true);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to latest message
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, loading]);

  // Auto-suggest topics on mount
  useEffect(() => {
    let cancelled = false;
    const suggestTopics = async () => {
      setInitializing(true);
      try {
        const suggested = await invoke<string[]>('suggest_project_topics', { projectId });
        if (cancelled) return;
        setTopics(suggested);
        setMessages([{
          role: 'assistant',
          content: `I see your project is about **${projectTitle}**. Here are some research topics I'd suggest:`,
          suggestedTopics: suggested,
        }]);
      } catch (err) {
        if (cancelled) return;
        setMessages([{
          role: 'assistant',
          content: `I couldn't generate topics: ${err}. You can type your own research topics below.`,
        }]);
      } finally {
        if (!cancelled) setInitializing(false);
      }
    };
    suggestTopics();
    return () => { cancelled = true; };
  }, [projectId, projectTitle]);

  const handleRunResearch = useCallback(async (researchTopics: string[]) => {
    if (researchTopics.length === 0) return;

    setLoading(true);
    setMessages(prev => [
      ...prev,
      { role: 'user', content: `Search for: ${researchTopics.join(', ')}` },
      { role: 'assistant', content: `Searching ${researchTopics.length} topics...` },
    ]);

    try {
      const results = await invoke<ProjectResearchResults>('run_project_research', {
        projectId,
        topics: researchTopics,
      });

      // Replace the "Searching..." placeholder with actual results
      setMessages(prev => {
        const updated = [...prev];
        updated[updated.length - 1] = {
          role: 'assistant',
          content: `Found ${results.web_results.length} web resources and ${results.suggested_gems.length} matching gems from your library.`,
          researchResults: results,
        };
        return updated;
      });
    } catch (err) {
      setMessages(prev => {
        const updated = [...prev];
        updated[updated.length - 1] = {
          role: 'assistant',
          content: `Research failed: ${err}. You can try again or refine your topics.`,
        };
        return updated;
      });
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  const handleSendMessage = async () => {
    if (!input.trim() || loading) return;
    const userMessage = input.trim();
    setInput('');

    // Simple intent detection (v1): keywords trigger actions
    const lower = userMessage.toLowerCase();

    if (lower.includes('search') || lower.includes('go ahead') || lower.includes('find')) {
      await handleRunResearch(topics);
      return;
    }

    if (lower.includes('summarize') || lower.includes('summary')) {
      setLoading(true);
      setMessages(prev => [
        ...prev,
        { role: 'user', content: userMessage },
        { role: 'assistant', content: 'Summarizing your project...' },
      ]);

      try {
        const summary = await invoke<string>('get_project_summary', { projectId });
        setMessages(prev => {
          const updated = [...prev];
          updated[updated.length - 1] = { role: 'assistant', content: summary };
          return updated;
        });
      } catch (err) {
        setMessages(prev => {
          const updated = [...prev];
          updated[updated.length - 1] = { role: 'assistant', content: `Failed to summarize: ${err}` };
          return updated;
        });
      } finally {
        setLoading(false);
      }
      return;
    }

    // Default: treat as a new topic to add
    setTopics(prev => [...prev, userMessage]);
    setMessages(prev => [
      ...prev,
      { role: 'user', content: userMessage },
      {
        role: 'assistant',
        content: `Added "${userMessage}" to your research topics. Say "search" when you're ready.`,
      },
    ]);
  };

  const handleRemoveTopic = (index: number) => {
    const removed = topics[index];
    setTopics(prev => prev.filter((_, i) => i !== index));
    setMessages(prev => [
      ...prev,
      { role: 'system', content: `Removed topic: "${removed}"` },
    ]);
  };

  const handleAddGem = async (gemId: string) => {
    try {
      await invoke('add_gems_to_project', { projectId, gemIds: [gemId] });
      setAddedGemIds(prev => new Set(prev).add(gemId));
      onGemsAdded?.();
    } catch (err) {
      console.error('Failed to add gem:', err);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  };

  if (initializing) {
    return (
      <div className="research-chat">
        <div className="research-chat-loading">
          <div className="spinner" />
          <span>Analyzing your project...</span>
        </div>
      </div>
    );
  }

  return (
    <div className="research-chat">
      <div className="research-chat-messages">
        {messages.map((msg, index) => (
          <div key={index} className={`chat-message chat-${msg.role}`}>
            {msg.role !== 'system' ? (
              <div className="chat-bubble">
                <div className="chat-text">{msg.content}</div>

                {/* Topic chips with remove buttons */}
                {msg.suggestedTopics && (
                  <div className="research-topics-list">
                    {topics.map((topic, i) => (
                      <div key={i} className="research-topic-chip">
                        <span>{i + 1}. {topic}</span>
                        <button className="topic-remove" onClick={() => handleRemoveTopic(i)}>x</button>
                      </div>
                    ))}
                    <button
                      className="action-button research-go-button"
                      onClick={() => handleRunResearch(topics)}
                      disabled={topics.length === 0 || loading}
                    >
                      Search ({topics.length} topics)
                    </button>
                  </div>
                )}

                {/* Web result cards inline in chat */}
                {msg.researchResults && msg.researchResults.web_results.length > 0 && (
                  <div className="research-section">
                    <h4 className="research-section-title">From the web</h4>
                    {msg.researchResults.web_results.map((result, i) => (
                      <div key={i} className="web-result-card" onClick={() => open(result.url)}>
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

                {/* Gem suggestion cards inline in chat */}
                {msg.researchResults && msg.researchResults.suggested_gems.length > 0 && (
                  <div className="research-section">
                    <h4 className="research-section-title">From your library</h4>
                    {msg.researchResults.suggested_gems.map((gem) => (
                      <div key={gem.id} className="research-gem-card">
                        <div className="gem-info">
                          <span className={`source-badge ${gem.source_type.toLowerCase()}`}>{gem.source_type}</span>
                          <span className="gem-title">{gem.title}</span>
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
            ) : (
              <div className="chat-system-msg">{msg.content}</div>
            )}
          </div>
        ))}

        {loading && (
          <div className="chat-message chat-assistant">
            <div className="chat-bubble thinking">Thinking...</div>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      <div className="chat-input-bar">
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyPress={handleKeyPress}
          placeholder="Add a topic, say 'search', or ask a question..."
          disabled={loading}
          className="chat-input"
        />
        <button
          onClick={handleSendMessage}
          disabled={!input.trim() || loading}
          className="chat-send-button"
        >
          Send
        </button>
      </div>
    </div>
  );
}
```

### RightPanel Integration (MODIFIED)

**File**: `src/components/RightPanel.tsx`

When `activeNav === 'projects'`, the RightPanel shows the Research Agent chat. This replaces the current placeholder ("Select a gem to view details").

Add to RightPanel props:
```typescript
selectedProjectId?: string | null;
selectedProjectTitle?: string | null;
onProjectGemsChanged?: () => void;
```

Replace the `activeNav === 'projects'` block (lines 402-479) with:

```tsx
if (activeNav === 'projects') {
  // No project selected
  if (!selectedProjectId) {
    return (
      <div className="right-panel" style={style}>
        <div className="right-panel-placeholder">
          Select a project to start researching
        </div>
      </div>
    );
  }

  // Project selected + gem selected → show tabs: Research | Detail
  if (selectedGemId) {
    return (
      <div className="right-panel" style={style}>
        <div className="record-tabs-view">
          <div className="tab-buttons">
            <button
              className={`tab-button ${activeTab === 'chat' ? 'active' : ''}`}
              onClick={() => handleTabChange('chat')}
            >
              Research
            </button>
            <button
              className={`tab-button ${activeTab === 'transcript' ? 'active' : ''}`}
              onClick={() => handleTabChange('transcript')}
            >
              Detail
            </button>
          </div>
          <div className="tab-content">
            {activeTab === 'chat' ? (
              <ProjectResearchChat
                projectId={selectedProjectId}
                projectTitle={selectedProjectTitle || ''}
                onGemsAdded={onProjectGemsChanged}
              />
            ) : (
              <GemDetailPanel
                gemId={selectedGemId}
                onDelete={onDeleteGem}
                onTranscribe={onTranscribeGem}
                onEnrich={onEnrichGem}
                aiAvailable={aiAvailable}
                onOpenKnowledgeFile={handleOpenKnowledgeFile}
              />
            )}
          </div>
        </div>
      </div>
    );
  }

  // Project selected, no gem selected → research chat full-height
  return (
    <div className="right-panel" style={style}>
      <ProjectResearchChat
        projectId={selectedProjectId}
        projectTitle={selectedProjectTitle || ''}
        onGemsAdded={onProjectGemsChanged}
      />
    </div>
  );
}
```

### App.tsx Integration

Lift `selectedProjectId` state from `ProjectsContainer` up to `App.tsx` so it can be passed to both `ProjectsContainer` and `RightPanel`. This follows the existing pattern used for `selectedGemId`.

```tsx
// In App.tsx state:
const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
const [selectedProjectTitle, setSelectedProjectTitle] = useState<string | null>(null);

// Pass to ProjectsContainer:
<ProjectsContainer
  onGemSelect={handleGemSelect}
  onProjectSelect={(id, title) => {
    setSelectedProjectId(id);
    setSelectedProjectTitle(title);
  }}
/>

// Pass to RightPanel:
<RightPanel
  // ... existing props ...
  selectedProjectId={selectedProjectId}
  selectedProjectTitle={selectedProjectTitle}
  onProjectGemsChanged={() => { /* refresh project gem list */ }}
/>
```

---

## CSS Additions

Add to `App.css`:

```css
/* ── Research Chat ── */

.research-chat {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.research-chat-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 12px;
  color: var(--text-secondary, #aaa);
}

.research-chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

/* ── Topic Chips ── */

.research-topics-list {
  margin-top: 12px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.research-topic-chip {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--border-color, #333);
  font-size: 13px;
}

.topic-remove {
  background: none;
  border: none;
  color: var(--text-muted, #666);
  cursor: pointer;
  font-size: 12px;
  padding: 2px 6px;
}

.topic-remove:hover {
  color: var(--error-color, #ef4444);
}

.research-go-button {
  margin-top: 8px;
  align-self: flex-end;
}

/* ── System Messages ── */

.chat-system-msg {
  font-size: 11px;
  color: var(--text-muted, #666);
  text-align: center;
  padding: 4px 0;
  font-style: italic;
}

/* ── Research Sections in Chat ── */

.research-section {
  margin-top: 12px;
}

.research-section-title {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted, #888);
  margin: 0 0 8px 0;
}

/* ── Web Result Cards ── */

.web-result-card {
  padding: 10px;
  border-radius: 6px;
  cursor: pointer;
  margin-bottom: 6px;
  border: 1px solid var(--border-color, #333);
}

.web-result-card:hover {
  background: rgba(255, 255, 255, 0.05);
  border-color: var(--accent-color, #3b82f6);
}

.web-result-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.source-type-badge {
  display: inline-block;
  padding: 1px 5px;
  border-radius: 3px;
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
  font-size: 13px;
  font-weight: 500;
  margin-bottom: 2px;
}

.web-result-snippet {
  font-size: 12px;
  color: var(--text-secondary, #aaa);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

/* ── Gem Suggestion Cards ── */

.research-gem-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 10px;
  margin-bottom: 4px;
  border-radius: 6px;
  border: 1px solid var(--border-color, #333);
}

.research-gem-card .gem-info {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
}

.research-gem-card .gem-title {
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.research-add-gem {
  flex-shrink: 0;
  padding: 4px 10px;
  border-radius: 4px;
  border: 1px solid var(--accent-color, #3b82f6);
  background: transparent;
  color: var(--accent-color, #3b82f6);
  font-size: 11px;
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
```

---

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| No Tavily API key | `web_provider` is `None`. `run_research` returns empty `web_results`. Gem suggestions still work. |
| Tavily API rate limited | Individual topic search returns error, logged, skipped. Remaining topics continue. |
| LLM returns non-JSON topics | `serde_json::from_str` fails. `suggest_topics` returns error. Chat shows error message. |
| LLM returns markdown-wrapped JSON | Code fence stripping handles `` ```json [...] ``` `` wrapping. |
| LLM unavailable | `chat()` returns error. Frontend shows error in chat bubble. |
| No gems in library | `search_provider.search()` returns empty. `suggested_gems` empty. Web results still show. |
| Project has no description or objective | Context is just the title. LLM generates topics from title alone. |
| Duplicate URLs across topics | `dedup_by` on sorted URLs removes duplicates. |
| Summarize with 0 gems | Returns friendly message: "This project has no gems yet." |
| User adds custom topic | Added to local `topics` state. Included in next `run_research` call. |
| User removes all topics then clicks search | "Search" button disabled when 0 topics selected. |
| Gem already in project | "Add" button should check existing project gems. Future enhancement. |
| App restart clears research chat | Chat is ephemeral (v1). Research can be re-run anytime. |
| Multiple projects open | Each `ProjectResearchChat` instance has its own state. Switching projects remounts. |
| Switch project while research loading | `useEffect` cleanup cancels stale requests via `cancelled` flag. |

---

## Testing Strategy

### Unit Tests

**`project_agent.rs` tests:**
- `suggest_topics()` returns 3-5 strings for valid project
- `suggest_topics()` handles markdown-wrapped JSON
- `run_research()` returns web + gem results for given topics
- `run_research()` with empty topics returns empty results
- `run_research()` handles web search failure gracefully (skips, continues)
- `run_research()` deduplicates URLs across topics
- `summarize()` returns summary for project with gems
- `summarize()` returns friendly message for project with 0 gems
- `start_chat()` + `send_chat_message()` + `end_chat()` lifecycle

**`project_chat.rs` tests:**
- `label()` returns "Project: {title}"
- `needs_preparation()` returns false
- `get_context()` assembles project metadata + gem content

### Manual Testing Checklist

- [ ] Open project → RightPanel shows research chat with loading spinner
- [ ] Topics appear as numbered chips with remove buttons
- [ ] Remove a topic → chip disappears, system message shown
- [ ] Type custom topic → added to topic list
- [ ] Click "Search (N topics)" → loading state → web cards + gem cards appear in chat
- [ ] Click web result card → opens URL in system browser
- [ ] Click "Add" on gem card → button changes to "Added", gem appears in project
- [ ] Type "summarize" → summary appears as chat message
- [ ] Select a gem in project → "Detail" tab appears alongside "Research" tab
- [ ] Switch between Research and Detail tabs → state preserved
- [ ] Remove Tavily API key → restart → web section empty, gems still work
- [ ] Switch to different project → research chat resets with new topics
- [ ] Existing gem search in GemsPanel still works unchanged

---

## Summary

The Research Agent becomes a **conversational collaborator** in the RightPanel. The two-phase research flow (suggest topics → user curates → execute search) gives users control while keeping the interaction natural. The existing Chatbot engine and ProjectChatSource provide chat infrastructure, while `ProjectResearchChat` on the frontend handles rich message rendering (topic chips, web cards, gem cards).

Key changes from the original one-shot design:
1. **`research()` split into `suggest_topics()` + `run_research(topics)`** — user controls what gets searched
2. **Chat-first UI** — research lives in RightPanel as a conversation, not a standalone panel
3. **Rich messages** — web results and gem suggestions render as interactive cards within chat bubbles
4. **RightPanel integration** — "Research" tab for projects (default), "Detail" tab when gem selected
5. **Frontend intent detection (v1)** — keywords like "search", "summarize" trigger actions; everything else adds topics

All search infrastructure (Phases 1-2) remains unchanged. Backend changes are minimal (one method becomes two). The heaviest work is the frontend `ProjectResearchChat` component and RightPanel wiring.
