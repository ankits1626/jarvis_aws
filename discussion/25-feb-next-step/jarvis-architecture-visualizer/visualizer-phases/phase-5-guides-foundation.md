# Phase 5: Tech Stack 101 — Foundation & Shared Components

> Build the guide system before writing individual guides.

---

## Goal

A reusable guide framework: shared layout, section navigation, interactive code blocks, quiz/exercise blocks, concept cards, and progress tracking. Once this foundation exists, each individual guide is just data + composition.

---

## Tasks

### 5.1 — Guide data types
- Create `src/data/guides/types.ts`
- Define shared types:
  ```typescript
  GuideSection {
    id: string
    title: string
    content: GuideSectionContent[]  // ordered content blocks
  }

  GuideSectionContent =
    | { type: 'text', body: string }
    | { type: 'code', language: string, code: string, caption?: string }
    | { type: 'interactive-code', language: string, starterCode: string, solution: string, hint: string, validator: (input: string) => boolean }
    | { type: 'concept-card', term: string, explanation: string, example?: string }
    | { type: 'comparison', leftLabel: string, rightLabel: string, rows: { left: string, right: string, label: string }[] }
    | { type: 'diagram', nodes: DiagramNode[], connections: DiagramConnection[] }
    | { type: 'quiz', question: string, options: string[], correctIndex: number, explanation: string }
    | { type: 'exercise', title: string, description: string, hints: string[], solution: string }

  Guide {
    id: string
    title: string
    subtitle: string
    color: string           // Tailwind color prefix
    icon: string            // Emoji
    sections: GuideSection[]
    jarvisConnections: { concept: string, file: string, description: string }[]
  }
  ```

### 5.2 — GuideShell component
- Build `GuideShell.tsx` — shared layout for all 8 guides
- Layout:
  - Left: mini table-of-contents (section titles, scrollspy active indicator)
  - Center: content area rendering section blocks
  - Section headers with guide color accent
- Section-to-section smooth scroll
- "How Jarvis uses it" footer section (auto-generated from `jarvisConnections`)
- Progress indicator: sections visited (stored in localStorage)

### 5.3 — InteractiveCode component
- Implement `InteractiveCode.tsx`
- Editable `<textarea>` styled as a code editor (monospace, dark background)
- "Run" button → calls validator function → shows pass/fail
- "Hint" button → reveals hint text
- "Show Solution" button → reveals solution code
- Language label in top-right corner
- No actual compilation — validation is string-matching or regex

### 5.4 — QuizBlock component
- Implement `QuizBlock.tsx`
- Multiple-choice question with 3-4 options
- Click an option → green (correct) or red (incorrect) with shake animation
- After answering: shows explanation text
- Tracks completion in localStorage

### 5.5 — ConceptCard component
- Build `ConceptCard.tsx`
- Flip-card style:
  - Front: term name (large text) + "Click to reveal"
  - Back: explanation + optional code example
- Smooth 3D flip animation (CSS perspective + rotateY)
- Can be arranged in a grid (2-3 per row)

### 5.6 — ComparisonTable component
- Build `ComparisonTable.tsx`
- Two-column table with:
  - Left header (e.g., "Rust")
  - Right header (e.g., "Swift")
  - Rows with label + left value + right value
- Color-coded headers matching the guide color
- Hover row highlights both sides

### 5.7 — DiagramBlock component
- Build `DiagramBlock.tsx`
- Simple node-and-arrow renderer (reuses `FlowArrow` logic)
- Nodes positioned via CSS grid coordinates
- Used for inline diagrams within guide sections
- Clickable nodes for additional detail

### 5.8 — Progress tracking
- `useGuideProgress` hook
- Stores in localStorage: `{ [guideId]: { sectionsVisited: string[], quizzesCompleted: string[], exercisesCompleted: string[] } }`
- Sidebar shows completion percentage per guide
- Section-level: dot indicator (visited = filled, not visited = empty)

---

## Expected Output

What you see when clicking any guide in the sidebar:
- Guide header: colored bar with icon, title, subtitle
- Mini table-of-contents on the left with section links
- Content sections rendering mixed content blocks:
  - Text paragraphs
  - Code snippets (read-only)
  - Interactive code editors (editable, with Run/Hint/Solution)
  - Concept flip cards in grids
  - Comparison tables
  - Quiz blocks
  - Mini diagrams
- "How Jarvis uses it" section at the bottom
- Progress dots in the sidebar

### Interactions
- Scroll through sections, TOC highlights current section
- Click concept card → flips to reveal explanation
- Type in interactive code → click Run → see pass/fail
- Answer quiz → see correct/wrong + explanation
- Progress auto-saves to localStorage

### Files Created/Modified
```
src/data/guides/types.ts                  (NEW)
src/components/guides/GuideShell.tsx       (NEW)
src/components/shared/InteractiveCode.tsx  (IMPLEMENT)
src/components/shared/QuizBlock.tsx        (IMPLEMENT)
src/components/shared/ConceptCard.tsx      (NEW)
src/components/shared/ComparisonTable.tsx  (NEW)
src/components/shared/DiagramBlock.tsx     (NEW)
src/hooks/useGuideProgress.ts             (NEW)
```

---

## Definition of Done
- [ ] GuideShell renders with TOC, content area, and footer
- [ ] All 7 content block types render correctly
- [ ] InteractiveCode: edit, run, hint, solution all work
- [ ] QuizBlock: select answer, see feedback, tracks completion
- [ ] ConceptCard: flip animation works smoothly
- [ ] ComparisonTable: renders two-column data with hover
- [ ] Progress persists in localStorage across page reloads
- [ ] Sidebar shows guide completion percentages
- [ ] Tested with mock guide data (dummy content)
