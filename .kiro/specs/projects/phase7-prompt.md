# Phase 7 Implementation Prompt — Inline Project Editing + Add-to-Project from GemCard

## What to Implement

Implement **Phase 7** from `.kiro/specs/projects/tasks.md` — Tasks 15 and 16. This phase adds two features:

1. **Inline project editing** in `ProjectGemList` — toggle the metadata header between display and edit mode, with editable title, description, objective, and a status dropdown
2. **"Add to Project" dropdown on GemCard** in `GemsPanel.tsx` — a button on each gem card that opens a dropdown of all projects, allowing users to add/remove a gem from projects directly from the Gems panel

After this phase, users can manage project membership from both the project view (Phase 6) and the gem card.

## Context Files (Read These First)

1. **Current ProjectsContainer.tsx** (modify `ProjectGemList` to add edit mode):
   `jarvis-app/src/components/ProjectsContainer.tsx` — Full file, 623 lines. Key area:
   - Lines 338-577: `ProjectGemList` component
   - Lines 444-463: metadata header (replace with edit/display toggle)
   - Lines 446-453: `project-meta-row` with status badge, gem count, Delete button — add "Edit" button here

2. **Current GemsPanel.tsx** (modify `GemCard` to add project dropdown):
   `jarvis-app/src/components/GemsPanel.tsx` — Full file, 627 lines. Key areas:
   - Lines 1-4: imports (add `ProjectPreview`)
   - Lines 25-36: `GemCard` props (no changes needed — gem.id is available)
   - Lines 38-50: GemCard state (add dropdown state)
   - Lines 291-357: `gem-actions` div — add the project button here, **before** the delete confirmation section (line 335)

3. **Tauri commands available**:
   - `update_project(id: string, title?: string, description?: string, objective?: string, status?: string)` → `Project`
   - `list_projects()` → `ProjectPreview[]`
   - `get_gem_projects(gemId: string)` → `ProjectPreview[]`
   - `add_gems_to_project(projectId: string, gemIds: string[])` → `number`
   - `remove_gem_from_project(projectId: string, gemId: string)` → `()`

4. **Design doc** (reference for data flow):
   `.kiro/specs/projects/design.md` — Section: "Data Flow — Add to Project from GemCard"

5. **Requirements spec**:
   `.kiro/specs/projects/requirements.md` — Requirements 11 (project editing) and 13 (add-to-project from GemCard)

## Tasks

### Task 15: Implement inline project editing in `ProjectGemList`

Modify the `ProjectGemList` component in `jarvis-app/src/components/ProjectsContainer.tsx`.

#### 15a. Add edit state

Add these state variables to `ProjectGemList` (after the existing state declarations around line 354):

```typescript
const [editing, setEditing] = useState(false);
const [editTitle, setEditTitle] = useState('');
const [editDescription, setEditDescription] = useState('');
const [editObjective, setEditObjective] = useState('');
const [editStatus, setEditStatus] = useState('');
const [saving, setSaving] = useState(false);
const [editError, setEditError] = useState<string | null>(null);
```

#### 15b. Add edit handlers

Add these functions after the existing handlers (after `handleDeleteProject`):

```typescript
const startEditing = () => {
  if (!detail) return;
  setEditTitle(detail.project.title);
  setEditDescription(detail.project.description || '');
  setEditObjective(detail.project.objective || '');
  setEditStatus(detail.project.status);
  setEditError(null);
  setEditing(true);
};

const cancelEditing = () => {
  setEditing(false);
  setEditError(null);
};

const handleSave = async () => {
  if (!projectId || !editTitle.trim()) return;

  setSaving(true);
  setEditError(null);

  try {
    await invoke<Project>('update_project', {
      id: projectId,
      title: editTitle.trim(),
      description: editDescription.trim() || null,
      objective: editObjective.trim() || null,
      status: editStatus,
    });
    setEditing(false);
    loadProject(projectId);
    onProjectsChanged(); // refresh project list (title/status may have changed)
  } catch (err) {
    setEditError(String(err));
  } finally {
    setSaving(false);
  }
};
```

#### 15c. Update the metadata header to toggle between display and edit mode

**Replace** the current metadata header section (lines 444-463 approximately):

```tsx
{/* Project metadata header */}
<div className="project-metadata-header">
  <h2>{detail.project.title}</h2>
  <div className="project-meta-row">
    <span className={`status-badge status-${detail.project.status}`}>
      {detail.project.status}
    </span>
    <span>{detail.gem_count} gems</span>
    <button className="action-button small danger" onClick={() => setConfirmDelete(true)}>
      Delete
    </button>
  </div>
  {detail.project.objective && (
    <div className="project-objective">{detail.project.objective}</div>
  )}
  {detail.project.description && (
    <div className="project-description" style={{ marginTop: '4px', fontSize: '13px', color: 'var(--text-secondary, #aaa)' }}>
      {detail.project.description}
    </div>
  )}
</div>
```

**With this display/edit toggle:**

```tsx
{/* Project metadata header */}
<div className="project-metadata-header">
  {editing ? (
    /* Edit mode */
    <div className="project-edit-form">
      <input
        type="text"
        value={editTitle}
        onChange={(e) => setEditTitle(e.target.value)}
        placeholder="Project title (required)"
        style={{
          width: '100%', padding: '8px', marginBottom: '8px',
          border: '1px solid var(--border-color, #444)', borderRadius: '4px',
          background: 'var(--input-bg, #1a1a2e)', color: 'var(--text-primary, #e0e0e0)',
          fontSize: '16px', fontWeight: 600,
        }}
        autoFocus
        disabled={saving}
      />
      <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '8px' }}>
        <label style={{ fontSize: '12px', color: 'var(--text-muted, #888)' }}>Status:</label>
        <select
          value={editStatus}
          onChange={(e) => setEditStatus(e.target.value)}
          style={{
            padding: '4px 8px',
            border: '1px solid var(--border-color, #444)', borderRadius: '4px',
            background: 'var(--input-bg, #1a1a2e)', color: 'var(--text-primary, #e0e0e0)',
            fontSize: '13px',
          }}
          disabled={saving}
        >
          <option value="active">Active</option>
          <option value="paused">Paused</option>
          <option value="completed">Completed</option>
          <option value="archived">Archived</option>
        </select>
      </div>
      <textarea
        value={editDescription}
        onChange={(e) => setEditDescription(e.target.value)}
        placeholder="Description (optional)"
        rows={2}
        style={{
          width: '100%', padding: '8px', marginBottom: '8px',
          border: '1px solid var(--border-color, #444)', borderRadius: '4px',
          background: 'var(--input-bg, #1a1a2e)', color: 'var(--text-primary, #e0e0e0)',
          fontSize: '13px', fontFamily: 'inherit', resize: 'vertical',
        }}
        disabled={saving}
      />
      <textarea
        value={editObjective}
        onChange={(e) => setEditObjective(e.target.value)}
        placeholder="Objective (optional)"
        rows={2}
        style={{
          width: '100%', padding: '8px', marginBottom: '8px',
          border: '1px solid var(--border-color, #444)', borderRadius: '4px',
          background: 'var(--input-bg, #1a1a2e)', color: 'var(--text-primary, #e0e0e0)',
          fontSize: '13px', fontFamily: 'inherit', resize: 'vertical',
        }}
        disabled={saving}
      />
      {editError && (
        <div className="error-state" style={{ marginBottom: '8px', fontSize: '12px' }}>
          {editError}
        </div>
      )}
      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px' }}>
        <button
          className="action-button secondary"
          onClick={cancelEditing}
          disabled={saving}
        >
          Cancel
        </button>
        <button
          className="action-button"
          onClick={handleSave}
          disabled={!editTitle.trim() || saving}
        >
          {saving ? 'Saving...' : 'Save'}
        </button>
      </div>
    </div>
  ) : (
    /* Display mode */
    <>
      <h2>{detail.project.title}</h2>
      <div className="project-meta-row">
        <span className={`status-badge status-${detail.project.status}`}>
          {detail.project.status}
        </span>
        <span>{detail.gem_count} gems</span>
        <button className="action-button small" onClick={startEditing}>
          Edit
        </button>
        <button className="action-button small danger" onClick={() => setConfirmDelete(true)}>
          Delete
        </button>
      </div>
      {detail.project.objective && (
        <div className="project-objective">{detail.project.objective}</div>
      )}
      {detail.project.description && (
        <div className="project-description" style={{ marginTop: '4px', fontSize: '13px', color: 'var(--text-secondary, #aaa)' }}>
          {detail.project.description}
        </div>
      )}
    </>
  )}
</div>
```

**Key behaviors:**
- "Edit" button initializes edit fields from current `detail.project` values, then shows edit form
- Status dropdown with 4 options: active, paused, completed, archived
- Title input is required — "Save" disabled when empty
- "Save" calls `update_project` with all fields, then exits edit mode and refreshes
- "Cancel" reverts to display mode without saving
- `onProjectsChanged()` is called after save to refresh the project list (title/status may have changed)
- Inputs disabled while saving to prevent double-submit
- Error shown inline if save fails

---

### Task 16: Add "Add to Project" dropdown on GemCard in `GemsPanel.tsx`

Modify the `GemCard` component in `jarvis-app/src/components/GemsPanel.tsx`.

#### 16a. Add import

Update the type import (line 4):

**Change:**
```typescript
import type { GemPreview, GemSearchResult, Gem, AvailabilityResult } from '../state/types';
```

**To:**
```typescript
import type { GemPreview, GemSearchResult, Gem, AvailabilityResult, ProjectPreview } from '../state/types';
```

#### 16b. Add state to GemCard

Add these state variables inside the `GemCard` function (after the existing state declarations, around line 50):

```typescript
const [showProjectDropdown, setShowProjectDropdown] = useState(false);
const [allProjects, setAllProjects] = useState<ProjectPreview[]>([]);
const [gemProjects, setGemProjects] = useState<Set<string>>(new Set());
const [projectsLoading, setProjectsLoading] = useState(false);
```

#### 16c. Add handlers for the project dropdown

Add these functions inside `GemCard` (after the existing handlers, before the `return` statement):

```typescript
const handleToggleProjectDropdown = async () => {
  if (showProjectDropdown) {
    setShowProjectDropdown(false);
    return;
  }

  setProjectsLoading(true);
  try {
    // Fetch all projects and gem's current projects in parallel
    const [projects, currentProjects] = await Promise.all([
      invoke<ProjectPreview[]>('list_projects'),
      invoke<ProjectPreview[]>('get_gem_projects', { gemId: gem.id }),
    ]);
    setAllProjects(projects);
    setGemProjects(new Set(currentProjects.map(p => p.id)));
    setShowProjectDropdown(true);
  } catch (err) {
    console.error('Failed to load projects:', err);
  } finally {
    setProjectsLoading(false);
  }
};

const handleToggleProject = async (projectId: string) => {
  const isInProject = gemProjects.has(projectId);
  try {
    if (isInProject) {
      await invoke('remove_gem_from_project', { projectId, gemId: gem.id });
      setGemProjects(prev => {
        const next = new Set(prev);
        next.delete(projectId);
        return next;
      });
    } else {
      await invoke('add_gems_to_project', { projectId, gemIds: [gem.id] });
      setGemProjects(prev => {
        const next = new Set(prev);
        next.add(projectId);
        return next;
      });
    }
  } catch (err) {
    console.error('Failed to toggle project membership:', err);
  }
};
```

#### 16d. Add click-outside handler to close the dropdown

Add this effect inside `GemCard` (after the other useEffects):

```typescript
// Close project dropdown on click outside
useEffect(() => {
  if (!showProjectDropdown) return;
  const handleClickOutside = (e: MouseEvent) => {
    const target = e.target as HTMLElement;
    if (!target.closest('.project-dropdown-container')) {
      setShowProjectDropdown(false);
    }
  };
  document.addEventListener('mousedown', handleClickOutside);
  return () => document.removeEventListener('mousedown', handleClickOutside);
}, [showProjectDropdown]);
```

#### 16e. Add the button and dropdown to the gem-actions bar

In the `gem-actions` div (line 291), add the project button **before** the delete confirmation section (before line 335). Insert it after the Transcribe button block (after line 334):

```tsx
{/* Add to Project dropdown */}
<div className="project-dropdown-container" style={{ position: 'relative', display: 'inline-block' }}>
  <button
    onClick={handleToggleProjectDropdown}
    className="gem-enrich-button"
    disabled={projectsLoading}
    title="Add to Project"
  >
    {projectsLoading ? '...' : '📁+'}
  </button>
  {showProjectDropdown && (
    <div className="project-dropdown" style={{
      position: 'absolute',
      bottom: '100%',
      right: 0,
      marginBottom: '4px',
      background: 'var(--panel-bg, #16213e)',
      border: '1px solid var(--border-color, #333)',
      borderRadius: '6px',
      minWidth: '220px',
      maxHeight: '240px',
      overflowY: 'auto',
      zIndex: 100,
      boxShadow: '0 4px 12px rgba(0, 0, 0, 0.3)',
    }}>
      <div style={{
        padding: '8px 12px',
        borderBottom: '1px solid var(--border-color, #333)',
        fontSize: '12px',
        fontWeight: 600,
        color: 'var(--text-muted, #888)',
      }}>
        Projects
      </div>
      {allProjects.length === 0 && (
        <div style={{ padding: '12px', fontSize: '12px', color: 'var(--text-muted, #666)' }}>
          No projects yet
        </div>
      )}
      {allProjects.map(project => (
        <div
          key={project.id}
          onClick={(e) => { e.stopPropagation(); handleToggleProject(project.id); }}
          style={{
            display: 'flex', alignItems: 'center', gap: '8px',
            padding: '8px 12px', cursor: 'pointer',
            fontSize: '13px',
          }}
        >
          <input
            type="checkbox"
            checked={gemProjects.has(project.id)}
            readOnly
            style={{ pointerEvents: 'none' }}
          />
          <span style={{ flex: 1 }}>{project.title}</span>
          <span style={{ fontSize: '11px', color: 'var(--text-muted, #888)' }}>
            {project.gem_count} gems
          </span>
        </div>
      ))}
    </div>
  )}
</div>
```

**Key behaviors:**
- Clicking `📁+` fetches all projects AND the gem's current projects in parallel (`Promise.all`)
- Dropdown shows all projects with checkboxes — pre-checked if gem is already in that project
- Checking a project → `add_gems_to_project` with `gemIds: [gem.id]`
- Unchecking a project → `remove_gem_from_project` with `gemId: gem.id`
- State updates are optimistic (update `gemProjects` Set immediately)
- Dropdown closes on click outside via `mousedown` listener
- Clicking the button again closes the dropdown (toggle behavior)
- "No projects yet" shown if no projects exist
- Dropdown opens **above** the button (`bottom: '100%'`) to avoid overflow issues at the bottom of the page
- `e.stopPropagation()` on project rows prevents triggering `onSelect` on the gem card

#### 16f. Important: `stopPropagation` on the button

The `📁+` button is inside the `gem-actions` div which already has `onClick={(e) => e.stopPropagation()}` (line 291). So clicking the button won't trigger `onSelect`. No additional stopPropagation needed on the button itself.

However, clicking a project row inside the dropdown WILL need `e.stopPropagation()` because the dropdown is rendered inside the gem card. This is already included in the code above.

## Verification

After implementing, verify the frontend builds:
```bash
cd jarvis-app && npm run build
```

Must pass with no errors.

**Things to check manually (if running the app):**

**Task 15 — Inline editing:**
- Select a project → click "Edit" → form appears with current values pre-filled
- Change title → click "Save" → title updates in both header and project list
- Change status to "paused" → click "Save" → status badge updates
- Click "Cancel" → reverts to display mode without changes
- Clear title and try to save → "Save" button is disabled
- Edit shows description and objective fields even if currently empty

**Task 16 — Add to Project from GemCard:**
- In Gems panel, click `📁+` on a gem card → dropdown opens with all projects listed
- Projects the gem already belongs to are pre-checked
- Check a project → gem is added (checkbox toggles immediately)
- Uncheck a project → gem is removed (checkbox toggles immediately)
- Click outside the dropdown → it closes
- Click `📁+` again → dropdown closes (toggle)
- "No projects yet" shows if no projects exist
- Dropdown appears above the button, not below

## What NOT to Do

- Do NOT add CSS to `App.css` — Phase 8 handles styling
- Do NOT modify `App.tsx`
- Do NOT modify `RightPanel.tsx`
- Do NOT create new files — both changes are in existing components
- Do NOT change the `GemCard` props interface — the project dropdown is self-contained within GemCard
- Do NOT add a project refresh callback to GemCard — the dropdown manages its own state. When the user navigates to Projects, it will re-fetch on mount

## After Implementation

Once `npm run build` passes:
1. Show me the changes to `ProjectsContainer.tsx` (the edit mode in `ProjectGemList`)
2. Show me the changes to `GemsPanel.tsx` (the project dropdown in `GemCard`)
3. I'll review before we proceed to Phase 8

**If you have any confusion or questions — about the edit mode toggle pattern, the dropdown positioning (`bottom: '100%'` to open upward), the `Promise.all` for parallel fetching, click-outside handling, or anything else — please ask before guessing.**
