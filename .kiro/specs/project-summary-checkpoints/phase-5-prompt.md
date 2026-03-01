# Phase 5: Frontend — TypeScript Types + ProjectSummaryChat Component + CSS

## What You're Building

You are implementing Phase 5 of the Project Summary Checkpoints feature. This phase creates the frontend:

1. **`ProjectSummaryResult` TypeScript interface** in `types.ts`
2. **`ProjectSummaryChat` React component** — a new component with 4 states (empty, generating, review, saved)
3. **CSS styles** for the summary chat in `App.css`

**No RightPanel integration in this phase.** That's Phase 6. The component just needs to exist, build, and be ready to mount.

## Context

Read these files before starting:

- **Design doc:** `.kiro/specs/project-summary-checkpoints/design.md` — see "Frontend" sections
- **Tasks:** `.kiro/specs/project-summary-checkpoints/tasks.md` — Phase 5 has Tasks 11-13
- **Existing pattern:** `src/components/ProjectResearchChat.tsx` — follow this component's structure exactly (imports, hooks, invoke pattern, chat messages, auto-scroll)
- **Types file:** `src/state/types.ts` — where to add the new interface
- **CSS file:** `src/App.css` — where to add the new styles

## Tauri Commands Available (from Phase 4)

These three commands are registered and callable from the frontend:

```typescript
// Generate summary for review (does not save)
invoke<ProjectSummaryResult>('generate_project_summary_checkpoint', { projectId })

// Save reviewed summary as a gem
invoke<Gem>('save_project_summary_checkpoint', { projectId, summaryContent, compositeDoc })

// Ask a question about the summary
invoke<string>('send_summary_question', { question, summary, compositeDoc })
```

**Important:** Tauri uses `camelCase` for argument names on the JS side. The Rust commands use `snake_case` but Tauri auto-converts. Follow the existing pattern in `ProjectResearchChat.tsx` — use `{ projectId }` not `{ project_id }`.

## Tasks

### Task 11: Add TypeScript Type

**File:** `src/state/types.ts`

Add near the existing `ProjectResearchResults` interface (around line 1012):

```typescript
/** Result of summary checkpoint generation — returned for review before saving */
export interface ProjectSummaryResult {
  summary: string;
  composite_doc: string;
  gems_analyzed: number;
  chunks_used: number;
}
```

### Task 12: Create `ProjectSummaryChat` Component

**File:** `src/components/ProjectSummaryChat.tsx` (new file)

Follow the `ProjectResearchChat.tsx` pattern exactly for structure. Here's the blueprint:

**Imports:**
```typescript
import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { ProjectSummaryResult } from '../state/types';
```

**Local types (not exported — same pattern as ProjectResearchChat):**
```typescript
type SummaryState = 'empty' | 'generating' | 'review' | 'saved';

interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
}
```

**Props:**
```typescript
interface ProjectSummaryChatProps {
  projectId: string;
  projectTitle: string;
  onGemSaved?: () => void;
}
```

**State hooks:**
```typescript
const [state, setState] = useState<SummaryState>('empty');
const [summaryResult, setSummaryResult] = useState<ProjectSummaryResult | null>(null);
const [saved, setSaved] = useState(false);
const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
const [input, setInput] = useState('');
const [loading, setLoading] = useState(false);
const [error, setError] = useState<string | null>(null);
const messagesEndRef = useRef<HTMLDivElement>(null);
```

**Auto-scroll (same pattern as ProjectResearchChat line 42-44):**
```typescript
useEffect(() => {
  messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
}, [chatMessages, loading]);
```

**Reset on project change:**
```typescript
useEffect(() => {
  setState('empty');
  setSummaryResult(null);
  setSaved(false);
  setChatMessages([]);
  setInput('');
  setError(null);
}, [projectId]);
```

**Handler: `handleGenerate`**
```typescript
const handleGenerate = async () => {
  setState('generating');
  setError(null);
  try {
    const result = await invoke<ProjectSummaryResult>(
      'generate_project_summary_checkpoint',
      { projectId }
    );
    setSummaryResult(result);
    setState('review');
    setSaved(false);
    setChatMessages([]);
  } catch (err) {
    setState('empty');
    setError(err instanceof Error ? err.message : String(err));
  }
};
```

**Handler: `handleSave`**
```typescript
const handleSave = async () => {
  if (!summaryResult) return;
  try {
    await invoke('save_project_summary_checkpoint', {
      projectId,
      summaryContent: summaryResult.summary,
      compositeDoc: summaryResult.composite_doc,
    });
    setSaved(true);
    setState('saved');
    onGemSaved?.();
  } catch (err) {
    setError(err instanceof Error ? err.message : String(err));
  }
};
```

**Handler: `handleAskQuestion`**
```typescript
const handleAskQuestion = async () => {
  const question = input.trim();
  if (!question || loading || !summaryResult) return;

  setInput('');
  setChatMessages(prev => [...prev, { role: 'user', content: question }]);
  setLoading(true);

  try {
    const answer = await invoke<string>('send_summary_question', {
      question,
      summary: summaryResult.summary,
      compositeDoc: summaryResult.composite_doc,
    });
    setChatMessages(prev => [...prev, { role: 'assistant', content: answer }]);
  } catch (err) {
    setChatMessages(prev => [...prev, {
      role: 'assistant',
      content: `Error: ${err instanceof Error ? err.message : String(err)}`,
    }]);
  } finally {
    setLoading(false);
  }
};
```

**Handler: `handleKeyPress`**
```typescript
const handleKeyPress = (e: React.KeyboardEvent) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    handleAskQuestion();
  }
};
```

**Render — four states:**

```tsx
// Empty state
if (state === 'empty') {
  return (
    <div className="summary-chat-empty">
      <h3>Project Summary</h3>
      <p>Generate a summary covering all key points from every gem in this project.</p>
      <button className="action-button" onClick={handleGenerate}>
        Generate Summary
      </button>
      {error && <p className="summary-error">{error}</p>}
    </div>
  );
}

// Generating state
if (state === 'generating') {
  return (
    <div className="summary-chat-generating">
      <div className="spinner" />
      <span>Analyzing gems...</span>
    </div>
  );
}

// Review / Saved state
return (
  <div className="summary-chat">
    <div className="summary-preview">
      <pre className="summary-content">{summaryResult?.summary}</pre>
      <div className="summary-meta">
        {summaryResult?.gems_analyzed} gems analyzed · {summaryResult?.chunks_used} chunks
      </div>
    </div>

    <div className="summary-actions">
      {!saved ? (
        <button className="action-button" onClick={handleSave}>Save as Gem</button>
      ) : (
        <span className="summary-saved-badge">✓ Saved</span>
      )}
      <button className="action-button" onClick={handleGenerate}>Regenerate</button>
    </div>

    <div className="summary-qa">
      {chatMessages.map((msg, i) => (
        <div key={i} className={`chat-message chat-${msg.role}`}>
          <div className="chat-bubble">{msg.content}</div>
        </div>
      ))}
      {loading && (
        <div className="chat-message chat-assistant">
          <div className="chat-bubble thinking">Thinking...</div>
        </div>
      )}
      <div ref={messagesEndRef} />
    </div>

    <div className="chat-input-bar">
      <input
        type="text"
        value={input}
        onChange={(e) => setInput(e.target.value)}
        onKeyDown={handleKeyPress}
        placeholder="Ask about the summary..."
        disabled={loading}
        className="chat-input"
      />
      <button
        onClick={handleAskQuestion}
        disabled={!input.trim() || loading}
        className="chat-send-button"
      >
        Send
      </button>
    </div>
    {error && <p className="summary-error">{error}</p>}
  </div>
);
```

### Task 13: Add CSS Styles

**File:** `src/App.css`

Add these styles after the existing chat styles (around the end of the file or after the `.chat-*` rules):

```css
/* ── Summary Chat ── */

.summary-chat {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.summary-chat-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 12px;
  text-align: center;
  padding: 24px;
}

.summary-chat-empty p {
  color: var(--text-secondary);
  font-size: 13px;
  max-width: 280px;
}

.summary-chat-generating {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 12px;
  color: var(--text-secondary);
}

.summary-preview {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

.summary-content {
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-wrap: break-word;
}

.summary-meta {
  font-size: 11px;
  color: var(--text-muted);
  padding-top: 8px;
  border-top: 1px solid var(--border-color);
  margin-top: 12px;
}

.summary-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border-top: 1px solid var(--border-color);
}

.summary-saved-badge {
  font-size: 12px;
  color: var(--success-color);
  padding: 4px 10px;
}

.summary-qa {
  border-top: 1px solid var(--border-color);
  padding: 8px 16px;
  max-height: 200px;
  overflow-y: auto;
}

.summary-error {
  color: var(--error-color, #ef4444);
  font-size: 12px;
  margin-top: 8px;
}
```

**Note:** The existing `.chat-message`, `.chat-bubble`, `.chat-input-bar`, `.chat-input`, `.chat-send-button`, `.spinner`, and `.action-button` classes are already defined in `App.css`. Reuse them — don't redefine.

## Checkpoint

Run `npm run build` (or the frontend build command). The component should compile with no TypeScript errors.

**What to verify:**
- `ProjectSummaryResult` interface exported from `types.ts`
- `ProjectSummaryChat.tsx` exists with all 4 state renders
- Component imports `invoke` from `@tauri-apps/api/core` (not from a wrapper)
- Tauri commands use `camelCase` argument names: `projectId`, `summaryContent`, `compositeDoc`
- CSS uses existing design tokens (`var(--text-secondary)`, `var(--border-color)`, etc.)
- Reuses existing CSS classes: `.action-button`, `.chat-message`, `.chat-bubble`, `.chat-input-bar`, `.spinner`
- No modifications to existing files except adding to `types.ts` and `App.css`

## If You Have Questions

- **Ask me before guessing.** Especially about CSS variable names — check the existing `:root` in `App.css` for exact token names.
- **Don't touch RightPanel.tsx yet.** That's Phase 6. The component just needs to exist and build.
- **Follow ProjectResearchChat.tsx patterns exactly.** Same import style, same hook patterns, same invoke pattern.
- **The `Gem` type is already in `types.ts`.** You don't need to add it. The `save` handler uses `invoke<Gem>` but you can also use `invoke` without the type annotation since we don't use the return value beyond the success check.
