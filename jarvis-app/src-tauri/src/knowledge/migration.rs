use std::path::Path;
use crate::gems::{Gem, GemStore};
use crate::knowledge::store::*;
use crate::knowledge::KnowledgeStore;
use crate::knowledge::local_store::CURRENT_KNOWLEDGE_VERSION;

/// Check if migration is needed and run it
///
/// This function is called during app startup to ensure all gems have knowledge files.
/// It checks for a version marker file and runs migration if needed.
///
/// # Arguments
///
/// * `knowledge_store` - The knowledge store implementation
/// * `gem_store` - The gem store for loading gems
/// * `event_emitter` - Event emitter for progress notifications
/// * `knowledge_base_path` - Base path for knowledge files
///
/// # Returns
///
/// * `Ok(())` - Migration completed successfully or not needed
/// * `Err(String)` - Migration failed
pub async fn check_and_run_migration(
    knowledge_store: &dyn KnowledgeStore,
    gem_store: &dyn GemStore,
    event_emitter: &(dyn KnowledgeEventEmitter + Sync),
    knowledge_base_path: &Path,
) -> Result<(), String> {
    let version_file = knowledge_base_path.join(".version");

    // Check if version file exists
    let needs_migration = if version_file.exists() {
        // Read stored version
        let stored = tokio::fs::read_to_string(&version_file)
            .await
            .unwrap_or_default();
        let stored_version: u32 = stored.trim().parse().unwrap_or(0);

        if stored_version < CURRENT_KNOWLEDGE_VERSION {
            eprintln!(
                "Knowledge: version {} → {}, reassembly needed",
                stored_version, CURRENT_KNOWLEDGE_VERSION
            );
            true // version bump — reassemble all
        } else {
            eprintln!("Knowledge: up to date (version {})", stored_version);
            false
        }
    } else {
        eprintln!("Knowledge: no version marker, running initial migration");
        true
    };

    if !needs_migration {
        return Ok(());
    }

    // Load ALL gems for migration
    // GemStore::list() returns GemPreview (truncated), we need full Gem objects
    // Strategy: list all IDs, then get() each one
    let previews = gem_store
        .list(10000, 0)
        .await
        .map_err(|e| format!("Failed to list gems for migration: {}", e))?;

    let mut gems: Vec<Gem> = Vec::new();
    for preview in &previews {
        match gem_store.get(&preview.id).await {
            Ok(Some(gem)) => gems.push(gem),
            Ok(None) => eprintln!(
                "Knowledge migration: gem {} not found, skipping",
                preview.id
            ),
            Err(e) => eprintln!(
                "Knowledge migration: failed to load gem {}: {}",
                preview.id, e
            ),
        }
    }

    eprintln!("Knowledge: migrating {} gems", gems.len());

    // Run migration
    let result = knowledge_store.migrate_all(gems, event_emitter).await?;

    eprintln!(
        "Knowledge migration complete: {} created, {} skipped, {} failed",
        result.created, result.skipped, result.failed
    );

    // Write version marker
    tokio::fs::write(&version_file, CURRENT_KNOWLEDGE_VERSION.to_string())
        .await
        .map_err(|e| format!("Failed to write version marker: {}", e))?;

    Ok(())
}
