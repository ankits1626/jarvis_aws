# Phase 7: Checkpoint Persistence — Load Latest on Mount

## What You're Building

You are implementing Phase 7 of the Project Summary Checkpoints feature. This phase makes saved summaries persistent — when a user opens the Summary tab, it loads the latest saved checkpoint instead of showing an empty state.

**Three changes:**
1. **Backend method** `get_latest_summary_checkpoint` on `ProjectResearchAgent`
2. **Tauri command** `get_latest_project_summary_checkpoint` + registration
3. **Frontend update** to `ProjectSummaryChat` — load checkpoint on mount

## Context

Read these files before starting:

- **Tasks:** `.kiro/specs/project-summary-checkpoints/tasks.md` — Phase 7 has Tasks 18-20
- **The agent:** `jarvis-app/src-tauri/src/agents/project_agent.rs` — add method to existing `impl` block
- **Tauri commands:** `jarvis-app/src-tauri/src/projects/commands.rs` — add one command
- **Frontend component:** `jarvis-app/src/components/ProjectSummaryChat.tsx` — update mount behavior

## What Already Exists

When a summary is saved (Phase 3), it creates a gem with:
- `source_type: "ProjectSummary"`
- `content: Some(summary_text)` — the generated summary
- Knowledge file `composite_summary_of_all_gems.md` — the full composite source document

The project's gem list (`ProjectDetail.gems`) returns `Vec<GemPreview>` which includes `source_type` and `captured_at` — enough to find the latest summary gem.

Key methods available:
```rust
// Load project with gem previews (has source_type, captured_at, id)
self.project_store.get(project_id) -> Result<ProjectDetail, String>

// Load full gem (has content field with the summary text)
self.gem_store.get(gem_id) -> Result<Option<Gem>, String>

// Read the composite doc from knowledge files
self.knowledge_store.get_subfile(gem_id, "composite_summary_of_all_gems.md") -> Result<Option<String>, String>
```

## Tasks

### Task 18: Backend — `get_latest_summary_checkpoint`

Add a public async method to `ProjectResearchAgent`:

```rust
pub async fn get_latest_summary_checkpoint(
    &self,
    project_id: &str,
) -> Result<Option<ProjectSummaryResult>, String>
```

**Implementation:**

```rust
eprintln!("Projects/Summary: Loading latest checkpoint for project {}", project_id);

// 1. Load project
let project_detail = self.project_store.get(project_id).await
    .map_err(|e| format!("Failed to load project: {}", e))?;

// 2. Find the latest summary gem
let mut summary_gems: Vec<_> = project_detail.gems.iter()
    .filter(|g| g.source_type == "ProjectSummary")
    .collect();

if summary_gems.is_empty() {
    eprintln!("Projects/Summary: No checkpoints found for project {}", project_id);
    return Ok(None);
}

// Sort by captured_at descending (latest first) — ISO 8601 strings sort lexicographically
summary_gems.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));
let latest = &summary_gems[0];

// 3. Load full gem to get the summary content
let gem = self.gem_store.get(&latest.id).await
    .map_err(|e| format!("Failed to load summary gem: {}", e))?
    .ok_or_else(|| format!("Summary gem {} not found", latest.id))?;

let summary = gem.content.unwrap_or_default();

// 4. Load composite doc from knowledge files
let composite_doc = self.knowledge_store
    .get_subfile(&gem.id, "composite_summary_of_all_gems.md").await
    .unwrap_or(None)
    .unwrap_or_default();

// 5. Estimate gems_analyzed from composite doc (count separator pairs)
let separator_count = composite_doc.matches("========================================").count();
let gems_analyzed = separator_count / 2; // Each gem has 2 separator lines

eprintln!(
    "Projects/Summary: Loaded checkpoint for project {} ({} chars, {} gems)",
    project_id, summary.len(), gems_analyzed
);

Ok(Some(ProjectSummaryResult {
    summary,
    composite_doc,
    gems_analyzed,
    chunks_used: 0, // Not tracked for loaded checkpoints
}))
```

### Task 19: Tauri Command + Registration

**In `projects/commands.rs`**, add after the existing summary commands:

```rust
#[tauri::command]
pub async fn get_latest_project_summary_checkpoint(
    project_id: String,
    agent: State<'_, Arc<TokioMutex<ProjectResearchAgent>>>,
) -> Result<Option<ProjectSummaryResult>, String> {
    let agent = agent.lock().await;
    agent.get_latest_summary_checkpoint(&project_id).await
}
```

**In `lib.rs`**, add to `generate_handler![]`:
```rust
projects::commands::get_latest_project_summary_checkpoint,
```

### Task 20: Frontend — Load Checkpoint on Mount

**In `ProjectSummaryChat.tsx`**, update the component to load the latest checkpoint when mounted.

**Step 1: Add `initializing` state**

Add alongside existing state hooks:
```typescript
const [initializing, setInitializing] = useState(true);
```

**Step 2: Add mount effect**

Replace the existing `projectId` reset effect with one that loads the checkpoint:

```typescript
// Load latest checkpoint on mount (or when project changes)
useEffect(() => {
  let cancelled = false;
  const loadCheckpoint = async () => {
    setInitializing(true);
    try {
      const result = await invoke<ProjectSummaryResult | null>(
        'get_latest_project_summary_checkpoint',
        { projectId }
      );
      if (cancelled) return;

      if (result) {
        setSummaryResult(result);
        setState('saved');
        setSaved(true);
      } else {
        setState('empty');
      }
    } catch (err) {
      if (cancelled) return;
      setState('empty');
    } finally {
      if (!cancelled) {
        setInitializing(false);
      }
    }
    // Reset Q&A state
    setChatMessages([]);
    setInput('');
    setError(null);
  };
  loadCheckpoint();
  return () => { cancelled = true; };
}, [projectId]);
```

**Important:** This replaces the current simple reset `useEffect`. The `cancelled` flag prevents state updates if the project changes during the async fetch (same pattern as `ProjectResearchChat` line 48).

**Step 3: Add loading state to render**

At the top of the render, before the empty state check:

```typescript
if (initializing) {
  return (
    <div className="summary-chat-generating">
      <div className="spinner" />
      <span>Loading...</span>
    </div>
  );
}
```

**Step 4: Update meta line for loaded checkpoints**

In the review/saved state render, update the meta line to handle `chunks_used === 0`:

```typescript
<div className="summary-meta">
  {summaryResult?.gems_analyzed} gems analyzed
  {summaryResult?.chunks_used > 0 && ` · ${summaryResult.chunks_used} chunks`}
</div>
```

This hides the "chunks" info for loaded checkpoints since we don't track that.

## Checkpoint

Run `cargo check` and `npm run build`. Both should pass.

**What to verify:**
- Backend: `get_latest_summary_checkpoint` returns `None` for projects with no saved summaries
- Backend: Returns the latest `ProjectSummaryResult` for projects with saved summaries
- Backend: `composite_doc` loaded from knowledge file, not just the gem's content field
- Tauri: Command registered and callable
- Frontend: Opening Summary tab shows loading spinner briefly, then either loaded checkpoint or empty state
- Frontend: After generating + saving, switching away and back shows the saved checkpoint
- Frontend: Q&A works on loaded checkpoints (composite_doc is available for context)
- Frontend: Regenerate still works from the loaded checkpoint state
- Frontend: `chunks_used` hidden when 0

## If You Have Questions

- **Ask me before guessing.** Especially about the `GemPreview` filtering — `source_type` is a field on `GemPreview`, so you can filter without loading full gems.
- **The `cancelled` pattern is critical.** Without it, rapid project switches could cause stale state updates. Follow the exact pattern from `ProjectResearchChat`.
- **`get_subfile` returns `Result<Option<String>>`** — handle both the error case and the None case (file might not exist for old checkpoints).
