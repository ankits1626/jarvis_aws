# Phase 0: Project Scaffolding & App Shell

> Foundation phase. Everything else builds on this.

---

## Goal

A running Vite + React + TypeScript + Tailwind app with the global layout in place: dark theme, collapsible sidebar, content area, and navigation that switches between the 5 parts. No actual content yet — just the skeleton.

---

## Tasks

### 0.1 — Initialize project
- Scaffold Vite + React + TypeScript project in `jarvis-architecture-visualizer/`
- Install Tailwind CSS 4 with `@tailwindcss/vite` plugin
- Configure `vite.config.ts` (dev port, React plugin, Tailwind plugin)
- Configure `tsconfig.json` (strict mode, path aliases)
- Verify `npm run dev` shows blank React app

### 0.2 — Global styles & dark theme
- Set up `index.css` with Tailwind imports
- Define dark theme: `slate-950` background, `slate-100` text
- Define the color palette CSS variables for all 6 layers + 8 guides
- Add `fadeIn` keyframe animation
- Add smooth transition utilities for expand/collapse

### 0.3 — App shell layout
- Build `App.tsx` with two-column layout: sidebar (fixed) + content area (scrollable)
- Sidebar width: 280px expanded, 64px collapsed
- Content area: max-width 1200px, centered, padded

### 0.4 — Sidebar navigation
- Build `Sidebar.tsx` with collapsible toggle
- Navigation sections:
  - **Part 1**: The Big Picture (single item)
  - **Part 2**: Layer Explorer (6 sub-items, one per layer)
  - **Part 3**: Data Flows (3 sub-items)
  - **Part 4**: Spec Map (single item)
  - **Part 5**: Tech Stack 101 (8 sub-items, one per guide)
- Active item highlighting with layer-appropriate color
- Collapsed state shows only icons

### 0.5 — Navigation state & routing
- Implement navigation state with `useState` (no router library)
- Define `NavigationItem` type: `{ part: number, section?: string }`
- Content area renders correct component based on active nav item
- Placeholder components for all 5 parts (just the title + "Coming soon")

### 0.6 — Shared component stubs
- Create empty stub files for all shared components:
  - `FlowArrow.tsx`, `CodeSnippet.tsx`, `NodeCard.tsx`
  - `Tooltip.tsx`, `StatusBadge.tsx`
  - `InteractiveCode.tsx`, `QuizBlock.tsx`
- Each exports a placeholder `<div>` with component name

---

## Expected Output

```
npm run dev → opens browser at localhost:5173
```

What you see:
- Dark slate background, full screen
- Left sidebar with 5 sections, expandable sub-items
- Clicking any nav item shows its placeholder in the content area
- Sidebar collapses to icon-only on toggle
- Smooth transitions on collapse/expand and nav item switching

### Files Created
```
jarvis-architecture-visualizer/
├── index.html
├── package.json
├── vite.config.ts
├── tsconfig.json
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── index.css
│   ├── components/
│   │   ├── Sidebar.tsx
│   │   └── shared/
│   │       ├── FlowArrow.tsx
│   │       ├── CodeSnippet.tsx
│   │       ├── NodeCard.tsx
│   │       ├── Tooltip.tsx
│   │       ├── StatusBadge.tsx
│   │       ├── InteractiveCode.tsx
│   │       └── QuizBlock.tsx
```

---

## Definition of Done
- [ ] `npm run dev` works without errors
- [ ] Dark theme renders correctly
- [ ] Sidebar shows all 5 parts with correct sub-items
- [ ] Sidebar collapse/expand works with smooth animation
- [ ] Clicking any nav item switches the content area
- [ ] No TypeScript errors, no console warnings
