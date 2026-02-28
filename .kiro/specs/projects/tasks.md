# Projects — Implementation Tasks

## Phase 1: Backend — Trait, Types, and Module Structure (Requirements 1, 2, 5)

### Task 1: Create `src/projects/store.rs` — Trait and Data Types

- [ ] 1.1 Create `src-tauri/src/projects/` directory
- [ ] 1.2 Create `store.rs` with data structs: `Project`, `ProjectPreview`, `ProjectDetail`, `CreateProject`, `UpdateProject` — all deriving `Debug`, `Clone`, `Serialize`, `Deserialize`
- [ ] 1.3 Import `GemPreview` from `crate::gems` for use in `ProjectDetail`
- [ ] 1.4 Define `ProjectStore` trait with `#[async_trait]` and `Send + Sync` bounds:
  - `create(&self, input: CreateProject) -> Result<Project, String>`
  - `list(&self) -> Result<Vec<ProjectPreview>, String>`
  - `get(&self, id: &str) -> Result<ProjectDetail, String>`
  - `update(&self, id: &str, updates: UpdateProject) -> Result<Project, String>`
  - `delete(&self, id: &str) -> Result<(), String>`
  - `add_gems(&self, project_id: &str, gem_ids: &[String]) -> Result<usize, String>`
  - `remove_gem(&self, project_id: &str, gem_id: &str) -> Result<(), String>`
  - `get_project_gems(&self, project_id: &str, query: Option<&str>, limit: Option<usize>) -> Result<Vec<GemPreview>, String>`
  - `get_gem_projects(&self, gem_id: &str) -> Result<Vec<ProjectPreview>, String>`
- [ ] 1.5 Add trait doc comment: "Backend-agnostic project store. Tauri commands call this trait, never a concrete implementation."

### Task 2: Create `src/projects/mod.rs` — Module Root

- [ ] 2.1 Create `mod.rs` with `pub mod store; pub mod sqlite_store; pub mod commands;`
- [ ] 2.2 Re-export public types: `ProjectStore`, `Project`, `ProjectPreview`, `ProjectDetail`, `CreateProject`, `UpdateProject`, `SqliteProjectStore`
- [ ] 2.3 Add `pub mod projects;` to `src-tauri/src/lib.rs` module declarations

**Checkpoint**: `cargo check` passes. All types defined, no implementations yet.

---

## Phase 2: Backend — SqliteProjectStore (Requirements 1, 3)

### Task 3: Create `src/projects/sqlite_store.rs` — Schema and Constructor

- [ ] 3.1 Define `SqliteProjectStore` struct with field `conn: Arc<Mutex<Connection>>`
- [ ] 3.2 Implement `new(conn: Arc<Mutex<Connection>>) -> Result<Self, String>` constructor that calls `initialize_schema()`
- [ ] 3.3 Implement `initialize_schema()` that creates tables via `CREATE TABLE IF NOT EXISTS`:
  - `projects` table: `id TEXT PRIMARY KEY`, `title TEXT NOT NULL`, `description TEXT`, `objective TEXT`, `status TEXT NOT NULL DEFAULT 'active'`, `created_at TEXT NOT NULL`, `updated_at TEXT NOT NULL`
  - `project_gems` junction table: `project_id TEXT NOT NULL`, `gem_id TEXT NOT NULL`, `added_at TEXT NOT NULL`, `PRIMARY KEY (project_id, gem_id)`, foreign keys with `ON DELETE CASCADE`
- [ ] 3.4 Create indexes: `idx_project_gems_gem` on `project_gems(gem_id)`, `idx_projects_status` on `projects(status)`, `idx_projects_updated` on `projects(updated_at DESC)`
- [ ] 3.5 Enable `PRAGMA foreign_keys = ON` in `initialize_schema()`

### Task 4: Implement CRUD methods on SqliteProjectStore

- [ ] 4.1 Implement `create()`: generate UUID v4, set `status = "active"`, set timestamps to `chrono::Utc::now().to_rfc3339()`, INSERT into projects, return `Project`
- [ ] 4.2 Implement `list()`: SELECT with LEFT JOIN on `project_gems` for `gem_count`, GROUP BY, ORDER BY `updated_at DESC`, return `Vec<ProjectPreview>`
- [ ] 4.3 Implement `get()`: query project by id, query associated gems via JOIN with `project_gems`, build `GemPreview` from gem columns (reuse `parse_ai_enrichment` helper), return `ProjectDetail` or `Err("Project not found")`
- [ ] 4.4 Implement `update()`: dynamic SET clause for `Some` fields only, always update `updated_at`, return `Err("Project not found")` if 0 rows affected
- [ ] 4.5 Implement `delete()`: DELETE by id, CASCADE handles `project_gems` cleanup

### Task 5: Implement gem association methods on SqliteProjectStore

- [ ] 5.1 Implement `add_gems()`: INSERT OR IGNORE for each gem_id with `added_at` timestamp, update project's `updated_at`, return count of newly added
- [ ] 5.2 Implement `remove_gem()`: DELETE from `project_gems` by composite key, update project's `updated_at`
- [ ] 5.3 Implement `get_project_gems()`: query gems via JOIN, support optional FTS5 search filtering with `gems_fts MATCH`, support optional limit (default 100), order by `added_at DESC`
- [ ] 5.4 Implement `get_gem_projects()`: query projects via JOIN on `project_gems`, include subquery for `gem_count`, order by `updated_at DESC`
- [ ] 5.5 Add `parse_ai_enrichment()` helper function (same logic as `SqliteGemStore` — extracts tags, summary, enrichment_source from JSON)

**Checkpoint**: `cargo check` passes. Full `SqliteProjectStore` implementation compiles.

---

## Phase 3: Backend — Tauri Commands and Registration (Requirements 4, 5)

### Task 6: Create `src/projects/commands.rs` — Tauri Commands

- [ ] 6.1 Implement `create_project` command: accepts `title`, `description`, `objective`, uses `State<'_, Arc<dyn ProjectStore>>`, returns `Result<Project, String>`
- [ ] 6.2 Implement `list_projects` command: returns `Result<Vec<ProjectPreview>, String>`
- [ ] 6.3 Implement `get_project` command: accepts `id`, returns `Result<ProjectDetail, String>`
- [ ] 6.4 Implement `update_project` command: accepts `id`, `title`, `description`, `objective`, `status` (all optional except id), returns `Result<Project, String>`
- [ ] 6.5 Implement `delete_project` command: accepts `id`, returns `Result<(), String>`
- [ ] 6.6 Implement `add_gems_to_project` command: accepts `project_id`, `gem_ids: Vec<String>`, returns `Result<usize, String>`
- [ ] 6.7 Implement `remove_gem_from_project` command: accepts `project_id`, `gem_id`, returns `Result<(), String>`
- [ ] 6.8 Implement `get_project_gems` command: accepts `project_id`, `query: Option<String>`, `limit: Option<usize>`, returns `Result<Vec<GemPreview>, String>`
- [ ] 6.9 Implement `get_gem_projects` command: accepts `gem_id`, returns `Result<Vec<ProjectPreview>, String>`

### Task 7: Register project store and commands in `lib.rs`

- [ ] 7.1 Refactor `lib.rs` to create shared `Arc<Mutex<Connection>>` for `gems.db` and pass to both `SqliteGemStore` and `SqliteProjectStore`
  - Add `SqliteGemStore::from_conn(conn: Arc<Mutex<Connection>>)` constructor (or expose `get_conn()`)
  - Create `SqliteProjectStore::new(conn_arc.clone())`
- [ ] 7.2 Register `Arc<dyn ProjectStore>` as Tauri managed state: `app.manage(project_store_arc)`
- [ ] 7.3 Add all 9 project commands to `generate_handler![]` macro:
  - `projects::commands::create_project`
  - `projects::commands::list_projects`
  - `projects::commands::get_project`
  - `projects::commands::update_project`
  - `projects::commands::delete_project`
  - `projects::commands::add_gems_to_project`
  - `projects::commands::remove_gem_from_project`
  - `projects::commands::get_project_gems`
  - `projects::commands::get_gem_projects`

**Checkpoint**: `cargo build` succeeds. All 9 Tauri commands registered. Backend is fully functional.

---

## Phase 4: Frontend — TypeScript Types and Navigation (Requirements 6, 7)

### Task 8: Add TypeScript types

- [ ] 8.1 Add `Project` interface to `src/state/types.ts`: `id`, `title`, `description`, `objective`, `status` (union type), `created_at`, `updated_at`
- [ ] 8.2 Add `ProjectPreview` interface: `id`, `title`, `description`, `status`, `gem_count`, `updated_at`
- [ ] 8.3 Add `ProjectDetail` interface: `project: Project`, `gem_count: number`, `gems: GemPreview[]`
- [ ] 8.4 Export all new types

### Task 9: Add Projects to navigation

- [ ] 9.1 Update `ActiveNav` type in `LeftNav.tsx` to include `'projects'`
- [ ] 9.2 Add nav item: `{ id: 'projects', label: 'Projects', icon: '📁' }` after gems
- [ ] 9.3 Update `App.tsx` center panel rendering to handle `activeNav === 'projects'` — render `ProjectsContainer`
- [ ] 9.4 Update `showRightPanel` logic to include `'projects'`
- [ ] 9.5 Pass `onGemSelect` callback to `ProjectsContainer`

**Checkpoint**: Frontend builds. Projects nav item visible. Clicking it shows empty ProjectsContainer.

---

## Phase 5: Frontend — ProjectsContainer and ProjectList (Requirements 8, 9, 10)

### Task 10: Create `ProjectsContainer` component

- [ ] 10.1 Create `src/components/ProjectsContainer.tsx`
- [ ] 10.2 Implement split layout with `display: flex`: `ProjectList` (260px) + `ProjectGemList` (flex: 1)
- [ ] 10.3 Manage state: `projects: ProjectPreview[]`, `selectedProjectId: string | null`, `loading: boolean`
- [ ] 10.4 Fetch projects on mount via `invoke<ProjectPreview[]>('list_projects')`
- [ ] 10.5 Pass `onGemSelect` callback through to `ProjectGemList`

### Task 11: Create `ProjectList` component

- [ ] 11.1 Create `ProjectList` as a component within `ProjectsContainer.tsx` or separate file
- [ ] 11.2 Render header with "Projects" title and "+ New Project" button
- [ ] 11.3 Render project cards: title, status badge (color-coded), gem count, description (truncated 2 lines)
- [ ] 11.4 Highlight selected project with `.active` class
- [ ] 11.5 Handle empty state: "No projects yet" message
- [ ] 11.6 Call `onSelectProject(id)` on card click

### Task 12: Create `CreateProjectForm` component

- [ ] 12.1 Create inline form shown when "+ New Project" clicked
- [ ] 12.2 Three fields: Title (text input, required), Description (textarea, optional), Objective (textarea, optional)
- [ ] 12.3 "Create Project" button disabled when title is empty
- [ ] 12.4 On submit: call `invoke('create_project', { title, description, objective })`, close form, refresh list, auto-select new project
- [ ] 12.5 "Cancel" button closes form without action
- [ ] 12.6 Show error message inline on failure

**Checkpoint**: Frontend builds. Can create projects, see them in list, select them.

---

## Phase 6: Frontend — ProjectGemList and AddGemsModal (Requirements 11, 12)

### Task 13: Create `ProjectGemList` component

- [ ] 13.1 Render empty state when no project selected: "Select a project to see its gems"
- [ ] 13.2 Load project detail on `projectId` change via `invoke<ProjectDetail>('get_project', { id })`
- [ ] 13.3 Render project metadata header: title, status badge, objective, gem count
- [ ] 13.4 Render "Edit" and "Delete" action buttons
- [ ] 13.5 Implement "Delete" with confirmation dialog → `invoke('delete_project', { id })` → clear selection → refresh list
- [ ] 13.6 Render toolbar with search input and "+ Add Gems" button
- [ ] 13.7 Render gem cards using existing `GemCard` component or simplified version
- [ ] 13.8 Add remove button (×) on each gem card → `invoke('remove_gem_from_project', { projectId, gemId })` → refresh
- [ ] 13.9 Handle gem card click → `onGemSelect(gemId)` to open in right panel
- [ ] 13.10 Implement search filtering with 300ms debounce via `invoke('get_project_gems', { projectId, query })`

### Task 14: Create `AddGemsModal` component

- [ ] 14.1 Render modal overlay when "+ Add Gems" clicked
- [ ] 14.2 Load all gems on mount via `invoke<GemSearchResult[]>('search_gems', { query: '', limit: 100 })`
- [ ] 14.3 Search input with 300ms debounce
- [ ] 14.4 Show gem list with checkboxes — pre-checked and disabled for already-added gems with "Already added" label
- [ ] 14.5 Track selected gem IDs in `Set<string>`
- [ ] 14.6 "Add Selected ({count})" button → `invoke('add_gems_to_project', { projectId, gemIds })` → close modal → refresh
- [ ] 14.7 "Add Selected" disabled when no new gems selected
- [ ] 14.8 Close on Cancel, click outside, or Escape key

**Checkpoint**: Frontend builds. Full project-gem management flow works: create project, add gems, remove gems, delete project.

---

## Phase 7: Frontend — Edit Project and Add-to-Project from GemCard (Requirements 11, 13)

### Task 15: Implement inline project editing

- [ ] 15.1 Toggle edit mode on "Edit" button click in project metadata header
- [ ] 15.2 Show editable fields: title, description, objective, status dropdown (active/paused/completed/archived)
- [ ] 15.3 "Save" button → `invoke('update_project', { id, title, description, objective, status })` → exit edit mode → refresh
- [ ] 15.4 "Cancel" button reverts to display mode

### Task 16: Add "Add to Project" dropdown on GemCard in GemsPanel

- [ ] 16.1 Add `📁+` icon button to `GemCard` action bar in `GemsPanel.tsx`
- [ ] 16.2 On click, show dropdown listing all projects via `invoke<ProjectPreview[]>('list_projects')`
- [ ] 16.3 Pre-check projects that already contain this gem via `invoke<ProjectPreview[]>('get_gem_projects', { gemId })`
- [ ] 16.4 Check a project → `invoke('add_gems_to_project', { projectId, gemIds: [gemId] })`
- [ ] 16.5 Uncheck a project → `invoke('remove_gem_from_project', { projectId, gemId })`
- [ ] 16.6 Close dropdown on click outside

**Checkpoint**: Full feature set working. Users can manage project membership from both the project view and the gem card.

---

## Phase 8: CSS Styling (Requirement 14)

### Task 17: Add CSS for ProjectsContainer and ProjectList

- [ ] 17.1 Add `.projects-container` styles: `display: flex`, full height
- [ ] 17.2 Add `.project-list` styles: `width: 260px`, `flex-shrink: 0`, border-right, `overflow-y: auto`
- [ ] 17.3 Add `.project-list-header` styles: flex, justify-content, padding, border-bottom
- [ ] 17.4 Add `.project-card` styles: padding, border-radius, cursor pointer, hover effect, `.active` state with accent border
- [ ] 17.5 Add `.project-card-title`, `.project-card-meta`, `.project-card-desc` styles

### Task 18: Add CSS for status badges and project gem list

- [ ] 18.1 Add `.status-badge` base styles: inline-block, padding, border-radius, font-size, text-transform
- [ ] 18.2 Add status color variants: `.status-active` (green), `.status-paused` (yellow), `.status-completed` (blue), `.status-archived` (gray)
- [ ] 18.3 Add `.project-gem-list` styles: flex: 1, overflow-y, flex-direction column
- [ ] 18.4 Add `.project-gem-list.empty-state` styles: centered text
- [ ] 18.5 Add `.project-metadata-header` styles: padding, border-bottom, heading sizes
- [ ] 18.6 Add `.project-gem-toolbar` styles: flex, gap, padding, border-bottom
- [ ] 18.7 Add `.project-gem-card` and `.remove-from-project` styles: positioned overlay, hover reveal

### Task 19: Add CSS for CreateProjectForm and AddGemsModal

- [ ] 19.1 Add `.create-project-form` styles: padding, border, background, input/textarea styling
- [ ] 19.2 Add `.modal-overlay` styles: fixed, inset 0, dark background, centered flex
- [ ] 19.3 Add `.modal-card` styles: background, border, border-radius, max-height, flex column
- [ ] 19.4 Add `.modal-header`, `.modal-search`, `.modal-gem-list`, `.modal-gem-row`, `.modal-footer` styles
- [ ] 19.5 Add `.modal-gem-row.selected`, `.modal-gem-row.disabled`, `.already-added-label` styles

**Checkpoint**: All UI styled consistently with dark theme. Layout responsive with fixed project list + flex gem list.

---

## Phase 9: Testing and Polish

### Task 20: Verify backend — SqliteProjectStore

- [ ] 20.1 Test `create`: UUID generated, status="active", timestamps set
- [ ] 20.2 Test `list`: ordering by `updated_at DESC`, gem_count accuracy
- [ ] 20.3 Test `get`: project + gems returned, "Project not found" on invalid id
- [ ] 20.4 Test `update`: partial updates (only `Some` fields changed), timestamp updated
- [ ] 20.5 Test `delete`: project removed, CASCADE removes associations, gems untouched
- [ ] 20.6 Test `add_gems`: idempotency via `INSERT OR IGNORE`, count accuracy
- [ ] 20.7 Test `remove_gem`: association removed, gem still exists
- [ ] 20.8 Test `get_project_gems`: ordering by `added_at DESC`, search filtering via FTS5
- [ ] 20.9 Test `get_gem_projects`: reverse lookup returns correct projects

### Task 21: Verify edge cases

- [ ] 21.1 No projects exist → ProjectList shows "No projects yet"
- [ ] 21.2 Project with 0 gems → ProjectGemList shows empty state message
- [ ] 21.3 Gem deleted from DB → CASCADE removes from project, next load reflects change
- [ ] 21.4 Same gem added twice → `INSERT OR IGNORE` silently skips, count returns 0
- [ ] 21.5 Project deleted while selected → selection cleared, list refreshed
- [ ] 21.6 100+ gems in project → scrollable gem list works
- [ ] 21.7 Search within project gems returns 0 → "No gems match" message
- [ ] 21.8 Nav away and back to Projects → state re-initializes correctly

### Task 22: End-to-end verification

- [ ] 22.1 Create project with title only → verify active status, empty gem list
- [ ] 22.2 Create project with all fields → verify description and objective shown
- [ ] 22.3 Select project → verify gems load in right side of split panel
- [ ] 22.4 Click gem in project → verify GemDetailPanel opens in right panel
- [ ] 22.5 Add gems via modal → verify they appear in project gem list
- [ ] 22.6 Remove gem from project → verify it disappears, gem still in GemsPanel
- [ ] 22.7 Delete project → verify gems unaffected, project list updated
- [ ] 22.8 Edit project status → verify badge color changes
- [ ] 22.9 Search within project gems → verify filtering works
- [ ] 22.10 Add gem to project from GemsPanel card → verify dropdown works
- [ ] 22.11 App builds and starts without errors
