# Co-Pilot Card Stack UX — Design Document

## Overview

This design transforms the Co-Pilot agent's live intelligence display from a static document-style panel into an animated card-based interface. The redesign addresses cognitive load issues by presenting insights as individual cards that animate in, auto-collapse after a timeout, and persist after recording stops with a comprehensive summary card.

### Design Goals

1. **Reduce cognitive load**: Users can glance at the panel and immediately see what's new without scanning entire category lists
2. **Highlight changes**: New insights animate in with visual feedback (slide + fade + pulse)
3. **Maintain context**: Cards remain accessible after auto-collapse via expand/collapse interaction
4. **Preserve session value**: Panel persists after recording stops with a final summary card
5. **Zero backend changes**: Pure frontend transformation using existing `CoPilotState` data model

### Key Design Decisions

- **Card-per-insight model**: Each key point, decision, action item, or question becomes an individual card (not category-based grouping)
- **Auto-collapse at 8 seconds**: New cards appear expanded and automatically collapse after 8s to keep the interface compact
- **Newest-first ordering**: New cards prepend to the stack (top of list) so users see changes immediately
- **Type-based color coding**: Each card type has a distinct color dot for at-a-glance identification
- **Sticky footer**: Cycle status always visible at bottom regardless of scroll position
- **Frontend-only**: No changes to Rust backend, Tauri events, or `CoPilotState` structure

### User Experience Flow

1. **Recording starts** → Co-Pilot tab becomes visible → Panel shows placeholder
2. **First cycle completes** → Cards animate in (expanded) → Auto-collapse timers start
3. **Subsequent cycles** → New cards prepend to stack → Existing cards shift down → New cards auto-collapse after 8s
4. **User interaction** → Click header to toggle expand/collapse → Hover pauses auto-collapse timer
5. **Recording stops** → Final summary card appears at top → Panel persists with "Complete" status → All cards remain interactive

---

## Architecture

### Component Hierarchy

```
CoPilotCardStack (new component)
├── Panel Header
│   ├── Title ("Co-Pilot Agent")
│   ├── Recording Status Indicator (pulsing dot / checkmark)
│   └── Bulk Actions ("Expand All" | "Collapse All")
├── Card Stack Container (scrollable)
│   ├── Final Summary Card (when recording stops)
│   └── CoPilotCard[] (newest first)
│       ├── Card Header (chevron, title, type dot)
│       ├── Card Body (insight text, conditionally visible)
│       └── Card Metadata (cycle #, timestamp, conditionally visible)
└── Sticky Footer
    ├── Status Indicator (pulsing dot / checkmark)
    ├── Cycle Info ("Cycle N in ~Xs" / "Processing cycle N..." / "Session complete")
    └── Stats (cycles done, audio analyzed)
```

### State Management Strategy

**Component-level state** (not persisted):
- `cards: CoPilotCard[]` — Array of all cards created from `CoPilotState` updates
- `finalSummaryCard: FinalSummaryCard | null` — Summary card data when recording stops
- `autoCollapseTimers: Map<string, TimerState>` — Per-card timer state for auto-collapse (stored in `useRef`)
- `hasCompleted: boolean` — Sticky flag set when recording stops, cleared when new recording starts
- `runningStatus: 'idle' | 'recording' | 'processing' | 'complete'` — Derived from `hasCompleted`, `recordingState`, and `status` props

**Derived State**:
```typescript
const [hasCompleted, setHasCompleted] = useState(false);

// Track recording state transitions to set/clear hasCompleted flag
useEffect(() => {
  if (previousRecordingState.current === 'recording' && recordingState === 'idle') {
    setHasCompleted(true);
  }
  if (recordingState === 'recording') {
    setHasCompleted(false);
  }
  previousRecordingState.current = recordingState;
}, [recordingState]);

// Derive runningStatus from hasCompleted flag and props
const runningStatus = useMemo(() => {
  if (hasCompleted) return 'complete';
  if (status === 'processing') return 'processing';
  if (recordingState === 'recording') return 'recording';
  return 'idle';
}, [hasCompleted, recordingState, status]);
```

**Props from parent** (RightPanel):
- `state: CoPilotState | null` — Current Co-Pilot state from Tauri events
- `status: CoPilotStatus` — Agent status ("idle", "processing", "stopped", "error")
- `error: string | null` — Error message if agent fails
- `recordingState: RecordingState` — Recording state from app state ("idle", "recording", "processing")
- `cycleInterval: number` — Seconds between cycles (from settings, default 60)
- `onDismissQuestion: (index: number) => void` — Callback for dismissing suggested questions (not used in v1)

### Data Flow

```
Rust Backend (copilot.rs)
    ↓ (Tauri event: copilot-updated)
App.tsx (event listener)
    ↓ (updates copilotState in app state)
RightPanel.tsx
    ↓ (passes state, status, recordingState as props)
CoPilotCardStack.tsx
    ↓ (useEffect watches state changes)
Card Diffing Logic
    ↓ (creates new CoPilotCard[] for genuinely new items)
Component State Update
    ↓ (prepends new cards, triggers animations)
DOM Render
    ↓ (cards animate in, auto-collapse timers start)
```

### Integration Points

1. **RightPanel.tsx**: Swap `CoPilotPanel` import for `CoPilotCardStack`, pass additional `recordingState` and `cycleInterval` props
2. **App.css**: Add new CSS classes for card styling, animations, and transitions
3. **types.ts**: Add `CoPilotCard`, `FinalSummaryCard`, and `CoPilotCardStackState` interfaces
4. **No backend changes**: Existing `CoPilotState`, Tauri events, and agent lifecycle unchanged

---

## Components and Interfaces

### CoPilotCardStack Component

**File**: `src/components/CoPilotCardStack.tsx`

**Responsibilities**:
- Listen for `CoPilotState` updates and create new cards via diffing logic
- Manage card state (expansion, auto-collapse timers)
- Render panel header, card stack, and sticky footer
- Handle user interactions (expand/collapse, bulk actions)
- Detect recording stop and create final summary card

**Props Interface**:
```typescript
interface CoPilotCardStackProps {
  state: CoPilotState | null;
  status: CoPilotStatus;
  error: string | null;
  recordingState: RecordingState;
  cycleInterval: number;
  onDismissQuestion: (index: number) => void;
}
```

**Internal State**:
```typescript
const [cards, setCards] = useState<CoPilotCard[]>([]);
const [finalSummaryCard, setFinalSummaryCard] = useState<FinalSummaryCard | null>(null);
const autoCollapseTimers = useRef<Map<string, TimerState>>(new Map());
const previousState = useRef<CoPilotState | null>(null);
const previousRecordingState = useRef<RecordingState>('idle');
const cardAreaRef = useRef<HTMLDivElement>(null);
```

**useEffect for Card Creation from State Changes**:
```typescript
// Watch for CoPilotState changes and create new cards
useEffect(() => {
  if (!state || state.cycle_metadata.cycle_number === 0) return;
  
  const newCards = createCardsFromStateDiff(state, previousState.current, cards);
  if (newCards.length > 0) {
    setCards(prev => [...newCards, ...prev]);
    
    // Start auto-collapse timers for new cards
    newCards.forEach(card => {
      const delay = card.type === 'summary_update' ? 5 : 8;
      startAutoCollapseTimer(card.id, delay);
    });
    
    // Scroll to top to show new cards
    cardAreaRef.current?.scrollTo({ top: 0, behavior: 'smooth' });
  }
  
  previousState.current = state;
}, [state]);
```

**useEffect for Final Summary Card Creation**:
```typescript
// Detect recording stop transition and create final summary card
useEffect(() => {
  const wasRecording = previousRecordingState.current === 'recording';
  const nowStopped = recordingState === 'idle';
  
  if (wasRecording && nowStopped && state && state.cycle_metadata.cycle_number > 0) {
    setFinalSummaryCard(createFinalSummaryCard(state));
  }
  
  previousRecordingState.current = recordingState;
}, [recordingState, state]);
```

**useEffect for Countdown Timer**:
```typescript
// Countdown timer for footer "Next cycle in ~Xs"
const [nextCycleIn, setNextCycleIn] = useState(cycleInterval);
const previousCycleNumber = useRef(0);

useEffect(() => {
  if (!state) return;
  
  const currentCycleNumber = state.cycle_metadata.cycle_number;
  
  // Reset countdown when cycle number changes (new cycle started)
  if (currentCycleNumber !== previousCycleNumber.current) {
    setNextCycleIn(cycleInterval);
    previousCycleNumber.current = currentCycleNumber;
  }
  
  // Decrement countdown every second during recording
  if (recordingState === 'recording' && status !== 'processing') {
    const interval = setInterval(() => {
      setNextCycleIn(prev => Math.max(0, prev - 1));
    }, 1000);
    
    return () => clearInterval(interval);
  }
}, [state, recordingState, status, cycleInterval]);
```

**Key Methods**:
- `createCardsFromStateDiff(newState: CoPilotState, oldState: CoPilotState | null, existingCards: CoPilotCard[]): CoPilotCard[]` — Diff logic to identify new items
- `startAutoCollapseTimer(cardId: string, duration: number)` — Start timer for a card
- `pauseAutoCollapseTimer(cardId: string)` — Pause timer when user hovers
- `resumeAutoCollapseTimer(cardId: string)` — Resume timer when hover ends
- `cancelAutoCollapseTimer(cardId: string)` — Cancel timer when user interacts
- `toggleCardExpansion(cardId: string)` — Toggle expand/collapse state
- `expandAllCards()` — Bulk expand action
- `collapseAllCards()` — Bulk collapse action
- `createFinalSummaryCard(state: CoPilotState): FinalSummaryCard` — Generate summary card from final state

**toggleCardExpansion Implementation**:
```typescript
function toggleCardExpansion(cardId: string) {
  setCards(prevCards =>
    prevCards.map(card =>
      card.id === cardId
        ? { ...card, isExpanded: !card.isExpanded, isNew: false }
        : card
    )
  );
  // Cancel auto-collapse timer when user manually interacts
  cancelAutoCollapseTimer(cardId);
}
```

### Card Component (inline)

The individual card is rendered inline within `CoPilotCardStack` (not a separate component) to simplify state management and avoid prop drilling. Each card is a `<div>` with conditional classes for animation and expansion state.

**Card Structure**:
```tsx
<div 
  className={`copilot-card ${card.isNew ? 'entering' : ''} ${card.isExpanded ? 'expanded' : 'collapsed'}`}
  onMouseEnter={() => pauseAutoCollapseTimer(card.id)}
  onMouseLeave={() => resumeAutoCollapseTimer(card.id)}
>
  <div 
    className="copilot-card-header" 
    onClick={() => toggleCardExpansion(card.id)}
    role="button"
    aria-expanded={card.isExpanded}
    tabIndex={0}
    onKeyDown={(e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        toggleCardExpansion(card.id);
      }
    }}
  >
    <span className={`copilot-card-chevron ${card.isExpanded ? 'expanded' : ''}`}>▸</span>
    <span className="copilot-card-title">{card.title}</span>
    <span className={`copilot-card-dot copilot-card-dot-${card.type}`}></span>
  </div>
  <div className="copilot-card-body">{card.body}</div>
  <div className="copilot-card-metadata">
    Cycle {card.cycle} · {formatTimestamp(card.timestamp)}
  </div>
</div>
```

**Note**: The body and metadata are always rendered in the DOM. The `.expanded` / `.collapsed` class on the parent card controls visibility via CSS `max-height` transitions. This ensures smooth animations work correctly. The `onMouseEnter` and `onMouseLeave` handlers implement the hover pause/resume behavior for auto-collapse timers.

### Final Summary Card Component (inline)

The final summary card is also rendered inline with a distinct structure:

```tsx
<div className="copilot-final-summary-card">
  <div className="copilot-card-header">
    <span className="copilot-summary-icon">📋</span>
    <span className="copilot-card-title">Session Summary</span>
  </div>
  <div className="copilot-card-body">
    {finalSummaryCard.summary && (
      <div className="summary-section">
        <h5>Summary</h5>
        <p>{finalSummaryCard.summary}</p>
      </div>
    )}
    {finalSummaryCard.keyTakeaways.length > 0 && (
      <div className="summary-section">
        <h5>Key Takeaways</h5>
        <ul>{finalSummaryCard.keyTakeaways.map((item, i) => <li key={i}>{item}</li>)}</ul>
      </div>
    )}
    {/* ... similar sections for actionItems, decisions, openQuestions */}
  </div>
</div>
```

### Panel Header Component (inline)

```tsx
<div className="copilot-panel-header">
  <div className="copilot-panel-title">
    <h3>Co-Pilot Agent</h3>
    <span className={`copilot-status-indicator ${runningStatus}`}>
      {runningStatus === 'recording' && <span className="pulse-dot" />}
      {runningStatus === 'complete' && <span className="checkmark">✓</span>}
    </span>
  </div>
  <div className="copilot-bulk-actions">
    <button onClick={expandAllCards}>Expand All</button>
    <span className="separator">|</span>
    <button onClick={collapseAllCards}>Collapse All</button>
  </div>
</div>
```

### Sticky Footer Component (inline)

```tsx
<div className="copilot-sticky-footer">
  <div className="copilot-footer-status">
    <span className={`status-indicator status-${runningStatus}`}>
      {runningStatus === 'processing' && <span className="pulse-dot" />}
      {runningStatus === 'complete' && <span className="checkmark">✓</span>}
    </span>
    <span className="status-text">
      {runningStatus === 'recording' && `Cycle ${currentCycle} in ~${nextCycleIn}s`}
      {runningStatus === 'processing' && `Processing cycle ${currentCycle}...`}
      {runningStatus === 'complete' && `Session complete`}
    </span>
  </div>
  <div className="copilot-footer-stats">
    {cyclesDone} cycles · {formatDuration(totalAudioAnalyzed)}
  </div>
</div>
```

---

## Data Models

### CoPilotCard Interface

```typescript
interface CoPilotCard {
  /** Unique identifier: "cycle-{N}-{type}-{index}" */
  id: string;
  
  /** Card type classification */
  type: 'insight' | 'decision' | 'action_item' | 'question' | 'summary_update';
  
  /** Short title extracted from body (max 60 chars) */
  title: string;
  
  /** Full insight text */
  body: string;
  
  /** Cycle number that produced this card */
  cycle: number;
  
  /** ISO 8601 timestamp when card was created */
  timestamp: string;
  
  /** Whether card is currently expanded */
  isExpanded: boolean;
  
  /** Whether card is new (true until auto-collapse or user interaction) */
  isNew: boolean;
}
```

### FinalSummaryCard Interface

```typescript
interface FinalSummaryCard {
  /** Final running summary text */
  summary: string;
  
  /** Accumulated key points */
  keyTakeaways: string[];
  
  /** Accumulated action items */
  actionItems: string[];
  
  /** Accumulated decisions */
  decisions: string[];
  
  /** Accumulated open questions */
  openQuestions: string[];
}
```

### CoPilotCardStackState Interface

```typescript
interface CoPilotCardStackState {
  /** All cards created from CoPilotState updates */
  cards: CoPilotCard[];
  
  /** Current running status */
  runningStatus: 'idle' | 'recording' | 'processing' | 'complete';
  
  /** Current cycle number */
  currentCycle: number;
  
  /** Seconds until next cycle */
  nextCycleIn: number;
  
  /** Total audio analyzed in seconds */
  totalAudioAnalyzed: number;
  
  /** Final summary card (null until recording stops) */
  finalSummaryCard: FinalSummaryCard | null;
}
```

### Card Creation Logic

**Deterministic ID Generation**:
```typescript
function generateCardId(cycle: number, type: string, index: number): string {
  return `cycle-${cycle}-${type}-${index}`;
}
```

**Title Extraction Heuristic**:
```typescript
function extractTitle(body: string): string {
  // Find first clause before comma, period, or dash
  const match = body.match(/^([^,.\-—]+)/);
  const title = match ? match[1].trim() : body;
  
  // Cap at 60 characters with ellipsis
  return title.length > 60 ? title.substring(0, 57) + '...' : title;
}
```

**Card Type Mapping**:
```typescript
const CARD_TYPE_MAP = {
  key_points: 'insight',
  decisions: 'decision',
  action_items: 'action_item',
  open_questions: 'question',
  running_summary: 'summary_update',
} as const;
```

**Card Priority Ordering** (within a single cycle):
```typescript
const CARD_TYPE_PRIORITY = {
  decision: 1,
  action_item: 2,
  question: 3,
  insight: 4,
  summary_update: 5,
};
```

### Diffing Algorithm

The diffing logic compares the new `CoPilotState` against the previous state to identify genuinely new items:

```typescript
function createCardsFromStateDiff(
  newState: CoPilotState,
  oldState: CoPilotState | null,
  existingCards: CoPilotCard[]
): CoPilotCard[] {
  const newCards: CoPilotCard[] = [];
  const cycle = newState.cycle_metadata.cycle_number;
  const timestamp = newState.cycle_metadata.last_updated_at;
  
  // Helper to check if item already exists in existing cards
  const existsInCards = (body: string): boolean => {
    return existingCards.some(card => card.body === body);
  };
  
  // Check key_points for new insights
  newState.key_points.forEach((point, index) => {
    if (!existsInCards(point)) {
      newCards.push({
        id: generateCardId(cycle, 'insight', index),
        type: 'insight',
        title: extractTitle(point),
        body: point,
        cycle,
        timestamp,
        isExpanded: true,
        isNew: true,
      });
    }
  });
  
  // Check decisions for new decision cards
  newState.decisions.forEach((decision, index) => {
    if (!existsInCards(decision)) {
      newCards.push({
        id: generateCardId(cycle, 'decision', index),
        type: 'decision',
        title: extractTitle(decision),
        body: decision,
        cycle,
        timestamp,
        isExpanded: true,
        isNew: true,
      });
    }
  });
  
  // Check action_items for new action cards
  newState.action_items.forEach((item, index) => {
    if (!existsInCards(item)) {
      newCards.push({
        id: generateCardId(cycle, 'action_item', index),
        type: 'action_item',
        title: extractTitle(item),
        body: item,
        cycle,
        timestamp,
        isExpanded: true,
        isNew: true,
      });
    }
  });
  
  // Check open_questions for new question cards
  newState.open_questions.forEach((question, index) => {
    if (!existsInCards(question)) {
      newCards.push({
        id: generateCardId(cycle, 'question', index),
        type: 'question',
        title: extractTitle(question),
        body: question,
        cycle,
        timestamp,
        isExpanded: true,
        isNew: true,
      });
    }
  });
  
  // Check running_summary for changes (create summary_update card if changed)
  if (!oldState || newState.running_summary !== oldState.running_summary) {
    newCards.push({
      id: generateCardId(cycle, 'summary_update', 0),
      type: 'summary_update',
      title: extractTitle(newState.running_summary),
      body: newState.running_summary,
      cycle,
      timestamp,
      isExpanded: true,
      isNew: true,
    });
  }
  
  // Sort by priority (decision > action_item > question > insight > summary_update)
  newCards.sort((a, b) => CARD_TYPE_PRIORITY[a.type] - CARD_TYPE_PRIORITY[b.type]);
  
  return newCards;
}
```

### Auto-Collapse Timer Management

```typescript
interface TimerState {
  timeout: NodeJS.Timeout | null;
  startedAt: number;
  duration: number;
  remaining: number;
}

const autoCollapseTimers = useRef<Map<string, TimerState>>(new Map());

function startAutoCollapseTimer(cardId: string, duration: number) {
  // Cancel existing timer if any
  cancelAutoCollapseTimer(cardId);
  
  // Start new timer
  const timeout = setTimeout(() => {
    setCards(prevCards =>
      prevCards.map(card =>
        card.id === cardId
          ? { ...card, isExpanded: false, isNew: false }
          : card
      )
    );
    autoCollapseTimers.current.delete(cardId);
  }, duration * 1000);
  
  autoCollapseTimers.current.set(cardId, {
    timeout,
    startedAt: Date.now(),
    duration: duration * 1000,
    remaining: duration * 1000,
  });
}

function pauseAutoCollapseTimer(cardId: string) {
  const timerState = autoCollapseTimers.current.get(cardId);
  if (timerState && timerState.timeout) {
    clearTimeout(timerState.timeout);
    const elapsed = Date.now() - timerState.startedAt;
    timerState.remaining = timerState.duration - elapsed;
    timerState.timeout = null;
  }
}

function resumeAutoCollapseTimer(cardId: string) {
  const timerState = autoCollapseTimers.current.get(cardId);
  if (timerState && !timerState.timeout && timerState.remaining > 0) {
    const timeout = setTimeout(() => {
      setCards(prevCards =>
        prevCards.map(card =>
          card.id === cardId
            ? { ...card, isExpanded: false, isNew: false }
            : card
        )
      );
      autoCollapseTimers.current.delete(cardId);
    }, timerState.remaining);
    
    timerState.timeout = timeout;
    timerState.startedAt = Date.now();
    timerState.duration = timerState.remaining;
  }
}

function cancelAutoCollapseTimer(cardId: string) {
  const timerState = autoCollapseTimers.current.get(cardId);
  if (timerState) {
    if (timerState.timeout) {
      clearTimeout(timerState.timeout);
    }
    autoCollapseTimers.current.delete(cardId);
  }
}

// Cleanup on unmount
useEffect(() => {
  return () => {
    autoCollapseTimers.current.forEach(timerState => {
      if (timerState.timeout) {
        clearTimeout(timerState.timeout);
      }
    });
    autoCollapseTimers.current.clear();
  };
}, []);
```

---

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property Reflection

After analyzing all acceptance criteria, I identified the following testable properties. Many UI-specific criteria (CSS animations, styling, positioning) are not amenable to property-based testing and will be validated through manual testing and visual regression testing instead.

**Redundancy Analysis**:
- Properties 1.4 and 1.5 (ID generation and title extraction) are independent utility functions
- Property 2.1 (deduplication) subsumes 2.3 (exact string match) since deduplication is the behavior and exact match is the implementation
- Properties 2.4 and 2.5 (prepending and priority sorting) can be combined into a single property about card ordering
- Properties 4.1 and 4.3 (card states and toggle) can be combined since toggling is the mechanism for state change
- Properties 5.2 and 5.5 (expanded by default and isNew flag) are both about initial card state and can be combined
- Properties 6.1, 6.2, 6.3 are all about auto-collapse timer behavior and are better tested as examples (specific scenarios) rather than properties
- Property 7.4 (cards remain after recording stops) is a specific case of state persistence
- Properties 10.4 and 10.5 (Expand All and Collapse All) are inverse operations and can be tested together

### Property 1: Card ID Determinism

*For any* cycle number, card type, and index, the `generateCardId` function should produce the same ID string following the pattern `cycle-{N}-{type}-{index}`, and calling it multiple times with the same inputs should always return identical results.

**Validates: Requirements 1.4**

### Property 2: Title Extraction Format

*For any* body text, the `extractTitle` function should return a string that: (1) contains only the first clause before a comma, period, or dash, (2) is trimmed of whitespace, (3) is at most 60 characters long, and (4) ends with "..." if the original clause exceeded 60 characters.

**Validates: Requirements 1.5**

### Property 3: Card Deduplication

*For any* `CoPilotState` update and existing card list, the diffing logic should create new cards only for items whose body text does not exactly match any existing card's body text, ensuring no duplicate cards are created.

**Validates: Requirements 2.1, 2.3**

### Property 4: Card Type Mapping

*For any* item from a `CoPilotState` field (key_points, decisions, action_items, open_questions, running_summary), the created card should have the correct type: `insight` for key_points, `decision` for decisions, `action_item` for action_items, `question` for open_questions, and `summary_update` for running_summary changes.

**Validates: Requirements 2.2**

### Property 5: Card Ordering

*For any* set of new cards created from a single cycle, when prepended to the existing cards array, the new cards should appear at the beginning of the array (newest first) and should be sorted among themselves by priority: decision > action_item > question > insight > summary_update.

**Validates: Requirements 2.4, 2.5**

### Property 6: Excluded Fields

*For any* `CoPilotState` containing `suggested_questions` or `key_concepts`, the diffing logic should not create any cards for these fields during live recording.

**Validates: Requirements 2.6**

### Property 7: Card Expansion Toggle

*For any* card in the cards array, toggling its expansion state should flip the `isExpanded` boolean value, and the card should have exactly two possible states: expanded (isExpanded = true) or collapsed (isExpanded = false).

**Validates: Requirements 4.1, 4.3**

### Property 8: Metadata Visibility

*For any* card, the metadata (cycle number and timestamp) should be visually hidden when collapsed (via CSS `max-height: 0` and `opacity: 0`) and visible when expanded (via CSS `max-height: 50px` and `opacity: 1`). The metadata elements remain in the DOM at all times.

**Validates: Requirements 4.6**

### Property 9: New Card Initial State

*For any* newly created card, it should have `isExpanded: true` and `isNew: true` at the moment of creation, ensuring new insights are immediately visible to the user.

**Validates: Requirements 5.2, 5.5**

### Property 10: Auto-Collapse Timer Independence

*For any* set of cards with active auto-collapse timers, each card's timer should operate independently such that one timer completing or being cancelled does not affect any other card's timer or state.

**Validates: Requirements 6.5**

### Property 11: Card Persistence After Recording Stop

*For any* set of cards that exist when recording stops (recordingState transitions from 'recording' to 'idle'), all cards should remain in the cards array with their current state (isExpanded, isNew) unchanged.

**Validates: Requirements 7.4**

### Property 12: Bulk Expand/Collapse

*For any* set of cards, calling `expandAllCards()` should set `isExpanded: true` on all cards (except final summary which is always expanded), and calling `collapseAllCards()` should set `isExpanded: false` on all cards and cancel all active auto-collapse timers.

**Validates: Requirements 10.4, 10.5**

### Property 13: Final Summary Card Content

*For any* `CoPilotState` used to create a final summary card, the card should contain exactly the non-empty sections from the state: summary (if running_summary is non-empty), keyTakeaways (if key_points has items), actionItems (if action_items has items), decisions (if decisions has items), and openQuestions (if open_questions has items).

**Validates: Requirements 8.6, 8.7**

### Property 14: Placeholder Display

*For any* component state where `state` is null or `state.cycle_metadata.cycle_number === 0`, the component should render the placeholder message instead of the card stack.

**Validates: Requirements 11.9**

---

## Error Handling

### Error Scenarios

1. **Invalid CoPilotState structure**: If the incoming state is malformed or missing required fields, the component should gracefully handle it by:
   - Logging a warning to console
   - Not creating any cards from the invalid state
   - Displaying the existing cards without modification
   - Not crashing or entering an error state

2. **Timer cleanup on unmount**: If the component unmounts while auto-collapse timers are active:
   - All timers should be cancelled in the cleanup function
   - No memory leaks from orphaned timers
   - No state updates attempted on unmounted component

3. **Rapid state updates**: If multiple `copilot-updated` events arrive in quick succession:
   - Each update should be processed sequentially
   - Diffing logic should use the most recent previous state
   - No race conditions in card creation
   - Auto-collapse timers should not interfere with each other

4. **Empty state transitions**: If recording stops before any cycles complete (state is null or cycle 0):
   - No final summary card should be created
   - Placeholder should remain visible
   - No errors should be thrown

5. **Title extraction edge cases**: If body text is empty, very short, or contains only special characters:
   - `extractTitle` should return a valid string (empty string if body is empty)
   - No crashes from regex failures
   - Ellipsis should only be added if truncation occurred

### Error Recovery

- **Graceful degradation**: If card creation fails for a specific item, log the error and continue processing other items
- **State consistency**: If an error occurs during state update, the component should maintain the previous valid state
- **User feedback**: Errors from the backend (via `error` prop) should be displayed in the panel header with a warning icon

---

## Testing Strategy

### Dual Testing Approach

This feature requires both unit tests and property-based tests to ensure comprehensive coverage:

**Unit Tests** focus on:
- Specific examples of title extraction (empty string, very long text, text with multiple delimiters)
- Edge cases for card creation (empty state, first cycle, state with no changes)
- Timer behavior (auto-collapse after 8s, cancellation on click, pause on hover)
- Component lifecycle (mount, unmount, state transitions)
- Integration with RightPanel (prop passing, conditional rendering)

**Property-Based Tests** focus on:
- Universal properties that hold for all inputs (ID generation, title extraction format, deduplication)
- Randomized state updates to verify diffing logic correctness
- Bulk operations on arbitrary card sets (expand all, collapse all)
- State consistency across random sequences of operations

### Property-Based Testing Configuration

**Library**: `fast-check` (JavaScript/TypeScript property-based testing library)

**Configuration**:
- Minimum 100 iterations per property test
- Each test tagged with: `Feature: copilot-card-stack-ux, Property {N}: {property_text}`
- Custom generators for `CoPilotState`, `CoPilotCard`, and body text strings

**Example Test Structure**:
```typescript
import fc from 'fast-check';

// Feature: copilot-card-stack-ux, Property 1: Card ID Determinism
test('generateCardId produces deterministic IDs', () => {
  fc.assert(
    fc.property(
      fc.integer({ min: 1, max: 100 }), // cycle
      fc.constantFrom('insight', 'decision', 'action_item', 'question', 'summary_update'), // type
      fc.integer({ min: 0, max: 50 }), // index
      (cycle, type, index) => {
        const id1 = generateCardId(cycle, type, index);
        const id2 = generateCardId(cycle, type, index);
        
        expect(id1).toBe(id2);
        expect(id1).toMatch(/^cycle-\d+-\w+-\d+$/);
        expect(id1).toBe(`cycle-${cycle}-${type}-${index}`);
      }
    ),
    { numRuns: 100 }
  );
});

// Feature: copilot-card-stack-ux, Property 3: Card Deduplication
test('diffing logic prevents duplicate cards', () => {
  fc.assert(
    fc.property(
      arbitraryCoPilotState(), // custom generator
      fc.array(arbitraryCard()), // existing cards
      (newState, existingCards) => {
        const newCards = createCardsFromStateDiff(newState, null, existingCards);
        
        // No new card should have a body matching an existing card
        newCards.forEach(newCard => {
          const isDuplicate = existingCards.some(
            existing => existing.body === newCard.body
          );
          expect(isDuplicate).toBe(false);
        });
      }
    ),
    { numRuns: 100 }
  );
});
```

### Unit Test Coverage

**Core Functions**:
- `generateCardId`: Test pattern matching, uniqueness, determinism
- `extractTitle`: Test empty string, short text, long text, multiple delimiters, special characters
- `createCardsFromStateDiff`: Test empty state, first cycle, incremental updates, no changes
- `startAutoCollapseTimer`: Test timer creation, duration, callback execution
- `cancelAutoCollapseTimer`: Test timer cancellation, cleanup
- `toggleCardExpansion`: Test state flip, timer cancellation on manual toggle
- `expandAllCards`: Test bulk expansion, timer cancellation
- `collapseAllCards`: Test bulk collapse, timer cancellation
- `createFinalSummaryCard`: Test empty sections, full sections, null state

**Component Behavior**:
- Placeholder rendering when state is null or cycle 0
- Card rendering with correct structure (header, body, metadata)
- Final summary card rendering with conditional sections
- Footer rendering with correct status and countdown
- Header rendering with bulk action buttons
- State transitions (idle → recording → complete)

**Integration**:
- RightPanel passes correct props to CoPilotCardStack
- Tab visibility logic when recording stops
- Event listener setup and cleanup

### Manual Testing Checklist

Since many requirements involve CSS animations and visual behavior, manual testing is essential:

- [ ] New cards animate in smoothly (slide + fade)
- [ ] New cards display pulse animation for 2 seconds
- [ ] Cards auto-collapse after 8 seconds (or 5s for summary_update)
- [ ] Clicking card header toggles expansion with smooth animation
- [ ] Chevron rotates 90 degrees when expanding
- [ ] Body text animates open/closed with max-height transition
- [ ] Hovering on a card pauses the auto-collapse timer
- [ ] Leaving hover resumes the timer from where it paused
- [ ] Clicking header during auto-collapse cancels the timer
- [ ] Expand All button expands all cards instantly
- [ ] Collapse All button collapses all cards and cancels timers
- [ ] Final summary card appears when recording stops
- [ ] Final summary card is visually distinct (border, background)
- [ ] Final summary card does not collapse when clicked
- [ ] Footer stays visible at bottom when scrolling cards
- [ ] Footer countdown decrements every second
- [ ] Footer shows correct status during recording/processing/complete
- [ ] Panel persists after recording stops (doesn't disappear)
- [ ] Panel header shows checkmark when recording completes
- [ ] Type dots display correct colors for each card type
- [ ] Scroll position resets to top when new cards arrive

---

## Implementation Notes

### Performance Considerations

1. **Card limit**: The component should handle up to 50 cards without performance degradation. Beyond 50 cards, consider implementing virtualization (deferred to future iteration).

2. **Timer management**: Use `useRef` for timer storage to avoid stale closures. Clean up all timers on unmount to prevent memory leaks.

3. **Diffing efficiency**: The diffing algorithm is O(n*m) where n = new items and m = existing cards. For typical sessions (< 50 cards), this is acceptable. If performance becomes an issue, consider using a Set for O(1) lookup.

4. **Re-render optimization**: Use `React.memo` for individual card rendering if profiling shows excessive re-renders. The current inline approach is simpler but may need optimization for long sessions.

5. **Countdown timer implementation**: The footer countdown should use a `setInterval` that decrements every second. The interval should reset when `state.cycle_metadata.cycle_number` changes (indicating a new cycle started), not when `status` changes (which can flicker). Use a `useRef` to track the previous cycle number to detect transitions reliably.

### Accessibility

1. **Keyboard navigation**: Cards should be focusable via Tab key. Enter or Space should toggle expansion.

2. **ARIA attributes**: 
   - Card headers should have `role="button"` and `aria-expanded` attribute
   - Chevron state should be communicated via `aria-expanded="true"` or `aria-expanded="false"`
   - Final summary card should have `aria-label="Session Summary Card"`

3. **Screen reader announcements**: When new cards arrive, consider using an ARIA live region to announce "New insight added" (deferred to future iteration).

4. **Focus management**: When Expand All is clicked, focus should remain on the button (not jump to first card).

### CSS Architecture

All styles should be added to `src/App.css` using existing design tokens:

**Design Tokens**:
- `--bg-primary`: Card background
- `--bg-secondary`: Footer background
- `--bg-elevated`: Final summary card background
- `--text-primary`: Card title and body text
- `--text-secondary`: Card metadata text
- `--border-color`: Card borders
- `--accent-primary`: Final summary card left border

**New CSS Variables** (to be added):
```css
:root {
  --copilot-card-dot-insight: #3B82F6;
  --copilot-card-dot-decision: #22C55E;
  --copilot-card-dot-action: #F59E0B;
  --copilot-card-dot-question: #EF4444;
  --copilot-card-dot-summary: #8B5CF6;
}
```

**Animation Keyframes**:
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

@keyframes subtlePulse {
  0%, 100% {
    box-shadow: 0 0 0 0 rgba(59, 130, 246, 0.15);
  }
  50% {
    box-shadow: 0 0 8px 2px rgba(59, 130, 246, 0.15);
  }
}
```

**Note**: The `subtlePulse` animation uses a fixed blue color at 15% opacity. For type-specific pulse colors, the card can set a CSS custom property inline: `style={{ '--pulse-color': 'rgba(59, 130, 246, 0.15)' }}` and the keyframe can reference `var(--pulse-color)`. The current design uses a fixed color for simplicity.

**Card Collapse/Expand Styles**:
```css
/* Body and metadata are always in DOM, controlled by parent class */
.copilot-card.collapsed .copilot-card-body {
  max-height: 0;
  padding: 0;
  overflow: hidden;
  transition: max-height 200ms ease, padding 200ms ease;
}

.copilot-card.expanded .copilot-card-body {
  max-height: 300px;
  padding: 8px 12px;
  overflow: hidden;
  transition: max-height 200ms ease, padding 200ms ease;
}

.copilot-card.collapsed .copilot-card-metadata {
  max-height: 0;
  padding: 0;
  overflow: hidden;
  opacity: 0;
  transition: max-height 200ms ease, padding 200ms ease, opacity 200ms ease;
}

.copilot-card.expanded .copilot-card-metadata {
  max-height: 50px;
  padding: 4px 12px 8px 12px;
  opacity: 1;
  transition: max-height 200ms ease, padding 200ms ease, opacity 200ms ease;
}
```

### Future Enhancements (Out of Scope for v1)

1. **Card dismissal**: Swipe-to-dismiss or X button to remove cards
2. **Card filtering**: Filter cards by type (show only decisions, only questions, etc.)
3. **Card search**: Search within card bodies
4. **Suggested questions popover**: Render suggested questions as a separate popover interaction
5. **Key concepts live cards**: Render key concepts as individual cards during recording
6. **Configurable auto-collapse duration**: User setting for timer duration
7. **Card virtualization**: For sessions with >50 cards
8. **Card persistence**: Save card state to localStorage or Tauri app state
9. **Drag-to-reorder**: Manual card reordering
10. **Export cards**: Export cards as markdown or JSON

---

## Migration Plan

### Phase 1: Create New Component (No Breaking Changes)

1. Add new TypeScript interfaces to `src/state/types.ts`
2. Create `src/components/CoPilotCardStack.tsx` with full implementation
3. Add new CSS to `src/App.css`
4. Keep `CoPilotPanel.tsx` unchanged (not deleted)

### Phase 2: Integration (Swap Components in RightPanel)

1. Import `CoPilotCardStack` in `RightPanel.tsx`
2. Replace `CoPilotPanel` with `CoPilotCardStack` in the render logic
3. Pass additional props (`recordingState`, `cycleInterval`) to `CoPilotCardStack`
4. Update tab visibility logic to keep Co-Pilot tab visible after recording stops
5. Test the new component thoroughly

### Phase 3: Cleanup

1. Comment out `CoPilotPanel` import in `RightPanel.tsx` (keep file for reference)
2. Final testing and polish

### Rollback Plan

If critical issues are discovered after deployment:
1. Revert `RightPanel.tsx` to import and render `CoPilotPanel`
2. No data loss (backend unchanged)
3. No migration needed (component state is ephemeral)

---

## Appendix: Example Card Stack State

### Example 1: Mid-Recording State

```typescript
{
  cards: [
    {
      id: "cycle-3-decision-0",
      type: "decision",
      title: "Use React for frontend framework",
      body: "Use React for frontend framework, given team expertise and ecosystem maturity",
      cycle: 3,
      timestamp: "2024-02-28T15:23:45Z",
      isExpanded: true,
      isNew: true
    },
    {
      id: "cycle-3-action_item-0",
      type: "action_item",
      title: "Set up CI/CD pipeline",
      body: "Set up CI/CD pipeline with GitHub Actions for automated testing and deployment",
      cycle: 3,
      timestamp: "2024-02-28T15:23:45Z",
      isExpanded: true,
      isNew: true
    },
    {
      id: "cycle-2-insight-0",
      type: "insight",
      title: "Performance is critical for user experience",
      body: "Performance is critical for user experience, especially on mobile devices with limited resources",
      cycle: 2,
      timestamp: "2024-02-28T15:22:30Z",
      isExpanded: false,
      isNew: false
    },
    {
      id: "cycle-1-summary_update-0",
      type: "summary_update",
      title: "Team is discussing architecture decisions for the new...",
      body: "Team is discussing architecture decisions for the new web application. Focus on scalability and maintainability.",
      cycle: 1,
      timestamp: "2024-02-28T15:21:15Z",
      isExpanded: false,
      isNew: false
    }
  ],
  runningStatus: "recording",
  currentCycle: 3,
  nextCycleIn: 45,
  totalAudioAnalyzed: 180,
  finalSummaryCard: null
}
```

### Example 2: Recording Complete State

```typescript
{
  cards: [
    // ... all cards from recording session
  ],
  runningStatus: "complete",
  currentCycle: 5,
  nextCycleIn: 0,
  totalAudioAnalyzed: 300,
  finalSummaryCard: {
    summary: "Team discussed architecture for new web app, decided on React + TypeScript stack, identified 3 action items for next sprint.",
    keyTakeaways: [
      "Performance is critical for user experience",
      "Team has strong React expertise",
      "Scalability is a primary concern"
    ],
    actionItems: [
      "Set up CI/CD pipeline with GitHub Actions",
      "Create initial project structure",
      "Schedule architecture review meeting"
    ],
    decisions: [
      "Use React for frontend framework",
      "Use TypeScript for type safety",
      "Deploy to AWS for scalability"
    ],
    openQuestions: [
      "Which state management library should we use?",
      "How should we handle authentication?"
    ]
  }
}
```

---

## Summary

This design transforms the Co-Pilot agent display from a static document into an animated, card-based interface that reduces cognitive load and improves glanceability. The implementation is frontend-only, requiring no backend changes, and maintains backward compatibility by keeping the existing `CoPilotPanel` component. The card stack provides a superior user experience through auto-collapse behavior, visual animations, and persistent session summaries, while remaining performant and accessible.
