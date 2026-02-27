# Requirements: Three-Panel Layout Redesign

## Introduction

Jarvis currently uses a single vertical column layout in `App.tsx` where all content stacks top-to-bottom: header with hamburger menu, status/record button, recordings list (with inline transcripts), audio player, live transcript display, and then YouTube/Browser/Gems/Settings as full-screen modal overlays triggered from a hamburger dropdown. This creates several UX problems: excessive scrolling, modal overlays that block the main view, inline transcripts that clutter the recording list, no persistent navigation, and poor use of horizontal screen space on a desktop app.

This feature replaces the single-column layout with a three-panel layout (Left Nav | Center Content | Right Context Panel), converting all modal overlays into navigable views and moving detail content (transcripts, audio player, gem details) into a dedicated right panel.

### Key Architecture Context

- **Current layout**: Single-column in `App.tsx` with `div.app > div.container` as the root. All content vertically stacked.
- **Modal overlays**: `Settings`, `YouTubeSection`, `BrowserTool`, `GemsPanel` are rendered conditionally as `dialog-overlay` divs wrapping each component. Each has an `onClose` prop.
- **Hamburger menu**: A dropdown toggled by `showHamburgerMenu` state in App.tsx (lines 402-438) that opens YouTube, Browser, and Gems overlays.
- **Boolean state flags**: `showSettings`, `showYouTube`, `showBrowserTool`, `showGems`, `showHamburgerMenu` control which overlay is visible.
- **Inline content**: Recording list renders transcripts inline below each recording row (lines 555-597). Audio player renders inline after the recording list (lines 610-630). Live transcript (`TranscriptDisplay`) renders inline after audio player (lines 632-638).
- **Existing components**: `Settings.tsx`, `YouTubeSection.tsx`, `BrowserTool.tsx`, `GemsPanel.tsx` all accept `onClose: () => void` and render their own close button. These will become center-panel content views (remove overlay wrapper, optionally keep or remove `onClose`).
- **Recording selection**: `state.selectedRecording` already tracks which recording is selected. Currently toggles the audio player open.
- **Gem selection**: `GemsPanel.tsx` manages its own selected gem state internally. Will need to expose selected gem for right panel rendering.
- **CSS**: All styles are in `App.css`. No CSS framework -- plain CSS with flexbox/grid.

### Goals

1. Replace the single-column layout with a persistent three-panel layout (left nav, center content, right context panel).
2. Convert all modal overlay panels (Settings, YouTube, Browser, Gems) into navigable views in the center panel.
3. Move detail content (transcripts, audio player, gem details) from inline/overlay to the right context panel.
4. Provide persistent, always-visible navigation via a collapsible left sidebar.
5. Maintain all existing functionality -- no features removed, only reorganized.

---

## Requirements

### Requirement 1: Left Navigation Panel

**User Story:** As a user, I want a persistent left sidebar with navigation icons so that I can switch between app sections without using a hamburger dropdown or modal overlays.

#### Acceptance Criteria

1. THE SYSTEM SHALL render a left navigation panel as the first child of the app layout container.
2. THE LEFT NAV SHALL display the following nav items in order, each with an icon and label:
   - Record (recording controls)
   - Recordings (recordings list)
   - Gems (gems panel)
   - YouTube (YouTube extractor)
   - Browser (Browser tool)
   - Settings (settings panel) -- positioned at the bottom of the nav, separated from the main items
3. THE LEFT NAV SHALL visually highlight the currently active nav item.
4. WHEN the user clicks a nav item THEN THE SYSTEM SHALL update `activeNav` state and render the corresponding content in the center panel.
5. THE LEFT NAV SHALL support two states: expanded (~140px, showing icons + labels) and collapsed (~48px, icons only).
6. THE SYSTEM SHALL render a collapse/expand toggle button at the bottom of the left nav.
7. THE LEFT NAV SHALL default to the expanded state.
8. THE LEFT NAV SHALL be always visible (not hidden behind a hamburger menu).

**File:** `src/components/LeftNav.tsx` -- new component; `src/App.tsx` -- integrate into layout; `src/App.css` -- left nav styles

---

### Requirement 2: Center Panel with Nav-Driven Content Routing

**User Story:** As a user, I want the center panel to show the content relevant to my selected navigation item, so that I can access all features without modal overlays blocking my view.

#### Acceptance Criteria

1. THE SYSTEM SHALL render a center panel as the second child of the app layout container, between the left nav and right panel.
2. THE CENTER PANEL SHALL display different content based on the `activeNav` state value:
   - `'record'`: Recording controls (status display, record/stop button, elapsed timer) -- currently in App.tsx lines 442-482
   - `'recordings'`: Recordings list (with gem indicators, transcribe buttons, delete buttons) -- currently in App.tsx lines 484-608, but WITHOUT inline transcripts (transcripts move to right panel)
   - `'gems'`: GemsPanel component (currently rendered as overlay, becomes inline) -- existing `GemsPanel.tsx`
   - `'youtube'`: YouTubeSection component (currently rendered as overlay, becomes inline) -- existing `YouTubeSection.tsx`
   - `'browser'`: BrowserTool component (currently rendered as overlay, becomes inline) -- existing `BrowserTool.tsx`
   - `'settings'`: Settings component (currently rendered as overlay, becomes inline) -- existing `Settings.tsx`
3. THE SYSTEM SHALL remove the `dialog-overlay` wrapper from Settings, YouTubeSection, BrowserTool, and GemsPanel rendering. These components SHALL render directly in the center panel.
4. THE SYSTEM SHALL remove the hamburger menu button, hamburger dropdown, and all `showSettings`, `showYouTube`, `showBrowserTool`, `showGems`, `showHamburgerMenu` boolean state flags from App.tsx.
5. THE SYSTEM SHALL add a new state: `activeNav` with type `'record' | 'recordings' | 'gems' | 'youtube' | 'browser' | 'settings'`, defaulting to `'record'`.
6. THE CENTER PANEL SHALL be scrollable independently from the left nav and right panel.
7. WHEN `activeNav` is `'recordings'` THEN the recording list SHALL NOT render transcripts inline. Transcripts SHALL only appear in the right panel (see Requirement 4).

**File:** `src/App.tsx` -- refactor content rendering; `src/App.css` -- center panel styles

---

### Requirement 3: Right Context Panel -- Structure and Behavior

**User Story:** As a user, I want a right panel that shows contextual details relevant to my current action, so that I can see detail information without cluttering the main list view.

#### Acceptance Criteria

1. THE SYSTEM SHALL render a right context panel as the third child of the app layout container, after the center panel.
2. THE RIGHT PANEL SHALL display different content based on the active nav and user action:
   - `'record'` nav: Live transcript output during recording (currently `TranscriptDisplay` component at App.tsx lines 632-638). Empty/placeholder when idle.
   - `'recordings'` nav: Selected recording detail -- audio player, transcribe button, transcript result, save gem button. Placeholder when no recording is selected.
   - `'gems'` nav: Selected gem detail -- title, tags, summary, transcript, action buttons (transcribe, enrich, delete). Placeholder when no gem is selected.
   - `'youtube'` nav: Extracted video content after extraction completes.
   - `'browser'` nav: Extracted page content after extraction completes.
   - `'settings'` nav: Model capability info, or hidden/collapsed.
3. WHEN no contextual content is available (e.g., no recording selected, no gem selected) THEN THE SYSTEM SHALL display a placeholder message (e.g., "Select a recording to play or transcribe").
4. THE RIGHT PANEL SHALL be scrollable independently from the center panel.
5. THE RIGHT PANEL SHALL support collapsing (width 0) when not needed (e.g., settings view).

**File:** `src/components/RightPanel.tsx` -- new component; `src/App.tsx` -- integrate into layout; `src/App.css` -- right panel styles

---

### Requirement 4: Recording Detail in Right Panel

**User Story:** As a user, when I select a recording from the list, I want to see its details, audio player, and transcription controls in the right panel, so that the recording list stays clean and uncluttered.

#### Acceptance Criteria

1. WHEN the user clicks a recording in the center panel recording list THEN THE SYSTEM SHALL display the recording detail in the right panel.
2. THE RECORDING DETAIL in the right panel SHALL include:
   - Recording filename and metadata (date, duration, size)
   - Audio player (currently rendered inline in App.tsx lines 610-630)
   - Transcribe button (when AI is available)
   - Transcript display (after transcription completes) with language indicator
   - Save as Gem / Update Gem button (after transcription completes)
   - Gem status indicator
3. THE CENTER PANEL recording list SHALL NOT render transcripts inline. The recording list SHALL only show: filename, gem indicator, metadata, transcribe button (icon only, as status indicator), and delete button.
4. WHEN a recording is being transcribed THEN THE CENTER PANEL SHALL show a spinner/indicator on the recording row, and THE RIGHT PANEL SHALL show the transcript result when complete.
5. WHEN the user clicks "Save as Gem" in the right panel THEN the transcript SHALL clear from the right panel after successful save (matching existing behavior).
6. THE AUDIO PLAYER SHALL move from its current inline position (after recording list) to the right panel, within the selected recording detail.

**File:** `src/components/RecordingDetailPanel.tsx` -- new component (right panel content for recordings); `src/App.tsx` -- remove inline audio player and inline transcript rendering

---

### Requirement 5: Gem Detail in Right Panel

**User Story:** As a user, when I select a gem from the gems list, I want to see its full details in the right panel instead of expanding inline, so that the gem list stays compact and browseable.

#### Acceptance Criteria

1. WHEN the user clicks a gem in the GemsPanel center view THEN THE SYSTEM SHALL display the gem detail in the right panel.
2. THE GEM DETAIL in the right panel SHALL include:
   - Gem title and capture date
   - Tags display
   - Summary text
   - Full transcript with language indicator (scrollable)
   - Action buttons: Transcribe (re-transcribe), Enrich (re-enrich), Delete
3. THE GemsPanel component SHALL expose the selected gem ID to the parent (App.tsx) so that the right panel can render gem detail.
4. THE SYSTEM SHALL add a new state: `selectedGemId: string | null`, defaulting to `null`.
5. WHEN `selectedGemId` is null THEN THE RIGHT PANEL SHALL show a placeholder message ("Select a gem to view details").
6. WHEN a gem is deleted from the right panel THEN THE SYSTEM SHALL clear `selectedGemId` and refresh the gems list in the center panel.

**File:** `src/components/GemDetailPanel.tsx` -- new component (right panel content for gems); `src/components/GemsPanel.tsx` -- modify to expose selected gem; `src/App.tsx` -- wire selected gem state

---

### Requirement 6: Live Transcript in Right Panel

**User Story:** As a user, I want to see the live Whisper transcript in the right panel during recording, so that I can monitor the transcription output without it taking up space in the recording controls area.

#### Acceptance Criteria

1. WHEN `activeNav` is `'record'` and recording is in progress THEN THE SYSTEM SHALL display the `TranscriptDisplay` component in the right panel.
2. THE `TranscriptDisplay` component SHALL be moved from its current inline position (App.tsx lines 632-638) to the right panel when `activeNav` is `'record'`.
3. WHEN recording is idle (not recording) THEN THE RIGHT PANEL SHALL show a placeholder message ("Start recording to see live transcript").
4. WHEN recording completes and transcript is available THEN THE RIGHT PANEL SHALL display the final transcript with a "Save as Gem" button.
5. THE LIVE TRANSCRIPT SHALL NOT appear in the center panel in the new layout.

**File:** `src/App.tsx` -- move TranscriptDisplay rendering to right panel; `src/App.css` -- adjust transcript display styles for right panel context

---

### Requirement 7: CSS Layout Structure

**User Story:** As a user, I want the three-panel layout to use the full width of my desktop window with proper sizing and independent scrolling, so that the app feels like a proper desktop application.

#### Acceptance Criteria

1. THE SYSTEM SHALL use CSS Flexbox for the three-column layout with the following structure:
   ```
   div.app-layout {
     display: flex;
     height: 100vh;
   }
   ```
2. THE LEFT NAV SHALL have:
   - Expanded width: ~140px
   - Collapsed width: ~48px
   - CSS transition on width change for smooth animation
   - `overflow-y: auto` for scrollable nav when many items
3. THE CENTER PANEL SHALL have:
   - `flex: 1` to take remaining horizontal space
   - `min-width: 300px` to prevent excessive squishing
   - `overflow-y: auto` for independent scrolling
4. THE RIGHT PANEL SHALL have:
   - `flex: 1` to share space with center panel
   - `max-width: 50%` to prevent it from dominating
   - `min-width: 0` to allow collapsing to 0 width
   - `overflow-y: auto` for independent scrolling
   - CSS transition on width for smooth show/hide
5. THE SYSTEM SHALL remove or replace the existing `.container` styles that enforce a single-column max-width layout.
6. THE SYSTEM SHALL remove the `.dialog-overlay` styles for Settings, YouTube, Browser, and Gems since they are no longer rendered as overlays.

**File:** `src/App.css` -- major restructure of layout styles

---

### Requirement 8: State Cleanup and Migration

**User Story:** As a developer, I want obsolete state and UI code removed cleanly so that the codebase stays maintainable and doesn't carry dead code from the old layout.

#### Acceptance Criteria

1. THE SYSTEM SHALL remove the following state variables from App.tsx:
   - `showSettings`
   - `showYouTube`
   - `showBrowserTool`
   - `showGems`
   - `showHamburgerMenu`
2. THE SYSTEM SHALL remove the hamburger menu button from the header.
3. THE SYSTEM SHALL remove the hamburger dropdown menu and its click-outside handler (useEffect at lines 107-120).
4. THE SYSTEM SHALL remove all `dialog-overlay` wrappers for Settings, YouTubeSection, BrowserTool, and GemsPanel (lines 682-714).
5. THE SYSTEM SHALL preserve the `youtubeNotification` state variable and update the notification badge to appear on the YouTube nav item in the left nav instead of the hamburger button.
6. THE SYSTEM SHALL keep the `DeleteConfirmDialog` and `PermissionDialog` as overlays (these are truly modal actions that should block interaction).
7. THE SYSTEM SHALL keep the `ErrorToast` component as-is (it floats independently of layout).
8. THE SYSTEM SHALL preserve all existing Tauri command invocations, event listeners, and backend interactions -- only the rendering location changes, not the functionality.

**File:** `src/App.tsx` -- remove dead state and overlay code; `src/components/LeftNav.tsx` -- add YouTube notification badge

---

## Technical Constraints

1. **No new dependencies**: Use plain CSS (flexbox/grid) for layout. No CSS framework or router library needed.
2. **No routing library**: Navigation is state-driven (`activeNav` state variable), not URL-based. This is a Tauri desktop app, not a web app with URLs.
3. **Component `onClose` props**: `Settings.tsx`, `YouTubeSection.tsx`, `BrowserTool.tsx`, `GemsPanel.tsx` all have `onClose` props. These can be removed or repurposed since there are no overlays to close. Alternatively, keep them as no-ops during migration.
4. **`TranscriptDisplay` dual usage**: `TranscriptDisplay` is used for live recording transcripts AND could be reused for showing transcription results. It currently receives `transcript`, `status`, `error`, and `recordingFilename` props.
5. **Existing `RecordingRow.tsx` and `RecordingsList.tsx`**: These extracted components exist but are currently unused (recordings render inline in App.tsx). The three-panel refactor is a good opportunity to start using them, but this is optional.
6. **`GemsPanel` internal state**: `GemsPanel.tsx` currently manages its own selected gem and expanded state internally. To render gem detail in the right panel, it will need to either: (a) expose `selectedGemId` via a callback prop, or (b) use a shared state/context.
7. **Audio player state**: The audio player is currently rendered in App.tsx and depends on `state.selectedRecording` and `audioUrl`. Moving it to `RecordingDetailPanel` means passing these as props or using the recording hook.
8. **Tauri window size**: The app runs in a Tauri window. Default window size is configured in `tauri.conf.json`. The three-panel layout may require updating the default window width to accommodate three panels comfortably (suggest minimum ~900px width).

---

## Out of Scope

- Adding a router library (React Router, etc.) -- state-driven navigation is sufficient
- Responsive/mobile layout -- this is a desktop-only Tauri app
- Drag-to-resize panels -- fixed/flex sizing is sufficient for v1
- Keyboard shortcuts for navigation -- can be added later
- Persisting `activeNav` or collapsed state across app restarts
- Animations beyond basic CSS transitions for panel width changes
- Refactoring `useRecording` hook or backend commands -- only frontend layout changes
- Dark mode or theming -- separate concern
- Tabs or multi-panel-in-panel layouts (e.g., multiple right panel tabs)
