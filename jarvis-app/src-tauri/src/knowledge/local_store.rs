use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use dashmap::DashMap;
use tokio::sync::Mutex;
use chrono::Utc;

use crate::gems::Gem;
use crate::intelligence::provider::AvailabilityResult;
use crate::knowledge::store::*;
use crate::knowledge::assembler;

pub const CURRENT_KNOWLEDGE_VERSION: u32 = 1;

const KNOWN_SUBFILES: &[&str] = &[
    "meta.json",
    "content.md",
    "enrichment.md",
    "transcript.md",
    "copilot.md",
    "gem.md",
];

pub struct LocalKnowledgeStore {
    base_path: PathBuf,
    gem_locks: DashMap<String, Arc<Mutex<()>>>,
    event_emitter: Arc<dyn KnowledgeEventEmitter + Send + Sync>,
}

impl LocalKnowledgeStore {
    pub fn new(
        base_path: PathBuf,
        event_emitter: Arc<dyn KnowledgeEventEmitter + Send + Sync>,
    ) -> Self {
        Self {
            base_path,
            gem_locks: DashMap::new(),
            event_emitter,
        }
    }

    fn gem_folder(&self, gem_id: &str) -> PathBuf {
        self.base_path.join(gem_id)
    }

    fn get_lock(&self, gem_id: &str) -> Arc<Mutex<()>> {
        self.gem_locks
            .entry(gem_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    fn gem_to_meta(gem: &Gem) -> GemMeta {
        GemMeta {
            id: gem.id.clone(),
            source_type: gem.source_type.clone(),
            source_url: gem.source_url.clone(),
            domain: gem.domain.clone(),
            title: gem.title.clone(),
            author: gem.author.clone(),
            captured_at: gem.captured_at.clone(),
            project_id: None,
            source_meta: gem.source_meta.clone(),
            knowledge_version: CURRENT_KNOWLEDGE_VERSION,
            last_assembled: Utc::now().to_rfc3339(),
        }
    }

    async fn read_subfile_metadata(&self, gem_id: &str) -> Vec<KnowledgeSubfile> {
        let folder = self.gem_folder(gem_id);
        let mut subfiles = Vec::new();

        for filename in KNOWN_SUBFILES {
            let path = folder.join(filename);
            match tokio::fs::metadata(&path).await {
                Ok(metadata) => {
                    let last_modified = metadata
                        .modified()
                        .ok()
                        .and_then(|time| {
                            time.duration_since(std::time::UNIX_EPOCH)
                                .ok()
                                .map(|d| {
                                    chrono::DateTime::from_timestamp(d.as_secs() as i64, 0)
                                        .map(|dt| dt.to_rfc3339())
                                })
                        })
                        .flatten();

                    subfiles.push(KnowledgeSubfile {
                        filename: filename.to_string(),
                        exists: true,
                        size_bytes: metadata.len(),
                        last_modified,
                    });
                }
                Err(_) => {
                    subfiles.push(KnowledgeSubfile {
                        filename: filename.to_string(),
                        exists: false,
                        size_bytes: 0,
                        last_modified: None,
                    });
                }
            }
        }

        subfiles
    }

    async fn read_meta(&self, gem_id: &str) -> Result<GemMeta, String> {
        let path = self.gem_folder(gem_id).join("meta.json");
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| format!("Failed to read meta.json: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse meta.json: {}", e))
    }

    async fn write_all_subfiles(&self, gem: &Gem) -> Result<KnowledgeEntry, String> {
        let folder = self.gem_folder(&gem.id);

        tokio::fs::create_dir_all(&folder)
            .await
            .map_err(|e| format!("Failed to create knowledge folder: {}", e))?;

        let mut meta = Self::gem_to_meta(gem);

        let meta_json = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("Failed to serialize meta: {}", e))?;
        tokio::fs::write(folder.join("meta.json"), &meta_json)
            .await
            .map_err(|e| format!("Failed to write meta.json: {}", e))?;

        if let Some(ref content) = gem.content {
            if !content.is_empty() {
                let formatted = assembler::format_content(&gem.title, content);
                tokio::fs::write(folder.join("content.md"), &formatted)
                    .await
                    .map_err(|e| format!("Failed to write content.md: {}", e))?;
            }
        }

        if let Some(ref enrichment) = gem.ai_enrichment {
            let formatted = assembler::format_enrichment(enrichment);
            if !formatted.is_empty() {
                tokio::fs::write(folder.join("enrichment.md"), &formatted)
                    .await
                    .map_err(|e| format!("Failed to write enrichment.md: {}", e))?;
            }
        }

        if let Some(ref transcript) = gem.transcript {
            if !transcript.is_empty() {
                let language = gem.transcript_language.as_deref().unwrap_or("en");
                let formatted = assembler::format_transcript(transcript, language);
                tokio::fs::write(folder.join("transcript.md"), &formatted)
                    .await
                    .map_err(|e| format!("Failed to write transcript.md: {}", e))?;
            }
        }

        // Write copilot.md (if copilot data exists in source_meta)
        if let Some(copilot_data) = gem.source_meta.get("copilot") {
            if !copilot_data.is_null() {
                let formatted = assembler::format_copilot(copilot_data);
                if !formatted.is_empty() {
                    tokio::fs::write(folder.join("copilot.md"), &formatted)
                        .await
                        .map_err(|e| format!("Failed to write copilot.md: {}", e))?;
                }
            }
        }

        let assembled = assembler::assemble_gem_md(&folder, &meta).await?;
        tokio::fs::write(folder.join("gem.md"), &assembled)
            .await
            .map_err(|e| format!("Failed to write gem.md: {}", e))?;

        meta.last_assembled = Utc::now().to_rfc3339();
        let meta_json = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("Failed to serialize meta: {}", e))?;
        tokio::fs::write(folder.join("meta.json"), &meta_json)
            .await
            .map_err(|e| format!("Failed to write meta.json: {}", e))?;

        let subfiles = self.read_subfile_metadata(&gem.id).await;

        Ok(KnowledgeEntry {
            gem_id: gem.id.clone(),
            assembled,
            subfiles,
            version: CURRENT_KNOWLEDGE_VERSION,
            last_assembled: meta.last_assembled,
        })
    }
}

#[async_trait]
impl KnowledgeStore for LocalKnowledgeStore {
    async fn check_availability(&self) -> AvailabilityResult {
        match tokio::fs::create_dir_all(&self.base_path).await {
            Ok(_) => AvailabilityResult {
                available: true,
                reason: None,
            },
            Err(e) => AvailabilityResult {
                available: false,
                reason: Some(format!("Failed to create knowledge directory: {}", e)),
            },
        }
    }

    async fn create(&self, gem: &Gem) -> Result<KnowledgeEntry, String> {
        let lock = self.get_lock(&gem.id);
        let _guard = lock.lock().await;

        self.event_emitter.emit_progress(KnowledgeEvent::SubfileUpdated {
            gem_id: gem.id.clone(),
            filename: "all".to_string(),
            status: "writing".to_string(),
        });

        let result = self.write_all_subfiles(gem).await?;

        self.event_emitter.emit_progress(KnowledgeEvent::SubfileUpdated {
            gem_id: gem.id.clone(),
            filename: "all".to_string(),
            status: "done".to_string(),
        });

        Ok(result)
    }

    async fn get(&self, gem_id: &str) -> Result<Option<KnowledgeEntry>, String> {
        let folder = self.gem_folder(gem_id);
        let gem_md_path = folder.join("gem.md");

        if !gem_md_path.exists() {
            return Ok(None);
        }

        let assembled = tokio::fs::read_to_string(&gem_md_path)
            .await
            .map_err(|e| format!("Failed to read gem.md: {}", e))?;

        let meta = self.read_meta(gem_id).await?;
        let subfiles = self.read_subfile_metadata(gem_id).await;

        Ok(Some(KnowledgeEntry {
            gem_id: gem_id.to_string(),
            assembled,
            subfiles,
            version: meta.knowledge_version,
            last_assembled: meta.last_assembled,
        }))
    }

    async fn get_assembled(&self, gem_id: &str) -> Result<Option<String>, String> {
        let path = self.gem_folder(gem_id).join("gem.md");
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("Failed to read gem.md: {}", e)),
        }
    }

    async fn get_subfile(&self, gem_id: &str, filename: &str) -> Result<Option<String>, String> {
        let path = self.gem_folder(gem_id).join(filename);
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("Failed to read {}: {}", filename, e)),
        }
    }

    async fn exists(&self, gem_id: &str) -> Result<bool, String> {
        let path = self.gem_folder(gem_id).join("gem.md");
        Ok(path.exists())
    }

    async fn update_subfile(
        &self,
        gem_id: &str,
        filename: &str,
        content: &str,
    ) -> Result<(), String> {
        let lock = self.get_lock(gem_id);
        let _guard = lock.lock().await;

        let folder = self.gem_folder(gem_id);
        tokio::fs::create_dir_all(&folder)
            .await
            .map_err(|e| format!("Failed to create knowledge folder: {}", e))?;

        self.event_emitter.emit_progress(KnowledgeEvent::SubfileUpdated {
            gem_id: gem_id.to_string(),
            filename: filename.to_string(),
            status: "writing".to_string(),
        });

        tokio::fs::write(folder.join(filename), content)
            .await
            .map_err(|e| format!("Failed to write {}: {}", filename, e))?;

        self.event_emitter.emit_progress(KnowledgeEvent::SubfileUpdated {
            gem_id: gem_id.to_string(),
            filename: filename.to_string(),
            status: "assembling".to_string(),
        });

        let mut meta = self.read_meta(gem_id).await?;
        let assembled = assembler::assemble_gem_md(&folder, &meta).await?;
        tokio::fs::write(folder.join("gem.md"), &assembled)
            .await
            .map_err(|e| format!("Failed to write gem.md: {}", e))?;

        meta.last_assembled = Utc::now().to_rfc3339();
        let meta_json = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("Failed to serialize meta: {}", e))?;
        tokio::fs::write(folder.join("meta.json"), &meta_json)
            .await
            .map_err(|e| format!("Failed to write meta.json: {}", e))?;

        self.event_emitter.emit_progress(KnowledgeEvent::SubfileUpdated {
            gem_id: gem_id.to_string(),
            filename: filename.to_string(),
            status: "done".to_string(),
        });

        Ok(())
    }

    async fn reassemble(&self, gem_id: &str) -> Result<(), String> {
        let lock = self.get_lock(gem_id);
        let _guard = lock.lock().await;

        let folder = self.gem_folder(gem_id);
        let mut meta = self.read_meta(gem_id).await?;

        let assembled = assembler::assemble_gem_md(&folder, &meta).await?;
        tokio::fs::write(folder.join("gem.md"), &assembled)
            .await
            .map_err(|e| format!("Failed to write gem.md: {}", e))?;

        meta.last_assembled = Utc::now().to_rfc3339();
        let meta_json = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("Failed to serialize meta: {}", e))?;
        tokio::fs::write(folder.join("meta.json"), &meta_json)
            .await
            .map_err(|e| format!("Failed to write meta.json: {}", e))?;

        Ok(())
    }

    async fn delete(&self, gem_id: &str) -> Result<(), String> {
        let lock = self.get_lock(gem_id);
        let _guard = lock.lock().await;
        
        let folder = self.gem_folder(gem_id);
        
        if folder.exists() {
            tokio::fs::remove_dir_all(&folder)
                .await
                .map_err(|e| format!("Failed to delete knowledge folder: {}", e))?;
        }

        drop(_guard);
        self.gem_locks.remove(gem_id);

        Ok(())
    }

    async fn delete_subfile(&self, gem_id: &str, filename: &str) -> Result<(), String> {
        let lock = self.get_lock(gem_id);
        let _guard = lock.lock().await;

        let path = self.gem_folder(gem_id).join(filename);
        if path.exists() {
            tokio::fs::remove_file(&path)
                .await
                .map_err(|e| format!("Failed to delete {}: {}", filename, e))?;
        }

        let folder = self.gem_folder(gem_id);
        let mut meta = self.read_meta(gem_id).await?;
        let assembled = assembler::assemble_gem_md(&folder, &meta).await?;
        tokio::fs::write(folder.join("gem.md"), &assembled)
            .await
            .map_err(|e| format!("Failed to write gem.md: {}", e))?;

        meta.last_assembled = Utc::now().to_rfc3339();
        let meta_json = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("Failed to serialize meta: {}", e))?;
        tokio::fs::write(folder.join("meta.json"), &meta_json)
            .await
            .map_err(|e| format!("Failed to write meta.json: {}", e))?;

        Ok(())
    }

    async fn migrate_all(
        &self,
        gems: Vec<Gem>,
        event_emitter: &(dyn KnowledgeEventEmitter + Sync),
    ) -> Result<MigrationResult, String> {
        let total = gems.len();
        let mut created = 0;
        let skipped = 0;
        let mut failed = 0;
        let mut errors = Vec::new();

        for (i, gem) in gems.iter().enumerate() {
            event_emitter.emit_progress(KnowledgeEvent::MigrationProgress {
                current: i + 1,
                total,
                gem_id: gem.id.clone(),
                gem_title: gem.title.clone(),
                status: "generating".to_string(),
            });

            match self.create(gem).await {
                Ok(_) => {
                    created += 1;
                    event_emitter.emit_progress(KnowledgeEvent::MigrationProgress {
                        current: i + 1,
                        total,
                        gem_id: gem.id.clone(),
                        gem_title: gem.title.clone(),
                        status: "done".to_string(),
                    });
                }
                Err(e) => {
                    failed += 1;
                    errors.push((gem.id.clone(), e.clone()));
                    event_emitter.emit_progress(KnowledgeEvent::MigrationProgress {
                        current: i + 1,
                        total,
                        gem_id: gem.id.clone(),
                        gem_title: gem.title.clone(),
                        status: "failed".to_string(),
                    });
                }
            }
        }

        let result = MigrationResult {
            total,
            created,
            skipped,
            failed,
            errors,
        };

        event_emitter.emit_progress(KnowledgeEvent::MigrationComplete {
            result: result.clone(),
        });

        Ok(result)
    }

    async fn list_indexed(&self) -> Result<Vec<String>, String> {
        let mut gem_ids = Vec::new();

        let mut entries = tokio::fs::read_dir(&self.base_path)
            .await
            .map_err(|e| format!("Failed to read knowledge directory: {}", e))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| format!("Failed to read directory entry: {}", e))?
        {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if !name.starts_with('.') {
                        gem_ids.push(name.to_string());
                    }
                }
            }
        }

        Ok(gem_ids)
    }
}
