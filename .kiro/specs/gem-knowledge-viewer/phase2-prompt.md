# Prompt: Implement Gem Knowledge Viewer — Phase 2 (Tabbed Viewer in RightPanel)

## Your Task

Implement **Phase 3: RightPanel Tabbed View** (Tasks 5–7) from `.kiro/specs/gem-knowledge-viewer/tasks.md`. This phase wires the file tree clicks into a real tabbed viewer — clicking a knowledge file in the gem detail tree opens it in a tab beside the "Detail" tab, following the exact same pattern used by recordings (Details + Chat tabs).

**After completing this, stop and ask me to review.**

If you have any confusion during implementation — about how the existing tab system works, how to manage the state, or how `stopPropagation` should work for the close button — feel free to ask.

---

## Context

Phase 1 is complete:
- `get_gem_knowledge_subfile` Tauri command exists and works (reads individual `.md` files)
- `KnowledgeEntry` and `KnowledgeSubfile` TypeScript types exist in `src/state/types.ts`
- `GemDetailPanel.tsx` shows a file tree with clickable rows, fires `onOpenKnowledgeFile(filename)` callback
- `RightPanel.tsx` currently passes `onOpenKnowledgeFile={() => {}}` (no-op) — this is what we replace
- CSS for `.knowledge-file-viewer`, `.knowledge-file-content`, `.tab-close` already exists in `App.css`

**Spec:** `.kiro/specs/gem-knowledge-viewer/requirements.md` (Requirements 3–4)
**Design:** `.kiro/specs/gem-knowledge-viewer/design.md` (see "RightPanel.tsx Changes" section)
**Tasks:** `.kiro/specs/gem-knowledge-viewer/tasks.md` (Tasks 5–7)

---

## What This Phase Modifies

```
src/components/RightPanel.tsx    ← MODIFY — add tab state, tabbed rendering for gems section
```

Only one file changes. No backend changes. No new components (the file viewer is inline JSX).

---

## Existing Tab Pattern to Follow

The recordings section (lines 192-234) already does exactly what we need. When a chat session is active, it shows tabs:

```tsx
// Lines 192-234 — recordings with chat tabs
if (chatSessionId) {
  return (
    <div className="right-panel" style={style}>
      <div className="record-tabs-view">
        <div className="tab-buttons">
          <button className={`tab-button ${activeTab === 'transcript' ? 'active' : ''}`} ...>
            Details
          </button>
          <button className={`tab-button ${activeTab === 'chat' ? 'active' : ''}`} ...>
            Chat
          </button>
        </div>
        <div className="tab-content">
          {activeTab === 'transcript' ? <RecordingDetailPanel ... /> : <ChatPanel ... />}
        </div>
      </div>
    </div>
  );
}
```

The gems section will follow this same structure: when files are open, show tabs. When no files are open, show GemDetailPanel directly.

---

## Implementation

### Step 1: Add new state variables

Add these after the existing state declarations (after line 72):

```typescript
// Knowledge file viewer state (gems section)
const [openKnowledgeFiles, setOpenKnowledgeFiles] = useState<string[]>([]);
const [activeGemTab, setActiveGemTab] = useState<string>('detail');
const [knowledgeFileContents, setKnowledgeFileContents] = useState<Record<string, string>>({});
```

### Step 2: Add import for invoke

Add `invoke` to the imports at the top of the file:

```typescript
import { invoke } from '@tauri-apps/api/core';
```

### Step 3: Reset state when gem changes

Add a `useEffect` that resets all knowledge tab state when `selectedGemId` changes. Add after the existing `useEffect` for `chatSessionId` (after line 79):

```typescript
// Reset knowledge file tabs when gem changes
useEffect(() => {
  setOpenKnowledgeFiles([]);
  setActiveGemTab('detail');
  setKnowledgeFileContents({});
}, [selectedGemId]);
```

### Step 4: Add handler functions

Add these handler functions after the `handleTabChange` function (after line 94):

```typescript
// Knowledge file tab handlers
const handleOpenKnowledgeFile = async (filename: string) => {
  // Add tab if not already open
  if (!openKnowledgeFiles.includes(filename)) {
    setOpenKnowledgeFiles(prev => [...prev, filename]);
  }
  // Switch to it
  setActiveGemTab(filename);

  // Fetch content if not cached
  if (!knowledgeFileContents[filename] && selectedGemId) {
    try {
      const content = await invoke<string | null>(
        'get_gem_knowledge_subfile',
        { gemId: selectedGemId, filename }
      );
      setKnowledgeFileContents(prev => ({
        ...prev,
        [filename]: content ?? 'File not found',
      }));
    } catch (e) {
      setKnowledgeFileContents(prev => ({
        ...prev,
        [filename]: `Error loading file: ${e}`,
      }));
    }
  }
};

const handleCloseKnowledgeTab = (filename: string) => {
  const remaining = openKnowledgeFiles.filter(f => f !== filename);
  setOpenKnowledgeFiles(remaining);

  // If closing the active tab, switch to another
  if (activeGemTab === filename) {
    setActiveGemTab(remaining.length > 0 ? remaining[remaining.length - 1] : 'detail');
  }

  // Clean up cached content
  setKnowledgeFileContents(prev => {
    const next = { ...prev };
    delete next[filename];
    return next;
  });
};
```

### Step 5: Replace the gems section

Replace the entire gems section (lines 264-288) with this:

```tsx
// Gems nav: show gem detail panel when a gem is selected
if (activeNav === 'gems') {
  if (selectedGemId) {
    // Tabbed mode: when knowledge files are open
    if (openKnowledgeFiles.length > 0) {
      return (
        <div className="right-panel" style={style}>
          <div className="record-tabs-view">
            <div className="tab-buttons">
              <button
                className={`tab-button ${activeGemTab === 'detail' ? 'active' : ''}`}
                onClick={() => setActiveGemTab('detail')}
              >
                Detail
              </button>
              {openKnowledgeFiles.map(filename => (
                <button
                  key={filename}
                  className={`tab-button ${activeGemTab === filename ? 'active' : ''}`}
                  onClick={() => setActiveGemTab(filename)}
                >
                  {filename}
                  <span
                    className="tab-close"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleCloseKnowledgeTab(filename);
                    }}
                  >
                    ×
                  </span>
                </button>
              ))}
            </div>
            <div className="tab-content">
              {activeGemTab === 'detail' ? (
                <GemDetailPanel
                  gemId={selectedGemId}
                  onDelete={onDeleteGem}
                  onTranscribe={onTranscribeGem}
                  onEnrich={onEnrichGem}
                  aiAvailable={aiAvailable}
                  onOpenKnowledgeFile={handleOpenKnowledgeFile}
                />
              ) : (
                <div className={`knowledge-file-viewer ${!knowledgeFileContents[activeGemTab] ? 'loading' : ''}`}>
                  {knowledgeFileContents[activeGemTab] ? (
                    <pre className="knowledge-file-content">
                      {knowledgeFileContents[activeGemTab]}
                    </pre>
                  ) : (
                    <>Loading {activeGemTab}...</>
                  )}
                </div>
              )}
            </div>
          </div>
        </div>
      );
    }

    // Single-panel mode: no knowledge files open (current behavior)
    return (
      <div className="right-panel" style={style}>
        <GemDetailPanel
          gemId={selectedGemId}
          onDelete={onDeleteGem}
          onTranscribe={onTranscribeGem}
          onEnrich={onEnrichGem}
          aiAvailable={aiAvailable}
          onOpenKnowledgeFile={handleOpenKnowledgeFile}
        />
      </div>
    );
  }

  return (
    <div className="right-panel" style={style}>
      <div className="right-panel-placeholder">
        Select a gem to view details
      </div>
    </div>
  );
}
```

**Key differences from the old gems section:**
1. `onOpenKnowledgeFile={() => {}}` is replaced with `onOpenKnowledgeFile={handleOpenKnowledgeFile}` in **both** render paths (tabbed and single-panel)
2. When `openKnowledgeFiles.length > 0`, wraps everything in `.record-tabs-view` with tab bar
3. Tab bar has "Detail" button + one button per open file (with close `×`)
4. Content area switches between `GemDetailPanel` and the file viewer `<pre>`
5. When no files are open, renders exactly like before (just with the real handler instead of no-op)

---

## How It All Connects

```
User clicks "gem.md" in GemDetailPanel file tree
  → GemDetailPanel calls props.onOpenKnowledgeFile("gem.md")
  → RightPanel.handleOpenKnowledgeFile("gem.md")
    → Adds "gem.md" to openKnowledgeFiles state
    → Sets activeGemTab = "gem.md"
    → Fetches content via invoke('get_gem_knowledge_subfile', { gemId, filename: "gem.md" })
    → Stores in knowledgeFileContents["gem.md"]
  → RightPanel re-renders with tab bar visible
  → Tab bar shows: [Detail] [gem.md ×]
  → Content area shows: <pre>{content}</pre>

User clicks × on "gem.md" tab
  → e.stopPropagation() prevents tab switch
  → handleCloseKnowledgeTab("gem.md")
    → Removes from openKnowledgeFiles
    → Since it was active, switches to 'detail'
    → Removes cached content
  → openKnowledgeFiles is now [] → single-panel mode (no tab bar)
```

---

## Verification Checklist

- [ ] Clicking a file in the gem detail tree opens a tab in RightPanel with file content
- [ ] Tab bar shows "Detail" + one tab per open file with filename as label
- [ ] "Detail" tab renders the GemDetailPanel (unchanged from before)
- [ ] File tabs show the file content in monospace preformatted text
- [ ] Clicking × on a file tab closes it (does NOT switch to that tab first)
- [ ] Closing the last file tab returns to single-panel mode (no tab bar visible)
- [ ] Clicking a file that's already open switches to its tab (no duplicate)
- [ ] Content is cached — switching between tabs doesn't re-fetch from backend
- [ ] Switching to a different gem resets all tabs (returns to single-panel Detail)
- [ ] Loading state shows "Loading {filename}..." while content is being fetched
- [ ] If file content fetch fails, tab shows error message (doesn't crash)
- [ ] Tab bar uses existing `.tab-buttons` / `.tab-button` / `.tab-button.active` CSS (matches recording tabs)
- [ ] `npm run dev` starts without TypeScript errors
- [ ] No console errors during normal usage (open, switch, close tabs)

---

## Important Notes

- **Only `RightPanel.tsx` changes.** No backend changes, no new files, no `GemDetailPanel` changes. The file tree and callback already work from Phase 1.
- **`e.stopPropagation()` on the close button is critical.** Without it, clicking × also triggers the tab button's `onClick`, which would switch to the tab before closing it.
- **The file viewer is inline JSX, not a separate component.** It's just a `<div className="knowledge-file-viewer"><pre>...</pre></div>`. Keep it simple.
- **Content cache cleanup:** When closing a tab, remove the content from `knowledgeFileContents` to free memory. When switching gems, clear everything.
- **`invoke` import:** RightPanel likely doesn't import `invoke` yet since it doesn't call Tauri commands directly. You'll need to add the import.
- **CSS is already done.** `.knowledge-file-viewer`, `.knowledge-file-content`, `.tab-close`, `.knowledge-file-viewer.loading` all exist in `App.css` from Phase 1. No CSS changes needed.
- **Two render paths for GemDetailPanel:** In both the tabbed path AND the single-panel path, pass `onOpenKnowledgeFile={handleOpenKnowledgeFile}` — not the no-op. The single-panel path becomes tabbed as soon as the user clicks a file.
