# Kiro Implementation Prompt — Phases 3-4: Agent Backend + Tauri Commands

## Goal

Refactor the existing `ProjectResearchAgent` from a single `research()` method to a **two-phase research flow** (`suggest_topics()` + `run_research(topics)`), then add **7 Tauri commands** and register the agent in `lib.rs`.

**If you have any confusion or queries during implementation, please ask before proceeding.** After completing all changes, stop and ask for review before moving on.

---

## Current State (What's Already Built)

### Phases 1-2 (DONE — do NOT modify)
- `src/search/provider.rs` — `SearchResultProvider` trait with `web_search`, `supports_web_search` defaults, `WebSearchResult`, `WebSourceType`, `GemSearchResult`
- `src/search/tavily_provider.rs` — `TavilyProvider` implementing web search via Tavily API
- `src/search/composite_provider.rs` — `CompositeSearchProvider` wrapping gem + web providers
- `lib.rs` lines 291-350 — CompositeSearchProvider registered in Tauri state

### Phase 3 (PARTIALLY BUILT — needs refactor)
- `src/agents/project_chat.rs` — **DONE, no changes needed.** Already matches the new design.
- `src/agents/project_agent.rs` — **EXISTS but uses OLD design.** Has a single `research()` method and `topics_generated` field. Needs refactoring (see below).
- `src/agents/mod.rs` — Already has `pub mod project_chat;` and `pub mod project_agent;`

### Phase 4 (NOT BUILT)
- `src/projects/commands.rs` — Only has 9 CRUD commands, no agent commands
- `lib.rs` — No agent registration, no `intel_queue` Arc wrapping

---

## What You Need To Do

### Part 1: Refactor `src/agents/project_agent.rs`

The existing file has a single `research()` method. Split it into two methods and rename one field.

#### Changes Required:

1. **Rename `topics_generated` → `topics_searched`** in `ProjectResearchResults` struct:
```rust
// BEFORE (line 30):
pub topics_generated: Vec<String>,

// AFTER:
/// The topics that were actually searched (user-curated)
pub topics_searched: Vec<String>,
```

2. **Split `research()` into `suggest_topics()` + `run_research(topics)`:**

**Delete the existing `research()` method (lines 97-205)** and replace with these two methods:

```rust
// ────────────────────────────────────────────
// Phase A: Topic Suggestion
// ────────────────────────────────────────────

/// Generate research topic suggestions for a project.
///
/// Called when the research chat opens. Returns 3-5 topic strings.
/// The user curates these before triggering the actual search.
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

    // Enrich with full gem data (same pattern as search_gems command)
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
```

3. **Keep everything else unchanged:** The `summarize()`, `start_chat()`, `send_chat_message()`, `get_chat_history()`, and `end_chat()` methods remain exactly as they are.

---

### Part 2: Add Agent Tauri Commands to `src/projects/commands.rs`

Add these imports and 7 new commands to the **existing** `commands.rs` file (append after the existing 9 CRUD commands):

```rust
// ── Add these imports at the top (alongside existing imports) ──

use tokio::sync::Mutex as TokioMutex;
use crate::agents::project_agent::{ProjectResearchAgent, ProjectResearchResults};
use crate::agents::chatbot::ChatMessage;

// ── New agent commands (append after existing commands) ──

/// Suggest research topics for a project (Phase A of two-phase research).
#[tauri::command]
pub async fn suggest_project_topics(
    project_id: String,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<Vec<String>, String> {
    let agent = agent.lock().await;
    agent.suggest_topics(&project_id).await
}

/// Execute research on user-curated topics (Phase B of two-phase research).
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

---

### Part 3: Register Agent in `lib.rs`

Two changes in `lib.rs`:

#### Change 1: Wrap `intel_queue` in `Arc`

In `lib.rs` at lines 121-126, change:
```rust
// BEFORE (lines 121-126):
let intel_queue = tauri::async_runtime::block_on(async {
    intelligence::IntelQueue::new(intel_provider.clone())
});
app.manage(intel_queue);

// AFTER:
let intel_queue = tauri::async_runtime::block_on(async {
    intelligence::IntelQueue::new(intel_provider.clone())
});
let intel_queue_arc = Arc::new(intel_queue);
app.manage(intel_queue_arc.clone());
```

**Important — 2 existing usages must be updated:**

In `src/commands.rs`, there are exactly 2 commands using `State<'_, IntelQueue>`:
1. Line 4310: `chat_with_recording` — change `intel_queue: State<'_, IntelQueue>` → `intel_queue: State<'_, Arc<IntelQueue>>`
2. Line 4379: `chat_send_message` — change `intel_queue: State<'_, IntelQueue>` → `intel_queue: State<'_, Arc<IntelQueue>>`

In both functions, the `intel_queue` is used as `&intel_queue` or `&*intel_queue` — since `Arc<IntelQueue>` auto-derefs to `&IntelQueue`, the inner usage should work. But you may need to add `use std::sync::Arc;` to the imports in `commands.rs` if not already there, and dereference appropriately (e.g., `&**intel_queue` or `intel_queue.as_ref()`).

#### Change 2: Create and register `ProjectResearchAgent`

Add this block **after** the search provider registration (after `app.manage(search_provider)` on line ~350) but **before** `Ok(())`:

```rust
// ── Project Research Agent Setup ──
// Clone search_provider BEFORE app.manage consumes it
// (Move this clone BEFORE the app.manage(search_provider) line)
let search_provider_for_agent = search_provider.clone();
// Then app.manage(search_provider) as before

let project_agent = agents::project_agent::ProjectResearchAgent::new(
    project_store_arc.clone(),
    gem_store_arc.clone(),
    intel_provider.clone(),
    search_provider_for_agent,
    intel_queue_arc.clone(),
);
app.manage(Arc::new(tokio::sync::Mutex::new(project_agent)));
```

**Note about `search_provider` clone ordering:** Since `app.manage(search_provider)` on line 350 moves the `Arc` into Tauri state, you need to clone it BEFORE that line. So reorder:
```rust
// BEFORE (current, line 350):
app.manage(search_provider);

// AFTER:
let search_provider_for_agent = search_provider.clone();
app.manage(search_provider);
```

**Critical: `project_store_arc` is consumed without `.clone()`!**

In `lib.rs`, `gem_store_arc` is registered with `.clone()` (line 56), but `project_store_arc` is NOT (line 62):
```rust
// Line 56 — gem_store is cloned (variable survives):
app.manage(gem_store_arc.clone());

// Line 62 — project_store is MOVED (variable consumed):
app.manage(project_store_arc);  // ← Must change to .clone()
```

Fix line 62 to preserve the variable for agent creation:
```rust
app.manage(project_store_arc.clone());
```

Similarly, `intel_provider` is consumed by `app.manage(intel_provider)` on line 128. Change to:
```rust
app.manage(intel_provider.clone());
```

**Summary of `.clone()` fixes needed for agent to access providers:**

| Variable | Line | Current | Fix |
|----------|------|---------|-----|
| `gem_store_arc` | 56 | `app.manage(gem_store_arc.clone())` | Already OK |
| `project_store_arc` | 62 | `app.manage(project_store_arc)` | Change to `app.manage(project_store_arc.clone())` |
| `intel_provider` | 128 | `app.manage(intel_provider)` | Change to `app.manage(intel_provider.clone())` |
| `search_provider` | 350 | `app.manage(search_provider)` | Clone before manage (already in instructions above) |

The variable names in `lib.rs` are:
- `gem_store_arc` (line 55) — `Arc<dyn GemStore>`
- `project_store_arc` (line 61) — `Arc<dyn ProjectStore>`
- `intel_provider` (line 109) — `Arc<dyn IntelProvider>`
- `intel_queue_arc` — `Arc<IntelQueue>` (created from the new Arc wrapping above)

#### Change 3: Register commands in `generate_handler!`

Add these 7 commands to the `generate_handler![]` macro (after the existing `projects::commands::get_gem_projects`):

```rust
projects::commands::suggest_project_topics,
projects::commands::run_project_research,
projects::commands::get_project_summary,
projects::commands::start_project_chat,
projects::commands::send_project_chat_message,
projects::commands::get_project_chat_history,
projects::commands::end_project_chat,
```

---

## Files to Modify (Summary)

| File | Action |
|------|--------|
| `src/agents/project_agent.rs` | Refactor: split `research()` → `suggest_topics()` + `run_research(topics)`, rename `topics_generated` → `topics_searched` |
| `src/agents/project_chat.rs` | **NO CHANGES** — already correct |
| `src/agents/mod.rs` | **NO CHANGES** — already has both modules |
| `src/projects/commands.rs` | Add 3 imports + 7 new Tauri commands |
| `src/lib.rs` | 1) Wrap `intel_queue` in `Arc`, 2) Clone search_provider before manage, 3) Create + register `ProjectResearchAgent`, 4) Add 7 commands to `generate_handler!` |
| `src/commands.rs` | Update 2 usages of `State<'_, IntelQueue>` → `State<'_, Arc<IntelQueue>>` (lines 4310, 4379) |

## Files to NOT Modify

- Anything in `src/search/` — Phases 1-2 are complete
- `src/agents/chatable.rs` — unchanged
- `src/agents/chatbot.rs` — unchanged
- `src/agents/recording_chat.rs` — unchanged
- `src/agents/copilot.rs` — unchanged
- `src/projects/store.rs` — unchanged
- `src/intelligence/` — unchanged (except where `State<'_, IntelQueue>` needs `Arc` wrapping)
- `src/gems/` — unchanged

---

## Reference: Existing Patterns

### How `IntelQueue` is used in commands (for `Arc` wrapping):
Commands access it via `State<'_, IntelQueue>`. After wrapping in `Arc`, they'll need `State<'_, Arc<IntelQueue>>`. The queue has a `submit()` method that takes `IntelCommand` and returns `Result<IntelResponse, String>`.

### How `Chatbot.send_message()` is called:
```rust
self.chatbot.send_message(session_id, message, source, &self.intel_queue).await
```
The `intel_queue` parameter is `&IntelQueue`. Since `Arc<IntelQueue>` auto-derefs to `&IntelQueue`, the existing `send_chat_message()` method in `project_agent.rs` works without changes.

### How `CoPilotAgent` is registered (pattern to follow):
In `lib.rs` line 236:
```rust
app.manage(Arc::new(tokio::sync::Mutex::new(None::<agents::copilot::CoPilotAgent>)));
```
The CoPilot uses `Option<CoPilotAgent>` because it's created lazily. For `ProjectResearchAgent`, we create it eagerly (no `Option`):
```rust
app.manage(Arc::new(tokio::sync::Mutex::new(project_agent)));
```

---

## Verification

After all changes, run:
```bash
cargo check
```

This should compile without errors. If there are errors related to `IntelQueue` Arc wrapping, search the entire codebase for `State<'_, IntelQueue>` and update each occurrence.

Then run:
```bash
cargo build
```

The app should build successfully with all 7 new commands registered.

---

## What NOT To Do

- Do NOT create any frontend files (that's a later phase)
- Do NOT modify search providers (Phases 1-2 are complete)
- Do NOT add tests yet (that's Phase 8)
- Do NOT change the `Chatbot` or `Chatable` trait
- Do NOT rename files or move modules

**After completing all changes and verifying `cargo check` passes, stop and ask for review.**
