# Projects Feature — Design Document

**Date:** Feb 28, 2026
**Status:** Draft
**Depends on:** Gems (existing), Knowledge Files (existing)

---

## Problem

Gems are flat — a growing, unsorted list. Users capture knowledge about specific topics (e.g., "ECS migration", "MLX fine-tuning research") but have no way to group related gems, track project context, or see all knowledge for a goal in one place.

---

## Core Concept

A **Project** is a named container that groups gems under a shared goal. It carries its own metadata (title, description, objective, status, deadline) and provides a focused view of all knowledge relevant to that effort.

---

## User Flows

### Flow 1: Create a Project

1. User clicks **Projects** in the left nav
2. Center panel shows the **projects list** (empty state on first visit)
3. User clicks **"+ New Project"** button at the top of the list
4. A **create project form** appears in the center panel with:
   - **Title** (required) — short name, e.g., "ECS Migration Research"
   - **Description** (optional) — what this project is about
   - **Objective** (optional) — the goal or deliverable, e.g., "Decide between ECS and EKS by March 10"
   - **Deadline** (optional) — target completion date
   - **Status** (default: "active") — active / paused / completed / archived
5. User submits → project is created → redirected to the new project's detail view

### Flow 2: Browse Projects

1. User clicks **Projects** in the left nav
2. Center panel shows a **list of projects**, sorted by last updated (most recent first)
3. Each project card shows:
   - Title
   - Status badge (active/paused/completed/archived)
   - Gem count
   - Description (truncated)
   - Last updated date
4. User clicks a project → center panel switches to **project detail view**

### Flow 3: View Project Detail

1. User taps on a project from the list
2. Center panel shows the **project detail view**:

   **Top section — Project metadata:**
   - Title (editable inline or via edit button)
   - Description
   - Objective
   - Status badge
   - Deadline (if set)
   - Gem count
   - Created / last updated dates
   - Edit and Delete buttons

   **Bottom section — Gems list:**
   - List of all gems assigned to this project (same card layout as GemsPanel)
   - Search/filter within project gems
   - "Add Gems" button to assign existing gems
   - Each gem card has a "Remove from project" option

3. User clicks a gem → **right panel** opens the gem detail (same as current GemDetailPanel behavior)

### Flow 4: Add Gems to a Project

Two entry points:

**A. From Project Detail ("Add Gems" button):**
1. Opens a modal/overlay with the full gem list (searchable)
2. Each gem shows a checkbox
3. Gems already in this project are pre-checked and disabled
4. User checks the gems to add → clicks "Add Selected"
5. Gems appear in the project's gem list

**B. From Gem Card / Gem Detail ("Add to Project"):**
1. On any gem card (in GemsPanel or project detail), there's an "Add to Project" action
2. Opens a dropdown/modal listing all projects
3. User selects one or more projects
4. Gem is linked to those projects
5. A gem can belong to multiple projects (many-to-many)

### Flow 5: Remove Gems from a Project

1. In the project detail gem list, each gem card has a "Remove" button
2. Click → confirmation → gem is unlinked from the project
3. The gem itself is NOT deleted — only the association is removed

---

## UI Layout (Three-Panel Integration)

The center panel is a **split container** — project list on the left, gems under the selected project on the right. This keeps both lists visible simultaneously.

### State: No project selected (initial)

```
+------------------+-------------------+---------------------+---------------------+
| Left Nav (180px) | Project List      | Gem List            | Right Panel         |
|                  | (fixed ~260px)    | (flex)              | (resizable)         |
+------------------+-------------------+---------------------+---------------------+
| Record           | + New Project     |                     |                     |
| Recordings       | [Search...]       | Select a project    |                     |
| Gems             |                   | to see its gems     |                     |
| > Projects  <--- | ┌───────────────┐ |                     |                     |
| YouTube          | │ ECS Migration ▸│ |                     |                     |
| Browser          | │ active · 5 gems│ |                     |                     |
|                  | └───────────────┘ |                     |                     |
|                  | ┌───────────────┐ |                     |                     |
|                  | │ MLX Fine-tune ▸│ |                     |                     |
|                  | │ paused · 3 gems│ |                     |                     |
|                  | └───────────────┘ |                     |                     |
|                  | ┌───────────────┐ |                     |                     |
|                  | │ AWS Costs     ▸│ |                     |                     |
|                  | │ active · 8 gems│ |                     |                     |
|                  | └───────────────┘ |                     |                     |
|                  |                   |                     |                     |
| Settings         |                   |                     |                     |
+------------------+-------------------+---------------------+---------------------+
```

### State: Project selected, no gem selected

```
+------------------+-------------------+---------------------+---------------------+
| Left Nav (180px) | Project List      | Gem List            | Right Panel         |
+------------------+-------------------+---------------------+---------------------+
| Record           | + New Project     | ECS Migration       |                     |
| Recordings       | [Search...]       | Status: active      |                     |
| Gems             |                   | Objective: Decide.. |                     |
| > Projects       | ┌───────────────┐ | Deadline: Mar 10    |                     |
| YouTube          | │▶ECS Migration │ | 5 gems · [Edit]     |                     |
| Browser          | │ active · 5 gems│ | ─────────────────── |                     |
|                  | └───────────────┘ | [Search gems...][+] |                     |
|                  | ┌───────────────┐ |                     |                     |
|                  | │ MLX Fine-tune │ | ┌─────────────────┐ |                     |
|                  | │ paused · 3 gems│ | │ ECS vs EKS Guide× |                     |
|                  | └───────────────┘ | │ Article · aws.com│ |                     |
|                  | ┌───────────────┐ | └─────────────────┘ |                     |
|                  | │ AWS Costs     │ | ┌─────────────────┐ |                     |
|                  | │ active · 8 gems│ | │ Container notes ×│ |                     |
|                  | └───────────────┘ | │ Recording        │ |                     |
|                  |                   | └─────────────────┘ |                     |
| Settings         |                   |                     |                     |
+------------------+-------------------+---------------------+---------------------+
```

### State: Project selected + gem selected (full three-panel)

```
+------------------+-------------------+---------------------+---------------------+
| Left Nav (180px) | Project List      | Gem List            | Gem Detail Panel    |
+------------------+-------------------+---------------------+---------------------+
| Record           | + New Project     | ECS Migration       | ECS vs EKS Guide   |
| Recordings       | [Search...]       | Status: active      | Source: Article     |
| Gems             |                   | 5 gems · [Edit]     | aws.amazon.com     |
| > Projects       | ┌───────────────┐ | ─────────────────── | ───────────────── |
| YouTube          | │▶ECS Migration │ | [Search gems...][+] | Tags: aws, ecs,   |
| Browser          | │ active · 5 gems│ |                     |   containers       |
|                  | └───────────────┘ | ┌─────────────────┐ | Summary: Compares  |
|                  | ┌───────────────┐ | │▶ECS vs EKS Guide│ |   ECS and EKS...   |
|                  | │ MLX Fine-tune │ | │ Article · aws.com│ | ───────────────── |
|                  | │ paused · 3 gems│ | └─────────────────┘ | Content:           |
|                  | └───────────────┘ | ┌─────────────────┐ | Amazon ECS is a... |
|                  |                   | │ Container notes ×│ |                     |
|                  |                   | │ Recording        │ | Knowledge Files:   |
|                  |                   | └─────────────────┘ |  content.md (2KB)  |
|                  |                   |                     |  enrichment.md     |
| Settings         |                   |                     |  gem.md            |
+------------------+-------------------+---------------------+---------------------+
```

### Gem list header detail

The top of the gem list pane shows a **compact project metadata bar** when a project is selected:

```
┌─────────────────────────────────────────────┐
│ ECS Migration Research                      │
│ Status: ● active    Deadline: Mar 10, 2026  │
│ Objective: Decide between ECS and EKS       │
│ 5 gems                        [Edit] [Delete]│
│─────────────────────────────────────────────│
│ [Search project gems...]         [+ Add Gems]│
│                                             │
│ ┌─────────────────────────────────────────┐ │
│ │ ECS vs EKS Guide                     × │ │
│ │ Article · aws.amazon.com               │ │
│ └─────────────────────────────────────────┘ │
│ ┌─────────────────────────────────────────┐ │
│ │ Container orchestration notes         × │ │
│ │ Recording · jarvis-app                 │ │
│ └─────────────────────────────────────────┘ │
└─────────────────────────────────────────────┘
```

The `×` on each gem card removes it from the project (not delete). The `[+ Add Gems]` button opens the AddGemsModal.

---

## Data Model

### `projects` table (SQLite)

| Column        | Type     | Constraints                  |
|---------------|----------|------------------------------|
| `id`          | TEXT     | PRIMARY KEY (UUID v4)        |
| `title`       | TEXT     | NOT NULL                     |
| `description` | TEXT     | nullable                     |
| `objective`   | TEXT     | nullable                     |
| `deadline`    | TEXT     | nullable (ISO 8601)          |
| `status`      | TEXT     | NOT NULL DEFAULT 'active'    |
| `created_at`  | TEXT     | NOT NULL (ISO 8601)          |
| `updated_at`  | TEXT     | NOT NULL (ISO 8601)          |

**Status values:** `active`, `paused`, `completed`, `archived`

### `project_gems` table (Junction)

| Column       | Type | Constraints                                             |
|--------------|------|---------------------------------------------------------|
| `project_id` | TEXT | NOT NULL, FOREIGN KEY → projects(id) ON DELETE CASCADE  |
| `gem_id`     | TEXT | NOT NULL, FOREIGN KEY → gems(id) ON DELETE CASCADE      |
| `added_at`   | TEXT | NOT NULL (ISO 8601)                                     |

**Constraints:**
- `PRIMARY KEY (project_id, gem_id)` — a gem can only be added to a project once
- Cascade deletes: deleting a project removes all associations; deleting a gem removes it from all projects

---

## Tauri Commands (Backend RPC)

| Command                    | Params                                                       | Returns                  |
|----------------------------|--------------------------------------------------------------|--------------------------|
| `create_project`           | `title, description?, objective?, deadline?`                 | `Project`                |
| `list_projects`            |                                                              | `Vec<ProjectPreview>`    |
| `get_project`              | `id`                                                         | `ProjectDetail`          |
| `update_project`           | `id, title?, description?, objective?, deadline?, status?`   | `Project`                |
| `delete_project`           | `id`                                                         | `()`                     |
| `add_gems_to_project`      | `project_id, gem_ids: Vec<String>`                           | `usize` (count added)    |
| `remove_gem_from_project`  | `project_id, gem_id`                                         | `()`                     |
| `get_project_gems`         | `project_id, query?, limit?`                                 | `Vec<GemPreview>`        |
| `get_gem_projects`         | `gem_id`                                                     | `Vec<ProjectPreview>`    |

---

## Types

### Rust

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub objective: Option<String>,
    pub deadline: Option<String>,
    pub status: String,         // "active" | "paused" | "completed" | "archived"
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectPreview {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub gem_count: usize,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDetail {
    pub project: Project,
    pub gem_count: usize,
    pub gems: Vec<GemPreview>,
}
```

### TypeScript

```typescript
interface Project {
    id: string;
    title: string;
    description: string | null;
    objective: string | null;
    deadline: string | null;
    status: 'active' | 'paused' | 'completed' | 'archived';
    created_at: string;
    updated_at: string;
}

interface ProjectPreview {
    id: string;
    title: string;
    description: string | null;
    status: string;
    gem_count: number;
    updated_at: string;
}

interface ProjectDetail {
    project: Project;
    gem_count: number;
    gems: GemPreview[];
}
```

---

## Frontend Components

| Component              | Location                    | Responsibility                                              |
|------------------------|-----------------------------|-------------------------------------------------------------|
| `ProjectsContainer`    | Center panel (root)         | Split layout — renders ProjectList + ProjectGemList side by side |
| `ProjectList`          | Center panel (left ~260px)  | Scrollable project list, "New Project" button, search       |
| `ProjectGemList`       | Center panel (right flex)   | Project metadata header + gem cards for selected project    |
| `CreateProjectForm`    | Inline in ProjectList       | Form for creating a new project (replaces empty state)      |
| `AddGemsModal`         | Overlay/modal               | Gem picker — search + checkbox multi-select                 |
| `AddToProjectDropdown` | Gem card action             | Dropdown listing projects to assign a gem to                |

### Center Panel Split Layout

```
ProjectsContainer (display: flex, flex-direction: row)
├── ProjectList (width: 260px, flex-shrink: 0, border-right)
│   ├── Header: "Projects" + "+ New Project" button
│   ├── Search input
│   └── Scrollable list of ProjectCard items
│       └── ProjectCard: title, status badge, gem count, description
└── ProjectGemList (flex: 1, overflow-y: auto)
    ├── [Empty state if no project selected]
    ├── Project metadata header (title, status, objective, deadline, actions)
    ├── Search gems + "Add Gems" button
    └── Scrollable list of GemCards (reused from GemsPanel)
        └── Each card has "×" remove-from-project button
```

### Left Nav Change

```typescript
type ActiveNav = 'record' | 'recordings' | 'gems' | 'projects' | 'youtube' | 'browser' | 'settings';
//                                                    ^^^^^^^^^ NEW
```

Add `{ id: 'projects', label: 'Projects', icon: '📁' }` to `navItems` after `gems`.

---

## Backend Architecture

Follow existing patterns:

```
src-tauri/src/projects/
    mod.rs              — module root, re-exports
    store.rs            — ProjectStore trait (async_trait)
    sqlite_store.rs     — SQLite implementation (same pattern as gems/sqlite_store.rs)
    commands.rs         — Tauri commands (same pattern as gems commands in src/commands.rs)
```

**`ProjectStore` trait:**
```rust
#[async_trait]
pub trait ProjectStore: Send + Sync {
    async fn create(&self, project: Project) -> Result<Project, String>;
    async fn list(&self) -> Result<Vec<ProjectPreview>, String>;
    async fn get(&self, id: &str) -> Result<ProjectDetail, String>;
    async fn update(&self, id: &str, updates: ProjectUpdate) -> Result<Project, String>;
    async fn delete(&self, id: &str) -> Result<(), String>;
    async fn add_gems(&self, project_id: &str, gem_ids: &[String]) -> Result<usize, String>;
    async fn remove_gem(&self, project_id: &str, gem_id: &str) -> Result<(), String>;
    async fn get_project_gems(&self, project_id: &str, query: Option<&str>, limit: Option<usize>) -> Result<Vec<GemPreview>, String>;
    async fn get_gem_projects(&self, gem_id: &str) -> Result<Vec<ProjectPreview>, String>;
}
```

**SQLite implementation** reuses the existing `gems.db` database — just adds new tables via migration on app startup.

---

## Migration Strategy

On app launch, check if `projects` table exists. If not, run:

```sql
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    objective TEXT,
    deadline TEXT,
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
```

---

## Build Sequence

| Phase | Work                                  | Effort   |
|-------|---------------------------------------|----------|
| 1     | DB tables + migration                 | 0.5 day  |
| 2     | `ProjectStore` trait + SQLite impl    | 0.5 day  |
| 3     | Tauri commands + `lib.rs` wiring      | 0.5 day  |
| 4     | TypeScript types                      | 0.25 day |
| 5     | `ProjectsPanel` (list + create form)  | 0.5 day  |
| 6     | `ProjectDetailPanel` (metadata + gems)| 0.5 day  |
| 7     | `AddGemsModal` + "Add to Project"     | 0.5 day  |
| 8     | Left nav + routing integration        | 0.25 day |
| 9     | Testing + polish                      | 0.5 day  |
|       | **Total**                             | **4 days**|

---

## Design Decisions

1. **Flat junction table, not nested gems** — Gems exist independently. Projects are a view/grouping layer. Deleting a project never deletes gems.

2. **Many-to-many** — A gem can belong to multiple projects. A research article about ECS might be relevant to both "ECS Migration" and "AWS Cost Optimization" projects.

3. **Reuse existing GemCard** — Project detail gem list uses the same `GemCard` component with an added "Remove from project" action. No duplication.

4. **Same database** — Projects live in `gems.db` alongside the gems table. No new database file.

5. **Split center panel** — Project list (fixed ~260px) and gem list (flex) render side by side in the center panel. No "Back" navigation needed — both lists are always visible. Selecting a project populates the right half; selecting a gem opens the right panel. This gives the user a master → detail → deep-detail flow across all four columns: `Left Nav → Project List → Gem List → Gem Detail`.

6. **Right panel unchanged** — Clicking a gem in the project gem list opens GemDetailPanel in the right panel, identical to clicking a gem in GemsPanel. No new right panel components.

---

## Future Extensions (Not in Scope)

- **Project synthesis** — LLM-powered synthesis across all project gems ("summarize everything I know about ECS"). Planned for roadmap Feature 2.
- **Smart gem recommendations** — "Find gems relevant to this project" via semantic search. Planned for roadmap Feature 3.
- **Web research suggestions** — Tavily-powered research recommendations based on project context. Planned for roadmap Feature 4.
- **Project templates** — Pre-configured project types (research, learning, meeting series).
- **Project sharing/export** — Export a project and all its gems as a knowledge bundle.
