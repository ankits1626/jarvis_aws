# UI Redesign — Professional Dark Theme

## Introduction

The Jarvis desktop application has a functional three-panel layout but uses prototype-quality styling: light gray backgrounds, system default fonts at 16px, hardcoded colors with zero CSS variables, inconsistent spacing, and ~500 lines of dead CSS. The result looks like a developer prototype rather than a professional desktop tool.

This spec defines the requirements for a complete visual redesign using a dark theme inspired by Linear, VS Code, and Notion. The redesign introduces a CSS design token system, professional typography (Inter + JetBrains Mono), and consistent component styling across all panels and navigation states.

**Reference:** Research findings documented in `discussion/27-feb-next-step/ui-redesign-findings.md`.

## Glossary

- **Design Token**: A CSS custom property (variable) that stores a design decision (color, spacing, font size) for reuse across the stylesheet.
- **Surface Layer**: A background color tier used to create visual depth. The system uses three: base (darkest), surface (panels), elevated (modals/overlays).
- **Cool-tinted Gray**: A gray color with a subtle blue/purple hue, as opposed to pure neutral gray.
- **Type Scale**: A set of predefined font sizes that maintain visual hierarchy and consistency.
- **4px Grid**: A spacing system where all spacing values are multiples of 4px.

---

## Requirement 1: CSS Design Token System

**User Story:** As a developer, I want all visual properties (colors, fonts, spacing, radii, transitions) defined as CSS custom properties in a single `:root` block, so that the entire theme can be modified by changing token values rather than hunting through 2,800 lines of hardcoded values.

### Acceptance Criteria

1. THE System SHALL define a `:root` block in `App.css` containing all design tokens before any other rules
2. THE System SHALL define background tokens: `--bg-base` (#0a0a0c), `--bg-surface` (#111114), `--bg-elevated` (#18181b), `--bg-hover` (#1e1e22), `--bg-active` (#27272a)
3. THE System SHALL define border tokens: `--border-subtle` (#1e1e22), `--border-default` (#27272a), `--border-strong` (#3f3f46)
4. THE System SHALL define text tokens: `--text-primary` (#fafafa), `--text-secondary` (#a1a1aa), `--text-tertiary` (#71717a), `--text-inverse` (#09090b)
5. THE System SHALL define accent tokens: `--accent-primary` (#6366f1), `--accent-hover` (#818cf8), `--accent-subtle` (rgba(99,102,241,0.12)), `--accent-border` (rgba(99,102,241,0.3))
6. THE System SHALL define semantic tokens: `--success` (#22c55e), `--warning` (#f59e0b), `--error` (#ef4444), `--info` (#3b82f6)
7. THE System SHALL define font family tokens: `--font-sans` ('Inter' with system fallbacks), `--font-mono` ('JetBrains Mono' with monospace fallbacks)
8. THE System SHALL define font size tokens: `--text-xs` (0.75rem), `--text-sm` (0.8125rem), `--text-base` (0.875rem), `--text-lg` (1rem), `--text-xl` (1.125rem), `--text-2xl` (1.25rem)
9. THE System SHALL define font weight tokens: `--font-normal` (400), `--font-medium` (500), `--font-semibold` (600)
10. THE System SHALL define spacing tokens on a 4px grid: `--space-1` (4px) through `--space-8` (32px)
11. THE System SHALL define layout tokens: `--nav-width-collapsed` (48px), `--nav-width-expanded` (180px)
12. THE System SHALL define radius tokens: `--radius-sm` (4px), `--radius-md` (6px), `--radius-lg` (8px)
13. THE System SHALL define transition tokens: `--duration-fast` (100ms), `--duration-normal` (150ms), `--duration-slow` (200ms), `--ease-out` (cubic-bezier(0.16,1,0.3,1))
14. THE System SHALL NOT contain any hardcoded color, font-size, font-family, or border-radius values outside the `:root` block — all rules SHALL reference tokens

---

## Requirement 2: Font Loading

**User Story:** As a user, I want the app to use Inter for UI text and JetBrains Mono for code/transcripts, loading reliably even when offline, so that the typography looks professional regardless of network connectivity.

### Acceptance Criteria

1. THE System SHALL self-host Inter font files (woff2 format) in weights 400, 500, and 600
2. THE System SHALL self-host JetBrains Mono font files (woff2 format) in weights 400 and 500
3. THE font files SHALL be placed in `jarvis-app/public/fonts/`
4. THE System SHALL declare `@font-face` rules in `App.css` for each font weight with `font-display: swap`
5. THE System SHALL NOT depend on Google Fonts CDN or any external network request for font loading
6. THE `--font-sans` token SHALL resolve to Inter with system font fallbacks: `-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif`
7. THE `--font-mono` token SHALL resolve to JetBrains Mono with fallbacks: `'SF Mono', 'Fira Code', monospace`

---

## Requirement 3: Global Reset and Base Styles

**User Story:** As a user, I want the app to immediately present a dark background with professional typography when it loads, so that there is no flash of unstyled or light-themed content.

### Acceptance Criteria

1. THE `html` and `body` elements SHALL have `background-color: var(--bg-base)` and `color: var(--text-primary)`
2. THE `body` SHALL use `font-family: var(--font-sans)` with `font-size: var(--text-base)` (14px)
3. THE `body` SHALL use `line-height: 1.5` and `-webkit-font-smoothing: antialiased`
4. ALL `button` elements SHALL inherit `font-family: var(--font-sans)` and use `cursor: pointer`
5. ALL `input`, `select`, and `textarea` elements SHALL inherit font-family and use dark-themed colors: `background: var(--bg-elevated)`, `color: var(--text-primary)`, `border: 1px solid var(--border-default)`
6. THE System SHALL set `*, *::before, *::after` to `box-sizing: border-box`
7. THE System SHALL reset default margins and paddings on `h1`–`h4`, `p`, `ul` elements
8. ALL headings SHALL use `color: var(--text-primary)` and `font-weight: var(--font-semibold)`

---

## Requirement 4: Three-Panel Layout Shell

**User Story:** As a user, I want the three-panel layout (nav, center, right) to use dark surfaces with subtle dividers, creating clear visual separation without heavy borders.

### Acceptance Criteria

1. THE `.app-layout` container SHALL use `background: var(--bg-base)`
2. THE `.left-nav` SHALL use `background: var(--bg-surface)` with `border-right: 1px solid var(--border-subtle)`
3. THE `.left-nav` width SHALL use `var(--nav-width-expanded)` when expanded and `var(--nav-width-collapsed)` when collapsed
4. THE `.left-nav` collapse/expand transition SHALL use `var(--duration-slow)` with `var(--ease-out)`
5. THE `.center-panel` SHALL use `background: var(--bg-base)` with no border
6. THE `.right-panel` SHALL use `background: var(--bg-surface)` with `border-left: 1px solid var(--border-subtle)`
7. THE `.right-panel-placeholder` text SHALL use `color: var(--text-tertiary)` and be centered vertically and horizontally

---

## Requirement 5: Left Navigation Styling

**User Story:** As a user, I want the left navigation to clearly indicate which section is active using accent colors, with smooth hover states, so I can navigate confidently.

### Acceptance Criteria

1. THE `.nav-item` SHALL use `color: var(--text-secondary)` in its default state with no background
2. WHEN a `.nav-item` is hovered, THE System SHALL apply `background: var(--bg-hover)` and `color: var(--text-primary)` with `transition: var(--duration-fast)`
3. WHEN a `.nav-item` has the `.active` class, THE System SHALL apply `background: var(--accent-subtle)` and `color: var(--accent-primary)`
4. THE `.nav-item-icon` SHALL be `18px` in size
5. THE `.nav-item-label` SHALL use `font-size: var(--text-sm)` and `font-weight: var(--font-medium)`
6. THE `.nav-item-badge` (YouTube notification) SHALL use `background: var(--error)` as a small dot indicator
7. THE `.nav-toggle` (collapse button) SHALL use `color: var(--text-tertiary)` and appear at the bottom of the nav
8. WHEN `.nav-toggle` is hovered, THE System SHALL apply `color: var(--text-secondary)`
9. THE `.nav-bottom` SHALL be separated from `.nav-items` with a subtle divider or spacing

---

## Requirement 6: Center Panel — List Views

**User Story:** As a user, I want recordings, gems, YouTube videos, and browser tabs to appear as clean list items with clear hover and selection states, so I can scan content quickly and know what's selected.

### Acceptance Criteria

1. THE `.recordings-list`, `.gems-list`, `.videos-list`, `.tab-list` SHALL have no visible outer border and use `background: transparent`
2. List items (`.recording-item`, `.gem-card`, `.video-card`, `.tab-item`) SHALL have `padding: var(--space-2) var(--space-3)`, `border-radius: var(--radius-md)`, and no visible border by default
3. WHEN a list item is hovered, THE System SHALL apply `background: var(--bg-hover)`
4. WHEN a list item has the `.selected` class, THE System SHALL apply `background: var(--bg-active)` with a `2px` left border in `var(--accent-primary)`
5. Item titles SHALL use `font-size: var(--text-base)`, `font-weight: var(--font-medium)`, `color: var(--text-primary)`
6. Item metadata (dates, durations, sizes) SHALL use `font-size: var(--text-sm)`, `color: var(--text-secondary)`
7. THE `.empty-message` and `.empty-state` SHALL use `color: var(--text-tertiary)` centered in the panel
8. THE `.gems-search-input` SHALL use `background: var(--bg-elevated)`, `border: 1px solid var(--border-default)`, `color: var(--text-primary)`, `border-radius: var(--radius-md)` with placeholder color `var(--text-tertiary)`
9. WHEN `.gems-search-input` is focused, THE System SHALL apply `border-color: var(--accent-primary)` with `outline: none`

---

## Requirement 7: Right Panel — Detail Views

**User Story:** As a user, I want the right panel detail views (recording details, gem details, live transcript) to display content with clear section hierarchy and readable transcript text in a monospace font.

### Acceptance Criteria

1. THE `.recording-detail-panel` and `.gem-detail-panel` SHALL use `padding: var(--space-4)` with `background: var(--bg-surface)`
2. Detail panel headings (`h3`) SHALL use `font-size: var(--text-xl)`, `font-weight: var(--font-semibold)`, `color: var(--text-primary)`
3. Section sub-headings (`h4`) SHALL use `font-size: var(--text-sm)`, `font-weight: var(--font-semibold)`, `color: var(--text-secondary)`, `text-transform: uppercase`, `letter-spacing: 0.05em`
4. THE `.metadata-item` SHALL display as a row with `.metadata-label` in `color: var(--text-secondary)` and `.metadata-value` in `color: var(--text-primary)`, both at `font-size: var(--text-sm)`
5. THE `.transcript-text` SHALL use `font-family: var(--font-mono)`, `font-size: var(--text-sm)`, `background: var(--bg-elevated)`, `padding: var(--space-4)`, `border-radius: var(--radius-md)`, `color: var(--text-primary)`, `line-height: var(--leading-relaxed)`, and `overflow-y: auto`
6. THE `.gem-detail-tag` and `.gem-tag` SHALL use `font-size: var(--text-xs)`, `background: var(--accent-subtle)`, `color: var(--accent-primary)`, `padding: var(--space-1) var(--space-2)`, `border-radius: var(--radius-sm)`
7. THE `.gem-summary` paragraph text SHALL use `font-size: var(--text-base)`, `color: var(--text-primary)`, `line-height: var(--leading-relaxed)`
8. THE `.source-badge` SHALL use `font-size: var(--text-xs)`, `background: var(--bg-elevated)`, `color: var(--text-secondary)`, `padding: var(--space-1) var(--space-2)`, `border-radius: var(--radius-sm)`
9. THE `.language-indicator` SHALL use `font-size: var(--text-xs)`, `color: var(--text-tertiary)`, `font-weight: var(--font-medium)`

---

## Requirement 8: Button System

**User Story:** As a user, I want buttons to have consistent, distinguishable styles — primary actions in accent color, secondary actions as ghost/outlined, and destructive actions in red — so I can quickly identify button importance and intent.

### Acceptance Criteria

1. ALL primary action buttons (`.record-button`, `.transcribe-button`, `.save-gem-button`, `.gem-enrich-button`, `.prepare-gist-button`) SHALL use `background: var(--accent-primary)`, `color: var(--text-inverse)`, `border: none`, `border-radius: var(--radius-md)`, `font-size: var(--text-sm)`, `font-weight: var(--font-medium)`, `padding: var(--space-2) var(--space-4)`
2. WHEN a primary button is hovered, THE System SHALL apply `background: var(--accent-hover)` with `transition: background var(--duration-fast) var(--ease-out)`
3. WHEN a primary button is disabled, THE System SHALL apply `opacity: 0.5` and `cursor: not-allowed`
4. THE `.record-button` in recording state SHALL use `background: var(--error)` instead of accent, with hover `background` slightly lighter
5. ALL secondary/ghost buttons (`.action-button`, `.close-button`, `.gem-open-button`, `.gem-view-button`, `.copy-button`, `.refresh-button`) SHALL use `background: transparent`, `color: var(--text-secondary)`, `border: 1px solid var(--border-default)`, same radius and padding as primary
6. WHEN a secondary button is hovered, THE System SHALL apply `background: var(--bg-hover)` and `color: var(--text-primary)`
7. ALL destructive buttons (`.delete-button`, `.gem-delete-button`, `.cancel-button`) SHALL use `background: transparent`, `color: var(--error)`, `border: 1px solid rgba(239, 68, 68, 0.3)`
8. WHEN a destructive button is hovered, THE System SHALL apply `background: rgba(239, 68, 68, 0.1)`
9. THE `.gem-actions`, `.recording-actions`, `.transcript-actions` containers SHALL use `display: flex`, `gap: var(--space-2)`, aligning buttons horizontally

---

## Requirement 9: Settings Panel Styling

**User Story:** As a user, I want the Settings panel to use the same dark theme with clearly labeled sections, readable form controls, and consistent spacing, so it feels like part of the same application.

### Acceptance Criteria

1. THE `.settings-panel` SHALL use `background: var(--bg-base)`, `padding: var(--space-4)`, `height: 100%`, `overflow-y: auto`
2. THE `.settings-header` SHALL use `font-size: var(--text-xl)`, `color: var(--text-primary)`, `margin-bottom: var(--space-6)`
3. THE `.settings-section` headings SHALL use `font-size: var(--text-sm)`, `font-weight: var(--font-semibold)`, `color: var(--text-secondary)`, `text-transform: uppercase`, `letter-spacing: 0.05em`, `margin-bottom: var(--space-3)`
4. THE `.setting-row` SHALL use `padding: var(--space-3) 0` with a bottom border of `1px solid var(--border-subtle)`
5. THE `.engine-option` SHALL use `background: var(--bg-surface)`, `border: 1px solid var(--border-default)`, `border-radius: var(--radius-md)`, `padding: var(--space-3)`
6. WHEN `.engine-option` is selected/active, THE System SHALL apply `border-color: var(--accent-primary)` and `background: var(--accent-subtle)`
7. THE `.model-item` SHALL use `background: var(--bg-surface)`, `border-radius: var(--radius-md)`, `padding: var(--space-3)`, `margin-bottom: var(--space-2)`
8. THE `.progress-bar` SHALL use `background: var(--accent-primary)` on a track of `background: var(--bg-elevated)`
9. THE `.model-name` SHALL use `color: var(--text-primary)`, `font-weight: var(--font-medium)` and `.model-description` SHALL use `color: var(--text-secondary)`, `font-size: var(--text-sm)`

---

## Requirement 10: Live Transcript and Recording States

**User Story:** As a user, I want the live transcript display during recording to be clearly readable with visual distinction between partial and finalized segments, using a monospace font on a dark elevated surface.

### Acceptance Criteria

1. THE `.transcript-display` SHALL use `background: var(--bg-surface)`, `padding: var(--space-4)`, full height of right panel
2. THE `.segment-final` SHALL use `font-family: var(--font-mono)`, `font-size: var(--text-sm)`, `color: var(--text-primary)`
3. THE `.segment-partial` SHALL use same font as final but with `color: var(--text-tertiary)` and `font-style: italic`
4. THE `.transcribing-indicator` SHALL use `color: var(--text-secondary)` with the `.pulse-dot` using `background: var(--success)` and a CSS pulse animation
5. THE `.error-indicator` SHALL use `color: var(--error)`
6. THE `.transcript-header` SHALL use `font-size: var(--text-sm)`, `font-weight: var(--font-semibold)`, `color: var(--text-secondary)`, with a bottom border of `var(--border-subtle)`
7. THE `.audio-player` container SHALL use `background: var(--bg-elevated)`, `border-radius: var(--radius-md)`, `padding: var(--space-3)`

---

## Requirement 11: Dialog and Error Styling

**User Story:** As a user, I want confirmation dialogs and error messages to be clearly visible against the dark theme, with destructive actions highlighted in red, so I don't accidentally delete content.

### Acceptance Criteria

1. THE `.dialog-overlay` SHALL use `background: rgba(0, 0, 0, 0.6)` with `backdrop-filter: blur(4px)`
2. THE `.dialog` SHALL use `background: var(--bg-elevated)`, `border: 1px solid var(--border-default)`, `border-radius: var(--radius-lg)`, `padding: var(--space-6)`, `max-width: 400px`
3. THE `.dialog-header` text SHALL use `font-size: var(--text-lg)`, `color: var(--text-primary)`, `font-weight: var(--font-semibold)`
4. THE `.dialog-content` text SHALL use `color: var(--text-secondary)`, `font-size: var(--text-base)`
5. THE `.confirm-delete-button` SHALL follow destructive button styling (Requirement 8.7)
6. THE `.cancel-delete-button` SHALL follow secondary button styling (Requirement 8.5)
7. THE `.error-toast` SHALL use `background: var(--bg-elevated)`, `border-left: 3px solid var(--error)`, `color: var(--text-primary)`, `border-radius: var(--radius-md)`
8. THE `.error-message` and `.error-state` SHALL use `color: var(--error)`, `font-size: var(--text-sm)`
9. THE `.inline-error` SHALL use `color: var(--error)`, `font-size: var(--text-sm)`, `margin-top: var(--space-2)`

---

## Requirement 12: YouTube and Browser Tool Styling

**User Story:** As a user, I want the YouTube section and Browser tool to follow the same dark theme and list patterns as recordings and gems, so the entire app feels unified.

### Acceptance Criteria

1. THE `.video-card` SHALL follow the same list item pattern as `.gem-card` (Requirement 6.2–6.6)
2. THE `.gist-display` SHALL use `background: var(--bg-elevated)`, `border-radius: var(--radius-md)`, `padding: var(--space-4)`
3. THE `.gist-field` labels SHALL use `font-size: var(--text-xs)`, `color: var(--text-tertiary)`, `text-transform: uppercase`
4. THE `.gist-description-text` SHALL use `font-size: var(--text-base)`, `color: var(--text-primary)`, `line-height: var(--leading-relaxed)`
5. THE `.tab-item` SHALL follow the same list item pattern as `.gem-card` (Requirement 6.2–6.6)
6. THE `.source-badge` variants (youtube, article, code, docs, etc.) SHALL each use a distinct `color` value but all share the same pill shape: `font-size: var(--text-xs)`, `padding: var(--space-1) var(--space-2)`, `border-radius: var(--radius-sm)`, `background: var(--bg-elevated)`
7. THE `.observer-status` indicator SHALL use `color: var(--text-secondary)` with a dot using `var(--success)` when active
8. THE `.browser-toolbar` SHALL use `background: var(--bg-surface)`, `border-bottom: 1px solid var(--border-subtle)`, `padding: var(--space-2) var(--space-3)`

---

## Requirement 13: Loading and Skeleton States

**User Story:** As a user, I want loading states to use subtle animated skeletons that match the dark theme, so the app feels responsive even while data loads.

### Acceptance Criteria

1. THE `.skeleton-line` SHALL use `background: var(--bg-elevated)` with a shimmer animation using a lighter gradient of `var(--bg-hover)`
2. THE shimmer animation SHALL move left-to-right over `1.5s` with `infinite` repeat and `ease-in-out` timing
3. THE `.skeleton-item` SHALL have `border-radius: var(--radius-sm)` and `margin-bottom: var(--space-2)`
4. THE `.spinner` SHALL use `border-color: var(--border-default)` with `border-top-color: var(--accent-primary)` and a rotate animation
5. THE `.loading-state` text SHALL use `color: var(--text-tertiary)`, centered in its container

---

## Requirement 14: Dead CSS Cleanup

**User Story:** As a developer, I want unused, duplicate, and conflicting CSS rules removed, so the stylesheet is maintainable and there are no rendering conflicts.

### Acceptance Criteria

1. THE System SHALL remove all CSS rules that are not referenced by any component (dead code)
2. THE System SHALL resolve all duplicate class definitions by keeping only one authoritative rule per class name
3. THE System SHALL define CSS rules for all 14 classes used in components but currently missing definitions: `metadata-item`, `metadata-label`, `metadata-value`, `transcription-controls`, `transcript-section`, `language-indicator`, `transcript-actions`, `gem-title-section`, `gem-transcript`, `action-button`, `scrollable`, `expanded`, `source-badge`, `retry-button`
4. THE System SHALL NOT break any existing component rendering — all class names used in TSX files SHALL have corresponding CSS definitions
5. THE final `App.css` SHOULD be organized in the following section order: (1) Font faces, (2) Design tokens, (3) Global reset, (4) Layout shell, (5) Left nav, (6) Center panel lists, (7) Right panel details, (8) Buttons, (9) Forms/inputs, (10) Dialogs, (11) Loading states, (12) Utilities
6. EACH section SHALL be preceded by a comment header (e.g., `/* === Layout Shell === */`)

---

## Technical Constraints

1. **Offline-first fonts**: Font files must be self-hosted (woff2 in `public/fonts/`). No CDN dependencies. Tauri apps run offline.
2. **Single CSS file**: All styles remain in `App.css`. No CSS modules, CSS-in-JS, or preprocessors introduced.
3. **No component changes for styling**: CSS-only changes. No TSX modifications unless a class name must be added or renamed. Prefer styling existing class names.
4. **Browser compatibility**: Target WebKit/Chromium (Tauri's webview). No need for Firefox/Safari cross-browser hacks.
5. **Performance**: CSS file should be under 1,500 lines after cleanup (down from ~2,800). Avoid deeply nested selectors.
6. **Base font size**: 14px (`0.875rem`) for desktop density. Do not use 16px browser default.

## Out of Scope

1. Light theme or theme toggle — dark only for this iteration
2. Custom audio player component — use native `<audio>` with styled container
3. CSS preprocessors (SASS, Less, PostCSS) — plain CSS only
4. CSS modules or scoped styles — single file approach maintained
5. Animation library integration — CSS transitions and keyframes only
6. Responsive/mobile breakpoints — desktop-only Tauri app
7. Icon system replacement — existing emoji icons retained
8. Component refactoring — no TSX structural changes
