# Phase 5 Implementation Prompt — ProjectsContainer + ProjectList + CreateProjectForm

## What to Implement

Implement **Phase 5** from `.kiro/specs/projects/tasks.md` — Tasks 10, 11, and 12. This creates the three core frontend components for the Projects feature: the split-layout container, the project list with cards, and the inline create form. After this phase, users can create projects, see them in a list, and select them. The right side of the split (ProjectGemList) is a placeholder — that's Phase 6.

## Context Files (Read These First)

1. **Design doc** (has full React code for all components):
   `.kiro/specs/projects/design.md` — Sections: "ProjectsContainer.tsx", "ProjectList.tsx", "ProjectGemList.tsx" (for placeholder structure)

2. **App.tsx** (replace the Phase 4 placeholder with real component):
   `jarvis-app/src/App.tsx` — Key areas:
   - Line 2: imports (add `ProjectsContainer`)
   - Lines 828-832: **Phase 4 placeholder** — replace with `<ProjectsContainer>`
   - Line 485-487: `handleGemSelect` function (passed as `onGemSelect` prop)

3. **TypeScript types** (already have Project, ProjectPreview, ProjectDetail):
   `jarvis-app/src/state/types.ts` — Lines 248-279: `Project`, `ProjectPreview`, `ProjectDetail` interfaces

4. **GemsPanel.tsx** (reference for UI patterns — GemCard, search, invoke usage):
   `jarvis-app/src/components/GemsPanel.tsx` — Shows how to use `invoke()`, state management, loading/error patterns

5. **Requirements spec**:
   `.kiro/specs/projects/requirements.md` — Requirements 8, 9, 10

## Tasks

### Task 10: Create `ProjectsContainer` component

Create `jarvis-app/src/components/ProjectsContainer.tsx` with:

**Props interface:**
```typescript
interface ProjectsContainerProps {
  onGemSelect?: (gemId: string | null) => void;
}
```

**State:**
```typescript
const [projects, setProjects] = useState<ProjectPreview[]>([]);
const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
const [loading, setLoading] = useState(true);
```

**Data fetching:**
```typescript
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
```

**Callback for project creation:**
```typescript
const handleProjectCreated = (projectId: string) => {
  fetchProjects();
  setSelectedProjectId(projectId);
};
```

**Render — split layout:**
```tsx
<div className="projects-container">
  <ProjectList
    projects={projects}
    selectedProjectId={selectedProjectId}
    onSelectProject={setSelectedProjectId}
    onProjectCreated={handleProjectCreated}
    onProjectsChanged={fetchProjects}
    loading={loading}
  />
  {/* ProjectGemList placeholder — Phase 6 will replace this */}
  <div className="project-gem-list empty-state">
    {selectedProjectId
      ? 'Project gem list — coming in Phase 6'
      : 'Select a project to see its gems'}
  </div>
</div>
```

**Imports needed:**
```typescript
import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { ProjectPreview } from '../state/types';
```

**Key layout note:** The `.projects-container` uses `display: flex` with the project list at fixed `260px` width and the right side at `flex: 1`. Don't add CSS yet (Phase 8) — but use inline styles or className references as shown above. The classNames are defined in the design doc and will be styled in Phase 8.

### Task 11: Create `ProjectList` component

Create `ProjectList` as a **named function component inside `ProjectsContainer.tsx`** (not a separate file). This keeps all project components co-located until they grow large enough to warrant extraction.

**Props:**
```typescript
function ProjectList({
  projects,
  selectedProjectId,
  onSelectProject,
  onProjectCreated,
  onProjectsChanged,
  loading,
}: {
  projects: ProjectPreview[];
  selectedProjectId: string | null;
  onSelectProject: (id: string) => void;
  onProjectCreated: (id: string) => void;
  onProjectsChanged: () => void;
  loading: boolean;
})
```

**State:**
```typescript
const [showCreateForm, setShowCreateForm] = useState(false);
```

**Render structure:**
```tsx
<div className="project-list">
  {/* Header with title and new project button */}
  <div className="project-list-header">
    <h3>Projects</h3>
    <button
      className="action-button"
      onClick={() => setShowCreateForm(true)}
    >
      + New Project
    </button>
  </div>

  {/* Inline create form (shown when button clicked) */}
  {showCreateForm && (
    <CreateProjectForm
      onCreated={(id) => { setShowCreateForm(false); onProjectCreated(id); }}
      onCancel={() => setShowCreateForm(false)}
    />
  )}

  {/* Project cards list */}
  <div className="project-list-items">
    {loading && projects.length === 0 && (
      <div className="loading-state" style={{ padding: '16px', textAlign: 'center', color: 'var(--text-muted, #666)' }}>
        Loading projects...
      </div>
    )}

    {!loading && projects.length === 0 && (
      <div className="empty-state" style={{ padding: '16px', textAlign: 'center', color: 'var(--text-muted, #666)' }}>
        No projects yet. Click "+ New Project" to create one.
      </div>
    )}

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
```

**Key behaviors:**
- Project cards show: title, status badge (with CSS class `status-{status}`), gem count, and truncated description
- Clicking a card calls `onSelectProject(id)` — the container updates `selectedProjectId`
- The active card gets the `active` class for visual highlighting
- Empty state shows when `!loading && projects.length === 0`
- Loading state shows when `loading && projects.length === 0`

### Task 12: Create `CreateProjectForm` component

Create `CreateProjectForm` as another **named function component inside `ProjectsContainer.tsx`**.

**Props:**
```typescript
function CreateProjectForm({
  onCreated,
  onCancel,
}: {
  onCreated: (id: string) => void;
  onCancel: () => void;
})
```

**State:**
```typescript
const [title, setTitle] = useState('');
const [description, setDescription] = useState('');
const [objective, setObjective] = useState('');
const [creating, setCreating] = useState(false);
const [error, setError] = useState<string | null>(null);
```

**Submit handler:**
```typescript
const handleSubmit = async (e: React.FormEvent) => {
  e.preventDefault();
  if (!title.trim()) return;

  setCreating(true);
  setError(null);

  try {
    const project = await invoke<Project>('create_project', {
      title: title.trim(),
      description: description.trim() || null,
      objective: objective.trim() || null,
    });
    onCreated(project.id);
  } catch (err) {
    setError(String(err));
    setCreating(false);
  }
};
```

**Render structure:**
```tsx
<form className="create-project-form" onSubmit={handleSubmit}>
  <input
    type="text"
    placeholder="Project title (required)"
    value={title}
    onChange={(e) => setTitle(e.target.value)}
    autoFocus
    disabled={creating}
  />
  <textarea
    placeholder="Description (optional)"
    value={description}
    onChange={(e) => setDescription(e.target.value)}
    rows={2}
    disabled={creating}
  />
  <textarea
    placeholder="Objective (optional)"
    value={objective}
    onChange={(e) => setObjective(e.target.value)}
    rows={2}
    disabled={creating}
  />
  {error && (
    <div className="error-state" style={{ marginBottom: '8px', fontSize: '12px' }}>
      {error}
    </div>
  )}
  <div className="form-actions">
    <button
      type="button"
      className="action-button secondary"
      onClick={onCancel}
      disabled={creating}
    >
      Cancel
    </button>
    <button
      type="submit"
      className="action-button"
      disabled={!title.trim() || creating}
    >
      {creating ? 'Creating...' : 'Create Project'}
    </button>
  </div>
</form>
```

**Key behaviors:**
- Title is required — the "Create Project" button is disabled when title is empty
- Description and objective are optional — sent as `null` if empty
- Auto-focuses the title input when the form appears
- On success: calls `onCreated(project.id)` which closes the form, refreshes the list, and auto-selects the new project
- On error: shows error message inline, keeps form open so user can retry
- "Cancel" button closes the form without any action
- Form inputs are disabled while creating (prevents double-submit)

**Import for Project type:**
```typescript
import type { ProjectPreview, Project } from '../state/types';
```
Note: `Project` is needed in `CreateProjectForm` because `invoke<Project>('create_project', ...)` returns a `Project` (to get the `id`).

### Task 10b: Update `App.tsx` — Replace placeholder with real component

**1. Add import** (after the existing component imports, around line 12):
```typescript
import { ProjectsContainer } from './components/ProjectsContainer';
```

**2. Replace the Phase 4 placeholder** (lines 828-832):

**Remove:**
```tsx
{activeNav === 'projects' && (
  <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--text-muted, #666)' }}>
    Projects — coming in Phase 5
  </div>
)}
```

**Replace with:**
```tsx
{activeNav === 'projects' && (
  <ProjectsContainer onGemSelect={handleGemSelect} />
)}
```

## Complete File — `ProjectsContainer.tsx`

For reference, here is what the complete file should look like after implementing Tasks 10, 11, and 12:

```tsx
import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { ProjectPreview, Project } from '../state/types';

interface ProjectsContainerProps {
  onGemSelect?: (gemId: string | null) => void;
}

function CreateProjectForm({
  onCreated,
  onCancel,
}: {
  onCreated: (id: string) => void;
  onCancel: () => void;
}) {
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [objective, setObjective] = useState('');
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim()) return;

    setCreating(true);
    setError(null);

    try {
      const project = await invoke<Project>('create_project', {
        title: title.trim(),
        description: description.trim() || null,
        objective: objective.trim() || null,
      });
      onCreated(project.id);
    } catch (err) {
      setError(String(err));
      setCreating(false);
    }
  };

  return (
    <form className="create-project-form" onSubmit={handleSubmit}>
      <input
        type="text"
        placeholder="Project title (required)"
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        autoFocus
        disabled={creating}
      />
      <textarea
        placeholder="Description (optional)"
        value={description}
        onChange={(e) => setDescription(e.target.value)}
        rows={2}
        disabled={creating}
      />
      <textarea
        placeholder="Objective (optional)"
        value={objective}
        onChange={(e) => setObjective(e.target.value)}
        rows={2}
        disabled={creating}
      />
      {error && (
        <div className="error-state" style={{ marginBottom: '8px', fontSize: '12px' }}>
          {error}
        </div>
      )}
      <div className="form-actions">
        <button
          type="button"
          className="action-button secondary"
          onClick={onCancel}
          disabled={creating}
        >
          Cancel
        </button>
        <button
          type="submit"
          className="action-button"
          disabled={!title.trim() || creating}
        >
          {creating ? 'Creating...' : 'Create Project'}
        </button>
      </div>
    </form>
  );
}

function ProjectList({
  projects,
  selectedProjectId,
  onSelectProject,
  onProjectCreated,
  onProjectsChanged,
  loading,
}: {
  projects: ProjectPreview[];
  selectedProjectId: string | null;
  onSelectProject: (id: string) => void;
  onProjectCreated: (id: string) => void;
  onProjectsChanged: () => void;
  loading: boolean;
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
        {loading && projects.length === 0 && (
          <div className="loading-state" style={{ padding: '16px', textAlign: 'center', color: 'var(--text-muted, #666)' }}>
            Loading projects...
          </div>
        )}

        {!loading && projects.length === 0 && (
          <div className="empty-state" style={{ padding: '16px', textAlign: 'center', color: 'var(--text-muted, #666)' }}>
            No projects yet. Click "+ New Project" to create one.
          </div>
        )}

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
        loading={loading}
      />
      <div className="project-gem-list empty-state">
        {selectedProjectId
          ? 'Project gem list — coming in Phase 6'
          : 'Select a project to see its gems'}
      </div>
    </div>
  );
}
```

## Verification

After implementing, verify the frontend builds:
```bash
cd jarvis-app && npm run build
```

Must pass with no errors.

**Things to check manually (if running the app):**
- Click "Projects" in left nav — split layout appears with ProjectList on the left
- Click "+ New Project" — inline form appears with title, description, objective fields
- Enter a title and click "Create Project" — project appears in list, auto-selected
- Click a project card — it gets highlighted with `.active` class
- Right side shows placeholder text ("Project gem list — coming in Phase 6" when project selected)
- Empty state shows "No projects yet" when no projects exist
- Creating a project with only a title works (description and objective are optional)
- "Cancel" on the create form closes it

## What NOT to Do

- Do NOT create `ProjectGemList` as a real component — that's Phase 6. Use the placeholder `<div>` shown above
- Do NOT create `AddGemsModal` — that's Phase 6
- Do NOT add CSS to `App.css` — that's Phase 8. Use the className references (they'll look unstyled but functional)
- Do NOT modify `RightPanel.tsx` — the right panel already works for projects (same `selectedGemId` flow)
- Do NOT add project editing or deletion UI — that's Phase 7
- Do NOT modify `GemsPanel.tsx` — the "Add to Project" dropdown from GemCard is Phase 7
- Do NOT create separate files for `ProjectList` and `CreateProjectForm` — keep them all in `ProjectsContainer.tsx` for now

## After Implementation

Once `npm run build` passes:
1. Show me the new `ProjectsContainer.tsx` file and the changes to `App.tsx`
2. I'll review before we proceed to Phase 6

**If you have any confusion or questions — about the placeholder approach for ProjectGemList, the component structure (all in one file), the invoke command names matching the Tauri backend, or anything else — please ask before guessing.**
