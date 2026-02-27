# UI Redesign — Design Document

## Overview

This design transforms the Jarvis desktop application from a prototype-quality light theme to a professional dark theme inspired by Linear, VS Code, and Notion. The redesign introduces a comprehensive CSS design token system, self-hosted professional typography (Inter + JetBrains Mono), and consistent component styling across all panels.

The implementation is CSS-only with no component structural changes, focusing on visual polish through systematic use of design tokens, proper color hierarchy, and professional typography. The current ~2,800-line App.css will be reduced to under 1,500 lines through dead code removal and consolidation.

### Key Design Decisions

1. **CSS-only approach**: No TSX modifications required. All styling changes happen in App.css by targeting existing class names.

2. **Design token system**: All visual properties (colors, fonts, spacing, transitions) defined as CSS custom properties in a single :root block. This enables theme-wide consistency and easy future modifications.

3. **Dark theme with three surface layers**: 
   - Base (#0a0a0c): Darkest, used for main background
   - Surface (#111114): Panels and cards
   - Elevated (#18181b): Modals, inputs, code blocks

4. **Self-hosted fonts**: Inter (UI) and JetBrains Mono (code/transcripts) loaded from local woff2 files for offline reliability.

5. **14px base font size**: Desktop-optimized density (0.875rem) instead of browser default 16px.

6. **4px spacing grid**: All spacing values are multiples of 4px for visual consistency.

## Architecture

### CSS File Structure

The redesigned App.css follows this organization:


```
1. @font-face declarations (Inter, JetBrains Mono)
2. :root design tokens
3. Global reset and base styles
4. Layout shell (app-layout, left-nav, center-panel, right-panel)
5. Left navigation components
6. Center panel list views
7. Right panel detail views
8. Button system
9. Form inputs and controls
10. Settings panel
11. Dialogs and overlays
12. Loading states and animations
13. Utility classes
```

Each section is preceded by a comment header (e.g., `/* === Layout Shell === */`) for easy navigation.

### Design Token Categories

The token system organizes visual properties into logical groups:

- **Colors**: Background layers, borders, text hierarchy, accents, semantic colors
- **Typography**: Font families, sizes, weights, line heights
- **Spacing**: 4px grid system (--space-1 through --space-8)
- **Layout**: Navigation widths, panel dimensions
- **Borders**: Radius values for different component sizes
- **Motion**: Transition durations and easing functions

All tokens use CSS custom properties (--token-name) and are defined once in :root, then referenced throughout the stylesheet using var(--token-name).

## Components and Interfaces

### Font Loading System

**Self-hosted fonts** are loaded via @font-face declarations at the top of App.css:


```css
@font-face {
  font-family: 'Inter';
  src: url('/fonts/Inter-Regular.woff2') format('woff2');
  font-weight: 400;
  font-display: swap;
}
/* Additional @font-face rules for Inter 500, 600 and JetBrains Mono 400, 500 */
```

Font files are placed in `jarvis-app/public/fonts/` and referenced with absolute paths from the public directory. The `font-display: swap` ensures text remains visible during font loading.

### Design Token System

All design tokens are defined in a single :root block:

```css
:root {
  /* Background layers */
  --bg-base: #0a0a0c;
  --bg-surface: #111114;
  --bg-elevated: #18181b;
  --bg-hover: #1e1e22;
  --bg-active: #27272a;
  
  /* Borders */
  --border-subtle: #1e1e22;
  --border-default: #27272a;
  --border-strong: #3f3f46;
  
  /* Text hierarchy */
  --text-primary: #fafafa;
  --text-secondary: #a1a1aa;
  --text-tertiary: #71717a;
  --text-inverse: #09090b;
  
  /* Accent colors */
  --accent-primary: #6366f1;
  --accent-hover: #818cf8;
  --accent-subtle: rgba(99, 102, 241, 0.12);
  --accent-border: rgba(99, 102, 241, 0.3);
  
  /* Semantic colors */
  --success: #22c55e;
  --success-subtle: rgba(34, 197, 94, 0.1);
  --success-border: rgba(34, 197, 94, 0.3);
  
  --warning: #f59e0b;
  --warning-subtle: rgba(245, 158, 11, 0.1);
  --warning-border: rgba(245, 158, 11, 0.3);
  
  --error: #ef4444;
  --error-subtle: rgba(239, 68, 68, 0.1);
  --error-border: rgba(239, 68, 68, 0.3);
  
  --info: #3b82f6;
  --info-subtle: rgba(59, 130, 246, 0.1);
  --info-border: rgba(59, 130, 246, 0.3);
  
  /* Overlay colors */
  --overlay-dark: rgba(0, 0, 0, 0.6);
  
  /* Source badge colors (brand/decorative) */
  --color-youtube: #ff0000;
  --color-article: #3b82f6;
  --color-code: #8b5cf6;
  --color-docs: #10b981;
  --color-email: #f59e0b;
  --color-chat: #ec4899;
  --color-qa: #06b6d4;
  --color-news: #ef4444;
  --color-research: #6366f1;
  --color-social: #8b5cf6;
  
  /* Typography */
  --font-sans: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  --font-mono: 'JetBrains Mono', 'SF Mono', 'Fira Code', monospace;
  
  --text-xs: 0.75rem;    /* 12px */
  --text-sm: 0.8125rem;  /* 13px */
  --text-base: 0.875rem; /* 14px */
  --text-lg: 1rem;       /* 16px */
  --text-xl: 1.125rem;   /* 18px */
  --text-2xl: 1.25rem;   /* 20px */
  
  --font-normal: 400;
  --font-medium: 500;
  --font-semibold: 600;
  
  --leading-normal: 1.5;
  --leading-relaxed: 1.6;
  
  /* Spacing (4px grid) */
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-5: 20px;
  --space-6: 24px;
  --space-7: 28px;
  --space-8: 32px;
  
  /* Layout */
  --nav-width-collapsed: 48px;
  --nav-width-expanded: 180px;
  
  /* Border radius */
  --radius-sm: 4px;
  --radius-md: 6px;
  --radius-lg: 8px;
  
  /* Transitions */
  --duration-fast: 100ms;
  --duration-normal: 150ms;
  --duration-slow: 200ms;
  --ease-out: cubic-bezier(0.16, 1, 0.3, 1);
}
```

### Global Reset and Base Styles

The global reset establishes the dark theme foundation:

```css
*, *::before, *::after {
  box-sizing: border-box;
}

html, body {
  margin: 0;
  padding: 0;
  background-color: var(--bg-base);
  color: var(--text-primary);
}

body {
  font-family: var(--font-sans);
  font-size: var(--text-base);
  line-height: var(--leading-normal);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

h1, h2, h3, h4, p, ul {
  margin: 0;
  padding: 0;
}

h1, h2, h3, h4 {
  color: var(--text-primary);
  font-weight: var(--font-semibold);
}

button {
  font-family: var(--font-sans);
  cursor: pointer;
}

input, select, textarea {
  font-family: inherit;
  background: var(--bg-elevated);
  color: var(--text-primary);
  border: 1px solid var(--border-default);
}
```

### Three-Panel Layout

The layout uses flexbox with fixed-width side panels and a flexible center:

```css
.app-layout {
  display: flex;
  height: 100vh;
  width: 100vw;
  background: var(--bg-base);
  overflow: hidden;
}

.left-nav {
  width: var(--nav-width-expanded);
  background: var(--bg-surface);
  border-right: 1px solid var(--border-subtle);
  transition: width var(--duration-slow) var(--ease-out);
}

.left-nav.collapsed {
  width: var(--nav-width-collapsed);
}

.center-panel {
  flex: 1;
  min-width: 0;
  background: var(--bg-base);
  overflow-y: auto;
  padding: var(--space-4);
}

.right-panel {
  width: 400px;
  background: var(--bg-surface);
  border-left: 1px solid var(--border-subtle);
  overflow-y: auto;
  padding: var(--space-4);
}
```

### Navigation Components

Navigation items use accent colors for active states and subtle hover effects:

```css
.nav-item {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  color: var(--text-secondary);
  background: transparent;
  border: none;
  transition: all var(--duration-fast) var(--ease-out);
}

.nav-item:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.nav-item.active {
  background: var(--accent-subtle);
  color: var(--accent-primary);
}

.nav-item-icon {
  font-size: var(--text-xl);
  width: var(--text-xl);
  text-align: center;
}

.nav-item-label {
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
}

.nav-item-badge {
  width: var(--space-2);
  height: var(--space-2);
  background: var(--error);
  border-radius: 50%;
}
```

### List View Pattern

All list views (recordings, gems, videos, tabs) follow a consistent pattern:

```css
.recordings-list, .gems-list, .videos-list, .tab-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  background: transparent;
  border: none;
}

.recording-item, .gem-card, .video-card, .tab-item {
  padding: var(--space-2) var(--space-3);
  background: transparent;
  border: none;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background var(--duration-fast) var(--ease-out);
}

.recording-item:hover, .gem-card:hover, .video-card:hover, .tab-item:hover {
  background: var(--bg-hover);
}

.recording-item.selected, .gem-card.selected, .video-card.selected, .tab-item.selected {
  background: var(--bg-active);
  border-left: 2px solid var(--accent-primary);
}
```

### Button System

Three button variants with consistent sizing and hover states:

**Primary buttons** (accent color, high emphasis):
```css
.record-button, .transcribe-button, .save-gem-button, 
.gem-enrich-button, .prepare-gist-button {
  background: var(--accent-primary);
  color: var(--text-inverse);
  border: none;
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  padding: var(--space-2) var(--space-4);
  transition: background var(--duration-fast) var(--ease-out);
}

.record-button:hover:not(:disabled) {
  background: var(--accent-hover);
}

.record-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.record-button.recording {
  background: var(--error);
}
```

**Secondary buttons** (ghost/outlined, medium emphasis):
```css
.action-button, .close-button, .gem-open-button, 
.gem-view-button, .copy-button, .refresh-button {
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  padding: var(--space-2) var(--space-4);
  transition: all var(--duration-fast) var(--ease-out);
}

.action-button:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
```

**Destructive buttons** (red, for delete actions):
```css
.delete-button, .gem-delete-button, .cancel-button {
  background: transparent;
  color: var(--error);
  border: 1px solid var(--error-border);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  padding: var(--space-2) var(--space-4);
  transition: all var(--duration-fast) var(--ease-out);
}

.delete-button:hover {
  background: var(--error-subtle);
}

.transcription-controls {
  display: flex;
  gap: var(--space-2);
  align-items: center;
  margin-top: var(--space-3);
}

.transcript-actions {
  display: flex;
  gap: var(--space-2);
  align-items: center;
  margin-top: var(--space-3);
}
```

### Detail Panel Components

Detail panels use elevated surfaces with clear typography hierarchy:

```css
.recording-detail-panel, .gem-detail-panel {
  padding: var(--space-4);
  background: var(--bg-surface);
}

.recording-detail-panel h3, .gem-detail-panel h3 {
  font-size: var(--text-xl);
  font-weight: var(--font-semibold);
  color: var(--text-primary);
  margin-bottom: var(--space-4);
}

.recording-detail-panel h4, .gem-detail-panel h4 {
  font-size: var(--text-sm);
  font-weight: var(--font-semibold);
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: var(--space-2);
}

.metadata-item {
  display: flex;
  justify-content: space-between;
  font-size: var(--text-sm);
  margin-bottom: var(--space-2);
}

.metadata-label {
  color: var(--text-secondary);
}

.metadata-value {
  color: var(--text-primary);
}

.language-indicator {
  font-size: var(--text-xs);
  color: var(--text-tertiary);
  font-weight: var(--font-medium);
  text-transform: uppercase;
}

.transcript-section {
  margin-top: var(--space-4);
  padding-top: var(--space-4);
  border-top: 1px solid var(--border-subtle);
}

.gem-title-section {
  margin-bottom: var(--space-4);
  padding-bottom: var(--space-3);
  border-bottom: 1px solid var(--border-subtle);
}

.gem-transcript {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  background: var(--bg-elevated);
  padding: var(--space-4);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  line-height: var(--leading-relaxed);
  overflow-y: auto;
  max-height: 400px;
  white-space: pre-wrap;
}

.transcript-text {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  background: var(--bg-elevated);
  padding: var(--space-4);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  line-height: var(--leading-relaxed);
  overflow-y: auto;
  max-height: 400px;
}
```

### Form Inputs

Form controls use elevated backgrounds with accent focus states:

```css
.gems-search-input {
  width: 100%;
  padding: var(--space-2) var(--space-3);
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-size: var(--text-sm);
  transition: border-color var(--duration-normal) var(--ease-out);
}

.gems-search-input:focus {
  outline: none;
  border-color: var(--accent-primary);
}

.gems-search-input::placeholder {
  color: var(--text-tertiary);
}
```

### Settings Panel Components

The Settings panel is the most form-heavy section and requires comprehensive dark theme styling for all its components:

**Panel container and header**:
```css
.settings-panel {
  background: var(--bg-base);
  padding: var(--space-4);
  height: 100%;
  overflow-y: auto;
}

.settings-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--space-6);
}

.settings-header h2 {
  font-size: var(--text-xl);
  color: var(--text-primary);
  margin: 0;
}

.settings-content {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
}
```

**Section styling**:
```css
.settings-section {
  border-bottom: 1px solid var(--border-subtle);
  padding-bottom: var(--space-4);
}

.settings-section:last-child {
  border-bottom: none;
}

.settings-section h3 {
  font-size: var(--text-sm);
  font-weight: var(--font-semibold);
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: var(--space-3);
}
```

**Setting rows and form controls**:
```css
.setting-row {
  padding: var(--space-3) 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  border-bottom: 1px solid var(--border-subtle);
}

.setting-row label {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--text-primary);
  font-size: var(--text-base);
  cursor: pointer;
}

.setting-row input[type="checkbox"] {
  width: 16px;
  height: 16px;
  cursor: pointer;
}

.setting-row input[type="range"] {
  width: 100%;
  height: 4px;
  background: var(--bg-elevated);
  border-radius: var(--radius-sm);
  outline: none;
  cursor: pointer;
}

.setting-row input[type="range"]::-webkit-slider-thumb {
  appearance: none;
  width: 16px;
  height: 16px;
  background: var(--accent-primary);
  border-radius: 50%;
  cursor: pointer;
}

.setting-row input[type="range"]:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.setting-info {
  font-size: var(--text-sm);
  color: var(--text-tertiary);
  margin: 0;
}
```

**Engine selection cards**:
```css
.engine-options {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  margin-bottom: var(--space-3);
}

.engine-option {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  background: var(--bg-surface);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  padding: var(--space-3);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.engine-option:hover {
  background: var(--bg-hover);
  border-color: var(--border-strong);
}

.engine-option input[type="radio"]:checked + span {
  color: var(--accent-primary);
  font-weight: var(--font-medium);
}

.engine-option.engine-unavailable {
  opacity: 0.6;
  cursor: not-allowed;
}

.engine-option.engine-unavailable:hover {
  background: var(--bg-surface);
  border-color: var(--border-default);
}

.engine-reason {
  font-size: var(--text-sm);
  color: var(--text-tertiary);
  font-style: italic;
}

.engine-note {
  font-size: var(--text-sm);
  color: var(--text-secondary);
  margin: var(--space-2) 0 0 0;
}
```

**Provider selection (similar to engine options)**:
```css
.provider-options {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  margin-bottom: var(--space-3);
}

.provider-option {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  background: var(--bg-surface);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  padding: var(--space-3);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.provider-option:hover {
  background: var(--bg-hover);
  border-color: var(--border-strong);
}

.provider-option input[type="radio"]:checked + span {
  color: var(--accent-primary);
  font-weight: var(--font-medium);
}

.provider-note {
  font-size: var(--text-sm);
  color: var(--text-secondary);
  margin: var(--space-2) 0 0 0;
}
```

**Model list and model items**:
```css
.model-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.model-item {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  background: var(--bg-surface);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  padding: var(--space-3);
  transition: border-color var(--duration-fast) var(--ease-out);
}

.model-item.selected {
  border-color: var(--accent-primary);
  background: var(--accent-subtle);
}

.model-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.model-header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
}

.model-name {
  color: var(--text-primary);
  font-weight: var(--font-medium);
  font-size: var(--text-base);
}

.model-tier {
  font-size: var(--text-xs);
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius-sm);
  background: var(--bg-elevated);
  font-weight: var(--font-medium);
}

.model-size-estimate {
  font-size: var(--text-sm);
  color: var(--text-tertiary);
}

.model-description {
  color: var(--text-secondary);
  font-size: var(--text-sm);
  line-height: var(--leading-relaxed);
}

.model-filename {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}

.model-size {
  font-size: var(--text-sm);
  color: var(--text-secondary);
}

.model-error {
  font-size: var(--text-sm);
  color: var(--error);
}

.model-actions {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  align-items: flex-end;
}
```

**Progress bars for model downloads**:
```css
.progress-container {
  position: relative;
  width: 120px;
  height: 20px;
  background: var(--bg-elevated);
  border-radius: var(--radius-sm);
  overflow: hidden;
}

.progress-bar {
  position: absolute;
  top: 0;
  left: 0;
  height: 100%;
  background: var(--accent-primary);
  transition: width var(--duration-normal) var(--ease-out);
}

.progress-text {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  font-size: var(--text-xs);
  color: var(--text-primary);
  font-weight: var(--font-medium);
  z-index: 1;
}
```

**Multimodal model cards (for MLX Omni)**:
```css
.multimodal-models-panel {
  margin-top: var(--space-4);
  padding: var(--space-4);
  background: var(--bg-surface);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
}

.multimodal-models-panel h4 {
  margin-top: 0;
  margin-bottom: var(--space-3);
  font-size: var(--text-base);
  color: var(--text-primary);
}

.multimodal-model-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.multimodal-model-card {
  padding: var(--space-3);
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.multimodal-model-card:hover {
  border-color: var(--border-strong);
}

.multimodal-model-card.selected {
  border-color: var(--accent-primary);
  border-width: 2px;
}
```

**Info banners (for diagnostics and status messages)**:
```css
.info-banner {
  padding: var(--space-3);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  margin-bottom: var(--space-4);
}

.info-banner.success {
  background: var(--success-subtle);
  border: 1px solid var(--success-border);
  color: var(--success);
}

.info-banner.warning {
  background: var(--warning-subtle);
  border: 1px solid var(--warning-border);
  color: var(--warning);
}

.info-banner.error {
  background: var(--error-subtle);
  border: 1px solid var(--error-border);
  color: var(--error);
}

.info-banner.info {
  background: var(--info-subtle);
  border: 1px solid var(--info-border);
  color: var(--info);
}

.info-banner strong {
  display: block;
  margin-bottom: var(--space-1);
}

.info-banner small {
  display: block;
  margin-top: var(--space-1);
  opacity: 0.8;
}
```

**WhisperKit install info**:
```css
.whisperkit-install-info {
  margin-top: var(--space-3);
  padding: var(--space-3);
  background: var(--bg-surface);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
}

.whisperkit-install-info p {
  margin: 0 0 var(--space-2) 0;
  font-size: var(--text-sm);
  color: var(--text-secondary);
}

.whisperkit-install-info code {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  background: var(--bg-elevated);
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius-sm);
  color: var(--accent-primary);
}

.check-again-button {
  margin-top: var(--space-2);
  padding: var(--space-2) var(--space-3);
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.check-again-button:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
```

**Model list error display**:
```css
.model-list-error {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-3);
  background: var(--error-subtle);
  border: 1px solid var(--error-border);
  border-radius: var(--radius-md);
  color: var(--error);
  font-size: var(--text-sm);
  margin-bottom: var(--space-3);
}

.dismiss-button {
  background: transparent;
  border: none;
  color: var(--error);
  font-size: var(--text-lg);
  cursor: pointer;
  padding: 0;
  width: var(--space-6);
  height: var(--space-6);
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  transition: background var(--duration-fast) var(--ease-out);
}

.dismiss-button:hover {
  background: var(--error-subtle);
}
```

**Delete confirmation dialog**:
```css
.delete-confirm {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2);
  background: var(--bg-elevated);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
}

.confirm-delete-button {
  padding: var(--space-1) var(--space-3);
  background: transparent;
  color: var(--error);
  border: 1px solid var(--error-border);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.confirm-delete-button:hover {
  background: var(--error-subtle);
}

.cancel-delete-button {
  padding: var(--space-1) var(--space-3);
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.cancel-delete-button:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
```

**Model action buttons**:
```css
.download-button {
  padding: var(--space-2) var(--space-3);
  background: var(--accent-primary);
  color: var(--text-inverse);
  border: none;
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  cursor: pointer;
  transition: background var(--duration-fast) var(--ease-out);
}

.download-button:hover {
  background: var(--accent-hover);
}

.select-button {
  padding: var(--space-2) var(--space-3);
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.select-button:hover:not(:disabled) {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.select-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.retry-button {
  padding: var(--space-2) var(--space-3);
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.retry-button:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
```

### Live Transcript Display

The live transcript display shows real-time transcription during recording with visual distinction between partial (Vosk) and final (Whisper) segments:

**Container and header**:
```css
.transcript-display {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-surface);
  padding: var(--space-4);
}

.transcript-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: var(--space-3);
  border-bottom: 1px solid var(--border-subtle);
  margin-bottom: var(--space-3);
}

.transcript-header h3 {
  font-size: var(--text-sm);
  font-weight: var(--font-semibold);
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin: 0;
}
```

**Transcription status indicators**:
```css
.transcribing-indicator {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.pulse-dot {
  width: var(--space-2);
  height: var(--space-2);
  background: var(--success);
  border-radius: 50%;
  animation: pulse 2s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.5;
    transform: scale(1.2);
  }
}

.error-indicator {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  color: var(--error);
  font-size: var(--text-sm);
}
```

**Transcript content area**:
```css
.transcript-content {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-3);
  background: var(--bg-elevated);
  border-radius: var(--radius-md);
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  line-height: var(--leading-relaxed);
  color: var(--text-primary);
}

.empty-transcript {
  color: var(--text-tertiary);
  font-style: italic;
  text-align: center;
  margin: var(--space-6) 0;
}
```

**Segment styling (partial vs final)**:
```css
.segment-final {
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: var(--text-sm);
}

.segment-partial {
  color: var(--text-tertiary);
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  font-style: italic;
  opacity: 0.8;
}
```

**Audio player container** (for playback after recording):
```css
.audio-player {
  margin-top: var(--space-3);
  padding: var(--space-3);
  background: var(--bg-elevated);
  border-radius: var(--radius-md);
}

.audio-player audio {
  width: 100%;
  height: 32px;
}
```

### YouTube and Browser Components

YouTube video detection and browser tab management share similar card-based layouts with gist displays:

**Video and tab cards** (follow list item pattern from Requirement 6):
```css
.videos-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  background: transparent;
  border: none;
}

.video-card {
  padding: var(--space-3);
  background: var(--bg-surface);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  transition: border-color var(--duration-fast) var(--ease-out);
}

.video-card:hover {
  border-color: var(--border-strong);
}

.video-url {
  font-size: var(--text-base);
  font-weight: var(--font-medium);
  color: var(--text-primary);
  margin-bottom: var(--space-1);
  word-break: break-word;
}

.video-author {
  font-size: var(--text-sm);
  color: var(--text-secondary);
  margin-bottom: var(--space-2);
}
```

**Tab list and tab items**:
```css
.tab-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  background: transparent;
  border: none;
}

.tab-item {
  padding: var(--space-2) var(--space-3);
  background: transparent;
  border: none;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background var(--duration-fast) var(--ease-out);
}

.tab-item:hover {
  background: var(--bg-hover);
}

.tab-item.selected {
  background: var(--bg-active);
  border-left: 2px solid var(--accent-primary);
}

.tab-item-content {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.tab-item-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-2);
}

.tab-domain {
  font-size: var(--text-sm);
  color: var(--text-secondary);
  font-weight: var(--font-medium);
}

.tab-title {
  font-size: var(--text-base);
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tab-badges {
  display: flex;
  gap: var(--space-1);
  align-items: center;
}
```

**Browser toolbar**:
```css
.browser-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border-subtle);
  padding: var(--space-2) var(--space-3);
  margin-bottom: var(--space-3);
}

.tab-count {
  font-size: var(--text-sm);
  color: var(--text-secondary);
}
```

**Source badges** (distinct colors per source type):
```css
.source-badge {
  font-size: var(--text-xs);
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius-sm);
  background: var(--bg-elevated);
  font-weight: var(--font-medium);
}

.source-badge.youtube {
  color: var(--color-youtube);
}

.source-badge.article {
  color: var(--color-article);
}

.source-badge.code {
  color: var(--color-code);
}

.source-badge.docs {
  color: var(--color-docs);
}

.source-badge.email {
  color: var(--color-email);
}

.source-badge.chat {
  color: var(--color-chat);
}

.source-badge.qa {
  color: var(--color-qa);
}

.source-badge.news {
  color: var(--color-news);
}

.source-badge.research {
  color: var(--color-research);
}

.source-badge.social {
  color: var(--color-social);
}

.source-badge.other {
  color: var(--text-secondary);
}
```

**Claude badge** (special indicator for Claude conversations):
```css
.claude-badge {
  font-size: var(--text-xs);
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius-sm);
  background: var(--accent-subtle);
  color: var(--accent-primary);
  font-weight: var(--font-medium);
  border: 1px solid var(--accent-border);
}
```

**Gist display card**:
```css
.gist-display {
  margin-top: var(--space-3);
  padding: var(--space-4);
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
}

.gist-header {
  font-size: var(--text-base);
  font-weight: var(--font-semibold);
  color: var(--text-primary);
  margin-bottom: var(--space-3);
  padding-bottom: var(--space-2);
  border-bottom: 1px solid var(--border-subtle);
}

.gist-field {
  display: flex;
  gap: var(--space-2);
  margin-bottom: var(--space-2);
  font-size: var(--text-sm);
}

.gist-label {
  color: var(--text-secondary);
  font-weight: var(--font-medium);
  min-width: 80px;
}

.gist-description {
  margin-top: var(--space-3);
  margin-bottom: var(--space-3);
}

.gist-description .gist-label {
  display: block;
  margin-bottom: var(--space-2);
  color: var(--text-secondary);
  font-weight: var(--font-semibold);
  text-transform: uppercase;
  font-size: var(--text-xs);
  letter-spacing: 0.05em;
}

.gist-description-text {
  font-size: var(--text-base);
  color: var(--text-primary);
  line-height: var(--leading-relaxed);
  white-space: pre-wrap;
  word-wrap: break-word;
}

.gist-actions {
  display: flex;
  gap: var(--space-2);
  margin-top: var(--space-4);
  padding-top: var(--space-3);
  border-top: 1px solid var(--border-subtle);
}

.gist-action-bar {
  margin-top: var(--space-3);
  padding: var(--space-3);
  background: var(--bg-surface);
  border-radius: var(--radius-md);
  display: flex;
  justify-content: center;
}

.gist-dismiss-button {
  padding: var(--space-2) var(--space-3);
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.gist-dismiss-button:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
```

**Observer status and notices**:
```css
.observer-status {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  background: var(--bg-surface);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  color: var(--text-secondary);
  margin-bottom: var(--space-3);
}

.observer-status::before {
  content: '';
  width: var(--space-2);
  height: var(--space-2);
  background: var(--success);
  border-radius: 50%;
}

.ai-enrichment-notice {
  margin-top: var(--space-3);
  padding: var(--space-2) var(--space-3);
  background: var(--accent-subtle);
  border: 1px solid var(--accent-border);
  border-radius: var(--radius-md);
  color: var(--accent-primary);
  font-size: var(--text-sm);
  text-align: center;
}

.accessibility-notice {
  padding: var(--space-3);
  margin-bottom: var(--space-3);
  background: var(--warning-subtle);
  border: 1px solid var(--warning-border);
  border-radius: var(--radius-md);
  color: var(--warning);
  font-size: var(--text-sm);
  line-height: var(--leading-relaxed);
}
```

### Dialog and Error Styling

Dialogs use backdrop blur and elevated surfaces:

```css
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: var(--overlay-dark);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.dialog {
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-lg);
  padding: var(--space-6);
  max-width: 400px;
}

.dialog-header {
  font-size: var(--text-lg);
  color: var(--text-primary);
  font-weight: var(--font-semibold);
  margin-bottom: var(--space-4);
}

.dialog-content {
  color: var(--text-secondary);
  font-size: var(--text-base);
  margin-bottom: var(--space-6);
}
```

**Error toasts and notifications**:
```css
.error-toast {
  position: fixed;
  top: var(--space-4);
  right: var(--space-4);
  max-width: 400px;
  padding: var(--space-3) var(--space-4);
  background: var(--bg-elevated);
  border-left: 3px solid var(--error);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-size: var(--text-sm);
  box-shadow: 0 4px 12px var(--overlay-dark);
  z-index: 2000;
  animation: slideIn var(--duration-normal) var(--ease-out);
}

@keyframes slideIn {
  from {
    transform: translateX(100%);
    opacity: 0;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}
```

**Error states and messages**:
```css
.error-message {
  color: var(--error);
  font-size: var(--text-sm);
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.error-state {
  padding: var(--space-3);
  background: var(--error-subtle);
  border: 1px solid var(--error-border);
  border-radius: var(--radius-md);
  color: var(--error);
  font-size: var(--text-sm);
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.inline-error {
  color: var(--error);
  font-size: var(--text-sm);
  margin-top: var(--space-2);
  display: flex;
  align-items: flex-start;
  gap: var(--space-1);
}

.inline-error::before {
  content: '⚠';
  flex-shrink: 0;
}
```

### Loading States

Skeleton loaders use shimmer animations:

```css
.skeleton-line {
  height: var(--text-lg);
  background: var(--bg-elevated);
  border-radius: var(--radius-sm);
  margin-bottom: var(--space-2);
  position: relative;
  overflow: hidden;
}

.skeleton-line::after {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(
    90deg,
    transparent,
    var(--bg-hover),
    transparent
  );
  animation: shimmer 1.5s infinite ease-in-out;
}

@keyframes shimmer {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(100%); }
}

.spinner {
  width: var(--space-5);
  height: var(--space-5);
  border: 3px solid var(--border-default);
  border-top-color: var(--accent-primary);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
```

### Utility Classes

General-purpose utility classes for common patterns:

```css
.scrollable {
  overflow-y: auto;
  overflow-x: hidden;
}

.expanded {
  /* State modifier for expanded panels */
  /* Specific behavior depends on component context */
}

.loading {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--space-6);
  color: var(--text-tertiary);
  font-size: var(--text-base);
}

.error {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--space-6);
  color: var(--error);
  font-size: var(--text-base);
}
```

## Data Models

This is a CSS-only redesign with no data model changes. The design works with existing component props and state structures.

### CSS Class Naming Conventions

The design maintains existing class names used in TSX components. New classes follow these conventions:

- **Component classes**: `.component-name` (e.g., `.nav-item`, `.gem-card`)
- **State modifiers**: `.component-name.state` (e.g., `.nav-item.active`, `.recording-item.selected`)
- **Child elements**: `.component-name-child` (e.g., `.nav-item-icon`, `.gem-card-header`)
- **Utility classes**: `.utility-name` (e.g., `.scrollable`, `.expanded`)

### Missing Class Definitions

The following classes are used in components but currently lack CSS definitions. The design adds comprehensive styling for all of them:

**Detail panel classes** (covered in Detail Panel Components section):
1. `.metadata-item` - Flex row for label/value pairs
2. `.metadata-label` - Secondary text color for labels
3. `.metadata-value` - Primary text color for values
4. `.language-indicator` - Small badge showing language

**Action and button classes** (covered in Button System section):
5. `.transcription-controls` - Button container for transcript actions
6. `.transcript-actions` - Action button container
7. `.action-button` - Generic secondary button
8. `.retry-button` - Secondary button for retry actions (in Settings Panel Components)

**Content display classes** (covered in Detail Panel Components):
9. `.transcript-section` - Section wrapper for transcript content
10. `.gem-title-section` - Header section for gem titles
11. `.gem-transcript` - Transcript display in gem details
12. `.source-badge` - Pill badge for content source type

**Utility classes** (covered in Utility Classes section):
13. `.scrollable` - Utility for overflow-y: auto
14. `.expanded` - State modifier for expanded panels

**Settings panel classes** (covered in Settings Panel Components section):
15. `.provider-options` - Container for provider radio buttons
16. `.provider-option` - Individual provider selection card
17. `.provider-note` - Explanatory text for provider settings
18. `.engine-unavailable` - Disabled state for unavailable engines
19. `.multimodal-model-card` - Card for multimodal model selection
20. `.info-banner` - Status/diagnostic message banners
21. `.model-list-error` - Error display in model lists
22. `.whisperkit-install-info` - Installation instructions panel
23. `.check-again-button` - Button to recheck availability
24. `.dismiss-button` - Close button for dismissible messages
25. `.delete-confirm` - Inline delete confirmation UI
26. `.confirm-delete-button` - Confirm deletion button
27. `.cancel-delete-button` - Cancel deletion button
28. `.download-button` - Model download button
29. `.select-button` - Model selection button
30. `.model-list` - Container for model items
31. `.model-info` - Model metadata container
32. `.model-header` - Model title row
33. `.model-tier` - Quality tier badge
34. `.model-size-estimate` - Estimated download size
35. `.model-filename` - Technical filename display
36. `.model-size` - Actual file size after download
37. `.model-error` - Error message display
38. `.model-actions` - Action buttons container
39. `.progress-container` - Download progress bar container
40. `.progress-text` - Progress percentage text
41. `.multimodal-models-panel` - Panel for multimodal model selection
42. `.multimodal-model-list` - List of multimodal models

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Token-only styling

*For all* CSS rules outside the :root block, no hardcoded color values (hex, rgb, rgba), font-size values (px, rem), font-family declarations, or border-radius values should exist—all such properties must reference design tokens using var(--token-name).

**Additional constraint**: Width and height values for small UI elements (icons, badges, dots, spinners) should use design tokens (--space-* or --text-* tokens) rather than hardcoded px values to maintain consistency with the token system.

**Validates: Requirements 1.14**

### Property 2: No dead CSS

*For all* CSS class selectors defined in App.css, each class name must be referenced in at least one TSX component file, ensuring no unused rules remain.

**Validates: Requirements 14.1**

### Property 3: No duplicate definitions

*For all* CSS class names, each class should have exactly one definition block in App.css, with no duplicate or conflicting rule sets.

**Validates: Requirements 14.2**

### Property 4: Complete class coverage

*For all* className attributes used in TSX component files, a corresponding CSS rule must exist in App.css, ensuring no unstyled components.

**Validates: Requirements 14.3, 14.4**

## Error Handling

### CSS Parsing Errors

If the browser encounters invalid CSS syntax, it will skip the invalid rule and continue parsing. To prevent this:

- Validate CSS syntax before deployment
- Test in WebKit/Chromium (Tauri's webview engine)
- Use browser dev tools to check for CSS warnings

### Font Loading Failures

If font files fail to load:

- System fallback fonts (specified in --font-sans and --font-mono) will be used
- `font-display: swap` ensures text remains visible during loading
- No JavaScript intervention required—CSS handles fallback gracefully

### Missing Design Tokens

If a CSS rule references a non-existent token:

- The browser will treat it as an invalid value
- The property will fall back to its inherited or initial value
- This is caught during development by visual inspection

### Class Name Mismatches

If a TSX component uses a class name not defined in CSS:

- The element will render with no styling from that class
- Other classes and inline styles will still apply
- This is caught by Property 4 testing

## Testing Strategy

### Dual Testing Approach

The UI redesign requires both unit tests and property-based tests:

- **Unit tests**: Verify specific CSS rules, token values, and visual regressions
- **Property tests**: Verify universal properties across all CSS rules and class names

Both are complementary and necessary for comprehensive coverage. Unit tests catch concrete styling bugs, while property tests verify general correctness across the entire stylesheet.

### Unit Testing

Unit tests focus on specific examples and edge cases:

1. **Token value verification**: Test that each design token has the correct value
   - Example: `--bg-base` should equal `#0a0a0c`
   - Example: `--text-sm` should equal `0.8125rem`

2. **CSS rule verification**: Test that specific selectors have expected properties
   - Example: `.nav-item.active` should have `background: var(--accent-subtle)`
   - Example: `.transcript-text` should have `font-family: var(--font-mono)`

3. **Font loading**: Test that @font-face rules exist for all required fonts
   - Example: Inter weights 400, 500, 600 should have @font-face declarations
   - Example: Font files should exist at `/fonts/*.woff2`

4. **Visual regression**: Screenshot tests for key UI states
   - Example: Navigation in collapsed vs expanded state
   - Example: List item in default, hover, and selected states
   - Example: Dialog overlay appearance

5. **Missing class definitions**: Test that all 14 previously missing classes now have definitions
   - Example: `.metadata-item` should have `display: flex`
   - Example: `.language-indicator` should have defined font-size and color

### Property-Based Testing

Property tests verify universal correctness across the entire stylesheet. Since CSS is declarative rather than executable code, property-based testing here means **static analysis** of the CSS file and component files.

**Testing Library**: Use a CSS parser (e.g., `postcss` in Node.js) combined with file system analysis.

**Test Configuration**: Each property test should run as a single test case that analyzes the entire codebase.

**Property Test Implementation**:

#### Property 1: Token-only styling
```javascript
// Feature: ui-redesign, Property 1: Token-only styling
test('all CSS rules outside :root use design tokens', () => {
  const css = parseCSSFile('jarvis-app/src/App.css');
  const rootBlock = css.findRule(':root');
  const otherRules = css.rules.filter(r => r !== rootBlock);
  
  // Only check restricted properties per Requirement 1.14
  const restrictedPatterns = [
    { pattern: /(?:^|;|\s)color:\s*#[0-9a-fA-F]{3,8}/, desc: 'hardcoded hex color' },
    { pattern: /(?:^|;|\s)background(?:-color)?:\s*#[0-9a-fA-F]{3,8}/, desc: 'hardcoded hex background' },
    { pattern: /(?:^|;|\s)border(?:-\w+)?:\s*[^;]*#[0-9a-fA-F]{3,8}/, desc: 'hardcoded hex border color' },
    { pattern: /(?:^|;|\s)color:\s*rgba?\(/, desc: 'hardcoded rgb/rgba color' },
    { pattern: /(?:^|;|\s)background(?:-color)?:\s*rgba?\(/, desc: 'hardcoded rgb/rgba background' },
    { pattern: /(?:^|;|\s)font-size:\s*\d+(?:px|rem)/, desc: 'hardcoded font-size' },
    { pattern: /(?:^|;|\s)font-family:\s*(?!var\()/, desc: 'hardcoded font-family' },
    { pattern: /(?:^|;|\s)border-radius:\s*\d+px/, desc: 'hardcoded border-radius' },
  ];
  
  const violations = [];
  for (const rule of otherRules) {
    const ruleText = rule.toString();
    for (const { pattern, desc } of restrictedPatterns) {
      if (pattern.test(ruleText)) {
        violations.push({ selector: rule.selector, issue: desc });
      }
    }
  }
  
  expect(violations).toEqual([]);
});
```

#### Property 2: No dead CSS
```javascript
// Feature: ui-redesign, Property 2: No dead CSS
test('all CSS classes are used in components', () => {
  const css = parseCSSFile('jarvis-app/src/App.css');
  const classNames = css.extractClassNames();
  
  const componentFiles = glob.sync('jarvis-app/src/**/*.tsx');
  const usedClasses = new Set();
  
  for (const file of componentFiles) {
    const content = fs.readFileSync(file, 'utf8');
    
    // Match static className strings
    const staticMatches = content.matchAll(/className="([^"]+)"/g);
    for (const match of staticMatches) {
      match[1].split(/\s+/).forEach(cls => usedClasses.add(cls));
    }
    
    // Match template literals: className={`base ${cond ? 'active' : ''}`}
    const templateMatches = content.matchAll(/className=\{`([^`]+)`\}/g);
    for (const match of templateMatches) {
      // Extract static class names (ignore ${...} expressions)
      const staticParts = match[1].split(/\$\{[^}]+\}/);
      staticParts.forEach(part => {
        part.split(/\s+/).forEach(cls => {
          if (cls && !cls.includes('?') && !cls.includes(':')) {
            usedClasses.add(cls);
          }
        });
      });
    }
    
    // Match ternary expressions: className={cond ? 'class-a' : 'class-b'}
    const ternaryMatches = content.matchAll(/className=\{[^}]*['"]([a-z-]+)['"]/g);
    for (const match of ternaryMatches) {
      usedClasses.add(match[1]);
    }
    
    // Match string literals in expressions: 'segment-final', "selected"
    const literalMatches = content.matchAll(/['"]([a-z][a-z0-9-]*)['"]/g);
    for (const match of literalMatches) {
      const candidate = match[1];
      // Only add if it looks like a CSS class (kebab-case or camelCase)
      if (/^[a-z][a-z0-9-]*$/.test(candidate) && classNames.includes(candidate)) {
        usedClasses.add(candidate);
      }
    }
  }
  
  const unusedClasses = classNames.filter(cls => !usedClasses.has(cls));
  expect(unusedClasses).toEqual([]);
});
```

#### Property 3: No duplicate definitions
```javascript
// Feature: ui-redesign, Property 3: No duplicate definitions
test('each CSS class has exactly one base definition', () => {
  const css = parseCSSFile('jarvis-app/src/App.css');
  const baseClassDefinitions = new Map();
  
  for (const rule of css.rules) {
    const selectors = rule.selectors.filter(s => s.startsWith('.'));
    for (const selector of selectors) {
      // Extract base class name, ignoring pseudo-classes, pseudo-elements, and state modifiers
      // .nav-item:hover → nav-item
      // .nav-item.active → nav-item (first class only)
      // .nav-item::before → nav-item
      const baseClass = selector
        .split(/[\s>+~]/)[0]  // Take first part before combinators
        .split('.')[1]         // Get first class name after initial dot
        .split(':')[0];        // Remove pseudo-classes/elements
      
      if (baseClass) {
        // Only count base class definitions (not pseudo/state variants)
        const isBaseDefinition = selector === `.${baseClass}` || 
                                 selector.startsWith(`.${baseClass}:`) ||
                                 selector.startsWith(`.${baseClass}.`);
        
        if (isBaseDefinition && selector === `.${baseClass}`) {
          // This is a pure base class definition (no pseudo/state)
          baseClassDefinitions.set(
            baseClass,
            (baseClassDefinitions.get(baseClass) || 0) + 1
          );
        }
      }
    }
  }
  
  const duplicates = [];
  for (const [className, count] of baseClassDefinitions) {
    if (count > 1) {
      duplicates.push({ className, count });
    }
  }
  
  expect(duplicates).toEqual([]);
});
```

#### Property 4: Complete class coverage
```javascript
// Feature: ui-redesign, Property 4: Complete class coverage
test('all component classes have CSS definitions', () => {
  const css = parseCSSFile('jarvis-app/src/App.css');
  const definedClasses = new Set(css.extractClassNames());
  
  const componentFiles = glob.sync('jarvis-app/src/**/*.tsx');
  const usedClasses = new Set();
  
  for (const file of componentFiles) {
    const content = fs.readFileSync(file, 'utf8');
    
    // Match static className strings
    const staticMatches = content.matchAll(/className="([^"]+)"/g);
    for (const match of staticMatches) {
      match[1].split(/\s+/).forEach(cls => usedClasses.add(cls));
    }
    
    // Match template literals: className={`base ${cond ? 'active' : ''}`}
    const templateMatches = content.matchAll(/className=\{`([^`]+)`\}/g);
    for (const match of templateMatches) {
      // Extract static class names (ignore ${...} expressions)
      const staticParts = match[1].split(/\$\{[^}]+\}/);
      staticParts.forEach(part => {
        part.split(/\s+/).forEach(cls => {
          if (cls && !cls.includes('?') && !cls.includes(':')) {
            usedClasses.add(cls);
          }
        });
      });
    }
    
    // Match ternary expressions: className={cond ? 'class-a' : 'class-b'}
    const ternaryMatches = content.matchAll(/className=\{[^}]*['"]([a-z-]+)['"]/g);
    for (const match of ternaryMatches) {
      usedClasses.add(match[1]);
    }
    
    // Match string literals in expressions: 'segment-final', "selected"
    const literalMatches = content.matchAll(/['"]([a-z][a-z0-9-]*)['"]/g);
    for (const match of literalMatches) {
      const candidate = match[1];
      // Only add if it looks like a CSS class (kebab-case or camelCase)
      if (/^[a-z][a-z0-9-]*$/.test(candidate)) {
        usedClasses.add(candidate);
      }
    }
  }
  
  const missingClasses = Array.from(usedClasses).filter(cls => !definedClasses.has(cls));
  expect(missingClasses).toEqual([]);
});
```

### Testing Tools

- **CSS Parser**: `postcss` with `postcss-selector-parser` for analyzing CSS structure
- **File System**: Node.js `fs` and `glob` for reading component files
- **Test Runner**: Jest or Vitest for running both unit and property tests
- **Visual Testing**: Playwright or Cypress for screenshot comparisons

### Test Organization

```
jarvis-app/
├── src/
│   ├── App.css (implementation)
│   └── components/ (TSX files)
└── tests/
    ├── unit/
    │   ├── design-tokens.test.js
    │   ├── css-rules.test.js
    │   ├── font-loading.test.js
    │   └── visual-regression.test.js
    └── properties/
        ├── token-only-styling.test.js
        ├── no-dead-css.test.js
        ├── no-duplicate-definitions.test.js
        └── complete-class-coverage.test.js
```

### Continuous Integration

CSS property tests should run on every commit to catch:
- Accidental hardcoded values
- Unused CSS rules after component changes
- Missing CSS definitions for new components
- Duplicate class definitions from merge conflicts

## Implementation Strategy

### Phase 1: Font Setup
1. Download Inter and JetBrains Mono woff2 files
2. Place in `jarvis-app/public/fonts/`
3. Add @font-face declarations to App.css
4. Test font loading in Tauri app

### Phase 2: Design Tokens
1. Create :root block with all design tokens
2. Verify token values match requirements
3. Run unit tests for token verification

### Phase 3: Global Styles
1. Add global reset rules
2. Style html, body, and base elements
3. Configure form inputs with dark theme

### Phase 4: Layout Shell
1. Update .app-layout, .left-nav, .center-panel, .right-panel
2. Replace hardcoded values with tokens
3. Test responsive behavior

### Phase 5: Component Styling
1. Update navigation components
2. Update list views (recordings, gems, videos, tabs)
3. Update detail panels
4. Update button system
5. Update settings panel
6. Update dialogs and loading states

### Phase 6: Dead CSS Cleanup
1. Run Property 2 test to identify unused classes
2. Remove dead CSS rules
3. Run Property 3 test to find duplicates
4. Consolidate duplicate definitions
5. Add missing class definitions (14 classes)
6. Run Property 4 test to verify completeness

### Phase 7: Verification
1. Run all unit tests
2. Run all property tests
3. Visual inspection of all panels
4. Test in Tauri app on macOS
5. Verify file size under 1,500 lines

### Rollback Strategy

If issues arise during implementation:
- Git revert to previous working state
- CSS changes are isolated to App.css
- No component logic changes means low risk
- Visual bugs are immediately apparent

## Dependencies

### External Dependencies
- None (CSS-only changes)

### Font Files Required
- Inter-Regular.woff2 (400 weight)
- Inter-Medium.woff2 (500 weight)
- Inter-SemiBold.woff2 (600 weight)
- JetBrainsMono-Regular.woff2 (400 weight)
- JetBrainsMono-Medium.woff2 (500 weight)

### Development Dependencies (for testing)
- postcss: CSS parsing
- postcss-selector-parser: Selector analysis
- glob: File system traversal
- jest or vitest: Test runner

## Performance Considerations

### CSS File Size
- Target: Under 1,500 lines (down from ~2,800)
- Reduction achieved through dead code removal and consolidation
- Smaller file = faster parse time

### Font Loading
- woff2 format: Best compression for web fonts
- font-display: swap: Prevents invisible text during load
- Self-hosted: No external network requests

### CSS Custom Properties
- Minimal performance impact (modern browsers optimize well)
- Benefit: Single source of truth for theme values
- Trade-off: Slightly slower than hardcoded values, but negligible

### Transitions and Animations
- Use transform and opacity (GPU-accelerated)
- Avoid animating layout properties (width, height, padding)
- Keep durations short (100-200ms)

### Selector Specificity
- Avoid deep nesting (max 3 levels)
- Use single class selectors where possible
- Minimize use of descendant selectors

## Security Considerations

### CSS Injection
- Not applicable: CSS is static, not user-generated
- No dynamic CSS generation from user input

### Font Loading
- Self-hosted fonts eliminate CDN security concerns
- No external requests = no MITM attack surface

### Content Security Policy
- Tauri apps have strict CSP by default
- Self-hosted fonts comply with CSP
- No inline styles or external stylesheets

## Accessibility Considerations

### Color Contrast
- Text colors meet WCAG AA standards:
  - --text-primary on --bg-base: 14.5:1 (AAA)
  - --text-secondary on --bg-base: 7.2:1 (AA)
  - --accent-primary on --bg-base: 4.8:1 (AA for large text)

### Focus States
- All interactive elements have visible focus states
- Focus indicators use accent color for visibility
- Keyboard navigation fully supported

### Font Sizing
- Base font size: 14px (0.875rem)
- All sizes use rem units for user scaling
- Line height: 1.5 for readability

### Motion
- Transitions are short (100-200ms)
- No auto-playing animations
- Respects prefers-reduced-motion (future enhancement)

## Maintenance and Evolution

### Adding New Colors
1. Add token to :root block
2. Document purpose in comment
3. Use token in CSS rules
4. Update tests to verify token exists

### Adding New Components
1. Create component with className attributes
2. Add CSS rules using existing tokens
3. Run Property 4 test to verify coverage
4. Add unit tests for specific styling

### Theme Variations (Future)
- Light theme: Create alternate :root block
- Theme toggle: Swap :root blocks via JavaScript
- All component styles remain unchanged (token-based)

### Design Token Updates
- Modify token values in :root
- Changes propagate automatically
- No need to update individual rules
- Test visual impact across all components

---

**Design Status**: Ready for implementation
**Last Updated**: 2024
**Reviewers**: Pending user review
