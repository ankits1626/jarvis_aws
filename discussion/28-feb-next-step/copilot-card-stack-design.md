# Co-Pilot Card Stack — Refined Design Specification

## Design Direction

**Option C from the UX analysis**: Card Stack (Fireflies-inspired), refined with animation, expand/collapse state management, persistent post-recording view, a terminal summary card, and a sticky cycle footer.

---

## Core Principles

1. **One insight per card** — not one category per card
2. **Cards animate in** — the user sees change happening, not a static wall
3. **Expand/collapse with visual state** — glanceable when collapsed, detailed when expanded
4. **Agent persists after recording** — no abrupt disappearance; the final state is the most valuable
5. **Sticky footer** — cycle status always visible, anchored to bottom

---

## Card Anatomy

Each card represents a single insight extracted during a cycle.

```
┌──────────────────────────────────────┐
│ ▸  Cyber Threats                  🔵 │  ← Header: chevron + title + type dot
│                                      │
│    Discussion about dark web         │  ← Body (visible when expanded)
│    exploits and why traditional      │
│    antivirus tools fall short.       │
│                                      │
│    Cycle 3 · 4:38 PM                 │  ← Card meta (visible when expanded)
└──────────────────────────────────────┘
```

### Card States

| State | Chevron | Body | Behavior |
|-------|---------|------|----------|
| **Collapsed** | `▸` (right) | Hidden | Only title + type dot visible. Single line. |
| **Expanded** | `▾` (down) | Visible | Full insight text + card metadata. |
| **New** (just appeared) | `▾` (down) | Visible | Appears expanded with entrance animation. Auto-collapses after 8s if user doesn't interact. |

### Card Types & Color Coding

| Type | Dot Color | Use |
|------|-----------|-----|
| Insight / Key Point | `#3B82F6` (blue) | Main observations from the conversation |
| Decision | `#22C55E` (green) | Explicit decisions made |
| Action Item | `#F59E0B` (amber) | Tasks or follow-ups identified |
| Question | `#EF4444` (red-orange) | Open questions raised |
| Summary Update | `#8B5CF6` (purple) | Running summary changed |

---

## Card Lifecycle

### Entrance Animation

When a new cycle completes and produces new insights:

1. New cards slide in from the top with a `translateY(-20px) → 0` + `opacity: 0 → 1` animation (300ms ease-out)
2. Existing cards shift down smoothly (200ms)
3. New cards appear **expanded** by default so the user sees the content
4. After **8 seconds** without interaction, new cards auto-collapse to just the title line (200ms ease)
5. If user clicks/hovers a card during the 8s window, it stays expanded (user took ownership)

### Collapse/Expand Animation

- Body height animates via `max-height` transition (200ms ease)
- Chevron rotates 90° (`▸` → `▾`) with a CSS transform (150ms)
- Smooth, not jarring

### Card Ordering

- **Newest cards at the top** (reverse chronological)
- Within a single cycle's output, order by priority: Decision > Action Item > Question > Insight > Summary Update
- Cards from older cycles stay below, collapsed by default

---

## Panel Layout

### During Recording (Live Mode)

```
┌─────────────────────────────────────────┐
│  Co-Pilot Agent                      ●  │  ← Panel header (● = recording indicator)
├─────────────────────────────────────────┤
│                                         │
│  ▾  Exploits sold on dark web     🔵    │  ← Newest card (expanded, just animated in)
│     Marketplaces operate openly         │
│     on Tor with escrow systems.         │
│     Cycle 3 · 4:38 PM                  │
│                                         │
│  ▸  Traditional AV insufficient   🟢    │  ← Older card (collapsed)
│                                         │
│  ▸  What alternatives exist?      🔴    │  ← Older card (collapsed)
│                                         │
│  ▸  Discussion shifted to cyber…  🟣    │  ← Summary update (collapsed)
│                                         │
│  ▸  Dark web scraping challenges  🔵    │  ← Oldest card (collapsed)
│                                         │
│                                         │
│                                         │  ← Scrollable area
│                                         │
├─────────────────────────────────────────┤
│  ● Cycle 4 in ~18s · 3 cycles done     │  ← Sticky footer
└─────────────────────────────────────────┘
```

### Key Layout Rules

- **Card area** is scrollable, scroll position resets to top when new cards arrive
- **Panel header** fixed at top
- **Sticky footer** fixed at bottom, always visible regardless of scroll
- Cards fill the space between header and footer

---

## Post-Recording: Persistent Agent View

### The Problem Being Solved

Currently, stopping the recording kills the Co-Pilot panel. But the moment recording stops is exactly when the user wants to review what happened. The agent view should **stay visible** and transition to a review state.

### Behavior After Recording Stops

1. Recording stops → the panel does **NOT** disappear
2. The recording indicator (`●`) in the header changes to a checkmark or "Complete" badge
3. A **Final Summary Card** animates in at the top as the last card
4. The sticky footer updates to show "Session complete · X cycles · Ym Zs analyzed"
5. All cards remain interactive (expand/collapse)
6. Panel stays until user navigates away or explicitly closes it

### The Final Summary Card

This is a special card that only appears when recording stops. It's visually distinct — slightly larger, with a subtle border or background tint.

```
┌─────────────────────────────────────────┐
│  ▾  Session Summary                 📋  │
│                                         │
│  Summary                                │
│  This conversation covered dark web     │
│  threats, limitations of traditional    │
│  antivirus software, and emerging       │
│  defense strategies including...        │
│                                         │
│  Key Takeaways                          │
│  • Exploit marketplaces operate with    │
│    escrow systems on Tor               │
│  • Signature-based AV misses 60% of    │
│    novel threats                        │
│  • Behavioral analysis is the leading  │
│    alternative approach                 │
│                                         │
│  Action Items                           │
│  ☐ Research behavioral analysis tools   │
│  ☐ Review current AV coverage gaps      │
│  ☐ Schedule follow-up on defense plan   │
│                                         │
│  Decisions                              │
│  ✓ Move away from signature-only AV     │
│  ✓ Prioritize endpoint detection        │
│                                         │
│  Open Questions                         │
│  ? Budget implications of new tooling   │
│  ? Timeline for migration               │
│                                         │
└─────────────────────────────────────────┘
```

### Final Summary Card Rules

- Always appears **expanded** and does **not** auto-collapse
- Rendered with a distinct visual treatment (subtle background, thicker left border, or card elevation)
- Contains all accumulated data: summary, key takeaways (key_points), action items, decisions, open questions
- Sections within the card only render if they have content (no empty headers)
- This card is essentially the "gem preview" — what will be saved

---

## Sticky Cycle Footer

### During Recording

```
┌─────────────────────────────────────────┐
│  ● Cycle 4 in ~18s · 3 cycles done     │
└─────────────────────────────────────────┘
```

- Pulsing dot when actively processing
- Countdown to next cycle (approximate, based on cycle interval)
- Total cycles completed

### While Processing

```
┌─────────────────────────────────────────┐
│  ◉ Processing cycle 4...               │
└─────────────────────────────────────────┘
```

- Animated spinner/pulse on the dot
- "Processing cycle N..." text

### After Recording Stops

```
┌─────────────────────────────────────────┐
│  ✓ Session complete · 5 cycles · 2m 40s │
└─────────────────────────────────────────┘
```

- Static checkmark
- Final stats: total cycles, total audio duration

### Footer CSS

```css
.copilot-footer {
  position: sticky;
  bottom: 0;
  background: var(--bg-secondary);
  border-top: 1px solid var(--border-color);
  padding: 8px 12px;
  font-size: 12px;
  z-index: 10;
}
```

---

## Interaction Details

### Click to Expand/Collapse
- Click anywhere on the card header row to toggle
- Chevron rotates as visual feedback
- Body slides open/closed

### Expand All / Collapse All
- Small text buttons in the panel header: "Expand All" | "Collapse All"
- Useful when user wants to review everything or minimize everything

### Card Dismissal (Future)
- Not in v1, but design for it: swipe left or X button to dismiss a card
- Dismissed cards don't reappear even if the same insight is re-emitted

---

## Data Flow: Cycle → Cards

Each Co-Pilot cycle produces a `CoPilotCycleResult`:

```typescript
interface CoPilotCycleResult {
  new_content: string;
  updated_summary: string;
  key_points: string[];
  decisions: string[];
  action_items: string[];
  open_questions: string[];
  suggested_questions: SuggestedQuestion[];
  key_concepts: KeyConcept[];
}
```

### Mapping Cycle Output → Cards

| Field | Card Type | When to Create Card |
|-------|-----------|-------------------|
| `key_points` (new items) | Insight (blue) | Each new key point = 1 card |
| `decisions` (new items) | Decision (green) | Each new decision = 1 card |
| `action_items` (new items) | Action Item (amber) | Each new action item = 1 card |
| `open_questions` (new items) | Question (red-orange) | Each new question = 1 card |
| `updated_summary` (if changed) | Summary Update (purple) | 1 card per cycle if summary changed |

### Deduplication

- Compare new cycle output against existing cards
- Only create cards for genuinely new items (not already shown)
- Use text similarity or exact match — same logic as current `CoPilotState` deduplication

### Card Data Structure

```typescript
interface CoPilotCard {
  id: string;              // unique, e.g. `cycle-3-keypoint-0`
  type: 'insight' | 'decision' | 'action_item' | 'question' | 'summary_update';
  title: string;           // short label (first ~60 chars or extracted topic)
  body: string;            // full insight text
  cycle: number;           // which cycle produced this
  timestamp: string;       // ISO timestamp
  isExpanded: boolean;     // UI state
  isNew: boolean;          // true until auto-collapse timer fires
}
```

### State Management

```typescript
interface CoPilotCardState {
  cards: CoPilotCard[];
  runningStatus: 'idle' | 'recording' | 'processing' | 'complete';
  currentCycle: number;
  nextCycleIn: number;      // seconds until next cycle
  totalAudioAnalyzed: number; // seconds
  finalSummaryCard: FinalSummaryCard | null; // only set after recording stops
}

interface FinalSummaryCard {
  summary: string;
  keyTakeaways: string[];
  actionItems: string[];
  decisions: string[];
  openQuestions: string[];
}
```

---

## CSS Animation Specs

### Card Entrance

```css
@keyframes cardSlideIn {
  from {
    opacity: 0;
    transform: translateY(-20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.copilot-card.entering {
  animation: cardSlideIn 300ms ease-out forwards;
}
```

### Card Body Expand/Collapse

```css
.copilot-card-body {
  max-height: 0;
  overflow: hidden;
  transition: max-height 200ms ease, padding 200ms ease;
}

.copilot-card.expanded .copilot-card-body {
  max-height: 300px; /* large enough for content */
  padding: 8px 12px;
}
```

### Chevron Rotation

```css
.copilot-card-chevron {
  transition: transform 150ms ease;
  display: inline-block;
}

.copilot-card.expanded .copilot-card-chevron {
  transform: rotate(90deg);
}
```

### New Card Pulse (subtle attention grab)

```css
@keyframes subtlePulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(59, 130, 246, 0); }
  50% { box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.15); }
}

.copilot-card.is-new {
  animation: subtlePulse 2s ease-in-out 2; /* pulse twice then stop */
}
```

---

## Migration from Current CoPilotPanel

### What Changes

| Current | Card Stack |
|---------|-----------|
| 7 sections always visible | Cards appear one at a time |
| Categories as headings | Card type as colored dot |
| No animation | Slide-in + auto-collapse |
| Disappears on recording stop | Persists with final summary |
| Status footer scrolls away | Sticky footer |
| All data same visual weight | Summary card visually distinct |

### What Stays the Same

- Same backend data (`CoPilotState`, `CoPilotCycleResult`)
- Same cycle timing and processing logic
- Same Tauri event system (`copilot://cycle-complete`)
- Same deduplication logic (reused, not rewritten)
- `GemDetailPanel` co-pilot section unchanged (post-save review)

### Implementation Approach

1. Create new `CoPilotCardStack.tsx` component alongside existing `CoPilotPanel.tsx`
2. Add `CoPilotCard` state management (can be local useState or a small reducer)
3. Transform incoming `CoPilotState` diffs into card operations (add new cards, update summary)
4. Wire up in `RightPanel.tsx` — swap `CoPilotPanel` for `CoPilotCardStack`
5. Add CSS animations in `App.css` or a dedicated `copilot-cards.css`
6. Handle recording stop: keep panel visible, inject final summary card
7. Make footer sticky with CSS

### Files to Create/Modify

| File | Action |
|------|--------|
| `src/components/CoPilotCardStack.tsx` | **Create** — new card stack component |
| `src/components/CoPilotPanel.tsx` | **Keep** — as fallback or remove later |
| `src/App.css` | **Edit** — add card animations and sticky footer styles |
| `src/components/RightPanel.tsx` | **Edit** — render CoPilotCardStack instead of CoPilotPanel |
| `src/state/types.ts` | **Edit** — add CoPilotCard and CoPilotCardState types |

---

## Open Design Decisions (For Implementation)

1. **Auto-collapse timer**: 8s proposed. Should this be configurable or is 8s the right default?
2. **Max visible cards**: Should we limit to, say, 20 cards and archive older ones? Or unlimited scroll?
3. **Suggested questions**: Show as cards (type = suggestion) or keep as a separate popover? The redesign doc suggested popover — keeping that as a future enhancement.
4. **Key concepts**: Not mapped to cards. Keep them for the final summary card only, or drop them from live view entirely?
5. **Card title extraction**: Use first N characters of the insight, or ask the model to provide a short title? Current model output doesn't include per-item titles — may need a simple heuristic (first clause before comma/period, capped at 60 chars).
