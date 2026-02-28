# Kiro Prompt — Searchable Gems Phase 7: Frontend TypeScript Types & Search Integration

## What You're Building

Update the React frontend to use the new `search_gems` command (which returns `GemSearchResult[]` instead of `GemPreview[]`) and show relevance score badges on gem cards for semantic/hybrid search results.

## Spec Files

- **Tasks**: `.kiro/specs/searchable-gems/tasks.md` — Phase 7, Tasks 10–12

## Context: What Already Exists

**Backend (complete):** The `search_gems` Tauri command now returns `Vec<GemSearchResult>` (not `Vec<GemPreview>`). `GemSearchResult` includes `score`, `matched_chunk`, `match_type` plus all gem metadata. The empty-query case is also handled — it returns `GemSearchResult[]` with score `1.0` and `match_type: "Keyword"`.

**Frontend (current):**
- `GemsPanel.tsx` calls `invoke<GemPreview[]>('search_gems', { query })` — type is wrong now
- `GemCard` accepts `GemPreview` as its prop type
- `localGem` state inside GemCard is `GemPreview`
- `filter_gems_by_tag` still returns `GemPreview[]` — this stays unchanged
- `list_gems` returns `GemPreview[]` — but we can now use `search_gems` with empty query instead

**The key type difference:**
- `GemSearchResult` (backend) has: `score`, `matched_chunk`, `match_type`, `id`, `source_type`, `source_url`, `domain`, `title`, `author`, `description`, `captured_at`, `tags`, `summary`
- `GemPreview` (frontend) has: same gem fields PLUS `content_preview`, `enrichment_source`, `transcript_language`
- `GemSearchResult` is MISSING: `content_preview`, `enrichment_source`, `transcript_language`

The GemCard currently uses all three missing fields conditionally. We handle this by making those fields optional on the TypeScript `GemSearchResult` type.

## Exact Changes

### Part A: Add TypeScript Types in `src/state/types.ts`

**Add these types** after the `GemPreview` interface (after line 551):

```typescript
/** Match type for search results */
export type MatchType = 'Keyword' | 'Semantic' | 'Hybrid';

/** Search result combining search metadata with gem metadata.
 *  Returned by the search_gems Tauri command.
 *  Matches Rust GemSearchResult struct. */
export interface GemSearchResult {
  /** Relevance score (0.0 to 1.0) */
  score: number;

  /** Text snippet that matched the query */
  matched_chunk: string;

  /** How this result was matched */
  match_type: MatchType;

  /** Unique identifier (UUID v4) */
  id: string;

  /** Source classification */
  source_type: string;

  /** Original URL */
  source_url: string;

  /** Domain extracted from URL */
  domain: string;

  /** Page/video/article title */
  title: string;

  /** Author/channel name */
  author: string | null;

  /** Short description or summary */
  description: string | null;

  /** ISO 8601 timestamp when gem was captured */
  captured_at: string;

  /** AI-generated topic tags */
  tags: string[] | null;

  /** AI-generated one-sentence summary */
  summary: string | null;

  // ── Fields from GemPreview not in backend GemSearchResult ──
  // Present when converted from GemPreview (filter_gems_by_tag),
  // undefined when from search_gems backend response.

  /** Content truncated to 200 characters */
  content_preview?: string | null;

  /** Source of enrichment, e.g. "mlx / qwen3-8b-4bit" */
  enrichment_source?: string | null;

  /** Language detected during transcription (ISO 639-1 code) */
  transcript_language?: string | null;
}

/** Result of the automated QMD semantic search setup flow */
export interface QmdSetupResult {
  success: boolean;
  node_version: string | null;
  qmd_version: string | null;
  docs_indexed: number | null;
  error: string | null;
}

/** Progress event emitted during semantic search setup */
export interface SetupProgressEvent {
  step: number;
  total: number;
  description: string;
  status: string;
}
```

**Update the exports** — add `GemSearchResult`, `MatchType`, `QmdSetupResult`, `SetupProgressEvent` to whatever export mechanism is used. (The file uses `export interface` directly, so they're auto-exported.)

### Part B: Update `GemsPanel.tsx` — Import and State

**Update the import** on line 4:

Find:
```typescript
import type { GemPreview, Gem, AvailabilityResult } from '../state/types';
```

Replace with:
```typescript
import type { GemPreview, GemSearchResult, Gem, AvailabilityResult } from '../state/types';
```

**Update the `gems` state** (currently around line 391):

Find:
```typescript
const [gems, setGems] = useState<GemPreview[]>([]);
```

Replace with:
```typescript
const [gems, setGems] = useState<GemSearchResult[]>([]);
```

### Part C: Update `fetchGems` Function

The current `fetchGems` (lines 411–432) branches into three cases. Replace the whole function:

Find:
```typescript
  const fetchGems = useCallback(async (query: string, tag: string | null) => {
    setLoading(true);
    setError(null);
    try {
      let results: GemPreview[];
      if (tag) {
        // Filter by tag
        results = await invoke<GemPreview[]>('filter_gems_by_tag', { tag });
      } else if (query.trim()) {
        // Search by query
        results = await invoke<GemPreview[]>('search_gems', { query });
      } else {
        // List all
        results = await invoke<GemPreview[]>('list_gems', {});
      }
      setGems(results);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);
```

Replace with:
```typescript
  const fetchGems = useCallback(async (query: string, tag: string | null) => {
    setLoading(true);
    setError(null);
    try {
      let results: GemSearchResult[];
      if (tag) {
        // Filter by tag — returns GemPreview[], convert to GemSearchResult[]
        const previews = await invoke<GemPreview[]>('filter_gems_by_tag', { tag });
        results = previews.map(gem => ({
          score: 1.0,
          matched_chunk: '',
          match_type: 'Keyword' as const,
          ...gem,
        }));
      } else {
        // search_gems handles both empty query (list all) and keyword/semantic search
        results = await invoke<GemSearchResult[]>('search_gems', { query, limit: 50 });
      }
      setGems(results);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);
```

**Key changes:**
1. State type is `GemSearchResult[]`
2. `search_gems` is called for BOTH empty queries and keyword queries (backend handles both)
3. `list_gems` call removed (search_gems with empty query does the same thing)
4. `filter_gems_by_tag` still returns `GemPreview[]` — we spread the preview fields and add the search metadata defaults (`score: 1.0`, `matched_chunk: ''`, `match_type: 'Keyword'`)
5. Added `limit: 50` to the search_gems call

### Part D: Update GemCard Component

#### D1: Change prop type

Find the GemCard function signature (lines 25–37):
```typescript
function GemCard({
  gem,
  onDelete,
  aiAvailable,
  onFilterByTag,
  onSelect
}: {
  gem: GemPreview;
  onDelete: (id: string) => Promise<void>;
  aiAvailable: boolean;
  onFilterByTag: (tag: string) => void;
  onSelect?: (gemId: string) => void;
}) {
```

Replace with:
```typescript
function GemCard({
  gem,
  onDelete,
  aiAvailable,
  onFilterByTag,
  onSelect
}: {
  gem: GemSearchResult;
  onDelete: (id: string) => Promise<void>;
  aiAvailable: boolean;
  onFilterByTag: (tag: string) => void;
  onSelect?: (gemId: string) => void;
}) {
```

#### D2: Change localGem state type

Find (line ~50):
```typescript
  const [localGem, setLocalGem] = useState<GemPreview>(gem);
```

Replace with:
```typescript
  const [localGem, setLocalGem] = useState<GemSearchResult>(gem);
```

#### D3: Add relevance score badge in card header

Find the card header (lines 190–195):
```typescript
      <div className="gem-card-header">
        <span className={badgeClass}>{gem.source_type}</span>
        <span className="gem-date">
          {new Date(gem.captured_at).toLocaleDateString()}
        </span>
      </div>
```

Replace with:
```typescript
      <div className="gem-card-header">
        <span className={badgeClass}>{gem.source_type}</span>
        {(gem.match_type === 'Semantic' || gem.match_type === 'Hybrid') && (
          <span className="relevance-badge" title={`${gem.match_type} match`}>
            {Math.round(gem.score * 100)}%
          </span>
        )}
        <span className="gem-date">
          {new Date(gem.captured_at).toLocaleDateString()}
        </span>
      </div>
```

**Logic:** Only show the relevance badge when `match_type` is `Semantic` or `Hybrid`. For `Keyword` results (FTS5), no badge is shown — preserving current behavior.

### Part E: Add CSS for Relevance Badge

**File:** `src/App.css`

Find the `.gem-card-header` block (should be around line 1819–1825):
```css
.gem-card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
```

**Add this new rule** right after the `.gem-card-header` block:
```css
.relevance-badge {
  display: inline-block;
  background-color: #e3f2fd;
  color: #1565c0;
  font-size: 11px;
  font-weight: 600;
  padding: 2px 6px;
  border-radius: 3px;
  margin-left: auto;
  margin-right: 8px;
}
```

**Design notes:**
- Blue tint background (`#e3f2fd`) with blue text (`#1565c0`) — matches the existing gem-domain color scheme
- `margin-left: auto` pushes it to the right in the flex header, before the date
- `margin-right: 8px` creates spacing between the badge and the date
- Small font (`11px`) consistent with other metadata badges (`.gem-lang-badge`)

### Part F: Update the GemCard Rendering Call

In the gems list rendering section (search for where `GemCard` is rendered), the component should already receive `gem` from the `gems` array. Since we changed the array type to `GemSearchResult[]`, the prop type now matches. No change needed here — just verify the rendering loop passes gems correctly.

Look for something like:
```typescript
{gems.map(gem => (
  <GemCard
    key={gem.id}
    gem={gem}
    ...
```

This should work as-is since `gems` is now `GemSearchResult[]` and GemCard accepts `GemSearchResult`.

## Gotchas

1. **`content_preview`, `enrichment_source`, `transcript_language` are optional** — The TypeScript `GemSearchResult` type has these as `?: string | null`. When search results come from the backend, these will be `undefined`. When converted from `GemPreview` (tag filter), they'll be present. The GemCard already uses conditional rendering (`localGem.transcript_language &&`, `localGem.enrichment_source &&`, `gem.content_preview &&`) so this works safely — `undefined` is falsy.

2. **`localGem` spread in `handleEnrich`/`handleTranscribe`** — These callbacks update `localGem` by spreading the current state and adding new fields from the Gem response (lines 66–71, 92–97). This pattern still works because spread + assignment creates a valid `GemSearchResult` object. The search metadata fields (`score`, `matched_chunk`, `match_type`) are preserved from the original prop via the spread.

3. **`as const` on `match_type`** — When creating the default `GemSearchResult` from `GemPreview`, use `'Keyword' as const` so TypeScript narrows the type correctly.

4. **Don't remove `GemPreview` type** — It's still used by `filter_gems_by_tag` return type and by the full `Gem` type operations. Keep both types in `types.ts`.

5. **`list_gems` is no longer called** from `fetchGems`, but keep the command registered — other components might use it. Don't remove `commands::list_gems` from `lib.rs` or `commands.rs`.

6. **The `Gem` import stays** — `GemCard` still calls `invoke<Gem>('enrich_gem', ...)` and `invoke<Gem>('transcribe_gem', ...)` which return the full `Gem` type. This import is unchanged.

7. **Score badge position** — The `gem-card-header` uses `display: flex; justify-content: space-between`. The three children are: source badge, relevance badge (optional), date. With `margin-left: auto` on the relevance badge, it pushes toward the right. When absent (keyword results), the header looks identical to before.

8. **No changes to `handleDelete`** — The delete handler calls `setGems(prev => prev.filter(g => g.id !== id))` which works for any array with `.id` property.

## Verification

1. Run `npm run build` (or `pnpm build`) from `jarvis-app/` — must pass with no TypeScript errors
2. Verify the search still works (type `invoke<GemSearchResult[]>` matches backend response)
3. Verify tag filter still works (GemPreview → GemSearchResult conversion)
4. Verify no badge shows for keyword results (current FTS5 behavior preserved)

**Expected outcome:**
- 1 file modified: `src/state/types.ts` (new types: `MatchType`, `GemSearchResult`, `QmdSetupResult`, `SetupProgressEvent`)
- 1 file modified: `src/components/GemsPanel.tsx` (import, state type, fetchGems, GemCard prop type, score badge)
- 1 file modified: `src/App.css` (`.relevance-badge` style)
- Frontend builds with no errors
- Search behavior identical to before (FTS5 active, no score badges shown)

## Summary of All Changes

| File | Change |
|------|--------|
| `src/state/types.ts` | Add `MatchType`, `GemSearchResult`, `QmdSetupResult`, `SetupProgressEvent` |
| `src/components/GemsPanel.tsx` | Import `GemSearchResult` |
| `src/components/GemsPanel.tsx` | Change `gems` state type to `GemSearchResult[]` |
| `src/components/GemsPanel.tsx` | Rewrite `fetchGems` — use `search_gems` for all queries, convert tag filter results |
| `src/components/GemsPanel.tsx` | GemCard prop: `GemPreview` → `GemSearchResult` |
| `src/components/GemsPanel.tsx` | `localGem` state: `GemPreview` → `GemSearchResult` |
| `src/components/GemsPanel.tsx` | Add relevance badge in `.gem-card-header` |
| `src/App.css` | Add `.relevance-badge` style |

## When Done

Stop and ask for review. Show me:
1. The new types added to `types.ts`
2. The updated `fetchGems` function
3. The updated GemCard header with relevance badge
4. The new CSS rule
5. Build output (no TypeScript errors)
6. Any questions or decisions you made

Do NOT proceed to Phase 8 until I review and approve.
