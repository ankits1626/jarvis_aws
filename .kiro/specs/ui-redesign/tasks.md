# UI Redesign — Implementation Tasks

## Overview

This task list implements the professional dark theme redesign for the Jarvis desktop application. The implementation follows a phased approach: font setup, design tokens, global styles, layout shell, component styling, dead CSS cleanup, and verification.

**Spec Type:** Feature (New)
**Workflow:** Requirements-First
**Implementation Approach:** CSS-only changes to `jarvis-app/src/App.css`

---

# PHASE 1: Font Setup

---

## Task 1: Font Setup and Loading

Download and configure self-hosted fonts for offline reliability.

### Sub-tasks

- [x] 1.1 Download Inter font files (woff2 format) in weights 400, 500, 600
- [x] 1.2 Download JetBrains Mono font files (woff2 format) in weights 400, 500
- [x] 1.3 Create `jarvis-app/public/fonts/` directory if it doesn't exist
- [x] 1.4 Place all font files in `jarvis-app/public/fonts/`
- [x] 1.5 Add @font-face declarations to top of App.css with font-display: swap
- [x] 1.6 Test font loading in Tauri app (verify fonts load offline)

**Validates:** Requirement 2 (Font Loading)

---

# PHASE 2: Design Token System

---

## Task 2: Design Token System

Create the :root block with all design tokens as CSS custom properties.

### Sub-tasks

- [x] 2.1 Create :root block at top of App.css (after @font-face rules)
- [x] 2.2 Define background layer tokens (--bg-base, --bg-surface, --bg-elevated, --bg-hover, --bg-active)
- [x] 2.3 Define border tokens (--border-subtle, --border-default, --border-strong)
- [x] 2.4 Define text hierarchy tokens (--text-primary, --text-secondary, --text-tertiary, --text-inverse)
- [x] 2.5 Define accent color tokens (--accent-primary, --accent-hover, --accent-subtle, --accent-border)
- [x] 2.6 Define semantic color tokens (--success, --warning, --error, --info) with subtle/border variants
- [x] 2.7 Define overlay and source badge color tokens
- [x] 2.8 Define typography tokens (--font-sans, --font-mono, --text-xs through --text-2xl, font weights, line heights)
- [x] 2.9 Define spacing tokens (--space-1 through --space-8 on 4px grid)
- [x] 2.10 Define layout tokens (--nav-width-collapsed, --nav-width-expanded)
- [x] 2.11 Define border radius tokens (--radius-sm, --radius-md, --radius-lg)
- [x] 2.12 Define transition tokens (--duration-fast, --duration-normal, --duration-slow, --ease-out)
- [x] 2.13 Verify all token values match design specification exactly

**Validates:** Requirement 1 (CSS Design Token System)

---

# PHASE 3: Global Styles and Foundation

---

## Task 3: Global Reset and Base Styles

Establish dark theme foundation with global CSS reset.

### Sub-tasks

- [x] 3.1 Add box-sizing: border-box reset for all elements
- [x] 3.2 Style html and body with dark background and primary text color
- [x] 3.3 Configure body typography (font-family, font-size: 14px, line-height, font-smoothing)
- [x] 3.4 Reset margins and paddings on h1-h4, p, ul elements
- [ ] 3.5 Style all headings with primary text color and semibold weight
- [x] 3.6 Configure button elements (font inheritance, cursor pointer)
- [x] 3.7 Style form inputs (input, select, textarea) with dark theme colors
- [x] 3.8 Test that page loads with dark background immediately (no flash)

**Validates:** Requirement 3 (Global Reset and Base Styles)

---

## Task 3.5: Checkpoint - Verify Foundation

Verify that the foundation (fonts, tokens, global styles) is working correctly before proceeding to component styling.

### Sub-tasks

- [x] 3.5.1 Verify dark background loads immediately (no flash of white)
- [x] 3.5.2 Verify Inter font renders correctly for UI text
- [x] 3.5.3 Verify JetBrains Mono font renders correctly for code/monospace text
- [x] 3.5.4 Verify no console errors or warnings in browser console
- [x] 3.5.5 Test in Tauri app (not just browser)
- [x] 3.5.6 Verify all design tokens are defined in :root
- [x] 3.5.7 Ask user if any issues arise before continuing

**Validates:** Foundation phase completion (Phases 1-3)

---

# PHASE 4: Layout Shell

---

## Task 4: Three-Panel Layout Shell

Update layout container and panel styling with dark surfaces.

### Sub-tasks

- [x] 4.1 Style .app-layout container with base background
- [x] 4.2 Style .left-nav with surface background and subtle right border
- [x] 4.3 Configure .left-nav width tokens for expanded/collapsed states
- [x] 4.4 Add smooth collapse/expand transition to .left-nav
- [x] 4.5 Style .center-panel with base background and no border
- [x] 4.6 Style .right-panel with surface background and subtle left border
- [x] 4.7 Style .right-panel-placeholder with tertiary text color and centering
- [x] 4.8 Test layout at different window sizes and collapsed/expanded states

**Validates:** Requirement 4 (Three-Panel Layout Shell)

---

# PHASE 5: Component Styling

---

## Task 5: Left Navigation Components

Style navigation items with accent colors for active states.

### Sub-tasks

- [x] 5.1 Style .nav-item default state (secondary text, no background)
- [x] 5.2 Add .nav-item:hover state (hover background, primary text)
- [x] 5.3 Style .nav-item.active state (accent subtle background, accent text)
- [x] 5.4 Style .nav-item-icon with correct size (var(--text-xl) = 18px)
- [x] 5.5 Style .nav-item-label with small font and medium weight
- [x] 5.6 Style .nav-item-badge as small red dot indicator
- [x] 5.7 Style .nav-toggle button (tertiary text, hover to secondary)
- [x] 5.8 Add spacing/divider between .nav-items and .nav-bottom
- [x] 5.9 Test navigation interactions (hover, active, collapse)

**Validates:** Requirement 5 (Left Navigation Styling)

---

## Task 6: Center Panel List Views

Style recordings, gems, videos, and tabs with consistent list patterns.

### Sub-tasks

- [x] 6.1 Style list containers (.recordings-list, .gems-list, .videos-list, .tab-list) with transparent background
- [x] 6.2 Style list items (.recording-item, .gem-card, .video-card, .tab-item) with padding and border-radius
- [x] 6.3 Add hover states to list items (hover background)
- [x] 6.4 Style .selected state with active background and accent left border
- [x] 6.5 Style item titles (base font, medium weight, primary text)
- [x] 6.6 Style item metadata (small font, secondary text)
- [x] 6.7 Style .empty-message and .empty-state (tertiary text, centered)
- [x] 6.8 Style .gems-search-input with elevated background and border
- [x] 6.9 Add focus state to .gems-search-input (accent border, no outline)
- [x] 6.10 Test list interactions (hover, selection, search input focus)

**Validates:** Requirement 6 (Center Panel — List Views)

---

## Task 7: Right Panel Detail Views

Style detail panels with clear hierarchy and monospace transcripts.

### Sub-tasks

- [x] 7.1 Style .recording-detail-panel and .gem-detail-panel containers
- [x] 7.2 Style detail panel h3 headings (xl font, semibold, primary text)
- [x] 7.3 Style detail panel h4 sub-headings (small font, uppercase, secondary text)
- [x] 7.4 Style .metadata-item, .metadata-label, .metadata-value for key-value pairs
- [x] 7.5 Style .transcript-text with monospace font and elevated background
- [x] 7.6 Style .gem-detail-tag and .gem-tag as accent-colored pills
- [x] 7.7 Style .gem-summary paragraph text with relaxed line height
- [x] 7.8 Style .source-badge as small pill with elevated background
- [x] 7.9 Style .language-indicator as small uppercase badge
- [x] 7.10 Style .transcript-section, .gem-title-section, .gem-transcript with proper spacing
- [x] 7.11 Test detail panel rendering with various content lengths

**Validates:** Requirement 7 (Right Panel — Detail Views)

---

## Task 8: Button System

Implement three-tier button system (primary, secondary, destructive).

### Sub-tasks

- [x] 8.1 Style primary buttons (.record-button, .transcribe-button, etc.) with accent background
- [x] 8.2 Add hover state to primary buttons (accent-hover background)
- [x] 8.3 Add disabled state to primary buttons (opacity, cursor)
- [x] 8.4 Style .record-button.recording state with error background
- [x] 8.5 Style secondary buttons (.action-button, .close-button, etc.) as ghost/outlined
- [x] 8.6 Add hover state to secondary buttons (hover background, primary text)
- [x] 8.7 Style destructive buttons (.delete-button, etc.) with error color and border
- [x] 8.8 Add hover state to destructive buttons (error subtle background)
- [x] 8.9 Style button containers (.gem-actions, .recording-actions, .transcript-actions, .transcription-controls)
- [x] 8.10 Test all button states and interactions

**Validates:** Requirement 8 (Button System)

---

## Task 9: Settings Panel Components

Style all Settings panel form controls and model management UI.

### Sub-tasks

- [ ] 9.1 Style .settings-panel container with base background
- [ ] 9.2 Style .settings-header with xl font and spacing
- [ ] 9.3 Style .settings-section headings (small, uppercase, secondary)
- [ ] 9.4 Style .setting-row with padding and bottom border
- [ ] 9.5 Style .engine-option cards with surface background and border
- [ ] 9.6 Add selected state to .engine-option (accent border and background)
- [ ] 9.7 Style .engine-unavailable state (opacity, disabled cursor)
- [ ] 9.8 Style .provider-option cards (similar to engine options)
- [ ] 9.9 Style .model-item cards with surface background
- [ ] 9.10 Add selected state to .model-item (accent border and background)
- [ ] 9.11 Style model metadata (.model-name, .model-description, .model-tier, etc.)
- [ ] 9.12 Style .progress-container and .progress-bar for downloads
- [ ] 9.13 Style .multimodal-models-panel and .multimodal-model-card
- [ ] 9.14 Style .info-banner variants (success, warning, error, info)
- [ ] 9.15 Style .model-list-error with error colors
- [ ] 9.16 Style .whisperkit-install-info panel
- [ ] 9.17 Style action buttons (.download-button, .select-button, .retry-button, .check-again-button)
- [ ] 9.18 Style .delete-confirm inline confirmation UI
- [ ] 9.19 Style .dismiss-button for dismissible messages
- [ ] 9.20 Style range inputs and checkboxes
- [ ] 9.21 Test all Settings panel interactions and states

**Validates:** Requirement 9 (Settings Panel Styling)

---

## Task 10: Live Transcript Display

Style real-time transcription display with segment differentiation.

### Sub-tasks

- [ ] 10.1 Style .transcript-display container with surface background
- [ ] 10.2 Style .transcript-header with small font and bottom border
- [ ] 10.3 Style .segment-final with monospace font and primary text
- [ ] 10.4 Style .segment-partial with monospace font, tertiary text, and italic
- [ ] 10.5 Style .transcribing-indicator with secondary text
- [ ] 10.6 Add .pulse-dot animation with success color
- [ ] 10.7 Style .error-indicator with error color
- [ ] 10.8 Style .transcript-content with elevated background and monospace font
- [ ] 10.9 Style .empty-transcript with tertiary text and centering
- [ ] 10.10 Style .audio-player container with elevated background
- [ ] 10.11 Test live transcript display during recording

**Validates:** Requirement 10 (Live Transcript and Recording States)

---

## Task 11: Dialog and Error Styling

Style confirmation dialogs, error toasts, and error states.

### Sub-tasks

- [ ] 11.1 Style .dialog-overlay with dark background and backdrop blur
- [ ] 11.2 Style .dialog with elevated background and border
- [ ] 11.3 Style .dialog-header with large font and semibold weight
- [ ] 11.4 Style .dialog-content with secondary text
- [ ] 11.5 Style .error-toast with elevated background and error left border
- [ ] 11.6 Add slideIn animation for .error-toast
- [ ] 11.7 Style .error-message with error color
- [ ] 11.8 Style .error-state with error subtle background and border
- [ ] 11.9 Style .inline-error with error color and warning icon
- [ ] 11.10 Test dialog and error displays

**Validates:** Requirement 11 (Dialog and Error Styling)

---

## Task 12: YouTube and Browser Tool Styling

Style YouTube videos, browser tabs, and gist displays.

### Sub-tasks

- [ ] 12.1 Style .video-card following list item pattern
- [ ] 12.2 Style .video-url and .video-author
- [ ] 12.3 Style .gist-display with elevated background
- [ ] 12.4 Style .gist-header, .gist-field, .gist-label
- [ ] 12.5 Style .gist-description-text with relaxed line height
- [ ] 12.6 Style .gist-actions and .gist-action-bar
- [ ] 12.7 Style .tab-item following list item pattern
- [ ] 12.8 Style .tab-item-content, .tab-item-header, .tab-domain, .tab-title
- [ ] 12.9 Style .source-badge variants (youtube, article, code, docs, etc.) with distinct colors
- [ ] 12.10 Style .claude-badge with accent colors
- [ ] 12.11 Style .observer-status with success dot indicator
- [ ] 12.12 Style .browser-toolbar with surface background and border
- [ ] 12.13 Style .ai-enrichment-notice and .accessibility-notice
- [ ] 12.14 Test YouTube and Browser tool rendering

**Validates:** Requirement 12 (YouTube and Browser Tool Styling)

---

## Task 13: Loading and Skeleton States

Implement animated loading states and skeletons.

### Sub-tasks

- [ ] 13.1 Style .skeleton-line with elevated background
- [ ] 13.2 Add shimmer animation to .skeleton-line
- [ ] 13.3 Style .skeleton-item with border radius and spacing
- [ ] 13.4 Style .spinner with border colors and rotate animation
- [ ] 13.5 Style .loading-state text with tertiary color and centering
- [ ] 13.6 Style .loading utility class
- [ ] 13.7 Test loading states and animations

**Validates:** Requirement 13 (Loading and Skeleton States)

---

## Task 14: Dead CSS Cleanup

Remove unused CSS, resolve duplicates, and organize stylesheet.

### Sub-tasks

- [ ] 14.1 Identify all CSS classes not referenced in any TSX component
- [ ] 14.2 Remove dead CSS rules (unused classes)
- [ ] 14.3 Identify duplicate class definitions
- [ ] 14.4 Resolve duplicates by keeping one authoritative rule per class
- [ ] 14.5 Verify all 42 previously missing classes now have definitions
- [ ] 14.6 Organize App.css into sections with comment headers
- [ ] 14.7 Verify section order: fonts, tokens, reset, layout, nav, lists, details, buttons, forms, dialogs, loading, utilities
- [ ] 14.8 Verify final App.css is under 1,500 lines
- [ ] 14.9 Verify no component rendering is broken

**Validates:** Requirement 14 (Dead CSS Cleanup)

---

## Task 15: Utility Classes

Add general-purpose utility classes for common patterns.

### Sub-tasks

- [ ] 15.1 Style .scrollable utility (overflow-y: auto)
- [ ] 15.2 Style .expanded utility (state modifier)
- [ ] 15.3 Style .error utility class
- [ ] 15.4 Verify utility classes work across different components

**Validates:** Design utility classes section

---

## Task 15.5: Checkpoint - Verify Component Styling

Verify that all component styling is correct and no visual regressions have occurred before proceeding to testing phase.

### Sub-tasks

- [ ] 15.5.1 Verify no visual regressions across all panels (left nav, center, right)
- [ ] 15.5.2 Verify all components render correctly (nav items, list items, detail panels, buttons)
- [ ] 15.5.3 Verify hover states work on all interactive elements
- [ ] 15.5.4 Verify active/selected states work correctly
- [ ] 15.5.5 Quick visual scan of all sections (recordings, gems, settings, YouTube, browser)
- [ ] 15.5.6 Verify no broken layouts or missing styles
- [ ] 15.5.7 Ask user if any issues arise before continuing

**Validates:** Component styling phase completion (Phases 4-5)

---

# PHASE 6: Testing Infrastructure (Optional)

---

## Task 16:* Property-Based Testing Setup

Set up testing infrastructure for CSS property verification.

### Sub-tasks

- [ ]* 16.1 Install testing dependencies (postcss, postcss-selector-parser, glob, jest/vitest)
- [ ]* 16.2 Create test directory structure (tests/unit/ and tests/properties/)
- [ ]* 16.3 Set up CSS parser utilities
- [ ]* 16.4 Set up component file scanner utilities
- [ ]* 16.5 Configure test runner

**Validates:** Design testing strategy section
**Note:** This task is optional. The implementation can rely on manual verification (Task 22) and checkpoint tasks (3.5, 15.5) instead.

---

## Task 17:* Unit Tests

Write unit tests for specific CSS rules and token values.

### Sub-tasks

- [ ]* 17.1 Write token value verification tests
- [ ]* 17.2 Write CSS rule verification tests for key selectors
- [ ]* 17.3 Write font loading tests (@font-face declarations)
- [ ]* 17.4 Write missing class definition tests (verify all 42 classes exist)
- [ ]* 17.5 Run all unit tests and verify they pass

**Validates:** Design testing strategy section
**Note:** This task is optional. The implementation can rely on manual verification (Task 22) and checkpoint tasks (3.5, 15.5) instead.

---

## Task 18:* Property Test 1 - Token-only Styling

Verify no hardcoded color, font-size, font-family, or border-radius values outside :root.

### Sub-tasks

- [ ]* 18.1 Implement Property 1 test using CSS parser
- [ ]* 18.2 Configure test to check only restricted properties (color, background, font-size, font-family, border-radius)
- [ ]* 18.3 Run test and verify no violations
- [ ]* 18.4 Fix any violations found

**Validates:** Property 1 (Token-only styling)
**Note:** This task is optional. The implementation can rely on manual verification (Task 22) and checkpoint tasks (3.5, 15.5) instead.

---

## Task 19:* Property Test 2 - No Dead CSS

Verify all CSS classes are used in at least one TSX component.

### Sub-tasks

- [ ]* 19.1 Implement Property 2 test using CSS parser and file scanner
- [ ]* 19.2 Configure test to extract classes from static strings, template literals, ternaries, and string literals
- [ ]* 19.3 Run test and verify no unused classes
- [ ]* 19.4 Remove any unused classes found

**Validates:** Property 2 (No dead CSS)
**Note:** This task is optional. The implementation can rely on manual verification (Task 22) and checkpoint tasks (3.5, 15.5) instead.

---

## Task 20:* Property Test 3 - No Duplicate Definitions

Verify each CSS class has exactly one base definition.

### Sub-tasks

- [ ]* 20.1 Implement Property 3 test using CSS parser
- [ ]* 20.2 Configure test to distinguish base rules from pseudo-classes and state modifiers
- [ ]* 20.3 Run test and verify no duplicate base definitions
- [ ]* 20.4 Resolve any duplicates found

**Validates:** Property 3 (No duplicate definitions)
**Note:** This task is optional. The implementation can rely on manual verification (Task 22) and checkpoint tasks (3.5, 15.5) instead.

---

## Task 21:* Property Test 4 - Complete Class Coverage

Verify all className attributes in TSX have corresponding CSS rules.

### Sub-tasks

- [ ]* 21.1 Implement Property 4 test using file scanner and CSS parser
- [ ]* 21.2 Configure test to extract classes from static strings, template literals, ternaries, and string literals
- [ ]* 21.3 Run test and verify all classes have definitions
- [ ]* 21.4 Add missing CSS definitions if any found

**Validates:** Property 4 (Complete class coverage)
**Note:** This task is optional. The implementation can rely on manual verification (Task 22) and checkpoint tasks (3.5, 15.5) instead.

---

## Task 21.5:* Checkpoint - Verify All Property Tests Pass

Verify that all property-based tests pass before proceeding to visual verification and final testing.

### Sub-tasks

- [ ]* 21.5.1 Run Property Test 1 (Token-only styling) and verify it passes
- [ ]* 21.5.2 Run Property Test 2 (No dead CSS) and verify it passes
- [ ]* 21.5.3 Run Property Test 3 (No duplicate definitions) and verify it passes
- [ ]* 21.5.4 Run Property Test 4 (Complete class coverage) and verify it passes
- [ ]* 21.5.5 Fix any test failures before continuing
- [ ]* 21.5.6 Verify all unit tests also pass
- [ ]* 21.5.7 Ask user if any issues arise before continuing

**Validates:** Testing phase completion (Phase 6-7)
**Note:** This task is optional. The implementation can rely on manual verification (Task 22) and checkpoint tasks (3.5, 15.5) instead.

---

# PHASE 7: Final Verification

---

## Task 22: Visual Verification

Manually verify visual appearance across all panels and states.

### Sub-tasks

- [ ] 22.1 Test left navigation (default, hover, active, collapsed states)
- [ ] 22.2 Test recordings list (empty, populated, hover, selected states)
- [ ] 22.3 Test gems list (empty, populated, search, hover, selected states)
- [ ] 22.4 Test recording detail panel (metadata, transcript display)
- [ ] 22.5 Test gem detail panel (title, tags, summary, transcript)
- [ ] 22.6 Test live transcript display (recording, partial/final segments, error states)
- [ ] 22.7 Test Settings panel (all sections, engine selection, model management)
- [ ] 22.8 Test YouTube section (video cards, gist display)
- [ ] 22.9 Test Browser tool (tab list, source badges, observer status)
- [ ] 22.10 Test all button states (primary, secondary, destructive, hover, disabled)
- [ ] 22.11 Test dialogs and error toasts
- [ ] 22.12 Test loading states and skeletons
- [ ] 22.13 Verify color contrast meets WCAG AA standards
- [ ] 22.14 Verify typography is readable at 14px base size
- [ ] 22.15 Verify transitions are smooth (100-200ms)

**Validates:** All requirements visually

---

## Task 23: Performance Verification

Verify CSS file size and performance characteristics.

### Sub-tasks

- [ ] 23.1 Verify App.css is under 1,500 lines
- [ ] 23.2 Verify no deeply nested selectors (max 3 levels)
- [ ] 23.3 Verify font files load correctly offline
- [ ] 23.4 Verify page renders without flash of unstyled content
- [ ] 23.5 Verify transitions use GPU-accelerated properties (transform, opacity)
- [ ] 23.6 Test app performance in Tauri (no lag or jank)

**Validates:** Design performance considerations

---

## Task 24: Accessibility Verification

Verify accessibility compliance for the dark theme.

### Sub-tasks

- [ ] 24.1 Verify text color contrast ratios (primary, secondary, tertiary on backgrounds)
- [ ] 24.2 Verify accent color contrast on backgrounds
- [ ] 24.3 Verify all interactive elements have visible focus states
- [ ] 24.4 Test keyboard navigation through all panels
- [ ] 24.5 Verify font sizes scale with user preferences (rem units)
- [ ] 24.6 Test with screen reader (basic navigation)

**Validates:** Design accessibility considerations

---

## Task 25: Cross-Browser Compatibility

Verify rendering in Tauri's WebKit/Chromium webview.

### Sub-tasks

- [ ] 25.1 Test in Tauri app on macOS
- [ ] 25.2 Verify all CSS features work in WebKit/Chromium
- [ ] 25.3 Verify backdrop-filter works correctly
- [ ] 25.4 Verify CSS custom properties work correctly
- [ ] 25.5 Check browser console for CSS warnings or errors

**Validates:** Technical constraints (browser compatibility)

---

## Task 26: Documentation and Cleanup

Document changes and finalize implementation.

### Sub-tasks

- [ ] 26.1 Add inline comments for complex CSS sections
- [ ] 26.2 Verify all section headers are present and correct
- [ ] 26.3 Document any deviations from design spec (if any)
- [ ] 26.4 Update project documentation with new theme details
- [ ] 26.5 Create before/after screenshots for reference

**Validates:** Overall implementation quality

---

## Implementation Notes

### Phased Approach

The tasks are organized to follow the implementation strategy from the design document:

1. **Phase 1 (Tasks 1)**: Font setup
2. **Phase 2 (Task 2)**: Design tokens
3. **Phase 3 (Task 3)**: Global styles
4. **Phase 4 (Task 4)**: Layout shell
5. **Phase 5 (Tasks 5-13, 15)**: Component styling
6. **Phase 6 (Task 14)**: Dead CSS cleanup
7. **Phase 7 (Tasks 16-26)**: Verification and testing

### Testing Strategy

- **Unit tests** (Task 17): Verify specific CSS rules and token values
- **Property tests** (Tasks 18-21): Verify universal correctness across entire stylesheet
- **Visual tests** (Task 22): Manual verification of appearance
- **Performance tests** (Task 23): File size and rendering performance
- **Accessibility tests** (Task 24): Color contrast and keyboard navigation

**Note on Optional Testing Tasks:** Tasks 16-21 and 21.5 are marked as optional because they require installing npm dependencies and creating test infrastructure, which goes beyond the "CSS-only changes" constraint stated in the spec. The core implementation (Tasks 1-15, 22-26) can be completed and verified using:
- Checkpoint tasks (3.5, 15.5) for incremental validation
- Manual verification (Task 22) for visual appearance
- Performance verification (Task 23) for file size and rendering
- Accessibility verification (Task 24) for WCAG compliance

Automated testing is valuable for those who want continuous verification, but manual verification via checkpoints is sufficient for this CSS-only redesign.

### Key Constraints

- CSS-only changes (no TSX modifications unless absolutely necessary)
- Single file approach (all styles in App.css)
- Offline-first fonts (self-hosted woff2 files)
- Target file size: under 1,500 lines (down from ~2,800)
- Base font size: 14px (0.875rem) for desktop density

### Rollback Strategy

If issues arise:
- Git revert to previous working state
- CSS changes are isolated to App.css
- No component logic changes means low risk
- Visual bugs are immediately apparent

---

## Success Criteria

The implementation is complete when:

1. All 26 tasks are marked complete
2. All unit tests pass
3. All 4 property tests pass
4. Visual verification confirms professional appearance
5. App.css is under 1,500 lines
6. No component rendering is broken
7. Performance is smooth with no lag
8. Accessibility standards are met
9. Fonts load correctly offline

---

**Status**: Not Started
**Estimated Effort**: 3-5 days
**Risk Level**: Low (CSS-only, easily reversible)
