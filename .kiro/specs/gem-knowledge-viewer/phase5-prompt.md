# Prompt: Gem Knowledge Viewer — Phase 5 (Polish + Edge Cases + End-to-End Verification)

## Your Task

Complete **Phase 5: Polish + Edge Cases** (Tasks 11–12) from `.kiro/specs/gem-knowledge-viewer/tasks.md`. This is the final phase — verify that all edge cases are handled correctly and do a full end-to-end walkthrough.

**After completing this, stop and report your findings.**

If you encounter any bugs or issues, fix them and describe what you changed.

---

## Context

Phases 1–3 are complete (Phase 4 CSS was delivered with Phase 1):
- `get_gem_knowledge_subfile` Tauri command works
- `KnowledgeEntry` and `KnowledgeSubfile` TypeScript types exist
- `GemDetailPanel.tsx` shows a file tree with clickable rows, "Generate" button for empty state
- `RightPanel.tsx` has full tabbed viewer — Detail tab + file tabs with close buttons
- All CSS is in place (file tree, tab close, file viewer, loading state)

**Files involved:**
- `src/components/RightPanel.tsx` — tabbed viewer with handlers
- `src/components/GemDetailPanel.tsx` — file tree + knowledge fetch
- `src-tauri/src/knowledge/commands.rs` — `get_gem_knowledge_subfile` command
- `src/App.css` — all styling

---

## Task 11: Edge Case Verification + Fixes

Run through each scenario below. For each one, verify the behavior is correct. If anything is broken, fix it.

### 11a: Gem with no knowledge files

1. Find or create a gem that has no knowledge directory (or an empty one)
2. Open its detail panel
3. **Expected:** "Knowledge Files" section shows "No knowledge files" text + "Generate" button
4. **Expected:** No console errors or crashes

### 11b: "Generate" button

1. On a gem with no knowledge files, click "Generate"
2. **Expected:** Calls `regenerate_gem_knowledge` command, then refreshes the file tree
3. **Expected:** After generation completes, the file tree shows the newly created files
4. Check `GemDetailPanel.tsx` line 67-74 — the `handleRegenerate` function calls `invoke('regenerate_gem_knowledge', { gemId })` then `loadKnowledge()`

### 11c: File not found (subfile returns null)

1. Open a knowledge file tab
2. **Expected:** If `get_gem_knowledge_subfile` returns `null`, the tab shows "File not found"
3. Check `RightPanel.tsx` line 133 — `content ?? 'File not found'`

### 11d: File fetch error

1. If `get_gem_knowledge_subfile` throws an error
2. **Expected:** The tab shows "Error loading file: {error message}" (not a crash)
3. Check `RightPanel.tsx` lines 135-139

### 11e: Knowledge fetch fails entirely

1. If `get_gem_knowledge` (the metadata fetch in GemDetailPanel) fails
2. **Expected:** `knowledgeEntry` is `null`, the Knowledge Files section is hidden entirely
3. **Expected:** No error shown, no crash — the rest of the gem detail renders normally
4. Check `GemDetailPanel.tsx` lines 57-65 — silent catch sets `null`

### 11f: No console errors during normal usage

1. Open a gem with knowledge files
2. Click a file to open a tab
3. Switch between Detail and file tabs
4. Close a file tab
5. Open multiple file tabs
6. Switch between them
7. Close all tabs
8. Switch to a different gem
9. **Expected:** No `console.error`, no React warnings, no unhandled promise rejections in the dev tools console

### 11g: TypeScript compilation

1. Run `npx tsc --noEmit` (or `npm run build` if available)
2. **Expected:** No TypeScript errors

---

## Task 12: End-to-End Verification

Walk through the complete user flow and verify each step works.

### 12a: Full flow — gem with existing knowledge files

1. Select a gem that already has knowledge files generated
2. **Verify:** File tree appears in the detail panel showing the `.md` files
3. **Verify:** Files are sorted: content.md, enrichment.md, transcript.md, copilot.md, gem.md
4. **Verify:** Each row shows: 📄 icon, filename (monospace), size (e.g., "2.3 KB")
5. **Verify:** meta.json is NOT shown in the file tree
6. Click on `gem.md` in the file tree
7. **Verify:** Tab bar appears with "Detail" and "gem.md ×" tabs
8. **Verify:** gem.md tab is active, showing assembled markdown content in monospace `<pre>`
9. Click on `content.md` in the tree (click "Detail" tab first to see the tree again)
10. **Verify:** Tab bar now shows: "Detail", "gem.md ×", "content.md ×"
11. **Verify:** content.md tab is active with its content
12. Click the "gem.md" tab
13. **Verify:** Switches to gem.md content (cached, no re-fetch — check Network tab)
14. Click "Detail" tab
15. **Verify:** GemDetailPanel with file tree is shown again
16. Click × on "content.md" tab
17. **Verify:** content.md tab disappears, still on Detail tab (since Detail was active)
18. Click × on "gem.md" tab
19. **Verify:** All file tabs closed, tab bar disappears, back to single-panel mode

### 12b: Switching gems resets tabs

1. Open gem A, click a file to open a tab
2. Select gem B from the left panel
3. **Verify:** Tab bar disappears, gem B's detail panel shows
4. **Verify:** No leftover tabs from gem A
5. Go back to gem A
6. **Verify:** Starts fresh — single-panel mode, no tabs

### 12c: Duplicate prevention

1. Open a gem, click "gem.md" to open its tab
2. Switch to "Detail" tab
3. Click "gem.md" again in the file tree
4. **Verify:** Switches to the existing gem.md tab (doesn't create a duplicate)
5. **Verify:** Tab bar still shows exactly: "Detail", "gem.md ×"

### 12d: Close active tab fallback

1. Open two file tabs: gem.md and content.md
2. Make sure content.md is the active tab
3. Click × on content.md
4. **Verify:** Switches to gem.md (last remaining file tab)
5. Click × on gem.md
6. **Verify:** Switches to Detail, tab bar disappears (single-panel mode)

### 12e: Build verification

1. Run `npm run dev` (or the project's dev command)
2. **Verify:** App starts without TypeScript errors
3. **Verify:** No console errors on startup
4. **Verify:** Knowledge viewer works end-to-end as described above

---

## If You Find Issues

If any edge case or verification step fails:

1. Describe what's wrong
2. Fix it in the code
3. Explain the fix
4. Re-verify that the fix works

Common things to watch for:
- React state update warnings (calling setState during render)
- Stale closure bugs in handlers (referencing old state values)
- Missing `key` props on mapped elements
- Unhandled promise rejections from `invoke` calls
- CSS overflow issues with very long file content

---

## Summary of What Should Work When Done

```
✅ Gem with knowledge files → file tree visible, sorted, clickable
✅ Gem without knowledge files → "No knowledge files" + Generate button
✅ Generate button → creates files, refreshes tree
✅ Click file → opens tab, fetches content, displays in monospace
✅ Multiple tabs → switch between them, content cached
✅ Close tab → removes tab, falls back correctly
✅ Close all tabs → returns to single-panel mode
✅ Switch gems → resets all tab state
✅ File not found → shows "File not found" in tab
✅ Fetch error → shows error message in tab
✅ Knowledge fetch fails → file tree hidden, no crash
✅ No TypeScript errors
✅ No console errors during normal usage
```
