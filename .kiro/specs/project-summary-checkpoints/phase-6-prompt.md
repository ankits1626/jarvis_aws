# Phase 6: RightPanel Integration — Summary Tab

## What You're Building

You are implementing Phase 6 of the Project Summary Checkpoints feature. This phase integrates the `ProjectSummaryChat` component (built in Phase 5) into the `RightPanel` as a new "Summary" tab.

**Changes are limited to `RightPanel.tsx` only.** The component already exists and works.

## Context

Read these files before starting:

- **Tasks:** `.kiro/specs/project-summary-checkpoints/tasks.md` — Phase 6 has Task 14
- **Design doc:** `.kiro/specs/project-summary-checkpoints/design.md` — see "RightPanel Change" section
- **The component you're integrating:** `src/components/ProjectSummaryChat.tsx` — already built, accepts `projectId`, `projectTitle`, `onGemSaved`
- **The file you're modifying:** `src/components/RightPanel.tsx` — read the full file, especially the `activeNav === 'projects'` section (lines 424-514)

## Current State of RightPanel (Projects Section)

The `activeNav === 'projects'` block has three cases:

1. **No project selected** (line 427): Shows placeholder text
2. **Project + gem selected** (line 438): Shows tabs `[Research] [Detail] [knowledge files]`
3. **Project selected, no gem** (line 503): Shows `ProjectResearchChat` full-height (no tabs)

Current state type (line 79):
```typescript
const [activeTab, setActiveTab] = useState<'transcript' | 'copilot' | 'chat'>('transcript');
```

Current `handleTabChange` (line 109):
```typescript
const handleTabChange = (tab: 'transcript' | 'copilot' | 'chat') => {
```

## Task 14: Add Summary Tab to RightPanel

### Step 1: Add import

At the top of `RightPanel.tsx`, add after the `ProjectResearchChat` import (line 9):

```typescript
import ProjectSummaryChat from './ProjectSummaryChat';
```

### Step 2: Extend `activeTab` type

Update the state type to include `'summary'` (line 79):

```typescript
const [activeTab, setActiveTab] = useState<'transcript' | 'copilot' | 'chat' | 'summary'>('transcript');
```

### Step 3: Update `handleTabChange`

Update the handler to accept `'summary'` (line 109):

```typescript
const handleTabChange = (tab: 'transcript' | 'copilot' | 'chat' | 'summary') => {
```

### Step 4: Update Case 2 — Project + gem selected (line 438)

Add a "Summary" tab button between Research and Detail. The tab buttons section currently has:

```tsx
<button ... onClick={() => handleTabChange('chat')}>Research</button>
<button ... onClick={() => { handleTabChange('transcript'); setActiveGemTab('detail'); }}>Detail</button>
{openKnowledgeFiles.map(...)}
```

**Add the Summary button between Research and Detail:**

```tsx
<button
  className={`tab-button ${activeTab === 'chat' ? 'active' : ''}`}
  onClick={() => handleTabChange('chat')}
>
  Research
</button>
<button
  className={`tab-button ${activeTab === 'summary' ? 'active' : ''}`}
  onClick={() => handleTabChange('summary')}
>
  Summary
</button>
<button
  className={`tab-button ${activeTab === 'transcript' && activeGemTab === 'detail' ? 'active' : ''}`}
  onClick={() => { handleTabChange('transcript'); setActiveGemTab('detail'); }}
>
  Detail
</button>
```

**Update the tab content rendering** (currently lines 472-496). The current logic is:

```tsx
{activeTab === 'chat' ? (
  <ProjectResearchChat ... />
) : activeGemTab !== 'detail' ... ? (
  <div className="knowledge-file-viewer">...</div>
) : (
  <GemDetailPanel ... />
)}
```

**Replace with logic that handles 'summary' too:**

```tsx
{activeTab === 'chat' ? (
  <ProjectResearchChat
    key={selectedProjectId}
    projectId={selectedProjectId}
    projectTitle={selectedProjectTitle || ''}
    onGemsAdded={onProjectGemsChanged}
  />
) : activeTab === 'summary' ? (
  <ProjectSummaryChat
    key={selectedProjectId}
    projectId={selectedProjectId}
    projectTitle={selectedProjectTitle || ''}
    onGemSaved={onProjectGemsChanged}
  />
) : activeGemTab !== 'detail' && openKnowledgeFiles.includes(activeGemTab) ? (
  <div className="knowledge-file-viewer">
    {knowledgeFileContents[activeGemTab] ? (
      <pre className="knowledge-file-content">{knowledgeFileContents[activeGemTab]}</pre>
    ) : (
      <div className="loading">Loading {activeGemTab}...</div>
    )}
  </div>
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
```

### Step 5: Update Case 3 — Project selected, no gem (line 503)

Currently this renders `ProjectResearchChat` directly with no tabs. **Replace** it with a tabbed view showing `[Research] [Summary]`:

```tsx
// Project selected, no gem selected → tabs: Research | Summary
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
          className={`tab-button ${activeTab === 'summary' ? 'active' : ''}`}
          onClick={() => handleTabChange('summary')}
        >
          Summary
        </button>
      </div>
      <div className="tab-content">
        {activeTab === 'summary' ? (
          <ProjectSummaryChat
            key={selectedProjectId}
            projectId={selectedProjectId}
            projectTitle={selectedProjectTitle || ''}
            onGemSaved={onProjectGemsChanged}
          />
        ) : (
          <ProjectResearchChat
            key={selectedProjectId}
            projectId={selectedProjectId}
            projectTitle={selectedProjectTitle || ''}
            onGemsAdded={onProjectGemsChanged}
          />
        )}
      </div>
    </div>
  </div>
);
```

### Step 6: Verify existing effects are correct

The two existing effects (lines 124-136) should still work correctly:

1. **Default to Research tab** (line 124-128): Sets `activeTab` to `'chat'` when entering projects view — this is correct, no change needed.

2. **Switch to Detail on gem select** (line 131-136): Sets `activeTab` to `'transcript'` when a gem is selected — this is correct, it overrides whatever tab the user was on (including 'summary') to show the selected gem's detail.

**No changes needed to these effects.**

## Checkpoint

Run the frontend build. The Summary tab should appear.

**What to verify:**
- **No project selected**: placeholder text (unchanged)
- **Project selected, no gem**: Shows `[Research] [Summary]` tabs. Research is default active.
- **Project selected + gem**: Shows `[Research] [Summary] [Detail]` tabs (+ knowledge file tabs). Research is default active.
- **Click Summary tab**: Shows `ProjectSummaryChat` empty state with "Generate Summary" button
- **Click Research tab**: Research chat is preserved (not reset)
- **Switch projects**: Both Research and Summary reset (via `key={selectedProjectId}`)
- **Select a gem**: Automatically switches to Detail tab
- **`onGemSaved` → `onProjectGemsChanged`**: When summary is saved as gem, the project's gem list refreshes

## If You Have Questions

- **Ask me before guessing.** Especially about the conditional rendering chain — the order matters (chat → summary → knowledge file → detail).
- **Don't modify ProjectSummaryChat.tsx.** It's done. Just mount it.
- **`key={selectedProjectId}`** is critical on both `ProjectSummaryChat` and `ProjectResearchChat` — it forces React to remount (reset state) when the project changes.
- **The `onGemSaved` prop maps to `onProjectGemsChanged`** from the parent — this callback refreshes the project's gem list in the left panel.
