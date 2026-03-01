# Phase 6: Frontend Integration — RightPanel, App.tsx, ProjectsContainer

## Context

You are implementing Phase 6 of the Project Research Assistant feature for Jarvis AWS (Tauri desktop app: Rust backend + React/TypeScript frontend).

**Phases 1-5 are COMPLETE:**
- Backend: search infrastructure, providers, agent, 7 Tauri commands
- Frontend: `WebSearchResult` and `ProjectResearchResults` types in `types.ts`, `ProjectResearchChat` component in `src/components/ProjectResearchChat.tsx` (default export)

**Phase 6 wires `ProjectResearchChat` into the app** by modifying 3 existing files:
1. `RightPanel.tsx` — show research chat when a project is selected
2. `App.tsx` — lift project selection state, pass new props
3. `ProjectsContainer.tsx` — emit project selection events, accept refresh trigger

---

## File 1: `RightPanel.tsx`

**File**: `jarvis-app/src/components/RightPanel.tsx` (483 lines)

### 1.1 Add import (after line 8)

```tsx
import ProjectResearchChat from './ProjectResearchChat';
```

### 1.2 Add 3 new props to `RightPanelProps` interface (lines 12-41)

Add these after line 40 (`style?: React.CSSProperties;`), before the closing `}`:

```typescript
selectedProjectId?: string | null;
selectedProjectTitle?: string | null;
onProjectGemsChanged?: () => void;
```

### 1.3 Add 3 new props to destructuring (lines 43-71)

Add these after line 70 (`style`), before the closing `}: RightPanelProps)`:

```typescript
selectedProjectId,
selectedProjectTitle,
onProjectGemsChanged,
```

### 1.4 Replace the `activeNav === 'projects'` block (lines 401-479)

Replace the ENTIRE block from line 401 (`// Projects nav:`) through line 479 (closing `}`) with:

```tsx
  // Projects nav: show research chat when a project is selected
  if (activeNav === 'projects') {
    // No project selected
    if (!selectedProjectId) {
      return (
        <div className="right-panel" style={style}>
          <div className="right-panel-placeholder">
            Select a project to start researching
          </div>
        </div>
      );
    }

    // Project selected + gem selected → tabs: Research | Detail
    if (selectedGemId) {
      return (
        <div className="right-panel" style={style}>
          <div className="record-tabs-view">
            <div className="tab-buttons">
              <button
                className={`tab-button ${activeTab === 'chat' ? 'active' : ''}`}
                onClick={() => handleTabChange('chat')}
              >
                Research
              </button>
              <button
                className={`tab-button ${activeTab === 'transcript' ? 'active' : ''}`}
                onClick={() => handleTabChange('transcript')}
              >
                Detail
              </button>
            </div>
            <div className="tab-content">
              {activeTab === 'chat' ? (
                <ProjectResearchChat
                  key={selectedProjectId}
                  projectId={selectedProjectId}
                  projectTitle={selectedProjectTitle || ''}
                  onGemsAdded={onProjectGemsChanged}
                />
              ) : (
                <GemDetailPanel
                  gemId={selectedGemId}
                  onDelete={onDeleteGem}
                  onTranscribe={onTranscribeGem}
                  onEnrich={onEnrichGem}
                  aiAvailable={aiAvailable}
                  onOpenKnowledgeFile={handleOpenKnowledgeFile}
                />
              )}
            </div>
          </div>
        </div>
      );
    }

    // Project selected, no gem selected → research chat full-height
    return (
      <div className="right-panel" style={style}>
        <ProjectResearchChat
          key={selectedProjectId}
          projectId={selectedProjectId}
          projectTitle={selectedProjectTitle || ''}
          onGemsAdded={onProjectGemsChanged}
        />
      </div>
    );
  }
```

**Key details:**
- `key={selectedProjectId}` forces remount when project changes, triggering the auto-suggest topics mount effect
- Research tab reuses `activeTab === 'chat'`, Detail tab reuses `activeTab === 'transcript'` — these are just the existing tab state values with different label text
- The old knowledge-file tabbing for projects is removed (it was a duplicate of the gems nav pattern and isn't needed when Research tab exists)
- `GemDetailPanel` still receives `onOpenKnowledgeFile` — knowledge files work within the Detail tab

### 1.5 Add a useEffect to default to Research tab for projects

The `activeTab` starts as `'transcript'`. When the user is in projects view and selects a gem, we want the Research tab to be the default (not Detail). Add this effect after the existing tab-related effects (after line 92):

```tsx
// Default to Research tab when entering projects + gem view
useEffect(() => {
  if (activeNav === 'projects' && selectedProjectId) {
    setActiveTab('chat'); // 'chat' = Research tab in projects context
  }
}, [activeNav, selectedProjectId]);
```

This ensures when the user navigates to projects or selects a project, the Research tab is active by default.

---

## File 2: `App.tsx`

**File**: `jarvis-app/src/App.tsx` (927 lines)

### 2.1 Add project state (after line 57)

After `gemsPanelRefreshKey` state on line 57, add:

```tsx
// Project selection state (lifted from ProjectsContainer for RightPanel access)
const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
const [selectedProjectTitle, setSelectedProjectTitle] = useState<string | null>(null);
const [projectGemsRefreshKey, setProjectGemsRefreshKey] = useState(0);
```

### 2.2 Add project callbacks (after `handleEnrichGem` around line 535)

Add these after the existing gem handler functions:

```tsx
/**
 * Handle project selection from ProjectsContainer
 */
const handleProjectSelect = useCallback((id: string | null, title: string | null) => {
  setSelectedProjectId(id);
  setSelectedProjectTitle(title);
}, []);

/**
 * Handle gem list refresh after research agent adds a gem
 */
const handleProjectGemsChanged = useCallback(() => {
  setProjectGemsRefreshKey(prev => prev + 1);
}, []);
```

### 2.3 Reset project state in `handleNavChange` (lines 568-590)

Inside `handleNavChange`, after line 583 (`setSelectedGemId(null);`), add:

```tsx
setSelectedProjectId(null);
setSelectedProjectTitle(null);
```

This resets project selection when switching between nav sections (record/recordings/gems/projects/youtube/browser/settings).

### 2.4 Update `ProjectsContainer` render (lines 833-834)

Change:
```tsx
<ProjectsContainer onGemSelect={handleGemSelect} />
```

To:
```tsx
<ProjectsContainer
  onGemSelect={handleGemSelect}
  onProjectSelect={handleProjectSelect}
  refreshTrigger={projectGemsRefreshKey}
/>
```

### 2.5 Add 3 props to `RightPanel` render (lines 856-884)

Add these props to the `<RightPanel>` JSX, after `chatStatus={chatStatus}` (line 882) and before `style={{ width: rightPanelWidth }}` (line 883):

```tsx
selectedProjectId={selectedProjectId}
selectedProjectTitle={selectedProjectTitle}
onProjectGemsChanged={handleProjectGemsChanged}
```

---

## File 3: `ProjectsContainer.tsx`

**File**: `jarvis-app/src/components/ProjectsContainer.tsx` (700 lines)

### 3.1 Update `ProjectsContainerProps` (lines 5-7)

Change:
```typescript
interface ProjectsContainerProps {
  onGemSelect?: (gemId: string | null) => void;
}
```

To:
```typescript
interface ProjectsContainerProps {
  onGemSelect?: (gemId: string | null) => void;
  onProjectSelect?: (id: string | null, title: string | null) => void;
  refreshTrigger?: number;
}
```

### 3.2 Update `ProjectsContainer` function signature (line 657)

Change:
```tsx
export function ProjectsContainer({ onGemSelect }: ProjectsContainerProps) {
```

To:
```tsx
export function ProjectsContainer({ onGemSelect, onProjectSelect, refreshTrigger }: ProjectsContainerProps) {
```

### 3.3 Update `CreateProjectForm.onCreated` to pass title (line 35)

In the `CreateProjectForm` component, change line 35:

```tsx
onCreated(project.id);
```

To:
```tsx
onCreated(project.id, project.title);
```

### 3.4 Update `CreateProjectForm.onCreated` type (lines 10-11)

Change:
```typescript
onCreated: (id: string) => void;
```

To:
```typescript
onCreated: (id: string, title: string) => void;
```

### 3.5 Update `ProjectList.onProjectCreated` type (line 261)

Change:
```typescript
onProjectCreated: (id: string) => void;
```

To:
```typescript
onProjectCreated: (id: string, title: string) => void;
```

### 3.6 Update `ProjectList` create form callback (line 280)

Change:
```tsx
onCreated={(id) => { setShowCreateForm(false); onProjectCreated(id); }}
```

To:
```tsx
onCreated={(id, title) => { setShowCreateForm(false); onProjectCreated(id, title); }}
```

### 3.7 Update `handleProjectCreated` (lines 675-678)

Change:
```tsx
const handleProjectCreated = (projectId: string) => {
  fetchProjects();
  setSelectedProjectId(projectId);
};
```

To:
```tsx
const handleProjectCreated = (projectId: string, projectTitle: string) => {
  fetchProjects();
  setSelectedProjectId(projectId);
  onProjectSelect?.(projectId, projectTitle);
};
```

### 3.8 Update `ProjectList` render to emit selection with title (lines 682-688)

Change `onSelectProject={setSelectedProjectId}` to:
```tsx
onSelectProject={(id) => {
  setSelectedProjectId(id);
  const project = projects.find(p => p.id === id);
  onProjectSelect?.(id, project?.title || null);
}}
```

### 3.9 Update `onProjectDeleted` callback (lines 693-696)

Change:
```tsx
onProjectDeleted={() => {
  setSelectedProjectId(null);
  fetchProjects();
}}
```

To:
```tsx
onProjectDeleted={() => {
  setSelectedProjectId(null);
  onProjectSelect?.(null, null);
  fetchProjects();
}}
```

### 3.10 Add `refreshTrigger` prop to `ProjectGemList` (line 689-696)

Add `refreshTrigger={refreshTrigger}` to the `<ProjectGemList>` render:

```tsx
<ProjectGemList
  projectId={selectedProjectId}
  onGemSelect={onGemSelect}
  onProjectsChanged={fetchProjects}
  onProjectDeleted={() => {
    setSelectedProjectId(null);
    onProjectSelect?.(null, null);
    fetchProjects();
  }}
  refreshTrigger={refreshTrigger}
/>
```

### 3.11 Add `refreshTrigger` to `ProjectGemList` props (lines 319-328)

Add `refreshTrigger?: number;` to the props type:

```typescript
function ProjectGemList({
  projectId,
  onGemSelect,
  onProjectsChanged,
  onProjectDeleted,
  refreshTrigger,
}: {
  projectId: string | null;
  onGemSelect?: (gemId: string | null) => void;
  onProjectsChanged: () => void;
  onProjectDeleted: () => void;
  refreshTrigger?: number;
}) {
```

### 3.12 Add `refreshTrigger` to `ProjectGemList` useEffect deps (line 344-352)

The existing useEffect loads the project when `projectId` changes. Add `refreshTrigger` to the dependency array so it also reloads when gems are added via the research chat.

Change:
```tsx
useEffect(() => {
  if (!projectId) {
    setDetail(null);
    setSearchQuery('');
    setSearchResults(null);
    return;
  }
  loadProject(projectId);
}, [projectId]);
```

To:
```tsx
useEffect(() => {
  if (!projectId) {
    setDetail(null);
    setSearchQuery('');
    setSearchResults(null);
    return;
  }
  loadProject(projectId);
}, [projectId, refreshTrigger]);
```

---

## Data Flow Summary

```
User clicks project in ProjectList
  → ProjectsContainer.onSelectProject(id)
    → setSelectedProjectId(id)
    → onProjectSelect(id, title)  → App.tsx
      → setSelectedProjectId(id), setSelectedProjectTitle(title)
      → passed to RightPanel as props

RightPanel sees activeNav='projects' + selectedProjectId
  → renders ProjectResearchChat

User clicks "Add" on gem in research results
  → ProjectResearchChat.handleAddGem(gemId)
    → invoke('add_gems_to_project')
    → onGemsAdded() → onProjectGemsChanged() → App.tsx
      → setProjectGemsRefreshKey(prev + 1)
      → ProjectsContainer.refreshTrigger changes
        → ProjectGemList useEffect fires
          → loadProject() → gem list refreshes
```

---

## Gotchas

1. **`activeTab` state is shared across nav contexts.** The `activeTab` state in RightPanel (`'transcript' | 'copilot' | 'chat'`) is used for recordings AND projects. For projects: `'chat'` = Research tab, `'transcript'` = Detail tab. The useEffect in section 1.5 ensures Research is the default when entering projects view.

2. **Don't remove the `handleOpenKnowledgeFile` handler.** It's still used by `GemDetailPanel` in the Detail tab. The knowledge file state variables (`openKnowledgeFiles`, `activeGemTab`, `knowledgeFileContents`) remain — they just aren't rendered as separate tabs in the projects context anymore.

3. **`ProjectsContainer` keeps internal `selectedProjectId` state.** This is intentional — it's used for `ProjectList` highlighting and `ProjectGemList` loading. The `onProjectSelect` callback mirrors changes up to App.tsx so RightPanel can access the selection. This is a controlled+callback pattern, not full state lifting.

4. **`key={selectedProjectId}` on `ProjectResearchChat`** — this is critical. Without it, switching projects wouldn't remount the component, so the auto-suggest topics effect wouldn't re-run. The key forces a full remount.

5. **`loadProject` is defined inside `ProjectGemList`** (line 375). Adding `refreshTrigger` to the useEffect that calls it (line 344) is sufficient — the function reference doesn't need to be stable since it's defined in the same component scope.

6. **Order of props in JSX matters for readability** but not functionality. When adding props to `<RightPanel>` in App.tsx, group the new project props together.

---

## Verification Checklist

After implementation:
- [ ] RightPanel imports `ProjectResearchChat`
- [ ] RightPanel accepts 3 new props: `selectedProjectId`, `selectedProjectTitle`, `onProjectGemsChanged`
- [ ] Projects nav in RightPanel has 3 states: no project, project+gem (tabs), project only (full chat)
- [ ] Research tab is default when entering projects view (useEffect)
- [ ] `key={selectedProjectId}` on all `<ProjectResearchChat>` instances
- [ ] App.tsx has `selectedProjectId`, `selectedProjectTitle`, `projectGemsRefreshKey` state
- [ ] App.tsx passes `onProjectSelect` and `refreshTrigger` to `ProjectsContainer`
- [ ] App.tsx passes `selectedProjectId`, `selectedProjectTitle`, `onProjectGemsChanged` to `RightPanel`
- [ ] App.tsx resets project state in `handleNavChange`
- [ ] ProjectsContainer emits `onProjectSelect` on: select, create, delete
- [ ] ProjectGemList reloads when `refreshTrigger` changes
- [ ] `CreateProjectForm.onCreated` passes both `id` and `title`
- [ ] `npm run build` or `npm run check` passes (no TypeScript errors)
- [ ] Switching projects remounts `ProjectResearchChat` (fresh topic suggestions)
- [ ] Adding a gem via research chat refreshes the project gem list

---

## Questions You Should Ask Before Starting

1. Does `GemDetailPanel` require `onOpenKnowledgeFile` as a required prop, or is it optional? If required, make sure to pass `handleOpenKnowledgeFile` in the Detail tab.
2. Are there any other places in the codebase that import or reference `ProjectsContainer` besides `App.tsx`? Check to ensure the new props don't break other consumers.
