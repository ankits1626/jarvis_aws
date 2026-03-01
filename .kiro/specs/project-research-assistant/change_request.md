# Change Request: Conversational Research Agent

## Evolution of Thinking

**v1 (original):** One-shot black box — click "Research", get results. No user control.

**v2 (topic bucket):** Two-step structured UI — suggest topics → user picks → run research. Better control, but feels like a form, not an assistant.

**v3 (this proposal):** Chat-first interaction — the Research Agent lives in a right panel as a conversational collaborator. The user talks to it. It suggests, the user refines, and research emerges from dialogue.

## The Core Idea

The Research Agent IS a chat agent. The first interaction with any project is a conversation, not a button click.

```
┌─────────────────────────────────┬──────────────────────────────┐
│         LEFT PANEL              │        RIGHT PANEL           │
│                                 │                              │
│  Project: "AWS Migration"       │  🔬 Research Assistant       │
│  ─────────────────────          │  ─────────────────────       │
│  Description: [............]    │                              │
│  Objective:   [............]    │  Agent: I see your project   │
│                                 │  is about AWS Migration.     │
│                                 │  Here are some research      │
│  Gems (3)                       │  topics I'd suggest:         │
│  ┌─────────────────────┐       │                              │
│  │ ECS Best Practices  │       │  1. ECS to Fargate migration │
│  │ Fargate Pricing     │       │     networking changes       │
│  │ Container Security  │       │  2. Fargate task definition  │
│  └─────────────────────┘       │     best practices           │
│                                 │  3. AWS Fargate vs ECS EC2   │
│                                 │     cost comparison          │
│                                 │                              │
│                                 │  Want me to search for       │
│                                 │  these? Feel free to add     │
│                                 │  your own topics too.        │
│                                 │                              │
│                                 │  User: Also add "container   │
│                                 │  security scanning tools"    │
│                                 │  and drop #2                 │
│                                 │                              │
│                                 │  Agent: Got it! Searching    │
│                                 │  for 3 topics...             │
│                                 │                              │
│                                 │  [web result cards]          │
│                                 │  [gem suggestion cards]      │
│                                 │                              │
│                                 │  ┌────────────────────────┐  │
│                                 │  │ Type a message...    ↵ │  │
│                                 │  └────────────────────────┘  │
└─────────────────────────────────┴──────────────────────────────┘
```

## How It Works

### Conversation Lifecycle

```
1. User creates/opens project
   → Right panel shows Research Assistant chat
   → Agent auto-sends opening message with suggested topics
     (uses project title/description/objective as context)

2. User responds naturally:
   "Also search for kubernetes security"
   "Remove topic 2, it's not relevant"
   "Yes, go ahead and search"
   "What about serverless alternatives?"
   "Summarize what we've found so far"

3. Agent understands intent and acts:
   → Topic refinement  → updates internal topic list
   → "go ahead" / "search" → executes web + gem search, renders results
   → Follow-up questions → answers from project context (Chatbot Q&A)
   → "summarize" → runs summarization pipeline

4. Results appear as rich messages in the chat:
   → Web results as clickable cards
   → Gem suggestions with "Add to Project" buttons
   → Summaries as formatted markdown
```

### Why Reuse the Chatbot Engine

The existing `Chatbot` + `Chatable` pattern already handles:
- Session management (start, message, history, end)
- LLM prompting with context
- Message history and persistence to disk
- IntelQueue serialization

The Research Agent chat IS a project chat — `ProjectChatSource` assembles project + gem context. The difference is the system prompt: instead of generic Q&A, the agent's system prompt includes instructions to suggest topics, understand research commands, and format results.

**Key reuse:** `ProjectChatSource.get_context()` already assembles project metadata + gem content. The Chatbot calls this on every message, so the agent always has fresh context as gems get added.

### What the Agent Needs Beyond Basic Chat

The chat-first approach means the agent needs to understand **intent** within the conversation:

| User Says | Agent Intent | Action |
|-----------|-------------|--------|
| "search for X" / "add topic X" | Topic addition | Add to internal topic list |
| "remove topic 2" / "drop the kubernetes one" | Topic removal | Remove from topic list |
| "go ahead" / "search" / "find resources" | Execute research | Run web search + gem search for current topics |
| "summarize" / "what have we found?" | Summarize | Run summarization on project gems |
| General question about project content | Q&A | Answer from project context (standard Chatbot) |

**Two approaches to handle this:**

**Option A: LLM-driven intent (simpler, recommended for v1)**
- Give the agent a rich system prompt that includes the topic list
- LLM decides what to do based on conversation
- For "search" intent, the agent's response includes a structured marker (e.g., `[SEARCH_TOPICS: ...]`) that the frontend parses
- Frontend renders rich results when it detects the marker

**Option B: Tool-calling / structured output (more robust, v2)**
- Agent has "tools" it can call: `suggest_topics`, `web_search`, `add_gem`, `summarize`
- Each response may contain tool calls alongside text
- More reliable intent detection but more complex to implement

**Recommendation:** Start with **Option A** for v1. The LLM is good enough at understanding "search for these" vs "tell me about X". If intent detection proves unreliable, upgrade to Option B.

## Technical Implications

### What Changes from Current Design

| Component | Current Design | New Design |
|-----------|---------------|------------|
| `ProjectResearchAgent.research()` | Single method, auto-generates + auto-searches | Split: `suggest_topics()` for opening message, `run_research(topics)` for execution |
| Research trigger | "Research" button → one-shot pipeline | Chat message → agent decides when to search |
| Frontend research UI | `ProjectResearchPanel` — standalone component | Chat panel in right sidebar — messages + rich cards |
| Project detail layout | Left panel only (gems list) | Left panel (gems) + Right panel (research chat) |
| Results rendering | Separate panel above gem list | Inline in chat as rich message cards |

### What Stays the Same

- `TavilyProvider`, `CompositeSearchProvider` — unchanged
- `SearchResultProvider` trait — unchanged
- `ProjectChatSource` + `Chatable` — unchanged (still needed for context assembly)
- `Chatbot` engine — reused as-is for session/message management
- `IntelProvider.chat()` — unchanged
- `summarize()` logic — same, just triggered from chat instead of button
- `lib.rs` provider registration — unchanged
- All Phase 1-2 work — unchanged

### Backend Changes

**`ProjectResearchAgent` methods:**

```
suggest_topics(project_id) → Vec<String>
  — Called once when chat opens. Returns suggested topics for opening message.

run_research(project_id, topics: Vec<String>) → ProjectResearchResults
  — Called when user says "search" / "go ahead". Takes user-curated topics.

summarize(project_id) → String
  — Same as before. Called when user says "summarize".

start_chat / send_chat_message / get_chat_history / end_chat
  — Same as before. The chat panel uses these.
```

The agent's system prompt for the research chat is richer than generic Q&A — it includes:
- The current topic list
- Instructions for topic management
- Instructions for when/how to trigger research
- The project context (via ProjectChatSource)

**Tauri commands:**
- `suggest_project_topics(project_id)` → `Vec<String>` (new)
- `run_project_research(project_id, topics)` → `ProjectResearchResults` (renamed)
- All chat commands stay the same
- `get_project_summary` stays the same

### Frontend Changes

**New: Research chat panel (right sidebar)**
- Chat message list with rich rendering (text + web cards + gem cards)
- Text input at bottom
- Auto-opens when project is created or "Research" is clicked
- Agent sends first message with suggested topics

**Modified: Project detail layout**
- Split into left (gem list) + right (research chat) panels
- Right panel collapsible/toggleable

## Open Questions

1. **Auto-open chat on new project?**
   - Recommendation: Yes. When creating a project, the right panel auto-opens with the agent's topic suggestions. This is the "first interaction" the user described.

2. **Chat persistence across sessions?**
   - Recommendation: Yes, via existing Chatbot session logs (.md files). User can pick up where they left off.

3. **Multiple research rounds?**
   - Recommendation: Yes. User can say "search for more about X" at any point. Results accumulate in the chat history. Each "search" trigger is a new `run_research` call.

4. **Rich message rendering — how complex?**
   - Recommendation: v1 keeps it simple. Agent responds with markdown + a structured results block. Frontend detects result blocks and renders cards. No need for a full chat widget framework.

5. **Summarize from chat vs button?**
   - Recommendation: Both. "Summarize" button still works (convenience), AND user can type "summarize this project" in chat. Same backend method either way.

## Summary

The Research Agent becomes a conversational collaborator that lives in a right panel. The user's first interaction with any project is a chat where the agent suggests topics, the user refines them through natural dialogue, and research results flow back as rich messages. This reuses the existing Chatbot engine and ProjectChatSource, with a richer system prompt and frontend rendering for research-specific content (web cards, gem cards, topic lists).

The key insight: **research is a conversation, not a form submission.**
