# Projects — Design Document

## Overview

This design adds a **Projects** feature — named containers that group gems under a shared goal. The backend follows the same trait-based architecture used by `GemStore`, `KnowledgeStore`, and `SearchResultProvider`: a `ProjectStore` trait with a `SqliteProjectStore` implementation sharing the existing `gems.db` database. The frontend introduces a split center panel (`ProjectList` + `ProjectGemList` side by side) that creates a four-column drill-down: `Left Nav → Project List → Gem List → Gem Detail`.

The architecture follows the existing patterns exactly:
- `gems/` has `store.rs` (trait) + `sqlite_store.rs` (impl) + `mod.rs`
- `knowledge/` has `store.rs` (trait) + `local_store.rs` (impl) + `mod.rs`
- `search/` has `provider.rs` (trait) + `fts_provider.rs` / `qmd_provider.rs` (impls) + `commands.rs`

Projects follows the same structure: `projects/store.rs` (trait) + `projects/sqlite_store.rs` (impl) + `projects/commands.rs`.

### Design Goals

1. **Familiar patterns**: Same trait-based architecture, same database, same UI components where possible
2. **Fast creation**: Create a project with just a title — description and objective are optional
3. **Non-destructive**: Deleting a project never deletes gems. Removing a gem from a project only removes the association
4. **Multi-assignment**: A gem can belong to multiple projects (many-to-many)
5. **Foundation for synthesis**: Project metadata (title, description, objective) will feed into future LLM-powered synthesis and research recommendation features

### Key Design Decisions

- **Split center panel**: Project list (260px fixed) and gem list (flex) render side by side. No "Back" navigation — both lists are always visible
- **Same database**: Projects tables are added to `gems.db` via `CREATE TABLE IF NOT EXISTS` migration
- **Reuse GemCard**: The project gem list reuses the existing `GemCard` component with an `onRemove` callback
- **Junction table with CASCADE**: `project_gems` table with composite primary key. Deleting a project cascades to associations. Deleting a gem cascades to associations
- **Three-field create form**: Title (required), description (optional), objective (optional). No deadline or status at creation time
- **Status is edit-only**: Projects start as "active". Status changes happen via the edit action

### Operational Flow

1. **User clicks Projects in nav** → center panel renders `ProjectsContainer` with split layout
2. **User creates a project** → inline form in `ProjectList` → `invoke('create_project')` → project appears in list, auto-selected
3. **User selects a project** → `ProjectGemList` loads metadata + gems via `invoke('get_project', { id })`
4. **User adds gems** → `AddGemsModal` opens → search + checkbox select → `invoke('add_gems_to_project')` → gems appear in list
5. **User clicks a gem** → `onGemSelect(gemId)` → `GemDetailPanel` opens in right panel (existing behavior)
6. **User removes a gem** → `invoke('remove_gem_from_project')` → gem disappears from project list (gem itself unchanged)

---

## Architecture

### Module Hierarchy

```
src/projects/
├── mod.rs              — Module root, re-exports public types
├── store.rs            — ProjectStore trait, Project, ProjectPreview, ProjectDetail,
│                         CreateProject, UpdateProject
├── sqlite_store.rs     — SqliteProjectStore (wraps rusqlite, shares gems.db connection)
└── commands.rs         — Tauri commands: create_project, list_projects, get_project,
                          update_project, delete_project, add_gems_to_project,
                          remove_gem_from_project, get_project_gems, get_gem_projects
```

### Dependency Graph

```
                     ┌─────────────┐
                     │   lib.rs    │
                     │   (setup)   │
                     └──────┬──────┘
                            │ creates SqliteProjectStore, registers as Arc<dyn ProjectStore>
                            ▼
              ┌───────────────────────────────┐
              │  Arc<dyn ProjectStore>        │
              │    (Tauri managed state)      │
              └───────────┬───────────────────┘
                          │
            ┌─────────────┼─────────────────┐
            ▼                               ▼
  ┌──────────────────┐            ┌──────────────────┐
  │ projects/        │            │ gems/            │
  │ commands.rs      │            │ store.rs         │
  │ (9 Tauri cmds)   │            │ (GemPreview      │
  └──────────────────┘            │  reused in       │
                                  │  ProjectDetail)  │
                                  └──────────────────┘

                ┌──────────────────────┐
                │  SqliteProjectStore  │
                │  (same gems.db)      │
                │                      │
                │  projects table      │
                │  project_gems table  │
                └──────────────────────┘
```

### Data Flow — Create Project

```
CreateProjectForm (ProjectList)
  │
  │ invoke('create_project', { title: "ECS Migration", description: "...", objective: "..." })
  │
  ▼
projects/commands.rs :: create_project()
  │
  ├── project_store.create(CreateProject { title, description, objective }).await
  │     │
  │     └── SqliteProjectStore::create()
  │           ├── Generate UUID v4
  │           ├── Set status = "active", timestamps = now
  │           └── INSERT INTO projects (...)
  │
  └── Return Project { id, title, description, objective, status, created_at, updated_at }
        │
        ▼
  Frontend: add to project list, auto-select
```

### Data Flow — View Project (Select)

```
ProjectList :: onClick(projectId)
  │
  │ invoke('get_project', { id: projectId })
  │
  ▼
projects/commands.rs :: get_project()
  │
  ├── project_store.get(id).await
  │     │
  │     └── SqliteProjectStore::get()
  │           ├── SELECT * FROM projects WHERE id = ?
  │           ├── SELECT gems.* FROM gems
  │           │   INNER JOIN project_gems ON gems.id = project_gems.gem_id
  │           │   WHERE project_gems.project_id = ?
  │           │   ORDER BY project_gems.added_at DESC
  │           └── Return ProjectDetail { project, gem_count, gems }
  │
  └── Return ProjectDetail to frontend
        │
        ▼
  ProjectGemList: render metadata header + gem cards
```

### Data Flow — Add Gems to Project

```
AddGemsModal :: onSubmit(selectedGemIds)
  │
  │ invoke('add_gems_to_project', { projectId: "abc", gemIds: ["gem1", "gem2", "gem3"] })
  │
  ▼
projects/commands.rs :: add_gems_to_project()
  │
  ├── project_store.add_gems(project_id, &gem_ids).await
  │     │
  │     └── SqliteProjectStore::add_gems()
  │           ├── For each gem_id:
  │           │     INSERT OR IGNORE INTO project_gems (project_id, gem_id, added_at)
  │           │     VALUES (?, ?, now)
  │           ├── UPDATE projects SET updated_at = now WHERE id = ?
  │           └── Return count of newly added associations
  │
  └── Return usize (count added)
        │
        ▼
  Frontend: close modal, refresh project gem list
```

### Data Flow — Add to Project from GemCard

```
GemCard :: "Add to Project" button
  │
  │ invoke('get_gem_projects', { gemId })  →  Vec<ProjectPreview> (pre-check)
  │ invoke('list_projects')                →  Vec<ProjectPreview> (all projects)
  │
  ▼
AddToProjectDropdown :: user checks/unchecks projects
  │
  ├── Check a project:   invoke('add_gems_to_project', { projectId, gemIds: [gemId] })
  └── Uncheck a project: invoke('remove_gem_from_project', { projectId, gemId })
```

---

## Modules and Interfaces

### `store.rs` — Trait and Data Types

**File**: `src/projects/store.rs`

**Responsibilities**: Define the `ProjectStore` trait and all shared data types.

```rust
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
```

### `sqlite_store.rs` — SQLite Implementation

**File**: `src/projects/sqlite_store.rs`

**Responsibilities**: Implement `ProjectStore` using rusqlite. Share the same `gems.db` database as `SqliteGemStore`.

```rust
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
                let sql = format!(
                    "SELECT g.id, g.source_type, g.source_url, g.domain, g.title, g.author,
                            g.description, SUBSTR(g.content, 1, 200), g.captured_at,
                            g.ai_enrichment, g.transcript_language
                     FROM gems g
                     INNER JOIN project_gems pg ON g.id = pg.gem_id
                     WHERE pg.project_id = ?1
                     ORDER BY pg.added_at DESC
                     LIMIT ?2"
                );
                (sql, vec![
                    Box::new(project_id.to_string()),
                    Box::new(limit.unwrap_or(100) as i64),
                ])
            } else {
                // Search within project gems using FTS5
                let sql = format!(
                    "SELECT g.id, g.source_type, g.source_url, g.domain, g.title, g.author,
                            g.description, SUBSTR(g.content, 1, 200), g.captured_at,
                            g.ai_enrichment, g.transcript_language
                     FROM gems g
                     INNER JOIN project_gems pg ON g.id = pg.gem_id
                     INNER JOIN gems_fts ON gems_fts.rowid = g.rowid
                     WHERE pg.project_id = ?1 AND gems_fts MATCH ?2
                     ORDER BY rank
                     LIMIT ?3"
                );
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
```

### `commands.rs` — Tauri Command Handlers

**File**: `src/projects/commands.rs`

**Responsibilities**: Expose project operations as Tauri commands.

```rust
use std::sync::Arc;
use tauri::State;
use crate::gems::{GemPreview, GemStore};
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
```

### `mod.rs` — Module Root

**File**: `src/projects/mod.rs`

```rust
pub mod store;
pub mod sqlite_store;
pub mod commands;

pub use store::{
    ProjectStore,
    Project,
    ProjectPreview,
    ProjectDetail,
    CreateProject,
    UpdateProject,
};
pub use sqlite_store::SqliteProjectStore;
```

---

## Provider Registration

### In `lib.rs` — App Setup

Add after `gem_store_arc` creation:

```rust
// Initialize ProjectStore (shares gems.db connection with SqliteGemStore)
let project_store = projects::SqliteProjectStore::new(gem_store_arc.get_conn().clone())
    .map_err(|e| format!("Failed to initialize project store: {}", e))?;
let project_store_arc = Arc::new(project_store) as Arc<dyn projects::ProjectStore>;
app.manage(project_store_arc);
```

**Note**: `SqliteProjectStore` needs access to the same `Arc<Mutex<Connection>>` used by `SqliteGemStore`. This requires either:
- (A) Exposing a `get_conn()` method on `SqliteGemStore` — simple but couples the two stores
- (B) Creating the `Arc<Mutex<Connection>>` in `lib.rs` and passing it to both stores — cleaner

**Recommended approach**: Option (B) — create the connection in `lib.rs`:

```rust
// Create shared database connection
let home = dirs::home_dir().expect("Could not find home directory");
let db_path = home.join(".jarvis").join("gems.db");
std::fs::create_dir_all(db_path.parent().unwrap()).expect("Failed to create .jarvis directory");
let conn = rusqlite::Connection::open(&db_path).expect("Failed to open database");
let conn_arc = Arc::new(Mutex::new(conn));

// Initialize GemStore with shared connection
let gem_store = SqliteGemStore::from_conn(conn_arc.clone())?;
let gem_store_arc = Arc::new(gem_store) as Arc<dyn GemStore>;
app.manage(gem_store_arc.clone());

// Initialize ProjectStore with same shared connection
let project_store = projects::SqliteProjectStore::new(conn_arc.clone())?;
let project_store_arc = Arc::new(project_store) as Arc<dyn projects::ProjectStore>;
app.manage(project_store_arc);
```

This requires adding a `SqliteGemStore::from_conn(conn: Arc<Mutex<Connection>>)` constructor. The existing `new()` constructor remains for backward compatibility.

### Register Commands

Add to `generate_handler![]` in `lib.rs`:

```rust
projects::commands::create_project,
projects::commands::list_projects,
projects::commands::get_project,
projects::commands::update_project,
projects::commands::delete_project,
projects::commands::add_gems_to_project,
projects::commands::remove_gem_from_project,
projects::commands::get_project_gems,
projects::commands::get_gem_projects,
```

---

## Frontend Changes

### TypeScript Types

Add to `src/state/types.ts`:

```typescript
/**
 * Project types
 */

/** Full project representation matching Rust Project struct */
export interface Project {
  id: string;
  title: string;
  description: string | null;
  objective: string | null;
  status: 'active' | 'paused' | 'completed' | 'archived';
  created_at: string;
  updated_at: string;
}

/** Lightweight project for list views matching Rust ProjectPreview struct */
export interface ProjectPreview {
  id: string;
  title: string;
  description: string | null;
  status: string;
  gem_count: number;
  updated_at: string;
}

/** Full project with associated gems matching Rust ProjectDetail struct */
export interface ProjectDetail {
  project: Project;
  gem_count: number;
  gems: GemPreview[];
}
```

### LeftNav.tsx Changes

```typescript
type ActiveNav = 'record' | 'recordings' | 'gems' | 'projects' | 'youtube' | 'browser' | 'settings';

const navItems: Array<{ id: ActiveNav; label: string; icon: string }> = [
  { id: 'record', label: 'Record', icon: '🎙️' },
  { id: 'recordings', label: 'Recordings', icon: '📼' },
  { id: 'gems', label: 'Gems', icon: '💎' },
  { id: 'projects', label: 'Projects', icon: '📁' },  // NEW
  { id: 'youtube', label: 'YouTube', icon: '📺' },
  { id: 'browser', label: 'Browser', icon: '🌐' }
];
```

### App.tsx Changes

In the center panel rendering:

```tsx
{activeNav === 'projects' && (
  <ProjectsContainer onGemSelect={handleGemSelect} />
)}
```

Update `showRightPanel` to include projects:

```typescript
const showRightPanel = activeNav === 'record' || activeNav === 'recordings'
  || activeNav === 'gems' || activeNav === 'projects';
```

### ProjectsContainer.tsx — Split Center Panel

```tsx
import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { ProjectPreview, ProjectDetail, GemPreview } from '../state/types';

interface ProjectsContainerProps {
  onGemSelect?: (gemId: string | null) => void;
}

export function ProjectsContainer({ onGemSelect }: ProjectsContainerProps) {
  const [projects, setProjects] = useState<ProjectPreview[]>([]);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const fetchProjects = useCallback(async () => {
    try {
      const result = await invoke<ProjectPreview[]>('list_projects');
      setProjects(result);
    } catch (err) {
      console.error('Failed to load projects:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { fetchProjects(); }, [fetchProjects]);

  const handleProjectCreated = (projectId: string) => {
    fetchProjects();
    setSelectedProjectId(projectId);
  };

  return (
    <div className="projects-container">
      <ProjectList
        projects={projects}
        selectedProjectId={selectedProjectId}
        onSelectProject={setSelectedProjectId}
        onProjectCreated={handleProjectCreated}
        onProjectsChanged={fetchProjects}
      />
      <ProjectGemList
        projectId={selectedProjectId}
        onGemSelect={onGemSelect}
        onProjectsChanged={fetchProjects}
      />
    </div>
  );
}
```

### ProjectList.tsx — Left side of split

```tsx
function ProjectList({
  projects, selectedProjectId, onSelectProject, onProjectCreated, onProjectsChanged
}: {
  projects: ProjectPreview[];
  selectedProjectId: string | null;
  onSelectProject: (id: string) => void;
  onProjectCreated: (id: string) => void;
  onProjectsChanged: () => void;
}) {
  const [showCreateForm, setShowCreateForm] = useState(false);

  return (
    <div className="project-list">
      <div className="project-list-header">
        <h3>Projects</h3>
        <button
          className="action-button"
          onClick={() => setShowCreateForm(true)}
        >
          + New Project
        </button>
      </div>

      {showCreateForm && (
        <CreateProjectForm
          onCreated={(id) => { setShowCreateForm(false); onProjectCreated(id); }}
          onCancel={() => setShowCreateForm(false)}
        />
      )}

      <div className="project-list-items">
        {projects.map(project => (
          <div
            key={project.id}
            className={`project-card ${selectedProjectId === project.id ? 'active' : ''}`}
            onClick={() => onSelectProject(project.id)}
          >
            <div className="project-card-title">{project.title}</div>
            <div className="project-card-meta">
              <span className={`status-badge status-${project.status}`}>
                {project.status}
              </span>
              <span className="gem-count">{project.gem_count} gems</span>
            </div>
            {project.description && (
              <div className="project-card-desc">{project.description}</div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
```

### ProjectGemList.tsx — Right side of split

```tsx
function ProjectGemList({
  projectId, onGemSelect, onProjectsChanged
}: {
  projectId: string | null;
  onGemSelect?: (gemId: string | null) => void;
  onProjectsChanged: () => void;
}) {
  const [detail, setDetail] = useState<ProjectDetail | null>(null);
  const [showAddModal, setShowAddModal] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  useEffect(() => {
    if (!projectId) { setDetail(null); return; }
    loadProject(projectId);
  }, [projectId]);

  const loadProject = async (id: string) => {
    try {
      const result = await invoke<ProjectDetail>('get_project', { id });
      setDetail(result);
    } catch (err) {
      console.error('Failed to load project:', err);
    }
  };

  const handleRemoveGem = async (gemId: string) => {
    if (!projectId) return;
    await invoke('remove_gem_from_project', { projectId, gemId });
    loadProject(projectId);
    onProjectsChanged(); // refresh gem counts in project list
  };

  if (!projectId) {
    return (
      <div className="project-gem-list empty-state">
        Select a project to see its gems
      </div>
    );
  }

  if (!detail) return <div className="project-gem-list loading">Loading...</div>;

  return (
    <div className="project-gem-list">
      {/* Project metadata header */}
      <div className="project-metadata-header">
        <h2>{detail.project.title}</h2>
        <div className="project-meta-row">
          <span className={`status-badge status-${detail.project.status}`}>
            {detail.project.status}
          </span>
          <span>{detail.gem_count} gems</span>
          <button className="action-button small">Edit</button>
          <button className="action-button small danger">Delete</button>
        </div>
        {detail.project.objective && (
          <div className="project-objective">{detail.project.objective}</div>
        )}
      </div>

      {/* Toolbar */}
      <div className="project-gem-toolbar">
        <input
          type="search"
          placeholder="Search project gems..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="gems-search-input"
        />
        <button
          className="action-button"
          onClick={() => setShowAddModal(true)}
        >
          + Add Gems
        </button>
      </div>

      {/* Gem cards */}
      <div className="project-gems-list">
        {detail.gems.map(gem => (
          <div key={gem.id} className="project-gem-card">
            {/* Reuse GemCard rendering or simplified version */}
            <div className="gem-card" onClick={() => onGemSelect?.(gem.id)}>
              <div className="gem-card-header">
                <span className={`source-badge ${gem.source_type.toLowerCase()}`}>
                  {gem.source_type}
                </span>
                <span className="gem-date">
                  {new Date(gem.captured_at).toLocaleDateString()}
                </span>
              </div>
              <div className="gem-title">{gem.title}</div>
              {gem.description && (
                <div className="gem-description">{gem.description}</div>
              )}
            </div>
            <button
              className="remove-from-project"
              onClick={(e) => { e.stopPropagation(); handleRemoveGem(gem.id); }}
              title="Remove from project"
            >
              ×
            </button>
          </div>
        ))}
      </div>

      {/* Add Gems Modal */}
      {showAddModal && (
        <AddGemsModal
          projectId={projectId}
          projectTitle={detail.project.title}
          existingGemIds={detail.gems.map(g => g.id)}
          onClose={() => setShowAddModal(false)}
          onAdded={() => { setShowAddModal(false); loadProject(projectId); onProjectsChanged(); }}
        />
      )}
    </div>
  );
}
```

### AddGemsModal.tsx — Gem Picker

```tsx
function AddGemsModal({
  projectId, projectTitle, existingGemIds, onClose, onAdded
}: {
  projectId: string;
  projectTitle: string;
  existingGemIds: string[];
  onClose: () => void;
  onAdded: () => void;
}) {
  const [gems, setGems] = useState<GemSearchResult[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

  useEffect(() => {
    loadGems('');
  }, []);

  // Debounced search
  useEffect(() => {
    const timer = setTimeout(() => loadGems(searchQuery), 300);
    return () => clearTimeout(timer);
  }, [searchQuery]);

  const loadGems = async (query: string) => {
    const results = await invoke<GemSearchResult[]>('search_gems', {
      query: query.trim(),
      limit: 100,
    });
    setGems(results);
  };

  const toggleGem = (gemId: string) => {
    setSelectedIds(prev => {
      const next = new Set(prev);
      if (next.has(gemId)) next.delete(gemId);
      else next.add(gemId);
      return next;
    });
  };

  const handleAdd = async () => {
    const gemIds = Array.from(selectedIds);
    await invoke('add_gems_to_project', { projectId, gemIds });
    onAdded();
  };

  const newSelections = selectedIds.size;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-card" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h3>Add Gems to {projectTitle}</h3>
          <button className="close-button" onClick={onClose}>×</button>
        </div>
        <div className="modal-search">
          <input
            type="search"
            placeholder="Search gems..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            autoFocus
          />
        </div>
        <div className="modal-gem-list">
          {gems.map(gem => {
            const alreadyAdded = existingGemIds.includes(gem.id);
            const isSelected = selectedIds.has(gem.id);
            return (
              <div
                key={gem.id}
                className={`modal-gem-row ${isSelected ? 'selected' : ''} ${alreadyAdded ? 'disabled' : ''}`}
                onClick={() => !alreadyAdded && toggleGem(gem.id)}
              >
                <input
                  type="checkbox"
                  checked={alreadyAdded || isSelected}
                  disabled={alreadyAdded}
                  readOnly
                />
                <div className="modal-gem-info">
                  <span className="modal-gem-title">{gem.title}</span>
                  <span className="modal-gem-meta">
                    {gem.source_type} · {gem.domain}
                  </span>
                </div>
                {alreadyAdded && (
                  <span className="already-added-label">Already added</span>
                )}
              </div>
            );
          })}
        </div>
        <div className="modal-footer">
          <button className="action-button secondary" onClick={onClose}>
            Cancel
          </button>
          <button
            className="action-button"
            onClick={handleAdd}
            disabled={newSelections === 0}
          >
            Add Selected ({newSelections})
          </button>
        </div>
      </div>
    </div>
  );
}
```

---

## CSS Additions

Add to `App.css`:

```css
/* Projects Container — Split Layout */
.projects-container {
  display: flex;
  flex-direction: row;
  height: 100%;
}

/* Project List — Left Side */
.project-list {
  width: 260px;
  flex-shrink: 0;
  border-right: 1px solid var(--border-color, #333);
  display: flex;
  flex-direction: column;
  overflow-y: auto;
}

.project-list-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  border-bottom: 1px solid var(--border-color, #333);
}

.project-list-header h3 {
  margin: 0;
  font-size: 16px;
}

.project-list-items {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

/* Project Card */
.project-card {
  padding: 12px;
  border-radius: 6px;
  cursor: pointer;
  margin-bottom: 4px;
}

.project-card:hover {
  background: var(--hover-bg, rgba(255, 255, 255, 0.05));
}

.project-card.active {
  background: var(--active-bg, rgba(59, 130, 246, 0.15));
  border-left: 3px solid var(--accent-color, #3b82f6);
}

.project-card-title {
  font-weight: 600;
  font-size: 14px;
  margin-bottom: 4px;
}

.project-card-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--text-muted, #888);
}

.project-card-desc {
  font-size: 12px;
  color: var(--text-secondary, #aaa);
  margin-top: 4px;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

/* Status Badges */
.status-badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 10px;
  font-size: 11px;
  font-weight: 600;
  text-transform: capitalize;
}

.status-active { background: rgba(34, 197, 94, 0.15); color: #4ade80; }
.status-paused { background: rgba(234, 179, 8, 0.15); color: #facc15; }
.status-completed { background: rgba(59, 130, 246, 0.15); color: #60a5fa; }
.status-archived { background: rgba(107, 114, 128, 0.15); color: #9ca3af; }

/* Project Gem List — Right Side */
.project-gem-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
}

.project-gem-list.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted, #666);
  font-size: 14px;
}

/* Project Metadata Header */
.project-metadata-header {
  padding: 16px;
  border-bottom: 1px solid var(--border-color, #333);
}

.project-metadata-header h2 {
  margin: 0 0 8px 0;
  font-size: 18px;
}

.project-meta-row {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 13px;
  color: var(--text-secondary, #aaa);
}

.project-objective {
  margin-top: 8px;
  font-size: 13px;
  color: var(--text-secondary, #aaa);
  font-style: italic;
}

/* Project Gem Toolbar */
.project-gem-toolbar {
  display: flex;
  gap: 8px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color, #333);
}

/* Project Gem Card (wrapper around GemCard with remove button) */
.project-gem-card {
  position: relative;
}

.remove-from-project {
  position: absolute;
  top: 8px;
  right: 8px;
  background: none;
  border: none;
  color: var(--text-muted, #666);
  font-size: 18px;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.15s;
}

.project-gem-card:hover .remove-from-project {
  opacity: 1;
}

.remove-from-project:hover {
  color: var(--error-color, #ef4444);
}

/* Create Project Form */
.create-project-form {
  padding: 12px;
  border-bottom: 1px solid var(--border-color, #333);
  background: var(--surface-bg, rgba(255, 255, 255, 0.02));
}

.create-project-form input,
.create-project-form textarea {
  width: 100%;
  margin-bottom: 8px;
  padding: 8px;
  border: 1px solid var(--border-color, #444);
  border-radius: 4px;
  background: var(--input-bg, #1a1a2e);
  color: var(--text-primary, #e0e0e0);
  font-size: 13px;
  font-family: inherit;
}

.create-project-form textarea {
  resize: vertical;
  min-height: 60px;
}

.create-project-form .form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

/* Modal Overlay */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-card {
  background: var(--panel-bg, #16213e);
  border: 1px solid var(--border-color, #333);
  border-radius: 8px;
  width: 560px;
  max-height: 70vh;
  display: flex;
  flex-direction: column;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  border-bottom: 1px solid var(--border-color, #333);
}

.modal-header h3 { margin: 0; }

.modal-search {
  padding: 12px 16px;
}

.modal-gem-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 16px;
}

.modal-gem-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 8px;
  border-radius: 4px;
  cursor: pointer;
}

.modal-gem-row:hover { background: var(--hover-bg, rgba(255, 255, 255, 0.05)); }
.modal-gem-row.selected { background: var(--active-bg, rgba(59, 130, 246, 0.1)); }
.modal-gem-row.disabled { opacity: 0.5; cursor: default; }

.modal-gem-info {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.modal-gem-title { font-size: 13px; }
.modal-gem-meta { font-size: 11px; color: var(--text-muted, #888); }

.already-added-label {
  font-size: 11px;
  color: var(--text-muted, #666);
  font-style: italic;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 16px;
  border-top: 1px solid var(--border-color, #333);
}
```

---

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| No projects exist | ProjectList shows empty state: "No projects yet" |
| Project with 0 gems | ProjectGemList shows metadata header + "No gems in this project. Click + Add Gems to get started." |
| Gem deleted from DB while in project | CASCADE removes the association. Next project load shows updated list |
| Same gem added to project twice | `INSERT OR IGNORE` — silently skipped, count returns 0 for that gem |
| Project deleted while selected | `onProjectsChanged()` refreshes list, `selectedProjectId` cleared |
| 100+ gems in a project | Scrollable gem list with `overflow-y: auto`. Limit param available but defaults to 100 |
| Search within project gems returns 0 | "No gems match your search" message |
| User switches nav away from Projects | State preserved via React component state (re-mounts on return to Projects nav) |

---

## Testing Strategy

### Unit Tests

**`sqlite_store.rs` tests** (using `new_in_memory()` pattern from `SqliteGemStore`):
- `create`: Verify UUID generation, status="active", timestamps set
- `list`: Verify ordering by `updated_at DESC`, gem_count accuracy
- `get`: Verify project + gems returned, "Project not found" on invalid id
- `update`: Verify partial updates (only `Some` fields changed), timestamp updated
- `delete`: Verify CASCADE removes associations
- `add_gems`: Verify idempotency (`INSERT OR IGNORE`), count accuracy
- `remove_gem`: Verify association removed, gem still exists in gems table
- `get_project_gems`: Verify ordering by `added_at DESC`, search filtering
- `get_gem_projects`: Verify reverse lookup

### Integration Tests

- Create project → add gems → verify `get_project` returns correct gems
- Delete gem from gems table → verify it disappears from project gem list (CASCADE)
- Delete project → verify gems still exist in gems table
- Add same gem to two projects → verify `get_gem_projects` returns both

### Manual Testing Checklist

- [ ] Create project with title only → verify active status, empty gem list
- [ ] Create project with all fields → verify description and objective shown
- [ ] Select project → verify gems load in right side
- [ ] Click gem → verify GemDetailPanel opens in right panel
- [ ] Add gems via modal → verify they appear in project
- [ ] Remove gem from project → verify it disappears, gem still in GemsPanel
- [ ] Delete project → verify gems unaffected, project list updated
- [ ] Edit project status → verify badge color changes
- [ ] Search within project gems → verify filtering works
- [ ] Add gem to project from GemsPanel card → verify dropdown works

---

## Summary

This design adds a `ProjectStore` trait with `SqliteProjectStore` implementation that shares `gems.db` with the existing gem store. Nine Tauri commands expose full CRUD + gem association management. The frontend introduces a split center panel (`ProjectList` + `ProjectGemList`) that gives users a master-detail view within the center panel. The right panel is unchanged — clicking a gem opens the existing `GemDetailPanel`. All patterns follow established Jarvis conventions: trait-based stores, `Arc<dyn Trait>` managed state, functional React components with hooks, and dark theme CSS with design tokens.
