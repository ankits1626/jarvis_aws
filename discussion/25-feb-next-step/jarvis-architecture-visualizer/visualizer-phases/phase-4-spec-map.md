# Phase 4: Spec Map (Part 4)

> Visual map of all 13 Kiro specs — what's built, what's planned.

---

## Goal

An interactive grid/map showing all 13 Kiro specifications, their implementation status, relationships, and connections to the codebase. Click any spec to see its requirements summary, which files it touches, and its evolution history.

---

## Tasks

### 4.1 — Spec data file
- Create `src/data/specs.ts`
- Define types: `KiroSpec`, `SpecStatus`, `SpecRelation`
- Populate all 13 specs with:
  - `id`: folder name (e.g., `jarvis-app`)
  - `name`: display name
  - `status`: `implemented` | `in-progress` | `planned`
  - `description`: one-sentence summary
  - `layer`: which architecture layer it belongs to
  - `requirementsSummary`: 3-5 bullet points from `requirements.md`
  - `keyFiles`: array of file paths this spec touches
  - `relatedSpecs`: which specs it depends on or evolved from
  - `specPath`: path to the Kiro spec folder

### 4.2 — StatusBadge component
- Implement `StatusBadge.tsx`
- Three states:
  - `implemented` — green badge, checkmark icon
  - `in-progress` — yellow badge, spinner/dots icon
  - `planned` — gray badge, clock icon
- Consistent size, rounded pill shape

### 4.3 — SpecMap component
- Build `SpecMap.tsx`
- **Grid view**: 13 spec cards in a responsive grid (3-4 columns)
- Each card shows:
  - Spec name
  - Status badge
  - Layer color indicator (left border)
  - One-line description
  - Click to expand

### 4.4 — Spec detail panel
- Expanded spec view (replaces grid or slides in from right):
  - Full spec name + status badge
  - Layer tag (colored chip)
  - Requirements summary (bullet list from Kiro spec)
  - Key files list (clickable — shows tooltip with file purpose)
  - Related specs (clickable links to other specs)
  - Evolution arrow if applicable (e.g., "Evolved from jarvis-browser-vision")

### 4.5 — Spec relationship lines
- In grid view, show faint lines connecting related specs
- Evolution chains: `jarvis-browser-vision` → `jarvis-browser-vision-v2`
- Dependency links: `jarvis-gems` → `intelligence-kit-integration`
- Lines use the layer color of the source spec

### 4.6 — Filter controls
- Filter by status: All | Implemented | In-Progress | Planned
- Filter by layer: All | Frontend | Audio | Browser | Intelligence | Gems | Settings
- Active filter highlighted, cards fade in/out with filter changes

---

## Expected Output

### The 13 Specs (with status)

| Spec | Status | Layer |
|------|--------|-------|
| jarvis-app | Implemented | Core |
| jarvis-listen | Implemented | Audio |
| jarvis-transcribe | Implemented | Audio |
| jarvis-settings | Implemented | Settings |
| jarvis-browser-vision | Implemented | Browser |
| jarvis-browser-vision-v2 | Implemented | Browser |
| jarvis-gems | Implemented | Gems |
| jarvis-medium-extractor | Implemented | Browser |
| jarvis-claude-extension-extractor | Implemented | Browser |
| intelligence-kit | Implemented | Intelligence |
| intelligence-kit-integration | Implemented | Intelligence |
| jarvis-whisperkit | In-Progress | Audio |
| mlx-intelligence-provider | Planned | Intelligence |

### Interactions
- Click a spec card → detail panel expands
- Click related spec → navigates to that spec
- Filter by status or layer → cards animate in/out
- Hover a spec → faint lines show its relationships

### Files Created/Modified
```
src/data/specs.ts                      (NEW)
src/components/SpecMap.tsx              (REPLACE placeholder)
src/components/shared/StatusBadge.tsx   (IMPLEMENT)
```

---

## Definition of Done
- [ ] All 13 specs render in the grid with correct status badges
- [ ] Clicking a spec shows its detail panel with requirements summary
- [ ] Key files listed for each spec are accurate
- [ ] Related specs are clickable and navigate correctly
- [ ] Evolution chains are visible (browser-vision → v2)
- [ ] Filters work: by status and by layer
- [ ] Relationship lines render between connected specs
- [ ] Grid is responsive (3 cols on wide, 2 on medium)
