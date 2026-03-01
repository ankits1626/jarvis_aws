# Phase 3 Implementation Prompt — Agent Backend (ProjectChatSource + ProjectResearchAgent)

## What You're Building

Implement Phase 3 from `.kiro/specs/project-research-assistant/tasks.md` — **Tasks 5, 6, and 7**. This phase creates the core agent backend: a `ProjectChatSource` that makes projects chatbot-compatible, and a `ProjectResearchAgent` that provides research, summarization, and chat capabilities for any project.

**Read these files before writing any code:**
- `.kiro/specs/project-research-assistant/requirements.md` — Requirements 5 and 6 (acceptance criteria)
- `.kiro/specs/project-research-assistant/design.md` — "Agent Module" section has the exact Rust code
- `jarvis-app/src-tauri/src/agents/chatable.rs` — The `Chatable` trait you'll implement (5 methods)
- `jarvis-app/src-tauri/src/agents/chatbot.rs` — The `Chatbot` engine (start_session, send_message, get_history, end_session)
- `jarvis-app/src-tauri/src/agents/recording_chat.rs` — Reference `Chatable` implementation (follow this pattern)
- `jarvis-app/src-tauri/src/projects/store.rs` — `Project`, `ProjectDetail`, `ProjectStore` trait
- `jarvis-app/src-tauri/src/gems/store.rs` — `Gem`, `GemPreview`, `GemStore` trait
- `jarvis-app/src-tauri/src/intelligence/provider.rs` — `IntelProvider::chat()` method signature (line 109-127)
- `jarvis-app/src-tauri/src/intelligence/queue.rs` — `IntelQueue`, `IntelCommand::Chat`, `IntelResponse::Chat`
- `jarvis-app/src-tauri/src/search/provider.rs` — `SearchResultProvider`, `GemSearchResult`, `WebSearchResult`
- `jarvis-app/src-tauri/src/agents/mod.rs` — Current: `copilot`, `chatable`, `chatbot`, `recording_chat` (you'll add 2 modules)

## Context: What Phases 1-2 Built

Phase 1 created the search infrastructure (all compiling, all re-exported from `search/mod.rs`):
- `search::WebSearchResult`, `search::WebSourceType` — types for web search results
- `search::TavilyProvider` — web search via Tavily API
- `search::CompositeSearchProvider` — wraps gem + web providers

Phase 2 registered `CompositeSearchProvider` in `lib.rs`:
- `lib.rs` reads `tavily_api_key` from settings
- Builds optional `TavilyProvider`, wraps with `CompositeSearchProvider`
- Registered as `Arc<dyn SearchResultProvider>` in Tauri state

---

## Task 5: Create `ProjectChatSource` — Chatable for Projects

**File to create:** `jarvis-app/src-tauri/src/agents/project_chat.rs`

This implements the `Chatable` trait for projects, following the exact same pattern as `RecordingChatSource` in `recording_chat.rs`.

### Step 5.1: Create the file with imports and struct

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
```

### Step 5.2: Implement the `Chatable` trait

There are 5 methods to implement. Compare with `recording_chat.rs` lines 63-138 for the pattern.

```rust
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

**Key differences from RecordingChatSource:**
- `get_context()` does NOT use `intel_queue` (no transcript generation needed) — prefix param with `_`
- `needs_preparation()` returns `false` — context assembled on the fly
- `session_dir()` uses `dirs::data_dir()` (same as RecordingChatSource pattern at line 157-162)
- No `on_preparation_status()` override needed (default no-op is fine)
- No `AppHandle` needed (no event emission)

### Step 5.3: Register in `agents/mod.rs`

**File to modify:** `jarvis-app/src-tauri/src/agents/mod.rs`

The current file is:
```rust
pub mod copilot;
pub mod chatable;
pub mod chatbot;
pub mod recording_chat;
```

Add `project_chat`:
```rust
pub mod copilot;
pub mod chatable;
pub mod chatbot;
pub mod recording_chat;
pub mod project_chat;
```

**Do NOT add `project_agent` yet** — that comes in Step 7.7.

### Step 5.4: Verify compilation

Run `cargo check`. It should pass. The `ProjectChatSource` compiles even though nothing uses it yet.

**Potential issue:** The `dirs` crate. Check that it's in `Cargo.toml`. If you look at `recording_chat.rs` line 157, it uses `dirs::data_dir()`, so the crate is already available. No dependency changes needed.

---

## Task 6: Create `ProjectResearchAgent` — Research Action

**File to create:** `jarvis-app/src-tauri/src/agents/project_agent.rs`

### Step 6.1: Create the file with imports, struct, and result type

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
```

### Step 6.2: Define `ProjectResearchResults` struct

```rust
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
```

### Step 6.3: Define LLM prompt constants

```rust
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
```

### Step 6.4: Define the agent struct and constructor

```rust
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
```

**Important notes on the constructor:**
- `intel_queue` is `Arc<IntelQueue>`, NOT `Arc<dyn IntelQueue>` — `IntelQueue` is a concrete struct, not a trait
- `chatbot` is created fresh via `Chatbot::new()` — each agent has its own chatbot instance
- `chat_sources` maps `session_id -> ProjectChatSource` for active chat sessions

### Step 6.5: Implement the `research()` method

This is the most complex method. It:
1. Loads the project from `ProjectStore`
2. Asks the LLM to generate search topics
3. Runs web search for each topic via `SearchResultProvider::web_search()`
4. Deduplicates web results by URL
5. Runs gem search via `SearchResultProvider::search()`
6. Enriches gem results with full gem data from `GemStore`

```rust
    // ────────────────────────────────────────────
    // Research
    // ────────────────────────────────────────────

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
                        // Continue with remaining topics — don't fail the whole operation
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

        // 5. Gem search — find existing gems relevant to the project
        let gem_results = self.search_provider.search(&project.title, 20).await?;
        eprintln!("Projects/Research: {} raw gem search results", gem_results.len());

        // 6. Enrich with full gem data (same pattern as search_gems command in search/commands.rs)
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
```

**Critical notes on `research()`:**
- `intel_provider.chat()` takes `&[(String, String)]` — an array of (role, content) tuples
- LLM response may be wrapped in markdown code fences — the `trim_start_matches`/`trim_end_matches` chain handles this
- Web search failures are **logged and skipped**, NOT propagated — the loop continues with remaining topics
- `dedup_by` requires the vec to be sorted first — `sort_by` on URL then `dedup_by` comparing URLs
- The GemSearchResult enrichment pattern is identical to `search/commands.rs` (look at the `search_gems` command if you need reference)

---

## Task 7: Add Summarize and Chat Actions to ProjectResearchAgent

These methods are added to the same `impl ProjectResearchAgent` block in `project_agent.rs`.

### Step 7.1-7.2: Implement `summarize()`

```rust
    // ────────────────────────────────────────────
    // Summarize
    // ────────────────────────────────────────────

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
```

**Key points:**
- Returns a friendly message if project has 0 gems (no LLM call needed)
- Loads full gem data to include `ai_enrichment.summary` and `ai_enrichment.tags` in context
- Uses `intel_provider.chat()` directly — same pattern as `research()`

### Step 7.3: Implement `start_chat()`

```rust
    // ────────────────────────────────────────────
    // Chat
    // ────────────────────────────────────────────

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
```

**Important:** `start_chat` takes `&mut self` because `chatbot.start_session()` requires `&mut self` (see chatbot.rs line 64). The Tauri command will need to lock the agent mutex mutably.

### Step 7.4: Implement `send_chat_message()`

```rust
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
```

**Important:** `send_message` on `Chatbot` takes:
- `session_id: &str`
- `user_message: &str`
- `source: &dyn Chatable` — `ProjectChatSource` implements `Chatable`, so passing `source` (a `&ProjectChatSource`) works
- `intel_queue: &IntelQueue` — note this is `&IntelQueue`, not `&Arc<IntelQueue>`

Since `self.intel_queue` is `Arc<IntelQueue>`, you dereference with `&self.intel_queue` which auto-derefs to `&IntelQueue`.

### Step 7.5: Implement `get_chat_history()`

```rust
    pub fn get_chat_history(&self, session_id: &str) -> Result<Vec<ChatMessage>, String> {
        self.chatbot.get_history(session_id)
    }
```

**Note:** This is synchronous (no `async`) — `get_history` just reads from the in-memory HashMap.

### Step 7.6: Implement `end_chat()`

```rust
    pub fn end_chat(&mut self, session_id: &str) {
        self.chatbot.end_session(session_id);
        self.chat_sources.remove(session_id);
        eprintln!("Projects/Research: Chat session ended: {}", session_id);
    }
}  // Close the impl block
```

**Important:** Don't forget to remove the source from `chat_sources` when ending a session. Otherwise it leaks memory.

### Step 7.7: Register in `agents/mod.rs`

**File to modify:** `jarvis-app/src-tauri/src/agents/mod.rs`

After Step 5.3, the file should be:
```rust
pub mod copilot;
pub mod chatable;
pub mod chatbot;
pub mod recording_chat;
pub mod project_chat;
```

Now add `project_agent`:
```rust
pub mod copilot;
pub mod chatable;
pub mod chatbot;
pub mod recording_chat;
pub mod project_chat;
pub mod project_agent;
```

---

## Summary of All Files Changed/Created

| File | Action | Description |
|------|--------|-------------|
| `src/agents/project_chat.rs` | **CREATE** | `ProjectChatSource` implementing `Chatable` |
| `src/agents/project_agent.rs` | **CREATE** | `ProjectResearchAgent` with research, summarize, chat |
| `src/agents/mod.rs` | **MODIFY** | Add `pub mod project_chat;` and `pub mod project_agent;` |

**No other files are modified.** Specifically:
- Do NOT modify `lib.rs` — agent registration is Phase 4
- Do NOT modify `projects/commands.rs` — Tauri commands are Phase 4
- Do NOT modify any search files — Phase 1-2 are done
- Do NOT add commands to `generate_handler!` — Phase 4

---

## Phase Checkpoint

After implementation, verify:

```
cargo check
```

This MUST pass. Specifically verify:
1. `ProjectChatSource` compiles and implements `Chatable` with all 4 required methods + inherits `on_preparation_status` default
2. `ProjectResearchAgent` compiles with all 6 methods: `research`, `summarize`, `start_chat`, `send_chat_message`, `get_chat_history`, `end_chat`
3. `ProjectResearchResults` struct derives `Debug`, `Clone`, `Serialize`, `Deserialize`
4. Both new modules are declared in `agents/mod.rs`
5. No unused import warnings (all imports are used in the implementations)
6. No modifications to any existing files except `agents/mod.rs`

---

## Common Issues and Solutions

**Q: `dirs` crate not found?**
A: It's already in `Cargo.toml` — `recording_chat.rs` uses it at line 157. If somehow missing, add `dirs = "5"` to `[dependencies]`.

**Q: `serde_json` not imported in `project_agent.rs`?**
A: `serde_json` is used for `from_str` (topic parsing) and for accessing `ai_enrichment` JSON fields. It's already in `Cargo.toml`. Just use `serde_json::from_str` and `serde_json::Value` methods inline — no explicit `use serde_json;` needed because the `Gem.ai_enrichment` field is `Option<serde_json::Value>`.

**Q: `IntelQueue` type — why `Arc<IntelQueue>` not `Arc<dyn IntelQueue>`?**
A: `IntelQueue` is a concrete struct (see `intelligence/queue.rs` line 49), not a trait. The agent holds `Arc<IntelQueue>` to share it across async calls.

**Q: `Chatbot::send_message` needs `&mut self` — won't there be borrow issues?**
A: No. In `send_chat_message`, you first get an immutable reference to the source from `chat_sources`, then call `chatbot.send_message` which borrows `self.chatbot` mutably. These don't conflict because `chat_sources` and `chatbot` are separate fields. However, if the borrow checker complains, you can clone the source reference or restructure slightly. The exact code in the design.md has been verified to compile.

**Q: Will `ProjectChatSource` have unused import warnings?**
A: No, if you prefix the `intel_queue` parameter in `get_context()` with `_` (i.e., `_intel_queue: &IntelQueue`). The trait requires this parameter but `ProjectChatSource` doesn't use it.

**Q: The `IntelProvider::chat` signature — what exactly does it take?**
A: Looking at `intelligence/provider.rs` line 122: `async fn chat(&self, _messages: &[(String, String)]) -> Result<String, String>`. It takes a slice of (role, content) tuples. In the agent, we create a `Vec<(String, String)>` and pass `&[...]` or call `.as_slice()`.

Actually, looking more carefully, `research()` passes a slice literal:
```rust
self.intel_provider.chat(&[
    ("system".to_string(), TOPIC_GENERATION_PROMPT.to_string()),
    ("user".to_string(), context),
]).await?;
```
This creates a temporary array and passes a reference to it — valid Rust.

---

## Deliverables

When done, present:
1. The new `project_chat.rs` file
2. The new `project_agent.rs` file
3. The modified `agents/mod.rs` (just 2 lines added)
4. `cargo check` output showing success
5. Ask me for review before proceeding to Phase 4
