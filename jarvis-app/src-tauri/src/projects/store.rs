use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::gems::GemPreview;

/// A project — a named container grouping gems under a shared goal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub objective: Option<String>,
    pub status: String,      // "active" | "paused" | "completed" | "archived"
    pub created_at: String,  // ISO 8601
    pub updated_at: String,  // ISO 8601
}

/// Lightweight project for list views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectPreview {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub gem_count: usize,
    pub updated_at: String,
}

/// Full project with associated gems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDetail {
    pub project: Project,
    pub gem_count: usize,
    pub gems: Vec<GemPreview>,
}

/// Input for creating a project.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateProject {
    pub title: String,
    pub description: Option<String>,
    pub objective: Option<String>,
}

/// Input for updating a project. Only `Some` fields are applied.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProject {
    pub title: Option<String>,
    pub description: Option<String>,
    pub objective: Option<String>,
    pub status: Option<String>,
}

/// Backend-agnostic project store.
///
/// Tauri commands call this trait, never a concrete implementation.
/// Follows the same pattern as GemStore, KnowledgeStore, SearchResultProvider.
#[async_trait]
pub trait ProjectStore: Send + Sync {
    /// Create a new project. Sets id, status="active", and timestamps automatically.
    async fn create(&self, input: CreateProject) -> Result<Project, String>;

    /// List all projects, ordered by updated_at DESC.
    async fn list(&self) -> Result<Vec<ProjectPreview>, String>;

    /// Get a project by ID, including its associated gems.
    async fn get(&self, id: &str) -> Result<ProjectDetail, String>;

    /// Update a project. Only fields that are Some in UpdateProject are changed.
    async fn update(&self, id: &str, updates: UpdateProject) -> Result<Project, String>;

    /// Delete a project. CASCADE removes associations. Gems are NOT deleted.
    async fn delete(&self, id: &str) -> Result<(), String>;

    /// Add gems to a project. Uses INSERT OR IGNORE for idempotency.
    /// Returns the count of newly added associations.
    async fn add_gems(&self, project_id: &str, gem_ids: &[String]) -> Result<usize, String>;

    /// Remove a single gem from a project. The gem itself is NOT deleted.
    async fn remove_gem(&self, project_id: &str, gem_id: &str) -> Result<(), String>;

    /// Get gems associated with a project, with optional search and limit.
    async fn get_project_gems(
        &self,
        project_id: &str,
        query: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<GemPreview>, String>;

    /// Get all projects a gem belongs to.
    async fn get_gem_projects(&self, gem_id: &str) -> Result<Vec<ProjectPreview>, String>;
}
