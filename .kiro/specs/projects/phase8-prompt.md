# Phase 8: CSS Styling for Projects Feature

## Context

You are working on a Tauri 2.x desktop app (Rust backend + React/TypeScript frontend). The Projects feature (Phases 1–7) is fully functional. This phase adds proper CSS styling, replacing inline `style={{}}` attributes with CSS classes and adding new rules to `App.css`.

**Tasks from tasks.md**: Tasks 17, 18, and 19.

## Goal

Add all CSS classes for the Projects feature to `jarvis-app/src/App.css`, then remove inline styles from the two component files where CSS classes now handle the styling.

## Important: Use Existing Design Tokens

The app uses design tokens defined in `:root` in `App.css`. **You MUST use these tokens** — NOT hardcoded color values or the `var(--some-name, #fallback)` pattern with fallbacks.

Here are the key tokens to use:

### Backgrounds
- `--bg-base: #0a0a0c` — app background
- `--bg-surface: #111114` — panel/card surfaces
- `--bg-elevated: #18181b` — elevated elements
- `--bg-hover: #1e1e22` — hover state
- `--bg-active: #27272a` — active/selected state

### Borders
- `--border-subtle: #1e1e22` — light borders
- `--border-default: #27272a` — normal borders
- `--border-strong: #3f3f46` — prominent borders

### Text
- `--text-primary: #fafafa` — headings, main text
- `--text-secondary: #a1a1aa` — secondary/muted text
- `--text-tertiary: #71717a` — least prominent text

### Accents & Semantics
- `--accent-primary: #6366f1` — primary accent (indigo)
- `--accent-hover: #818cf8`
- `--accent-subtle: rgba(99, 102, 241, 0.12)`
- `--success: #22c55e`, `--success-subtle: rgba(34, 197, 94, 0.1)`
- `--warning: #f59e0b`, `--warning-subtle: rgba(245, 158, 11, 0.1)`
- `--error: #ef4444`, `--error-subtle: rgba(239, 68, 68, 0.1)`
- `--info: #3b82f6`, `--info-subtle: rgba(59, 130, 246, 0.1)`
- `--overlay-dark: rgba(0, 0, 0, 0.6)`

### Typography
- `--text-xs: 0.75rem` (12px), `--text-sm: 0.8125rem` (13px), `--text-base: 0.875rem` (14px), `--text-lg: 1rem` (16px), `--text-xl: 1.125rem` (18px)
- `--font-normal: 400`, `--font-medium: 500`, `--font-semibold: 600`

### Spacing (4px grid)
- `--space-1: 4px`, `--space-2: 8px`, `--space-3: 12px`, `--space-4: 16px`, `--space-5: 20px`, `--space-6: 24px`

### Radii
- `--radius-sm: 4px`, `--radius-md: 6px`, `--radius-lg: 8px`

### Transitions
- `--duration-fast: 100ms`, `--duration-normal: 150ms`
- `--ease-out` (exists as a token)

---

## Part 1: Add CSS to `App.css`

Append the following CSS block to the **end** of `jarvis-app/src/App.css`. Use the design tokens listed above (replace any hardcoded values with token references).

```css
/* ===========================
   PROJECTS FEATURE STYLES
   =========================== */

/* Projects Container — Split Layout */
.projects-container {
  display: flex;
  flex-direction: row;
  height: 100%;
}

/* Project List — Left Side (260px fixed) */
.project-list {
  width: 260px;
  flex-shrink: 0;
  border-right: 1px solid var(--border-default);
  display: flex;
  flex-direction: column;
  overflow-y: auto;
}

.project-list-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-4);
  border-bottom: 1px solid var(--border-default);
}

.project-list-header h3 {
  margin: 0;
  font-size: var(--text-lg);
  font-weight: var(--font-semibold);
  color: var(--text-primary);
}

.project-list-items {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-2);
}

/* Project Card */
.project-card {
  padding: var(--space-3);
  border-radius: var(--radius-md);
  cursor: pointer;
  margin-bottom: var(--space-1);
  transition: background var(--duration-fast) var(--ease-out);
}

.project-card:hover {
  background: var(--bg-hover);
}

.project-card.active {
  background: var(--accent-subtle);
  border-left: 3px solid var(--accent-primary);
}

.project-card-title {
  font-weight: var(--font-semibold);
  font-size: var(--text-base);
  color: var(--text-primary);
  margin-bottom: var(--space-1);
}

.project-card-meta {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}

.project-card-desc {
  font-size: var(--text-xs);
  color: var(--text-secondary);
  margin-top: var(--space-1);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

/* Status Badges */
.status-badge {
  display: inline-block;
  padding: 2px var(--space-2);
  border-radius: 10px;
  font-size: 11px;
  font-weight: var(--font-semibold);
  text-transform: capitalize;
}

.status-active  { background: var(--success-subtle); color: var(--success); }
.status-paused  { background: var(--warning-subtle); color: var(--warning); }
.status-completed { background: var(--info-subtle); color: var(--info); }
.status-archived  { background: rgba(107, 114, 128, 0.15); color: #9ca3af; }

/* Project Gem List — Right Side */
.project-gem-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
}

.project-gem-list.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-tertiary);
  font-size: var(--text-base);
}

.project-gem-list.loading {
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-tertiary);
}

/* Project Metadata Header */
.project-metadata-header {
  padding: var(--space-4);
  border-bottom: 1px solid var(--border-default);
}

.project-metadata-header h2 {
  margin: 0 0 var(--space-2) 0;
  font-size: var(--text-xl);
  font-weight: var(--font-semibold);
  color: var(--text-primary);
}

.project-meta-row {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  font-size: var(--text-sm);
  color: var(--text-secondary);
}

.project-objective {
  margin-top: var(--space-2);
  font-size: var(--text-sm);
  color: var(--text-secondary);
  font-style: italic;
}

.project-description {
  margin-top: var(--space-1);
  font-size: var(--text-sm);
  color: var(--text-secondary);
}

/* Inline Edit Form */
.project-edit-form input[type="text"],
.project-edit-form textarea,
.project-edit-form select {
  width: 100%;
  padding: var(--space-2);
  margin-bottom: var(--space-2);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--text-sm);
  font-family: inherit;
}

.project-edit-form input[type="text"] {
  font-size: var(--text-lg);
  font-weight: var(--font-semibold);
}

.project-edit-form textarea {
  resize: vertical;
}

.project-edit-form select {
  width: auto;
}

.project-edit-form .edit-status-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-bottom: var(--space-2);
}

.project-edit-form .edit-status-row label {
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}

.project-edit-form .edit-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
}

.project-edit-form .error-state {
  margin-bottom: var(--space-2);
  font-size: var(--text-xs);
}

/* Delete Confirmation Bar */
.delete-confirm-bar {
  padding: var(--space-3) var(--space-4);
  background-color: var(--error-subtle);
  border-bottom: 1px solid var(--border-default);
  display: flex;
  align-items: center;
  gap: var(--space-3);
  font-size: var(--text-sm);
  color: var(--text-primary);
}

/* Project Gem Toolbar */
.project-gem-toolbar {
  display: flex;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border-default);
}

.project-gem-toolbar .gems-search-input {
  flex: 1;
}

/* Project Gems List (scrollable area) */
.project-gems-list {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-2) var(--space-4);
}

/* Project Gem Card (wrapper around GemCard with remove button) */
.project-gem-card {
  position: relative;
  margin-bottom: var(--space-2);
}

.project-gem-card .gem-card {
  cursor: pointer;
}

.remove-from-project {
  position: absolute;
  top: var(--space-2);
  right: var(--space-2);
  background: none;
  border: none;
  color: var(--text-tertiary);
  font-size: var(--text-xl);
  cursor: pointer;
  opacity: 0;
  transition: opacity var(--duration-fast) var(--ease-out);
  padding: var(--space-1);
  line-height: 1;
}

.project-gem-card:hover .remove-from-project {
  opacity: 1;
}

.remove-from-project:hover {
  color: var(--error);
}

/* Create Project Form */
.create-project-form {
  padding: var(--space-3);
  border-bottom: 1px solid var(--border-default);
  background: var(--bg-surface);
}

.create-project-form input,
.create-project-form textarea {
  width: 100%;
  margin-bottom: var(--space-2);
  padding: var(--space-2);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: var(--bg-elevated);
  color: var(--text-primary);
  font-size: var(--text-sm);
  font-family: inherit;
  box-sizing: border-box;
}

.create-project-form textarea {
  resize: vertical;
  min-height: 60px;
}

.create-project-form .form-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
}

/* Modal Overlay */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: var(--overlay-dark);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-card {
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-lg);
  width: 560px;
  max-height: 70vh;
  display: flex;
  flex-direction: column;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-4);
  border-bottom: 1px solid var(--border-default);
}

.modal-header h3 {
  margin: 0;
  font-size: var(--text-lg);
  font-weight: var(--font-semibold);
  color: var(--text-primary);
}

.modal-search {
  padding: var(--space-3) var(--space-4);
}

.modal-gem-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 var(--space-4);
}

.modal-gem-list .empty-state {
  padding: var(--space-4);
  text-align: center;
  color: var(--text-tertiary);
  font-size: var(--text-sm);
}

.modal-gem-row {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: 10px var(--space-2);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background var(--duration-fast) var(--ease-out);
}

.modal-gem-row:hover {
  background: var(--bg-hover);
}

.modal-gem-row.selected {
  background: var(--accent-subtle);
}

.modal-gem-row.disabled {
  opacity: 0.5;
  cursor: default;
}

.modal-gem-row .modal-gem-info {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.modal-gem-row .modal-gem-title {
  font-size: var(--text-sm);
  color: var(--text-primary);
}

.modal-gem-row .modal-gem-meta {
  font-size: 11px;
  color: var(--text-tertiary);
}

.modal-gem-row .already-added-label {
  font-size: 11px;
  color: var(--text-tertiary);
  font-style: italic;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border-top: 1px solid var(--border-default);
}

/* Action Button Variants (used across projects) */
.action-button.small {
  padding: var(--space-1) var(--space-2);
  font-size: var(--text-xs);
}

.action-button.danger {
  color: var(--error);
  border-color: var(--error-border);
}

.action-button.danger:hover {
  background: var(--error-subtle);
}

.action-button.secondary {
  color: var(--text-secondary);
  border-color: var(--border-default);
}

/* Project Dropdown (on GemCard in GemsPanel) */
.project-dropdown-container {
  position: relative;
  display: inline-block;
}

.project-dropdown {
  position: absolute;
  bottom: 100%;
  right: 0;
  margin-bottom: var(--space-1);
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  min-width: 220px;
  max-height: 240px;
  overflow-y: auto;
  z-index: 100;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

.project-dropdown-header {
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--border-default);
  font-size: var(--text-xs);
  font-weight: var(--font-semibold);
  color: var(--text-tertiary);
}

.project-dropdown-empty {
  padding: var(--space-3);
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}

.project-dropdown-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  cursor: pointer;
  font-size: var(--text-sm);
  transition: background var(--duration-fast) var(--ease-out);
}

.project-dropdown-item:hover {
  background: var(--bg-hover);
}

.project-dropdown-item .project-gem-count {
  font-size: 11px;
  color: var(--text-tertiary);
}

/* Scrollbar for project lists */
.project-list-items::-webkit-scrollbar,
.project-gems-list::-webkit-scrollbar,
.modal-gem-list::-webkit-scrollbar,
.project-dropdown::-webkit-scrollbar {
  width: 8px;
}

.project-list-items::-webkit-scrollbar-track,
.project-gems-list::-webkit-scrollbar-track,
.modal-gem-list::-webkit-scrollbar-track,
.project-dropdown::-webkit-scrollbar-track {
  background: var(--bg-elevated);
  border-radius: var(--radius-sm);
}

.project-list-items::-webkit-scrollbar-thumb,
.project-gems-list::-webkit-scrollbar-thumb,
.modal-gem-list::-webkit-scrollbar-thumb,
.project-dropdown::-webkit-scrollbar-thumb {
  background: var(--accent-primary);
  border-radius: var(--radius-sm);
}

.project-list-items::-webkit-scrollbar-thumb:hover,
.project-gems-list::-webkit-scrollbar-thumb:hover,
.modal-gem-list::-webkit-scrollbar-thumb:hover,
.project-dropdown::-webkit-scrollbar-thumb:hover {
  background: var(--accent-hover);
}
```

---

## Part 2: Remove Inline Styles from `ProjectsContainer.tsx`

After adding the CSS, go through `jarvis-app/src/components/ProjectsContainer.tsx` and remove inline `style={{}}` attributes where CSS classes now handle the styling. Here's the mapping of what to change:

### CreateProjectForm (lines ~67)
- The `.error-state` div already has a CSS class. Remove `style={{ marginBottom: '8px', fontSize: '12px' }}` — the CSS handles it.

### AddGemsModal (lines ~176–267)
The modal currently has many inline styles. Replace them with CSS classes:

1. **`.modal-card`** (line ~178): Remove the entire `style={{...}}` — the CSS `.modal-card` class handles it.
2. **`.modal-header`** (line ~187): Remove `style={{...}}` — CSS `.modal-header` handles it.
3. **`<h3>` inside modal-header** (line ~191): Remove `style={{ margin: 0 }}` — CSS `.modal-header h3` handles it.
4. **`.modal-search`** (line ~194): Remove `style={{ padding: '12px 16px' }}` — CSS handles it.
5. **`.modal-gem-list`** (line ~204): Remove `style={{ flex: 1, overflowY: 'auto', padding: '0 16px' }}` — CSS handles it.
6. **Empty/loading state divs** (lines ~206, ~211): Replace inline styles with the class `className="empty-state"`. The CSS `.modal-gem-list .empty-state` handles it.
7. **`.modal-gem-row`** items (line ~223): Remove the `style={{...}}` — CSS `.modal-gem-row` handles it.
8. **Gem info inside modal rows** (lines ~235–238): Wrap title and meta in a `<div className="modal-gem-info">` and add `className="modal-gem-title"` / `className="modal-gem-meta"` to inner spans. Remove inline styles.
9. **Already added label** (line ~242): Add `className="already-added-label"`, remove inline style.
10. **`.modal-footer`** (line ~250): Remove `style={{...}}` — CSS handles it.

### ProjectList (lines ~270–335)
11. **Loading state** (line ~306): Remove `style={{ padding: '16px', textAlign: 'center', color: 'var(--text-muted, #666)' }}` — the existing `.loading-state` CSS class covers it.
12. **Empty state** (line ~311): Remove `style={{...}}` — the existing `.empty-state` CSS class covers it.

### ProjectGemList (lines ~338–713)
13. **Loading state** (line ~475): Remove the `style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--text-muted, #666)' }}` — the `.project-gem-list.loading` CSS class covers this. Use `className="project-gem-list loading"`.
14. **Edit form inputs** (lines ~497–550): Remove ALL `style={{...}}` from the `<input>`, `<select>`, and `<textarea>` elements inside the edit form. The CSS `.project-edit-form input[type="text"]`, `.project-edit-form textarea`, `.project-edit-form select` handle all styling.
15. **Status row** (line ~506): Replace the `<div style={{...}}>` wrapping the status label+select with `<div className="edit-status-row">`.
16. **Status label** (line ~507): Remove `style={{ fontSize: '12px', color: 'var(--text-muted, #888)' }}`.
17. **Edit action buttons** (line ~556): Replace `<div style={{...}}>` with `<div className="edit-actions">`.
18. **Error state in edit form** (line ~552): Remove inline style — CSS `.project-edit-form .error-state` handles it.
19. **Project description** (line ~593): Remove `style={{ marginTop: '4px', fontSize: '13px', color: 'var(--text-secondary, #aaa)' }}` — CSS `.project-description` handles it.
20. **Delete confirmation bar** (line ~603): Remove `style={{...}}` — CSS `.delete-confirm-bar` handles it.
21. **Project gems list** (line ~640): Remove `style={{ flex: 1, overflowY: 'auto', padding: '8px 16px' }}` — CSS `.project-gems-list` handles it.
22. **Empty gems state** (line ~642): Remove inline style — `.empty-state` CSS handles it.
23. **Project gem card wrapper** (line ~649): Remove `style={{ position: 'relative', marginBottom: '8px' }}` — CSS `.project-gem-card` handles it.
24. **Gem card click** (line ~653): Remove `style={{ cursor: 'pointer' }}` — CSS `.project-gem-card .gem-card` handles it.
25. **Remove button** (lines ~684): Remove `style={{...}}` — CSS `.remove-from-project` handles it.

---

## Part 3: Remove Inline Styles from `GemsPanel.tsx`

Go through the GemCard project dropdown section (lines ~398–460) in `jarvis-app/src/components/GemsPanel.tsx`:

1. **`.project-dropdown-container`** (line ~398): Remove `style={{ position: 'relative', display: 'inline-block' }}` — CSS handles it.
2. **`.project-dropdown`** (line ~408): Remove the entire `style={{...}}` — CSS handles it.
3. **Dropdown header "Projects"** (lines ~422–428): Replace the `<div style={{...}}>` with `<div className="project-dropdown-header">`. Remove inline style.
4. **Empty projects message** (line ~432): Replace `<div style={{...}}>` with `<div className="project-dropdown-empty">`. Remove inline style.
5. **Project items in dropdown** (lines ~437–456): Replace `<div style={{...}}>` with `<div className="project-dropdown-item">`. Remove inline style. Replace the gem count `<span style={{...}}>` with `<span className="project-gem-count">`.

---

## What NOT to Do

- Do NOT change any component logic, state management, or event handlers
- Do NOT add new features, components, or props
- Do NOT modify any Rust backend code
- Do NOT change the design token values in `:root`
- Do NOT remove any existing CSS rules that other components use
- Do NOT add `!important` to any rules
- Keep the inline `style={{ pointerEvents: 'none' }}` on checkboxes — that's functional, not decorative

## Build Verification

After making changes, verify:
```bash
cd jarvis-app && npm run build
```

The build must pass with zero errors.

---

## Reference: Files to Modify

1. **`jarvis-app/src/App.css`** — Append CSS block at end of file
2. **`jarvis-app/src/components/ProjectsContainer.tsx`** — Remove ~25 inline styles
3. **`jarvis-app/src/components/GemsPanel.tsx`** — Remove ~5 inline styles from project dropdown section

## Reference: Current Inline Style Count

- `ProjectsContainer.tsx` has ~32 inline `style={{}}` attributes
- `GemsPanel.tsx` has ~6 inline `style={{}}` in the project dropdown section
- Target: reduce to only functional styles (like `pointerEvents: 'none'` on checkboxes)
