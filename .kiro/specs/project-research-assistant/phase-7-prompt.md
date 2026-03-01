# Phase 7: CSS Styling — Research Chat, Topic Chips, Web Cards, Gem Cards

## Context

You are implementing Phase 7 of the Project Research Assistant feature for Jarvis AWS (Tauri desktop app: Rust backend + React/TypeScript frontend).

**Phases 1-6 are COMPLETE:**
- Backend: search infrastructure, providers, agent, 7 Tauri commands
- Frontend: `ProjectResearchChat` component, wired into `RightPanel`, `App.tsx`, `ProjectsContainer`

**Phase 7 adds CSS styles** to `App.css` for the research chat UI. The component renders but has no custom styling yet — the chat messages reuse existing `.chat-*` classes from the Chat Panel, but research-specific elements (topic chips, web result cards, gem suggestion cards, system messages) need new styles.

---

## File: `jarvis-app/src/App.css` (4676 lines)

### Design System Reference

The app uses CSS custom properties (design tokens) defined in `:root`. **You MUST use these variables** instead of raw values to maintain consistency:

| Token | Value | Usage |
|-------|-------|-------|
| `--bg-base` | `#0a0a0c` | Page background |
| `--bg-surface` | `#111114` | Panel backgrounds |
| `--bg-elevated` | `#18181b` | Cards, elevated surfaces |
| `--bg-hover` | `#1e1e22` | Hover states |
| `--border-subtle` | `#1e1e22` | Light borders |
| `--border-default` | `#27272a` | Standard borders |
| `--border-strong` | `#3f3f46` | Emphasized borders |
| `--text-primary` | `#fafafa` | Primary text |
| `--text-secondary` | `#a1a1aa` | Secondary text |
| `--text-tertiary` | `#71717a` | Muted text |
| `--accent-primary` | `#6366f1` | Primary accent (indigo) |
| `--accent-hover` | `#818cf8` | Accent hover |
| `--accent-subtle` | `rgba(99, 102, 241, 0.12)` | Accent background |
| `--error` | `#ef4444` | Error/danger color |
| `--info` | `#3b82f6` | Info color (blue) |
| `--text-xs` | `0.75rem` (12px) | |
| `--text-sm` | `0.8125rem` (13px) | |
| `--text-base` | `0.875rem` (14px) | |
| `--space-1` through `--space-8` | 4px grid (4, 8, 12, 16, 20, 24, 28, 32) | |
| `--radius-sm` | `4px` | |
| `--radius-md` | `6px` | |
| `--font-medium` | `500` | |
| `--font-semibold` | `600` | |
| `--duration-fast` | `100ms` | |
| `--ease-out` | `cubic-bezier(0.16, 1, 0.3, 1)` | |

### Existing Styles That Research Chat Reuses

The `ProjectResearchChat` component reuses these existing CSS classes (already defined in App.css lines 3891-4068):

- `.chat-message`, `.chat-user`, `.chat-assistant` — message alignment
- `.chat-bubble` — message bubble styling (max-width, padding, radius)
- `.chat-user .chat-bubble` — user bubble (accent background)
- `.chat-assistant .chat-bubble` — assistant bubble (elevated bg, border)
- `.chat-bubble.thinking` — loading pulse animation
- `.chat-input-bar` — input bar container (flex, border-top)
- `.chat-input` — text input styling
- `.chat-send-button` — send button styling

Also reuses from line 2695:
- `.source-badge` — base badge styling for gem source types (YouTube, Article, etc.)

**DO NOT modify these existing classes.** Only add new classes.

### What To Add

Append all new CSS to the **end of `App.css`** (after line 4676). Add this exact CSS block:

```css
/* === Research Chat Styles === */

/* ── Research Chat Layout ── */

.research-chat {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.research-chat-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: var(--space-3);
  color: var(--text-secondary);
}

.research-chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.research-chat-messages::-webkit-scrollbar {
  width: 6px;
}

.research-chat-messages::-webkit-scrollbar-track {
  background: var(--bg-surface);
  border-radius: var(--radius-sm);
}

.research-chat-messages::-webkit-scrollbar-thumb {
  background: var(--border-default);
  border-radius: var(--radius-sm);
}

.research-chat-messages::-webkit-scrollbar-thumb:hover {
  background: var(--border-strong);
}

/* ── Topic Chips ── */

.research-topics-list {
  margin-top: var(--space-3);
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.research-topic-chip {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius-md);
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--border-default);
  font-size: var(--text-sm);
}

.topic-remove {
  background: none;
  border: none;
  color: var(--text-tertiary);
  cursor: pointer;
  font-size: var(--text-xs);
  padding: 2px 6px;
  transition: color var(--duration-fast) var(--ease-out);
}

.topic-remove:hover {
  color: var(--error);
}

.research-go-button {
  margin-top: var(--space-2);
  align-self: flex-end;
}

/* ── System Messages ── */

.chat-system-msg {
  font-size: 11px;
  color: var(--text-tertiary);
  text-align: center;
  padding: var(--space-1) 0;
  font-style: italic;
}

/* ── Research Sections in Chat ── */

.research-section {
  margin-top: var(--space-3);
}

.research-section-title {
  font-size: 11px;
  font-weight: var(--font-semibold);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-tertiary);
  margin: 0 0 var(--space-2) 0;
}

/* ── Web Result Cards ── */

.web-result-card {
  padding: 10px;
  border-radius: var(--radius-md);
  cursor: pointer;
  margin-bottom: 6px;
  border: 1px solid var(--border-default);
  transition: background var(--duration-fast) var(--ease-out),
              border-color var(--duration-fast) var(--ease-out);
}

.web-result-card:hover {
  background: var(--bg-hover);
  border-color: var(--accent-primary);
}

.web-result-header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-bottom: var(--space-1);
}

.source-type-badge {
  display: inline-block;
  padding: 1px 5px;
  border-radius: 3px;
  font-size: 10px;
  font-weight: var(--font-semibold);
  text-transform: uppercase;
}

.source-paper { background: rgba(168, 85, 247, 0.15); color: #c084fc; }
.source-article { background: rgba(59, 130, 246, 0.15); color: #60a5fa; }
.source-video { background: rgba(239, 68, 68, 0.15); color: #f87171; }
.source-other { background: rgba(107, 114, 128, 0.15); color: #9ca3af; }

.web-result-domain {
  font-size: 11px;
  color: var(--text-tertiary);
}

.web-result-title {
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  margin-bottom: 2px;
}

.web-result-snippet {
  font-size: var(--text-xs);
  color: var(--text-secondary);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

/* ── Gem Suggestion Cards ── */

.research-gem-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  padding: var(--space-2) 10px;
  margin-bottom: var(--space-1);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-default);
}

.research-gem-card .gem-info {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex: 1;
  min-width: 0;
}

.research-gem-card .gem-title {
  font-size: var(--text-sm);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.research-add-gem {
  flex-shrink: 0;
  padding: var(--space-1) 10px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--accent-primary);
  background: transparent;
  color: var(--accent-primary);
  font-size: 11px;
  cursor: pointer;
  transition: background var(--duration-fast) var(--ease-out);
}

.research-add-gem:hover {
  background: var(--accent-subtle);
}

.research-add-gem.added {
  border-color: var(--text-tertiary);
  color: var(--text-tertiary);
  cursor: default;
}
```

---

## CSS Class → Component Mapping

This table shows which CSS class is used by which JSX element in `ProjectResearchChat.tsx`:

| CSS Class | JSX Element | Purpose |
|-----------|-------------|---------|
| `.research-chat` | Root `<div>` | Full-height flex column layout |
| `.research-chat-loading` | Loading state `<div>` | Centered spinner during initialization |
| `.research-chat-messages` | Messages container | Scrollable message list |
| `.chat-message` | Each message wrapper | Existing: flex row |
| `.chat-user` / `.chat-assistant` / `.chat-${msg.role}` | Message role class | Existing: alignment |
| `.chat-bubble` | Message bubble | Existing: padding, radius, max-width |
| `.chat-text` | Text content inside bubble | (new, no special styling needed — inherits) |
| `.research-topics-list` | Topics container inside bubble | Column of topic chips |
| `.research-topic-chip` | Individual topic chip | Flex row with remove button |
| `.topic-remove` | "×" button on chip | Muted, turns red on hover |
| `.research-go-button` | "Search (N topics)" button | Uses existing `.action-button` + right-aligned |
| `.chat-system-msg` | System messages (topic added/removed) | Centered, italic, muted |
| `.research-section` | Web results / gem results container | Top margin |
| `.research-section-title` | "From the web" / "From your library" | Uppercase, muted, small |
| `.web-result-card` | Clickable web result card | Border, hover accent |
| `.web-result-header` | Badge + domain row | Flex row |
| `.source-type-badge` | "PAPER" / "ARTICLE" / "VIDEO" / "OTHER" | Colored pill badge |
| `.source-paper` / `.source-article` / `.source-video` / `.source-other` | Badge color variants | Background + text color |
| `.web-result-domain` | Domain text (e.g., "arxiv.org") | Small, muted |
| `.web-result-title` | Result title | Medium weight |
| `.web-result-snippet` | Result snippet | 2-line clamp |
| `.research-gem-card` | Gem suggestion card | Flex row, border |
| `.gem-info` (inside `.research-gem-card`) | Badge + title container | Flex, min-width: 0 |
| `.gem-title` (inside `.research-gem-card`) | Gem title text | Truncated with ellipsis |
| `.research-add-gem` | "Add" / "Added" button | Outline accent button |
| `.research-add-gem.added` | Button after gem is added | Muted, non-clickable |
| `.chat-input-bar` | Input bar at bottom | Existing: flex, border-top |
| `.chat-input` | Text input | Existing |
| `.chat-send-button` | Send button | Existing |
| `.chat-bubble.thinking` | "Thinking..." loading bubble | Existing: pulse animation |

---

## Design Decisions

1. **Use design tokens everywhere.** The design spec (design.md) used raw values like `#888`, `#666`, `#333`. Convert these to the correct design token:
   - `#888` / `#aaa` → `var(--text-secondary)` or `var(--text-tertiary)`
   - `#666` → `var(--text-tertiary)`
   - `#333` → `var(--border-default)`
   - `#ef4444` → `var(--error)`
   - `#3b82f6` → `var(--info)` or `var(--accent-primary)` where indigo is better

2. **Scrollbar styling** matches the existing `.chat-messages` scrollbar pattern (lines 3907-3923).

3. **`.source-type-badge` vs `.source-badge`**: These are different classes. `.source-badge` (line 2695) is used by gems in GemsPanel/GemDetailPanel. `.source-type-badge` is new, used only for web search result type badges. The color variants (`.source-paper`, etc.) are distinct from gem badge variants (`.source-badge.youtube`, etc.).

4. **Transition properties** added to `.web-result-card` and `.topic-remove` for smooth hover effects, matching the app's existing transition patterns.

5. **The `.research-add-gem` button** uses `var(--accent-primary)` (indigo) instead of the design spec's `#3b82f6` (blue) to match the app's accent color. The hover state uses `var(--accent-subtle)` for consistency.

---

## Gotchas

1. **Append only — do not modify existing CSS.** The existing `.chat-*` classes are shared with `ChatPanel.tsx` (recording chat). Any changes would affect both components.

2. **`.chat-text` class** is used in JSX (`<div className="chat-text">`) but doesn't need dedicated CSS — it inherits from `.chat-bubble`. Don't add an empty rule for it.

3. **`.action-button`** is already defined elsewhere in App.css. The "Search (N topics)" button uses `className="action-button research-go-button"` — the `.research-go-button` only adds `margin-top` and `align-self`, not button styling.

4. **`.spinner`** animation is already defined globally (used by Co-Pilot, recordings). The research chat loading state uses `<div className="spinner" />` which picks up the existing animation.

5. **Dark theme only.** This app has no light theme. All rgba values and colors are designed for dark backgrounds.

6. **The gem suggestion cards reuse `.source-badge`** (existing class from line 2695) for the gem's source type badge — NOT `.source-type-badge`. The class `source-type-badge` is only for web result source types.

---

## Verification Checklist

After implementation:
- [ ] All new CSS appended to end of `App.css`
- [ ] No existing CSS classes modified
- [ ] Design tokens used (no raw hex colors except in `.source-*` badge variants)
- [ ] `.research-chat` — flex column, height 100%
- [ ] `.research-chat-loading` — centered with gap
- [ ] `.research-chat-messages` — flex 1, overflow-y auto, scrollbar styled
- [ ] `.research-topic-chip` — flex row, subtle background, border
- [ ] `.topic-remove` — muted color, hover turns red (var(--error))
- [ ] `.research-go-button` — right-aligned with margin-top
- [ ] `.chat-system-msg` — centered, italic, muted
- [ ] `.web-result-card` — border, hover changes bg + border-color
- [ ] `.source-type-badge` — inline pill badge, uppercase
- [ ] `.source-paper` purple, `.source-article` blue, `.source-video` red, `.source-other` gray
- [ ] `.web-result-snippet` — 2-line clamp with `-webkit-line-clamp`
- [ ] `.research-gem-card` — flex row, space-between
- [ ] `.gem-title` inside `.research-gem-card` — ellipsis overflow
- [ ] `.research-add-gem` — outline button, `.added` state muted
- [ ] `npm run build` passes (no CSS parse errors)
