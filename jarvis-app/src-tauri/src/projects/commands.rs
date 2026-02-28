use std::sync::Arc;
use tauri::State;
use crate::gems::GemPreview;
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
