# Prompt: Implement Gem Knowledge Viewer — Phase 1 (Backend + File Tree)

## Your Task

Implement **Phase 1: Backend Command** (Task 1) and **Phase 2: TypeScript Types + File Tree** (Tasks 2–4) from `.kiro/specs/gem-knowledge-viewer/tasks.md`. This phase adds the Tauri command for reading individual knowledge subfiles and shows a clickable file tree in the gem detail panel.

**After completing this, stop and ask me to review.**

If you have any confusion during implementation — about where to add types, how the knowledge entry data looks, or where the file tree should go in the component — feel free to ask.

---

## Context

The knowledge file backend is fully implemented and working:
- `src/knowledge/store.rs` — `KnowledgeStore` trait with `get_subfile()` method
- `src/knowledge/local_store.rs` — `LocalKnowledgeStore` filesystem implementation
- `src/knowledge/commands.rs` — 4 existing Tauri commands (`get_gem_knowledge`, `get_gem_knowledge_assembled`, `regenerate_gem_knowledge`, `check_knowledge_availability`)
- Knowledge files live at `~/Library/Application Support/com.jarvis.app/knowledge/{gem_id}/`

**Spec:** `.kiro/specs/gem-knowledge-viewer/requirements.md` (Requirements 1–2)
**Design:** `.kiro/specs/gem-knowledge-viewer/design.md`
**Tasks:** `.kiro/specs/gem-knowledge-viewer/tasks.md` (Tasks 1–4)

---

## What This Phase Produces

```
src-tauri/src/knowledge/commands.rs    ← MODIFY — add get_gem_knowledge_subfile command
src-tauri/src/lib.rs                   ← MODIFY — register new command in generate_handler!

src/state/types.ts                     ← MODIFY — add KnowledgeSubfile, KnowledgeEntry interfaces
src/components/GemDetailPanel.tsx      ← MODIFY — add file tree section, new prop
```

---

## Part A: Backend Command (Task 1)

### Add `get_gem_knowledge_subfile` to `src/knowledge/commands.rs`

Add this command after the existing `check_knowledge_availability` (line 41):

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

This follows the exact same pattern as the existing commands — `State<'_, Arc<dyn KnowledgeStore>>`, async, returns `Result`.

### Register in `lib.rs`

Add to the `generate_handler![]` macro, after line 340 (`check_knowledge_availability`):

```rust
knowledge::commands::get_gem_knowledge_subfile,
```

### Verify

Run `cargo build` — should compile with no new errors.

---

## Part B: TypeScript Types (Task 2)

### Add to `src/state/types.ts`

Add these interfaces at the end of the file, before the closing (after the `CoPilotCardStackState` interface around line 806):

```typescript
/**
 * Knowledge file types
 *
 * These types match the Rust KnowledgeEntry and KnowledgeSubfile structs
 * from src-tauri/src/knowledge/store.rs
 */

/** Subfile metadata matching Rust KnowledgeSubfile struct */
export interface KnowledgeSubfile {
  /** Filename (e.g., "gem.md", "content.md", "enrichment.md") */
  filename: string;

  /** Whether the file exists on disk */
  exists: boolean;

  /** File size in bytes */
  size_bytes: number;

  /** ISO 8601 timestamp of last modification (null if file doesn't exist) */
  last_modified: string | null;
}

/** Knowledge entry matching Rust KnowledgeEntry struct */
export interface KnowledgeEntry {
  /** Gem ID this knowledge entry belongs to */
  gem_id: string;

  /** Full assembled gem.md content */
  assembled: string;

  /** List of all known subfiles with existence and size info */
  subfiles: KnowledgeSubfile[];

  /** Knowledge format version */
  version: number;

  /** ISO 8601 timestamp of last assembly */
  last_assembled: string;
}
```

---

## Part C: GemDetailPanel File Tree (Tasks 3–4)

### Existing Component Structure

`src/components/GemDetailPanel.tsx` currently:
- Props: `gemId`, `onDelete`, `onTranscribe`, `onEnrich`, `aiAvailable`
- State: `gem`, `loading`, `error`
- Layout: header → metadata → copilot → tags → summary → transcript → **actions** (bottom)
- Fetches gem via `invoke('get_gem', { id: gemId })` in useEffect

### Changes to Make

#### 1. Add new prop

```typescript
interface GemDetailPanelProps {
  gemId: string;
  onDelete: () => void;
  onTranscribe: () => void;
  onEnrich: () => void;
  aiAvailable: boolean;
  onOpenKnowledgeFile: (filename: string) => void;  // ← NEW
}
```

Update the destructuring:

```typescript
export default function GemDetailPanel({
  gemId,
  onDelete,
  onTranscribe,
  onEnrich,
  aiAvailable,
  onOpenKnowledgeFile  // ← NEW
}: GemDetailPanelProps) {
```

#### 2. Add knowledge state

After the existing state declarations (line 34):

```typescript
const [knowledgeEntry, setKnowledgeEntry] = useState<KnowledgeEntry | null>(null);
```

Import `KnowledgeEntry` from types:

```typescript
import { Gem, KnowledgeEntry } from '../state/types';
```

#### 3. Add knowledge fetch

Add a `loadKnowledge` function after `loadGem` (line 51):

```typescript
const loadKnowledge = async () => {
  try {
    const entry = await invoke<KnowledgeEntry | null>('get_gem_knowledge', { gemId });
    setKnowledgeEntry(entry);
  } catch {
    // Silent fail — knowledge viewer is optional, don't break the detail panel
    setKnowledgeEntry(null);
  }
};
```

Update the `useEffect` to call both (line 36-38):

```typescript
useEffect(() => {
  loadGem();
  loadKnowledge();
}, [gemId]);
```

#### 4. Add formatFileSize helper

Add after `formatDate` (line 55):

```typescript
const formatFileSize = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
};
```

#### 5. Define file display order

Add after helpers:

```typescript
// Display order for knowledge files (gem.md last since it's the assembled output)
const KNOWLEDGE_FILE_ORDER = ['content.md', 'enrichment.md', 'transcript.md', 'copilot.md', 'gem.md'];
```

#### 6. Add file tree section to the JSX

Insert this **between the transcript section (line 236) and the gem-actions div (line 238)**:

```tsx
{/* Knowledge Files */}
{knowledgeEntry && (() => {
  const existingFiles = knowledgeEntry.subfiles
    .filter(s => s.exists && s.filename !== 'meta.json')
    .sort((a, b) => {
      const aIdx = KNOWLEDGE_FILE_ORDER.indexOf(a.filename);
      const bIdx = KNOWLEDGE_FILE_ORDER.indexOf(b.filename);
      return (aIdx === -1 ? 999 : aIdx) - (bIdx === -1 ? 999 : bIdx);
    });

  if (existingFiles.length === 0) {
    return (
      <div className="knowledge-file-tree">
        <h4>Knowledge Files</h4>
        <div className="no-knowledge-files">
          <span>No knowledge files</span>
          <button
            onClick={async () => {
              try {
                await invoke('regenerate_gem_knowledge', { gemId });
                loadKnowledge();
              } catch (e) {
                console.error('Failed to generate knowledge files:', e);
              }
            }}
            className="action-button"
          >
            Generate
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="knowledge-file-tree">
      <h4>Knowledge Files</h4>
      {existingFiles.map(subfile => (
        <div
          key={subfile.filename}
          className="knowledge-file-row"
          onClick={() => onOpenKnowledgeFile(subfile.filename)}
        >
          <span className="file-icon">📄</span>
          <span className="file-name">{subfile.filename}</span>
          <span className="file-size">{formatFileSize(subfile.size_bytes)}</span>
        </div>
      ))}
    </div>
  );
})()}
```

**Important:** The file tree goes between the transcript and actions — it's part of the gem's information, not an action.

#### 7. Add CSS to `src/App.css`

Add these styles (near the existing gem detail styles, after the `.gem-actions` section):

```css
/* Knowledge File Tree */
.knowledge-file-tree {
  margin-top: 16px;
  padding-top: 12px;
  border-top: 1px solid var(--border-subtle);
}

.knowledge-file-tree h4 {
  font-size: 13px;
  font-weight: var(--font-medium);
  color: var(--text-secondary);
  margin: 0 0 8px 0;
}

.knowledge-file-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background var(--duration-fast) var(--ease-out);
}

.knowledge-file-row:hover {
  background: var(--bg-hover);
}

.knowledge-file-row .file-icon {
  font-size: 14px;
  flex-shrink: 0;
}

.knowledge-file-row .file-name {
  flex: 1;
  font-family: 'SF Mono', Menlo, Monaco, 'Courier New', monospace;
  font-size: 12px;
  color: var(--text-primary);
}

.knowledge-file-row .file-size {
  color: var(--text-secondary);
  font-size: 11px;
  flex-shrink: 0;
}

.no-knowledge-files {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px;
  color: var(--text-secondary);
  font-size: 13px;
}
```

---

## Where to Update RightPanel (for now — just pass the prop)

In `src/components/RightPanel.tsx`, the gem section (lines 265-277) renders `GemDetailPanel`. For now, pass a no-op callback so the component compiles. The actual tab system is Phase 3.

At line 269-275 where `GemDetailPanel` is rendered:

```tsx
<GemDetailPanel
  gemId={selectedGemId}
  onDelete={onDeleteGem}
  onTranscribe={onTranscribeGem}
  onEnrich={onEnrichGem}
  aiAvailable={aiAvailable}
  onOpenKnowledgeFile={() => {}}  // ← ADD THIS — wired in Phase 3
/>
```

**Do the same** at any other place `GemDetailPanel` is rendered in `RightPanel.tsx` (search for `<GemDetailPanel` — there should be only one instance in the gems section).

---

## Verification Checklist

- [ ] `cargo build` succeeds — `get_gem_knowledge_subfile` command compiles
- [ ] Command is registered in `lib.rs` `generate_handler![]`
- [ ] `KnowledgeSubfile` and `KnowledgeEntry` interfaces added to `types.ts`
- [ ] `GemDetailPanel` has new `onOpenKnowledgeFile` prop
- [ ] `GemDetailPanel` fetches knowledge entry on mount (parallel with gem fetch)
- [ ] Knowledge file tree renders between transcript and actions
- [ ] Files are sorted: content.md, enrichment.md, transcript.md, copilot.md, gem.md
- [ ] `meta.json` is excluded from the file tree
- [ ] Each row shows: file icon, filename (monospace), size (human-readable)
- [ ] Rows are clickable (fires `onOpenKnowledgeFile` callback)
- [ ] If no knowledge files exist: shows "No knowledge files" + "Generate" button
- [ ] "Generate" button calls `regenerate_gem_knowledge` then refreshes the file tree
- [ ] If knowledge fetch fails: file tree section is hidden entirely (no error shown)
- [ ] `RightPanel` passes `onOpenKnowledgeFile={() => {}}` placeholder to `GemDetailPanel`
- [ ] `npm run dev` (or equivalent) starts without TypeScript errors
- [ ] App builds and starts without errors

---

## Important Notes

- **Knowledge fetch is fire-and-forget.** If `get_gem_knowledge` fails (e.g., no knowledge files migrated yet), just set `knowledgeEntry` to `null` and hide the section. Don't show an error.
- **File tree goes between transcript and actions.** Not at the top, not in a separate panel — it's additional gem detail information.
- **`onOpenKnowledgeFile` is a no-op for now.** Phase 3 will wire it into the RightPanel tab system. For this phase, just make sure the callback prop exists and clicking files doesn't crash.
- **File order matters.** `gem.md` goes last because it's the assembled output. The source files (`content.md`, `enrichment.md`, etc.) come first since they're the building blocks.
- **Use existing CSS variables.** The app uses `var(--bg-hover)`, `var(--text-secondary)`, `var(--border-subtle)`, `var(--radius-md)`, etc. Use these instead of hardcoded values.
- **The `KnowledgeEntry` returned by `get_gem_knowledge` includes ALL known subfiles** (both existing and non-existing). That's why you filter with `.filter(s => s.exists && s.filename !== 'meta.json')`.
