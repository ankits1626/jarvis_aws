# Implementation Plan: Three-Panel Layout Redesign

## Overview

Transform the Jarvis desktop app from a single-column layout with modal overlays into a modern three-panel desktop interface (Left Nav | Center Content | Right Context Panel). This implementation creates 4 new components, refactors App.tsx to use state-driven navigation instead of boolean overlay flags, and restructures CSS for flexbox three-panel layout. All existing Tauri commands and functionality are preserved.

## Implementation Phases

This plan is organized into 4 phases for incremental development:

1. **Phase 1: New Components** - Create all new components (LeftNav, RecordingDetailPanel, GemDetailPanel, RightPanel)
2. **Phase 2: App.tsx Refactoring** - Refactor App.tsx to use three-panel layout and state-driven navigation
3. **Phase 3: Component Updates** - Update existing components (GemsPanel, Settings, YouTubeSection, BrowserTool)
4. **Phase 4: Styling & Polish** - Add CSS for three-panel layout and finalize styling

## Tasks

### Phase 1: New Components

- [x] 1. Create LeftNav component with collapsible navigation
  - [x] 1.1 Implement LeftNav.tsx with nav items and collapse toggle
    - Create `src/components/LeftNav.tsx` with TypeScript interface for props (activeNav, onNavChange, youtubeNotification, collapsed, onToggleCollapse)
    - Render nav items: Record, Recordings, Gems, YouTube (with notification badge), Browser
    - Render Settings nav item at bottom, separated from main items
    - Render collapse/expand toggle button at bottom
    - Apply 'active' CSS class to nav item matching activeNav prop
    - Call onNavChange callback when nav item is clicked
    - Call onToggleCollapse callback when toggle button is clicked
    - Apply 'collapsed' or 'expanded' CSS class based on collapsed prop
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8_
  
  - [ ]* 1.2 Write unit tests for LeftNav component
    - Test all nav items render in correct order
    - Test Settings positioned at bottom
    - Test collapse toggle renders
    - Test YouTube notification badge appears when prop is true
    - Test onNavChange called with correct value on nav item click
    - Test onToggleCollapse called on toggle click
    - Test active CSS class applied to matching nav item
    - Test collapsed CSS class applied when collapsed prop is true
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6_

- [x] 2. Create RecordingDetailPanel component for right panel
  - [x] 2.1 Implement RecordingDetailPanel.tsx with audio player and transcription controls
    - Create `src/components/RecordingDetailPanel.tsx` with TypeScript interface for props (recording, audioUrl, onClose, recordingState, onTranscribe, onSaveGem, aiAvailable)
    - Render recording metadata: filename, date, duration, size
    - Render audio player with controls using audioUrl prop
    - Render close button that calls onClose callback
    - Render transcribe button when aiAvailable is true, disabled when transcribing
    - Render transcript section after transcription completes with language indicator
    - Render "Save as Gem" / "Update Gem" button after transcription (text based on hasGem state)
    - Display gem status indicator when recordingState.hasGem is true
    - Display error message when recordingState.transcriptError is present
    - _Requirements: 4.1, 4.2, 4.6_
  
  - [ ]* 2.2 Write unit tests for RecordingDetailPanel component
    - Test renders recording metadata correctly
    - Test audio player renders with provided audioUrl
    - Test transcribe button shows when aiAvailable is true
    - Test transcribe button hides when aiAvailable is false
    - Test transcript section shows after transcription completes
    - Test "Save as Gem" button shows after transcription
    - Test "Update Gem" button shows when hasGem is true
    - Test gem indicator displays when hasGem is true
    - Test error message displays when transcriptError is present
    - _Requirements: 4.2_

- [x] 3. Create GemDetailPanel component for right panel
  - [x] 3.1 Implement GemDetailPanel.tsx with gem details and actions
    - Create `src/components/GemDetailPanel.tsx` with TypeScript interface for props (gemId, onDelete, onTranscribe, onEnrich, aiAvailable)
    - Fetch full gem data on component mount using gemId
    - Render gem title, source badge, metadata (domain, author, date)
    - Render tags display if gem has tags
    - Render summary text if gem has summary
    - Render full transcript with language indicator (scrollable)
    - Render action buttons: Transcribe (for audio gems), Enrich, Delete
    - Hide Enrich button when aiAvailable is false
    - Handle loading state while fetching gem data
    - Handle error state if gem fetch fails with retry button
    - Call onDelete, onTranscribe, onEnrich callbacks when buttons clicked
    - _Requirements: 5.1, 5.2_
  
  - [ ]* 3.2 Write unit tests for GemDetailPanel component
    - Test fetches gem data on mount
    - Test displays gem title, metadata, tags, summary, transcript
    - Test action buttons render (Enrich, Transcribe, Delete)
    - Test Enrich button hides when aiAvailable is false
    - Test loading state displays while fetching
    - Test error message displays on fetch failure
    - Test retry button appears on error
    - _Requirements: 5.2_

- [x] 4. Create RightPanel router component
  - [x] 4.1 Implement RightPanel.tsx with context-based routing
    - Create `src/components/RightPanel.tsx` with TypeScript interface for props (activeNav, selectedRecording, selectedGemId, recordingState, transcript, transcriptionStatus, transcriptionError, audioUrl, onClosePlayer, recordingStates, onTranscribeRecording, onSaveGem, aiAvailable)
    - Implement routing logic: if activeNav is 'record' and recording, show TranscriptDisplay; if recording completed with transcript, show TranscriptDisplay with Save as Gem button; else show placeholder
    - Implement routing logic: if activeNav is 'recordings' and selectedRecording exists, show RecordingDetailPanel; else show placeholder
    - Implement routing logic: if activeNav is 'gems' and selectedGemId exists, show GemDetailPanel; else show placeholder
    - Implement routing logic: for 'youtube', 'browser', 'settings' activeNav, return null or minimal content
    - Render placeholder messages when no content is selected ("Start recording to see live transcript", "Select a recording to play or transcribe", "Select a gem to view details")
    - Apply CSS class for independent scrolling (overflow-y: auto)
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 4.1, 5.1, 6.1, 6.2, 6.3, 6.4_
  
  - [ ]* 4.2 Write unit tests for RightPanel routing logic
    - Test shows placeholder when activeNav is 'record' and not recording
    - Test shows TranscriptDisplay when activeNav is 'record' and recording
    - Test shows TranscriptDisplay with Save as Gem button when activeNav is 'record' and recording completed with transcript available
    - Test shows placeholder when activeNav is 'recordings' and no recording selected
    - Test shows RecordingDetailPanel when activeNav is 'recordings' and recording selected
    - Test shows placeholder when activeNav is 'gems' and no gem selected
    - Test shows GemDetailPanel when activeNav is 'gems' and gem selected
    - Test returns null for 'youtube', 'browser', 'settings' activeNav
    - _Requirements: 3.2, 3.3, 6.4_

- [x] 5. Phase 1 Checkpoint - Ensure all new components compile and tests pass
  - Ensure all tests pass, ask the user if questions arise.

### Phase 2: App.tsx Refactoring

- [x] 6. Refactor App.tsx to three-panel layout
  - [x] 6.1 Add new state variables and remove obsolete state
    - Add `activeNav` state with type `'record' | 'recordings' | 'gems' | 'youtube' | 'browser' | 'settings'`, default to `'record'`
    - Add `leftNavCollapsed` state with type `boolean`, default to `false`
    - Add `selectedGemId` state with type `string | null`, default to `null`
    - Remove state variables: `showSettings`, `showYouTube`, `showBrowserTool`, `showGems`, `showHamburgerMenu`
    - Preserve `youtubeNotification` state (moves from hamburger to left nav)
    - Preserve all existing recording and transcription state
    - _Requirements: 8.1, 8.5, 2.5_
  
  - [x] 6.2 Replace single-column layout with three-panel structure
    - Replace `div.app > div.container` structure with `div.app-layout` flexbox container
    - Render LeftNav component as first child with props (activeNav, onNavChange, youtubeNotification, collapsed, onToggleCollapse)
    - Render center panel as second child (div.center-panel) with content routing based on activeNav
    - Render RightPanel component as third child with all necessary props
    - _Requirements: 2.1, 3.1, 7.1_
  
  - [x] 6.3 Implement center panel content routing
    - When activeNav is 'record': render recording controls (status display, record/stop button, elapsed timer)
    - When activeNav is 'recordings': render recordings list WITHOUT inline transcripts
    - When activeNav is 'gems': render GemsPanel component (remove dialog-overlay wrapper)
    - When activeNav is 'youtube': render YouTubeSection component (remove dialog-overlay wrapper)
    - When activeNav is 'browser': render BrowserTool component (remove dialog-overlay wrapper)
    - When activeNav is 'settings': render Settings component (remove dialog-overlay wrapper)
    - _Requirements: 2.2, 2.3, 2.7_
  
  - [x] 6.4 Remove hamburger menu, overlay rendering, and inline content
    - Remove hamburger menu button from header
    - Remove hamburger dropdown menu rendering (lines 402-438)
    - Remove click-outside handler useEffect for hamburger menu (lines 107-120)
    - Remove all dialog-overlay wrappers for Settings, YouTubeSection, BrowserTool, GemsPanel (lines 682-714)
    - Remove inline audio player rendering (App.tsx lines 610-630) — moved to RecordingDetailPanel in right panel
    - Remove inline TranscriptDisplay rendering (App.tsx lines 632-638) — moved to RightPanel
    - Preserve DeleteConfirmDialog and PermissionDialog as overlays (truly modal actions)
    - Preserve ErrorToast component as-is (floats independently)
    - _Requirements: 4.6, 6.2, 6.5, 8.2, 8.3, 8.4, 8.6, 8.7_
  
  - [x] 6.5 Wire GemsPanel to expose selected gem
    - Add `onGemSelect` callback prop to GemsPanel component
    - Implement callback handler in App.tsx that updates selectedGemId state
    - Pass onGemSelect callback to GemsPanel when rendering in center panel
    - _Requirements: 5.3, 5.4_
  
  - [x] 6.6 Wire gem deletion flow from right panel to center panel
    - Implement gem delete handler in App.tsx that clears selectedGemId and triggers GemsPanel refresh after onDelete completes
    - Pass delete handler to GemDetailPanel via RightPanel component
    - Ensure GemsPanel refreshes its list after deletion completes
    - _Requirements: 5.6_
  
  - [ ]* 6.7 Write integration tests for App.tsx three-panel layout
    - Test three-panel layout structure renders (left nav, center panel, right panel)
    - Test hamburger menu button is removed from header
    - Test hamburger dropdown menu is removed
    - Test dialog-overlay wrappers removed for Settings, YouTube, Browser, Gems
    - Test center panel shows recording controls when activeNav is 'record'
    - Test center panel shows recordings list when activeNav is 'recordings'
    - Test center panel shows GemsPanel when activeNav is 'gems'
    - Test center panel shows YouTubeSection when activeNav is 'youtube'
    - Test center panel shows BrowserTool when activeNav is 'browser'
    - Test center panel shows Settings when activeNav is 'settings'
    - Test recording list does NOT render transcripts inline
    - Test audio player does NOT render inline after recordings list
    - Test TranscriptDisplay does NOT render inline in center panel
    - _Requirements: 2.1, 2.2, 2.3, 2.7, 8.2, 8.3, 8.4, 8.8_

- [x] 7. Phase 2 Checkpoint - Ensure App.tsx refactoring is complete
  - Verify three-panel layout renders correctly
  - Verify all navigation works
  - Ask the user if questions arise

### Phase 3: Component Updates

- [x] 8. Update GemsPanel component to expose selected gem
  - [x] 8.1 Modify GemsPanel.tsx to accept onGemSelect callback
    - Make `onClose` prop optional in TypeScript interface (no longer overlay)
    - Add `onGemSelect?: (gemId: string | null) => void` prop to interface
    - Call `onGemSelect(gem.id)` when user clicks a gem card
    - Remove inline expansion of gem details (details move to right panel)
    - Simplify gem cards to show preview only (no expanded state)
    - _Requirements: 5.3, 8.1_
  
  - [ ]* 8.2 Write unit tests for GemsPanel onGemSelect callback
    - Test onGemSelect called with gem ID when gem card clicked
    - Test gem cards do not expand inline
    - Test onClose prop is optional (component works without it)
    - _Requirements: 5.3_

- [x] 9. Make onClose prop optional in converted components
  - [x] 9.1 Update Settings.tsx, YouTubeSection.tsx, BrowserTool.tsx interfaces
    - Make `onClose` prop optional in TypeScript interface for Settings.tsx
    - Make `onClose` prop optional in TypeScript interface for YouTubeSection.tsx
    - Make `onClose` prop optional in TypeScript interface for BrowserTool.tsx
    - Update close button rendering to only show when onClose prop is provided
    - _Requirements: 2.3_

- [x] 10. Phase 3 Checkpoint - Ensure component updates are complete
  - Verify GemsPanel selection works
  - Verify optional onClose props work correctly
  - Ask the user if questions arise

### Phase 4: Styling & Polish

- [x] 11. Update CSS for three-panel layout
  - [x] 11.1 Create three-panel flexbox layout styles
    - Add `.app-layout` styles: `display: flex`, `height: 100vh`
    - Add `.left-nav` styles: width ~140px when expanded, ~48px when collapsed, CSS transition on width, `overflow-y: auto`
    - Add `.left-nav.collapsed` styles: width ~48px
    - Add `.center-panel` styles: `flex: 1`, `min-width: 300px`, `overflow-y: auto`
    - Add `.right-panel` styles: `flex: 1`, `max-width: 50%`, `min-width: 0`, `overflow-y: auto`, CSS transition on width
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_
  
  - [x] 11.2 Add left nav component styles
    - Add styles for nav items: icon, label, active state, hover state
    - Add styles for nav-bottom section (Settings and collapse toggle)
    - Add styles for collapse toggle button
    - Add styles for YouTube notification badge
    - Add styles for expanded/collapsed label visibility (hide labels when collapsed)
    - _Requirements: 1.2, 1.5, 1.6, 8.5_
  
  - [x] 11.3 Add right panel component styles
    - Add styles for RecordingDetailPanel: header, metadata, audio player section, transcript section, action buttons
    - Add styles for GemDetailPanel: header, metadata, tags, summary, transcript, action buttons
    - Add styles for placeholder messages
    - Add styles for scrollable transcript sections
    - _Requirements: 3.4, 4.2, 5.2_
  
  - [x] 11.4 Remove or update obsolete styles
    - Remove or replace `.container` styles that enforce single-column max-width layout
    - Remove `.dialog-overlay` styles for Settings, YouTube, Browser, Gems (or ensure they're not applied)
    - Update any inline content styles that are no longer needed (inline transcripts, inline audio player)
    - _Requirements: 7.5, 7.6_

- [x] 12. Final checkpoint - Ensure all tests pass and manual testing
  - Run full test suite
  - Perform manual testing of all navigation flows
  - Verify responsive behavior and styling
  - Ask the user if questions arise

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation at the end of each phase
- All existing Tauri commands and backend functionality are preserved
- Only frontend rendering and layout changes
- No new dependencies required - plain CSS with flexbox
- No routing library needed - state-driven navigation

## Phase Summary

- **Phase 1** (Tasks 1-5): Build all new components in isolation
- **Phase 2** (Tasks 6-7): Integrate components into App.tsx with new layout
- **Phase 3** (Tasks 8-10): Update existing components to work with new architecture
- **Phase 4** (Tasks 11-12): Apply styling and perform final testing
