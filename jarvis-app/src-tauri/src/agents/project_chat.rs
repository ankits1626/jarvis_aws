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
