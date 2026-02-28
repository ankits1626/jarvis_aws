# Implementation Plan: Co-Pilot Card Stack UX

## Overview

This implementation transforms the Co-Pilot agent's live intelligence display from a static document-style panel into an animated card-based interface. The redesign addresses cognitive load issues by presenting insights as individual cards that animate in, auto-collapse after a timeout, and persist after recording stops with a comprehensive summary card.

The implementation is frontend-only with zero backend changes, using existing `CoPilotState` data model and Tauri events.

**Spec Type:** Feature  
**Workflow:** Requirements-First  
**Design Document:** `.kiro/specs/copilot-card-stack-ux/design.md`  
**Requirements Document:** `.kiro/specs/copilot-card-stack-ux/requirements.md`

---

## Implementation Phases

### Phase 1: Foundation — Data Models & Utilities (Tasks 1-4)

**Goal:** Set up TypeScript interfaces, utility functions, and core logic that can be developed and tested independently.

**Tasks:**
- Task 1: Add TypeScript Interfaces
- Task 2: Create Utility Functions
- Task 3: Implement Card Diffing Logic
- Task 4: Implement Auto-Collapse Timer Management

**Validation:** All utility functions have unit tests and work correctly in isolation.

---

### Phase 2: Component Structure — State & Hooks (Tasks 5-10)

**Goal:** Build the CoPilotCardStack component structure with state management and React hooks.

**Tasks:**
- Task 5: Create CoPilotCardStack Component Structure
- Task 6: Implement Card Creation useEffect
- Task 7: Implement Recording State Tracking useEffect
- Task 8: Implement Final Summary Card Creation
- Task 9: Implement Countdown Timer useEffect
- Task 10: Implement Card Interaction Functions

**Validation:** Component state management works correctly with proper lifecycle handling.

---

### Phase 3: UI Rendering — Cards & Layout (Tasks 11-14)

**Goal:** Implement the visual components for panel header, card stack, summary card, and footer.

**Tasks:**
- Task 11: Render Panel Header
- Task 12: Render Card Stack
- Task 13: Render Final Summary Card
- Task 14: Render Sticky Footer

**Validation:** All UI elements render correctly with proper structure and accessibility.

---

### Phase 4: Styling — CSS Animations & Transitions (Tasks 15-19)

**Goal:** Add all CSS styling, animations, and transitions for the card-based interface.

**Tasks:**
- Task 15: Add CSS Variables and Base Styles
- Task 16: Add Card Animation Keyframes
- Task 17: Add Card Expand/Collapse CSS Transitions
- Task 18: Add Final Summary Card CSS
- Task 19: Add Panel Header and Footer CSS

**Validation:** All animations and transitions work smoothly with proper visual polish.

---

### Phase 5: Integration — RightPanel & Testing (Tasks 20-24)

**Goal:** Integrate with existing app, add automated tests, perform manual testing, and finalize documentation.

**Tasks:**
- Task 20: Update RightPanel Integration
- Task 21: Write Property-Based Tests
- Task 22: Write Unit Tests
- Task 23: Manual Testing and Polish
- Task 24: Documentation and Cleanup

**Validation:** Full card stack UX works end-to-end with proper animations, interactions, and persistence.

---

## Tasks

### Phase 1: Foundation — Data Models & Utilities

### Phase 1: Foundation — Data Models & Utilities

- [-] 1. Add TypeScript Interfaces

**Requirement:** Requirement 1 (Card Data Model)

Add the core TypeScript interfaces for cards and card stack state to `src/state/types.ts`.

**Acceptance:**
- [x] 1.1 Add `CoPilotCard` interface with all required fields (id, type, title, body, cycle, timestamp, isExpanded, isNew)
- [x] 1.2 Add `FinalSummaryCard` interface with summary sections (summary, keyTakeaways, actionItems, decisions, openQuestions)
- [x] 1.3 Add `CoPilotCardStackState` interface with component state fields (cards, runningStatus, currentCycle, nextCycleIn, totalAudioAnalyzed, finalSummaryCard)
- [x] 1.4 Add type definitions for card types: `type CoPilotCardType = 'insight' | 'decision' | 'action_item' | 'question' | 'summary_update'`
- [x] 1.5 Reference existing `RecordingState` type from types.ts (already defined as `"idle" | "recording" | "processing"`) — do not add duplicate
- [x] 1.6 Add type definition for running status: `type CoPilotRunningStatus = 'idle' | 'recording' | 'processing' | 'complete'`

---

- [-] 2. Create Utility Functions

**Requirement:** Requirements 1.4, 1.5, 2

Create helper functions for card ID generation, title extraction, timestamp formatting, and card type mapping.

**Acceptance:**
- [x] 2.1 Implement `generateCardId(cycle: number, type: string, index: number): string` following pattern `cycle-{N}-{type}-{index}`
- [x] 2.2 Implement `extractTitle(body: string): string` that extracts first clause before comma/period/dash, caps at 60 chars with ellipsis
- [x] 2.3 Implement `formatTimestamp(timestamp: number): string` that formats Unix timestamp to readable time (e.g., "2:34 PM")
- [x] 2.4 Define `CARD_TYPE_MAP` constant mapping CoPilotState fields to card types
- [x] 2.5 Define `CARD_TYPE_PRIORITY` constant for sorting cards within a cycle (decision=1, action_item=2, question=3, insight=4, summary_update=5)
- [ ] 2.6 Add unit tests for `generateCardId` (determinism, pattern matching)
- [ ] 2.7 Add unit tests for `extractTitle` (empty string, long text, multiple delimiters, special characters)
- [ ] 2.8 Add unit tests for `formatTimestamp` (various timestamps, edge cases)

---

- [-] 3. Implement Card Diffing Logic

**Requirement:** Requirement 2 (Card Creation from CoPilotState)

Create the diffing algorithm that compares new CoPilotState against existing cards to identify genuinely new items.

**Acceptance:**
- [x] 3.1 Implement `createCardsFromStateDiff(newState: CoPilotState, oldState: CoPilotState | null, existingCards: CoPilotCard[]): CoPilotCard[]`
- [x] 3.2 Implement deduplication logic using exact string match against existing card bodies
- [x] 3.3 Create cards for new items in `key_points` array (type: 'insight')
- [x] 3.4 Create cards for new items in `decisions` array (type: 'decision')
- [x] 3.5 Create cards for new items in `action_items` array (type: 'action_item')
- [x] 3.6 Create cards for new items in `open_questions` array (type: 'question')
- [x] 3.7 Create summary_update card when `running_summary` changes (compare against oldState)
- [x] 3.8 Sort new cards by priority before returning
- [x] 3.9 Exclude `suggested_questions` and `key_concepts` from card creation
- [ ] 3.10 Add unit tests for diffing logic (empty state, first cycle, incremental updates, no changes)

---

- [-] 4. Implement Auto-Collapse Timer Management

**Requirement:** Requirement 6 (Card Auto-Collapse)

Create timer management functions for auto-collapse behavior with pause/resume support.

**Acceptance:**
- [x] 4.1 Define `TimerState` interface with fields: timeout, startedAt, duration, remaining
- [x] 4.2 Implement `startAutoCollapseTimer(cardId: string, duration: number)` that sets card to collapsed after duration
- [x] 4.3 Implement `pauseAutoCollapseTimer(cardId: string)` that pauses timer and tracks remaining time
- [x] 4.4 Implement `resumeAutoCollapseTimer(cardId: string)` that resumes timer from remaining time
- [x] 4.5 Implement `cancelAutoCollapseTimer(cardId: string)` that cancels timer and removes from map
- [x] 4.6 Ensure timers are stored in `useRef<Map<string, TimerState>>` to avoid stale closures
- [x] 4.7 Add cleanup function in useEffect to cancel all timers on unmount
- [ ] 4.8 Add unit tests for timer behavior (start, pause, resume, cancel, cleanup)

---

### Phase 2: Component Structure — State & Hooks

- [x] 5. Create CoPilotCardStack Component Structure

**Requirement:** Requirement 11 (CoPilotCardStack Component)

Create the main component file with props interface, state management, and refs.

**Acceptance:**
- [x] 5.1 Create `src/components/CoPilotCardStack.tsx` file
- [x] 5.2 Define `CoPilotCardStackProps` interface extending existing CoPilotPanel props plus recordingState and cycleInterval
- [x] 5.3 Initialize component state: `cards`, `finalSummaryCard`, `hasCompleted`, `nextCycleIn`
- [x] 5.4 Initialize refs: `autoCollapseTimers`, `previousState`, `previousRecordingState`, `previousCycleNumber`, `cardAreaRef`
- [x] 5.5 Derive `runningStatus` using useMemo from hasCompleted, recordingState, and status props
- [x] 5.6 Add placeholder rendering when state is null or cycle 0

---

- [x] 6. Implement Card Creation useEffect

**Requirement:** Requirements 2, 5 (Card Creation and Entrance Animation)

Add useEffect that watches CoPilotState changes and creates new cards.

**Acceptance:**
- [x] 6.1 Implement useEffect watching `state` prop
- [x] 6.2 Call `createCardsFromStateDiff` to identify new cards
- [x] 6.3 Prepend new cards to existing cards array (newest first)
- [x] 6.4 Start auto-collapse timers for each new card (8s default, 5s for summary_update)
- [x] 6.5 Scroll card area to top when new cards arrive
- [x] 6.6 Update `previousState` ref after processing
- [x] 6.7 Set new cards with `isExpanded: true` and `isNew: true`

---

- [x] 7. Implement Recording State Tracking useEffect

**Requirement:** Requirement 7 (Persistent Agent View After Recording Stops)

Add useEffect that tracks recording state transitions to set hasCompleted flag.

**Acceptance:**
- [x] 7.1 Implement useEffect watching `recordingState` prop
- [x] 7.2 Detect recording→idle transition and set `hasCompleted: true`
- [x] 7.3 Detect idle→recording transition and set `hasCompleted: false` (new recording started)
- [x] 7.4 Update `previousRecordingState` ref after processing
- [x] 7.5 Ensure runningStatus derives correctly from hasCompleted flag

---

- [x] 8. Implement Final Summary Card Creation

**Requirement:** Requirement 8 (Final Summary Card)

Add logic to create and display final summary card when recording stops.

**Acceptance:**
- [x] 8.1 Implement `createFinalSummaryCard(state: CoPilotState): FinalSummaryCard` function
- [x] 8.2 Extract summary from `running_summary` field
- [x] 8.3 Extract keyTakeaways from `key_points` array
- [x] 8.4 Extract actionItems from `action_items` array
- [x] 8.5 Extract decisions from `decisions` array
- [x] 8.6 Extract openQuestions from `open_questions` array
- [x] 8.7 Call `createFinalSummaryCard` in recording state useEffect when wasRecording && nowStopped
- [x] 8.8 Set `finalSummaryCard` state with created card
- [ ] 8.9 Add unit tests for final summary card creation (empty sections, full sections, null state)

---

- [x] 9. Implement Countdown Timer useEffect

**Requirement:** Requirement 9 (Sticky Cycle Footer)

Add useEffect that manages the countdown timer for "Next cycle in ~Xs".

**Acceptance:**
- [x] 9.1 Initialize `nextCycleIn` state with `cycleInterval` prop
- [x] 9.2 Implement useEffect watching `state`, `recordingState`, `status`, `cycleInterval`
- [x] 9.3 Reset countdown when `cycle_metadata.cycle_number` changes (new cycle started)
- [x] 9.4 Decrement countdown every second using setInterval when recording and not processing
- [x] 9.5 Use `previousCycleNumber` ref to detect cycle transitions
- [x] 9.6 Clean up interval on unmount or when conditions change
- [x] 9.7 Ensure countdown doesn't go below 0 (use Math.max(0, prev - 1))

---

- [x] 10. Implement Card Interaction Functions

**Requirement:** Requirement 4 (Card Expand/Collapse Interaction)

Add functions for toggling card expansion and bulk operations.

**Acceptance:**
- [x] 10.1 Implement `toggleCardExpansion(cardId: string)` that flips isExpanded and sets isNew: false
- [x] 10.2 Call `cancelAutoCollapseTimer` when user manually toggles
- [x] 10.3 Implement `expandAllCards()` that sets isExpanded: true on all cards
- [x] 10.4 Implement `collapseAllCards()` that sets isExpanded: false on all cards and cancels all timers
- [ ] 10.5 Add unit tests for toggle, expand all, collapse all

---

### Phase 3: UI Rendering — Cards & Layout

- [x] 11. Render Panel Header

**Requirement:** Requirement 10 (Panel Header)

Implement the panel header with title, status indicator, and bulk actions.

**Acceptance:**
- [x] 11.1 Render panel title "Co-Pilot Agent"
- [x] 11.2 Render status indicator: pulsing dot when recording, checkmark when complete
- [x] 11.3 Render "Expand All" button with onClick handler
- [x] 11.4 Render "Collapse All" button with onClick handler
- [x] 11.5 Add separator pipe "|" between buttons
- [x] 11.6 Apply CSS classes for styling and layout

---

- [x] 12. Render Card Stack

**Requirement:** Requirements 3, 4, 5 (Card Type Color Coding, Expand/Collapse, Entrance Animation)

Implement the scrollable card area with individual card rendering.

**Acceptance:**
- [x] 12.1 Render scrollable container with ref={cardAreaRef}
- [x] 12.2 Render final summary card at top if finalSummaryCard is not null
- [x] 12.3 Map over cards array and render each card
- [x] 12.4 Apply conditional classes: `entering` for isNew, `expanded`/`collapsed` for isExpanded
- [x] 12.5 Render card header with chevron, title, and type dot
- [x] 12.6 Add onClick handler to card header for toggleCardExpansion
- [x] 12.7 Add onMouseEnter/onMouseLeave handlers for timer pause/resume
- [x] 12.8 Render card body (always in DOM, visibility controlled by CSS)
- [x] 12.9 Render card metadata with cycle number and timestamp
- [x] 12.10 Add accessibility attributes: role="button", aria-expanded, tabIndex, onKeyDown

---

- [x] 13. Render Final Summary Card

**Requirement:** Requirement 8 (Final Summary Card)

Implement the final summary card with distinct styling and conditional sections.

**Acceptance:**
- [x] 13.1 Render final summary card with distinct CSS class
- [x] 13.2 Render header with clipboard icon and "Session Summary" title
- [x] 13.3 Conditionally render Summary section if summary text exists
- [x] 13.4 Conditionally render Key Takeaways section if keyTakeaways array has items
- [x] 13.5 Conditionally render Action Items section if actionItems array has items (with ☐ prefix)
- [x] 13.6 Conditionally render Decisions section if decisions array has items (with ✓ prefix)
- [x] 13.7 Conditionally render Open Questions section if openQuestions array has items (with ? prefix)
- [x] 13.8 Ensure card is not collapsible (no onClick on header)

---

- [x] 14. Render Sticky Footer

**Requirement:** Requirement 9 (Sticky Cycle Footer)

Implement the sticky footer with cycle status and stats.

**Acceptance:**
- [x] 14.1 Render footer with position: sticky, bottom: 0
- [x] 14.2 Render status indicator based on runningStatus (pulsing dot, checkmark)
- [x] 14.3 Render status text: "Cycle N in ~Xs" when recording, "Processing cycle N..." when processing, "Session complete" when complete
- [x] 14.4 Render stats: cycles done and total audio duration formatted as "Xm Ys"
- [x] 14.5 Implement `formatDuration(seconds: number): string` helper function
- [x] 14.6 Apply CSS classes for styling and layout

---

### Phase 4: Styling — CSS Animations & Transitions

- [x] 15. Add CSS Variables and Base Styles

**Requirement:** Requirement 13 (CSS Animations and Styling)

Add CSS variables for card type colors and base card styles to App.css.

**Acceptance:**
- [x] 15.1 Add CSS variables to :root for card type dot colors (insight, decision, action, question, summary)
- [x] 15.2 Add base `.copilot-card` styles (border-radius, margin, background, border)
- [x] 15.3 Add `.copilot-card-header` styles (display flex, padding, cursor pointer)
- [x] 15.4 Add `.copilot-card-title` styles (flex 1, font-size, color)
- [x] 15.5 Add `.copilot-card-dot` styles (width, height, border-radius, background)
- [x] 15.6 Add type-specific dot color classes (`.copilot-card-dot-insight`, etc.)
- [x] 15.7 Add `.copilot-card-chevron` styles with rotation transition

---

- [x] 16. Add Card Animation Keyframes

**Requirement:** Requirement 13 (CSS Animations and Styling)

Add CSS keyframe animations for card entrance and pulse effects.

**Acceptance:**
- [x] 16.1 Add `@keyframes cardSlideIn` (translateY -20px → 0, opacity 0 → 1, 300ms ease-out)
- [x] 16.2 Add `@keyframes subtlePulse` (box-shadow pulse, 2 cycles, 2s ease-in-out)
- [x] 16.3 Apply `cardSlideIn` animation to `.copilot-card.entering` class
- [x] 16.4 Apply `subtlePulse` animation to `.copilot-card.entering` class
- [x] 16.5 Ensure entering CSS animations use animation-fill-mode: forwards and play once (animation-iteration-count for pulse set to 2 for 2-second pulse effect)

---

- [x] 17. Add Card Expand/Collapse CSS Transitions

**Requirement:** Requirement 13 (CSS Animations and Styling)

Add CSS transitions for card body and metadata expand/collapse.

**Acceptance:**
- [x] 17.1 Add `.copilot-card.collapsed .copilot-card-body` styles (max-height: 0, padding: 0, overflow: hidden)
- [x] 17.2 Add `.copilot-card.expanded .copilot-card-body` styles (max-height: 300px, padding: 8px 12px)
- [x] 17.3 Add transition properties to body (max-height 200ms ease, padding 200ms ease)
- [x] 17.4 Add `.copilot-card.collapsed .copilot-card-metadata` styles (max-height: 0, padding: 0, opacity: 0)
- [x] 17.5 Add `.copilot-card.expanded .copilot-card-metadata` styles (max-height: 50px, padding: 4px 12px 8px, opacity: 1)
- [x] 17.6 Add transition properties to metadata (max-height, padding, opacity 200ms ease)

---

- [x] 18. Add Final Summary Card CSS

**Requirement:** Requirement 8 (Final Summary Card)

Add distinct styling for the final summary card.

**Acceptance:**
- [x] 18.1 Add `.copilot-final-summary-card` styles (border-left: 3px solid var(--accent-primary), background: var(--bg-elevated))
- [x] 18.2 Add `.copilot-summary-icon` styles for clipboard icon
- [x] 18.3 Add `.summary-section` styles for each section (margin, padding)
- [x] 18.4 Add section heading styles (h5 font-size, color, margin)
- [x] 18.5 Add list styles for bullet lists, checklists with prefixes

---

- [x] 19. Add Panel Header and Footer CSS

**Requirement:** Requirements 10, 9 (Panel Header, Sticky Cycle Footer)

Add CSS for panel header and sticky footer layout and styling.

**Acceptance:**
- [x] 19.1 Add `.copilot-panel-header` styles (display flex, justify-content space-between, padding, border-bottom)
- [x] 19.2 Add `.copilot-panel-title` styles (display flex, align-items center, gap)
- [x] 19.3 Add `.copilot-status-indicator` styles for pulsing dot and checkmark
- [x] 19.4 Add `.copilot-bulk-actions` styles (display flex, gap, font-size)
- [x] 19.5 Add `.copilot-sticky-footer` styles (position sticky, bottom 0, background, border-top, padding, z-index)
- [x] 19.6 Add `.copilot-footer-status` and `.copilot-footer-stats` styles (display flex, align-items, gap)
- [x] 19.7 Add pulsing dot animation for status indicators

---

### Phase 5: Integration — RightPanel & Testing

- [x] 20. Update RightPanel Integration

**Requirement:** Requirement 12 (RightPanel Integration)

Update RightPanel.tsx to render CoPilotCardStack instead of CoPilotPanel.

**Acceptance:**
- [x] 20.1 Import `CoPilotCardStack` from `./CoPilotCardStack`
- [x] 20.2 Replace `<CoPilotPanel />` with `<CoPilotCardStack />` in render
- [x] 20.3 Pass existing props: state, status, error, onDismissQuestion
- [x] 20.4 Pass additional props: recordingState (from app state), cycleInterval (from settings or default 60)
- [x] 20.5 Update tab visibility logic to keep Co-Pilot tab visible when: `copilotEnabled && (isRecording || (copilotState && copilotState.cycle_metadata.cycle_number > 0))` (Note: hasCompleted is internal to CoPilotCardStack and not available in RightPanel)
- [x] 20.6 Comment out or remove CoPilotPanel import
- [x] 20.7 Test that tab switching works correctly

---

- [ ] 21. Write Property-Based Tests

**Requirement:** Design Document (Correctness Properties)

Implement property-based tests using fast-check for core properties.

**Acceptance:**
- [ ] 21.1 Install fast-check as dev dependency
- [ ] 21.2 Write Property 1 test: Card ID Determinism (generateCardId produces same ID for same inputs)
- [ ] 21.3 Write Property 2 test: Title Extraction Format (extractTitle follows format rules)
- [ ] 21.4 Write Property 3 test: Card Deduplication (diffing prevents duplicate cards)
- [ ] 21.5 Write Property 4 test: Card Type Mapping (correct type assigned to each CoPilotState field)
- [ ] 21.6 Write Property 5 test: Card Ordering (newest first, priority sorted within cycle)
- [ ] 21.7 Write Property 6 test: Excluded Fields (suggested_questions and key_concepts not in cards)
- [ ] 21.8 Write Property 7 test: Card Expansion Toggle (isExpanded flips correctly)
- [ ] 21.9 Write Property 9 test: New Card Initial State (isExpanded: true, isNew: true)
- [ ] 21.10 Write Property 12 test: Bulk Expand/Collapse (all cards affected correctly)
- [ ] 21.11 Create custom generators for CoPilotState, CoPilotCard, and body text
- [ ] 21.12 Configure tests to run 100 iterations minimum

---

- [ ] 22. Write Unit Tests

**Requirement:** Design Document (Testing Strategy)

Implement unit tests for edge cases and component behavior.

**Acceptance:**
- [ ] 22.1 Test generateCardId with various inputs (cycle 0, large numbers, special characters in type)
- [ ] 22.2 Test extractTitle with edge cases (empty string, very long text, no delimiters, only special chars)
- [ ] 22.3 Test createCardsFromStateDiff with empty state, first cycle, no changes
- [ ] 22.4 Test timer functions (start, pause, resume, cancel, cleanup on unmount)
- [ ] 22.5 Test toggleCardExpansion (state flip, timer cancellation)
- [ ] 22.6 Test expandAllCards and collapseAllCards
- [ ] 22.7 Test createFinalSummaryCard with empty sections, full sections, null state
- [ ] 22.8 Test component rendering with null state (placeholder)
- [ ] 22.9 Test component rendering with valid state (cards appear)
- [ ] 22.10 Test final summary card rendering (conditional sections)

---

- [ ] 23. Manual Testing and Polish

**Requirement:** Design Document (Manual Testing Checklist)

Perform manual testing of animations, interactions, and visual polish.

**Acceptance:**
- [ ] 23.1 Verify new cards animate in smoothly (slide + fade)
- [ ] 23.2 Verify new cards display pulse animation for 2 seconds
- [ ] 23.3 Verify cards auto-collapse after 8 seconds (5s for summary_update)
- [ ] 23.4 Verify clicking card header toggles expansion with smooth animation
- [ ] 23.5 Verify chevron rotates 90 degrees when expanding
- [ ] 23.6 Verify body text animates open/closed with max-height transition
- [ ] 23.7 Verify hovering on card pauses auto-collapse timer
- [ ] 23.8 Verify leaving hover resumes timer from where it paused
- [ ] 23.9 Verify clicking header during auto-collapse cancels timer
- [ ] 23.10 Verify Expand All button expands all cards instantly
- [ ] 23.11 Verify Collapse All button collapses all cards and cancels timers
- [ ] 23.12 Verify final summary card appears when recording stops
- [ ] 23.13 Verify final summary card is visually distinct (border, background)
- [ ] 23.14 Verify final summary card does not collapse when clicked
- [ ] 23.15 Verify footer stays visible at bottom when scrolling cards
- [ ] 23.16 Verify footer countdown decrements every second
- [ ] 23.17 Verify footer shows correct status during recording/processing/complete
- [ ] 23.18 Verify panel persists after recording stops (doesn't disappear)
- [ ] 23.19 Verify panel header shows checkmark when recording completes
- [ ] 23.20 Verify type dots display correct colors for each card type
- [ ] 23.21 Verify scroll position resets to top when new cards arrive
- [ ] 23.22 Test keyboard navigation (Tab to focus, Enter/Space to toggle)
- [ ] 23.23 Test with long session (30+ cards) for performance
- [ ] 23.24 Test rapid state updates (multiple cycles in quick succession)

---

- [x] 24. Documentation and Cleanup

**Requirement:** General project hygiene

Add comments, update documentation, and clean up code.

**Acceptance:**
- [x] 24.1 Add JSDoc comments to all exported functions and interfaces
- [x] 24.2 Add inline comments explaining complex logic (diffing, timer management)
- [x] 24.3 Update component file header with description and usage example
- [x] 24.4 Verify all console.log statements are removed or converted to proper logging
- [x] 24.5 Run linter and fix any warnings
- [x] 24.6 Verify no unused imports or variables
- [x] 24.7 Update README or project docs if needed

---

## Notes

- Tasks are organized into 5 phases for logical implementation flow
- Phase 1 (Tasks 1-4): Foundation work that can be developed and unit-tested independently
- Phase 2 (Tasks 5-10): Component structure with state management and React hooks (all hooks go into CoPilotCardStack.tsx)
- Phase 3 (Tasks 11-14): UI rendering for all visual components
- Phase 4 (Tasks 15-19): CSS styling, animations, and transitions
- Phase 5 (Tasks 20-24): Integration with existing app, automated tests, manual testing, and documentation
- Each task references specific requirements for traceability
- Property-based tests validate universal correctness properties
- Unit tests validate specific examples and edge cases
- The implementation is frontend-only with zero backend changes
