use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use crate::intelligence::provider::AvailabilityResult;
use crate::gems::Gem;

/// Machine-readable metadata for meta.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GemMeta {
    pub id: String,
    pub source_type: String,
    pub source_url: String,
    pub domain: String,
    pub title: String,
    pub author: Option<String>,
    pub captured_at: String,
    pub project_id: Option<String>,
    pub source_meta: serde_json::Value,
    pub knowledge_version: u32,
    pub last_assembled: String,
}

/// Knowledge entry returned by get()
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub gem_id: String,
    pub assembled: String,
    pub subfiles: Vec<KnowledgeSubfile>,
    pub version: u32,
    pub last_assembled: String,
}

/// Subfile metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSubfile {
    pub filename: String,
    pub exists: bool,
    pub size_bytes: u64,
    pub last_modified: Option<String>,
}

/// Migration result
#[derive(Debug, Clone, Serialize)]
pub struct MigrationResult {
    pub total: usize,
    pub created: usize,
    pub skipped: usize,
    pub failed: usize,
    pub errors: Vec<(String, String)>,
}

/// Knowledge event for progress tracking
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum KnowledgeEvent {
    SubfileUpdated {
        gem_id: String,
        filename: String,
        status: String,
    },
    MigrationProgress {
        current: usize,
        total: usize,
        gem_id: String,
        gem_title: String,
        status: String,
    },
    MigrationComplete {
        result: MigrationResult,
    },
}

/// Event emitter trait for knowledge file operations
pub trait KnowledgeEventEmitter: Send + Sync {
    fn emit_progress(&self, event: KnowledgeEvent);
}

/// Tauri implementation of KnowledgeEventEmitter
pub struct TauriKnowledgeEventEmitter {
    app_handle: tauri::AppHandle,
}

impl TauriKnowledgeEventEmitter {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }
}

impl KnowledgeEventEmitter for TauriKnowledgeEventEmitter {
    fn emit_progress(&self, event: KnowledgeEvent) {
        if let Err(e) = self.app_handle.emit("knowledge-progress", &event) {
            eprintln!("Failed to emit knowledge progress event: {}", e);
        }
    }
}

/// Backend-agnostic knowledge store interface
#[async_trait]
pub trait KnowledgeStore: Send + Sync {
    /// Check if the knowledge store is available
    async fn check_availability(&self) -> AvailabilityResult;

    /// Create knowledge entry for a new gem
    async fn create(&self, gem: &Gem) -> Result<KnowledgeEntry, String>;

    /// Get the full knowledge entry for a gem
    async fn get(&self, gem_id: &str) -> Result<Option<KnowledgeEntry>, String>;

    /// Get the assembled gem.md content
    async fn get_assembled(&self, gem_id: &str) -> Result<Option<String>, String>;

    /// Get a specific subfile's content
    async fn get_subfile(&self, gem_id: &str, filename: &str) -> Result<Option<String>, String>;

    /// Check if a gem has knowledge files
    async fn exists(&self, gem_id: &str) -> Result<bool, String>;

    /// Update a specific subfile and reassemble gem.md
    async fn update_subfile(
        &self,
        gem_id: &str,
        filename: &str,
        content: &str,
    ) -> Result<(), String>;

    /// Force reassemble gem.md from existing subfiles
    async fn reassemble(&self, gem_id: &str) -> Result<(), String>;

    /// Delete all knowledge files for a gem
    async fn delete(&self, gem_id: &str) -> Result<(), String>;

    /// Delete a specific subfile and reassemble gem.md
    async fn delete_subfile(&self, gem_id: &str, filename: &str) -> Result<(), String>;

    /// Generate knowledge files for all gems (migration / rebuild)
    async fn migrate_all(
        &self,
        gems: Vec<Gem>,
        event_emitter: &(dyn KnowledgeEventEmitter + Sync),
    ) -> Result<MigrationResult, String>;

    /// List all gem_ids that have knowledge files
    async fn list_indexed(&self) -> Result<Vec<String>, String>;
}
