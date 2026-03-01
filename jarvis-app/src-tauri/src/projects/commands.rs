use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex as TokioMutex;
use crate::gems::{GemPreview, Gem};
use crate::agents::project_agent::{ProjectResearchAgent, ProjectResearchResults, ProjectSummaryResult};
use crate::agents::chatbot::ChatMessage;
use super::store::*;

#[tauri::command]
pub async fn create_project(
    title: String,
    description: Option<String>,
    objective: Option<String>,
    project_store: State<'_, Arc<dyn ProjectStore>>,
) -> Result<Project, String> {
    project_store.create(CreateProject { title, description, objective }).await
}

#[tauri::command]
pub async fn list_projects(
    project_store: State<'_, Arc<dyn ProjectStore>>,
) -> Result<Vec<ProjectPreview>, String> {
    project_store.list().await
}

#[tauri::command]
pub async fn get_project(
    id: String,
    project_store: State<'_, Arc<dyn ProjectStore>>,
) -> Result<ProjectDetail, String> {
    project_store.get(&id).await
}

#[tauri::command]
pub async fn update_project(
    id: String,
    title: Option<String>,
    description: Option<String>,
    objective: Option<String>,
    status: Option<String>,
    project_store: State<'_, Arc<dyn ProjectStore>>,
) -> Result<Project, String> {
    project_store.update(&id, UpdateProject { title, description, objective, status }).await
}

#[tauri::command]
pub async fn delete_project(
    id: String,
    project_store: State<'_, Arc<dyn ProjectStore>>,
) -> Result<(), String> {
    project_store.delete(&id).await
}

#[tauri::command]
pub async fn add_gems_to_project(
    project_id: String,
    gem_ids: Vec<String>,
    project_store: State<'_, Arc<dyn ProjectStore>>,
) -> Result<usize, String> {
    project_store.add_gems(&project_id, &gem_ids).await
}

#[tauri::command]
pub async fn remove_gem_from_project(
    project_id: String,
    gem_id: String,
    project_store: State<'_, Arc<dyn ProjectStore>>,
) -> Result<(), String> {
    project_store.remove_gem(&project_id, &gem_id).await
}

#[tauri::command]
pub async fn get_project_gems(
    project_id: String,
    query: Option<String>,
    limit: Option<usize>,
    project_store: State<'_, Arc<dyn ProjectStore>>,
) -> Result<Vec<GemPreview>, String> {
    project_store.get_project_gems(&project_id, query.as_deref(), limit).await
}

#[tauri::command]
pub async fn get_gem_projects(
    gem_id: String,
    project_store: State<'_, Arc<dyn ProjectStore>>,
) -> Result<Vec<ProjectPreview>, String> {
    project_store.get_gem_projects(&gem_id).await
}

// ── Agent Commands ──

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

// ── Research State Persistence ──

/// Save research chat state for a project (opaque JSON string).
#[tauri::command]
pub async fn save_project_research_state(
    project_id: String,
    state: String,
) -> Result<(), String> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| "Failed to determine app data directory".to_string())?;
    let project_dir = data_dir
        .join("com.jarvis.app")
        .join("projects")
        .join(&project_id);

    tokio::fs::create_dir_all(&project_dir)
        .await
        .map_err(|e| format!("Failed to create project directory: {}", e))?;

    let file_path = project_dir.join("research_state.json");
    tokio::fs::write(&file_path, state.as_bytes())
        .await
        .map_err(|e| format!("Failed to save research state: {}", e))?;

    Ok(())
}

/// Load research chat state for a project. Returns None if no saved state exists.
#[tauri::command]
pub async fn load_project_research_state(
    project_id: String,
) -> Result<Option<String>, String> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| "Failed to determine app data directory".to_string())?;
    let file_path = data_dir
        .join("com.jarvis.app")
        .join("projects")
        .join(&project_id)
        .join("research_state.json");

    if !file_path.exists() {
        return Ok(None);
    }

    let content = tokio::fs::read_to_string(&file_path)
        .await
        .map_err(|e| format!("Failed to read research state: {}", e))?;

    Ok(Some(content))
}

/// Delete saved research state for a project (used by "New Research" reset).
#[tauri::command]
pub async fn clear_project_research_state(
    project_id: String,
) -> Result<(), String> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| "Failed to determine app data directory".to_string())?;
    let file_path = data_dir
        .join("com.jarvis.app")
        .join("projects")
        .join(&project_id)
        .join("research_state.json");

    if file_path.exists() {
        tokio::fs::remove_file(&file_path)
            .await
            .map_err(|e| format!("Failed to clear research state: {}", e))?;
    }

    Ok(())
}

// ── Summary Checkpoint Commands ──

/// Generate a summary checkpoint and auto-save to disk.
#[tauri::command]
pub async fn generate_project_summary_checkpoint(
    project_id: String,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<ProjectSummaryResult, String> {
    let agent = agent.lock().await;
    let result = agent.generate_summary_checkpoint(&project_id).await?;

    // Auto-save summary to disk as versioned files
    let data_dir = dirs::data_dir()
        .ok_or_else(|| "Failed to determine app data directory".to_string())?;
    let summaries_dir = data_dir
        .join("com.jarvis.app")
        .join("projects")
        .join(&project_id)
        .join("summaries");

    tokio::fs::create_dir_all(&summaries_dir)
        .await
        .map_err(|e| format!("Failed to create summaries directory: {}", e))?;

    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S").to_string();

    // Save human-readable summary as .md
    let md_path = summaries_dir.join(format!("summary_{}.md", timestamp));
    tokio::fs::write(&md_path, result.summary.as_bytes())
        .await
        .map_err(|e| format!("Failed to save summary .md: {}", e))?;

    // Save full result as .json (includes composite_doc for Q&A)
    let json_content = serde_json::to_string(&result)
        .map_err(|e| format!("Failed to serialize summary: {}", e))?;
    let json_path = summaries_dir.join(format!("summary_{}.json", timestamp));
    tokio::fs::write(&json_path, json_content.as_bytes())
        .await
        .map_err(|e| format!("Failed to save summary .json: {}", e))?;

    eprintln!(
        "Projects/Summary: Auto-saved summary to {}",
        summaries_dir.display()
    );

    Ok(result)
}

/// Save a reviewed summary as a gem checkpoint.
#[tauri::command]
pub async fn save_project_summary_checkpoint(
    project_id: String,
    summary_content: String,
    composite_doc: String,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<Gem, String> {
    let agent = agent.lock().await;
    agent.save_summary_checkpoint(&project_id, &summary_content, &composite_doc).await
}

/// Answer a question about a generated summary.
#[tauri::command]
pub async fn send_summary_question(
    question: String,
    summary: String,
    composite_doc: String,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<String, String> {
    let agent = agent.lock().await;
    agent.send_summary_question(&question, &summary, &composite_doc).await
}

/// Load the latest saved summary checkpoint from disk.
#[tauri::command]
pub async fn get_latest_project_summary_checkpoint(
    project_id: String,
) -> Result<Option<ProjectSummaryResult>, String> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| "Failed to determine app data directory".to_string())?;
    let summaries_dir = data_dir
        .join("com.jarvis.app")
        .join("projects")
        .join(&project_id)
        .join("summaries");

    if !summaries_dir.exists() {
        return Ok(None);
    }

    // Find the latest summary_*.json file (timestamps sort lexicographically)
    let mut entries = tokio::fs::read_dir(&summaries_dir)
        .await
        .map_err(|e| format!("Failed to read summaries directory: {}", e))?;

    let mut latest_json: Option<std::path::PathBuf> = None;

    while let Some(entry) = entries.next_entry().await
        .map_err(|e| format!("Failed to read directory entry: {}", e))?
    {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with("summary_") && name_str.ends_with(".json") {
            match &latest_json {
                Some(prev) => {
                    if entry.path() > *prev {
                        latest_json = Some(entry.path());
                    }
                }
                None => latest_json = Some(entry.path()),
            }
        }
    }

    let json_path = match latest_json {
        Some(p) => p,
        None => return Ok(None),
    };

    let content = tokio::fs::read_to_string(&json_path)
        .await
        .map_err(|e| format!("Failed to read summary file: {}", e))?;

    let result: ProjectSummaryResult = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse summary file: {}", e))?;

    eprintln!(
        "Projects/Summary: Loaded latest checkpoint from {}",
        json_path.display()
    );

    Ok(Some(result))
}
