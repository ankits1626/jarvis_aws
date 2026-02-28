# Phase 4 Implementation Prompt — TypeScript Types + Navigation Integration

## What to Implement

Implement **Phase 4** from `.kiro/specs/projects/tasks.md` — Tasks 8 and 9. This adds the TypeScript interfaces for Projects and wires the "Projects" nav item into the left navigation and center panel routing. After this phase, clicking "Projects" in the nav shows an empty placeholder component — the actual ProjectsContainer with content is Phase 5.

## Context Files (Read These First)

1. **Design doc** (has TypeScript types and LeftNav/App.tsx changes):
   `.kiro/specs/projects/design.md` — Sections: "TypeScript Types", "LeftNav.tsx Changes", "App.tsx Changes"

2. **TypeScript types file** (add new types here):
   `jarvis-app/src/state/types.ts` — Read the file to see existing patterns (`GemPreview`, `Settings`, etc.)

3. **LeftNav component** (add 'projects' nav item):
   `jarvis-app/src/components/LeftNav.tsx` — Full file, 65 lines

4. **App.tsx** (add center panel routing + right panel logic):
   `jarvis-app/src/App.tsx` — Key areas:
   - Line 18: `type ActiveNav = 'record' | 'recordings' | 'gems' | 'youtube' | 'browser' | 'settings';`
   - Line 60: `const showRightPanel = activeNav === 'record' || activeNav === 'recordings' || activeNav === 'gems';`
   - Lines 824-826: gems center panel rendering pattern
   - Lines 846-850: RightPanel rendering with selectedGemId

5. **Requirements spec**:
   `.kiro/specs/projects/requirements.md` — Requirements 6 (TypeScript types) and 7 (left nav)

## Tasks

### Task 8: Add TypeScript types to `types.ts`

Add the following interfaces to `jarvis-app/src/state/types.ts`, after the existing `SearchSettings` interface (around line 244):

```typescript
/** Full project representation matching Rust Project struct */
export interface Project {
  id: string;
  title: string;
  description: string | null;
  objective: string | null;
  status: 'active' | 'paused' | 'completed' | 'archived';
  created_at: string;
  updated_at: string;
}

/** Lightweight project for list views matching Rust ProjectPreview struct */
export interface ProjectPreview {
  id: string;
  title: string;
  description: string | null;
  status: string;
  gem_count: number;
  updated_at: string;
}

/** Full project with associated gems matching Rust ProjectDetail struct */
export interface ProjectDetail {
  project: Project;
  gem_count: number;
  gems: GemPreview[];
}
```

**Notes:**
- `Project.status` uses a union type for type safety on the frontend
- `ProjectPreview.status` is `string` (simpler for list rendering, avoids narrowing issues)
- `ProjectDetail.gems` uses `GemPreview[]` — the existing `GemPreview` interface already defined in the same file
- All field names are snake_case, matching the Rust struct serialization via serde

### Task 9: Add Projects to navigation

#### 9a. Update `LeftNav.tsx`

**Update the `ActiveNav` type** (line 1):
```typescript
type ActiveNav = 'record' | 'recordings' | 'gems' | 'projects' | 'youtube' | 'browser' | 'settings';
```

**Add the nav item** to the `navItems` array, after gems (line 21, before youtube):
```typescript
{ id: 'projects', label: 'Projects', icon: '📁' },
```

The final `navItems` array should be:
```typescript
const navItems: Array<{ id: ActiveNav; label: string; icon: string }> = [
  { id: 'record', label: 'Record', icon: '🎙️' },
  { id: 'recordings', label: 'Recordings', icon: '📼' },
  { id: 'gems', label: 'Gems', icon: '💎' },
  { id: 'projects', label: 'Projects', icon: '📁' },
  { id: 'youtube', label: 'YouTube', icon: '📺' },
  { id: 'browser', label: 'Browser', icon: '🌐' }
];
```

#### 9b. Update `App.tsx`

**Three changes needed:**

1. **Update `ActiveNav` type** (line 18) — add `'projects'`:
```typescript
type ActiveNav = 'record' | 'recordings' | 'gems' | 'projects' | 'youtube' | 'browser' | 'settings';
```

2. **Update `showRightPanel`** (line 60) — add `'projects'`:
```typescript
const showRightPanel = activeNav === 'record' || activeNav === 'recordings' || activeNav === 'gems' || activeNav === 'projects';
```

3. **Add center panel rendering** — after the gems section (after line 826), add:
```tsx
{activeNav === 'projects' && (
  <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--text-muted, #666)' }}>
    Projects — coming in Phase 5
  </div>
)}
```

This is a **temporary placeholder**. Phase 5 will replace it with the actual `<ProjectsContainer>` component. The placeholder ensures:
- Navigation works end-to-end
- The right panel shows when Projects is selected (for gem detail when we wire it)
- No import errors from components that don't exist yet

**Do NOT** import `ProjectsContainer` yet — it doesn't exist and will cause a build error.

## Verification

After implementing, verify the frontend builds:
```bash
cd jarvis-app && npm run build
```

Also verify the Rust backend still compiles (in case of accidental changes):
```bash
cd jarvis-app/src-tauri && cargo check
```

Both must pass.

**Things to check manually (if running the app):**
- Projects nav item appears between Gems and YouTube in the left nav
- Clicking Projects highlights the nav item and shows the placeholder in the center panel
- Right panel area is visible when Projects is active
- Clicking other nav items still works correctly

## What NOT to Do

- Do NOT create `ProjectsContainer.tsx` or any project components — that's Phase 5
- Do NOT import any project components in App.tsx — they don't exist yet
- Do NOT modify `RightPanel.tsx` — the existing gem detail panel will work for projects too (same `selectedGemId` flow), handled in Phase 6
- Do NOT modify `GemsPanel.tsx` — Phase 7 adds the "Add to Project" dropdown
- Do NOT add CSS — Phase 8 handles all styling

## After Implementation

Once `npm run build` passes:
1. Show me the changes to `types.ts`, `LeftNav.tsx`, and `App.tsx`
2. I'll review before we proceed to Phase 5

**If you have any confusion or questions — about the ActiveNav type being defined in two places (LeftNav.tsx and App.tsx), the placeholder approach, or anything else — please ask before guessing.**
