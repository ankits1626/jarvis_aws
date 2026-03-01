# Phase 2 Implementation Prompt — Provider Registration in lib.rs

## What You're Building

Implement Phase 2 from `.kiro/specs/project-research-assistant/tasks.md` — **Task 4**. This phase wires the `CompositeSearchProvider` (built in Phase 1) into the app startup in `lib.rs`, reading the Tavily API key from existing settings and wrapping the gem search provider with an optional web search provider.

**Read these files before writing any code:**
- `.kiro/specs/project-research-assistant/requirements.md` — Requirement 4 (acceptance criteria)
- `.kiro/specs/project-research-assistant/design.md` — "Provider & Agent Registration" section has the exact `lib.rs` code
- `jarvis-app/src-tauri/src/lib.rs` — The file you'll modify (search provider setup starts at line 291)
- `jarvis-app/src-tauri/src/settings/manager.rs` — Confirm `SearchSettings.tavily_api_key: Option<String>` at line 74

## Context: What Phase 1 Built

Phase 1 created these modules (all compiling, all re-exported from `search/mod.rs`):
- `search::WebSearchResult`, `search::WebSourceType` — types on the trait
- `search::TavilyProvider` — web search via Tavily API
- `search::CompositeSearchProvider` — wraps gem + web providers

## Task 4: Register CompositeSearchProvider in `lib.rs`

**File to modify:** `jarvis-app/src-tauri/src/lib.rs`

### Step 4.1: Update imports (line 26)

The current import line is:
```rust
use search::{FtsResultProvider, QmdResultProvider, SearchResultProvider};
```

Add `TavilyProvider` and `CompositeSearchProvider`:
```rust
use search::{FtsResultProvider, QmdResultProvider, SearchResultProvider, TavilyProvider, CompositeSearchProvider};
```

### Step 4.2: Update settings read block (lines 293-297)

The current block reads only `semantic_search_enabled` and `semantic_search_accuracy`:
```rust
let (search_enabled, search_accuracy) = {
    let manager = app.state::<Arc<RwLock<SettingsManager>>>();
    let settings = manager.read().expect("Failed to acquire settings read lock").get();
    (settings.search.semantic_search_enabled, settings.search.semantic_search_accuracy)
};
```

Add `tavily_api_key` to the destructure:
```rust
let (search_enabled, search_accuracy, tavily_api_key) = {
    let manager = app.state::<Arc<RwLock<SettingsManager>>>();
    let settings = manager.read().expect("Failed to acquire settings read lock").get();
    (
        settings.search.semantic_search_enabled,
        settings.search.semantic_search_accuracy,
        settings.search.tavily_api_key.clone(),
    )
};
```

### Step 4.3: Rename the existing search provider to `gem_provider` (lines 299-327)

The current code assigns to `let search_provider: Arc<dyn SearchResultProvider>`. Change this variable name to `gem_provider` since it's now just the gem search half of the composite:

```rust
let gem_provider: Arc<dyn SearchResultProvider> = if search_enabled {
    // ... existing QMD/FTS logic unchanged ...
} else {
    // ... existing FTS fallback unchanged ...
};
```

**Important:** Do NOT change the logic inside this block — only rename the variable from `search_provider` to `gem_provider`.

### Step 4.4: Build optional Tavily web provider (add after gem_provider block)

```rust
// Build web search provider from existing settings
let web_provider: Option<Arc<dyn SearchResultProvider>> = tavily_api_key
    .as_ref()
    .filter(|k| !k.is_empty())
    .map(|api_key| {
        eprintln!("Search: Tavily web search enabled");
        Arc::new(TavilyProvider::new(api_key.clone())) as Arc<dyn SearchResultProvider>
    });

if web_provider.is_none() {
    eprintln!("Search: Tavily web search disabled (no API key in settings)");
}
```

### Step 4.5: Wrap in CompositeSearchProvider and register (replace old `app.manage(search_provider)`)

Replace the old `app.manage(search_provider);` (line 329) with:

```rust
// Wrap in composite — single Arc<dyn SearchResultProvider> for all search needs
let search_provider: Arc<dyn SearchResultProvider> = Arc::new(
    CompositeSearchProvider::new(gem_provider, web_provider)
);
app.manage(search_provider);
```

### Summary of Changes to lib.rs

The total change is:
1. **Line 26**: Add `TavilyProvider, CompositeSearchProvider` to imports
2. **Lines 293-297**: Add `tavily_api_key` to settings read
3. **Line 299**: Rename `search_provider` → `gem_provider`
4. **After the gem_provider block**: Add web_provider creation
5. **Line 329**: Replace `app.manage(search_provider)` with composite wrapping + registration

Everything else stays exactly the same. The `generate_handler!` is unchanged. No new commands in this phase.

## Phase Checkpoint

After implementation, verify:

```
cargo build
```

This MUST pass. Specifically verify:
1. App compiles without errors
2. The `search_provider` registered in Tauri state is now a `CompositeSearchProvider`
3. All existing search commands (`search_gems`, `check_search_availability`, `rebuild_search_index`) are still in `generate_handler!` and unchanged
4. No unused import warnings for `TavilyProvider` or `CompositeSearchProvider`

## Important Notes

- **Only modify `lib.rs`.** No other files change in this phase.
- **Do NOT modify `commands.rs`.** Search commands are unchanged.
- **Do NOT add any new commands to `generate_handler!`.** Agent commands come in Phase 4.
- **Do NOT create any agent files.** Agent backend is Phase 3.
- **The existing QMD/FTS provider initialization logic stays identical** — you're just renaming the variable and wrapping it.
- The `tavily_api_key` field is `Option<String>` in settings. Filter for `Some` and non-empty. If `None` or empty string, `web_provider` is `None` and the composite gracefully returns empty web results.
- If you're unsure about `Arc` casting, follow the exact pattern from the design doc: `Arc::new(TavilyProvider::new(api_key.clone())) as Arc<dyn SearchResultProvider>`.

## Deliverables

When done, present:
1. The modified `lib.rs` (diff of changes)
2. `cargo build` output showing success
3. Ask me for review before proceeding to Phase 3

## Potential Questions You Might Have

**Q: Should I clone `search_provider` before `app.manage`?**
A: No. In this phase the `search_provider` is only used once (to register in state). In Phase 4 when the agent is added, we'll need `.clone()` — but not yet.

**Q: What if `tavily_api_key` is `Some("")` (empty string)?**
A: The `.filter(|k| !k.is_empty())` handles this — it converts `Some("")` to `None`.

**Q: Will existing `search_gems` still work?**
A: Yes. `CompositeSearchProvider::search()` delegates to `gem_provider.search()`. Existing commands don't call `web_search()` so they're unaffected.
