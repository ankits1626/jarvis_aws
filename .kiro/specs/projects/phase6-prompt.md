# Phase 6 Implementation Prompt — ProjectGemList + AddGemsModal

## What to Implement

Implement **Phase 6** from `.kiro/specs/projects/tasks.md` — Tasks 13 and 14. This replaces the placeholder in `ProjectsContainer.tsx` with a real `ProjectGemList` component that shows project metadata, gem cards with remove buttons, search filtering, and an `AddGemsModal` for picking gems to add. After this phase, the full project-gem management flow works: create project, add gems, view gems, remove gems, delete project.

## Context Files (Read These First)

1. **Design doc** (has full React code for ProjectGemList and AddGemsModal):
   `.kiro/specs/projects/design.md` — Sections: "ProjectGemList.tsx — Right side of split" and "AddGemsModal.tsx — Gem Picker"

2. **Current ProjectsContainer.tsx** (you'll modify this file):
   `jarvis-app/src/components/ProjectsContainer.tsx` — Full file, 200 lines. The placeholder to replace is lines 192-196:
   ```tsx
   <div className="project-gem-list empty-state">
     {selectedProjectId
       ? 'Project gem list — coming in Phase 6'
       : 'Select a project to see its gems'}
   </div>
   ```

3. **TypeScript types** (ProjectDetail, GemPreview, GemSearchResult):
   `jarvis-app/src/state/types.ts` — Key types:
   - `ProjectDetail` (lines 276-279): `{ project: Project, gem_count: number, gems: GemPreview[] }`
   - `GemPreview` (lines 559-598): has `id`, `source_type`, `source_url`, `domain`, `title`, `author`, `description`, `captured_at`, `tags`, `summary`, `enrichment_source`, `content_preview`, `transcript_language`
   - `GemSearchResult` (lines 604-643): extends GemPreview-like fields with `score`, `matched_chunk`, `match_type`, `domain`

4. **Tauri commands available** (from Phase 3):
   - `get_project(id: string)` → `ProjectDetail`
   - `remove_gem_from_project(projectId: string, gemId: string)` → `()`
   - `get_project_gems(projectId: string, query?: string, limit?: number)` → `GemPreview[]`
   - `add_gems_to_project(projectId: string, gemIds: string[])` → `number`
   - `delete_project(id: string)` → `()`
   - `search_gems(query: string, limit: number)` → `GemSearchResult[]`

5. **GemsPanel.tsx** (reference for GemCard UI patterns):
   `jarvis-app/src/components/GemsPanel.tsx` — Shows existing `GemCard` component structure, source badge classes, action button patterns

6. **Requirements spec**:
   `.kiro/specs/projects/requirements.md` — Requirements 11 (project gem list) and 12 (add gems modal)

## Tasks

### Task 13: Create `ProjectGemList` component

Add `ProjectGemList` as a **named function component inside `ProjectsContainer.tsx`** (same file as the other components). Then replace the placeholder `<div>` in `ProjectsContainer`'s render with the real `<ProjectGemList>`.

**Props:**
```typescript
function ProjectGemList({
  projectId,
  onGemSelect,
  onProjectsChanged,
  onProjectDeleted,
}: {
  projectId: string | null;
  onGemSelect?: (gemId: string | null) => void;
  onProjectsChanged: () => void;
  onProjectDeleted: () => void;
})
```

**State:**
```typescript
const [detail, setDetail] = useState<ProjectDetail | null>(null);
const [loading, setLoading] = useState(false);
const [showAddModal, setShowAddModal] = useState(false);
const [searchQuery, setSearchQuery] = useState('');
const [searchResults, setSearchResults] = useState<GemPreview[] | null>(null);
const [confirmDelete, setConfirmDelete] = useState(false);
```

**Data loading — on projectId change:**
```typescript
useEffect(() => {
  if (!projectId) { setDetail(null); setSearchQuery(''); setSearchResults(null); return; }
  loadProject(projectId);
}, [projectId]);

const loadProject = async (id: string) => {
  setLoading(true);
  try {
    const result = await invoke<ProjectDetail>('get_project', { id });
    setDetail(result);
  } catch (err) {
    console.error('Failed to load project:', err);
  } finally {
    setLoading(false);
  }
};
```

**Search with 300ms debounce:**
```typescript
useEffect(() => {
  if (!projectId || !searchQuery.trim()) {
    setSearchResults(null);
    return;
  }
  const timer = setTimeout(async () => {
    try {
      const results = await invoke<GemPreview[]>('get_project_gems', {
        projectId,
        query: searchQuery.trim(),
        limit: 100,
      });
      setSearchResults(results);
    } catch (err) {
      console.error('Failed to search project gems:', err);
    }
  }, 300);
  return () => clearTimeout(timer);
}, [searchQuery, projectId]);
```

**Remove gem handler:**
```typescript
const handleRemoveGem = async (gemId: string) => {
  if (!projectId) return;
  try {
    await invoke('remove_gem_from_project', { projectId, gemId });
    loadProject(projectId);
    onProjectsChanged();
  } catch (err) {
    console.error('Failed to remove gem:', err);
  }
};
```

**Delete project handler:**
```typescript
const handleDeleteProject = async () => {
  if (!projectId) return;
  try {
    await invoke('delete_project', { id: projectId });
    setConfirmDelete(false);
    onProjectDeleted();
  } catch (err) {
    console.error('Failed to delete project:', err);
  }
};
```

**Render — three states:**

**State 1: No project selected:**
```tsx
if (!projectId) {
  return (
    <div className="project-gem-list empty-state">
      Select a project to see its gems
    </div>
  );
}
```

**State 2: Loading:**
```tsx
if (loading && !detail) {
  return (
    <div className="project-gem-list" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--text-muted, #666)' }}>
      Loading...
    </div>
  );
}
```

**State 3: Project loaded — full UI:**
```tsx
if (!detail) return null;

const displayGems = searchResults !== null ? searchResults : detail.gems;

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

    {/* Delete confirmation */}
    {confirmDelete && (
      <div className="delete-confirm-bar" style={{
        padding: '12px 16px',
        backgroundColor: 'rgba(239, 68, 68, 0.1)',
        borderBottom: '1px solid var(--border-color, #333)',
        display: 'flex',
        alignItems: 'center',
        gap: '12px',
        fontSize: '13px',
      }}>
        <span>Delete "{detail.project.title}"? This cannot be undone. Gems will not be deleted.</span>
        <button className="action-button small danger" onClick={handleDeleteProject}>
          Confirm Delete
        </button>
        <button className="action-button small" onClick={() => setConfirmDelete(false)}>
          Cancel
        </button>
      </div>
    )}

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
    <div className="project-gems-list" style={{ flex: 1, overflowY: 'auto', padding: '8px 16px' }}>
      {displayGems.length === 0 && (
        <div className="empty-state" style={{ padding: '32px 16px', textAlign: 'center', color: 'var(--text-muted, #666)' }}>
          {searchQuery.trim()
            ? 'No gems match your search.'
            : 'No gems in this project. Click "+ Add Gems" to get started.'}
        </div>
      )}
      {displayGems.map(gem => (
        <div key={gem.id} className="project-gem-card" style={{ position: 'relative', marginBottom: '8px' }}>
          <div
            className="gem-card"
            onClick={() => onGemSelect?.(gem.id)}
            style={{ cursor: 'pointer' }}
          >
            <div className="gem-card-header">
              <span className={`source-badge ${gem.source_type.toLowerCase()}`}>
                {gem.source_type}
              </span>
              <span className="gem-date">
                {new Date(gem.captured_at).toLocaleDateString()}
              </span>
            </div>
            <div className="gem-title">{gem.title}</div>
            {gem.author && (
              <div className="gem-meta">
                <span className="gem-author">by {gem.author}</span>
              </div>
            )}
            {gem.description && (
              <div className="gem-description">{gem.description}</div>
            )}
            {gem.tags && gem.tags.length > 0 && (
              <div className="gem-tags">
                {gem.tags.map((tag, idx) => (
                  <span key={idx} className="gem-tag">{tag}</span>
                ))}
              </div>
            )}
          </div>
          <button
            className="remove-from-project"
            onClick={(e) => { e.stopPropagation(); handleRemoveGem(gem.id); }}
            title="Remove from project"
            style={{
              position: 'absolute', top: '8px', right: '8px',
              background: 'none', border: 'none',
              color: 'var(--text-muted, #666)', fontSize: '18px',
              cursor: 'pointer', opacity: 0.5,
            }}
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
        onAdded={() => {
          setShowAddModal(false);
          loadProject(projectId);
          onProjectsChanged();
        }}
      />
    )}
  </div>
);
```

**Key behaviors:**
- Loading project detail on `projectId` change
- Search filtering with 300ms debounce using `get_project_gems` command
- When `searchResults` is `null`, display `detail.gems`; when searching, display `searchResults`
- Remove gem button (×) on each card with `stopPropagation` to prevent gem selection
- Gem card click triggers `onGemSelect(gemId)` to show detail in right panel
- Delete project with inline confirmation bar (not a modal)
- Empty states: "No gems in this project" (no gems) and "No gems match your search" (search returned 0)
- **No "Edit" button** — that's Phase 7. Only "Delete" in the metadata header

### Task 14: Create `AddGemsModal` component

Add `AddGemsModal` as another **named function component inside `ProjectsContainer.tsx`**.

**Props:**
```typescript
function AddGemsModal({
  projectId,
  projectTitle,
  existingGemIds,
  onClose,
  onAdded,
}: {
  projectId: string;
  projectTitle: string;
  existingGemIds: string[];
  onClose: () => void;
  onAdded: () => void;
})
```

**State:**
```typescript
const [gems, setGems] = useState<GemSearchResult[]>([]);
const [searchQuery, setSearchQuery] = useState('');
const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
const [loading, setLoading] = useState(true);
const [adding, setAdding] = useState(false);
```

**Data loading — on mount + debounced search:**
```typescript
// Load gems on mount
useEffect(() => {
  loadGems('');
}, []);

// Debounced search (300ms)
useEffect(() => {
  if (!searchQuery.trim()) return; // Skip debounce for initial empty load
  const timer = setTimeout(() => loadGems(searchQuery), 300);
  return () => clearTimeout(timer);
}, [searchQuery]);

const loadGems = async (query: string) => {
  setLoading(true);
  try {
    const results = await invoke<GemSearchResult[]>('search_gems', {
      query: query.trim(),
      limit: 100,
    });
    setGems(results);
  } catch (err) {
    console.error('Failed to load gems:', err);
  } finally {
    setLoading(false);
  }
};
```

**Note on search behavior:** When the user clears the search input, immediately reload all gems (don't wait for debounce):
```typescript
const handleSearchChange = (e: React.ChangeEvent<HTMLInputElement>) => {
  const value = e.target.value;
  setSearchQuery(value);
  if (!value.trim()) {
    loadGems(''); // Immediately reload all gems when input cleared
  }
};
```

**Toggle gem selection:**
```typescript
const toggleGem = (gemId: string) => {
  setSelectedIds(prev => {
    const next = new Set(prev);
    if (next.has(gemId)) next.delete(gemId);
    else next.add(gemId);
    return next;
  });
};
```

**Add handler:**
```typescript
const handleAdd = async () => {
  setAdding(true);
  try {
    const gemIds = Array.from(selectedIds);
    await invoke('add_gems_to_project', { projectId, gemIds });
    onAdded();
  } catch (err) {
    console.error('Failed to add gems:', err);
    setAdding(false);
  }
};
```

**Close on Escape key:**
```typescript
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') onClose();
  };
  window.addEventListener('keydown', handleKeyDown);
  return () => window.removeEventListener('keydown', handleKeyDown);
}, [onClose]);
```

**Render:**
```tsx
return (
  <div className="modal-overlay" onClick={onClose}>
    <div className="modal-card" onClick={(e) => e.stopPropagation()} style={{
      background: 'var(--panel-bg, #16213e)',
      border: '1px solid var(--border-color, #333)',
      borderRadius: '8px',
      width: '560px',
      maxHeight: '70vh',
      display: 'flex',
      flexDirection: 'column',
    }}>
      <div className="modal-header" style={{
        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
        padding: '16px', borderBottom: '1px solid var(--border-color, #333)',
      }}>
        <h3 style={{ margin: 0 }}>Add Gems to {projectTitle}</h3>
        <button className="close-button" onClick={onClose}>×</button>
      </div>
      <div className="modal-search" style={{ padding: '12px 16px' }}>
        <input
          type="search"
          placeholder="Search gems..."
          value={searchQuery}
          onChange={handleSearchChange}
          className="gems-search-input"
          autoFocus
        />
      </div>
      <div className="modal-gem-list" style={{ flex: 1, overflowY: 'auto', padding: '0 16px' }}>
        {loading && gems.length === 0 && (
          <div style={{ padding: '16px', textAlign: 'center', color: 'var(--text-muted, #666)' }}>
            Loading gems...
          </div>
        )}
        {!loading && gems.length === 0 && (
          <div style={{ padding: '16px', textAlign: 'center', color: 'var(--text-muted, #666)' }}>
            {searchQuery.trim() ? 'No gems match your search.' : 'No gems available.'}
          </div>
        )}
        {gems.map(gem => {
          const alreadyAdded = existingGemIds.includes(gem.id);
          const isSelected = selectedIds.has(gem.id);
          return (
            <div
              key={gem.id}
              className={`modal-gem-row ${isSelected ? 'selected' : ''} ${alreadyAdded ? 'disabled' : ''}`}
              onClick={() => !alreadyAdded && toggleGem(gem.id)}
              style={{
                display: 'flex', alignItems: 'center', gap: '12px',
                padding: '10px 8px', borderRadius: '4px', cursor: alreadyAdded ? 'default' : 'pointer',
                opacity: alreadyAdded ? 0.5 : 1,
              }}
            >
              <input
                type="checkbox"
                checked={alreadyAdded || isSelected}
                disabled={alreadyAdded}
                readOnly
              />
              <div style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
                <span style={{ fontSize: '13px' }}>{gem.title}</span>
                <span style={{ fontSize: '11px', color: 'var(--text-muted, #888)' }}>
                  {gem.source_type} · {gem.domain}
                </span>
              </div>
              {alreadyAdded && (
                <span style={{ fontSize: '11px', color: 'var(--text-muted, #666)', fontStyle: 'italic' }}>
                  Already added
                </span>
              )}
            </div>
          );
        })}
      </div>
      <div className="modal-footer" style={{
        display: 'flex', justifyContent: 'flex-end', gap: '8px',
        padding: '12px 16px', borderTop: '1px solid var(--border-color, #333)',
      }}>
        <button className="action-button secondary" onClick={onClose}>
          Cancel
        </button>
        <button
          className="action-button"
          onClick={handleAdd}
          disabled={selectedIds.size === 0 || adding}
        >
          {adding ? 'Adding...' : `Add Selected (${selectedIds.size})`}
        </button>
      </div>
    </div>
  </div>
);
```

**Key behaviors:**
- Modal overlay with click-outside-to-close and Escape key support
- Auto-focus on search input
- Loads all gems on mount via `search_gems` with empty query
- 300ms debounced search — but immediate reload when input is cleared
- Checkboxes: already-added gems are pre-checked and disabled with "Already added" label
- Selection tracked in `Set<string>` — allows multi-select
- "Add Selected (N)" button disabled when no new gems selected
- `adding` state prevents double-submit
- On success: calls `onAdded()` which closes modal, refreshes project, and refreshes project list

### Task 13b: Update `ProjectsContainer` — Wire in `ProjectGemList`

**1. Update imports** at top of file (line 3):
```typescript
import type { ProjectPreview, Project, ProjectDetail, GemPreview, GemSearchResult } from '../state/types';
```

**2. Replace the placeholder div** (lines 192-196 of current file):

**Remove:**
```tsx
<div className="project-gem-list empty-state">
  {selectedProjectId
    ? 'Project gem list — coming in Phase 6'
    : 'Select a project to see its gems'}
</div>
```

**Replace with:**
```tsx
<ProjectGemList
  projectId={selectedProjectId}
  onGemSelect={onGemSelect}
  onProjectsChanged={fetchProjects}
  onProjectDeleted={() => {
    setSelectedProjectId(null);
    fetchProjects();
  }}
/>
```

**3. Update `ProjectsContainer` destructuring** — the `onGemSelect` prop is currently unused (line 160 has `{ }`). Fix it:

**Change:**
```tsx
export function ProjectsContainer({ }: ProjectsContainerProps) {
```

**To:**
```tsx
export function ProjectsContainer({ onGemSelect }: ProjectsContainerProps) {
```

## Important Notes

### Component order in the file

The components should appear in this order in `ProjectsContainer.tsx`:
1. `CreateProjectForm` (already exists)
2. `AddGemsModal` (new)
3. `ProjectList` (already exists)
4. `ProjectGemList` (new)
5. `ProjectsContainer` (already exists — exported)

This order ensures each component is defined before it's referenced.

### Type imports

The file needs these types from `../state/types`:
- `ProjectPreview` — already imported (used by ProjectList)
- `Project` — already imported (used by CreateProjectForm)
- `ProjectDetail` — **add** (used by ProjectGemList)
- `GemPreview` — **add** (used by ProjectGemList for gem display and search results)
- `GemSearchResult` — **add** (used by AddGemsModal)

### Inline styles vs. classNames

Since Phase 8 (CSS) hasn't happened yet, use a mix:
- **Use classNames** that match the design doc (`.project-gem-list`, `.project-metadata-header`, `.modal-overlay`, etc.) — they'll be styled properly in Phase 8
- **Use inline styles** for basic layout that needs to work now (flex, padding, colors) — these can be moved to CSS in Phase 8

### The `search_gems` command

`AddGemsModal` uses `invoke<GemSearchResult[]>('search_gems', { query, limit })` — this is the same command used by `GemsPanel.tsx`. It works with both empty and non-empty queries:
- Empty query (`''`): returns all gems ordered by `captured_at DESC`
- Non-empty query: performs keyword + optional semantic search

### The `get_project_gems` command

`ProjectGemList` search uses `invoke<GemPreview[]>('get_project_gems', { projectId, query, limit })`:
- When `query` is provided: filters using FTS5 full-text search within the project's gems
- Returns `GemPreview[]` (not `GemSearchResult[]`) — different type than `search_gems`

## Verification

After implementing, verify the frontend builds:
```bash
cd jarvis-app && npm run build
```

Must pass with no errors.

**Things to check manually (if running the app):**
- Select a project → gem list loads showing metadata header and gems (or empty state)
- Click "+ Add Gems" → modal opens with all gems listed
- Search in modal → results filter with 300ms debounce
- Check gems and click "Add Selected" → gems appear in project gem list
- Already-added gems in modal are pre-checked and disabled
- Click × on a gem card → gem removed from project (gem still exists in GemsPanel)
- Click a gem card → gem detail opens in right panel
- Click "Delete" on project → confirmation bar appears → "Confirm Delete" removes project
- Search within project gems → filters with debounce
- Empty project shows "No gems in this project" message
- No project selected shows "Select a project to see its gems"
- Escape key closes modal
- Clicking outside modal closes it

## What NOT to Do

- Do NOT create separate files for `ProjectGemList` or `AddGemsModal` — keep everything in `ProjectsContainer.tsx`
- Do NOT add project editing UI (Edit button) — that's Phase 7
- Do NOT add "Add to Project" dropdown on GemCards in GemsPanel — that's Phase 7
- Do NOT add CSS to `App.css` — Phase 8 handles styling. Use classNames from the design doc + minimal inline styles for layout
- Do NOT modify `App.tsx` — it's already wired correctly from Phase 5
- Do NOT modify `RightPanel.tsx` — gem selection already works via `onGemSelect` → `selectedGemId`
- Do NOT modify `GemsPanel.tsx`

## After Implementation

Once `npm run build` passes:
1. Show me the updated `ProjectsContainer.tsx` file
2. I'll review before we proceed to Phase 7

**If you have any confusion or questions — about the two different gem types (`GemPreview` vs `GemSearchResult`), the search debounce pattern, how `onProjectDeleted` should clear selection, the modal overlay approach, or anything else — please ask before guessing.**
