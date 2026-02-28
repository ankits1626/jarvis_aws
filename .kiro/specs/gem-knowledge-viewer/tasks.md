# Gem Knowledge Viewer — Tasks

## Phase 1: Backend Command (Requirement 1)

### Task 1: Add `get_gem_knowledge_subfile` Tauri command
- [ ] Add `get_gem_knowledge_subfile` function to `src/knowledge/commands.rs`
  - Accepts `gem_id: String`, `filename: String`, `knowledge_store: State<'_, Arc<dyn KnowledgeStore>>`
  - Calls `knowledge_store.get_subfile(&gem_id, &filename).await`
  - Returns `Result<Option<String>, String>`
- [ ] Register command in `lib.rs` `generate_handler![]` macro
- [ ] Verify `cargo build` succeeds

---

## Phase 2: TypeScript Types + GemDetailPanel File Tree (Requirement 2)

### Task 2: Add TypeScript types for KnowledgeEntry
- [ ] Add `KnowledgeSubfile` interface to `src/state/types.ts`: `filename`, `exists`, `size_bytes`, `last_modified`
- [ ] Add `KnowledgeEntry` interface to `src/state/types.ts`: `gem_id`, `assembled`, `subfiles`, `version`, `last_assembled`
- [ ] Add `formatFileSize(bytes: number): string` helper (can live in a utils file or inline in GemDetailPanel)

### Task 3: Add knowledge file tree to GemDetailPanel
- [ ] Add `onOpenKnowledgeFile: (filename: string) => void` to `GemDetailPanelProps`
- [ ] Add `knowledgeEntry` state (`useState<KnowledgeEntry | null>(null)`)
- [ ] Add `loadKnowledge()` function that calls `invoke('get_gem_knowledge', { gemId })` — silent catch on failure (set null)
- [ ] Call `loadKnowledge()` in the existing `useEffect` alongside `loadGem()`, re-fetch when `gemId` changes
- [ ] Render "Knowledge Files" section after the transcript section, before the action buttons:
  - Filter subfiles: `exists === true` and `filename !== 'meta.json'`
  - Order: `content.md`, `enrichment.md`, `transcript.md`, `copilot.md`, `gem.md` (gem.md last)
  - Each row: file icon, filename (monospace), size (muted)
  - Click handler calls `onOpenKnowledgeFile(filename)`
- [ ] Handle empty state: if no subfiles exist (or only meta.json), show "No knowledge files" with a "Generate" button that calls `invoke('regenerate_gem_knowledge', { gemId })` then re-fetches
- [ ] If `knowledgeEntry` is null (fetch failed), hide the section entirely

### Task 4: Verify Phase 2
- [ ] GemDetailPanel renders file tree when knowledge files exist
- [ ] File tree is hidden when knowledge files don't exist or fetch fails
- [ ] Clicking a file row does not crash (callback fires, even though RightPanel doesn't handle it yet)
- [ ] "Generate" button works for gems without knowledge files

---

## Phase 3: RightPanel Tabbed View (Requirements 3, 4)

### Task 5: Add tab state to RightPanel
- [ ] Add state variables to RightPanel:
  - `openKnowledgeFiles: string[]` (initially `[]`)
  - `activeGemTab: 'detail' | string` (initially `'detail'`)
  - `knowledgeFileContents: Record<string, string>` (initially `{}`)
- [ ] Add `useEffect` that resets all three states when `selectedGemId` changes
- [ ] Add `handleOpenKnowledgeFile(filename: string)` handler:
  - Add filename to `openKnowledgeFiles` if not already present
  - Set `activeGemTab` to the filename
  - Fetch content via `invoke('get_gem_knowledge_subfile', { gemId: selectedGemId, filename })` if not cached
  - Store in `knowledgeFileContents` (or store error message on failure)
- [ ] Add `handleCloseTab(filename: string)` handler:
  - Remove filename from `openKnowledgeFiles`
  - If closing the active tab, switch to last remaining tab or `'detail'`
  - Remove from `knowledgeFileContents` cache

### Task 6: Render tabbed view in gems section
- [ ] Replace the gems section in RightPanel (currently lines ~265-287) with conditional logic:
  - **If `openKnowledgeFiles.length > 0`**: render tab bar + content area
    - Tab bar: "Detail" tab (always first) + one tab per open file
    - Each file tab has filename label + close button (`×`)
    - Close button uses `e.stopPropagation()` to prevent switching to the tab
    - Content area: render `GemDetailPanel` when `activeGemTab === 'detail'`, otherwise render file content viewer
  - **If `openKnowledgeFiles.length === 0`**: render `GemDetailPanel` directly (current behavior, no tab bar)
- [ ] Pass `onOpenKnowledgeFile={handleOpenKnowledgeFile}` prop to `GemDetailPanel` in both paths
- [ ] File content viewer: scrollable `<pre>` with monospace font showing `knowledgeFileContents[activeGemTab]`
- [ ] Loading state: if content not yet in cache, show "Loading {filename}..."
- [ ] Tab bar reuses existing `.tab-buttons` / `.tab-button` / `.tab-button.active` CSS classes

### Task 7: Verify Phase 3
- [ ] Clicking a file in the tree opens a tab in RightPanel with file content
- [ ] Tab bar shows "Detail" + open file tabs
- [ ] Clicking "Detail" tab shows the gem detail panel
- [ ] Clicking a file tab shows that file's content
- [ ] Clicking X on a file tab closes it
- [ ] Closing all file tabs returns to single-panel mode (no tab bar)
- [ ] Switching gems resets all tabs
- [ ] Clicking the same file twice does not create duplicate tabs
- [ ] Content is cached — switching between tabs doesn't re-fetch

---

## Phase 4: CSS Styling (Requirement 5)

### Task 8: Add CSS for file tree
- [ ] Add `.knowledge-file-tree` styles to `App.css`:
  - Top border separator, section heading
- [ ] Add `.knowledge-file-row` styles:
  - Flex layout (icon, filename, size)
  - Hover background effect
  - Cursor pointer
  - Filename in monospace, size in muted color
- [ ] Add `.no-knowledge-files` styles for the empty state

### Task 9: Add CSS for tab close button and file viewer
- [ ] Add `.tab-button .tab-close` styles:
  - Small `×`, margin-left, reduced opacity, hover full opacity
- [ ] Add `.knowledge-file-viewer` styles:
  - Full height, scrollable overflow
  - Padding matching existing panels
- [ ] Add `.knowledge-file-content` styles:
  - Monospace font (`'SF Mono', Menlo, monospace`)
  - `white-space: pre-wrap`, `word-wrap: break-word`
  - Line height, font size matching transcript display
- [ ] Add `.knowledge-file-viewer.loading` styles for centered loading text

### Task 10: Verify Phase 4
- [ ] File tree looks consistent with other gem detail sections
- [ ] File rows highlight on hover
- [ ] Tab close buttons are visible and clickable
- [ ] File content is readable with proper monospace font
- [ ] Scrolling works for long files
- [ ] Styling matches the existing transcript and recording tab visuals

---

## Phase 5: Polish + Edge Cases

### Task 11: Handle edge cases
- [ ] Verify: gem with no knowledge files shows "No knowledge files" + "Generate" button
- [ ] Verify: "Generate" button calls `regenerate_gem_knowledge`, then refreshes the file tree
- [ ] Verify: if `get_gem_knowledge_subfile` returns `null`, tab shows "File not found"
- [ ] Verify: if `get_gem_knowledge_subfile` errors, tab shows error message
- [ ] Verify: knowledge file tree hidden when `get_gem_knowledge` call fails entirely
- [ ] Verify: no console errors or React warnings during normal usage

### Task 12: End-to-end verification
- [ ] Save a new gem → verify knowledge files appear in file tree
- [ ] Enrich a gem → verify enrichment.md appears/updates in file tree
- [ ] Open gem.md tab → verify assembled document displays correctly
- [ ] Open multiple file tabs → switch between them → verify content is correct
- [ ] Delete a gem → verify no errors (tabs close if open)
- [ ] App builds and starts without errors
