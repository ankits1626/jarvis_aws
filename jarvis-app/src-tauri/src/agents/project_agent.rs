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
