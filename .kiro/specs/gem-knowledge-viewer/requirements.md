# Gem Knowledge Viewer — File Tree + Tabbed Viewer in Gem Detail

## Introduction

Jarvis generates knowledge files for each gem (`knowledge/{gem_id}/*.md`) but there is no way to see them in the UI. The gem detail panel (`GemDetailPanel.tsx`) shows structured data inline (tags, summary, transcript, copilot sections) from the Gem struct, but never the actual markdown files that agents and search consume.

This spec adds two things: (1) a **knowledge file tree** at the bottom of GemDetailPanel listing the `.md` files that exist for that gem, and (2) a **tabbed viewer** in RightPanel that opens clicked files in tabs beside the gem detail — following the exact same tab pattern used by recordings (Details + Chat tabs).

The goal is simple visibility: "what knowledge files exist for this gem, and what's in them?"

**Reference:** Design doc at `discussion/28-feb-next-step/gem-knowledge-viewer.md`. Depends on: [Gem Knowledge Files spec](../gem-knowledge-files/requirements.md) (fully implemented — `KnowledgeStore` trait, `LocalKnowledgeStore`, Tauri commands, lifecycle wiring all complete).

## Glossary

- **Knowledge File Tree**: A section in `GemDetailPanel` that lists the `.md` subfiles present in the gem's knowledge folder. Each row shows filename and size. Rows are clickable.
- **Knowledge File Tab**: A tab in `RightPanel` that displays the raw content of a single `.md` knowledge file. Tabs appear alongside the "Detail" tab (which is the existing `GemDetailPanel`).
- **Detail Tab**: The default tab showing the current `GemDetailPanel` content (metadata, tags, summary, transcript, actions). Always present.
- **Subfile**: An individual knowledge file — `gem.md`, `content.md`, `enrichment.md`, `transcript.md`, `copilot.md`. Defined in the gem-knowledge-files spec.
- **KnowledgeEntry**: The data returned by `get_gem_knowledge` — contains `subfiles` (metadata array with filename, exists, size_bytes) and `assembled` (gem.md content). Already implemented.

## Frozen Design Decisions

These decisions were made during design review (2026-02-28):

1. **File tree lives in GemDetailPanel**: The knowledge file list is shown at the bottom of the gem detail panel, above the action buttons. It is part of the detail view, not a separate component.
2. **Tabs live in RightPanel**: When a file is clicked, RightPanel switches from single-panel mode to tabbed mode. This follows the same pattern as recordings getting "Details" + "Chat" tabs when a chat session starts.
3. **"Detail" tab is always default**: The tab system starts on Detail. Opening a file adds a new tab. Closing all file tabs returns to single-panel mode.
4. **Raw markdown display**: File content is shown as preformatted monospace text (like transcript display), not rendered markdown. Developers and power users prefer seeing the actual `.md` source.
5. **Lazy loading**: File content is fetched on tab click, not upfront. Only subfile metadata (filename, exists, size) is loaded with the gem.
6. **Closeable file tabs**: Each file tab has an X button. Closing the last file tab returns to single-panel mode (no tab bar).
7. **meta.json excluded**: The file tree only shows `.md` files, not `meta.json` (which is machine-readable JSON, not useful for human viewing).
8. **No editing**: The viewer is read-only. Knowledge files are derived from the database and managed by the backend.

---

## Requirement 1: New Tauri Command — get_gem_knowledge_subfile

**User Story:** As the frontend, I need a Tauri command to read the content of a specific knowledge subfile, so I can display it when the user clicks a file in the tree.

### Acceptance Criteria

1. THE System SHALL expose a `get_gem_knowledge_subfile` Tauri command in `src/knowledge/commands.rs`
2. THE command SHALL accept parameters: `gem_id: String`, `filename: String`
3. THE command SHALL call `knowledge_store.get_subfile(&gem_id, &filename)` and return `Result<Option<String>, String>`
4. THE command SHALL use `State<'_, Arc<dyn KnowledgeStore>>` for dependency injection, following the existing pattern in `knowledge/commands.rs`
5. THE command SHALL be registered in `lib.rs` in the `generate_handler!` macro alongside the existing knowledge commands
6. THE command SHALL perform no filename validation beyond what `KnowledgeStore::get_subfile()` already does (path traversal is prevented by the store using `base_path.join(gem_id).join(filename)`)

---

## Requirement 2: Knowledge File Tree in GemDetailPanel

**User Story:** As a user viewing a gem's details, I want to see which knowledge files exist for that gem and their sizes, so I can understand what data has been generated and click to inspect any file.

### Acceptance Criteria

1. THE `GemDetailPanel` component SHALL fetch knowledge entry metadata when mounting, by calling `invoke('get_gem_knowledge', { gemId })` alongside the existing `get_gem` call
2. THE component SHALL display a "Knowledge Files" section after the transcript section and before the action buttons
3. THE section SHALL show a list of subfiles where `exists === true`, excluding `meta.json`
4. EACH row SHALL display:
   - A file icon (plain text `📄` or CSS-styled)
   - The filename (e.g., `gem.md`, `content.md`, `enrichment.md`)
   - The file size formatted in human-readable units (e.g., `4.2 KB`, `1.5 KB`)
5. EACH row SHALL be clickable, calling the `onOpenKnowledgeFile(filename: string)` callback prop
6. IF no knowledge files exist for the gem, the section SHALL show a message "No knowledge files" with a "Generate" button that calls `invoke('regenerate_gem_knowledge', { gemId })`
7. IF the knowledge entry fetch fails, the section SHALL be hidden (not shown at all) — knowledge file viewing is optional and should not break the detail panel
8. THE component SHALL accept a new prop: `onOpenKnowledgeFile: (filename: string) => void`
9. THE file tree SHALL display files in the same order as `KNOWN_SUBFILES` in the backend: `content.md`, `enrichment.md`, `transcript.md`, `copilot.md`, `gem.md` — with `gem.md` last since it is the assembled output
10. THE knowledge entry SHALL be re-fetched when `gemId` changes (same useEffect pattern as the existing gem fetch)

---

## Requirement 3: Tabbed View in RightPanel for Gems

**User Story:** As a user, when I click a knowledge file in the gem detail tree, I want it to open in a new tab beside the detail view — so I can switch between the gem's structured detail and its raw knowledge files.

### Acceptance Criteria

1. THE `RightPanel` component (gems section, currently lines 265-287) SHALL support a tabbed view when knowledge files are open
2. WHEN no knowledge files are open, the gems section SHALL render `GemDetailPanel` directly with no tab bar (current behavior preserved)
3. WHEN one or more knowledge files are open, the gems section SHALL render:
   - A tab bar with a "Detail" tab (always first) and one tab per open file
   - A content area showing the active tab's content
4. THE "Detail" tab SHALL render the existing `GemDetailPanel` component (unchanged)
5. FILE tabs SHALL display the filename as the tab label (e.g., "gem.md", "enrichment.md")
6. FILE tabs SHALL include a close button (X) that removes the tab
7. CLICKING a file in the tree that is already open SHALL switch to that tab (not open a duplicate)
8. CLICKING a file in the tree that is not yet open SHALL add a new tab and switch to it
9. CLOSING the last file tab SHALL return to single-panel mode (no tab bar, Detail view only)
10. THE tab bar SHALL use the same CSS classes as the existing recording tabs: `.tab-buttons`, `.tab-button`, `.tab-button.active`
11. THE `RightPanel` SHALL maintain the following state for the gems tabbed view:
    - `openKnowledgeFiles: string[]` — ordered list of open filenames
    - `activeGemTab: 'detail' | string` — currently active tab ('detail' or a filename)
    - `knowledgeFileContents: Record<string, string>` — cached file contents (fetched once per file)
12. WHEN `selectedGemId` changes, ALL file tabs SHALL be closed and state reset to single-panel Detail view
13. THE `RightPanel` SHALL pass the `onOpenKnowledgeFile` callback to `GemDetailPanel`

---

## Requirement 4: Knowledge File Content Display

**User Story:** As a user viewing a knowledge file tab, I want to see the file's content in a readable, scrollable format, so I can inspect what the system has generated.

### Acceptance Criteria

1. WHEN a file tab becomes active, IF the content has not been fetched yet, THE system SHALL call `invoke('get_gem_knowledge_subfile', { gemId, filename })` to load it
2. WHILE content is loading, THE tab SHALL show a loading indicator (e.g., "Loading {filename}...")
3. ONCE loaded, THE content SHALL be cached in `knowledgeFileContents` state — subsequent switches to the same tab use the cache
4. THE content SHALL be displayed in a scrollable container with monospace font, matching the existing transcript display styling (`.transcript-text.scrollable`)
5. IF the subfile returns `null` (file was deleted between listing and reading), THE tab SHALL show "File not found" and the tab SHALL remain open (user can close it)
6. THE content display SHALL preserve whitespace and line breaks (preformatted text, `white-space: pre-wrap`)

---

## Requirement 5: CSS Styling

**User Story:** As a user, I want the knowledge file tree and tabbed viewer to look consistent with the rest of the app, so it feels like a native part of Jarvis.

### Acceptance Criteria

1. THE knowledge file tree section SHALL use a class `.knowledge-file-tree` with a subtle top border separator and section heading styled like other gem detail headings
2. EACH file row SHALL use a class `.knowledge-file-row` with:
   - Horizontal layout (icon, filename, size)
   - Hover effect (subtle background change)
   - Cursor pointer
   - Filename in regular weight, size in muted/lighter color
3. THE tab close button SHALL use a class `.tab-close` with:
   - Small "x" character or icon
   - Positioned to the right of the tab label
   - Hover effect for visibility
   - Clicking the close button SHALL NOT also switch to that tab
4. THE file content viewer SHALL use a class `.knowledge-file-viewer` matching the existing `.transcript-text.scrollable` styling:
   - Monospace font (`font-family: 'SF Mono', Menlo, monospace`)
   - Scrollable with max-height
   - Padding and background consistent with transcript display
5. ALL new CSS SHALL be added to `App.css` alongside the existing gem detail styles
6. THE tab bar SHALL reuse existing `.tab-buttons` and `.tab-button` classes without modification

---

## Technical Constraints

1. **React + TypeScript**: Frontend uses React functional components with hooks. No class components.
2. **Tauri invoke**: All backend calls use `invoke()` from `@tauri-apps/api/core`. Type the return values.
3. **Existing tab pattern**: Follow the recording tabs pattern exactly — same CSS classes, same conditional rendering approach.
4. **No new npm dependencies**: Display raw markdown as preformatted text. No `react-markdown` or similar needed.
5. **Backend already complete**: `KnowledgeStore::get_subfile()` exists and works. Only need a thin Tauri command wrapper.
6. **State lives in RightPanel**: Tab state (`openKnowledgeFiles`, `activeGemTab`, `knowledgeFileContents`) is managed by RightPanel, not GemDetailPanel. GemDetailPanel just fires the `onOpenKnowledgeFile` callback.
7. **KnowledgeEntry type**: Define a TypeScript interface matching the Rust struct — `{ gem_id: string, assembled: string, subfiles: Array<{ filename: string, exists: boolean, size_bytes: number, last_modified: string | null }>, version: number, last_assembled: string }`.

## Out of Scope

1. Rendered markdown (Markdown → HTML) — show raw `.md` source as preformatted text
2. Editing knowledge files from the UI — read-only viewer
3. File watching / auto-refresh when knowledge files update in the background
4. Drag-and-drop tab reordering
5. Side-by-side / split view of multiple files
6. Syntax highlighting for code blocks within markdown
7. Download / export knowledge files
8. Search within knowledge file content
