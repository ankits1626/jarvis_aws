use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex as TokioMutex;
use crate::gems::GemPreview;
use crate::agents::project_agent::{ProjectResearchAgent, ProjectResearchResults};
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
