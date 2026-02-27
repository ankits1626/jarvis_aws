# Design: Three-Panel Layout Redesign

## Overview

This design transforms the Jarvis desktop app from a single-column layout with modal overlays into a modern three-panel desktop interface (Left Nav | Center Content | Right Context Panel). The redesign addresses UX issues including excessive scrolling, modal overlays blocking the main view, inline content cluttering lists, and poor horizontal space utilization.

### Current Architecture

The existing App.tsx uses a single-column layout (`div.app > div.container`) where all content stacks vertically:
- Header with hamburger menu and settings button
- Status display and record button
- Recordings list with inline transcripts
- Audio player (inline after recordings)
- Live transcript display (inline)
- Modal overlays for Settings, YouTube, Browser, and Gems (rendered as `dialog-overlay` wrappers)

State management uses boolean flags (`showSettings`, `showYouTube`, `showBrowserTool`, `showGems`, `showHamburgerMenu`) to control overlay visibility.

### New Architecture

The new layout introduces three persistent panels:

1. **Left Navigation Panel** (~140px expanded, ~48px collapsed)
   - Persistent navigation with icons and labels
   - Nav items: Record, Recordings, Gems, YouTube, Browser, Settings
   - Collapsible state with smooth transitions
   - Always visible (no hamburger menu)

2. **Center Content Panel** (flex: 1, min-width: 300px)
   - Content routing based on `activeNav` state
   - Displays: recording controls, recordings list, gems panel, YouTube section, browser tool, or settings
   - Independently scrollable
   - No modal overlays

3. **Right Context Panel** (flex: 1, max-width: 50%)
   - Context-sensitive detail view
   - Displays: recording details with audio player, gem details, live transcript, or extraction results
   - Independently scrollable
   - Collapsible to width 0 when not needed

### State Management Changes

Replace boolean overlay flags with state-driven navigation:
- Remove: `showSettings`, `showYouTube`, `showBrowserTool`, `showGems`, `showHamburgerMenu`
- Add: `activeNav: 'record' | 'recordings' | 'gems' | 'youtube' | 'browser' | 'settings'` (default: `'record'`)
- Add: `leftNavCollapsed: boolean` (default: `false`)
- Add: `selectedGemId: string | null` (for gem detail in right panel)
- Keep: `state.selectedRecording` (already exists for recording selection)
- Keep: `youtubeNotification: boolean` (event-driven notification badge, moves from hamburger to left nav)

### Component Changes

**New Components:**
- `LeftNav.tsx` - Navigation sidebar with collapsible state
- `RightPanel.tsx` - Context panel router based on activeNav and selection state
- `RecordingDetailPanel.tsx` - Recording detail view for right panel
- `GemDetailPanel.tsx` - Gem detail view for right panel

**Modified Components:**
- `App.tsx` - Major refactor to three-panel layout, remove overlay logic
- `GemsPanel.tsx` - Expose selected gem via callback prop
- `Settings.tsx`, `YouTubeSection.tsx`, `BrowserTool.tsx` - Remove or make `onClose` prop optional (no longer overlays)

### CSS Architecture

Replace single-column `.container` styles with flexbox three-panel layout:
- `.app-layout` - Flex container (display: flex, height: 100vh)
- `.left-nav` - Fixed/collapsible width with transitions
- `.center-panel` - Flex: 1, independently scrollable
- `.right-panel` - Flex: 1, max-width: 50%, collapsible
- Remove `.dialog-overlay` styles for converted components

---

## Architecture

### Component Hierarchy

```
App.tsx
├── LeftNav.tsx
│   ├── NavItem (Record)
│   ├── NavItem (Recordings)
│   ├── NavItem (Gems)
│   ├── NavItem (YouTube) [with notification badge]
│   ├── NavItem (Browser)
│   ├── NavItem (Settings) [bottom-aligned]
│   └── CollapseToggle
├── CenterPanel (inline in App.tsx)
│   ├── [activeNav === 'record'] → Recording Controls
│   ├── [activeNav === 'recordings'] → Recordings List
│   ├── [activeNav === 'gems'] → GemsPanel
│   ├── [activeNav === 'youtube'] → YouTubeSection
│   ├── [activeNav === 'browser'] → BrowserTool
│   └── [activeNav === 'settings'] → Settings
└── RightPanel.tsx
    ├── [activeNav === 'record'] → Live Transcript or Placeholder
    ├── [activeNav === 'recordings' && selectedRecording] → RecordingDetailPanel
    ├── [activeNav === 'gems' && selectedGemId] → GemDetailPanel
    ├── [activeNav === 'youtube' && extractionResult] → Extraction Result
    ├── [activeNav === 'browser' && extractionResult] → Extraction Result
    └── [default] → Placeholder Message
```

### State Flow

```
User clicks nav item
  → Update activeNav state
  → Center panel renders corresponding content
  → Right panel updates based on activeNav + selection state

User clicks recording in list
  → Update state.selectedRecording (existing)
  → Right panel renders RecordingDetailPanel with audio player

User clicks gem in GemsPanel
  → GemsPanel calls onGemSelect callback
  → Update selectedGemId state
  → Right panel renders GemDetailPanel

User starts recording
  → Recording state updates (existing)
  → Right panel shows live TranscriptDisplay
```

### Layout Constraints

- Minimum window width: ~900px (to accommodate three panels comfortably)
- Left nav: 140px expanded, 48px collapsed
- Center panel: min-width 300px, flex: 1
- Right panel: max-width 50%, min-width 0 (collapsible), flex: 1
- All panels: independent scrolling with overflow-y: auto

---

## Components and Interfaces

### LeftNav Component

**File:** `src/components/LeftNav.tsx`

**Props:**
```typescript
interface LeftNavProps {
  activeNav: ActiveNav;
  onNavChange: (nav: ActiveNav) => void;
  youtubeNotification: boolean;
  collapsed: boolean;
  onToggleCollapse: () => void;
}
```

**Responsibilities:**
- Render navigation items with icons and labels
- Highlight active nav item
- Display YouTube notification badge when `youtubeNotification` is true
- Render collapse/expand toggle button
- Handle nav item clicks and call `onNavChange`
- Apply collapsed/expanded styles based on `collapsed` prop

**Structure:**
```tsx
<nav className={`left-nav ${collapsed ? 'collapsed' : 'expanded'}`}>
  <div className="nav-items">
    <NavItem icon="⏺" label="Record" active={activeNav === 'record'} onClick={() => onNavChange('record')} />
    <NavItem icon="📝" label="Recordings" active={activeNav === 'recordings'} onClick={() => onNavChange('recordings')} />
    <NavItem icon="💎" label="Gems" active={activeNav === 'gems'} onClick={() => onNavChange('gems')} />
    <NavItem icon="📹" label="YouTube" active={activeNav === 'youtube'} onClick={() => onNavChange('youtube')} notification={youtubeNotification} />
    <NavItem icon="🌐" label="Browser" active={activeNav === 'browser'} onClick={() => onNavChange('browser')} />
  </div>
  <div className="nav-bottom">
    <NavItem icon="⚙️" label="Settings" active={activeNav === 'settings'} onClick={() => onNavChange('settings')} />
    <button className="collapse-toggle" onClick={onToggleCollapse}>
      {collapsed ? '→' : '←'}
    </button>
  </div>
</nav>
```

### RightPanel Component

**File:** `src/components/RightPanel.tsx`

**Props:**
```typescript
interface RightPanelProps {
  activeNav: ActiveNav;
  selectedRecording: string | null;
  selectedGemId: string | null;
  recordingState: 'idle' | 'recording' | 'processing';
  transcript: TranscriptionSegment[];
  transcriptionStatus: 'idle' | 'active' | 'error' | 'disabled';
  transcriptionError: string | null;
  audioUrl: string | null;
  onClosePlayer: () => void;
  // Additional props for recording transcription and gem saving
  recordingStates: Record<string, RecordingTranscriptionState>;
  onTranscribeRecording: (filename: string) => Promise<void>;
  onSaveGem: (filename: string) => Promise<void>;
  aiAvailable: boolean;
}
```

**Responsibilities:**
- Route to appropriate content based on `activeNav` and selection state
- Render placeholder messages when no content is selected
- Support collapsing to width 0 for settings view
- Manage independent scrolling

**Routing Logic:**
```typescript
if (activeNav === 'record') {
  if (recordingState === 'recording') {
    return <TranscriptDisplay transcript={transcript} status={transcriptionStatus} error={transcriptionError} />;
  }
  return <Placeholder message="Start recording to see live transcript" />;
}

if (activeNav === 'recordings') {
  if (selectedRecording) {
    return <RecordingDetailPanel recording={selectedRecording} audioUrl={audioUrl} onClose={onClosePlayer} {...} />;
  }
  return <Placeholder message="Select a recording to play or transcribe" />;
}

if (activeNav === 'gems') {
  if (selectedGemId) {
    return <GemDetailPanel gemId={selectedGemId} onDelete={handleGemDelete} />;
  }
  return <Placeholder message="Select a gem to view details" />;
}

// For youtube, browser, settings - return null or minimal content
return null;
```

### RecordingDetailPanel Component

**File:** `src/components/RecordingDetailPanel.tsx`

**Props:**
```typescript
interface RecordingDetailPanelProps {
  recording: RecordingMetadata;
  audioUrl: string | null;
  onClose: () => void;
  recordingState: RecordingTranscriptionState;
  onTranscribe: () => Promise<void>;
  onSaveGem: () => Promise<void>;
  aiAvailable: boolean;
}
```

**Responsibilities:**
- Display recording metadata (filename, date, duration, size)
- Render audio player with controls
- Show transcribe button (when AI available)
- Display transcript result after transcription
- Show "Save as Gem" / "Update Gem" button after transcription
- Display gem status indicator

**Structure:**
```tsx
<div className="recording-detail-panel">
  <div className="detail-header">
    <h3>{recording.filename}</h3>
    <button className="close-button" onClick={onClose}>✕</button>
  </div>
  
  <div className="recording-metadata">
    <div>{formatDate(recording.created_at)}</div>
    <div>{formatTime(recording.duration_seconds)} • {formatFileSize(recording.size_bytes)}</div>
    {recordingState.hasGem && <span className="gem-indicator">💎 Has gem</span>}
  </div>
  
  <div className="audio-player-section">
    <audio controls src={audioUrl} autoPlay />
  </div>
  
  {aiAvailable && (
    <button onClick={onTranscribe} disabled={recordingState.transcribing}>
      {recordingState.transcribing ? 'Transcribing...' : 'Transcribe'}
    </button>
  )}
  
  {recordingState.transcript && (
    <>
      <div className="transcript-section">
        <h4>Transcript ({recordingState.transcript.language})</h4>
        <div className="transcript-text">{recordingState.transcript.transcript}</div>
      </div>
      <button onClick={onSaveGem} disabled={recordingState.savingGem}>
        {recordingState.savingGem ? 'Saving...' : recordingState.gemSaved ? '✓ Saved!' : recordingState.hasGem ? 'Update Gem' : 'Save as Gem'}
      </button>
    </>
  )}
  
  {recordingState.transcriptError && (
    <div className="error">{recordingState.transcriptError}</div>
  )}
</div>
```

### GemDetailPanel Component

**File:** `src/components/GemDetailPanel.tsx`

**Props:**
```typescript
interface GemDetailPanelProps {
  gemId: string;
  onDelete: (id: string) => Promise<void>;
  onTranscribe?: (id: string) => Promise<void>;
  onEnrich?: (id: string) => Promise<void>;
  aiAvailable: boolean;
}
```

**Responsibilities:**
- Fetch and display full gem details
- Show title, tags, summary, transcript
- Render action buttons (Transcribe, Enrich, Delete)
- Handle audio playback for audio transcript gems
- Manage loading and error states

**Structure:**
```tsx
<div className="gem-detail-panel">
  <div className="detail-header">
    <span className="source-badge">{gem.source_type}</span>
  </div>
  
  <h2>{gem.title}</h2>
  
  <div className="gem-metadata">
    <span>{gem.domain}</span>
    {gem.author && <span>by {gem.author}</span>}
    <span>{formatDate(gem.captured_at)}</span>
  </div>
  
  {gem.ai_enrichment?.tags && gem.ai_enrichment.tags.length > 0 && (
    <div className="gem-tags">
      {gem.ai_enrichment.tags.map((tag, index) => (
        <span key={index} className="tag">{tag}</span>
      ))}
    </div>
  )}
  
  {gem.ai_enrichment?.summary && (
    <div className="gem-summary">
      <h4>Summary</h4>
      <p>{gem.ai_enrichment.summary}</p>
    </div>
  )}
  
  <div className="gem-transcript">
    <h4>Transcript {gem.transcript_language && `(${gem.transcript_language})`}</h4>
    <div className="transcript-text">{gem.transcript || gem.content}</div>
  </div>
  
  <div className="gem-actions">
    {aiAvailable && <button onClick={() => onEnrich(gemId)}>Enrich</button>}
    {isAudioGem && aiAvailable && <button onClick={() => onTranscribe(gemId)}>Transcribe</button>}
    <button onClick={() => onDelete(gemId)}>Delete</button>
  </div>
</div>
```

### Modified GemsPanel Component

**Changes to:** `src/components/GemsPanel.tsx`

**New Props:**
```typescript
interface GemsPanelProps {
  onClose?: () => void; // Make optional since no longer overlay
  onGemSelect?: (gemId: string | null) => void; // New: expose selected gem
}
```

**Changes:**
- Add `onGemSelect` callback prop
- Call `onGemSelect(gem.id)` when user clicks a gem card
- Remove inline expansion of gem details (details move to right panel)
- Keep search, filter, and list functionality
- Simplify gem cards to show preview only (no expanded state)

---

## Data Models

### Navigation State

```typescript
type ActiveNav = 'record' | 'recordings' | 'gems' | 'youtube' | 'browser' | 'settings';

interface AppState {
  // New state
  activeNav: ActiveNav;
  leftNavCollapsed: boolean;
  selectedGemId: string | null;
  
  // Existing state (preserved from src/state/types.ts and App.tsx)
  recordingState: 'idle' | 'recording' | 'processing';
  selectedRecording: string | null;
  recordings: RecordingMetadata[];
  transcript: TranscriptionSegment[];
  transcriptionStatus: 'idle' | 'active' | 'error' | 'disabled';
  transcriptionError: string | null;
  youtubeNotification: boolean; // Event-driven notification badge (preserved)
  // ... other existing state
}
```

### Recording Detail State

```typescript
interface RecordingTranscriptionState {
  transcribing: boolean;
  transcript?: TranscriptResult;
  transcriptError?: string;
  hasGem: boolean;
  savingGem: boolean;
  gemSaved: boolean;
  gemError?: string;
}

// Map of filename → transcription state (already exists in App.tsx)
type RecordingStates = Record<string, RecordingTranscriptionState>;
```

### Gem Selection State

```typescript
// Simple string | null for selected gem ID
type SelectedGemId = string | null;

// Full gem data fetched by GemDetailPanel component
interface Gem {
  id: string;
  title: string;
  source_url: string;
  source_type: string;
  domain: string;
  author: string | null;
  description: string | null;
  content: string;
  transcript: string | null;
  transcript_language: string | null;
  captured_at: string;
  tags: string[] | null;
  summary: string | null;
  ai_enrichment: AIEnrichment | null;
  source_meta: Record<string, any> | null;
}
```

---

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Active Nav Highlighting

*For any* nav item in the left navigation, when `activeNav` state matches that nav item's value, the nav item should have an active visual indicator (CSS class or style).

**Validates: Requirements 1.3**

### Property 2: Nav Click Updates State and Content

*For any* nav item in the left navigation, clicking that nav item should update the `activeNav` state to the corresponding value and cause the center panel to render the appropriate content.

**Validates: Requirements 1.4, 2.2**

### Property 3: Right Panel Context Routing

*For any* combination of `activeNav` state and selection state (selectedRecording, selectedGemId), the right panel should display content appropriate to that context or a placeholder message when no content is available.

**Validates: Requirements 3.2, 3.3**

### Property 4: Recording Selection Shows Detail

*For any* recording in the recordings list, clicking that recording should update `selectedRecording` state and cause the right panel to display the recording detail with audio player and transcription controls.

**Validates: Requirements 4.1**

### Property 5: Transcription Indicator

*For any* recording that is currently being transcribed, the center panel recording list should display a loading indicator (spinner or status icon) for that recording.

**Validates: Requirements 4.4**

### Property 6: Gem Selection Shows Detail

*For any* gem in the gems list, clicking that gem should update `selectedGemId` state and cause the right panel to display the gem detail with full content and action buttons.

**Validates: Requirements 5.1**

---

## Error Handling

### Layout Rendering Errors

**Scenario:** Component fails to render due to missing props or state
- **Handling:** Use React error boundaries to catch rendering errors
- **Fallback:** Display error message in affected panel, keep other panels functional
- **Recovery:** Provide "Retry" button to re-render component

### State Synchronization Errors

**Scenario:** `activeNav` and selection state become inconsistent (e.g., selectedRecording set but activeNav is 'gems')
- **Handling:** Right panel routing logic should handle all state combinations gracefully
- **Fallback:** Show placeholder message when state is inconsistent
- **Prevention:** Clear selection state when changing activeNav (e.g., clear selectedRecording when switching away from 'recordings')

### Audio Player Errors

**Scenario:** WAV conversion fails or audio URL becomes invalid
- **Handling:** Catch errors in audio player component
- **Display:** Show error message in recording detail panel
- **Recovery:** Provide "Retry" button to re-convert and reload audio

### Gem Fetch Errors

**Scenario:** Failed to fetch full gem details for right panel
- **Handling:** Catch error in GemDetailPanel component
- **Display:** Show error message with gem ID
- **Recovery:** Provide "Retry" button to re-fetch gem data

### CSS Layout Errors

**Scenario:** Browser doesn't support flexbox or window is too narrow
- **Handling:** Use CSS feature detection and min-width constraints
- **Fallback:** Display warning message if window width < 900px
- **Graceful Degradation:** Panels should still be functional even if layout is suboptimal

---

## Testing Strategy

### Unit Testing

Unit tests will focus on specific examples, edge cases, and component behavior:

**LeftNav Component:**
- Renders all nav items in correct order (Record, Recordings, Gems, YouTube, Browser, Settings)
- Settings nav item is positioned at bottom, separated from main items
- Collapse toggle button is rendered at bottom
- YouTube notification badge appears when `youtubeNotification` prop is true
- Clicking nav item calls `onNavChange` with correct value
- Clicking collapse toggle calls `onToggleCollapse`
- Active nav item has 'active' CSS class
- Collapsed state applies 'collapsed' CSS class

**RightPanel Component:**
- Shows placeholder when activeNav is 'record' and not recording
- Shows TranscriptDisplay when activeNav is 'record' and recording is in progress
- Shows placeholder when activeNav is 'recordings' and no recording selected
- Shows RecordingDetailPanel when activeNav is 'recordings' and recording is selected
- Shows placeholder when activeNav is 'gems' and no gem selected
- Shows GemDetailPanel when activeNav is 'gems' and gem is selected
- Returns null or minimal content for 'youtube', 'browser', 'settings' activeNav

**RecordingDetailPanel Component:**
- Renders recording metadata (filename, date, duration, size)
- Renders audio player with provided audioUrl
- Shows transcribe button when aiAvailable is true
- Hides transcribe button when aiAvailable is false
- Shows transcript section after transcription completes
- Shows "Save as Gem" button after transcription
- Shows "Update Gem" button when recording already has gem
- Displays gem indicator when recordingState.hasGem is true
- Shows error message when transcriptError is present

**GemDetailPanel Component:**
- Fetches full gem data on mount
- Displays gem title, metadata, tags, summary, transcript
- Shows action buttons (Enrich, Transcribe, Delete)
- Hides Transcribe button when gem already has transcript
- Hides Enrich button when aiAvailable is false
- Handles audio playback for audio transcript gems
- Shows error message on fetch failure

**App.tsx Integration:**
- Removes hamburger menu button from header
- Removes hamburger dropdown menu
- Removes dialog-overlay wrappers for Settings, YouTube, Browser, Gems
- Renders three-panel layout structure (left nav, center panel, right panel)
- Center panel shows recording controls when activeNav is 'record'
- Center panel shows recordings list when activeNav is 'recordings'
- Center panel shows GemsPanel when activeNav is 'gems'
- Center panel shows YouTubeSection when activeNav is 'youtube'
- Center panel shows BrowserTool when activeNav is 'browser'
- Center panel shows Settings when activeNav is 'settings'
- Recording list does NOT render transcripts inline
- Audio player does NOT render inline after recordings list
- TranscriptDisplay does NOT render inline in center panel

**CSS Layout:**
- `.app-layout` has `display: flex` and `height: 100vh`
- `.left-nav` has width ~140px when expanded
- `.left-nav.collapsed` has width ~48px
- `.center-panel` has `flex: 1` and `min-width: 300px`
- `.right-panel` has `flex: 1` and `max-width: 50%`
- All panels have `overflow-y: auto` for independent scrolling
- `.dialog-overlay` styles are removed or not applied to converted components

### Property-Based Testing

Property tests will verify universal behaviors across all inputs (minimum 100 iterations per test):

**Property Test 1: Active Nav Highlighting**
- **Generator:** Generate random activeNav value from valid set
- **Test:** Render LeftNav with generated activeNav, verify corresponding nav item has 'active' class
- **Tag:** `Feature: three-panel-layout, Property 1: For any nav item, when activeNav matches, it should have active indicator`

**Property Test 2: Nav Click Updates State**
- **Generator:** Generate random nav item from valid set
- **Test:** Simulate click on generated nav item, verify onNavChange called with correct value
- **Tag:** `Feature: three-panel-layout, Property 2: For any nav item, clicking it should update activeNav state`

**Property Test 3: Right Panel Context Routing**
- **Generator:** Generate random combinations of (activeNav, selectedRecording, selectedGemId, recordingState)
- **Test:** Render RightPanel with generated props, verify correct content or placeholder is rendered
- **Tag:** `Feature: three-panel-layout, Property 3: For any state combination, right panel should show appropriate content or placeholder`

**Property Test 4: Recording Selection Shows Detail**
- **Generator:** Generate random recording data
- **Test:** Simulate click on recording, verify selectedRecording state updates and RecordingDetailPanel renders
- **Tag:** `Feature: three-panel-layout, Property 4: For any recording, clicking it should show detail in right panel`

**Property Test 5: Transcription Indicator**
- **Generator:** Generate random recording with transcribing: true
- **Test:** Render recording list with generated recording, verify loading indicator is present
- **Tag:** `Feature: three-panel-layout, Property 5: For any transcribing recording, list should show indicator`

**Property Test 6: Gem Selection Shows Detail**
- **Generator:** Generate random gem data
- **Test:** Simulate click on gem, verify selectedGemId state updates and GemDetailPanel renders
- **Tag:** `Feature: three-panel-layout, Property 6: For any gem, clicking it should show detail in right panel`

### Integration Testing

Integration tests will verify interactions between components:

- **Nav → Center Panel:** Clicking nav items updates center panel content
- **Center → Right Panel:** Selecting recording/gem in center updates right panel
- **Right Panel Actions:** Actions in right panel (delete gem, save gem) update center panel list
- **State Persistence:** Selection state persists when switching between nav items and returning
- **Audio Player:** Audio player in right panel works correctly with WAV conversion
- **Transcription Flow:** Transcribe button → loading state → transcript display → save gem flow

### Manual Testing Checklist

- [ ] Left nav renders with all items in correct order
- [ ] Clicking each nav item switches center panel content
- [ ] Active nav item is visually highlighted
- [ ] Left nav collapse/expand toggle works smoothly
- [ ] YouTube notification badge appears and clears correctly
- [ ] Recording list does not show inline transcripts
- [ ] Clicking recording shows detail in right panel with audio player
- [ ] Audio player plays recording correctly
- [ ] Transcribe button triggers transcription and shows result in right panel
- [ ] Save as Gem button saves transcript and clears from right panel
- [ ] Clicking gem shows detail in right panel
- [ ] Gem detail shows all content (tags, summary, transcript, actions)
- [ ] Deleting gem from right panel clears selection and refreshes list
- [ ] Live transcript appears in right panel during recording
- [ ] All panels scroll independently
- [ ] Right panel collapses for settings view
- [ ] No hamburger menu or modal overlays present
- [ ] Window resize handles gracefully (min 900px width)
- [ ] All existing Tauri commands still work (recording, transcription, gems)

---

## Implementation Notes

### Migration Strategy

1. **Phase 1: Create new components**
   - Implement LeftNav component
   - Implement RightPanel component
   - Implement RecordingDetailPanel component
   - Implement GemDetailPanel component

2. **Phase 2: Refactor App.tsx**
   - Add new state variables (activeNav, leftNavCollapsed, selectedGemId)
   - Remove old state variables (showSettings, showYouTube, etc.)
   - Replace single-column layout with three-panel layout
   - Remove hamburger menu and overlay rendering
   - Move content to appropriate panels

3. **Phase 3: Update CSS**
   - Add three-panel layout styles
   - Add left nav styles (expanded/collapsed)
   - Add right panel styles
   - Remove or update .container styles
   - Remove .dialog-overlay styles for converted components

4. **Phase 4: Update existing components**
   - Modify GemsPanel to expose onGemSelect callback
   - Make onClose prop optional in Settings, YouTubeSection, BrowserTool, GemsPanel
   - Update TranscriptDisplay usage in right panel

5. **Phase 5: Testing and refinement**
   - Run unit tests
   - Run property-based tests
   - Manual testing of all flows
   - Fix any issues discovered

### Backward Compatibility

- All existing Tauri commands remain unchanged
- All existing event listeners remain unchanged
- All existing state management (useRecording hook) remains unchanged
- Only frontend rendering and layout changes

### Performance Considerations

- Use React.memo for LeftNav to prevent unnecessary re-renders
- Use React.memo for panel components when selection state hasn't changed
- Lazy load gem details in GemDetailPanel (fetch on mount, not on parent render)
- Debounce left nav collapse/expand animations
- Use CSS transitions for smooth panel width changes

### Accessibility

- Ensure left nav items are keyboard navigable (tab order)
- Add ARIA labels to nav items and buttons
- Ensure active nav item is announced to screen readers
- Maintain focus management when switching panels
- Ensure all interactive elements have visible focus indicators

### Future Enhancements (Out of Scope)

- Drag-to-resize panels
- Keyboard shortcuts for navigation (Cmd+1, Cmd+2, etc.)
- Persist activeNav and collapsed state to localStorage
- Responsive/mobile layout
- Tabs within right panel for multiple detail views
- Animation between panel transitions
- Dark mode support
