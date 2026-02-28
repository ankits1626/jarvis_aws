# Project Creation Research Assistant — Smart Onboarding When a Project is Created

## Problem

When a user creates a project (e.g., "ECS Migration to Fargate"), they start with an empty container. They have to manually find and add relevant gems, and there's no help discovering external resources. The project is inert until the user does all the work.

## Proposed Solution

When a user creates a project, Jarvis kicks off two parallel intelligence pipelines that prime the project with useful starting material:

1. **Web Research Pipeline** — LLM suggests search topics derived from the project title/description/objective, then the existing `SearchResultProvider` interface (extended with a `web_search` method) fetches relevant papers, Medium articles, and YouTube videos.
2. **Gem Suggestion Pipeline** — Semantic search over existing gems (via the same `SearchResultProvider::search`) finds ones that are likely relevant to the new project.

The user sees a spinner/loading state while both pipelines run, then gets a curated "starter pack" of external resources and internal gems to review.

---

## Architecture

### Overview

```
User creates project (title, description, objective)
         |
         +------------------------------+
         |                              |
    [Web Research Pipeline]     [Gem Suggestion Pipeline]
         |                              |
    +----+-----+                   +----+----+
    | LLM Call |                   | .search |
    | (topics) |                   |  (gems) |
    +----+-----+                   +----+----+
         |                              |
    +----+----------+              Vec<GemSearchResult>
    | .web_search   |              (ranked by relevance)
    | (per topic)   |
    +----+----------+
         |
    Vec<WebSearchResult>
    (papers, articles, videos)
         |
         +------------------------------+
                        |
              Frontend receives both
              User reviews & acts on results
```

### Key Design Decision: Extend `SearchResultProvider`, Don't Create a New Trait

The existing `SearchResultProvider` trait is the codebase's single interface for search. Rather than introducing a parallel `WebSearchProvider` trait (which fragments the search contract), we extend `SearchResultProvider` with a `web_search` method that has a **default no-op** implementation.

**Why this works:**
- FTS and QMD providers don't need to change — they inherit the default `web_search` (returns empty)
- A new web-capable provider (e.g., `TavilyProvider`) overrides `web_search` while leaving `search`/`index_gem`/etc. as no-ops
- The Tauri command decides which method to call based on what it needs — same trait, same `Arc<dyn SearchResultProvider>` state
- Follows the existing pattern: `check_availability` already exists for capability detection

**What about the composite case?** If we want *both* gem search (QMD) and web search (Tavily) from the same registered provider, we can either:
- (A) Register two separate `Arc<dyn SearchResultProvider>` instances (one for gems, one for web) — but Tauri state doesn't support duplicate types
- (B) Create a `CompositeSearchProvider` that delegates `search` to QMD/FTS and `web_search` to Tavily — **this is the recommended approach**

---

## Trait Extension

### Changes to `src/search/provider.rs`

Add `WebSearchResult`, `WebSourceType` types and a default `web_search` method:

```rust
// New types — added to provider.rs alongside SearchResult

/// A search result from the web (not a gem).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub source_type: WebSourceType,
    pub domain: String,              // e.g., "arxiv.org", "medium.com", "youtube.com"
    pub published_date: Option<String>,
}

/// Classification of a web search result by content type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebSourceType {
    Paper,       // arxiv, scholar, semantic scholar
    Article,     // medium, dev.to, blog posts
    Video,       // youtube, vimeo
    Other,
}
```

```rust
// Extended SearchResultProvider trait

#[async_trait]
pub trait SearchResultProvider: Send + Sync {
    // --- Existing methods (unchanged) ---

    async fn check_availability(&self) -> AvailabilityResult;

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, String>;

    async fn index_gem(&self, gem_id: &str) -> Result<(), String>;

    async fn remove_gem(&self, gem_id: &str) -> Result<(), String>;

    async fn reindex_all(&self) -> Result<usize, String>;

    // --- New method with default implementation ---

    /// Search the web for external resources (papers, articles, videos).
    ///
    /// Default: returns empty vec (provider does not support web search).
    /// Override in providers that have web search capability (e.g., Tavily).
    async fn web_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<WebSearchResult>, String> {
        let _ = (query, limit); // suppress unused warnings
        Ok(Vec::new())
    }

    /// Check if this provider supports web search.
    ///
    /// Default: false. Override in web-capable providers.
    fn supports_web_search(&self) -> bool {
        false
    }
}
```

### Impact on Existing Providers

**FtsResultProvider** — No changes needed. Inherits default `web_search` (returns empty) and `supports_web_search` (returns false).

**QmdResultProvider** — No changes needed. Same as above.

### New: `TavilyProvider`

A new provider that implements `web_search` but returns no-ops for gem methods:

```rust
// src/search/tavily_provider.rs

pub struct TavilyProvider {
    api_key: String,
    client: reqwest::Client,
}

#[async_trait]
impl SearchResultProvider for TavilyProvider {
    async fn check_availability(&self) -> AvailabilityResult {
        // Check API key is set and endpoint is reachable
        AvailabilityResult { available: !self.api_key.is_empty(), reason: None }
    }

    // Gem search — not applicable for Tavily
    async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<SearchResult>, String> {
        Ok(Vec::new())
    }
    async fn index_gem(&self, _gem_id: &str) -> Result<(), String> { Ok(()) }
    async fn remove_gem(&self, _gem_id: &str) -> Result<(), String> { Ok(()) }
    async fn reindex_all(&self) -> Result<usize, String> { Ok(0) }

    // Web search — the real implementation
    async fn web_search(&self, query: &str, limit: usize) -> Result<Vec<WebSearchResult>, String> {
        eprintln!("Search/Tavily: web_search query=\"{}\" limit={}", query, limit);
        // POST https://api.tavily.com/search
        // { "query": "...", "max_results": 5, "search_depth": "basic" }
        // Parse response into Vec<WebSearchResult>
        // Classify source_type by domain (youtube.com -> Video, arxiv.org -> Paper, etc.)
        todo!("implement Tavily API call")
    }

    fn supports_web_search(&self) -> bool { true }
}
```

### New: `CompositeSearchProvider`

Wraps two providers — one for gem search (QMD/FTS), one for web search (Tavily):

```rust
// src/search/composite_provider.rs

pub struct CompositeSearchProvider {
    gem_provider: Arc<dyn SearchResultProvider>,    // QMD or FTS
    web_provider: Option<Arc<dyn SearchResultProvider>>,  // Tavily (optional)
}

#[async_trait]
impl SearchResultProvider for CompositeSearchProvider {
    async fn check_availability(&self) -> AvailabilityResult {
        self.gem_provider.check_availability().await
    }

    // Delegate gem methods to gem_provider
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
        self.gem_provider.search(query, limit).await
    }
    async fn index_gem(&self, gem_id: &str) -> Result<(), String> {
        self.gem_provider.index_gem(gem_id).await
    }
    async fn remove_gem(&self, gem_id: &str) -> Result<(), String> {
        self.gem_provider.remove_gem(gem_id).await
    }
    async fn reindex_all(&self) -> Result<usize, String> {
        self.gem_provider.reindex_all().await
    }

    // Delegate web search to web_provider (if configured)
    async fn web_search(&self, query: &str, limit: usize) -> Result<Vec<WebSearchResult>, String> {
        match &self.web_provider {
            Some(wp) => wp.web_search(query, limit).await,
            None => Ok(Vec::new()),
        }
    }

    fn supports_web_search(&self) -> bool {
        self.web_provider.as_ref().map_or(false, |wp| wp.supports_web_search())
    }
}
```

### Registration in `lib.rs`

The Tavily API key is **already in settings** — `SearchSettings.tavily_api_key: Option<String>` with a password input field in the Settings UI. No new settings work needed.

```rust
// Build gem search provider (existing logic, unchanged)
let gem_provider: Arc<dyn SearchResultProvider> = if use_qmd {
    Arc::new(QmdResultProvider::new(qmd_path, knowledge_path, accuracy))
} else {
    Arc::new(FtsResultProvider::new(gem_store_arc.clone()))
};

// Build web search provider from existing settings
let settings = settings_manager.read().expect("settings lock").get();
let web_provider: Option<Arc<dyn SearchResultProvider>> = settings
    .search
    .tavily_api_key
    .as_ref()
    .filter(|k| !k.is_empty())
    .map(|api_key| {
        Arc::new(TavilyProvider::new(api_key.clone())) as Arc<dyn SearchResultProvider>
    });

// Wrap in composite — single Arc<dyn SearchResultProvider> for all search needs
let search_provider = Arc::new(CompositeSearchProvider::new(gem_provider, web_provider));
app.manage(search_provider as Arc<dyn SearchResultProvider>);
```

**Result**: One `Arc<dyn SearchResultProvider>` in Tauri state. All existing code (`search_gems`, `check_search_availability`, etc.) works unchanged. New code calls `.web_search()` on the same provider.

**Note on settings change**: If the user adds/changes the Tavily API key in Settings after app start, the `CompositeSearchProvider` won't pick it up until restart. To support hot-reload, `CompositeSearchProvider` could hold `Arc<RwLock<SettingsManager>>` and check the key lazily on each `web_search()` call — but that's a refinement for later.

---

## Pipeline 1: Web Research

**Step 1 — LLM Topic Generation**

Input: project title + description + objective (concatenated as context).

Prompt the existing `IntelProvider` (IntelligenceKit or MLX) with something like:

```
Given a research project titled "{title}" with objective "{objective}",
suggest 3-5 specific search queries that would find useful resources
(academic papers, technical articles, YouTube tutorials).

Return as JSON array of strings. Be specific, not generic.
```

Output: `Vec<String>` — e.g., `["ECS to Fargate migration guide", "AWS Fargate networking best practices", "ECS task definition Fargate differences"]`

**Step 2 — Web Search Execution**

For each topic, call `search_provider.web_search(topic, 5)`. Aggregate, deduplicate by URL, and return.

**Step 3 — Result Presentation**

Results are returned to the frontend as `Vec<WebSearchResult>` with enough metadata to render cards (title, URL, snippet, source type). User can open links in their browser.

## Pipeline 2: Gem Suggestions

Use the same `search_provider.search()` (QMD semantic search or FTS5 keyword fallback) to search for gems relevant to the project.

**Query construction**: Use the project title as the primary query. If description/objective exist, run additional searches and merge results (deduplicate by gem_id, keep highest score).

**Output**: `Vec<GemSearchResult>` ranked by relevance score. The frontend shows these as "Suggested gems from your library" with a one-click "Add to Project" action.

---

## Tauri Command: `get_project_research`

```rust
// src/projects/commands.rs (new command)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectResearchResults {
    pub web_results: Vec<WebSearchResult>,
    pub suggested_gems: Vec<GemSearchResult>,
    pub topics_generated: Vec<String>,
}

#[tauri::command]
pub async fn get_project_research(
    project_id: String,
    project_store: State<'_, Arc<dyn ProjectStore>>,
    intel_provider: State<'_, Arc<dyn IntelProvider>>,
    search_provider: State<'_, Arc<dyn SearchResultProvider>>,
    gem_store: State<'_, Arc<dyn GemStore>>,
) -> Result<ProjectResearchResults, String> {
    // 1. Load project metadata
    let detail = project_store.get(&project_id).await?;
    let project = &detail.project;

    // 2. Build context string for LLM
    let context = build_research_context(project);
    eprintln!("Projects/Research: generating topics for '{}'", project.title);

    // 3. LLM generates search topics
    let topics_raw = intel_provider.chat(&[
        ("system".into(), TOPIC_GENERATION_PROMPT.into()),
        ("user".into(), context),
    ]).await?;
    let topics: Vec<String> = serde_json::from_str(&topics_raw)
        .map_err(|e| format!("Failed to parse LLM topics: {}", e))?;
    eprintln!("Projects/Research: {} topics generated", topics.len());

    // 4. Web search for each topic via the SAME search_provider
    let mut web_results = Vec::new();
    if search_provider.supports_web_search() {
        for topic in &topics {
            eprintln!("Projects/Research: web_search for '{}'", topic);
            match search_provider.web_search(topic, 5).await {
                Ok(results) => web_results.extend(results),
                Err(e) => eprintln!("Projects/Research: web_search failed for '{}': {}", topic, e),
            }
        }
        // Deduplicate by URL
        web_results.sort_by(|a, b| a.url.cmp(&b.url));
        web_results.dedup_by(|a, b| a.url == b.url);
        eprintln!("Projects/Research: {} web results after dedup", web_results.len());
    } else {
        eprintln!("Projects/Research: web search not available, skipping");
    }

    // 5. Gem search via the SAME search_provider.search()
    let gem_results = search_provider.search(&project.title, 20).await?;
    let suggested_gems = enrich_search_results(gem_results, &gem_store).await?;
    eprintln!("Projects/Research: {} gems suggested", suggested_gems.len());

    Ok(ProjectResearchResults {
        web_results,
        suggested_gems,
        topics_generated: topics,
    })
}
```

**Note**: Both `web_search()` and `search()` are called on the *same* `search_provider` — the `CompositeSearchProvider` routes them to the appropriate backend internally.

---

## Frontend Flow

### On Project Creation

```
1. User fills form -> clicks "Create Project"
2. invoke('create_project', { title, description, objective })
   -> Project created, auto-selected
3. ProjectGemList detects newly created project (empty gems list)
4. Automatically calls invoke('get_project_research', { projectId })
5. UI shows spinner overlay: "Researching your project..."
   - Subtext: "Finding relevant articles, papers, videos, and gems..."
6. Results arrive -> spinner dismissed
7. UI renders two sections:
   a. "Suggested from the web" - WebSearchResult cards
      - Each card: title, snippet, domain badge, source type icon
      - Click -> opens URL in system browser (shell.open)
   b. "From your gem library" - GemSearchResult cards
      - Each card: standard GemCard with "Add to Project" button
      - Click "Add" -> invoke('add_gems_to_project') -> card shows "Added" state
```

### UI States

| State | What the user sees |
|-------|-------------------|
| **Loading** | Spinner overlay on ProjectGemList: "Researching your project..." |
| **Results ready** | Two collapsible sections: web results + gem suggestions |
| **No web results** | "No web resources found. Try refining your project description." |
| **No gem suggestions** | "No matching gems in your library yet." |
| **Web not configured** | Gem suggestions still show. Subtle note: "Web search not configured." |
| **Error** | Toast/inline error: "Research failed: {error}. You can still add gems manually." |

### Component: `ProjectResearchPanel`

A new component rendered inside `ProjectGemList` when:
- The project was just created (0 gems), OR
- The user clicks a "Research" button in the project toolbar

```tsx
function ProjectResearchPanel({ projectId, onGemsAdded }: {
  projectId: string;
  onGemsAdded: () => void;
}) {
  const [loading, setLoading] = useState(true);
  const [results, setResults] = useState<ProjectResearchResults | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<ProjectResearchResults>('get_project_research', { projectId })
      .then(setResults)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [projectId]);

  if (loading) {
    return (
      <div className="research-loading">
        <Spinner />
        <p>Researching your project...</p>
        <p className="research-subtext">
          Finding relevant articles, papers, videos, and gems...
        </p>
      </div>
    );
  }

  // ... render web results + gem suggestions
}
```

---

## Logging

All operations log via `eprintln!` (captured by the existing file logging system):

```
[14:32:01.234] Projects/Research: generating topics for 'ECS Migration to Fargate'
[14:32:03.891] Projects/Research: 4 topics generated
[14:32:03.892] Projects/Research: web_search for 'ECS to Fargate migration guide AWS'
[14:32:04.567] Search/Tavily: web_search query="ECS to Fargate migration guide AWS" limit=5
[14:32:05.123] Projects/Research: web_search for 'Fargate networking VPC configuration'
[14:32:05.789] Projects/Research: web_search for 'AWS Fargate cost optimization strategies'
[14:32:06.012] Projects/Research: 15 web results after dedup
[14:32:06.234] Projects/Research: 8 gems suggested
[14:32:06.235] Projects/Research: completed for project abc-123
```

Prefix: `Projects/Research:` for the orchestrator, `Search/Tavily:` for the provider — consistent with existing patterns (`Search:`, `Search/QMD:`, `Intelligence:`).

---

## Potential Web Search Implementations

Any of these can implement `SearchResultProvider::web_search`:

| Provider | Pros | Cons |
|----------|------|------|
| **Tavily** | Built for AI agents, clean snippets, good relevance | Paid API, requires key |
| **Brave Search API** | Free tier (2000/mo), good privacy | Less AI-optimized results |
| **SerpAPI** | Google results, structured data | Paid, heavier |
| **Searx (self-hosted)** | Free, private, no API key | Requires running a server |
| **DuckDuckGo Instant** | Free, no key | Limited structured results |

**Recommendation**: Start with **Tavily** — purpose-built for AI agent workflows, returns clean structured data. Swapping to another is trivial since it's just a different `SearchResultProvider` implementation.

---

## Implementation Phases

### Phase A: Extend `SearchResultProvider` + TavilyProvider
- Add `WebSearchResult`, `WebSourceType` to `src/search/provider.rs`
- Add `web_search` + `supports_web_search` default methods to trait
- Create `TavilyProvider` in `src/search/tavily_provider.rs`
- Create `CompositeSearchProvider` in `src/search/composite_provider.rs`
- Update `lib.rs` registration to use `CompositeSearchProvider` (reads existing `settings.search.tavily_api_key`)
- Re-export new types from `src/search/mod.rs`

### Phase B: Research Command + LLM Topic Generation
- Add `get_project_research` command to `src/projects/commands.rs`
- Define `ProjectResearchResults` struct
- Implement topic generation prompt via `IntelProvider::chat`
- Wire both pipelines through `search_provider.web_search()` and `search_provider.search()`

### Phase C: Frontend — Research Panel
- Create `ProjectResearchPanel` component
- Add spinner/loading state
- Render web results as cards with "Open in browser" action
- Render gem suggestions with "Add to Project" action
- Trigger on project creation (auto) + toolbar button (manual)

### Phase D: Fallback + Polish
- Handle unavailable/unconfigured provider gracefully (gem suggestions still work)
- Add "Research" button to project toolbar for re-running later
- Settings UI already has Tavily API key field — no changes needed

---

## Open Questions

1. **Topic count**: How many search topics should the LLM generate? 3-5 seems right to balance coverage vs. API cost. Make it configurable?
2. **Result caching**: Should we cache research results per project? Or always re-run? Caching saves API calls but results go stale.
3. **Auto-trigger**: Should research run automatically on every project creation, or only when the user clicks a button? Auto feels magical but costs API calls even for throwaway projects.
4. **Parallel execution**: Should the web search calls for each topic run in parallel (faster, more API load) or sequentially (slower, gentler on rate limits)?
5. **Web result persistence**: Should web results be saved to the database, or are they ephemeral/re-fetched? Saving enables "research history" but adds schema complexity.
6. **Source type filtering**: Should the LLM be told to generate topic-specific queries (one for papers, one for articles, one for videos), or should we search broadly and classify results by domain?

---

## Dependencies

- **Existing**: `IntelProvider` (for LLM calls), `SearchResultProvider` (for gem search + web search), `GemStore` (for enriching results)
- **New files**: `tavily_provider.rs`, `composite_provider.rs` in `src/search/`
- **New crate**: `reqwest` (for HTTP calls to Tavily API) — check if already in `Cargo.toml`
- **API key**: Already in `SearchSettings.tavily_api_key` with Settings UI password field — no new settings work
- **No new traits**: Everything goes through `SearchResultProvider`

## Relationship to Existing Specs

- **Extends**: Projects feature (`.kiro/specs/projects/`) — adds intelligence layer on top of CRUD
- **Extends**: Search module (`src/search/provider.rs`) — adds `web_search` capability to existing trait
- **Uses**: Intelligence module (`src/intelligence/`) — leverages `IntelProvider::chat` for topic generation
- **Implements**: requirements.md Out of Scope items 2 ("Smart gem recommendations") and 3 ("Web research suggestions")
