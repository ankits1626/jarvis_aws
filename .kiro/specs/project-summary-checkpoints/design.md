# Project Summary Checkpoints — Design Document

## Overview

This design adds a **Summary Chat** — a dedicated tab in the project right panel where users generate an LLM-powered summary of all project gems, review it, ask questions, and optionally save it as a gem checkpoint.

### Design Goals

1. **Separate from Research Chat**: Summary is its own tab with its own component and state. Research discovers; Summary distills.
2. **Generate ≠ Save**: Two decoupled operations. User reviews and interacts before committing.
3. **Chunked summarization**: Handles projects of any size by splitting on gem boundaries and merging.
4. **Provenance via `composite_summary_of_all_gems.md`**: The full input document is preserved in the saved gem's knowledge directory.
5. **Extend, don't replace**: New methods on the existing `ProjectResearchAgent`. No new agent struct.

### Key Design Decisions

- **`gem.md` as per-gem input**: Each gem's assembled knowledge file already combines content, enrichment, transcript, and copilot data. One file per gem, no need to stitch subfiles ourselves.
- **Gem-boundary chunking**: Chunks never split mid-gem. Each chunk = 1 or more complete gems. This preserves per-gem context for the LLM.
- **Single call fast path**: If the composite document fits in one chunk (~4000 tokens), skip chunking and merge — one LLM call total.
- **`KNOWN_SUBFILES` extension**: Add `composite_summary_of_all_gems.md` to the constant in `local_store.rs`. Non-summary gems simply won't have this file — it's skipped during enumeration.
- **`ProjectSummaryResult` returned from generate**: Contains both the summary and composite doc so the frontend can pass both to the save command without the backend re-building the composite.
- **Tab state independence**: Research, Summary, and Detail tabs each maintain their own state. Switching tabs doesn't reset anything. Only switching projects resets (via `key={projectId}`).

### User Experience Flow

```
1. User opens a project → RightPanel shows [Research] [Summary] [Detail] tabs
   → Research tab is active by default (existing behavior)

2. User clicks "Summary" tab
   → Empty state: "Generate a summary of all gems in this project"
   → [Generate Summary] button

3. User clicks "Generate Summary"
   → Generating state: spinner + "Analyzing 12 gems..."
   → Backend: build composite → chunk → summarize → merge → return

4. Summary appears as formatted markdown
   → [Save as Gem] button + [Regenerate] button
   → Chat input: "Ask about the summary..."

5. User asks questions:
   "What were the main cost findings?"
   → LLM answers using composite doc + summary as context

6. User clicks [Save as Gem]
   → Summary saved as gem with source_type "ProjectSummary"
   → Gem appears in project gem list
   → Button changes to "Saved ✓"
   → Chat input remains for further questions
```

### Layout

```
┌──────────┬────────────────────────┬────────────────────────────────┐
│ LeftNav  │   Center Content       │       RightPanel                │
│          │                        │                                 │
│ Projects │  ProjectsContainer     │  [Research] [Summary] [Detail]  │
│          │  ┌──────┬────────────┐ │                                 │
│          │  │ List │ Gems       │ │  ┌────────────────────────────┐ │
│          │  │      │            │ │  │ # Summary: ECS Migration   │ │
│          │  │      │ • gem 1    │ │  │                            │ │
│          │  │      │ • gem 2    │ │  │ ## Feb 15, 2026            │ │
│          │  │      │ • gem 3    │ │  │ **"Understanding ECS..."** │ │
│          │  │      │            │ │  │ - Key point 1              │ │
│          │  │      │            │ │  │ - Key point 2              │ │
│          │  │      │ • Summary: │ │  │                            │ │
│          │  │      │   ECS...   │ │  │ ## Feb 18, 2026            │ │
│          │  │      │   (saved)  │ │  │ ...                        │ │
│          │  │      │            │ │  └────────────────────────────┘ │
│          │  │      │            │ │                                 │
│          │  │      │            │ │  [Save as Gem] [Regenerate]     │
│          │  │      │            │ │                                 │
│          │  │      │            │ │  ┌──────────────────────────┐   │
│          │  │      │            │ │  │ Ask about the summary... │   │
│          │  └──────┴────────────┘ │  └──────────────────────────┘   │
└──────────┴────────────────────────┴────────────────────────────────┘
```

---

## Architecture

### Module Hierarchy

```
src-tauri/src/agents/
├── project_agent.rs       — ProjectResearchAgent (MODIFIED: +generate_summary_checkpoint,
│                             +save_summary_checkpoint, +send_summary_question,
│                             +ProjectSummaryResult)
├── project_chat.rs        — ProjectChatSource (UNCHANGED)
├── chatbot.rs             — Chatbot (UNCHANGED)
├── chatable.rs            — Chatable trait (UNCHANGED)
├── recording_chat.rs      — RecordingChatSource (UNCHANGED)
├── copilot.rs             — CoPilotAgent (UNCHANGED)
└── mod.rs                 — (UNCHANGED)

src-tauri/src/knowledge/
├── local_store.rs         — (MODIFIED: add composite_summary_of_all_gems.md to KNOWN_SUBFILES)
├── assembler.rs           — (UNCHANGED)
├── store.rs               — KnowledgeStore trait (UNCHANGED)
└── mod.rs                 — (UNCHANGED)

src-tauri/src/projects/
├── commands.rs            — (MODIFIED: +generate_project_summary_checkpoint,
│                             +save_project_summary_checkpoint,
│                             +send_summary_question)
└── store.rs               — ProjectStore trait (UNCHANGED)

Frontend:
├── components/
│   ├── ProjectSummaryChat.tsx    — NEW: Summary Chat component
│   ├── ProjectResearchChat.tsx   — (UNCHANGED)
│   └── RightPanel.tsx            — (MODIFIED: add Summary tab)
├── state/types.ts                — (MODIFIED: +ProjectSummaryResult)
└── App.css                       — (MODIFIED: +summary chat styles)
```

### Dependency Graph

```
                      ┌──────────────────┐
                      │     lib.rs       │
                      │  (no changes —   │
                      │   agent already  │
                      │   registered)    │
                      └────────┬─────────┘
                               │ existing: Arc<TokioMutex<ProjectResearchAgent>>
                               ▼
          ┌──────────────────────────────────────────┐
          │        ProjectResearchAgent              │
          │  (existing struct, new methods)           │
          │                                          │
          │  // existing                             │
          │  .suggest_topics(id) → topics            │
          │  .run_research(id, topics) → results     │
          │  .summarize(id) → summary string         │
          │  .start_chat / send / get / end          │
          │                                          │
          │  // NEW                                  │
          │  .generate_summary_checkpoint(id)        │
          │     → ProjectSummaryResult               │
          │  .save_summary_checkpoint(id, summary,   │
          │     composite) → Gem                     │
          │  .send_summary_question(question,        │
          │     summary, composite) → String          │
          └──────────┬──────────────┬────────────────┘
                     │              │
          ┌──────────┘              └──────────────┐
          ▼                                        ▼
┌──────────────────────┐              ┌─────────────────────┐
│   KnowledgeStore     │              │    GemStore          │
│                      │              │                     │
│ .get_assembled(id)   │              │ .save(gem)          │
│   → gem.md content   │              │ .get(id)            │
│                      │              │                     │
│ .create(gem)         │              └─────────────────────┘
│ .update_subfile(     │                        │
│   id, filename,      │              ┌─────────┘
│   content)           │              ▼
└──────────────────────┘    ┌─────────────────────┐
                            │  ProjectStore        │
                            │                     │
                            │ .get(id)            │
                            │   → ProjectDetail   │
                            │ .add_gems(id, ids)  │
                            └─────────────────────┘
```

---

## Backend: Composite Document Builder

### Input Assembly

```rust
/// Build the composite document from all project gems' knowledge files.
///
/// For each gem (sorted by captured_at ASC):
///   1. Try KnowledgeStore::get_assembled(gem_id) → gem.md content
///   2. Fallback: assemble from DB fields (title + description + content + summary + transcript)
///   3. Wrap with separator header containing gem metadata
///
/// Returns the full composite markdown string.

fn build_composite_document(
    gems: &[GemPreview],
    gem_store: &Arc<dyn GemStore>,
    knowledge_store: &Arc<dyn KnowledgeStore>,
) -> Result<String, String>
```

**Composite document format:**

```markdown
# Project: {project_title}
**Objective:** {objective}
**Gems:** {count} | **Date range:** {earliest} — {latest}

---

========================================
GEM 1: "Understanding ECS Task Definitions"
Source: Article | Domain: medium.com | Captured: Feb 15, 2026
========================================

{contents of gem.md}

========================================
GEM 2: "AWS re:Invent 2025 - ECS Best Practices"
Source: Video | Domain: youtube.com | Captured: Feb 15, 2026
========================================

{contents of gem.md}

...
```

---

## Backend: Chunked Summarization

### Chunking Logic

```rust
/// Split the composite document into chunks on gem boundaries.
///
/// Walk through gems in order. Accumulate into current chunk until
/// adding the next gem would exceed `max_chunk_tokens`. Start a new chunk.
///
/// Each chunk contains 1+ complete gems — never split mid-gem.
/// If a single gem exceeds max_chunk_tokens, truncate its content.

fn chunk_by_gem_boundaries(
    composite_doc: &str,
    max_chunk_tokens: usize,  // ~4000
) -> Vec<String>
```

**Token estimation:** Simple heuristic — 1 token ≈ 4 characters. So 4000 tokens ≈ 16,000 characters. No need for a tokenizer.

### Summarization Pipeline

```
Case 1: Everything fits in one chunk
─────────────────────────────────────
  composite_doc → LLM(SUMMARY_PROMPT) → final_summary

Case 2: Multiple chunks needed
──────────────────────────────
  chunk_1 → LLM(CHUNK_SUMMARY_PROMPT) → chunk_summary_1
  chunk_2 → LLM(CHUNK_SUMMARY_PROMPT) → chunk_summary_2
  chunk_3 → LLM(CHUNK_SUMMARY_PROMPT) → chunk_summary_3
  ...
  all_chunk_summaries → LLM(MERGE_PROMPT) → final_summary
```

### LLM Prompts

**SUMMARY_PROMPT** (single chunk / full doc):
```
You are a research analyst. Given a project and its collected resources
(listed chronologically), generate a comprehensive summary covering all key points.

Format:
- Group findings by date
- Under each resource: 3-5 bullet points with the most important highlights
- End with a synthesis of cross-cutting themes

Rules:
- Be specific — cite actual facts, numbers, and insights
- Every resource should have key points
- Use markdown formatting
- Keep each bullet to one concise sentence
```

**CHUNK_SUMMARY_PROMPT** (per chunk):
```
You are summarizing a section of a research project. Extract the key points
and highlights from each resource below. Be specific and preserve important details.

Format: For each resource, list 3-5 key bullet points.
```

**MERGE_PROMPT** (combining chunk summaries):
```
You have summaries from different sections of a research project.
Combine them into one cohesive summary document.

Rules:
- Preserve all key points from each section
- Maintain chronological order by date
- Add a brief synthesis of cross-cutting themes at the end
- Use markdown formatting
```

---

## Backend: New Methods on ProjectResearchAgent

### `generate_summary_checkpoint`

```rust
/// Result of summary generation — returned to frontend for review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummaryResult {
    pub summary: String,
    pub composite_doc: String,
    pub gems_analyzed: usize,
    pub chunks_used: usize,
}

impl ProjectResearchAgent {
    /// Generate a summary checkpoint for review (does NOT save).
    ///
    /// 1. Load project gems sorted by captured_at ASC
    /// 2. Build composite document from gem.md knowledge files
    /// 3. Chunk by gem boundaries
    /// 4. Summarize each chunk via IntelProvider::chat
    /// 5. Merge chunk summaries if multiple chunks
    /// 6. Return summary + composite doc for frontend review
    pub async fn generate_summary_checkpoint(
        &self,
        project_id: &str,
    ) -> Result<ProjectSummaryResult, String>
}
```

**Flow:**
```
1. self.project_store.get(project_id) → ProjectDetail
2. Return error if 0 gems
3. Sort detail.gems by captured_at ASC
4. For each gem:
   a. self.knowledge_store.get_assembled(gem_id) → Option<String>
   b. If None: self.gem_store.get(gem_id) → build fallback from DB fields
   c. Wrap with separator header
5. Assemble composite_doc with project header
6. chunk_by_gem_boundaries(composite_doc, 4000)
7. If 1 chunk: IntelProvider::chat(SUMMARY_PROMPT, chunk) → summary
8. If N chunks:
   a. For each chunk: IntelProvider::chat(CHUNK_SUMMARY_PROMPT, chunk) → chunk_summary
   b. IntelProvider::chat(MERGE_PROMPT, all_chunk_summaries) → summary
9. Return ProjectSummaryResult { summary, composite_doc, gems_analyzed, chunks_used }
```

### `save_summary_checkpoint`

```rust
impl ProjectResearchAgent {
    /// Save a reviewed summary as a gem checkpoint.
    ///
    /// Creates a new gem, adds to project, generates knowledge files,
    /// writes composite_summary_of_all_gems.md, and indexes for search.
    pub async fn save_summary_checkpoint(
        &self,
        project_id: &str,
        summary_content: &str,
        composite_doc: &str,
    ) -> Result<Gem, String>
}
```

**Flow:**
```
1. self.project_store.get(project_id) → get title, gem count
2. Build gem:
   - id: uuid::Uuid::new_v4()
   - source_type: "ProjectSummary"
   - source_url: format!("jarvis://project/{}/summary/{}", project_id, unix_timestamp)
   - title: format!("Summary: {} — {}", project_title, formatted_date)
   - content: Some(summary_content.to_string())
   - domain: "jarvis"
   - description: Some(format!("Summary of {} gems from {}", gem_count, project_title))
   - captured_at: now ISO 8601
   - source_meta: json!({})
   - ai_enrichment: None (can be enriched later via standard enrich flow)
   - transcript: None
   - transcript_language: None
3. self.gem_store.save(&gem) → persisted
4. self.project_store.add_gems(project_id, &[gem.id.clone()])
5. self.knowledge_store.create(&gem) → generates content.md, enrichment.md, gem.md
6. Write composite file:
   self.knowledge_store.update_subfile(
       &gem.id,
       "composite_summary_of_all_gems.md",
       composite_doc
   )
7. self.search_provider.index_gem(&gem.id)
8. Return gem
```

### `send_summary_question`

```rust
impl ProjectResearchAgent {
    /// Answer a question about a generated summary.
    /// Uses the summary + composite doc as context.
    pub async fn send_summary_question(
        &self,
        question: &str,
        summary: &str,
        composite_doc: &str,
    ) -> Result<String, String>
}
```

**Flow:**
```
1. Build context: summary + truncated composite_doc (cap at ~10K chars to fit context window)
2. IntelProvider::chat with system prompt:
   "You are answering questions about a project summary.
    Use the summary and source material below to give specific, grounded answers."
3. Return LLM response
```

**Note:** This is stateless — each question is independent. No chat session needed. The frontend passes the summary and composite doc each time. Simple for v1; can upgrade to session-based chat later.

---

## Backend: Knowledge Store Change

### `KNOWN_SUBFILES` Extension

**File:** `src-tauri/src/knowledge/local_store.rs`

```rust
// Before:
const KNOWN_SUBFILES: &[&str] = &[
    "meta.json",
    "content.md",
    "enrichment.md",
    "transcript.md",
    "copilot.md",
    "gem.md",
];

// After:
const KNOWN_SUBFILES: &[&str] = &[
    "meta.json",
    "content.md",
    "enrichment.md",
    "transcript.md",
    "copilot.md",
    "composite_summary_of_all_gems.md",
    "gem.md",
];
```

**Why this works:** The `get()` method iterates through `KNOWN_SUBFILES` and checks if each file exists on disk. For non-summary gems, `composite_summary_of_all_gems.md` won't exist — it's simply skipped. No branching logic needed.

**Effect:** The file appears in the knowledge viewer's file tree for summary gems. Users can click to view the full composite source document.

### Assembler Change

**File:** `src-tauri/src/knowledge/assembler.rs`

The `assemble_gem_md()` function reads `content.md`, `enrichment.md`, `transcript.md`, `copilot.md` to build `gem.md`. It does NOT need to read `composite_summary_of_all_gems.md` — that file is a standalone reference, not part of the assembled output.

**No changes to the assembler.** The composite file lives alongside `gem.md` but is not included in it.

---

## Backend: New Tauri Commands

**File:** `src-tauri/src/projects/commands.rs` (add to existing)

```rust
/// Generate a summary checkpoint for review. Does not save.
#[tauri::command]
pub async fn generate_project_summary_checkpoint(
    project_id: String,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<ProjectSummaryResult, String> {
    let agent = agent.lock().await;
    agent.generate_summary_checkpoint(&project_id).await
}

/// Save a reviewed summary as a gem. Called after user approves.
#[tauri::command]
pub async fn save_project_summary_checkpoint(
    project_id: String,
    summary_content: String,
    composite_doc: String,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<Gem, String> {
    let agent = agent.lock().await;
    agent.save_summary_checkpoint(&project_id, &summary_content, &composite_doc).await
}

/// Ask a question about a generated summary.
#[tauri::command]
pub async fn send_summary_question(
    question: String,
    summary: String,
    composite_doc: String,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<String, String> {
    let agent = agent.lock().await;
    agent.send_summary_question(&question, &summary, &composite_doc).await
}
```

**Registration in `lib.rs`:** Add to existing `generate_handler![]`:
```rust
projects::commands::generate_project_summary_checkpoint,
projects::commands::save_project_summary_checkpoint,
projects::commands::send_summary_question,
```

**Note:** No changes to `lib.rs` agent setup — `ProjectResearchAgent` is already registered in Tauri state. The new methods use existing provider references (`knowledge_store`, `gem_store`, `project_store`, `search_provider`, `intel_provider`) that need to be added to the agent struct.

### Agent Struct Change

The `ProjectResearchAgent` currently does NOT hold `knowledge_store`. It needs to be added:

```rust
// Before:
pub struct ProjectResearchAgent {
    project_store: Arc<dyn ProjectStore>,
    gem_store: Arc<dyn GemStore>,
    intel_provider: Arc<dyn IntelProvider>,
    search_provider: Arc<dyn SearchResultProvider>,
    intel_queue: Arc<IntelQueue>,
    chatbot: Chatbot,
    chat_sources: HashMap<String, ProjectChatSource>,
}

// After:
pub struct ProjectResearchAgent {
    project_store: Arc<dyn ProjectStore>,
    gem_store: Arc<dyn GemStore>,
    intel_provider: Arc<dyn IntelProvider>,
    search_provider: Arc<dyn SearchResultProvider>,
    knowledge_store: Arc<dyn KnowledgeStore>,    // NEW
    intel_queue: Arc<IntelQueue>,
    chatbot: Chatbot,
    chat_sources: HashMap<String, ProjectChatSource>,
}
```

**`new()` signature changes:** Add `knowledge_store: Arc<dyn KnowledgeStore>` parameter.

**`lib.rs` change:** Pass `knowledge_store_arc.clone()` when constructing `ProjectResearchAgent::new()`.

---

## Frontend: TypeScript Types

**File:** `src/state/types.ts`

```typescript
/** Result of summary generation — returned for review before saving */
export interface ProjectSummaryResult {
  summary: string;
  composite_doc: string;
  gems_analyzed: number;
  chunks_used: number;
}
```

---

## Frontend: `ProjectSummaryChat` Component

**File:** `src/components/ProjectSummaryChat.tsx`

### Props

```typescript
interface ProjectSummaryChatProps {
  projectId: string;
  projectTitle: string;
  onGemSaved?: () => void;
}
```

### State

```typescript
type SummaryState = 'empty' | 'generating' | 'review' | 'saved';

const [state, setState] = useState<SummaryState>('empty');
const [summaryResult, setSummaryResult] = useState<ProjectSummaryResult | null>(null);
const [saved, setSaved] = useState(false);
const [chatMessages, setChatMessages] = useState<{role: string, content: string}[]>([]);
const [input, setInput] = useState('');
const [loading, setLoading] = useState(false);
```

### State Transitions

```
empty ──[Generate]──→ generating ──[result]──→ review ──[Save]──→ saved
                          │                      │                  │
                          └──[error]──→ empty     │                  │
                                                  └──[Regenerate]───┘──→ generating
```

### Render by State

**Empty:**
```tsx
<div className="summary-chat-empty">
  <h3>Project Summary</h3>
  <p>Generate a summary covering all key points from every gem in this project.</p>
  <button onClick={handleGenerate}>Generate Summary</button>
</div>
```

**Generating:**
```tsx
<div className="summary-chat-generating">
  <div className="spinner" />
  <span>Analyzing {gemsCount} gems...</span>
</div>
```

**Review / Saved:**
```tsx
<div className="summary-chat">
  <div className="summary-preview">
    {/* rendered markdown */}
    <pre className="summary-content">{summaryResult.summary}</pre>
    <div className="summary-meta">
      {summaryResult.gems_analyzed} gems analyzed · {summaryResult.chunks_used} chunks
    </div>
  </div>

  <div className="summary-actions">
    {!saved ? (
      <button className="action-button" onClick={handleSave}>Save as Gem</button>
    ) : (
      <span className="summary-saved-badge">Saved</span>
    )}
    <button className="action-button secondary" onClick={handleGenerate}>Regenerate</button>
  </div>

  {/* Q&A chat */}
  <div className="summary-qa">
    {chatMessages.map(...)}
    <div className="chat-input-bar">
      <input placeholder="Ask about the summary..." ... />
      <button onClick={handleAskQuestion}>Send</button>
    </div>
  </div>
</div>
```

### Key Handlers

```typescript
const handleGenerate = async () => {
  setState('generating');
  try {
    const result = await invoke<ProjectSummaryResult>(
      'generate_project_summary_checkpoint', { projectId }
    );
    setSummaryResult(result);
    setState('review');
    setSaved(false);
    setChatMessages([]);
  } catch (err) {
    setState('empty');
    // show error
  }
};

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
    // show error
  }
};

const handleAskQuestion = async () => {
  if (!input.trim() || !summaryResult || loading) return;
  const question = input.trim();
  setInput('');
  setLoading(true);
  setChatMessages(prev => [...prev, { role: 'user', content: question }]);

  try {
    const answer = await invoke<string>('send_summary_question', {
      question,
      summary: summaryResult.summary,
      compositeDoc: summaryResult.composite_doc,
    });
    setChatMessages(prev => [...prev, { role: 'assistant', content: answer }]);
  } catch (err) {
    setChatMessages(prev => [...prev, { role: 'assistant', content: `Error: ${err}` }]);
  } finally {
    setLoading(false);
  }
};
```

---

## Frontend: RightPanel Change

**File:** `src/components/RightPanel.tsx`

Replace the `activeNav === 'projects'` block. When a project is selected, show three tabs instead of two:

```tsx
// Project selected (with or without gem)
<div className="record-tabs-view">
  <div className="tab-buttons">
    <button className={activeTab === 'chat' ? 'active' : ''}>
      Research
    </button>
    <button className={activeTab === 'summary' ? 'active' : ''}>
      Summary
    </button>
    {selectedGemId && (
      <button className={activeTab === 'transcript' ? 'active' : ''}>
        Detail
      </button>
    )}
    {/* existing knowledge file tabs */}
  </div>
  <div className="tab-content">
    {activeTab === 'chat' && (
      <ProjectResearchChat key={selectedProjectId} ... />
    )}
    {activeTab === 'summary' && (
      <ProjectSummaryChat key={selectedProjectId} ... />
    )}
    {activeTab === 'transcript' && (
      <GemDetailPanel ... />
    )}
  </div>
</div>
```

**Tab values:** Reuses existing tab value scheme — `'chat'` for Research, `'summary'` (new) for Summary, `'transcript'` for Detail. The `'summary'` value is new.

**Default tab:** Stays `'chat'` (Research) when a project is first selected.

---

## CSS Additions

**File:** `App.css`

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
  color: var(--text-secondary, #aaa);
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
  color: var(--text-secondary, #aaa);
}

.summary-preview {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

.summary-content {
  font-family: var(--font-mono, 'JetBrains Mono', monospace);
  font-size: 12px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-wrap: break-word;
}

.summary-meta {
  font-size: 11px;
  color: var(--text-muted, #666);
  padding-top: 8px;
  border-top: 1px solid var(--border-color, #333);
  margin-top: 12px;
}

.summary-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border-top: 1px solid var(--border-color, #333);
}

.summary-saved-badge {
  font-size: 12px;
  color: var(--success-color, #22c55e);
  padding: 4px 10px;
}

.summary-qa {
  border-top: 1px solid var(--border-color, #333);
  padding: 8px 16px;
  max-height: 200px;
  overflow-y: auto;
}
```

---

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| 0 gems | `generate_summary_checkpoint` returns error. Frontend shows message in empty state. |
| 1 gem | Single chunk, single LLM call. Works fine. |
| All gems fit in one chunk | Skip chunking + merge. One LLM call total. |
| Single gem exceeds chunk size | Truncate content, preserve title/metadata. Note in output. |
| Gem has no knowledge files and no DB content | Skip gem, note in composite doc. |
| LLM fails on one chunk | Return partial summary with succeeded chunks + error note. |
| LLM completely unavailable | Return error. Frontend shows in empty state. |
| User clicks Regenerate | Re-runs pipeline. Previous summary replaced. Chat messages cleared. |
| User never saves | That's fine. Summary was for review only. |
| User switches projects | Component remounts via `key={projectId}`. All state resets. |
| `composite_doc` too large for save command | Tauri IPC handles large strings fine (it's local, not HTTP). |
| Knowledge store unavailable | Fallback to DB fields for gem content. |
| Summary gem's knowledge files | Generated via standard `create()` path. `composite_summary_of_all_gems.md` written via `update_subfile()` afterward. |

---

## Testing Strategy

### Unit Tests

**`project_agent.rs` tests:**
- `generate_summary_checkpoint` with 0 gems returns error
- `generate_summary_checkpoint` with 1 gem uses single LLM call (no merge)
- `generate_summary_checkpoint` with many gems produces multiple chunks
- `build_composite_document` sorts gems chronologically
- `build_composite_document` falls back to DB fields when no knowledge files
- `chunk_by_gem_boundaries` never splits mid-gem
- `chunk_by_gem_boundaries` handles single gem exceeding chunk size
- `save_summary_checkpoint` creates gem with correct fields
- `save_summary_checkpoint` adds gem to project
- `send_summary_question` returns LLM response

**`local_store.rs` tests:**
- `get()` includes `composite_summary_of_all_gems.md` in subfiles when file exists
- `get()` skips `composite_summary_of_all_gems.md` when file doesn't exist
- `update_subfile` works for `composite_summary_of_all_gems.md`

### Manual Testing Checklist

- [ ] Open project → [Research] [Summary] tabs visible
- [ ] Click Summary tab → empty state with Generate button
- [ ] Click Generate → spinner with gem count
- [ ] Summary appears as formatted markdown
- [ ] Summary meta shows gems analyzed + chunks used
- [ ] Ask a question → answer appears in Q&A section
- [ ] Click Save as Gem → badge shows "Saved", gem appears in project gem list
- [ ] Click saved summary gem → Detail tab shows content
- [ ] Open summary gem's knowledge files → `composite_summary_of_all_gems.md` listed
- [ ] Click `composite_summary_of_all_gems.md` → full concatenated source visible
- [ ] Click Regenerate → new summary replaces old, chat clears
- [ ] Switch projects → Summary tab resets to empty
- [ ] Project with 0 gems → Generate shows friendly error
- [ ] Summary gem searchable via FTS
