# Kiro Prompt — Searchable Gems Phase 8: Frontend Settings UI for Semantic Search

## What You're Building

Add a "Semantic Search" section to the Settings page where users can:
1. See current status (not configured / ready / unavailable)
2. Click "Enable Semantic Search" to run the automated 6-step setup
3. See step-by-step progress with status indicators
4. Rebuild the search index
5. Disable semantic search

## Spec Files

- **Tasks**: `.kiro/specs/searchable-gems/tasks.md` — Phase 8, Tasks 13–14

## Context: What Already Exists

**Backend (complete from Phases 4–6):**
- `setup_semantic_search` — runs the 6-step automated setup, emits progress events on `"semantic-search-setup"` channel
- `check_search_availability` — returns `AvailabilityResult { available, reason }`
- `rebuild_search_index` — reindexes all gems, returns count
- `update_settings` — saves settings including `search.semantic_search_enabled`
- Settings struct has `search: SearchSettings { semantic_search_enabled: bool }`

**Frontend types (from Phase 7):**
- `QmdSetupResult` — `{ success, node_version?, qmd_version?, docs_indexed?, error? }`
- `SetupProgressEvent` — `{ step, total, description, status }`
- `AvailabilityResult` — `{ available, reason? }`

**Settings component pattern (from MLX venv setup):**
- State vars: `venvSetupInProgress`, `venvSetupPhase`, `venvSetupError`
- Event listeners: `listen('mlx-venv-setup-progress')`, `listen('mlx-venv-setup-complete')`, `listen('mlx-venv-setup-error')`
- Inline-styled info banners: setup needed (blue), in progress (yellow), error (red), ready (green)
- Button triggers `invoke('setup_mlx_venv')`, sets state immediately

## Important: Settings Type Needs Updating

The TypeScript `Settings` interface in `types.ts` (line 239) currently has:
```typescript
export interface Settings {
  transcription: TranscriptionSettings;
  intelligence: IntelligenceSettings;
  copilot: CoPilotSettings;
}
```

The Rust backend now sends `search: SearchSettings` too (added in Phase 4, with `#[serde(default)]`). The frontend `Settings` interface needs a matching `search` field.

## Exact Changes

### Part A: Update `Settings` Type in `types.ts`

**Add `SearchSettings` interface** before the `Settings` interface (before line 239):

```typescript
/** Search settings matching Rust SearchSettings struct */
export interface SearchSettings {
  /** Whether semantic search (QMD) is enabled */
  semantic_search_enabled: boolean;
}
```

**Update `Settings` interface** to include the `search` field:

Find (lines 239–248):
```typescript
export interface Settings {
  /** Transcription-specific settings */
  transcription: TranscriptionSettings;

  /** Intelligence-specific settings */
  intelligence: IntelligenceSettings;

  /** Co-Pilot agent settings */
  copilot: CoPilotSettings;
}
```

Replace with:
```typescript
export interface Settings {
  /** Transcription-specific settings */
  transcription: TranscriptionSettings;

  /** Intelligence-specific settings */
  intelligence: IntelligenceSettings;

  /** Co-Pilot agent settings */
  copilot: CoPilotSettings;

  /** Search settings (semantic search via QMD) */
  search: SearchSettings;
}
```

### Part B: Add Imports to `Settings.tsx`

**Update the import block** (lines 5–19). Add the new types:

Find:
```typescript
import type {
  Settings,
  ModelInfo,
  LlmModelInfo,
  ModelProgressEvent,
  LlmModelProgressEvent,
  ModelDownloadCompleteEvent,
  LlmModelDownloadCompleteEvent,
  ModelDownloadErrorEvent,
  LlmModelDownloadErrorEvent,
  SettingsChangedEvent,
  WhisperKitStatus,
  MlxDiagnostics,
  MlxVenvProgressEvent,
} from '../state/types';
```

Replace with:
```typescript
import type {
  Settings,
  ModelInfo,
  LlmModelInfo,
  ModelProgressEvent,
  LlmModelProgressEvent,
  ModelDownloadCompleteEvent,
  LlmModelDownloadCompleteEvent,
  ModelDownloadErrorEvent,
  LlmModelDownloadErrorEvent,
  SettingsChangedEvent,
  WhisperKitStatus,
  MlxDiagnostics,
  MlxVenvProgressEvent,
  AvailabilityResult,
  QmdSetupResult,
  SetupProgressEvent,
} from '../state/types';
```

### Part C: Add State Variables

**Add these state variables** after the existing ones (after line 41, after `venvSetupError`):

```typescript
  // Semantic search state
  const [searchAvailability, setSearchAvailability] = useState<AvailabilityResult | null>(null);
  const [searchSetupInProgress, setSearchSetupInProgress] = useState(false);
  const [searchSetupSteps, setSearchSetupSteps] = useState<SetupProgressEvent[]>([]);
  const [searchSetupError, setSearchSetupError] = useState<string | null>(null);
  const [searchSetupResult, setSearchSetupResult] = useState<QmdSetupResult | null>(null);
  const [rebuildingIndex, setRebuildingIndex] = useState(false);
```

### Part D: Check Search Availability on Mount

**Add `check_search_availability`** to the `loadData` function. The existing `Promise.all` (lines 48–55) loads multiple things at once. Add the search availability check.

Find the `Promise.all` block (lines 48–55):
```typescript
        const [settingsData, browserSettingsData, modelsData, whisperKitModelsData, whisperKitStatusData, llmModelsData] = await Promise.all([
          invoke<Settings>('get_settings'),
          invoke<BrowserSettings>('get_browser_settings'),
          invoke<ModelInfo[]>('list_models'),
          invoke<ModelInfo[]>('list_whisperkit_models'),
          invoke<WhisperKitStatus>('check_whisperkit_status'),
          invoke<LlmModelInfo[]>('list_llm_models'),
        ]);
```

Replace with:
```typescript
        const [settingsData, browserSettingsData, modelsData, whisperKitModelsData, whisperKitStatusData, llmModelsData, searchAvailabilityData] = await Promise.all([
          invoke<Settings>('get_settings'),
          invoke<BrowserSettings>('get_browser_settings'),
          invoke<ModelInfo[]>('list_models'),
          invoke<ModelInfo[]>('list_whisperkit_models'),
          invoke<WhisperKitStatus>('check_whisperkit_status'),
          invoke<LlmModelInfo[]>('list_llm_models'),
          invoke<AvailabilityResult>('check_search_availability'),
        ]);
```

**Add the state setter** after the existing setters (after line 61, after `setLlmModels`):

```typescript
        setSearchAvailability(searchAvailabilityData);
```

### Part E: Add Event Listener for Setup Progress

**Add a new `useEffect`** for the `semantic-search-setup` event. Place it after the MLX venv setup event listener block (after line 233):

```typescript
  // Listen for semantic search setup events
  useEffect(() => {
    const unlisten = listen<SetupProgressEvent>('semantic-search-setup', (event) => {
      const step = event.payload;
      setSearchSetupSteps(prev => {
        // Replace existing step or add new one
        const existing = prev.findIndex(s => s.step === step.step);
        if (existing >= 0) {
          const updated = [...prev];
          updated[existing] = step;
          return updated;
        }
        return [...prev, step];
      });
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, []);
```

### Part F: Add Handler Functions

**Add these handler functions** inside the `Settings` component, after the existing handler functions (near the other `handle...` functions, before the `return` statement):

```typescript
  // ── Semantic Search handlers ──

  const handleSetupSemanticSearch = async () => {
    setSearchSetupInProgress(true);
    setSearchSetupSteps([]);
    setSearchSetupError(null);
    setSearchSetupResult(null);
    try {
      const result = await invoke<QmdSetupResult>('setup_semantic_search');
      setSearchSetupResult(result);
      if (result.success) {
        // Refresh availability
        const availability = await invoke<AvailabilityResult>('check_search_availability');
        setSearchAvailability(availability);
        // Refresh settings (semantic_search_enabled is now true)
        const updatedSettings = await invoke<Settings>('get_settings');
        setSettings(updatedSettings);
      } else {
        setSearchSetupError(result.error || 'Setup failed');
      }
    } catch (err) {
      setSearchSetupError(err instanceof Error ? err.message : String(err));
    } finally {
      setSearchSetupInProgress(false);
    }
  };

  const handleDisableSemanticSearch = async () => {
    if (!settings) return;
    try {
      const updatedSettings = {
        ...settings,
        search: {
          ...settings.search,
          semantic_search_enabled: false,
        },
      };
      await invoke('update_settings', { settings: updatedSettings });
      setSettings(updatedSettings);
      setSearchAvailability({ available: true, reason: undefined });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleRebuildIndex = async () => {
    setRebuildingIndex(true);
    try {
      const count = await invoke<number>('rebuild_search_index');
      setRebuildingIndex(false);
      // Brief success feedback — could use a toast, but inline message is simpler
      setSearchSetupError(null);
    } catch (err) {
      setRebuildingIndex(false);
      setSearchSetupError(err instanceof Error ? err.message : String(err));
    }
  };
```

### Part G: Add the Settings Section JSX

**Insert this section** after the Co-Pilot section (after line 1075, after `</section>` for Co-Pilot) and **before** the Whisper section (line 1077):

```tsx
        <section className="settings-section">
          <h3>Semantic Search</h3>

          {/* Not configured state */}
          {!settings.search.semantic_search_enabled && !searchSetupInProgress && (
            <>
              <div className="info-banner" style={{
                padding: '12px',
                marginBottom: '16px',
                backgroundColor: '#d1ecf1',
                border: '1px solid #bee5eb',
                borderRadius: '4px',
                color: '#0c5460'
              }}>
                <strong>Not configured</strong>
                <p style={{ margin: '8px 0 0 0', fontSize: '13px' }}>
                  Semantic search finds gems by meaning, not just keywords. For example, searching
                  "container orchestration" can find a gem titled "ECS vs EKS comparison" even without exact keyword matches.
                </p>
                <p style={{ margin: '4px 0 0 0', fontSize: '12px', opacity: 0.8 }}>
                  Requires: Node.js 22+, ~2GB disk space for search models. Setup takes 2-5 minutes.
                </p>
                <div style={{ marginTop: '12px' }}>
                  <button
                    onClick={handleSetupSemanticSearch}
                    style={{
                      padding: '8px 20px',
                      backgroundColor: '#0c5460',
                      color: 'white',
                      border: 'none',
                      borderRadius: '4px',
                      cursor: 'pointer',
                      fontSize: '13px',
                      fontWeight: 600,
                    }}
                  >
                    Enable Semantic Search
                  </button>
                </div>
              </div>
            </>
          )}

          {/* Setup in progress */}
          {searchSetupInProgress && (
            <div className="info-banner" style={{
              padding: '12px',
              marginBottom: '16px',
              backgroundColor: '#fff3cd',
              border: '1px solid #ffc107',
              borderRadius: '4px',
              color: '#856404'
            }}>
              <strong>Setting up semantic search...</strong>
              <div style={{ marginTop: '8px' }}>
                {searchSetupSteps.map(step => (
                  <div key={step.step} style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '8px',
                    padding: '4px 0',
                    fontSize: '13px',
                  }}>
                    <span style={{ width: '20px', textAlign: 'center' }}>
                      {step.status === 'done' ? '✓' : step.status === 'failed' ? '✗' : '⟳'}
                    </span>
                    <span style={{
                      opacity: step.status === 'done' ? 0.6 : 1,
                      textDecoration: step.status === 'done' ? 'none' : 'none',
                    }}>
                      {step.description}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Setup error */}
          {searchSetupError && !searchSetupInProgress && (
            <div className="info-banner" style={{
              padding: '12px',
              marginBottom: '16px',
              backgroundColor: '#f8d7da',
              border: '1px solid #f5c6cb',
              borderRadius: '4px',
              color: '#721c24'
            }}>
              <strong>Setup failed:</strong> {searchSetupError}
              <div style={{ marginTop: '8px' }}>
                <button
                  onClick={handleSetupSemanticSearch}
                  style={{
                    padding: '6px 16px',
                    backgroundColor: '#721c24',
                    color: 'white',
                    border: 'none',
                    borderRadius: '4px',
                    cursor: 'pointer',
                    fontSize: '13px',
                  }}
                >
                  Retry
                </button>
              </div>
            </div>
          )}

          {/* Setup success (just completed) */}
          {searchSetupResult?.success && !searchSetupInProgress && (
            <div className="info-banner" style={{
              padding: '12px',
              marginBottom: '16px',
              backgroundColor: '#d4edda',
              border: '1px solid #c3e6cb',
              borderRadius: '4px',
              color: '#155724'
            }}>
              <strong>Semantic search enabled!</strong>
              <p style={{ margin: '4px 0 0 0', fontSize: '13px' }}>
                QMD {searchSetupResult.qmd_version} installed. Restart Jarvis to activate semantic search.
              </p>
            </div>
          )}

          {/* Configured and active */}
          {settings.search.semantic_search_enabled && !searchSetupInProgress && !searchSetupResult?.success && (
            <div className="info-banner" style={{
              padding: '12px',
              marginBottom: '16px',
              backgroundColor: searchAvailability?.available ? '#d4edda' : '#fff3cd',
              border: `1px solid ${searchAvailability?.available ? '#c3e6cb' : '#ffc107'}`,
              borderRadius: '4px',
              color: searchAvailability?.available ? '#155724' : '#856404',
            }}>
              <strong>
                {searchAvailability?.available ? 'Semantic search active' : 'Semantic search enabled (not ready)'}
              </strong>
              {!searchAvailability?.available && searchAvailability?.reason && (
                <p style={{ margin: '4px 0 0 0', fontSize: '13px' }}>
                  {searchAvailability.reason}
                </p>
              )}
              <div style={{ marginTop: '12px', display: 'flex', gap: '8px' }}>
                <button
                  onClick={handleRebuildIndex}
                  disabled={rebuildingIndex || !searchAvailability?.available}
                  style={{
                    padding: '6px 16px',
                    backgroundColor: searchAvailability?.available ? '#155724' : '#6c757d',
                    color: 'white',
                    border: 'none',
                    borderRadius: '4px',
                    cursor: searchAvailability?.available ? 'pointer' : 'not-allowed',
                    fontSize: '13px',
                    opacity: rebuildingIndex ? 0.7 : 1,
                  }}
                >
                  {rebuildingIndex ? 'Rebuilding...' : 'Rebuild Index'}
                </button>
                <button
                  onClick={handleDisableSemanticSearch}
                  style={{
                    padding: '6px 16px',
                    backgroundColor: 'transparent',
                    color: '#721c24',
                    border: '1px solid #721c24',
                    borderRadius: '4px',
                    cursor: 'pointer',
                    fontSize: '13px',
                  }}
                >
                  Disable
                </button>
              </div>
              <p style={{ margin: '8px 0 0 0', fontSize: '11px', opacity: 0.7 }}>
                Disabling does not uninstall QMD or delete models. Restart required to take effect.
              </p>
            </div>
          )}
        </section>
```

## Gotchas

1. **Settings type must have `search` field** — The Rust backend sends `search: { semantic_search_enabled: bool }` via `#[serde(default)]`. If the TypeScript `Settings` interface doesn't have this field, it will be silently ignored by JSON deserialization. **You MUST add `SearchSettings` and the `search` field to `Settings`** in types.ts.

2. **`setup_semantic_search` returns the result, not events** — Unlike the MLX venv setup (which is fire-and-forget with event callbacks), `setup_semantic_search` is `await`-ed and returns `QmdSetupResult`. The progress events on `"semantic-search-setup"` channel provide live step-by-step updates during the await.

3. **`handleDisableSemanticSearch` only changes the setting** — It does NOT uninstall QMD or delete models. This is explicitly documented in the UI text. A restart is needed for the app to switch back to FTS5 provider.

4. **Status text values** — The backend emits `SetupProgressEvent` with `status` field values: `"running"`, `"done"`, `"failed"`. The JSX maps these to symbols: `✓` (done), `✗` (failed), `⟳` (running).

5. **Color scheme matches MLX setup** — All info-banner colors are identical to the MLX venv setup banners:
   - Not configured: `#d1ecf1` / `#0c5460` (blue/info)
   - In progress: `#fff3cd` / `#856404` (yellow/warning)
   - Error: `#f8d7da` / `#721c24` (red/danger)
   - Ready: `#d4edda` / `#155724` (green/success)

6. **Placement** — The Semantic Search section goes AFTER Co-Pilot (line 1075) and BEFORE Whisper (line 1077). This groups AI-related settings together.

7. **`searchSetupResult?.success`** — The "just completed" banner shows immediately after setup succeeds. It disappears on next mount (page navigation) since `searchSetupResult` starts as `null`. The "configured and active" banner takes over on subsequent visits.

8. **No new CSS needed** — All styles are inline, following the exact pattern of the existing MLX venv setup banners (lines 808–899). No changes to `App.css`.

## Verification

1. Run `npm run build` (or `pnpm build`) from `jarvis-app/` — must pass with no TypeScript errors
2. The Settings page should render without errors
3. The Semantic Search section should show "Not configured" state by default
4. No functional testing needed (QMD is not installed in dev environment)

**Expected outcome:**
- 1 file modified: `src/state/types.ts` (add `SearchSettings` interface, add `search` field to `Settings`)
- 1 file modified: `src/components/Settings.tsx` (imports, state vars, event listener, handlers, JSX section)
- Frontend builds with no errors

## Summary of All Changes

| File | Change |
|------|--------|
| `src/state/types.ts` | Add `SearchSettings` interface |
| `src/state/types.ts` | Add `search: SearchSettings` to `Settings` interface |
| `src/components/Settings.tsx` | Import `AvailabilityResult`, `QmdSetupResult`, `SetupProgressEvent` |
| `src/components/Settings.tsx` | Add 6 state variables for semantic search |
| `src/components/Settings.tsx` | Add `check_search_availability` to mount `Promise.all` |
| `src/components/Settings.tsx` | Add `semantic-search-setup` event listener |
| `src/components/Settings.tsx` | Add 3 handler functions |
| `src/components/Settings.tsx` | Add Semantic Search section JSX after Co-Pilot |

## When Done

Stop and ask for review. Show me:
1. The updated `Settings` interface in types.ts
2. The new state variables and event listener
3. The handler functions
4. The Semantic Search settings section JSX
5. Build output (no TypeScript errors)
6. Any questions or decisions you made

Do NOT proceed to Phase 9 (testing) until I review and approve.
