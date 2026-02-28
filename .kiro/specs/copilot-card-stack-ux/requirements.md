# Co-Pilot Card Stack UX — Live Agent Display Redesign

## Introduction

The Co-Pilot agent (spec: `copilot-agent`) produces real-time intelligence during live recording: rolling summary, key points, decisions, action items, open questions, suggested questions, and key concepts. The current `CoPilotPanel` component renders all 7 categories simultaneously as a scrollable document with static bullet lists. This creates excessive cognitive load during live use — the user cannot glance at the panel and understand what changed or what matters most.

This spec defines a **Card Stack UX** that replaces the document-style `CoPilotPanel` with an animated, card-based interface. Each card represents a single insight (not a category), cards animate in when new data arrives, and the panel persists after recording stops with a terminal summary card. The backend data model (`CoPilotState`, `CoPilotCycleResult`, Tauri events) is unchanged — this is a frontend-only redesign.

**Reference:** UX analysis in `discussion/28-feb-next-step/copilot-ux-redesign.md`. Refined design in `discussion/28-feb-next-step/copilot-card-stack-design.md`.

## Glossary

- **Card**: A single UI element representing one insight extracted during a Co-Pilot cycle. Contains a type, title, body text, and metadata.
- **Card Stack**: The scrollable container holding all cards, ordered newest-first.
- **Card Type**: Classification of the insight — `insight` (key point), `decision`, `action_item`, `question`, or `summary_update`. Each type has a distinct color dot.
- **Expanded / Collapsed**: A card's visibility state. Expanded shows the full body text; collapsed shows only the title row.
- **Auto-Collapse**: Behavior where new cards appear expanded and automatically collapse after 8 seconds if the user hasn't interacted with them.
- **Final Summary Card**: A special card injected when recording stops, containing the accumulated summary, key takeaways, action items, decisions, and open questions.
- **Sticky Footer**: The cycle status bar pinned to the bottom of the panel, always visible regardless of card scroll position.
- **Chevron**: The `▸` / `▾` indicator showing whether a card is collapsed or expanded.

## Frozen Design Decisions

These decisions were made during design review (2026-02-28):

1. **Card Stack (Option C)**: Selected over "Glanceable Summary" (Option A) and "Feed" (Option B) from the UX analysis. Cards provide the best balance of information density and glanceability.
2. **Frontend-only change**: The backend `CoPilotState`, `CoPilotCycleResult`, Tauri events, cycle timing, and deduplication logic are unchanged. The card stack is a presentation layer transformation.
3. **One insight per card**: Cards map to individual items (one key point = one card), not categories. A cycle producing 3 new key points creates 3 cards.
4. **Auto-collapse at 8 seconds**: New cards appear expanded and auto-collapse after 8s. This is a fixed default, not user-configurable in v1.
5. **Agent view persists after recording**: The Co-Pilot tab and its content stay visible when recording stops. The panel transitions to a "complete" state with a final summary card.
6. **Sticky cycle footer**: The footer is always visible at the bottom of the panel regardless of scroll position.
7. **No card dismissal in v1**: Cards cannot be dismissed or swiped away. This is deferred to a future iteration.
8. **Suggested questions excluded from cards**: Suggested questions are not rendered as cards. They are deferred to a separate popover interaction in a future iteration.
9. **Key concepts excluded from live cards**: Key concepts are not rendered as individual cards during live recording. They appear only in the final summary card.
10. **New component alongside existing**: `CoPilotCardStack.tsx` is created as a new component. `CoPilotPanel.tsx` is kept but no longer rendered.

---

## Requirement 1: Card Data Model

**User Story:** As a developer, I need a well-defined card data structure in TypeScript so the card stack component can manage card state (creation, expansion, auto-collapse) independently from the backend `CoPilotState`.

### Acceptance Criteria

1. THE System SHALL define a `CoPilotCard` interface in `src/state/types.ts` with fields: `id` (string, unique per card), `type` (`'insight' | 'decision' | 'action_item' | 'question' | 'summary_update'`), `title` (string, max 60 characters), `body` (string, full insight text), `cycle` (number, which cycle produced this card), `timestamp` (string, ISO 8601), `isExpanded` (boolean), `isNew` (boolean, true until auto-collapse fires)
2. THE System SHALL define a `FinalSummaryCard` interface with fields: `summary` (string), `keyTakeaways` (string[]), `actionItems` (string[]), `decisions` (string[]), `openQuestions` (string[])
3. THE System SHALL define a `CoPilotCardStackState` interface with fields: `cards` (CoPilotCard[]), `runningStatus` (`'idle' | 'recording' | 'processing' | 'complete'`), `currentCycle` (number), `nextCycleIn` (number, seconds), `totalAudioAnalyzed` (number, seconds), `finalSummaryCard` (FinalSummaryCard | null)
4. THE card `id` SHALL be deterministic and unique, following the pattern `cycle-{N}-{type}-{index}` (e.g., `cycle-3-insight-0`, `cycle-3-decision-1`)
5. THE card `title` SHALL be extracted from the body text using a heuristic: first clause before a comma, period, or dash, capped at 60 characters with ellipsis if truncated

---

## Requirement 2: Card Creation from CoPilotState

**User Story:** As the card stack component, I need to transform incoming `CoPilotState` updates into card creation operations, so that each new insight appears as an individual card without duplicating existing ones.

### Acceptance Criteria

1. WHEN a `copilot-updated` event arrives with a new `CoPilotState`, THE System SHALL compare the new state against the current card list and create cards only for genuinely new items
2. THE mapping from `CoPilotState` fields to card types SHALL be:

   | State Field | Card Type | Card Creation Rule |
   |---|---|---|
   | `key_points` | `insight` (blue) | Each new key point not already in cards = 1 new card |
   | `decisions` | `decision` (green) | Each new decision not already in cards = 1 new card |
   | `action_items` | `action_item` (amber) | Each new action item not already in cards = 1 new card |
   | `open_questions` | `question` (red-orange) | Each new question not already in cards = 1 new card |
   | `running_summary` | `summary_update` (purple) | 1 card per cycle if summary text changed |

3. THE deduplication SHALL compare new items against existing card bodies using exact string match (consistent with the backend's existing deduplication logic)
4. NEW cards SHALL be prepended to the cards array (newest first) so they appear at the top of the stack
5. WITHIN a single cycle's output, new cards SHALL be ordered by priority: `decision` > `action_item` > `question` > `insight` > `summary_update`
6. THE System SHALL NOT create cards for `suggested_questions` or `key_concepts` during live recording (frozen decision #8, #9)

---

## Requirement 3: Card Type Color Coding

**User Story:** As a user, I want each card type to have a distinct color indicator so I can identify the type of insight at a glance without reading the full text.

### Acceptance Criteria

1. EACH card SHALL display a small colored dot in the header row, to the right of the title, indicating the card type
2. THE color mapping SHALL be:

   | Card Type | Dot Color (hex) | CSS Variable |
   |---|---|---|
   | `insight` | `#3B82F6` (blue) | `--card-dot-insight` |
   | `decision` | `#22C55E` (green) | `--card-dot-decision` |
   | `action_item` | `#F59E0B` (amber) | `--card-dot-action` |
   | `question` | `#EF4444` (red-orange) | `--card-dot-question` |
   | `summary_update` | `#8B5CF6` (purple) | `--card-dot-summary` |

3. THE dot SHALL be 8px diameter, centered vertically in the header row
4. THE Final Summary Card SHALL use a distinct icon or badge (e.g., clipboard icon) instead of a type dot

---

## Requirement 4: Card Expand / Collapse Interaction

**User Story:** As a user, I want to expand or collapse any card by clicking its header, so I can control how much detail I see at any given moment.

### Acceptance Criteria

1. EACH card SHALL have two states: **collapsed** (only header row visible) and **expanded** (header row + body text + card metadata visible)
2. THE header row SHALL contain: a chevron indicator (`▸` when collapsed, `▾` when expanded), the card title, and the type dot
3. CLICKING anywhere on the header row SHALL toggle the card between expanded and collapsed states
4. THE chevron SHALL rotate 90 degrees with a CSS `transform: rotate(90deg)` transition (150ms ease) when expanding
5. THE body SHALL animate open/closed via a `max-height` transition (200ms ease) — `max-height: 0` when collapsed, `max-height: 300px` when expanded
6. THE card metadata (cycle number, timestamp) SHALL be visible only when expanded, displayed below the body text in a smaller font
7. EXPANDED state SHALL show padding of `8px 12px` around the body; collapsed state SHALL show `0` padding on the body

---

## Requirement 5: Card Entrance Animation

**User Story:** As a user, I want new cards to animate in so I can immediately see that new insights have arrived without scanning the entire panel.

### Acceptance Criteria

1. WHEN new cards are created from a cycle update, EACH new card SHALL animate in with `translateY(-20px) → translateY(0)` combined with `opacity: 0 → 1` (300ms ease-out)
2. NEW cards SHALL appear **expanded** by default (body visible) so the user sees the content immediately
3. EXISTING cards below the new cards SHALL shift down smoothly (200ms transition on the container)
4. THE scroll position of the card stack SHALL reset to the top when new cards arrive, ensuring the newest cards are visible
5. NEW cards SHALL have an `isNew: true` flag and display a subtle pulse animation (`box-shadow` pulse, 2 cycles, 2s ease-in-out) to draw attention
6. THE entrance animation SHALL use a CSS class `entering` applied on mount and removed after the animation completes (300ms)

---

## Requirement 6: Card Auto-Collapse

**User Story:** As a user, I want new cards to automatically collapse after a few seconds so the panel stays compact and I don't get overwhelmed by expanded content during a long recording.

### Acceptance Criteria

1. WHEN a new card appears with `isNew: true`, THE System SHALL start an 8-second timer
2. AFTER 8 seconds, IF the user has not clicked or hovered on the card, THE System SHALL set `isExpanded: false` and `isNew: false`, collapsing the card with the standard collapse animation
3. IF the user clicks the card header (to manually collapse or re-expand) during the 8-second window, THE System SHALL cancel the auto-collapse timer and set `isNew: false` (user took ownership)
4. IF the user hovers over the card during the 8-second window, THE System SHALL pause the timer; when the mouse leaves, the timer resumes from where it paused
5. THE auto-collapse timer SHALL be per-card (each card has its own independent timer)
6. THE Summary Update card type SHALL auto-collapse after 5 seconds (shorter than the default 8s, since summary updates are less actionable during live)

---

## Requirement 7: Persistent Agent View After Recording Stops

**User Story:** As a user, I want the Co-Pilot panel to stay visible after recording stops so I can review the insights from the session without them disappearing.

### Acceptance Criteria

1. WHEN the recording stops (`recordingState` changes from `recording` to `idle`), THE Co-Pilot panel SHALL NOT be removed from the DOM
2. THE panel header recording indicator SHALL change from a pulsing red dot (`●`) to a static checkmark with "Complete" text
3. THE tab buttons (Transcript / Co-Pilot) SHALL remain visible and functional after recording stops
4. ALL existing cards SHALL remain in the panel and remain interactive (expandable/collapsible)
5. THE `runningStatus` in `CoPilotCardStackState` SHALL transition from `'recording'` or `'processing'` to `'complete'`
6. THE panel SHALL stay visible until the user navigates away from the Record nav item (e.g., clicks Recordings, Gems, Settings)
7. WHEN the user navigates back to Record nav after the recording has stopped, THE panel SHALL show the placeholder ("Start recording to see live transcript") — the previous session's data is not preserved across navigations

---

## Requirement 8: Final Summary Card

**User Story:** As a user, I want a comprehensive summary card to appear when the recording ends, showing the complete analysis — summary, key takeaways, action items, decisions, and open questions — so I have a single place to review everything.

### Acceptance Criteria

1. WHEN the recording stops AND the `CoPilotState` has at least 1 completed cycle, THE System SHALL create and display a Final Summary Card at the top of the card stack
2. THE Final Summary Card SHALL animate in using the same entrance animation as regular cards (300ms slide + fade)
3. THE Final Summary Card SHALL always remain **expanded** and SHALL NOT auto-collapse
4. THE Final Summary Card SHALL be visually distinct from regular cards: thicker left border (3px solid `var(--accent-primary)`), subtle background tint (`var(--bg-elevated)`)
5. THE Final Summary Card header SHALL display "Session Summary" as the title with a clipboard icon (or equivalent) instead of a type dot
6. THE Final Summary Card body SHALL contain these sections, rendered only if they have content (no empty section headers):
   - **Summary**: The final `running_summary` text
   - **Key Takeaways**: The accumulated `key_points` as a bullet list
   - **Action Items**: The accumulated `action_items` as a checklist with `☐` prefix
   - **Decisions**: The accumulated `decisions` as a checklist with `✓` prefix
   - **Open Questions**: The accumulated `open_questions` as a list with `?` prefix
7. THE Final Summary Card data SHALL be constructed from the final `CoPilotState` received before/when the agent stops
8. THE Final Summary Card SHALL NOT be collapsible — clicking the header does nothing

---

## Requirement 9: Sticky Cycle Footer

**User Story:** As a user, I want to always see the cycle status at the bottom of the Co-Pilot panel so I know when the next analysis will happen and how many cycles have completed.

### Acceptance Criteria

1. THE cycle footer SHALL be positioned with `position: sticky; bottom: 0` inside the panel, always visible regardless of card scroll position
2. THE footer SHALL have a background matching `var(--bg-secondary)`, a top border `1px solid var(--border-color)`, padding `8px 12px`, font-size `12px`, and `z-index: 10`
3. DURING recording (idle between cycles), THE footer SHALL display: a pulsing dot, "Cycle {N} in ~{X}s", and "{M} cycles done"
4. DURING processing (cycle in progress), THE footer SHALL display: an animated pulsing dot, "Processing cycle {N}..."
5. AFTER recording stops, THE footer SHALL display: a static checkmark, "Session complete", total cycles count, and total audio duration formatted as "Xm Ys"
6. THE countdown timer (`nextCycleIn`) SHALL decrement every second using a `setInterval` in the component, resetting when a new cycle starts
7. THE footer SHALL render above any scroll shadow and maintain visibility even when the card stack overflows

---

## Requirement 10: Panel Header

**User Story:** As a user, I want the Co-Pilot panel header to show the panel title, recording status, and quick actions for managing card visibility.

### Acceptance Criteria

1. THE panel header SHALL display "Co-Pilot Agent" as the title on the left side
2. THE panel header SHALL display a recording status indicator on the right: a pulsing red dot during recording, a static checkmark after recording stops
3. THE panel header SHALL include "Expand All" and "Collapse All" text buttons (separated by a pipe `|`) below the title
4. CLICKING "Expand All" SHALL set `isExpanded: true` on all cards (excluding final summary card which is always expanded)
5. CLICKING "Collapse All" SHALL set `isExpanded: false` on all cards and cancel any pending auto-collapse timers
6. THE panel header SHALL be fixed at the top of the panel (not scrollable with cards)

---

## Requirement 11: CoPilotCardStack Component

**User Story:** As a developer, I need a self-contained React component that manages card state, listens for CoPilotState changes, and renders the card stack with all animations and interactions.

### Acceptance Criteria

1. THE component SHALL be created at `src/components/CoPilotCardStack.tsx`
2. THE component SHALL accept the same props as the current `CoPilotPanel`: `state: CoPilotState | null`, `status: CoPilotStatus`, `error: string | null`, `onDismissQuestion: (index: number) => void`
3. THE component SHALL additionally accept: `recordingState: 'idle' | 'recording' | 'processing'` and `cycleInterval: number` (from settings, for countdown calculation)
4. THE component SHALL manage card state internally using `useState<CoPilotCard[]>` and `useState<FinalSummaryCard | null>`
5. THE component SHALL use a `useEffect` watching the `state` prop to detect changes and create new cards via the diffing logic (Requirement 2)
6. THE component SHALL use a `useEffect` watching `recordingState` to detect recording stop and trigger final summary card creation (Requirement 8)
7. THE component SHALL use `useRef` for auto-collapse timers to avoid stale closure issues
8. THE component SHALL render: panel header (Req 10) → scrollable card area → sticky footer (Req 9)
9. THE component SHALL show a placeholder when `state` is null or cycle 0: "Co-Pilot is analyzing your conversation... Insights will appear here as the conversation progresses."

---

## Requirement 12: RightPanel Integration

**User Story:** As a developer, I need to swap in the `CoPilotCardStack` component in place of `CoPilotPanel` in the `RightPanel` component, and keep the Co-Pilot tab visible after recording stops.

### Acceptance Criteria

1. THE `RightPanel` component SHALL render `CoPilotCardStack` instead of `CoPilotPanel` when the Co-Pilot tab is active
2. THE `RightPanel` SHALL pass `recordingState` and `cycleInterval` (from settings or a default of 60) as additional props to `CoPilotCardStack`
3. THE tab buttons (Transcript / Co-Pilot) SHALL remain visible when `copilotEnabled` is true AND (`recordingState === 'recording'` OR the Co-Pilot has completed data to show — i.e., `copilotState` is not null and has at least 1 cycle)
4. THE `CoPilotPanel` import SHALL be removed or commented out from `RightPanel.tsx` once `CoPilotCardStack` is wired in
5. THE existing `copilotEnabled`, `copilotStatus`, `copilotState`, `copilotError`, and `onDismissCopilotQuestion` props SHALL be forwarded to `CoPilotCardStack` without modification

---

## Requirement 13: CSS Animations and Styling

**User Story:** As a user, I want the card stack to feel smooth and polished with proper animations, so the live agent experience is pleasant rather than jarring.

### Acceptance Criteria

1. THE System SHALL define a `@keyframes cardSlideIn` animation: `from { opacity: 0; transform: translateY(-20px); }` → `to { opacity: 1; transform: translateY(0); }`, duration 300ms ease-out
2. THE System SHALL define a `@keyframes subtlePulse` animation for new cards: box-shadow pulse using the card's type color at 15% opacity, 2 cycles, 2s ease-in-out
3. THE `.copilot-card-body` SHALL use `max-height: 0; overflow: hidden; transition: max-height 200ms ease, padding 200ms ease` when collapsed, and `max-height: 300px; padding: 8px 12px` when expanded
4. THE `.copilot-card-chevron` SHALL use `transition: transform 150ms ease` and `transform: rotate(90deg)` when expanded
5. THE card stack panel SHALL use `display: flex; flex-direction: column; height: 100%` to fill the right panel, with the card area using `flex: 1; overflow-y: auto` and the footer using `flex-shrink: 0`
6. ALL new CSS SHALL be added to `src/App.css` (frozen decision: single CSS file) using existing design tokens (`--bg-primary`, `--bg-secondary`, `--bg-elevated`, `--text-primary`, `--text-secondary`, `--border-color`, `--accent-primary`)
7. THE Final Summary Card SHALL use distinct styling: `border-left: 3px solid var(--accent-primary)`, `background: var(--bg-elevated)`
8. EACH card SHALL have `border-radius: 6px`, `margin-bottom: 6px`, `background: var(--bg-primary)`, `border: 1px solid var(--border-color)`

---

## Technical Constraints

1. **Frontend-only change**: No Rust backend modifications. The `CoPilotState`, `CoPilotCycleResult`, Tauri events, and agent lifecycle are unchanged.
2. **Same props interface**: `CoPilotCardStack` accepts the same core props as `CoPilotPanel` plus `recordingState` and `cycleInterval`, ensuring minimal changes to `App.tsx` and `RightPanel.tsx`.
3. **Single CSS file**: All new styles go in `App.css` using existing design tokens. No new CSS files.
4. **No new dependencies**: No new npm packages. Animations use CSS only (no framer-motion, react-spring, etc.).
5. **Card state is ephemeral**: Card state lives in component state (`useState`). It is not persisted to localStorage or Tauri app state. Navigating away resets it.
6. **Auto-collapse timers use refs**: Timers must be stored in `useRef` to avoid stale closures and must be cleaned up on unmount.
7. **Deduplication consistency**: Card deduplication uses the same exact-match logic as the backend's `CoPilotState` aggregation.
8. **Existing component preserved**: `CoPilotPanel.tsx` is kept in the codebase but not rendered. No deletion.
9. **Performance**: The card stack should handle up to 50 cards without jank. Beyond 50, older cards may be virtualized in a future iteration.
10. **Accessibility**: Cards must be keyboard-navigable (Tab to focus, Enter/Space to toggle expand/collapse). Chevron state must be communicated via `aria-expanded`.

## Out of Scope

1. Card dismissal / swipe-to-dismiss — deferred to future iteration
2. Suggested questions rendering (as cards or popover) — deferred
3. Key concepts as individual cards — they appear only in the final summary card
4. Card persistence across navigations or app restarts
5. Configurable auto-collapse timer duration
6. Card virtualization for very long sessions (>50 cards)
7. Drag-to-reorder cards
8. Card filtering by type
9. Backend changes to support per-item titles (currently uses heuristic extraction)
10. Animation library dependencies (framer-motion, etc.)
