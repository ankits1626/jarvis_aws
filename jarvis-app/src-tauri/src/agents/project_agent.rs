// ProjectResearchAgent — Research, Summarize, and Chat for Projects
//
// This agent manages all project intelligence capabilities:
// - Research: LLM topic generation + web search + gem suggestions
// - Summarize: LLM summary of all project gems
// - Chat: Conversational Q&A over project content via Chatbot + ProjectChatSource

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

use super::chatbot::{Chatbot, ChatMessage};
use super::project_chat::ProjectChatSource;
use crate::gems::{GemStore, Gem};
use crate::intelligence::provider::IntelProvider;
use crate::intelligence::queue::IntelQueue;
use crate::knowledge::KnowledgeStore;
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

/// Result of summary checkpoint generation — returned to frontend for review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummaryResult {
    /// The generated summary markdown
    pub summary: String,
    /// The full composite document used as LLM input
    pub composite_doc: String,
    /// Number of gems analyzed
    pub gems_analyzed: usize,
    /// Number of chunks used in summarization
    pub chunks_used: usize,
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

const CHECKPOINT_SUMMARY_PROMPT: &str = r#"You are a research analyst. Given a project and its collected resources (listed chronologically), generate a comprehensive summary covering all key points.

Format:
- Group findings by date
- Under each resource: 3-5 bullet points with the most important highlights
- End with a synthesis of cross-cutting themes

Rules:
- Be specific — cite actual facts, numbers, and insights
- Every resource should have key points extracted
- Use markdown formatting
- Keep each bullet to one concise sentence"#;

const CHUNK_SUMMARY_PROMPT: &str = r#"You are summarizing a section of a research project. Extract the key points and highlights from each resource below. Be specific and preserve important details.

Format: For each resource, list 3-5 key bullet points."#;

const MERGE_SUMMARY_PROMPT: &str = r#"You have summaries from different sections of a research project. Combine them into one cohesive summary document.

Rules:
- Preserve all key points from each section
- Maintain chronological order by date
- Add a brief synthesis of cross-cutting themes at the end
- Use markdown formatting"#;

const SUMMARY_QA_PROMPT: &str = r#"You are answering questions about a project summary. Use the summary and source material provided to give specific, grounded answers. If the answer isn't in the provided material, say so."#;

/// Persistent agent for project research, summarization, and chat.
///
/// Registered in Tauri state as Arc<TokioMutex<ProjectResearchAgent>>.
/// All actions can be invoked independently on any project at any time.
pub struct ProjectResearchAgent {
    project_store: Arc<dyn ProjectStore>,
    gem_store: Arc<dyn GemStore>,
    intel_provider: Arc<dyn IntelProvider>,
    search_provider: Arc<dyn SearchResultProvider>,
    knowledge_store: Arc<dyn KnowledgeStore>,
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
        knowledge_store: Arc<dyn KnowledgeStore>,
        intel_queue: Arc<IntelQueue>,
    ) -> Self {
        eprintln!("Projects/Research: Agent initialized");
        Self {
            project_store,
            gem_store,
            intel_provider,
            search_provider,
            knowledge_store,
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

    // ────────────────────────────────────────────
    // Summary Checkpoint Generation
    // ────────────────────────────────────────────

    /// Build the composite document from all project gems' knowledge files.
    ///
    /// Returns: (full_composite_doc, individual_gem_sections, gems_analyzed)
    ///
    /// For each gem (sorted by captured_at ASC):
    ///   1. Try KnowledgeStore::get_assembled(gem_id) → gem.md content
    ///   2. Fallback: assemble from DB fields (title + description + content + summary + transcript)
    ///   3. Wrap with separator header containing gem metadata
    async fn build_composite_document(
        &self,
        project_id: &str,
    ) -> Result<(String, Vec<String>, usize), String> {
        // Load project with gems
        let detail = self.project_store.get(project_id).await?;
        let project = &detail.project;

        // Return error if no gems
        if detail.gems.is_empty() {
            return Err("This project has no gems yet. Add some resources first.".to_string());
        }

        // Sort gems by captured_at ascending (chronological)
        let mut gems = detail.gems.clone();
        gems.sort_by(|a, b| a.captured_at.cmp(&b.captured_at));

        // Build composite header
        let objective = project.objective.as_deref().unwrap_or("Not specified");
        let first_date = &gems[0].captured_at;
        let last_date = &gems[gems.len() - 1].captured_at;
        
        let header = format!(
            "# Project: {}\n**Objective:** {}\n**Gems:** {} | **Date range:** {} — {}\n\n---\n",
            project.title,
            objective,
            gems.len(),
            first_date,
            last_date
        );

        // Build individual gem sections
        let mut gem_sections: Vec<String> = Vec::new();
        let mut gems_analyzed = 0;

        for (idx, gem_preview) in gems.iter().enumerate() {
            let gem_num = idx + 1;
            
            // Try to get assembled gem.md content first
            let content = match self.knowledge_store.get_assembled(&gem_preview.id).await {
                Ok(Some(assembled_content)) => {
                    eprintln!("Projects/Summary: Using assembled content for gem {}", gem_preview.id);
                    assembled_content
                }
                Ok(None) | Err(_) => {
                    // Fallback: assemble from DB fields
                    eprintln!("Projects/Summary: Falling back to DB fields for gem {}", gem_preview.id);
                    
                    match self.gem_store.get(&gem_preview.id).await {
                        Ok(Some(full_gem)) => {
                            let mut parts: Vec<String> = Vec::new();
                            
                            // Title
                            parts.push(format!("# {}", full_gem.title));
                            
                            // Description
                            if let Some(ref desc) = full_gem.description {
                                parts.push(format!("\n**Description:** {}", desc));
                            }
                            
                            // Content (first 2000 chars)
                            if let Some(ref content) = full_gem.content {
                                let preview = if content.len() > 2000 {
                                    format!("{}...\n\n[Content truncated — original was {} characters]", 
                                            &content[..2000], content.len())
                                } else {
                                    content.clone()
                                };
                                parts.push(format!("\n**Content:**\n{}", preview));
                            }
                            
                            // AI enrichment summary
                            if let Some(ref enrichment) = full_gem.ai_enrichment {
                                if let Some(summary) = enrichment.get("summary").and_then(|v| v.as_str()) {
                                    parts.push(format!("\n**Summary:** {}", summary));
                                }
                            }
                            
                            // Transcript (first 2000 chars)
                            if let Some(ref transcript) = full_gem.transcript {
                                let preview = if transcript.len() > 2000 {
                                    format!("{}...\n\n[Transcript truncated — original was {} characters]", 
                                            &transcript[..2000], transcript.len())
                                } else {
                                    transcript.clone()
                                };
                                parts.push(format!("\n**Transcript:**\n{}", preview));
                            }
                            
                            if parts.is_empty() {
                                "(No content available for this gem)".to_string()
                            } else {
                                parts.join("\n")
                            }
                        }
                        Ok(None) => {
                            eprintln!("Projects/Summary: Gem {} not found in store", gem_preview.id);
                            "(No content available for this gem)".to_string()
                        }
                        Err(e) => {
                            eprintln!("Projects/Summary: Error loading gem {}: {}", gem_preview.id, e);
                            "(No content available for this gem)".to_string()
                        }
                    }
                }
            };

            // Skip gems with no content
            if content.trim() == "(No content available for this gem)" {
                eprintln!("Projects/Summary: Skipping gem {} — no content to analyze", gem_preview.id);
                continue;
            }

            // Wrap with separator header
            let section = format!(
                "========================================\nGEM {}: \"{}\"\nSource: {} | Domain: {} | Captured: {}\n========================================\n\n{}",
                gem_num,
                gem_preview.title,
                gem_preview.source_type,
                gem_preview.domain,
                gem_preview.captured_at,
                content
            );

            gem_sections.push(section);
            gems_analyzed += 1;
        }

        // Build full composite document
        let full_composite_doc = format!("{}\n\n{}", header, gem_sections.join("\n\n"));

        eprintln!(
            "Projects/Summary: Built composite document: {} chars, {} gems",
            full_composite_doc.len(),
            gems_analyzed
        );

        Ok((full_composite_doc, gem_sections, gems_analyzed))
    }

    /// Group gem sections into chunks respecting gem boundaries.
    ///
    /// Walks through gem sections in order, accumulating into chunks until
    /// adding the next section would exceed max_chars. If a single gem exceeds
    /// the limit, it's truncated with a note.
    ///
    /// Returns: Vec of chunk strings, each containing one or more complete gem sections.
    fn chunk_by_gem_boundaries(gem_sections: &[String], max_chars: usize) -> Vec<String> {
        let mut chunks: Vec<String> = Vec::new();
        let mut current_chunk = String::new();

        for section in gem_sections {
            let section_len = section.len();
            let separator_len = if current_chunk.is_empty() { 0 } else { 2 }; // "\n\n"
            let would_be_len = current_chunk.len() + separator_len + section_len;

            if would_be_len <= max_chars {
                // Section fits in current chunk
                if !current_chunk.is_empty() {
                    current_chunk.push_str("\n\n");
                }
                current_chunk.push_str(section);
            } else if current_chunk.is_empty() {
                // Single gem exceeds limit — truncate it
                let truncated = if section_len > max_chars {
                    let truncate_at = max_chars.saturating_sub(100); // Leave room for note
                    format!(
                        "{}\n\n[Content truncated — original was {} characters]",
                        &section[..truncate_at],
                        section_len
                    )
                } else {
                    section.clone()
                };
                chunks.push(truncated);
            } else {
                // Current chunk is full — push it and start new chunk with this section
                chunks.push(current_chunk);
                current_chunk = section.clone();
            }
        }

        // Push final chunk if non-empty
        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        eprintln!(
            "Projects/Summary: Split into {} chunks from {} gem sections",
            chunks.len(),
            gem_sections.len()
        );

        chunks
    }

    /// Generate a summary checkpoint for review (does NOT save).
    ///
    /// Orchestrates the full pipeline:
    /// 1. Build composite document from all project gems
    /// 2. Chunk by gem boundaries
    /// 3. Summarize each chunk via LLM
    /// 4. Merge chunk summaries if multiple chunks
    /// 5. Return summary + composite doc for frontend review
    pub async fn generate_summary_checkpoint(
        &self,
        project_id: &str,
    ) -> Result<ProjectSummaryResult, String> {
        eprintln!("Projects/Summary: Generating summary checkpoint for project {}", project_id);

        // Build composite document from all gems
        let (composite_doc, gem_sections, gems_analyzed) = 
            self.build_composite_document(project_id).await?;

        // Chunk by gem boundaries (16000 chars ≈ 4000 tokens)
        let chunks = Self::chunk_by_gem_boundaries(&gem_sections, 16000);

        // Generate summary based on chunk count
        let summary = if chunks.len() == 1 {
            // Single chunk — one LLM call
            eprintln!("Projects/Summary: Single chunk — using direct summarization");
            self.intel_provider.chat(&[
                ("system".to_string(), CHECKPOINT_SUMMARY_PROMPT.to_string()),
                ("user".to_string(), chunks[0].clone()),
            ]).await?
        } else {
            // Multiple chunks — summarize each, then merge
            eprintln!("Projects/Summary: {} chunks — using chunked summarization", chunks.len());
            
            let mut chunk_summaries: Vec<String> = Vec::new();
            for (i, chunk) in chunks.iter().enumerate() {
                eprintln!("Projects/Summary: Summarizing chunk {} of {}", i + 1, chunks.len());
                match self.intel_provider.chat(&[
                    ("system".to_string(), CHUNK_SUMMARY_PROMPT.to_string()),
                    ("user".to_string(), chunk.clone()),
                ]).await {
                    Ok(chunk_summary) => chunk_summaries.push(chunk_summary),
                    Err(e) => {
                        eprintln!("Projects/Summary: Chunk {} failed: {}", i + 1, e);
                        // Continue with remaining chunks — don't fail entirely
                    }
                }
            }

            // Check if all chunks failed
            if chunk_summaries.is_empty() {
                return Err("All chunks failed during summarization".to_string());
            }

            // Merge pass
            eprintln!("Projects/Summary: Merging {} chunk summaries", chunk_summaries.len());
            let merged_input = chunk_summaries.join("\n\n---\n\n");
            self.intel_provider.chat(&[
                ("system".to_string(), MERGE_SUMMARY_PROMPT.to_string()),
                ("user".to_string(), merged_input),
            ]).await?
        };

        eprintln!(
            "Projects/Summary: Generated summary ({} chars) from {} gems in {} chunks",
            summary.len(),
            gems_analyzed,
            chunks.len()
        );

        Ok(ProjectSummaryResult {
            summary,
            composite_doc,
            gems_analyzed,
            chunks_used: chunks.len(),
        })
    }

    /// Answer a question about a generated summary.
    ///
    /// Stateless Q&A: user asks a question, LLM answers using summary + composite doc as context.
    /// Each question is independent — no chat session needed.
    pub async fn send_summary_question(
        &self,
        question: &str,
        summary: &str,
        composite_doc: &str,
    ) -> Result<String, String> {
        eprintln!("Projects/Summary: Answering question ({} chars)", question.len());

        // Truncate composite_doc if too long (preserve beginning with oldest/foundational gems)
        let max_context = 10000;
        let truncated_composite = if composite_doc.len() > max_context {
            &composite_doc[..max_context]
        } else {
            composite_doc
        };

        // Build context string
        let context = format!(
            "## Generated Summary\n\n{}\n\n## Source Material\n\n{}",
            summary, truncated_composite
        );

        // Call LLM
        let user_message = format!("{}\n\nQuestion: {}", context, question);
        let answer = self.intel_provider.chat(&[
            ("system".to_string(), SUMMARY_QA_PROMPT.to_string()),
            ("user".to_string(), user_message),
        ]).await?;

        eprintln!("Projects/Summary: Answer generated ({} chars)", answer.len());
        Ok(answer)
    }

    // ────────────────────────────────────────────
    // Save Summary as Gem
    // ────────────────────────────────────────────

    /// Save a generated summary as a new gem in the project.
    ///
    /// Creates a gem with source_type "ProjectSummary", adds it to the project,
    /// generates knowledge files, and writes the composite document as a subfile.
    ///
    /// Returns the created Gem for the frontend to display.
    pub async fn save_summary_checkpoint(
        &self,
        project_id: &str,
        summary_content: &str,
        composite_doc: &str,
    ) -> Result<Gem, String> {
        eprintln!("Projects/Summary: Saving summary checkpoint for project {}", project_id);

        // Step 1: Load project metadata
        let project_detail = self.project_store.get(project_id).await
            .map_err(|e| format!("Failed to load project: {}", e))?;
        let gem_count = project_detail.gems.len();
        let project_title = project_detail.project.title.clone();

        // Step 2: Build the Gem struct
        let gem = Gem {
            id: Uuid::new_v4().to_string(),
            source_type: "ProjectSummary".to_string(),
            source_url: format!(
                "jarvis://project/{}/summary/{}",
                project_id,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
            title: format!("Summary: {} — {}", project_title, Utc::now().format("%B %d, %Y")),
            content: Some(summary_content.to_string()),
            domain: "jarvis".to_string(),
            description: Some(format!("Summary of {} gems from {}", gem_count, project_title)),
            author: None,
            captured_at: Utc::now().to_rfc3339(),
            source_meta: serde_json::json!({}),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };

        // Step 3: Save gem to database
        let saved_gem = self.gem_store.save(gem).await
            .map_err(|e| format!("Failed to save summary gem: {}", e))?;

        // Step 4: Add gem to project
        self.project_store.add_gems(project_id, &[saved_gem.id.clone()]).await
            .map_err(|e| format!("Failed to add summary gem to project: {}", e))?;

        // Step 5: Create knowledge files
        self.knowledge_store.create(&saved_gem).await
            .map_err(|e| format!("Failed to create knowledge files: {}", e))?;

        // Step 6: Write composite file as subfile
        self.knowledge_store.update_subfile(
            &saved_gem.id,
            "composite_summary_of_all_gems.md",
            composite_doc,
        ).await
            .map_err(|e| format!("Failed to write composite file: {}", e))?;

        // Step 7: Index for search (fire-and-forget)
        if let Err(e) = self.search_provider.index_gem(&saved_gem.id).await {
            eprintln!("Projects/Summary: Failed to index summary gem {}: {}", saved_gem.id, e);
        }

        // Step 8: Log and return
        eprintln!(
            "Projects/Summary: Saved summary gem {} for project {} ({} chars)",
            saved_gem.id, project_id, summary_content.len()
        );

        Ok(saved_gem)
    }

    // ────────────────────────────────────────────
    // Load Latest Summary Checkpoint
    // ────────────────────────────────────────────

}
