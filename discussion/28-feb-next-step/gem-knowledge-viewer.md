# Gem Knowledge Viewer — File Tree + Tabbed Viewer

## Problem

Knowledge files are generated per-gem (`knowledge/{gem_id}/*.md`) but there's no way to see them in the UI.

## Design

Two parts: a **file tree** inside GemDetailPanel, and a **tab system** in RightPanel that opens files beside the detail view.

### How It Works

```
┌─────────────────────────────┐  ┌─────────────────────────────┐
│  RightPanel                 │  │  RightPanel (after click)    │
│                             │  │                              │
│  ┌─ GemDetailPanel ───────┐│  │  [Detail] [gem.md]           │
│  │ Title: "React Hooks..." ││  │  ┌──────────────────────────┐│
│  │ Domain: medium.com      ││  │  │ # React Hooks Guide      ││
│  │ Tags: react, hooks      ││  │  │                          ││
│  │ Summary: ...            ││  │  │ - **Source:** Article     ││
│  │ Transcript: ...         ││  │  │ - **URL:** medium.com/...││
│  │                         ││  │  │ - **Tags:** react, hooks ││
│  │ ── Knowledge Files ──── ││  │  │                          ││
│  │  📄 gem.md       4.2 KB ││  │  │ ## Summary               ││
│  │  📄 content.md   2.1 KB ││  │  │ A guide to React hooks...││
│  │  📄 enrichment.md 0.8 KB││  │  │                          ││
│  │                         ││  │  │ ## Content                ││
│  │  [Actions: Enrich|Del]  ││  │  │ ...                      ││
│  └─────────────────────────┘│  │  └──────────────────────────┘│
└─────────────────────────────┘  └──────────────────────────────┘
         Before click                    After clicking gem.md
```

### Step 1: File Tree in GemDetailPanel

At the bottom of GemDetailPanel (above the action buttons), show a "Knowledge Files" section listing the existing `.md` subfiles:

```
── Knowledge Files ──────────────
  📄 gem.md          4.2 KB
  📄 content.md      2.1 KB
  📄 enrichment.md   0.8 KB
  📄 transcript.md   1.5 KB
─────────────────────────────────
```

- Only files where `exists === true` are shown (skip `meta.json`)
- Each row is clickable — shows filename + size
- Data comes from `get_gem_knowledge` → `KnowledgeEntry.subfiles`
- If no knowledge files exist, show "No knowledge files" with a "Generate" button

### Step 2: Tab System in RightPanel (gems section)

When user clicks a filename in the tree, RightPanel switches from single-panel to tabbed view — exactly how recordings get "Details" + "Chat" tabs when a chat session starts:

```tsx
// RightPanel.tsx — gems section (currently lines 265-277)
// Before: renders GemDetailPanel directly
// After: if a knowledge file is open, show tabs

[Detail]  [gem.md]  [enrichment.md]
```

- **"Detail" tab** = current GemDetailPanel (always present, always default)
- **File tabs** = one per opened file, labeled with filename
- Clicking a file in the tree adds/switches to that tab
- Tabs can be closed (click X on tab) — returns to Detail if last file tab closed
- File content rendered as plain preformatted text (`.md` source, monospace) — same style as transcript display

### Data Flow

```
GemDetailPanel mounts
  → invoke('get_gem_knowledge', { gemId })
  → Receives KnowledgeEntry { subfiles: [...], assembled }
  → Renders file tree from subfiles.filter(s => s.exists && s.filename !== 'meta.json')

User clicks "gem.md" in tree
  → Calls onOpenKnowledgeFile(filename) prop (callback to RightPanel)
  → RightPanel adds tab, sets activeTab = 'gem.md'
  → invoke('get_gem_knowledge_subfile', { gemId, filename: 'gem.md' })
  → Renders content in tab pane

User clicks another file
  → Adds another tab (or switches if already open)

User clicks X on tab
  → Removes tab, switches to Detail or next open tab
```

### New Tauri Command

`get_gem_knowledge_subfile` — backend `get_subfile()` already exists, just needs a command wrapper:

```rust
#[tauri::command]
pub async fn get_gem_knowledge_subfile(
    gem_id: String,
    filename: String,
    knowledge_store: State<'_, Arc<dyn KnowledgeStore>>,
) -> Result<Option<String>, String> {
    knowledge_store.get_subfile(&gem_id, &filename).await
}
```

Register in `lib.rs` `generate_handler!` and `knowledge/commands.rs`.

### Component Changes

**`GemDetailPanel.tsx`**
- Add `knowledgeEntry` state (fetched on mount alongside gem)
- Add "Knowledge Files" section with clickable file rows
- New prop: `onOpenKnowledgeFile: (filename: string) => void`

**`RightPanel.tsx`** (gems section, lines 265-287)
- Add state: `openKnowledgeFiles: string[]` (list of open filenames)
- Add state: `activeGemTab: 'detail' | string` (filename or 'detail')
- Add state: `knowledgeFileContents: Record<string, string>` (cached content)
- When `openKnowledgeFiles` is non-empty, render tab bar + content
- When empty, render GemDetailPanel directly (current behavior)
- Pass `onOpenKnowledgeFile` callback down to GemDetailPanel

**New component: `KnowledgeFileViewer.tsx`** (optional, or inline)
- Takes `content: string` prop
- Renders in a scrollable `<pre>` with monospace font
- Same styling as `.transcript-text.scrollable`

### CSS

Reuse existing `.tab-buttons`, `.tab-button`, `.tab-content` classes from recording tabs. Add:
- `.knowledge-file-tree` — the file list section in GemDetailPanel
- `.knowledge-file-row` — clickable row (filename + size)
- `.knowledge-file-viewer` — the content pane (mirrors `.transcript-text`)
- `.tab-button .tab-close` — small X button on file tabs

## What We Skip

- Markdown rendering (show raw `.md` source — simpler, and devs prefer it)
- Editing knowledge files from UI
- File watcher / live refresh
- Drag-and-drop tab reordering
