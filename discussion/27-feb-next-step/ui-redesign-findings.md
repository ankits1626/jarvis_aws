# Jarvis UI Redesign — Research Findings

## Problem

The current UI uses light gray backgrounds, default system fonts, basic borders, and inconsistent spacing. It looks like a prototype, not a professional desktop app.

## Current CSS Audit

| Metric | Value |
|--------|-------|
| Total lines | ~2,800 |
| CSS variables | 0 (all colors hardcoded) |
| Missing class definitions | 14 |
| Duplicate/conflicting rules | ~8 |
| Dead/legacy code | ~500 lines |
| Theme support | None |

**Missing CSS classes** (used in components, no definition):
`metadata-item`, `metadata-label`, `metadata-value`, `transcription-controls`, `transcript-section`, `language-indicator`, `transcript-actions`, `gem-title-section`, `gem-transcript`, `action-button`, `scrollable`, `expanded`, `source-badge`, `retry-button`

---

## Design Direction: Dark Theme (Linear / Notion / VS Code inspired)

### Why Dark

- Industry standard for developer/power-user tools
- Reduces eye strain for extended use
- Creates visual depth with subtle layering
- Professional, modern feel out of the box

### Reference Apps

| App | Key Design Trait |
|-----|-----------------|
| **Linear** | Cool-tinted grays, tight spacing, LCH color model, surgical precision |
| **Notion** | Warm neutrals, generous whitespace, clean typography hierarchy |
| **VS Code** | Layered surfaces, icon-driven nav, activity bar pattern |
| **Slack** | Sidebar color branding, clear content hierarchy |
| **Raycast** | macOS-native feel, blur effects, compact density |

---

## Color Palette

### Core Surfaces (cool-tinted grays)

```css
:root {
  /* Backgrounds — darkest to lightest */
  --bg-base:        #0a0a0c;   /* App background */
  --bg-surface:     #111114;   /* Panels, cards */
  --bg-elevated:    #18181b;   /* Elevated elements, modals */
  --bg-hover:       #1e1e22;   /* Hover states */
  --bg-active:      #27272a;   /* Active/selected states */

  /* Borders */
  --border-subtle:  #1e1e22;   /* Dividers, card edges */
  --border-default: #27272a;   /* Input borders */
  --border-strong:  #3f3f46;   /* Focus rings */

  /* Text */
  --text-primary:   #fafafa;   /* Headings, primary content */
  --text-secondary: #a1a1aa;   /* Labels, metadata */
  --text-tertiary:  #71717a;   /* Placeholders, disabled */
  --text-inverse:   #09090b;   /* Text on accent backgrounds */

  /* Accent (blue — Linear-inspired) */
  --accent-primary:   #6366f1;  /* Buttons, links, active nav */
  --accent-hover:     #818cf8;  /* Hover on accent elements */
  --accent-subtle:    rgba(99, 102, 241, 0.12); /* Accent backgrounds */
  --accent-border:    rgba(99, 102, 241, 0.3);  /* Accent-tinted borders */

  /* Semantic */
  --success:        #22c55e;
  --warning:        #f59e0b;
  --error:          #ef4444;
  --info:           #3b82f6;
}
```

### Design Principles

- **3 surface layers** only: base, surface, elevated — keeps visual hierarchy clear
- **Cool-tinted grays** (hint of blue/purple) rather than pure gray — more refined
- **Indigo accent** (#6366f1) — professional, not aggressive, works well in dark UIs
- **Text: 3 tiers** — primary (white), secondary (zinc-400), tertiary (zinc-500)

---

## Typography

### Font Recommendations

| Font | Use Case | Source | Used By |
|------|----------|--------|---------|
| **Inter** | UI text, body | Google Fonts / self-host | Linear, Figma, Vercel, GitHub |
| **Geist Sans** | Alternative to Inter | npm `geist` | Vercel, Next.js |
| **JetBrains Mono** | Code, transcripts | Google Fonts | JetBrains IDEs |

**Recommendation**: Inter + JetBrains Mono

- Inter: Designed specifically for computer screens, excellent at small sizes, widely proven
- JetBrains Mono: Ligatures, clear distinction between similar characters (0/O, 1/l/I)

### Type Scale

```css
:root {
  --font-sans:  'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  --font-mono:  'JetBrains Mono', 'SF Mono', 'Fira Code', monospace;

  /* Sizes */
  --text-xs:    0.75rem;    /* 12px — badges, captions */
  --text-sm:    0.8125rem;  /* 13px — metadata, labels */
  --text-base:  0.875rem;   /* 14px — body text (desktop standard) */
  --text-lg:    1rem;       /* 16px — section headers */
  --text-xl:    1.125rem;   /* 18px — panel titles */
  --text-2xl:   1.25rem;    /* 20px — page titles */

  /* Weights */
  --font-normal:   400;
  --font-medium:   500;
  --font-semibold: 600;

  /* Line heights */
  --leading-tight:  1.3;
  --leading-normal: 1.5;
  --leading-relaxed: 1.6;

  /* Letter spacing — tighter for headings, normal for body */
  --tracking-tight:  -0.01em;
  --tracking-normal:  0;
}
```

### Key Rule

Desktop apps use **14px** as base, not 16px. This gives the dense, professional feel of Linear/VS Code vs the spacious web-page feel.

---

## Spacing & Layout

```css
:root {
  /* Spacing scale (4px base) */
  --space-1:  4px;
  --space-2:  8px;
  --space-3:  12px;
  --space-4:  16px;
  --space-5:  20px;
  --space-6:  24px;
  --space-8:  32px;

  /* Layout */
  --nav-width-collapsed: 48px;
  --nav-width-expanded:  180px;
  --radius-sm:  4px;
  --radius-md:  6px;
  --radius-lg:  8px;

  /* Transitions */
  --duration-fast:   100ms;
  --duration-normal: 150ms;
  --duration-slow:   200ms;
  --ease-out: cubic-bezier(0.16, 1, 0.3, 1);
}
```

---

## Component Patterns

### Left Nav

```
- bg: --bg-surface
- border-right: 1px solid --border-subtle
- Active item: --accent-subtle background + --accent-primary icon/text
- Hover: --bg-hover
- Icon size: 18px, label: --text-sm
- Collapse toggle: bottom, subtle
```

### Center Panel (Lists)

```
- bg: --bg-base
- List items: no background, hover → --bg-hover, selected → --bg-active
- Item padding: --space-3 horizontal, --space-2 vertical
- Dividers: 1px --border-subtle (or none, just spacing)
- Title: --text-base --font-medium --text-primary
- Metadata: --text-sm --text-secondary
- Tags: --text-xs, --accent-subtle bg, --accent-primary text, --radius-sm
```

### Right Panel (Detail)

```
- bg: --bg-surface
- border-left: 1px solid --border-subtle
- Section headers: --text-sm --font-semibold --text-secondary uppercase tracking-wide
- Content: --text-base --text-primary
- Transcript: --font-mono --text-sm, bg --bg-elevated, padding --space-4
```

### Buttons

```css
/* Primary */
.btn-primary {
  background: var(--accent-primary);
  color: var(--text-inverse);
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  padding: var(--space-2) var(--space-4);
  border-radius: var(--radius-md);
  border: none;
  transition: background var(--duration-fast) var(--ease-out);
}
.btn-primary:hover { background: var(--accent-hover); }

/* Secondary / Ghost */
.btn-secondary {
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border-default);
}
.btn-secondary:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

/* Destructive */
.btn-danger {
  background: transparent;
  color: var(--error);
  border: 1px solid rgba(239, 68, 68, 0.3);
}
.btn-danger:hover {
  background: rgba(239, 68, 68, 0.1);
}
```

### Audio Player

```
- Custom styling on <audio> is limited — wrap in container
- Container: --bg-elevated, --radius-md, padding --space-3
- Consider a thin progress bar with --accent-primary
```

### Cards (Gem Cards, Recording Items)

```
- No visible border by default — rely on hover/selection state
- Hover: --bg-hover + subtle left border accent
- Selected: --bg-active + left border --accent-primary (2px)
- Padding: --space-3
- Border-radius: --radius-md
```

---

## Before → After Comparison

| Element | Current | Proposed |
|---------|---------|----------|
| Background | `#f8f9fa` (light gray) | `#0a0a0c` (dark) |
| Panel bg | `#ffffff` | `#111114` |
| Font | System default (16px) | Inter 14px |
| Borders | `#e0e0e0` (visible gray) | `#1e1e22` (subtle) |
| Buttons | Blue with heavy borders | Indigo with no borders, subtle radius |
| Nav | Gray background, round icons | Dark surface, accent highlights |
| Tags | Gray pills | Accent-tinted subtle pills |
| Spacing | Inconsistent | 4px grid system |
| Transitions | Mix of durations | Consistent 100-200ms ease-out |
| CSS variables | 0 | ~40+ design tokens |
| Code font | None specified | JetBrains Mono |

---

## Implementation Strategy

### Phase 1: Foundation
1. Define all CSS custom properties (`:root`)
2. Load Inter + JetBrains Mono fonts
3. Apply global resets (body bg, font, text color)

### Phase 2: Layout Shell
4. Style three-panel container (bg, borders)
5. Style LeftNav (dark surface, accent active states)
6. Style panel dividers

### Phase 3: Components
7. Style center panel lists (recordings, gems, YouTube)
8. Style right panel detail views
9. Style buttons (primary, secondary, danger)
10. Style forms/inputs (Settings)
11. Style tags, badges, metadata

### Phase 4: Polish
12. Add transitions and hover states
13. Style audio player container
14. Style transcript display (mono font)
15. Fix 14 missing CSS classes
16. Remove ~500 lines dead CSS
17. Resolve duplicate rules

### Phase 5: Verify
18. Test all 6 nav states visually
19. Test collapsed/expanded nav
20. Test recording detail + transcription flow
21. Test gem detail + enrichment flow

---

## Font Loading

```html
<!-- In index.html <head> -->
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
```

Or self-host for offline Tauri use (recommended):
- Download Inter and JetBrains Mono woff2 files
- Place in `jarvis-app/public/fonts/`
- Use `@font-face` declarations in CSS

---

## Key Decision Points

1. **Google Fonts vs self-hosted?** — Self-host recommended for Tauri (works offline)
2. **Pure dark or dark + light toggle?** — Start with dark only, add toggle later if needed
3. **How aggressive on dead CSS cleanup?** — Recommend full cleanup in same pass
4. **Custom audio player?** — Default `<audio>` with styled container is pragmatic for now
