# Projects — Group Gems Under Shared Goals

## Introduction

Jarvis captures knowledge as gems — articles, recordings, transcripts, emails, conversations. But gems are a flat, ever-growing list. When a user is researching "ECS Migration" or "MLX Fine-tuning", there's no way to group the relevant gems, track the research goal, or see all knowledge for an effort in one place.

This spec adds **Projects** — named containers that group gems under a shared goal with metadata (title, description, objective). Projects provide a focused view of related gems and lay the foundation for future synthesis ("summarize everything I know about ECS") and research recommendations ("find more gems relevant to this project").

The UI follows a **split center panel** pattern: project list on the left, gems under the selected project on the right. Selecting a gem opens the existing GemDetailPanel in the right panel. This creates a four-column drill-down: `Left Nav → Project List → Gem List → Gem Detail`.

**Reference:** Design doc at `discussion/29-feb-next-step/projects-feature-design.md`. Depends on: [Gems spec](../jarvis-gems/requirements.md) (fully implemented — `GemStore` trait, `SqliteGemStore`, Tauri commands, `GemsPanel`, `GemDetailPanel` all complete).

## Glossary

- **Project**: A named container grouping gems under a shared goal. Has title, description, objective, status, and timestamps. Stored in the `projects` SQLite table.
- **ProjectPreview**: A lightweight project representation for list views — title, status, gem count, description, last updated. Analogous to `GemPreview`.
- **ProjectDetail**: A full project representation including metadata and the list of associated gems. Returned by `get_project`.
- **Project-Gem Association**: A many-to-many link between a project and a gem, stored in the `project_gems` junction table. A gem can belong to multiple projects. An association can be created or removed without affecting the gem itself.
- **ProjectStore**: The backend-agnostic trait defining the contract for project CRUD operations. Analogous to `GemStore`, `KnowledgeStore`, `SearchResultProvider`.
- **ProjectsContainer**: The center panel component that renders the split layout — `ProjectList` on the left, `ProjectGemList` on the right.
- **AddGemsModal**: An overlay that lets the user search and select gems to add to a project.

## Frozen Design Decisions

These decisions were made during design review (2026-02-28):

1. **Flat junction table, not nested gems.** Gems exist independently. Projects are a view/grouping layer. Deleting a project removes associations but never deletes gems. Deleting a gem removes it from all projects automatically (CASCADE).
2. **Many-to-many.** A gem can belong to multiple projects. A research article about ECS might be relevant to both "ECS Migration" and "AWS Cost Optimization" projects.
3. **Split center panel.** Project list (fixed ~260px) and gem list (flex) render side by side in the center panel. No "Back" navigation needed — both lists are always visible. This gives a master-detail flow within the center panel.
4. **Right panel unchanged.** Clicking a gem in the project gem list opens GemDetailPanel in the right panel, identical to clicking a gem in GemsPanel. No new right panel components.
5. **Same database.** Projects live in `gems.db` alongside the gems table. No new database file.
6. **Reuse existing GemCard.** The project gem list reuses the `GemCard` component from `GemsPanel.tsx` with an added "remove from project" action. No card duplication.
7. **Three-field creation form.** Creating a project requires only a title. Description and objective are optional. No deadline or status picker on create — status defaults to "active", deadline can be added later via edit.
8. **Status is edit-only.** Projects always start as "active". Status (active/paused/completed/archived) is changed via the edit action, not at creation time.

---

## Requirement 1: Database Schema — Projects and Project-Gems Tables

**User Story:** As the Jarvis system, I need database tables to persist projects and their gem associations, so project data survives app restarts and gems can be grouped.

### Acceptance Criteria

1. THE System SHALL create a `projects` table in `gems.db` with the following columns:
   - `id` TEXT PRIMARY KEY (UUID v4)
   - `title` TEXT NOT NULL
   - `description` TEXT (nullable)
   - `objective` TEXT (nullable)
   - `status` TEXT NOT NULL DEFAULT 'active'
   - `created_at` TEXT NOT NULL (ISO 8601)
   - `updated_at` TEXT NOT NULL (ISO 8601)
2. THE System SHALL create a `project_gems` junction table with the following columns:
   - `project_id` TEXT NOT NULL, FOREIGN KEY referencing `projects(id)` ON DELETE CASCADE
   - `gem_id` TEXT NOT NULL, FOREIGN KEY referencing `gems(id)` ON DELETE CASCADE
   - `added_at` TEXT NOT NULL (ISO 8601)
   - PRIMARY KEY (`project_id`, `gem_id`)
3. THE System SHALL create indexes:
   - `idx_project_gems_gem` on `project_gems(gem_id)` for reverse lookups
   - `idx_projects_status` on `projects(status)` for filtering
   - `idx_projects_updated` on `projects(updated_at DESC)` for sorted listing
4. THE migration SHALL run on app startup using `CREATE TABLE IF NOT EXISTS` and `CREATE INDEX IF NOT EXISTS` — same pattern as existing gem table creation
5. THE `status` column SHALL accept values: `active`, `paused`, `completed`, `archived`
6. CASCADE deletes SHALL ensure: deleting a project removes all its associations; deleting a gem removes it from all projects

---

## Requirement 2: ProjectStore Trait and Data Types

**User Story:** As a developer, I need a backend-agnostic trait defining the contract for project operations, so all consumers (Tauri commands) interact through a stable interface consistent with other Jarvis traits.

### Acceptance Criteria

1. THE System SHALL define a `ProjectStore` trait in `src/projects/store.rs` with the `#[async_trait]` attribute and `Send + Sync` bounds
2. THE trait SHALL define the following methods:
   - `create(&self, project: CreateProject) -> Result<Project, String>`
   - `list(&self) -> Result<Vec<ProjectPreview>, String>`
   - `get(&self, id: &str) -> Result<ProjectDetail, String>`
   - `update(&self, id: &str, updates: UpdateProject) -> Result<Project, String>`
   - `delete(&self, id: &str) -> Result<(), String>`
   - `add_gems(&self, project_id: &str, gem_ids: &[String]) -> Result<usize, String>`
   - `remove_gem(&self, project_id: &str, gem_id: &str) -> Result<(), String>`
   - `get_project_gems(&self, project_id: &str, query: Option<&str>, limit: Option<usize>) -> Result<Vec<GemPreview>, String>`
   - `get_gem_projects(&self, gem_id: &str) -> Result<Vec<ProjectPreview>, String>`
3. THE System SHALL define a `Project` struct with fields: `id` (String), `title` (String), `description` (Option<String>), `objective` (Option<String>), `status` (String), `created_at` (String), `updated_at` (String)
4. THE System SHALL define a `ProjectPreview` struct with fields: `id` (String), `title` (String), `description` (Option<String>), `status` (String), `gem_count` (usize), `updated_at` (String)
5. THE System SHALL define a `ProjectDetail` struct with fields: `project` (Project), `gem_count` (usize), `gems` (Vec<GemPreview>)
6. THE System SHALL define a `CreateProject` struct with fields: `title` (String), `description` (Option<String>), `objective` (Option<String>)
7. THE System SHALL define an `UpdateProject` struct with fields: `title` (Option<String>), `description` (Option<String>), `objective` (Option<String>), `status` (Option<String>)
8. ALL structs SHALL derive `Debug`, `Clone`, `Serialize`, `Deserialize`
9. THE trait doc comment SHALL state: "Backend-agnostic project store. Tauri commands call this trait, never a concrete implementation."

---

## Requirement 3: SqliteProjectStore Implementation

**User Story:** As the Jarvis system, I need a SQLite-backed implementation of ProjectStore that persists projects and gem associations to the existing gems.db database.

### Acceptance Criteria

1. THE System SHALL implement `SqliteProjectStore` struct in `src/projects/sqlite_store.rs` with field: `db` (same database connection type used by `SqliteGemStore`)
2. THE `create()` method SHALL:
   a. Generate a UUID v4 for the project `id`
   b. Set `created_at` and `updated_at` to the current ISO 8601 timestamp
   c. Set `status` to `"active"`
   d. Insert into the `projects` table
   e. Return the created `Project`
3. THE `list()` method SHALL query all projects ordered by `updated_at DESC`, joining with `project_gems` to compute `gem_count` for each project
4. THE `get()` method SHALL:
   a. Query the project by `id` from the `projects` table
   b. Query associated gems from `project_gems` joined with `gems`, returning `Vec<GemPreview>`
   c. Return `ProjectDetail` with the project, gem count, and gem list
   d. Return `Err("Project not found")` if the project does not exist
5. THE `update()` method SHALL:
   a. Apply only the fields that are `Some` in `UpdateProject`
   b. Always update `updated_at` to the current timestamp
   c. Return the updated `Project`
   d. Return `Err("Project not found")` if the project does not exist
6. THE `delete()` method SHALL delete the project by `id` — CASCADE handles `project_gems` cleanup
7. THE `add_gems()` method SHALL:
   a. Insert rows into `project_gems` for each `gem_id` with `added_at` set to the current timestamp
   b. Use `INSERT OR IGNORE` to skip gems that are already associated
   c. Update the project's `updated_at` timestamp
   d. Return the count of newly added associations
8. THE `remove_gem()` method SHALL delete the row from `project_gems` matching `project_id` and `gem_id`, and update the project's `updated_at` timestamp
9. THE `get_project_gems()` method SHALL:
   a. Query gems associated with the project, ordered by `added_at DESC`
   b. If `query` is provided, filter using the existing FTS5 search restricted to project gems
   c. If `limit` is provided, apply it; otherwise return all
   d. Return `Vec<GemPreview>` using the same column mapping as `SqliteGemStore`
10. THE `get_gem_projects()` method SHALL query all projects associated with a gem, ordered by `updated_at DESC`, returning `Vec<ProjectPreview>`
11. THE `SqliteProjectStore` SHALL share the same database connection as `SqliteGemStore` (same `gems.db` file)

---

## Requirement 4: Tauri Commands

**User Story:** As the frontend, I need Tauri commands for all project operations, so the UI can create, list, view, edit, and delete projects, and manage gem associations.

### Acceptance Criteria

1. THE System SHALL expose a `create_project` Tauri command that accepts `title: String`, `description: Option<String>`, `objective: Option<String>` and returns `Result<Project, String>`
2. THE System SHALL expose a `list_projects` Tauri command that returns `Result<Vec<ProjectPreview>, String>`
3. THE System SHALL expose a `get_project` Tauri command that accepts `id: String` and returns `Result<ProjectDetail, String>`
4. THE System SHALL expose an `update_project` Tauri command that accepts `id: String`, `title: Option<String>`, `description: Option<String>`, `objective: Option<String>`, `status: Option<String>` and returns `Result<Project, String>`
5. THE System SHALL expose a `delete_project` Tauri command that accepts `id: String` and returns `Result<(), String>`
6. THE System SHALL expose an `add_gems_to_project` Tauri command that accepts `project_id: String`, `gem_ids: Vec<String>` and returns `Result<usize, String>`
7. THE System SHALL expose a `remove_gem_from_project` Tauri command that accepts `project_id: String`, `gem_id: String` and returns `Result<(), String>`
8. THE System SHALL expose a `get_project_gems` Tauri command that accepts `project_id: String`, `query: Option<String>`, `limit: Option<usize>` and returns `Result<Vec<GemPreview>, String>`
9. THE System SHALL expose a `get_gem_projects` Tauri command that accepts `gem_id: String` and returns `Result<Vec<ProjectPreview>, String>`
10. ALL commands SHALL use `State<'_, Arc<dyn ProjectStore>>` for dependency injection
11. ALL commands SHALL be registered in `lib.rs` in the `generate_handler!` macro

---

## Requirement 5: Module Structure and Provider Registration

**User Story:** As a developer, I need the projects module to follow Jarvis's existing module patterns, so the codebase remains consistent and the project store is properly registered.

### Acceptance Criteria

1. THE System SHALL create a `src/projects/` module with the following files:
   - `mod.rs` — module root, re-exports public types
   - `store.rs` — `ProjectStore` trait, `Project`, `ProjectPreview`, `ProjectDetail`, `CreateProject`, `UpdateProject`
   - `sqlite_store.rs` — `SqliteProjectStore` implementation
   - `commands.rs` — Tauri command handlers
2. THE `mod.rs` SHALL re-export: `ProjectStore`, `Project`, `ProjectPreview`, `ProjectDetail`, `CreateProject`, `UpdateProject`, `SqliteProjectStore`
3. THE projects module SHALL be added to `src/lib.rs` module declarations: `pub mod projects;`
4. THE `SqliteProjectStore` SHALL be created in `lib.rs` setup using the same database connection as `SqliteGemStore`
5. THE project store SHALL be registered as Tauri managed state: `app.manage(Arc::new(store) as Arc<dyn ProjectStore>)`
6. THE table migration (Requirement 1) SHALL run during `SqliteProjectStore::new()` or during the existing database initialization sequence
7. THE `Cargo.toml` SHALL NOT add any new dependencies — the projects module uses existing `rusqlite`, `uuid`, `serde`, `serde_json`, `async-trait`, `chrono`

---

## Requirement 6: TypeScript Types

**User Story:** As the frontend, I need TypeScript interfaces matching the Rust project types, so I can type-safely call Tauri commands and render project data.

### Acceptance Criteria

1. THE System SHALL define the following interfaces in `src/state/types.ts`:
   - `Project` with fields: `id: string`, `title: string`, `description: string | null`, `objective: string | null`, `status: 'active' | 'paused' | 'completed' | 'archived'`, `created_at: string`, `updated_at: string`
   - `ProjectPreview` with fields: `id: string`, `title: string`, `description: string | null`, `status: string`, `gem_count: number`, `updated_at: string`
   - `ProjectDetail` with fields: `project: Project`, `gem_count: number`, `gems: GemPreview[]`
2. ALL new types SHALL be exported from `types.ts`
3. THE types SHALL match the Rust struct field names exactly (snake_case, as Tauri serializes with serde)

---

## Requirement 7: Left Navigation — Projects Tab

**User Story:** As a user, I want a "Projects" option in the left navigation bar, so I can access the projects view.

### Acceptance Criteria

1. THE `ActiveNav` type in `LeftNav.tsx` SHALL be extended to include `'projects'`
2. A new nav item SHALL be added after `gems`: `{ id: 'projects', label: 'Projects', icon: '📁' }`
3. CLICKING the Projects nav item SHALL set `activeNav` to `'projects'`
4. THE center panel in `App.tsx` SHALL render `ProjectsContainer` when `activeNav === 'projects'`
5. THE `ProjectsContainer` component SHALL receive an `onGemSelect` callback to open gems in the right panel (same pattern as `GemsPanel`)

---

## Requirement 8: ProjectsContainer — Split Center Panel

**User Story:** As a user viewing projects, I want to see the project list on the left and the selected project's gems on the right within the center panel, so I can browse projects and their contents without losing context.

### Acceptance Criteria

1. THE System SHALL create a `ProjectsContainer` component that renders a horizontal split layout using `display: flex`
2. THE left side SHALL render `ProjectList` with `width: 260px`, `flex-shrink: 0`, and a right border separator
3. THE right side SHALL render `ProjectGemList` with `flex: 1` and `overflow-y: auto`
4. THE `ProjectsContainer` SHALL manage state:
   - `selectedProjectId: string | null` — currently selected project
   - `projects: ProjectPreview[]` — loaded project list
5. WHEN no project is selected, the right side SHALL show an empty state: "Select a project to see its gems"
6. WHEN a project is selected, the right side SHALL show the project metadata header and its gems

---

## Requirement 9: ProjectList Component

**User Story:** As a user, I want to see all my projects in a scrollable list with a button to create new ones, so I can quickly find and select the project I'm working on.

### Acceptance Criteria

1. THE `ProjectList` component SHALL render a header with the title "Projects" and a "+ New Project" button
2. THE component SHALL load projects on mount by calling `invoke<ProjectPreview[]>('list_projects')`
3. EACH project in the list SHALL be rendered as a card showing:
   - Title
   - Status badge (color-coded: active=green, paused=yellow, completed=blue, archived=gray)
   - Gem count (e.g., "5 gems")
   - Description (truncated to 2 lines)
4. THE currently selected project SHALL have an `active` visual state (highlighted background, same pattern as active nav items)
5. CLICKING a project card SHALL call the `onSelectProject(id)` callback
6. CLICKING the "+ New Project" button SHALL show the `CreateProjectForm` inline at the top of the list (above the project cards)
7. THE project list SHALL be scrollable independently from the gem list (own `overflow-y: auto`)
8. THE project list SHALL refresh after a project is created, updated, or deleted

---

## Requirement 10: CreateProjectForm Component

**User Story:** As a user, I want to quickly create a new project with a title and optional description/objective, so I can start grouping gems immediately.

### Acceptance Criteria

1. THE `CreateProjectForm` SHALL render inline at the top of the `ProjectList` when the user clicks "+ New Project"
2. THE form SHALL contain three fields:
   - **Title** (text input, required) — placeholder: "Project title"
   - **Description** (textarea, optional) — placeholder: "What is this project about?"
   - **Objective** (textarea, optional) — placeholder: "What are you trying to achieve?"
3. THE form SHALL have two buttons: "Cancel" and "Create Project"
4. THE "Create Project" button SHALL be disabled when title is empty
5. ON submit, THE form SHALL call `invoke('create_project', { title, description, objective })` where `description` and `objective` are `null` if empty
6. ON successful creation, THE form SHALL:
   a. Close the form (hide it)
   b. Refresh the project list
   c. Auto-select the newly created project
7. ON error, THE form SHALL display the error message inline below the form fields
8. THE "Cancel" button SHALL close the form without creating a project

---

## Requirement 11: ProjectGemList Component

**User Story:** As a user viewing a project, I want to see the project's metadata at the top and all its gems listed below with search and management options.

### Acceptance Criteria

1. THE `ProjectGemList` SHALL render a project metadata header at the top showing:
   - Project title (large text)
   - Status badge (color-coded)
   - Objective (if set, displayed below title)
   - Gem count
   - Action buttons: "Edit" and "Delete"
2. BELOW the metadata header, THE component SHALL render a toolbar with:
   - Search input for filtering project gems (placeholder: "Search project gems...")
   - "+ Add Gems" button
3. BELOW the toolbar, THE component SHALL render the list of gems associated with the project
4. THE gems SHALL be loaded by calling `invoke<GemPreview[]>('get_project_gems', { projectId })` when the selected project changes
5. EACH gem SHALL be rendered using the existing `GemCard` component from `GemsPanel.tsx`
6. EACH gem card SHALL have a remove action (x button or "Remove" button) that:
   a. Shows a confirmation (e.g., "Remove from project?")
   b. Calls `invoke('remove_gem_from_project', { projectId, gemId })`
   c. Removes the gem from the list without refreshing the entire list
   d. Does NOT delete the gem itself
7. CLICKING a gem card SHALL call `onGemSelect(gemId)` to open the gem detail in the right panel
8. THE "Delete" button on the project metadata SHALL:
   a. Show a confirmation dialog ("Delete project? Gems will not be deleted.")
   b. Call `invoke('delete_project', { id })`
   c. Clear the selected project
   d. Refresh the project list
9. THE "Edit" button SHALL toggle inline editing of the project metadata (title, description, objective, status)
10. THE search input SHALL filter gems by calling `invoke('get_project_gems', { projectId, query })` with 300ms debounce

---

## Requirement 12: AddGemsModal Component

**User Story:** As a user, I want to search and select gems to add to my project, so I can build a collection of relevant knowledge.

### Acceptance Criteria

1. THE `AddGemsModal` SHALL render as a modal overlay when the "+ Add Gems" button is clicked
2. THE modal SHALL contain:
   - A title: "Add Gems to {project title}"
   - A search input for filtering gems
   - A scrollable list of gem cards with checkboxes
   - A footer with "Cancel" and "Add Selected ({count})" buttons
3. THE modal SHALL load all gems on mount by calling `invoke<GemSearchResult[]>('search_gems', { query: '', limit: 100 })`
4. THE search input SHALL filter gems with 300ms debounce by calling `invoke<GemSearchResult[]>('search_gems', { query, limit: 100 })`
5. GEMS already associated with the project SHALL be shown with a pre-checked, disabled checkbox and a "Already added" label
6. THE user SHALL be able to select multiple gems via checkboxes
7. THE "Add Selected" button SHALL:
   a. Call `invoke('add_gems_to_project', { projectId, gemIds })` with the selected gem IDs
   b. Close the modal
   c. Refresh the project gem list
8. THE "Add Selected" button SHALL be disabled when no new gems are selected
9. THE modal SHALL be closeable via the "Cancel" button, clicking outside the modal, or pressing Escape

---

## Requirement 13: Add to Project from Gem Card

**User Story:** As a user browsing gems in the GemsPanel, I want to add a gem to one or more projects directly from the gem card, without leaving the gems view.

### Acceptance Criteria

1. THE `GemCard` component in `GemsPanel.tsx` SHALL have an "Add to Project" action button (e.g., a `📁+` icon button) in the gem card action bar
2. CLICKING the button SHALL show a dropdown listing all projects (loaded via `invoke<ProjectPreview[]>('list_projects')`)
3. EACH project in the dropdown SHALL show the project title and a checkbox
4. PROJECTS that already contain this gem SHALL be pre-checked (loaded via `invoke<ProjectPreview[]>('get_gem_projects', { gemId })`)
5. THE user SHALL be able to check/uncheck projects to add/remove the gem
6. CHECKING a project SHALL call `invoke('add_gems_to_project', { projectId, gemIds: [gemId] })`
7. UNCHECKING a project SHALL call `invoke('remove_gem_from_project', { projectId, gemId })`
8. THE dropdown SHALL close when clicking outside of it

---

## Requirement 14: CSS Styling

**User Story:** As a user, I want the projects UI to look consistent with the rest of the Jarvis app, following the dark theme and existing design patterns.

### Acceptance Criteria

1. THE `ProjectsContainer` split layout SHALL use the same border and background colors as existing panel dividers
2. THE `ProjectList` sidebar SHALL match the left nav styling (dark background, hover effects, active state highlight)
3. PROJECT cards in the list SHALL follow the same card pattern as gem cards (`.gem-card` styling) adapted for project metadata
4. STATUS badges SHALL be color-coded:
   - `active` — green background (matching existing "available" indicators)
   - `paused` — yellow/amber background
   - `completed` — blue background
   - `archived` — gray background
5. THE `CreateProjectForm` SHALL match existing form styling in the app (input fields, buttons, spacing)
6. THE `AddGemsModal` SHALL use a dark overlay with a centered modal card, matching the app's dark theme
7. THE "remove from project" button on gem cards (x) SHALL be styled subtly (muted color, visible on hover)
8. ALL new CSS SHALL be added to `App.css` alongside existing component styles
9. THE split layout SHALL be responsive: project list stays at 260px, gem list flexes to fill remaining space

---

## Technical Constraints

1. **React + TypeScript**: Frontend uses React functional components with hooks. No class components.
2. **Tauri invoke**: All backend calls use `invoke()` from `@tauri-apps/api/core`. Type the return values.
3. **Trait-based architecture**: Consumers use `dyn ProjectStore` — never a concrete type. Consistent with `GemStore`, `KnowledgeStore`, `SearchResultProvider`.
4. **Existing database**: Projects tables are added to `gems.db`. No new database file.
5. **Reuse GemCard**: The project gem list uses the same `GemCard` component. The component may need an optional `onRemove` callback prop for the project context.
6. **No new Rust crate dependencies**: Uses existing `rusqlite`, `uuid`, `serde`, `serde_json`, `async-trait`, `chrono`.
7. **No new npm dependencies**: All UI components are built with React primitives. No form libraries, modal libraries, etc.
8. **GemPreview reuse**: Project gem lists return `GemPreview` (or `GemSearchResult` where needed) — the same type used by `GemsPanel`. No new gem display types.
9. **Cascade deletes**: SQLite FOREIGN KEY constraints with ON DELETE CASCADE handle referential integrity. `PRAGMA foreign_keys = ON` must be set on the database connection.

## Out of Scope

1. **Project synthesis** — LLM-powered synthesis across project gems. Future feature.
2. **Smart gem recommendations** — "Find gems relevant to this project" via semantic search. Future feature.
3. **Web research suggestions** — Tavily-powered research recommendations. Future feature.
4. **Drag-and-drop gem ordering** within a project — gems are ordered by `added_at`.
5. **Project templates** — pre-configured project types.
6. **Project sharing/export** — exporting a project and its gems as a bundle.
7. **Project-scoped search** — searching gems within a single project is handled by `get_project_gems` with a query parameter, but there is no separate search provider scope.
8. **Deadline field** — dropped from creation form to reduce friction. Can be added to `UpdateProject` later if needed.
9. **Project knowledge files** — generating a project-level knowledge document. Future feature.
10. **Nested projects / sub-projects** — projects are flat, no hierarchy.
