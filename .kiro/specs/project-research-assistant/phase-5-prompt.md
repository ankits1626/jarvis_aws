# Phase 5: Frontend — TypeScript Types + ProjectResearchChat Component

## Context

You are implementing Phase 5 of the Project Research Assistant feature for Jarvis AWS (Tauri desktop app: Rust backend + React/TypeScript frontend).

**Phases 1-4 are COMPLETE** — the Rust backend is fully functional:
- `SearchResultProvider` trait extended with `web_search()` and `supports_web_search()`
- `TavilyProvider`, `CompositeSearchProvider`, and `ProjectResearchAgent` all implemented
- 7 Tauri commands registered: `suggest_project_topics`, `run_project_research`, `get_project_summary`, `start_project_chat`, `send_project_chat_message`, `get_project_chat_history`, `end_project_chat`
- `add_gems_to_project` (pre-existing command) also available

**Phase 5 adds the frontend**: TypeScript type definitions + the main `ProjectResearchChat` component.

---

## Part 1: Add TypeScript Types to `src/state/types.ts`

**File**: `jarvis-app/src/state/types.ts` (980 lines)

Add the following two interfaces at the END of the file (after line 980, before the closing of the file). These mirror the Rust structs in `src-tauri/src/search/provider.rs` and `src-tauri/src/agents/project_agent.rs`.

### 1.1 Add `WebSearchResult` interface

```typescript
/** A search result from the web (not a gem). Mirrors Rust WebSearchResult. */
export interface WebSearchResult {
  title: string;
  url: string;
  snippet: string;
  source_type: 'Paper' | 'Article' | 'Video' | 'Other';
  domain: string;
  published_date: string | null;
}
```

**Why a string union instead of an enum?** Rust's `WebSourceType` enum serializes to these exact strings via serde. The TS union matches 1:1.

### 1.2 Add `ProjectResearchResults` interface

```typescript
/** Combined results from project research. Mirrors Rust ProjectResearchResults. */
export interface ProjectResearchResults {
  web_results: WebSearchResult[];
  suggested_gems: GemSearchResult[];
  topics_searched: string[];
}
```

**Note**: `GemSearchResult` is already defined at line 604 of types.ts — no need to add it.

### 1.3 Verify exports

Both interfaces are top-level `export interface` declarations, so they're automatically exported. No additional export statement needed.

---

## Part 2: Create `ProjectResearchChat.tsx`

**File**: `jarvis-app/src/components/ProjectResearchChat.tsx` (NEW FILE)

This is a chat interface with rich message rendering for research results. It follows the existing `ChatPanel.tsx` pattern (same file, same CSS class conventions) but adds:
1. Auto-suggests topics on mount (agent's opening message)
2. Renders web result cards and gem suggestion cards inline in chat
3. Handles "Run Research" action with user-curated topics
4. Keyword-based intent detection (v1) for routing user messages

### Reference: Existing `ChatPanel.tsx` Pattern

Look at `src/components/ChatPanel.tsx` (145 lines) for the established conventions:
- **State**: `messages`, `input`, `thinking/loading`, `messagesEndRef` for auto-scroll
- **CSS classes**: `chat-panel`, `chat-messages`, `chat-message`, `chat-bubble`, `chat-input-bar`, `chat-input`, `chat-send-button`
- **Key handler**: `handleKeyPress` with Enter (no shift) to send
- **Auto-scroll**: `useEffect` watching `[messages, loading]` with `messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })`

### 2.1 Imports and Types

```tsx
import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-shell';
import type { ProjectResearchResults, WebSearchResult, GemSearchResult } from '../state/types';
```

**Local ChatMessage type** (NOT the one from types.ts — this one has research-specific fields):

```typescript
interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  researchResults?: ProjectResearchResults;
  suggestedTopics?: string[];
}
```

### 2.2 Component Props

```typescript
interface ProjectResearchChatProps {
  projectId: string;
  projectTitle: string;
  onGemsAdded?: () => void;  // Called after a gem is added to the project
}
```

### 2.3 Component State

```typescript
const [messages, setMessages] = useState<ChatMessage[]>([]);
const [input, setInput] = useState('');
const [loading, setLoading] = useState(false);
const [topics, setTopics] = useState<string[]>([]);
const [addedGemIds, setAddedGemIds] = useState<Set<string>>(new Set());
const [initializing, setInitializing] = useState(true);
const messagesEndRef = useRef<HTMLDivElement>(null);
```

### 2.4 Auto-scroll effect

Same pattern as ChatPanel:
```typescript
useEffect(() => {
  messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
}, [messages, loading]);
```

### 2.5 Auto-suggest topics on mount

This is the agent's "opening move" — when the component mounts, it calls `suggest_project_topics` and renders the response as topic chips.

```typescript
useEffect(() => {
  let cancelled = false;
  const suggestTopics = async () => {
    setInitializing(true);
    try {
      const suggested = await invoke<string[]>('suggest_project_topics', { projectId });
      if (cancelled) return;
      setTopics(suggested);
      setMessages([{
        role: 'assistant',
        content: `I see your project is about **${projectTitle}**. Here are some research topics I'd suggest:`,
        suggestedTopics: suggested,
      }]);
    } catch (err) {
      if (cancelled) return;
      setMessages([{
        role: 'assistant',
        content: `I couldn't generate topics: ${err}. You can type your own research topics below.`,
      }]);
    } finally {
      if (!cancelled) setInitializing(false);
    }
  };
  suggestTopics();
  return () => { cancelled = true; };
}, [projectId, projectTitle]);
```

**Key detail**: The `cancelled` flag prevents state updates if the component unmounts before the invoke returns (e.g., user switches projects quickly).

### 2.6 `handleRunResearch` — Execute research with curated topics

```typescript
const handleRunResearch = useCallback(async (researchTopics: string[]) => {
  if (researchTopics.length === 0) return;

  setLoading(true);
  setMessages(prev => [
    ...prev,
    { role: 'user', content: `Search for: ${researchTopics.join(', ')}` },
    { role: 'assistant', content: `Searching ${researchTopics.length} topics...` },
  ]);

  try {
    const results = await invoke<ProjectResearchResults>('run_project_research', {
      projectId,
      topics: researchTopics,
    });

    // Replace the "Searching..." placeholder with actual results
    setMessages(prev => {
      const updated = [...prev];
      updated[updated.length - 1] = {
        role: 'assistant',
        content: `Found ${results.web_results.length} web resources and ${results.suggested_gems.length} matching gems from your library.`,
        researchResults: results,
      };
      return updated;
    });
  } catch (err) {
    setMessages(prev => {
      const updated = [...prev];
      updated[updated.length - 1] = {
        role: 'assistant',
        content: `Research failed: ${err}. You can try again or refine your topics.`,
      };
      return updated;
    });
  } finally {
    setLoading(false);
  }
}, [projectId]);
```

**Pattern**: Optimistically adds a "Searching..." message, then replaces it with results. This gives immediate feedback while the backend runs.

### 2.7 `handleSendMessage` — Keyword-based intent detection (v1)

```typescript
const handleSendMessage = async () => {
  if (!input.trim() || loading) return;
  const userMessage = input.trim();
  setInput('');

  const lower = userMessage.toLowerCase();

  // Intent: Run research
  if (lower.includes('search') || lower.includes('go ahead') || lower.includes('find')) {
    await handleRunResearch(topics);
    return;
  }

  // Intent: Summarize project
  if (lower.includes('summarize') || lower.includes('summary')) {
    setLoading(true);
    setMessages(prev => [
      ...prev,
      { role: 'user', content: userMessage },
      { role: 'assistant', content: 'Summarizing your project...' },
    ]);

    try {
      const summary = await invoke<string>('get_project_summary', { projectId });
      setMessages(prev => {
        const updated = [...prev];
        updated[updated.length - 1] = { role: 'assistant', content: summary };
        return updated;
      });
    } catch (err) {
      setMessages(prev => {
        const updated = [...prev];
        updated[updated.length - 1] = { role: 'assistant', content: `Failed to summarize: ${err}` };
        return updated;
      });
    } finally {
      setLoading(false);
    }
    return;
  }

  // Default: treat as a new topic to add
  setTopics(prev => [...prev, userMessage]);
  setMessages(prev => [
    ...prev,
    { role: 'user', content: userMessage },
    {
      role: 'assistant',
      content: `Added "${userMessage}" to your research topics. Say "search" when you're ready.`,
    },
  ]);
};
```

### 2.8 `handleRemoveTopic` — Remove a topic from the curated list

```typescript
const handleRemoveTopic = (index: number) => {
  const removed = topics[index];
  setTopics(prev => prev.filter((_, i) => i !== index));
  setMessages(prev => [
    ...prev,
    { role: 'system', content: `Removed topic: "${removed}"` },
  ]);
};
```

### 2.9 `handleAddGem` — Add a gem suggestion to the project

```typescript
const handleAddGem = async (gemId: string) => {
  try {
    await invoke('add_gems_to_project', { projectId, gemIds: [gemId] });
    setAddedGemIds(prev => new Set(prev).add(gemId));
    onGemsAdded?.();
  } catch (err) {
    console.error('Failed to add gem:', err);
  }
};
```

**Calls**: `add_gems_to_project` — this is a pre-existing Tauri command (not one we added). It takes `projectId` and `gemIds: string[]`.

### 2.10 `handleKeyPress`

Same as ChatPanel:
```typescript
const handleKeyPress = (e: React.KeyboardEvent) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    handleSendMessage();
  }
};
```

### 2.11 Render — Initializing state

```tsx
if (initializing) {
  return (
    <div className="research-chat">
      <div className="research-chat-loading">
        <div className="spinner" />
        <span>Analyzing your project...</span>
      </div>
    </div>
  );
}
```

### 2.12 Render — Main chat UI

The main return renders four kinds of messages:

1. **Regular text bubbles** (user/assistant) — same as ChatPanel
2. **Topic suggestion messages** — assistant bubble with topic chips + "Search" button
3. **Research result messages** — assistant bubble with web cards + gem cards
4. **System messages** — centered muted text (no bubble)

```tsx
return (
  <div className="research-chat">
    <div className="research-chat-messages">
      {messages.map((msg, index) => (
        <div key={index} className={`chat-message chat-${msg.role}`}>
          {msg.role !== 'system' ? (
            <div className="chat-bubble">
              <div className="chat-text">{msg.content}</div>

              {/* Topic chips with remove buttons */}
              {msg.suggestedTopics && (
                <div className="research-topics-list">
                  {topics.map((topic, i) => (
                    <div key={i} className="research-topic-chip">
                      <span>{i + 1}. {topic}</span>
                      <button className="topic-remove" onClick={() => handleRemoveTopic(i)}>x</button>
                    </div>
                  ))}
                  <button
                    className="action-button research-go-button"
                    onClick={() => handleRunResearch(topics)}
                    disabled={topics.length === 0 || loading}
                  >
                    Search ({topics.length} topics)
                  </button>
                </div>
              )}

              {/* Web result cards */}
              {msg.researchResults && msg.researchResults.web_results.length > 0 && (
                <div className="research-section">
                  <h4 className="research-section-title">From the web</h4>
                  {msg.researchResults.web_results.map((result, i) => (
                    <div key={i} className="web-result-card" onClick={() => open(result.url)}>
                      <div className="web-result-header">
                        <span className={`source-type-badge source-${result.source_type.toLowerCase()}`}>
                          {result.source_type}
                        </span>
                        <span className="web-result-domain">{result.domain}</span>
                      </div>
                      <div className="web-result-title">{result.title}</div>
                      <div className="web-result-snippet">{result.snippet}</div>
                    </div>
                  ))}
                </div>
              )}

              {/* Gem suggestion cards */}
              {msg.researchResults && msg.researchResults.suggested_gems.length > 0 && (
                <div className="research-section">
                  <h4 className="research-section-title">From your library</h4>
                  {msg.researchResults.suggested_gems.map((gem) => (
                    <div key={gem.id} className="research-gem-card">
                      <div className="gem-info">
                        <span className={`source-badge ${gem.source_type.toLowerCase()}`}>{gem.source_type}</span>
                        <span className="gem-title">{gem.title}</span>
                      </div>
                      <button
                        className={`research-add-gem ${addedGemIds.has(gem.id) ? 'added' : ''}`}
                        onClick={() => handleAddGem(gem.id)}
                        disabled={addedGemIds.has(gem.id)}
                      >
                        {addedGemIds.has(gem.id) ? 'Added' : '+ Add'}
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          ) : (
            <div className="chat-system-msg">{msg.content}</div>
          )}
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
        onKeyPress={handleKeyPress}
        placeholder="Add a topic, say 'search', or ask a question..."
        disabled={loading}
        className="chat-input"
      />
      <button
        onClick={handleSendMessage}
        disabled={!input.trim() || loading}
        className="chat-send-button"
      >
        Send
      </button>
    </div>
  </div>
);
```

### CSS Classes Used

The component uses these CSS classes. **Do NOT add CSS in this phase** — Phase 7 handles styling. For now, the component reuses existing chat classes where possible:

| Class | Source | Notes |
|---|---|---|
| `chat-message`, `chat-user`, `chat-assistant` | Existing (ChatPanel) | Reused as-is |
| `chat-bubble`, `chat-text` | Existing (ChatPanel) | Reused as-is |
| `chat-input-bar`, `chat-input`, `chat-send-button` | Existing (ChatPanel) | Reused as-is |
| `thinking` | Existing (ChatPanel) | Reused as-is |
| `spinner` | Existing (global) | Reused as-is |
| `action-button` | Existing (global) | Reused as-is |
| `source-badge` | Existing (GemsPanel) | Reused as-is |
| `research-chat` | **NEW** | Phase 7 |
| `research-chat-loading` | **NEW** | Phase 7 |
| `research-chat-messages` | **NEW** | Phase 7 |
| `research-topics-list` | **NEW** | Phase 7 |
| `research-topic-chip` | **NEW** | Phase 7 |
| `topic-remove` | **NEW** | Phase 7 |
| `research-go-button` | **NEW** | Phase 7 |
| `research-section` | **NEW** | Phase 7 |
| `research-section-title` | **NEW** | Phase 7 |
| `web-result-card` | **NEW** | Phase 7 |
| `web-result-header` | **NEW** | Phase 7 |
| `source-type-badge` | **NEW** | Phase 7 |
| `web-result-domain` | **NEW** | Phase 7 |
| `web-result-title` | **NEW** | Phase 7 |
| `web-result-snippet` | **NEW** | Phase 7 |
| `research-gem-card` | **NEW** | Phase 7 |
| `gem-info`, `gem-title` | **NEW** | Phase 7 |
| `research-add-gem` | **NEW** | Phase 7 |
| `chat-system-msg` | **NEW** | Phase 7 |

---

## Tauri Commands Referenced

These are the backend commands this component invokes. All are already implemented and registered:

| Command | Args | Returns | Used In |
|---|---|---|---|
| `suggest_project_topics` | `{ projectId: string }` | `string[]` | Mount effect |
| `run_project_research` | `{ projectId: string, topics: string[] }` | `ProjectResearchResults` | `handleRunResearch` |
| `get_project_summary` | `{ projectId: string }` | `string` | `handleSendMessage` (summarize intent) |
| `add_gems_to_project` | `{ projectId: string, gemIds: string[] }` | `number` | `handleAddGem` |

---

## Gotchas

1. **`open` from `@tauri-apps/plugin-shell`** — this opens URLs in the system browser. Check that `plugin-shell` is in the frontend dependencies. If not, install it: `npm install @tauri-apps/plugin-shell`. Also verify the shell plugin is registered in `src-tauri/capabilities/default.json` — look for `"shell:default"` or `"shell:allow-open"`.

2. **`onKeyPress` is deprecated** in React 17+. If the project uses React 18+, use `onKeyDown` instead. Check `package.json` for the React version. If React 18+, use `onKeyDown` and rename the handler to `handleKeyDown`.

3. **Camel case for Tauri invoke args** — Tauri's `#[tauri::command]` converts snake_case Rust params to camelCase for the frontend. So `project_id` in Rust becomes `projectId` in the invoke call. All the invoke calls above already use the correct camelCase.

4. **The `ChatMessage` interface is local** — do NOT import it from `types.ts`. The component defines its own `ChatMessage` with `researchResults?` and `suggestedTopics?` fields that don't exist on the backend struct.

5. **Topic chips render from `topics` state, not `msg.suggestedTopics`** — the `msg.suggestedTopics` flag is just a marker that this message should render the topic chip UI. The actual topics come from the `topics` state variable, which is mutable (users can add/remove topics after the initial suggestion).

---

## Verification Checklist

After implementation:
- [ ] `WebSearchResult` and `ProjectResearchResults` interfaces added to `types.ts`
- [ ] `ProjectResearchChat.tsx` created in `src/components/`
- [ ] Component exports a named export `ProjectResearchChat` (not default export)
- [ ] All 4 Tauri commands invoked with correct camelCase arg names
- [ ] `open` import from `@tauri-apps/plugin-shell` works (plugin installed)
- [ ] `npm run build` or `npm run check` passes (no TypeScript errors)
- [ ] Component is NOT imported anywhere yet (that's Phase 6)

---

## Questions You Should Ask Before Starting

1. Is `@tauri-apps/plugin-shell` already in `package.json`? If not, should I install it?
2. What React version is the project using? (Determines `onKeyPress` vs `onKeyDown`)
3. Should the component use named export (`export function ProjectResearchChat`) or default export (`export default function ProjectResearchChat`)? — I recommend named export to match other components, but check the codebase convention.
