use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use crate::gems::GemPreview;
use super::store::*;

/// SQLite-backed project store. Shares gems.db with SqliteGemStore.
pub struct SqliteProjectStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteProjectStore {
    /// Create a new SqliteProjectStore sharing the existing database connection.
    ///
    /// Runs migration to create projects and project_gems tables if they don't exist.
    pub fn new(conn: Arc<Mutex<Connection>>) -> Result<Self, String> {
        let store = Self { conn };
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
        
        // Create gems table and gems_fts for testing (needed for get_project_gems search)
        let conn_lock = store.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;
        
        conn_lock.execute(
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
                ai_enrichment TEXT,
                transcript TEXT,
                transcript_language TEXT
            )",
            [],
        ).map_err(|e| format!("Failed to create gems table: {}", e))?;
        
        conn_lock.execute(
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
        
        drop(conn_lock);
        
        store.initialize_schema()?;
        Ok(store)
    }

    fn initialize_schema(&self) -> Result<(), String> {
        let conn = self.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;

        // Enable foreign keys (required for CASCADE)
        conn.execute("PRAGMA foreign_keys = ON", [])
            .map_err(|e| format!("Failed to enable foreign keys: {}", e))?;

        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                objective TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS project_gems (
                project_id TEXT NOT NULL,
                gem_id TEXT NOT NULL,
                added_at TEXT NOT NULL,
                PRIMARY KEY (project_id, gem_id),
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
                FOREIGN KEY (gem_id) REFERENCES gems(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_project_gems_gem ON project_gems(gem_id);
            CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status);
            CREATE INDEX IF NOT EXISTS idx_projects_updated ON projects(updated_at DESC);
        ").map_err(|e| format!("Failed to create projects tables: {}", e))?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl ProjectStore for SqliteProjectStore {
    async fn create(&self, input: CreateProject) -> Result<Project, String> {
        let conn = self.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;

        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO projects (id, title, description, objective, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?5)",
            rusqlite::params![id, input.title, input.description, input.objective, now],
        ).map_err(|e| format!("Failed to create project: {}", e))?;

        Ok(Project {
            id,
            title: input.title,
            description: input.description,
            objective: input.objective,
            status: "active".to_string(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    async fn list(&self) -> Result<Vec<ProjectPreview>, String> {
        let conn = self.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT p.id, p.title, p.description, p.status, p.updated_at,
                    COUNT(pg.gem_id) as gem_count
             FROM projects p
             LEFT JOIN project_gems pg ON p.id = pg.project_id
             GROUP BY p.id
             ORDER BY p.updated_at DESC"
        ).map_err(|e| format!("Failed to prepare list query: {}", e))?;

        let projects = stmt.query_map([], |row| {
            Ok(ProjectPreview {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
                updated_at: row.get(4)?,
                gem_count: row.get::<_, i64>(5)? as usize,
            })
        })
        .map_err(|e| format!("Failed to query projects: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect projects: {}", e))?;

        Ok(projects)
    }

    async fn get(&self, id: &str) -> Result<ProjectDetail, String> {
        let conn = self.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;

        // Get the project
        let project = conn.query_row(
            "SELECT id, title, description, objective, status, created_at, updated_at
             FROM projects WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    objective: row.get(3)?,
                    status: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => "Project not found".to_string(),
            _ => format!("Failed to get project: {}", e),
        })?;

        // Get associated gems (reusing GemPreview column mapping from SqliteGemStore)
        let mut stmt = conn.prepare(
            "SELECT g.id, g.source_type, g.source_url, g.domain, g.title, g.author,
                    g.description, SUBSTR(g.content, 1, 200) as content_preview,
                    g.captured_at, g.ai_enrichment, g.transcript_language
             FROM gems g
             INNER JOIN project_gems pg ON g.id = pg.gem_id
             WHERE pg.project_id = ?1
             ORDER BY pg.added_at DESC"
        ).map_err(|e| format!("Failed to prepare gems query: {}", e))?;

        let gems = stmt.query_map(rusqlite::params![id], |row| {
            let ai_enrichment: Option<String> = row.get(9)?;
            let (tags, summary, enrichment_source) = parse_ai_enrichment(ai_enrichment.as_deref());

            Ok(GemPreview {
                id: row.get(0)?,
                source_type: row.get(1)?,
                source_url: row.get(2)?,
                domain: row.get(3)?,
                title: row.get(4)?,
                author: row.get(5)?,
                description: row.get(6)?,
                content_preview: row.get(7)?,
                captured_at: row.get(8)?,
                tags,
                summary,
                enrichment_source,
                transcript_language: row.get(10)?,
            })
        })
        .map_err(|e| format!("Failed to query gems: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect gems: {}", e))?;

        let gem_count = gems.len();

        Ok(ProjectDetail {
            project,
            gem_count,
            gems,
        })
    }

    async fn update(&self, id: &str, updates: UpdateProject) -> Result<Project, String> {
        let conn = self.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;

        let now = chrono::Utc::now().to_rfc3339();

        // Build dynamic UPDATE query based on which fields are Some
        let mut set_clauses = vec!["updated_at = ?1".to_string()];
        let mut param_index = 2;
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(now.clone())];

        if let Some(ref title) = updates.title {
            set_clauses.push(format!("title = ?{}", param_index));
            params.push(Box::new(title.clone()));
            param_index += 1;
        }
        if let Some(ref description) = updates.description {
            set_clauses.push(format!("description = ?{}", param_index));
            params.push(Box::new(description.clone()));
            param_index += 1;
        }
        if let Some(ref objective) = updates.objective {
            set_clauses.push(format!("objective = ?{}", param_index));
            params.push(Box::new(objective.clone()));
            param_index += 1;
        }
        if let Some(ref status) = updates.status {
            set_clauses.push(format!("status = ?{}", param_index));
            params.push(Box::new(status.clone()));
            param_index += 1;
        }

        let sql = format!(
            "UPDATE projects SET {} WHERE id = ?{}",
            set_clauses.join(", "),
            param_index
        );
        params.push(Box::new(id.to_string()));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows_affected = conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| format!("Failed to update project: {}", e))?;

        if rows_affected == 0 {
            return Err("Project not found".to_string());
        }

        // Return updated project
        conn.query_row(
            "SELECT id, title, description, objective, status, created_at, updated_at
             FROM projects WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    objective: row.get(3)?,
                    status: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        ).map_err(|e| format!("Failed to get updated project: {}", e))
    }

    async fn delete(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;

        // CASCADE handles project_gems cleanup
        conn.execute("DELETE FROM projects WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| format!("Failed to delete project: {}", e))?;

        Ok(())
    }

    async fn add_gems(&self, project_id: &str, gem_ids: &[String]) -> Result<usize, String> {
        let conn = self.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;

        let now = chrono::Utc::now().to_rfc3339();
        let mut count = 0;

        for gem_id in gem_ids {
            let result = conn.execute(
                "INSERT OR IGNORE INTO project_gems (project_id, gem_id, added_at)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![project_id, gem_id, now],
            ).map_err(|e| format!("Failed to add gem to project: {}", e))?;

            count += result; // 1 if inserted, 0 if ignored (already exists)
        }

        // Update project timestamp
        conn.execute(
            "UPDATE projects SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, project_id],
        ).map_err(|e| format!("Failed to update project timestamp: {}", e))?;

        Ok(count)
    }

    async fn remove_gem(&self, project_id: &str, gem_id: &str) -> Result<(), String> {
        let conn = self.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;

        conn.execute(
            "DELETE FROM project_gems WHERE project_id = ?1 AND gem_id = ?2",
            rusqlite::params![project_id, gem_id],
        ).map_err(|e| format!("Failed to remove gem from project: {}", e))?;

        // Update project timestamp
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE projects SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, project_id],
        ).map_err(|e| format!("Failed to update project timestamp: {}", e))?;

        Ok(())
    }

    async fn get_project_gems(
        &self,
        project_id: &str,
        query: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<GemPreview>, String> {
        let conn = self.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;

        let (sql, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(q) = query {
            if q.trim().is_empty() {
                // Empty query — return all project gems
                let sql = "SELECT g.id, g.source_type, g.source_url, g.domain, g.title, g.author,
                                  g.description, SUBSTR(g.content, 1, 200), g.captured_at,
                                  g.ai_enrichment, g.transcript_language
                           FROM gems g
                           INNER JOIN project_gems pg ON g.id = pg.gem_id
                           WHERE pg.project_id = ?1
                           ORDER BY pg.added_at DESC
                           LIMIT ?2".to_string();
                (sql, vec![
                    Box::new(project_id.to_string()),
                    Box::new(limit.unwrap_or(100) as i64),
                ])
            } else {
                // Search within project gems using FTS5
                let sql = "SELECT g.id, g.source_type, g.source_url, g.domain, g.title, g.author,
                                  g.description, SUBSTR(g.content, 1, 200), g.captured_at,
                                  g.ai_enrichment, g.transcript_language
                           FROM gems g
                           INNER JOIN project_gems pg ON g.id = pg.gem_id
                           INNER JOIN gems_fts ON gems_fts.rowid = g.rowid
                           WHERE pg.project_id = ?1 AND gems_fts MATCH ?2
                           ORDER BY rank
                           LIMIT ?3".to_string();
                (sql, vec![
                    Box::new(project_id.to_string()),
                    Box::new(q.to_string()),
                    Box::new(limit.unwrap_or(100) as i64),
                ])
            }
        } else {
            let sql = "SELECT g.id, g.source_type, g.source_url, g.domain, g.title, g.author,
                              g.description, SUBSTR(g.content, 1, 200), g.captured_at,
                              g.ai_enrichment, g.transcript_language
                       FROM gems g
                       INNER JOIN project_gems pg ON g.id = pg.gem_id
                       WHERE pg.project_id = ?1
                       ORDER BY pg.added_at DESC
                       LIMIT ?2".to_string();
            (sql, vec![
                Box::new(project_id.to_string()),
                Box::new(limit.unwrap_or(100) as i64),
            ])
        };

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let gems = stmt.query_map(param_refs.as_slice(), |row| {
            let ai_enrichment: Option<String> = row.get(9)?;
            let (tags, summary, enrichment_source) = parse_ai_enrichment(ai_enrichment.as_deref());

            Ok(GemPreview {
                id: row.get(0)?,
                source_type: row.get(1)?,
                source_url: row.get(2)?,
                domain: row.get(3)?,
                title: row.get(4)?,
                author: row.get(5)?,
                description: row.get(6)?,
                content_preview: row.get(7)?,
                captured_at: row.get(8)?,
                tags,
                summary,
                enrichment_source,
                transcript_language: row.get(10)?,
            })
        })
        .map_err(|e| format!("Failed to query gems: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect gems: {}", e))?;

        Ok(gems)
    }

    async fn get_gem_projects(&self, gem_id: &str) -> Result<Vec<ProjectPreview>, String> {
        let conn = self.conn.lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT p.id, p.title, p.description, p.status, p.updated_at,
                    (SELECT COUNT(*) FROM project_gems WHERE project_id = p.id) as gem_count
             FROM projects p
             INNER JOIN project_gems pg ON p.id = pg.project_id
             WHERE pg.gem_id = ?1
             ORDER BY p.updated_at DESC"
        ).map_err(|e| format!("Failed to prepare query: {}", e))?;

        let projects = stmt.query_map(rusqlite::params![gem_id], |row| {
            Ok(ProjectPreview {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
                updated_at: row.get(4)?,
                gem_count: row.get::<_, i64>(5)? as usize,
            })
        })
        .map_err(|e| format!("Failed to query projects: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect projects: {}", e))?;

        Ok(projects)
    }
}

/// Parse ai_enrichment JSON to extract tags, summary, and enrichment source.
/// Duplicated from SqliteGemStore — consider extracting to a shared util.
fn parse_ai_enrichment(json_str: Option<&str>) -> (Option<Vec<String>>, Option<String>, Option<String>) {
    let Some(s) = json_str else { return (None, None, None) };
    let Ok(val) = serde_json::from_str::<serde_json::Value>(s) else { return (None, None, None) };

    let tags = val.get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

    let summary = val.get("summary")
        .and_then(|v| v.as_str())
        .map(String::from);

    let provider = val.get("provider").and_then(|v| v.as_str());
    let model = val.get("model").and_then(|v| v.as_str());
    let enrichment_source = match (provider, model) {
        (Some(p), Some(m)) => Some(format!("{} / {}", p, m)),
        (Some(p), None) => Some(p.to_string()),
        _ => None,
    };

    (tags, summary, enrichment_source)
}
