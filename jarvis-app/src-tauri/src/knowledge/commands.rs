use std::sync::Arc;
use tauri::State;
use crate::gems::GemStore;
use crate::knowledge::store::{KnowledgeEntry, KnowledgeStore};
use crate::intelligence::provider::AvailabilityResult;

#[tauri::command]
pub async fn get_gem_knowledge(
    gem_id: String,
    knowledge_store: State<'_, Arc<dyn KnowledgeStore>>,
) -> Result<Option<KnowledgeEntry>, String> {
    knowledge_store.get(&gem_id).await
}

#[tauri::command]
pub async fn get_gem_knowledge_assembled(
    gem_id: String,
    knowledge_store: State<'_, Arc<dyn KnowledgeStore>>,
) -> Result<Option<String>, String> {
    knowledge_store.get_assembled(&gem_id).await
}

#[tauri::command]
pub async fn regenerate_gem_knowledge(
    gem_id: String,
    knowledge_store: State<'_, Arc<dyn KnowledgeStore>>,
    gem_store: State<'_, Arc<dyn GemStore>>,
) -> Result<KnowledgeEntry, String> {
    let gem = gem_store
        .get(&gem_id)
        .await?
        .ok_or_else(|| format!("Gem '{}' not found", gem_id))?;
    knowledge_store.create(&gem).await
}

#[tauri::command]
pub async fn check_knowledge_availability(
    knowledge_store: State<'_, Arc<dyn KnowledgeStore>>,
) -> Result<AvailabilityResult, String> {
    Ok(knowledge_store.check_availability().await)
}

#[tauri::command]
pub async fn get_gem_knowledge_subfile(
    gem_id: String,
    filename: String,
    knowledge_store: State<'_, Arc<dyn KnowledgeStore>>,
) -> Result<Option<String>, String> {
    knowledge_store.get_subfile(&gem_id, &filename).await
}
