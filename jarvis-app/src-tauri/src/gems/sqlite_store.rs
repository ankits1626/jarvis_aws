use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub struct SqliteGemStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteGemStore {
    /// Initialize store at ~/.jarvis/gems.db
    pub fn new() -> Result<Self, String> {
        let home = dirs::home_dir()
            .ok_or("Could not find home directory")?;
        let db_path = home.join(".jarvis").join("gems.db");
        
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create .jarvis directory: {}", e))?;
        }
        
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open database: {}", e))?;
        
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        
        store.initialize_schema()?;
        Ok(store)
    }
    
    /// Initialize in-memory store for testing
    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory()
            .map_err(|e| format!("Failed to open in-memory database: {}", e))?;
        
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        
        store.initialize_schema()?;
        Ok(store)
    }
    
    /// Get a clone of the database connection for sharing with other stores
    pub fn get_conn(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }
    
    fn initialize_schema(&self) -> Result<(), String> {
        let conn = self.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;
        
        // Main gems table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS gems (
                id TEXT PRIMARY KEY,
                source_type TEXT NOT NULL,
                source_url TEXT NOT NULL UNIQUE,
                domain TEXT NOT NULL,
                title TEXT NOT NULL,
                author TEXT,
                description TEXT,
                content TEXT,
                source_meta TEXT NOT NULL,
                captured_at TEXT NOT NULL,
                ai_enrichment TEXT
            )",
            [],
        ).map_err(|e| format!("Failed to create gems table: {}", e))?;
        
        // Migration: Add ai_enrichment column if it doesn't exist
        let mut stmt = conn.prepare("PRAGMA table_info(gems)")
            .map_err(|e| format!("Failed to prepare PRAGMA: {}", e))?;
        let columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| format!("Failed to query columns: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect columns: {}", e))?;
        
        if !columns.contains(&"ai_enrichment".to_string()) {
            // Add the column
            conn.execute("ALTER TABLE gems ADD COLUMN ai_enrichment TEXT", [])
                .map_err(|e| format!("Failed to add ai_enrichment column: {}", e))?;
            
            // Drop old triggers
            conn.execute("DROP TRIGGER IF EXISTS gems_ai", [])
                .map_err(|e| format!("Failed to drop gems_ai trigger: {}", e))?;
            conn.execute("DROP TRIGGER IF EXISTS gems_ad", [])
                .map_err(|e| format!("Failed to drop gems_ad trigger: {}", e))?;
            conn.execute("DROP TRIGGER IF EXISTS gems_au", [])
                .map_err(|e| format!("Failed to drop gems_au trigger: {}", e))?;
        }
        
        // Migration: Add transcript column if it doesn't exist
        if !columns.contains(&"transcript".to_string()) {
            conn.execute("ALTER TABLE gems ADD COLUMN transcript TEXT", [])
                .map_err(|e| format!("Failed to add transcript column: {}", e))?;
            // Drop old FTS table so it gets recreated with the transcript column
            conn.execute("DROP TABLE IF EXISTS gems_fts", [])
                .map_err(|e| format!("Failed to drop old FTS table: {}", e))?;
        }

        // Migration: Add transcript_language column if it doesn't exist
        if !columns.contains(&"transcript_language".to_string()) {
            conn.execute("ALTER TABLE gems ADD COLUMN transcript_language TEXT", [])
                .map_err(|e| format!("Failed to add transcript_language column: {}", e))?;
        }

        // Ensure FTS table schema is up-to-date (handles case where transcript column
        // was added to gems table but FTS wasn't recreated)
        let fts_needs_rebuild: bool = conn.query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='gems_fts'",
            [],
            |row| row.get::<_, String>(0),
        ).ok().map_or(false, |sql| !sql.contains("transcript"));

        if fts_needs_rebuild {
            conn.execute("DROP TABLE IF EXISTS gems_fts", [])
                .map_err(|e| format!("Failed to drop outdated FTS table: {}", e))?;
        }

        // FTS5 virtual table for full-text search
        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS gems_fts USING fts5(
                title,
                description,
                content,
                transcript,
                content=gems,
                content_rowid=rowid
            )",
            [],
        ).map_err(|e| format!("Failed to create FTS5 table: {}", e))?;

        // Rebuild FTS index if we had to drop an outdated FTS table
        if fts_needs_rebuild {
            conn.execute("INSERT INTO gems_fts(gems_fts) VALUES('rebuild')", [])
                .map_err(|e| format!("Failed to rebuild FTS index: {}", e))?;
        }

        // Triggers to keep FTS5 in sync (always recreate to ensure they have the latest logic)
        conn.execute("DROP TRIGGER IF EXISTS gems_ai", [])
            .map_err(|e| format!("Failed to drop gems_ai trigger: {}", e))?;
        conn.execute(
            "CREATE TRIGGER gems_ai AFTER INSERT ON gems BEGIN
                INSERT INTO gems_fts(rowid, title, description, content, transcript)
                VALUES (
                    new.rowid,
                    new.title,
                    new.description,
                    COALESCE(new.content, '') || ' ' || COALESCE(json_extract(new.ai_enrichment, '$.summary'), ''),
                    COALESCE(new.transcript, '')
                );
            END",
            [],
        ).map_err(|e| format!("Failed to create insert trigger: {}", e))?;
        
        conn.execute("DROP TRIGGER IF EXISTS gems_ad", [])
            .map_err(|e| format!("Failed to drop gems_ad trigger: {}", e))?;
        conn.execute(
            "CREATE TRIGGER gems_ad AFTER DELETE ON gems BEGIN
                INSERT INTO gems_fts(gems_fts, rowid, title, description, content, transcript)
                VALUES(
                    'delete',
                    old.rowid,
                    old.title,
                    old.description,
                    COALESCE(old.content, '') || ' ' || COALESCE(json_extract(old.ai_enrichment, '$.summary'), ''),
                    COALESCE(old.transcript, '')
                );
            END",
            [],
        ).map_err(|e| format!("Failed to create delete trigger: {}", e))?;
        
        conn.execute("DROP TRIGGER IF EXISTS gems_au", [])
            .map_err(|e| format!("Failed to drop gems_au trigger: {}", e))?;
        conn.execute(
            "CREATE TRIGGER gems_au AFTER UPDATE ON gems BEGIN
                INSERT INTO gems_fts(gems_fts, rowid, title, description, content, transcript)
                VALUES(
                    'delete',
                    old.rowid,
                    old.title,
                    old.description,
                    COALESCE(old.content, '') || ' ' || COALESCE(json_extract(old.ai_enrichment, '$.summary'), ''),
                    COALESCE(old.transcript, '')
                );
                INSERT INTO gems_fts(rowid, title, description, content, transcript)
                VALUES (
                    new.rowid,
                    new.title,
                    new.description,
                    COALESCE(new.content, '') || ' ' || COALESCE(json_extract(new.ai_enrichment, '$.summary'), ''),
                    COALESCE(new.transcript, '')
                );
            END",
            [],
        ).map_err(|e| format!("Failed to create update trigger: {}", e))?;
        
        Ok(())
    }
}

use async_trait::async_trait;
use rusqlite::{params, OptionalExtension};
use crate::gems::store::{Gem, GemPreview, GemStore};

impl SqliteGemStore {
    fn row_to_gem(row: &rusqlite::Row) -> rusqlite::Result<Gem> {
        // Read ai_enrichment as Option<String> and deserialize to Option<Value>
        let ai_enrichment: Option<serde_json::Value> = row.get::<_, Option<String>>(10)?
            .and_then(|s| serde_json::from_str(&s).ok());
        
        Ok(Gem {
            id: row.get(0)?,
            source_type: row.get(1)?,
            source_url: row.get(2)?,
            domain: row.get(3)?,
            title: row.get(4)?,
            author: row.get(5)?,
            description: row.get(6)?,
            content: row.get(7)?,
            source_meta: serde_json::from_str(&row.get::<_, String>(8)?)
                .unwrap_or(serde_json::Value::Null),
            captured_at: row.get(9)?,
            ai_enrichment,
            transcript: row.get(11)?,
            transcript_language: row.get(12)?,
        })
    }
    
    fn gem_to_preview(gem: &Gem) -> GemPreview {
        // Extract tags, summary, and enrichment source from ai_enrichment JSON
        let (tags, summary, enrichment_source) = if let Some(ai_enrichment) = &gem.ai_enrichment {
            let tags = ai_enrichment
                .get("tags")
                .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok());

            let summary = ai_enrichment
                .get("summary")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Build enrichment source string from provider + model
            let provider = ai_enrichment.get("provider").and_then(|v| v.as_str());
            let model = ai_enrichment.get("model").and_then(|v| v.as_str());
            let source = match (provider, model) {
                (Some(p), Some(m)) => Some(format!("{} / {}", p, m)),
                (Some(p), None) => Some(p.to_string()),
                _ => None,
            };

            (tags, summary, source)
        } else {
            (None, None, None)
        };

        GemPreview {
            id: gem.id.clone(),
            source_type: gem.source_type.clone(),
            source_url: gem.source_url.clone(),
            domain: gem.domain.clone(),
            title: gem.title.clone(),
            author: gem.author.clone(),
            description: gem.description.clone(),
            content_preview: gem.content.as_ref().map(|c| {
                // Safe UTF-8 truncation by character count, not byte offset
                if c.chars().count() > 200 {
                    format!("{}...", c.chars().take(200).collect::<String>())
                } else {
                    c.clone()
                }
            }),
            captured_at: gem.captured_at.clone(),
            tags,
            summary,
            enrichment_source,
            transcript_language: gem.transcript_language.clone(),
        }
    }
}

#[async_trait]
impl GemStore for SqliteGemStore {
    async fn save(&self, gem: Gem) -> Result<Gem, String> {
        let conn = self.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;
        
        // Serialize ai_enrichment to JSON string (or NULL if None)
        let ai_enrichment_str = gem.ai_enrichment.as_ref()
            .map(|v| v.to_string());
        
        conn.execute(
            "INSERT INTO gems (id, source_type, source_url, domain, title, author, 
                description, content, source_meta, captured_at, ai_enrichment, transcript, transcript_language)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(source_url) DO UPDATE SET
                title = excluded.title,
                author = excluded.author,
                description = excluded.description,
                content = excluded.content,
                source_meta = excluded.source_meta,
                captured_at = excluded.captured_at,
                ai_enrichment = excluded.ai_enrichment,
                transcript = excluded.transcript,
                transcript_language = excluded.transcript_language",
            params![
                gem.id,
                gem.source_type,
                gem.source_url,
                gem.domain,
                gem.title,
                gem.author,
                gem.description,
                gem.content,
                gem.source_meta.to_string(),
                gem.captured_at,
                ai_enrichment_str,
                gem.transcript,
                gem.transcript_language,
            ],
        ).map_err(|e| format!("Failed to save gem: {}", e))?;
        
        // Query back the actual row to get the correct ID (in case of conflict, the original ID is kept)
        let mut stmt = conn.prepare(
            "SELECT id, source_type, source_url, domain, title, author, 
                description, content, source_meta, captured_at, ai_enrichment, transcript, transcript_language
            FROM gems WHERE source_url = ?1"
        ).map_err(|e| format!("Failed to prepare query: {}", e))?;
        
        let saved_gem = stmt.query_row(params![gem.source_url], Self::row_to_gem)
            .map_err(|e| format!("Failed to query saved gem: {}", e))?;
        
        Ok(saved_gem)
    }
    
    async fn get(&self, id: &str) -> Result<Option<Gem>, String> {
        let conn = self.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;
        
        let mut stmt = conn.prepare(
            "SELECT id, source_type, source_url, domain, title, author, 
                description, content, source_meta, captured_at, ai_enrichment, transcript, transcript_language
            FROM gems WHERE id = ?1"
        ).map_err(|e| format!("Failed to prepare query: {}", e))?;
        
        let result = stmt.query_row(params![id], Self::row_to_gem)
            .optional()
            .map_err(|e| format!("Failed to query gem: {}", e))?;
        
        Ok(result)
    }
    
    async fn list(&self, limit: usize, offset: usize) -> Result<Vec<GemPreview>, String> {
        let conn = self.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;
        
        // NOTE: This fetches full content and truncates in Rust. For large gems (50K+ chars),
        // this is wasteful. Could optimize with SUBSTR(content, 1, 600) in SQL (600 bytes ~= 200 UTF-8 chars),
        // but SUBSTR works on bytes, not characters, risking mid-character splits.
        // Current approach prioritizes correctness over performance.
        let mut stmt = conn.prepare(
            "SELECT id, source_type, source_url, domain, title, author, 
                description, content, source_meta, captured_at, ai_enrichment, transcript, transcript_language
            FROM gems
            ORDER BY captured_at DESC
            LIMIT ?1 OFFSET ?2"
        ).map_err(|e| format!("Failed to prepare statement: {}", e))?;
        
        let gems = stmt.query_map(params![limit, offset], Self::row_to_gem)
            .map_err(|e| format!("Failed to query gems: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect gems: {}", e))?;
        
        Ok(gems.iter().map(Self::gem_to_preview).collect())
    }
    
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<GemPreview>, String> {
        // Handle empty query by delegating to list()
        if query.trim().is_empty() {
            return self.list(limit, 0).await;
        }
        
        let conn = self.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;
        
        // NOTE: Same performance consideration as list() - fetches full content for truncation.
        let mut stmt = conn.prepare(
            "SELECT g.id, g.source_type, g.source_url, g.domain, g.title, g.author,
                g.description, g.content, g.source_meta, g.captured_at, g.ai_enrichment, g.transcript, g.transcript_language
            FROM gems g
            INNER JOIN gems_fts fts ON g.rowid = fts.rowid
            WHERE gems_fts MATCH ?1
            ORDER BY rank
            LIMIT ?2"
        ).map_err(|e| {
            // Handle FTS5 syntax errors with user-friendly messages
            let error_msg = e.to_string();
            if error_msg.contains("fts5: syntax error") || error_msg.contains("unterminated string") {
                format!("Invalid search query syntax. Please check for unmatched quotes or special characters.")
            } else {
                format!("Failed to prepare search statement: {}", e)
            }
        })?;
        
        let gems = stmt.query_map(params![query, limit], Self::row_to_gem)
            .map_err(|e| {
                // Handle FTS5 query execution errors
                let error_msg = e.to_string();
                if error_msg.contains("fts5: syntax error") || error_msg.contains("unterminated string") {
                    format!("Invalid search query syntax. Please check for unmatched quotes or special characters.")
                } else {
                    format!("Failed to search gems: {}", e)
                }
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                // Handle FTS5 errors during result collection
                let error_msg = e.to_string();
                if error_msg.contains("fts5: syntax error") || error_msg.contains("unterminated string") {
                    format!("Invalid search query syntax. Please check for unmatched quotes or special characters.")
                } else {
                    format!("Failed to collect search results: {}", e)
                }
            })?;
        
        Ok(gems.iter().map(Self::gem_to_preview).collect())
    }
    
    async fn filter_by_tag(&self, tag: &str, limit: usize, offset: usize) -> Result<Vec<GemPreview>, String> {
        let conn = self.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;
        
        let mut stmt = conn.prepare(
            "SELECT DISTINCT g.id, g.source_type, g.source_url, g.domain, g.title, g.author,
                g.description, g.content, g.source_meta, g.captured_at, g.ai_enrichment, g.transcript, g.transcript_language
             FROM gems g, json_each(json_extract(g.ai_enrichment, '$.tags'))
             WHERE json_each.value = ?1
             ORDER BY g.captured_at DESC
             LIMIT ?2 OFFSET ?3"
        ).map_err(|e| format!("Failed to prepare statement: {}", e))?;
        
        let gems = stmt.query_map(params![tag, limit, offset], Self::row_to_gem)
            .map_err(|e| format!("Failed to query gems by tag: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect gems: {}", e))?;
        
        Ok(gems.iter().map(Self::gem_to_preview).collect())
    }
    
    async fn delete(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;
        
        let rows_affected = conn.execute(
            "DELETE FROM gems WHERE id = ?1",
            params![id],
        ).map_err(|e| format!("Failed to delete gem: {}", e))?;
        
        if rows_affected == 0 {
            return Err(format!("Gem with id '{}' not found", id));
        }
        
        Ok(())
    }
    
    async fn find_by_recording_filename(&self, filename: &str) -> Result<Option<GemPreview>, String> {
        let conn = self.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;
        
        let mut stmt = conn.prepare(
            "SELECT id, source_type, source_url, domain, title, author, 
                description, content, source_meta, captured_at, ai_enrichment, transcript, transcript_language
            FROM gems
            WHERE json_extract(source_meta, '$.recording_filename') = ?1
            ORDER BY captured_at DESC
            LIMIT 1"
        ).map_err(|e| format!("Failed to prepare query: {}", e))?;
        
        let result = stmt.query_row(params![filename], Self::row_to_gem)
            .optional()
            .map_err(|e| format!("Failed to query gem: {}", e))?;
        
        Ok(result.map(|gem| Self::gem_to_preview(&gem)))
    }

    async fn update_title(&self, id: &str, title: &str) -> Result<(), String> {
        let conn = self.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;

        let rows_affected = conn.execute(
            "UPDATE gems SET title = ?1 WHERE id = ?2",
            params![title, id],
        ).map_err(|e| format!("Failed to update gem title: {}", e))?;

        if rows_affected == 0 {
            return Err(format!("Gem with id '{}' not found", id));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_schema_initialization_creates_gems_table() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        let conn = store.conn.lock().unwrap();
        
        // Query sqlite_master to check if gems table exists
        let table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='gems'",
                [],
                |row| row.get(0),
            )
            .map(|count: i32| count == 1)
            .unwrap_or(false);
        
        assert!(table_exists, "gems table should exist");
        
        // Verify table has correct columns
        let mut stmt = conn.prepare("PRAGMA table_info(gems)").unwrap();
        let columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        
        let expected_columns = vec![
            "id", "source_type", "source_url", "domain", "title",
            "author", "description", "content", "source_meta", "captured_at", "ai_enrichment"
        ];
        
        assert_eq!(columns, expected_columns, "gems table should have correct columns");
    }

    #[test]
    fn test_schema_initialization_creates_fts_table() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        let conn = store.conn.lock().unwrap();
        
        // Query sqlite_master to check if gems_fts virtual table exists
        let fts_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='gems_fts'",
                [],
                |row| row.get(0),
            )
            .map(|count: i32| count == 1)
            .unwrap_or(false);
        
        assert!(fts_exists, "gems_fts virtual table should exist");
    }

    #[test]
    fn test_schema_initialization_creates_triggers() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        let conn = store.conn.lock().unwrap();
        
        // Query sqlite_master to check if all three triggers exist
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='trigger' ORDER BY name")
            .unwrap();
        
        let triggers: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        
        let expected_triggers = vec!["gems_ad", "gems_ai", "gems_au"];
        
        assert_eq!(triggers, expected_triggers, "All three FTS5 sync triggers should exist");
    }

    #[tokio::test]
    async fn test_upsert_behavior() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        let source_url = "https://example.com/article";
        
        // First save
        let gem1 = Gem {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: "Article".to_string(),
            source_url: source_url.to_string(),
            domain: "example.com".to_string(),
            title: "Original Title".to_string(),
            author: Some("Original Author".to_string()),
            description: Some("Original description".to_string()),
            content: Some("Original content with unique keyword FIRSTVERSION".to_string()),
            source_meta: serde_json::json!({"version": 1}),
            captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };
        
        let saved1 = store.save(gem1.clone()).await
            .expect("First save should succeed");
        
        // Second save with same source_url but different content
        let gem2 = Gem {
            id: uuid::Uuid::new_v4().to_string(), // Different ID
            source_type: "Article".to_string(),
            source_url: source_url.to_string(), // Same URL
            domain: "example.com".to_string(),
            title: "Updated Title".to_string(),
            author: Some("Updated Author".to_string()),
            description: Some("Updated description".to_string()),
            content: Some("Updated content with unique keyword SECONDVERSION".to_string()),
            source_meta: serde_json::json!({"version": 2}),
            captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };
        
        let saved2 = store.save(gem2.clone()).await
            .expect("Second save should succeed");
        
        // Verify only one gem exists by counting rows
        let conn = store.conn.lock().unwrap();
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM gems WHERE source_url = ?1",
            params![source_url],
            |row| row.get(0),
        ).expect("Count query should succeed");
        
        assert_eq!(count, 1, "Only one gem should exist for the source_url");
        
        // Verify the second save updated the existing record
        // The ID should be the original ID (from first save), not the new one
        assert_eq!(saved2.id, saved1.id, "ID should remain the same after upsert");
        assert_eq!(saved2.title, "Updated Title", "Title should be updated");
        assert_eq!(saved2.author, Some("Updated Author".to_string()), "Author should be updated");
        assert_eq!(saved2.description, Some("Updated description".to_string()), "Description should be updated");
        assert_eq!(saved2.content, Some("Updated content with unique keyword SECONDVERSION".to_string()), "Content should be updated");
        
        // Verify FTS5 sync after upsert: search for updated content returns the gem
        let search_results = conn.query_row(
            "SELECT g.id, g.title FROM gems g 
             INNER JOIN gems_fts fts ON g.rowid = fts.rowid 
             WHERE gems_fts MATCH ?1",
            params!["SECONDVERSION"],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        ).expect("Search for updated content should succeed");
        
        assert_eq!(search_results.0, saved1.id, "Search should find the gem by updated content");
        assert_eq!(search_results.1, "Updated Title", "Search should return updated title");
        
        // Test that old content is no longer searchable after upsert
        let old_content_result = conn.query_row(
            "SELECT COUNT(*) FROM gems g 
             INNER JOIN gems_fts fts ON g.rowid = fts.rowid 
             WHERE gems_fts MATCH ?1",
            params!["FIRSTVERSION"],
            |row| row.get::<_, i32>(0),
        ).expect("Search for old content should execute");
        
        assert_eq!(old_content_result, 0, "Old content should not be searchable after upsert");
    }

    #[tokio::test]
    async fn test_list_returns_gems_ordered_by_captured_at_desc() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        // Create three gems with different timestamps
        let now = chrono::Utc::now();
        
        let gem1 = Gem {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: "Article".to_string(),
            source_url: "https://example.com/1".to_string(),
            domain: "example.com".to_string(),
            title: "First Gem".to_string(),
            author: None,
            description: None,
            content: Some("First content".to_string()),
            source_meta: serde_json::json!({}),
            captured_at: (now - chrono::Duration::hours(2)).to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };
        
        let gem2 = Gem {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: "Article".to_string(),
            source_url: "https://example.com/2".to_string(),
            domain: "example.com".to_string(),
            title: "Second Gem".to_string(),
            author: None,
            description: None,
            content: Some("Second content".to_string()),
            source_meta: serde_json::json!({}),
            captured_at: now.to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };
        
        let gem3 = Gem {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: "Article".to_string(),
            source_url: "https://example.com/3".to_string(),
            domain: "example.com".to_string(),
            title: "Third Gem".to_string(),
            author: None,
            description: None,
            content: Some("Third content".to_string()),
            source_meta: serde_json::json!({}),
            captured_at: (now - chrono::Duration::hours(1)).to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };
        
        // Save in random order
        store.save(gem1.clone()).await.expect("Save gem1");
        store.save(gem3.clone()).await.expect("Save gem3");
        store.save(gem2.clone()).await.expect("Save gem2");
        
        // List all gems
        let results = store.list(10, 0).await.expect("List should succeed");
        
        // Verify order: most recent first (gem2, gem3, gem1)
        assert_eq!(results.len(), 3, "Should return 3 gems");
        assert_eq!(results[0].title, "Second Gem", "Most recent gem should be first");
        assert_eq!(results[1].title, "Third Gem", "Second most recent gem should be second");
        assert_eq!(results[2].title, "First Gem", "Oldest gem should be last");
    }

    #[tokio::test]
    async fn test_list_respects_limit_and_offset() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        // Create 5 gems
        for i in 0..5 {
            let gem = Gem {
                id: uuid::Uuid::new_v4().to_string(),
                source_type: "Article".to_string(),
                source_url: format!("https://example.com/{}", i),
                domain: "example.com".to_string(),
                title: format!("Gem {}", i),
                author: None,
                description: None,
                content: Some(format!("Content {}", i)),
                source_meta: serde_json::json!({}),
                captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
            };
            store.save(gem).await.expect("Save should succeed");
        }
        
        // Test limit
        let results = store.list(2, 0).await.expect("List should succeed");
        assert_eq!(results.len(), 2, "Should return only 2 gems");
        
        // Test offset
        let results_offset = store.list(2, 2).await.expect("List should succeed");
        assert_eq!(results_offset.len(), 2, "Should return 2 gems with offset");
        assert_ne!(results[0].id, results_offset[0].id, "Offset results should be different");
        
        // Test offset beyond available gems
        let results_beyond = store.list(10, 10).await.expect("List should succeed");
        assert_eq!(results_beyond.len(), 0, "Should return empty list when offset exceeds count");
    }

    #[tokio::test]
    async fn test_list_truncates_content_to_200_chars() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        // Create gem with long content (300 characters)
        let long_content = "a".repeat(300);
        let gem = Gem {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: "Article".to_string(),
            source_url: "https://example.com/long".to_string(),
            domain: "example.com".to_string(),
            title: "Long Content Gem".to_string(),
            author: None,
            description: None,
            content: Some(long_content.clone()),
            source_meta: serde_json::json!({}),
            captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };
        
        store.save(gem).await.expect("Save should succeed");
        
        let results = store.list(10, 0).await.expect("List should succeed");
        assert_eq!(results.len(), 1);
        
        let preview = &results[0];
        let preview_content = preview.content_preview.as_ref().unwrap();
        
        // Should be truncated to 200 chars + "..."
        assert_eq!(preview_content.chars().count(), 203, "Should be 200 chars + '...'");
        assert!(preview_content.ends_with("..."), "Should end with ellipsis");
        
        // Verify original content is not modified
        let retrieved = store.get(&preview.id).await.expect("Get should succeed").unwrap();
        assert_eq!(retrieved.content.as_ref().unwrap().len(), 300, "Original content should be unchanged");
    }

    #[tokio::test]
    async fn test_list_handles_short_content() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        // Create gem with short content (50 characters)
        let short_content = "This is a short content string.";
        let gem = Gem {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: "Article".to_string(),
            source_url: "https://example.com/short".to_string(),
            domain: "example.com".to_string(),
            title: "Short Content Gem".to_string(),
            author: None,
            description: None,
            content: Some(short_content.to_string()),
            source_meta: serde_json::json!({}),
            captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };
        
        store.save(gem).await.expect("Save should succeed");
        
        let results = store.list(10, 0).await.expect("List should succeed");
        assert_eq!(results.len(), 1);
        
        let preview = &results[0];
        let preview_content = preview.content_preview.as_ref().unwrap();
        
        // Should not be truncated
        assert_eq!(preview_content, short_content, "Short content should not be truncated");
        assert!(!preview_content.ends_with("..."), "Should not have ellipsis");
    }

    #[tokio::test]
    async fn test_list_handles_utf8_truncation() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        // Create gem with multi-byte UTF-8 characters (emoji + Chinese)
        let utf8_content = "Hello 世界 🌍 ".repeat(50); // Each repeat is ~15 chars, 50 repeats = ~750 chars
        let gem = Gem {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: "Article".to_string(),
            source_url: "https://example.com/utf8".to_string(),
            domain: "example.com".to_string(),
            title: "UTF-8 Content Gem".to_string(),
            author: None,
            description: None,
            content: Some(utf8_content.clone()),
            source_meta: serde_json::json!({}),
            captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };
        
        store.save(gem).await.expect("Save should succeed");
        
        let results = store.list(10, 0).await.expect("List should succeed");
        assert_eq!(results.len(), 1);
        
        let preview = &results[0];
        let preview_content = preview.content_preview.as_ref().unwrap();
        
        // Should be truncated to 200 characters (not bytes) + "..."
        assert_eq!(preview_content.chars().count(), 203, "Should be 200 chars + '...'");
        assert!(preview_content.ends_with("..."), "Should end with ellipsis");
        
        // Verify it's valid UTF-8 (no mid-character splits)
        assert!(std::str::from_utf8(preview_content.as_bytes()).is_ok(), "Should be valid UTF-8");
    }

    // Property 1: Save-Retrieve Round Trip
    // Validates: Requirements 3.1, 3.6
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_save_retrieve_round_trip(
            title in "[a-zA-Z0-9 ]{1,100}",
            url in "https://[a-z]+\\.com/[a-z]+",
            domain in "[a-z]+\\.com",
            source_type in "(Article|YouTube|Email|Chat)",
            author in proptest::option::of("[a-zA-Z ]{1,50}"),
            description in proptest::option::of("[a-zA-Z0-9 ]{1,200}"),
            content in proptest::option::of("[a-zA-Z0-9 ]{1,500}"),
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let store = SqliteGemStore::new_in_memory().unwrap();
                
                let gem = Gem {
                    id: uuid::Uuid::new_v4().to_string(),
                    source_type: source_type.clone(),
                    source_url: url.clone(),
                    domain: domain.clone(),
                    title: title.clone(),
                    author: author.clone(),
                    description: description.clone(),
                    content: content.clone(),
                    source_meta: serde_json::json!({"test": "data"}),
                    captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
                };
                
                let saved = store.save(gem.clone()).await.unwrap();
                let retrieved = store.get(&saved.id).await.unwrap().unwrap();
                
                // Verify all fields match
                prop_assert_eq!(&saved.id, &retrieved.id);
                prop_assert_eq!(&saved.source_type, &retrieved.source_type);
                prop_assert_eq!(&saved.source_url, &retrieved.source_url);
                prop_assert_eq!(&saved.domain, &retrieved.domain);
                prop_assert_eq!(&saved.title, &retrieved.title);
                prop_assert_eq!(&saved.author, &retrieved.author);
                prop_assert_eq!(&saved.description, &retrieved.description);
                prop_assert_eq!(&saved.content, &retrieved.content);
                prop_assert_eq!(&saved.source_meta, &retrieved.source_meta);
                prop_assert_eq!(&saved.captured_at, &retrieved.captured_at);
                
                Ok(())
            })?
        }
    }

    // Tests for search functionality (task 3.16)
    
    #[tokio::test]
    async fn test_search_finds_gems_by_title() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        // Create gems with different titles
        let gem1 = Gem {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: "Article".to_string(),
            source_url: "https://example.com/rust".to_string(),
            domain: "example.com".to_string(),
            title: "Introduction to Rust Programming".to_string(),
            author: None,
            description: Some("A guide to Rust".to_string()),
            content: Some("Rust is a systems programming language".to_string()),
            source_meta: serde_json::json!({}),
            captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };
        
        let gem2 = Gem {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: "Article".to_string(),
            source_url: "https://example.com/python".to_string(),
            domain: "example.com".to_string(),
            title: "Python for Beginners".to_string(),
            author: None,
            description: Some("Learn Python basics".to_string()),
            content: Some("Python is a high-level language".to_string()),
            source_meta: serde_json::json!({}),
            captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };
        
        store.save(gem1.clone()).await.expect("Save gem1");
        store.save(gem2.clone()).await.expect("Save gem2");
        
        // Search for "Rust"
        let results = store.search("Rust", 10).await.expect("Search should succeed");
        
        assert_eq!(results.len(), 1, "Should find one gem");
        assert_eq!(results[0].title, "Introduction to Rust Programming");
    }

    #[tokio::test]
    async fn test_search_finds_gems_by_content() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        let gem = Gem {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: "Article".to_string(),
            source_url: "https://example.com/async".to_string(),
            domain: "example.com".to_string(),
            title: "Programming Concepts".to_string(),
            author: None,
            description: None,
            content: Some("Async programming with tokio is powerful".to_string()),
            source_meta: serde_json::json!({}),
            captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };
        
        store.save(gem.clone()).await.expect("Save gem");
        
        // Search for "tokio" which is only in content
        let results = store.search("tokio", 10).await.expect("Search should succeed");
        
        assert_eq!(results.len(), 1, "Should find gem by content");
        assert_eq!(results[0].title, "Programming Concepts");
    }

    #[tokio::test]
    async fn test_search_finds_gems_by_description() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        let gem = Gem {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: "Article".to_string(),
            source_url: "https://example.com/oauth".to_string(),
            domain: "example.com".to_string(),
            title: "Security Guide".to_string(),
            author: None,
            description: Some("OAuth token authentication explained".to_string()),
            content: Some("Security is important".to_string()),
            source_meta: serde_json::json!({}),
            captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };
        
        store.save(gem.clone()).await.expect("Save gem");
        
        // Search for "OAuth" which is only in description
        let results = store.search("OAuth", 10).await.expect("Search should succeed");
        
        assert_eq!(results.len(), 1, "Should find gem by description");
        assert_eq!(results[0].title, "Security Guide");
    }

    #[tokio::test]
    async fn test_search_empty_query_delegates_to_list() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        // Create multiple gems
        for i in 0..3 {
            let gem = Gem {
                id: uuid::Uuid::new_v4().to_string(),
                source_type: "Article".to_string(),
                source_url: format!("https://example.com/{}", i),
                domain: "example.com".to_string(),
                title: format!("Gem {}", i),
                author: None,
                description: None,
                content: Some(format!("Content {}", i)),
                source_meta: serde_json::json!({}),
                captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
            };
            store.save(gem).await.expect("Save should succeed");
        }
        
        // Search with empty query
        let search_results = store.search("", 10).await.expect("Search should succeed");
        let list_results = store.list(10, 0).await.expect("List should succeed");
        
        assert_eq!(search_results.len(), list_results.len(), "Empty search should return same as list");
        assert_eq!(search_results.len(), 3, "Should return all gems");
    }

    #[tokio::test]
    async fn test_search_with_whitespace_query_delegates_to_list() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        let gem = Gem {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: "Article".to_string(),
            source_url: "https://example.com/test".to_string(),
            domain: "example.com".to_string(),
            title: "Test Gem".to_string(),
            author: None,
            description: None,
            content: Some("Test content".to_string()),
            source_meta: serde_json::json!({}),
            captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };
        store.save(gem).await.expect("Save should succeed");
        
        // Search with whitespace-only query
        let results = store.search("   ", 10).await.expect("Search should succeed");
        
        assert_eq!(results.len(), 1, "Whitespace query should return all gems like list");
    }

    #[tokio::test]
    async fn test_search_respects_limit() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        // Create 5 gems with "test" in title
        for i in 0..5 {
            let gem = Gem {
                id: uuid::Uuid::new_v4().to_string(),
                source_type: "Article".to_string(),
                source_url: format!("https://example.com/{}", i),
                domain: "example.com".to_string(),
                title: format!("Test Gem {}", i),
                author: None,
                description: None,
                content: Some(format!("Content {}", i)),
                source_meta: serde_json::json!({}),
                captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
            };
            store.save(gem).await.expect("Save should succeed");
        }
        
        // Search with limit of 2
        let results = store.search("Test", 2).await.expect("Search should succeed");
        
        assert_eq!(results.len(), 2, "Should respect limit parameter");
    }

    #[tokio::test]
    async fn test_search_returns_empty_for_no_matches() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        let gem = Gem {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: "Article".to_string(),
            source_url: "https://example.com/test".to_string(),
            domain: "example.com".to_string(),
            title: "Test Gem".to_string(),
            author: None,
            description: None,
            content: Some("Test content".to_string()),
            source_meta: serde_json::json!({}),
            captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };
        store.save(gem).await.expect("Save should succeed");
        
        // Search for non-existent term
        let results = store.search("nonexistent", 10).await.expect("Search should succeed");
        
        assert_eq!(results.len(), 0, "Should return empty for no matches");
    }

    #[tokio::test]
    async fn test_search_handles_fts5_syntax_error_unmatched_quotes() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        // Search with unmatched quote
        let result = store.search("\"unmatched", 10).await;
        
        assert!(result.is_err(), "Should return error for unmatched quotes");
        let error_msg = result.unwrap_err();
        println!("Actual error message: {}", error_msg);
        assert!(error_msg.contains("Invalid search query syntax"), "Error should be user-friendly: {}", error_msg);
    }

    #[tokio::test]
    async fn test_search_truncates_content_in_preview() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        // Create gem with long content
        let long_content = "a".repeat(300);
        let gem = Gem {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: "Article".to_string(),
            source_url: "https://example.com/long".to_string(),
            domain: "example.com".to_string(),
            title: "Long Content with keyword searchterm".to_string(),
            author: None,
            description: None,
            content: Some(long_content.clone()),
            source_meta: serde_json::json!({}),
            captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };
        
        store.save(gem).await.expect("Save should succeed");
        
        let results = store.search("searchterm", 10).await.expect("Search should succeed");
        assert_eq!(results.len(), 1);
        
        let preview = &results[0];
        let preview_content = preview.content_preview.as_ref().unwrap();
        
        // Should be truncated to 200 chars + "..."
        assert_eq!(preview_content.chars().count(), 203, "Should be 200 chars + '...'");
        assert!(preview_content.ends_with("..."), "Should end with ellipsis");
    }

    // Tests for delete functionality (task 3.19)
    
    #[tokio::test]
    async fn test_delete_existing_gem_succeeds() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        // Create and save a gem
        let gem = Gem {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: "Article".to_string(),
            source_url: "https://example.com/test".to_string(),
            domain: "example.com".to_string(),
            title: "Test Gem".to_string(),
            author: None,
            description: None,
            content: Some("Test content".to_string()),
            source_meta: serde_json::json!({}),
            captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };
        
        let saved = store.save(gem).await.expect("Save should succeed");
        
        // Verify gem exists
        let retrieved = store.get(&saved.id).await.expect("Get should succeed");
        assert!(retrieved.is_some(), "Gem should exist before delete");
        
        // Delete the gem
        let delete_result = store.delete(&saved.id).await;
        assert!(delete_result.is_ok(), "Delete should succeed");
        
        // Verify gem no longer exists
        let after_delete = store.get(&saved.id).await.expect("Get should succeed");
        assert!(after_delete.is_none(), "Gem should not exist after delete");
    }

    #[tokio::test]
    async fn test_delete_non_existent_gem_returns_error() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        let non_existent_id = uuid::Uuid::new_v4().to_string();
        
        // Try to delete non-existent gem
        let result = store.delete(&non_existent_id).await;
        
        assert!(result.is_err(), "Delete should return error for non-existent gem");
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("not found"), "Error should mention gem not found");
        assert!(error_msg.contains(&non_existent_id), "Error should include the gem ID");
    }

    #[tokio::test]
    async fn test_delete_removes_gem_from_fts_index() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        // Create and save a gem with unique searchable content
        let gem = Gem {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: "Article".to_string(),
            source_url: "https://example.com/unique".to_string(),
            domain: "example.com".to_string(),
            title: "Unique searchable title DELETEME".to_string(),
            author: None,
            description: None,
            content: Some("Unique content DELETEME".to_string()),
            source_meta: serde_json::json!({}),
            captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };
        
        let saved = store.save(gem).await.expect("Save should succeed");
        
        // Verify gem is searchable
        let search_before = store.search("DELETEME", 10).await.expect("Search should succeed");
        assert_eq!(search_before.len(), 1, "Gem should be searchable before delete");
        
        // Delete the gem
        store.delete(&saved.id).await.expect("Delete should succeed");
        
        // Verify gem is no longer searchable (FTS5 trigger removed it from index)
        let search_after = store.search("DELETEME", 10).await.expect("Search should succeed");
        assert_eq!(search_after.len(), 0, "Gem should not be searchable after delete");
    }

    // Phase 1 Tests: find_by_recording_filename

    #[tokio::test]
    async fn test_find_by_recording_filename_with_existing_gem() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        let filename = "recording_1234567890.pcm";
        
        // Create a gem with recording metadata
        let gem = Gem {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: "Other".to_string(),
            source_url: "jarvis://recording/test".to_string(),
            domain: "jarvis-app".to_string(),
            title: "Test Recording".to_string(),
            author: None,
            description: Some("Test description".to_string()),
            content: Some("Whisper transcript".to_string()),
            source_meta: serde_json::json!({
                "recording_filename": filename
            }),
            captured_at: chrono::Utc::now().to_rfc3339(),
            ai_enrichment: None,
            transcript: Some("MLX Omni transcript".to_string()),
            transcript_language: Some("en".to_string()),
        };
        
        store.save(gem.clone()).await.expect("Save should succeed");
        
        // Find by recording filename
        let result = store.find_by_recording_filename(filename).await
            .expect("Query should succeed");
        
        assert!(result.is_some(), "Should find gem with matching recording filename");
        let preview = result.unwrap();
        assert_eq!(preview.title, "Test Recording");
        assert_eq!(preview.domain, "jarvis-app");
        assert_eq!(preview.transcript_language, Some("en".to_string()));
    }

    #[tokio::test]
    async fn test_find_by_recording_filename_with_no_gem() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        // Search for non-existent recording
        let result = store.find_by_recording_filename("nonexistent.pcm").await
            .expect("Query should succeed");
        
        assert!(result.is_none(), "Should return None when no gem matches");
    }

    #[tokio::test]
    async fn test_find_by_recording_filename_returns_most_recent() {
        let store = SqliteGemStore::new_in_memory()
            .expect("Failed to create in-memory store");
        
        let filename = "recording_duplicate.pcm";
        let now = chrono::Utc::now();
        
        // Create first gem (older)
        let gem1 = Gem {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: "Other".to_string(),
            source_url: "jarvis://recording/1".to_string(),
            domain: "jarvis-app".to_string(),
            title: "Older Recording".to_string(),
            author: None,
            description: None,
            content: None,
            source_meta: serde_json::json!({
                "recording_filename": filename
            }),
            captured_at: (now - chrono::Duration::hours(2)).to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };
        
        // Create second gem (newer)
        let gem2 = Gem {
            id: uuid::Uuid::new_v4().to_string(),
            source_type: "Other".to_string(),
            source_url: "jarvis://recording/2".to_string(),
            domain: "jarvis-app".to_string(),
            title: "Newer Recording".to_string(),
            author: None,
            description: None,
            content: None,
            source_meta: serde_json::json!({
                "recording_filename": filename
            }),
            captured_at: now.to_rfc3339(),
            ai_enrichment: None,
            transcript: None,
            transcript_language: None,
        };
        
        store.save(gem1).await.expect("Save gem1 should succeed");
        store.save(gem2).await.expect("Save gem2 should succeed");
        
        // Find by recording filename - should return the newer one
        let result = store.find_by_recording_filename(filename).await
            .expect("Query should succeed");
        
        assert!(result.is_some(), "Should find gem");
        let preview = result.unwrap();
        assert_eq!(preview.title, "Newer Recording", "Should return the most recent gem");
    }

    // Property 3: Recording Filename Query Correctness
    proptest! {
        #[test]
        fn prop_find_by_recording_filename_correctness(
            filename in "[a-z0-9_]{5,20}\\.pcm",
            has_matching_gem in proptest::bool::ANY,
            num_other_gems in 0usize..5,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let store = SqliteGemStore::new_in_memory()
                    .expect("Failed to create in-memory store");
                
                let mut expected_gem_id: Option<String> = None;
                
                // Create a gem with matching recording_filename if has_matching_gem is true
                if has_matching_gem {
                    let gem = Gem {
                        id: uuid::Uuid::new_v4().to_string(),
                        source_type: "Other".to_string(),
                        source_url: format!("jarvis://recording/{}", uuid::Uuid::new_v4()),
                        domain: "jarvis-app".to_string(),
                        title: "Matching Recording".to_string(),
                        author: None,
                        description: None,
                        content: None,
                        source_meta: serde_json::json!({
                            "recording_filename": filename.clone()
                        }),
                        captured_at: chrono::Utc::now().to_rfc3339(),
                        ai_enrichment: None,
                        transcript: None,
                        transcript_language: None,
                    };
                    expected_gem_id = Some(gem.id.clone());
                    store.save(gem).await.expect("Save should succeed");
                }
                
                // Create other gems without matching recording_filename
                for i in 0..num_other_gems {
                    let other_gem = Gem {
                        id: uuid::Uuid::new_v4().to_string(),
                        source_type: "Other".to_string(),
                        source_url: format!("jarvis://recording/other_{}", i),
                        domain: "jarvis-app".to_string(),
                        title: format!("Other Recording {}", i),
                        author: None,
                        description: None,
                        content: None,
                        source_meta: serde_json::json!({
                            "recording_filename": format!("other_{}.pcm", i)
                        }),
                        captured_at: chrono::Utc::now().to_rfc3339(),
                        ai_enrichment: None,
                        transcript: None,
                        transcript_language: None,
                    };
                    store.save(other_gem).await.expect("Save should succeed");
                }
                
                // Query by recording filename
                let result = store.find_by_recording_filename(&filename).await
                    .expect("Query should succeed");
                
                // Property: find_by_recording_filename returns gem if and only if matching gem exists
                if has_matching_gem {
                    prop_assert!(result.is_some(), "Should find gem when matching gem exists");
                    let preview = result.unwrap();
                    prop_assert_eq!(preview.id, expected_gem_id.unwrap(), "Should return the correct gem");
                    prop_assert_eq!(preview.title, "Matching Recording");
                } else {
                    prop_assert!(result.is_none(), "Should return None when no matching gem exists");
                }
                
                Ok(())
            })?;
        }
    }
}

