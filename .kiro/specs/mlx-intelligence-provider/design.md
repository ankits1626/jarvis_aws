# Design: MLX Intelligence Provider

## Overview

This feature adds local LLM inference capabilities to Jarvis using MLX on Apple Silicon. It introduces a Python sidecar server running MLX-LM models, a Rust-based model manager for downloading and managing LLM models, and a new `MlxProvider` implementation of the `IntelProvider` trait. The system enables private, on-device AI enrichment of gems (tags and summaries) without sending data to external services.

The design follows the existing sidecar pattern established by `IntelligenceKitProvider` and the model management pattern from `ModelManager` (Whisper models). Users can download models from a curated catalog, switch between models, and configure provider preferences through settings.

### Key Design Goals

1. **Privacy-first**: All inference happens locally on the user's machine
2. **Swappable providers**: MLX integrates seamlessly via the existing `IntelProvider` trait
3. **Graceful degradation**: Falls back to IntelligenceKit or NoOpProvider if MLX is unavailable
4. **Familiar UX**: Model management mirrors the existing Whisper model workflow
5. **Hot-swappable models**: Switch models without restarting the app

## Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      Jarvis App (Tauri)                      │
├─────────────────────────────────────────────────────────────┤
│  Frontend (React)                                            │
│  ├─ Settings UI                                              │
│  │  ├─ Provider Selector (MLX / IntelligenceKit / API)      │
│  │  └─ Model List (Download / Switch / Delete)              │
│  └─ Gems Panel (Enrich button)                              │
├─────────────────────────────────────────────────────────────┤
│  Backend (Rust)                                              │
│  ├─ Commands                                                 │
│  │  ├─ list_llm_models()                                     │
│  │  ├─ download_llm_model(model_id)                         │
│  │  ├─ cancel_llm_download(model_id)                        │
│  │  ├─ delete_llm_model(model_id)                           │
│  │  ├─ switch_llm_model(model_id)                           │
│  │  └─ enrich_gem(gem_id) [existing, unchanged]             │
│  ├─ Intelligence Module                                      │
│  │  ├─ IntelProvider trait [existing]                       │
│  │  ├─ MlxProvider [NEW]                                    │
│  │  ├─ IntelligenceKitProvider [existing]                   │
│  │  ├─ NoOpProvider [existing]                              │
│  │  └─ LlmModelManager [NEW]                                │
│  ├─ Settings Module                                          │
│  │  ├─ SettingsManager [extended]                           │
│  │  └─ IntelligenceSettings [NEW]                           │
│  └─ Gems Module [existing, unchanged]                        │
├─────────────────────────────────────────────────────────────┤
│  Python MLX Sidecar (server.py)                             │
│  ├─ NDJSON Protocol (stdin/stdout)                          │
│  ├─ Commands: check-availability, load-model,               │
│  │            generate-tags, summarize, download-model,     │
│  │            model-info, shutdown                          │
│  └─ MLX-LM Integration                                       │
└─────────────────────────────────────────────────────────────┘
```

### Provider Selection Flow


```
App Startup
    ↓
Load Settings (intelligence.provider)
    ↓
┌───────────────────────────────────────┐
│ Provider = "mlx"?                     │
├───────────────────────────────────────┤
│ YES → Try MlxProvider::new()          │
│       ├─ Success → Use MlxProvider    │
│       └─ Fail → Try IntelligenceKit   │
│                 ├─ Success → Use IK   │
│                 └─ Fail → NoOpProvider│
├───────────────────────────────────────┤
│ Provider = "intelligencekit"?         │
│ YES → Try IntelligenceKitProvider     │
│       ├─ Success → Use IK             │
│       └─ Fail → NoOpProvider          │
├───────────────────────────────────────┤
│ Provider = "api"?                     │
│ YES → NoOpProvider (not implemented)  │
└───────────────────────────────────────┘
```

### Sidecar Communication Protocol

The Python MLX sidecar uses NDJSON (newline-delimited JSON) over stdin/stdout, matching the pattern from `IntelligenceKitProvider`. Each command is a single JSON object on one line, and each response is a single JSON object on one line.

**Command Format:**
```json
{"command": "...", "param1": "...", "param2": "..."}
```

**Response Format:**
```json
{"ok": true, "result": "..."}
{"ok": false, "error": "..."}
```

## Components and Interfaces

### 1. Python MLX Sidecar (`src-tauri/sidecars/mlx-server/server.py`)


**Responsibilities:**
- Load and manage MLX-LM models (for inference)
- Generate tags and summaries using loaded models
- Download models from HuggingFace (when spawned as a download-only process)
- Handle NDJSON commands from Rust backend

**Usage Modes:**

The sidecar operates in two distinct modes:

1. **Inference Mode** (long-lived process owned by MlxProvider):
   - Spawned once at app startup
   - Loads a model and keeps it resident
   - Handles `check-availability`, `load-model`, `generate-tags`, `summarize`, `model-info`, `shutdown`
   - Never handles `download-model` (would block inference)

2. **Download Mode** (short-lived process owned by LlmModelManager):
   - Spawned per download operation
   - Handles only `download-model` command
   - Terminates when download completes or fails
   - Never loads models or performs inference

**State:**
- Current loaded model (None or model instance) — inference mode only
- Tokenizer for the loaded model — inference mode only

**Commands:**

| Command | Parameters | Response | Description |
|---------|-----------|----------|-------------|
| `check-availability` | none | `{"ok":true,"available":bool,"reason":str?}` | Check if mlx_lm is importable |
| `load-model` | `model_path: str` | `{"ok":true,"loaded":true,"model":str}` or error | Load model from disk |
| `generate-tags` | `content: str` | `{"ok":true,"result":["tag1",...]}` or error | Generate 3-5 topic tags |
| `summarize` | `content: str` | `{"ok":true,"result":"summary"}` or error | Generate one-sentence summary |
| `download-model` | `repo_id: str, target_dir: str` | Progress: `{"ok":true,"progress":float,"downloaded_mb":float}`<br>Complete: `{"ok":true,"complete":true}` | Download model from HuggingFace |
| `model-info` | none | `{"ok":true,"model":str,"params":str}` or error | Get info about loaded model |
| `shutdown` | none | `{"ok":true}` | Graceful shutdown |

**Error Handling:**
- Malformed JSON → `{"ok":false,"error":"Invalid JSON: ..."}`
- No model loaded → `{"ok":false,"error":"No model loaded"}`
- Invalid model path → `{"ok":false,"error":"..."}`
- Download failure → `{"ok":false,"error":"..."}`

**Implementation Notes:**
- Use `/no_think` suffix in prompts to suppress Qwen thinking mode
- `max_tokens=200` for tag generation
- `max_tokens=150` for summarization
- Check `platform.machine()` and refuse to load on non-ARM64

### 2. LlmModelManager (`src/intelligence/llm_model_manager.rs`)


**Responsibilities:**
- Manage LLM model catalog
- Track download status for each model
- Orchestrate downloads via Python sidecar
- Validate downloaded models
- Handle cancellation and deletion

**Data Structures:**

```rust
pub struct LlmModelManager {
    models_dir: PathBuf,  // ~/.jarvis/models/llm/
    app_handle: AppHandle,
    download_queue: Arc<TokioMutex<HashMap<String, DownloadState>>>,
    error_states: Arc<TokioMutex<HashMap<String, String>>>,
}

struct DownloadState {
    progress: f32,
    cancel_token: CancellationToken,
}

#[derive(Serialize, Deserialize)]
pub struct LlmModelInfo {
    pub id: String,
    pub display_name: String,
    pub repo_id: String,
    pub description: String,
    pub size_estimate: String,
    pub quality_tier: String,
    pub status: ModelStatus,
}

// Reuse existing ModelStatus enum from model_manager.rs
// Import via: use crate::settings::model_manager::ModelStatus;
pub enum ModelStatus {
    Downloaded { size_bytes: u64 },
    Downloading { progress: f32 },
    Error { message: String },
    NotDownloaded,
}
```

**Model Catalog:**

```rust
const LLM_MODEL_CATALOG: &[LlmModelEntry] = &[
    LlmModelEntry {
        id: "qwen3-8b-4bit",
        repo_id: "mlx-community/Qwen3-8B-4bit",
        display_name: "Qwen 3 8B (Q4)",
        description: "Great quality, balanced performance. Recommended.",
        size_estimate: "~5 GB",
        quality_tier: "great",
    },
    // ... other models
];
```


**Methods:**

```rust
impl LlmModelManager {
    pub fn new(app_handle: AppHandle) -> Result<Self, String>;
    pub async fn list_models(&self) -> Result<Vec<LlmModelInfo>, String>;
    pub async fn download_model(&self, model_id: String) -> Result<(), String>;
    pub async fn cancel_download(&self, model_id: String) -> Result<(), String>;
    pub async fn delete_model(&self, model_id: String) -> Result<(), String>;
    pub fn model_path(&self, model_id: &str) -> PathBuf;
    fn validate_model(&self, model_id: &str) -> bool;
}
```

**Download Flow:**
1. Validate model_id against catalog
2. Check if already downloading → error if yes
3. Add to download_queue with CancellationToken
4. Spawn async task that:
   - Spawns a **separate** short-lived Python process for the download (not the inference sidecar)
   - Sends `download-model` command to this dedicated process
   - Listens for progress responses
   - Emits `llm-model-download-progress` Tauri events
   - On completion: validates model, emits `llm-model-download-complete`, terminates Python process
   - On error: cleans up `.downloads/`, emits `llm-model-download-error`, terminates Python process

**Rationale for Separate Download Process:**
- The inference sidecar (owned by MlxProvider) should not be blocked by multi-GB downloads
- Downloads are independent operations that don't require a loaded model
- Simpler lifecycle management — download process terminates when done
- No circular dependency between LlmModelManager and MlxProvider

**File Layout:**
```
~/.jarvis/models/llm/
├── .downloads/              # Temporary download directory
│   └── qwen3-8b-4bit/      # In-progress download
└── qwen3-8b-4bit/          # Completed download
    ├── config.json
    ├── tokenizer.json
    └── *.safetensors
```

### 3. MlxProvider (`src/intelligence/mlx_provider.rs`)

**Responsibilities:**
- Implement `IntelProvider` trait
- Manage Python sidecar lifecycle
- Handle content chunking for large inputs
- Deduplicate tags
- Combine multi-chunk summaries


**Data Structures:**

```rust
pub struct MlxProvider {
    app_handle: AppHandle,
    state: Arc<Mutex<ProviderState>>,
}

struct ProviderState {
    child: Option<Child>,
    stdin: Option<BufWriter<ChildStdin>>,
    stdout: Option<BufReader<ChildStdout>>,
    model_loaded: bool,
    current_model: Option<String>,
}
```

**Implementation:**

```rust
impl MlxProvider {
    pub async fn new(
        app_handle: AppHandle,
        model_path: PathBuf,
        python_path: String,
    ) -> Result<Self, String>;
    
    pub async fn switch_model(&self, model_path: PathBuf) -> Result<(), String>;
    
    async fn send_command(&self, cmd: NdjsonCommand) -> Result<NdjsonResponse, String>;
    
    async fn shutdown(&self);
    
    fn resolve_sidecar_path(app_handle: &AppHandle) -> Result<PathBuf, String>;
}

#[async_trait]
impl IntelProvider for MlxProvider {
    async fn check_availability(&self) -> AvailabilityResult;
    async fn generate_tags(&self, content: &str) -> Result<Vec<String>, String>;
    async fn summarize(&self, content: &str) -> Result<String, String>;
}
```

**Sidecar Script Resolution:**

The Python sidecar script must be located at runtime using the same pattern as `IntelligenceKitProvider`:

```rust
fn resolve_sidecar_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    // Try production bundle path first
    if let Ok(resource_path) = app_handle.path().resource_dir() {
        let bundled_path = resource_path.join("sidecars/mlx-server/server.py");
        if bundled_path.exists() {
            return Ok(bundled_path);
        }
    }
    
    // Fall back to development path
    let dev_path = PathBuf::from("src-tauri/sidecars/mlx-server/server.py");
    if dev_path.exists() {
        return Ok(dev_path);
    }
    
    Err("MLX sidecar script not found".to_string())
}
```

**Usage in `MlxProvider::new()`:**

```rust
let script_path = Self::resolve_sidecar_path(&app_handle)?;

let mut child = tokio::process::Command::new(&python_path)
    .arg(&script_path)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .map_err(|e| format!("Failed to spawn MLX sidecar: {}", e))?;
```

**Tauri Configuration:**

The `tauri.conf.json` must include the sidecar script in the bundle:

```json
{
  "bundle": {
    "resources": [
      "sidecars/mlx-server/**"
    ]
  }
}
```

**Content Chunking:**
- Max chunk size: 15,000 characters (MLX models have 8K+ token context)
- Split at paragraph boundaries (`\n\n`), then line boundaries (`\n`), then spaces
- For tags: deduplicate case-insensitively, return max 5
- For summaries: if multiple chunks, combine summaries and re-summarize

**Timeout Configuration:**
- 60 seconds for `generate-tags` and `summarize` (longer than IntelligenceKit's 30s)
- 15 seconds for initialization during app startup (includes sidecar spawn + model load)
  - Rationale: Qwen 8B loads in 3-5s on M3 Pro, 5-10s on M1/M2
  - Model loading is a one-time cost at startup, worth the wait for local inference

### 4. IntelligenceSettings (`src/settings/manager.rs` - extension)


**Data Structure:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceSettings {
    pub provider: String,        // "mlx" | "intelligencekit" | "api"
    pub active_model: String,    // catalog ID, e.g. "qwen3-8b-4bit"
    pub python_path: String,     // "python3" or absolute path
}

impl Default for IntelligenceSettings {
    fn default() -> Self {
        Self {
            provider: "mlx".to_string(),
            active_model: "qwen3-8b-4bit".to_string(),
            python_path: "python3".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub transcription: TranscriptionSettings,
    #[serde(default)]
    pub browser: BrowserSettings,
    #[serde(default)]  // Backward compatibility
    pub intelligence: IntelligenceSettings,
}
```

**Validation:**

```rust
impl SettingsManager {
    fn validate(settings: &Settings) -> Result<(), String> {
        // Existing validations...
        
        // Intelligence settings validation
        let valid_providers = ["mlx", "intelligencekit", "api"];
        if !valid_providers.contains(&settings.intelligence.provider.as_str()) {
            return Err(format!(
                "Invalid provider: {}. Must be one of: mlx, intelligencekit, api",
                settings.intelligence.provider
            ));
        }
        
        if settings.intelligence.active_model.trim().is_empty() {
            return Err("active_model cannot be empty".to_string());
        }
        
        if settings.intelligence.python_path.trim().is_empty() {
            return Err("python_path cannot be empty".to_string());
        }
        
        Ok(())
    }
}
```

### 5. Tauri Commands (`src/commands.rs` - additions)


**New Commands:**

```rust
#[tauri::command]
async fn list_llm_models(
    llm_manager: State<'_, Arc<LlmModelManager>>,
) -> Result<Vec<LlmModelInfo>, String> {
    llm_manager.list_models().await
}

#[tauri::command]
async fn download_llm_model(
    model_id: String,
    llm_manager: State<'_, Arc<LlmModelManager>>,
) -> Result<(), String> {
    llm_manager.download_model(model_id).await
}

#[tauri::command]
async fn cancel_llm_download(
    model_id: String,
    llm_manager: State<'_, Arc<LlmModelManager>>,
) -> Result<(), String> {
    llm_manager.cancel_download(model_id).await
}

#[tauri::command]
async fn delete_llm_model(
    model_id: String,
    llm_manager: State<'_, Arc<LlmModelManager>>,
) -> Result<(), String> {
    llm_manager.delete_model(model_id).await
}

#[tauri::command]
async fn switch_llm_model(
    model_id: String,
    llm_manager: State<'_, Arc<LlmModelManager>>,
    settings_manager: State<'_, Arc<RwLock<SettingsManager>>>,
    mlx_provider: State<'_, Arc<Mutex<Option<MlxProvider>>>>,
) -> Result<(), String> {
    // 1. Verify model is downloaded
    let models = llm_manager.list_models().await?;
    let model = models.iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| format!("Model {} not found", model_id))?;
    
    match &model.status {
        ModelStatus::Downloaded { .. } => {},
        _ => return Err(format!("Model {} is not downloaded", model_id)),
    }
    
    // 2. Update settings (hold write lock for entire operation)
    {
        let manager = settings_manager.write().unwrap();
        let mut settings = manager.get();
        settings.intelligence.active_model = model_id.clone();
        manager.update(settings)?;
    }
    
    // 3. Tell MlxProvider to reload model (if it exists)
    let provider_guard = mlx_provider.lock().await;
    if let Some(provider) = provider_guard.as_ref() {
        let model_path = llm_manager.model_path(&model_id);
        provider.switch_model(model_path).await?;
    }
    
    Ok(())
}
```

**Events Emitted:**
- `llm-model-download-progress`: `{model_id: string, progress: number, downloaded_mb: number}`
- `llm-model-download-complete`: `{model_id: string}`
- `llm-model-download-error`: `{model_id: string, error: string}`

**Managed State Setup:**

The system maintains two separate pieces of managed state:
1. `Arc<dyn IntelProvider>` - The active provider (trait object)
2. `Arc<Mutex<Option<MlxProvider>>>` - Direct reference to MlxProvider if it's active

This allows `switch_llm_model` to access MlxProvider directly without downcasting, preserving trait abstraction for all other code that uses `IntelProvider`.

## Data Models


### NDJSON Protocol Messages

**Python Sidecar Commands:**

```typescript
// Check availability
{
  command: "check-availability"
}

// Load model
{
  command: "load-model",
  model_path: string
}

// Generate tags
{
  command: "generate-tags",
  content: string
}

// Summarize
{
  command: "summarize",
  content: string
}

// Download model
{
  command: "download-model",
  repo_id: string,
  target_dir: string
}

// Get model info
{
  command: "model-info"
}

// Shutdown
{
  command: "shutdown"
}
```

**Python Sidecar Responses:**

```typescript
// Success responses
{
  ok: true,
  available?: boolean,
  reason?: string,
  loaded?: boolean,
  model?: string,
  result?: string | string[],
  progress?: number,
  downloaded_mb?: number,
  complete?: boolean,
  params?: string
}

// Error responses
{
  ok: false,
  error: string
}
```

### Frontend Data Models

**TypeScript Interfaces:**

```typescript
interface LlmModelInfo {
  id: string;
  display_name: string;
  repo_id: string;
  description: string;
  size_estimate: string;
  quality_tier: 'basic' | 'good' | 'great' | 'best';
  status: ModelStatus;
}

type ModelStatus =
  | { type: 'downloaded'; size_bytes: number }
  | { type: 'downloading'; progress: number }
  | { type: 'error'; message: string }
  | { type: 'not_downloaded' };

interface IntelligenceSettings {
  provider: 'mlx' | 'intelligencekit' | 'api';
  active_model: string;
  python_path: string;
}
```

## Correctness Properties


*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: NDJSON Protocol Round Trip

*For any* valid NDJSON command sent to the Python sidecar, the response SHALL be valid NDJSON that can be parsed without error, and the sidecar SHALL remain responsive for subsequent commands.

**Validates: Requirements 1.1, 1.12**

### Property 2: Model Loading State Consistency

*For any* sequence of `load-model` commands (valid or invalid), the sidecar SHALL maintain a consistent state where either a model is loaded (and its name is known) or no model is loaded, and this state SHALL be reflected accurately in responses to `generate-tags`, `summarize`, and `model-info` commands.

**Validates: Requirements 1.3, 1.4, 1.7**

### Property 3: Content Chunking Preserves Information

*For any* content string, when split into chunks by `MlxProvider`, the concatenation of all chunks SHALL equal the original content (no data loss), and chunk boundaries SHALL occur at valid UTF-8 character boundaries.

**Validates: Requirements 3.5, 3.6**

### Property 4: Tag Deduplication is Case-Insensitive

*For any* set of tags returned from multiple content chunks, the final deduplicated tag list SHALL contain at most one tag per case-insensitive string, and SHALL contain at most 5 tags total.

**Validates: Requirements 3.5**

### Property 5: Model Download Atomicity

*For any* model download operation, the model directory SHALL either be fully present in `~/.jarvis/models/llm/<model_id>/` with a valid `config.json` file, or not present at all—partial downloads SHALL only exist in the `.downloads/` temporary directory.

**Validates: Requirements 2.12**


### Property 6: Download Cancellation Cleanup

*For any* in-progress model download that is cancelled, the system SHALL remove all partial files from the `.downloads/` directory and remove the model from the download queue, leaving no orphaned state.

**Validates: Requirements 2.7**

### Property 7: Active Model Protection

*For any* model that is currently set as the active model in settings, attempting to delete that model SHALL return an error and the model directory SHALL remain unchanged.

**Validates: Requirements 2.9**

### Property 8: Settings Validation Rejects Invalid Providers

*For any* settings object where `intelligence.provider` is not one of `"mlx"`, `"intelligencekit"`, or `"api"`, the validation function SHALL return an error and settings SHALL not be persisted.

**Validates: Requirements 5.5**

### Property 9: Provider Fallback Chain

*For any* provider initialization failure, the system SHALL attempt the next provider in the fallback chain (MLX → IntelligenceKit → NoOp) and SHALL eventually return a valid provider instance (even if it's NoOpProvider).

**Validates: Requirements 4.2**

### Property 10: Model Switch Preserves Previous State on Failure

*For any* `switch_model()` call that fails (invalid path, load error), the MlxProvider SHALL remain loaded with the previous model, and subsequent `generate_tags()` or `summarize()` calls SHALL continue to work with the previous model.

**Validates: Requirements 3.10**

### Property 11: Backward Compatibility with Missing Intelligence Settings

*For any* existing `settings.json` file that does not contain an `intelligence` key, loading the settings SHALL succeed and SHALL use the default `IntelligenceSettings` values without error.

**Validates: Requirements 5.4**

### Property 12: Model List Completeness

*For any* call to `list_llm_models()`, the returned list SHALL contain exactly one `LlmModelInfo` entry for each model in the `LLM_MODEL_CATALOG`, with status accurately reflecting the current state (Downloaded/Downloading/Error/NotDownloaded).

**Validates: Requirements 2.4**


### Property 13: Concurrent Download Prevention

*For any* model ID, if a download is already in progress for that model, subsequent calls to `download_llm_model()` with the same model ID SHALL return an error without spawning a new download task.

**Validates: Requirements 2.6**

### Property 14: Sidecar Process Death Detection

*For any* MlxProvider instance where the sidecar process has terminated, the next command attempt SHALL detect the broken pipe and return an error rather than hanging indefinitely.

**Validates: Requirements 3.7**

### Property 15: IntelProvider Trait Compatibility

*For any* existing code that uses the `IntelProvider` trait (such as `enrich_gem` command), switching from IntelligenceKitProvider to MlxProvider SHALL require no changes to that code—the trait abstraction SHALL remain intact.

**Validates: Requirements 6.5**

## Error Handling

### Python Sidecar Errors

**Initialization Errors:**
- Python not found → Return error from `MlxProvider::new()`, fall back to IntelligenceKit
- `mlx_lm` not installed → `check-availability` returns `available: false`, fall back
- Invalid model path → `load-model` returns error, sidecar remains in previous state

**Runtime Errors:**
- Malformed JSON → Respond with error, continue listening (don't crash)
- No model loaded → Return `{"ok":false,"error":"No model loaded"}`
- Model inference failure → Return error with descriptive message
- Download failure → Return error, clean up partial files

**Process Errors:**
- Sidecar crash → Next command detects broken pipe, returns error
- Timeout (60s) → Return timeout error, consider sidecar unhealthy

### Rust Backend Errors

**Model Management Errors:**
- Model not in catalog → Return error immediately
- Model already downloading → Return error without spawning task
- Delete active model → Return error without deleting
- Model not downloaded → Return error on switch attempt


**Settings Errors:**
- Invalid provider value → Validation error, settings not persisted
- Empty active_model → Validation error
- Empty python_path → Validation error

**Provider Selection Errors:**
- All providers fail → Use NoOpProvider (graceful degradation)
- Initialization timeout (15s) → Log warning, fall back to next provider

### Error Recovery Strategies

**Download Failures:**
1. Clean up partial files from `.downloads/`
2. Set model status to `Error` with message
3. Emit `llm-model-download-error` event
4. Allow user to retry

**Sidecar Crashes:**
1. Detect broken pipe on next command
2. Return error to caller
3. Log error with context
4. Frontend shows toast notification
5. User can restart app or switch providers

**Missing Model Directory:**
1. Detect during provider initialization
2. Log warning
3. Fall back through provider chain
4. Frontend shows message to download model

## Testing Strategy

### Unit Tests

**Python Sidecar (`server.py`):**
- Test each command handler in isolation
- Mock `mlx_lm` module for testing without GPU
- Test malformed JSON handling
- Test error conditions (no model loaded, invalid paths)
- Test `/no_think` suffix is appended to prompts

**LlmModelManager:**
- Test catalog lookup
- Test model path resolution
- Test validation logic (config.json check)
- Test download state tracking
- Test concurrent download prevention
- Test active model deletion prevention

**MlxProvider:**
- Test content chunking at various sizes
- Test tag deduplication (case-insensitive)
- Test multi-chunk summary combination
- Test timeout enforcement
- Test error handling for sidecar failures

**Settings:**
- Test validation rules
- Test backward compatibility (missing intelligence key)
- Test default values
- Test persistence round-trip


### Property-Based Tests

All property tests should run with minimum 100 iterations and be tagged with references to the design document properties.

**Property 1: NDJSON Protocol Round Trip**
```rust
// Feature: mlx-intelligence-provider, Property 1: NDJSON Protocol Round Trip
#[test]
fn prop_ndjson_round_trip() {
    // Generate random valid commands
    // Send to sidecar
    // Verify response is valid NDJSON
    // Verify sidecar remains responsive
}
```

**Property 3: Content Chunking Preserves Information**
```rust
// Feature: mlx-intelligence-provider, Property 3: Content Chunking Preserves Information
#[test]
fn prop_chunking_preserves_content() {
    // Generate random content strings (various sizes)
    // Split into chunks
    // Verify concatenation equals original
    // Verify all chunk boundaries are valid UTF-8
}
```

**Property 4: Tag Deduplication**
```rust
// Feature: mlx-intelligence-provider, Property 4: Tag Deduplication is Case-Insensitive
#[test]
fn prop_tag_deduplication() {
    // Generate random tag lists with duplicates (varying case)
    // Apply deduplication
    // Verify no case-insensitive duplicates
    // Verify max 5 tags
}
```

**Property 5: Model Download Atomicity**
```rust
// Feature: mlx-intelligence-provider, Property 5: Model Download Atomicity
#[test]
fn prop_download_atomicity() {
    // Simulate various download scenarios (success, failure, cancel)
    // Verify model dir is either complete or absent
    // Verify no partial state in final location
}
```

**Property 9: Provider Fallback Chain**
```rust
// Feature: mlx-intelligence-provider, Property 9: Provider Fallback Chain
#[test]
fn prop_provider_fallback() {
    // Simulate various provider initialization failures
    // Verify fallback chain is followed
    // Verify a provider is always returned (even NoOp)
}
```

**Property 12: Model List Completeness**
```rust
// Feature: mlx-intelligence-provider, Property 12: Model List Completeness
#[test]
fn prop_model_list_completeness() {
    // Call list_llm_models() in various states
    // Verify one entry per catalog model
    // Verify status accuracy
}
```

### Integration Tests

**End-to-End Enrichment Flow:**
1. Start app with MLX provider configured
2. Create a gem
3. Call `enrich_gem` command
4. Verify tags and summary are generated
5. Verify gem is updated in database

**Model Management Flow:**
1. List models (all NotDownloaded)
2. Download a model
3. Monitor progress events
4. Verify completion event
5. Verify model directory exists with config.json
6. Switch to downloaded model
7. Verify settings updated
8. Delete model
9. Verify directory removed

**Provider Fallback Flow:**
1. Configure MLX provider with invalid python path
2. Start app
3. Verify fallback to IntelligenceKit
4. Verify enrichment still works

### Manual Testing

**Model Download:**
- Download large model (Qwen 14B), verify progress updates
- Cancel mid-download, verify cleanup
- Download with network interruption, verify error handling

**Model Switching:**
- Download multiple models
- Switch between them
- Verify inference uses correct model

**Sidecar Robustness:**
- Kill sidecar process manually
- Verify next command fails gracefully
- Restart app, verify recovery

**Settings UI:**
- Switch providers
- Download/delete models
- Verify UI state updates correctly

## Implementation Notes


### Python Sidecar Implementation

**Dependencies (`requirements.txt`):**
```
mlx>=0.0.9
mlx-lm>=0.0.9
huggingface-hub>=0.20.0
```

**Key Implementation Details:**

1. **Platform Check:**
```python
import platform
if platform.machine() != "arm64":
    sys.stderr.write("Error: MLX requires Apple Silicon (arm64)\n")
    sys.exit(1)
```

2. **Prompt Engineering:**
```python
# For tags
prompt = f"Generate 3-5 topic tags (1-3 words each) for this content. Return as JSON array./no_think\n\n{content}"

# For summary
prompt = f"Summarize this content in one sentence (max 100 words)./no_think\n\n{content}"
```

3. **Progress Reporting:**
```python
# During download, emit progress every 1% or 10MB
def progress_callback(downloaded, total):
    progress = (downloaded / total) * 100
    print(json.dumps({
        "ok": True,
        "progress": progress,
        "downloaded_mb": downloaded / (1024 * 1024)
    }))
    sys.stdout.flush()
```

### Rust Backend Implementation

**Provider Selection Logic (`intelligence/mod.rs`):**

```rust
pub async fn create_provider(
    app_handle: AppHandle,
    settings: &Settings,
    llm_manager: &LlmModelManager,
) -> (Arc<dyn IntelProvider>, Arc<Mutex<Option<Arc<MlxProvider>>>>) {
    let provider_name = &settings.intelligence.provider;
    
    match provider_name.as_str() {
        "mlx" => {
            // Resolve model path and python path from settings
            let model_path = llm_manager.model_path(&settings.intelligence.active_model);
            let python_path = settings.intelligence.python_path.clone();
            
            match MlxProvider::new(app_handle.clone(), model_path, python_path).await {
                Ok(provider) => {
                    eprintln!("Intelligence: Using MlxProvider with model {}", 
                             settings.intelligence.active_model);
                    let provider = Arc::new(provider);
                    let mlx_ref = Arc::new(Mutex::new(Some(Arc::clone(&provider))));
                    let trait_obj: Arc<dyn IntelProvider> = provider;
                    return (trait_obj, mlx_ref);
                }
                Err(e) => {
                    eprintln!("Intelligence: MLX initialization failed: {}", e);
                    eprintln!("Intelligence: Falling back to IntelligenceKit");
                    // Fall through to IntelligenceKit
                }
            }
            
            match IntelligenceKitProvider::new(app_handle.clone()).await {
                Ok(provider) => {
                    eprintln!("Intelligence: Using IntelligenceKitProvider");
                    return (Arc::new(provider), Arc::new(Mutex::new(None)));
                }
                Err(e) => {
                    eprintln!("Intelligence: IntelligenceKit failed: {}", e);
                    eprintln!("Intelligence: Using NoOpProvider");
                    return (
                        Arc::new(NoOpProvider::new(format!("MLX and IntelligenceKit failed: {}", e))),
                        Arc::new(Mutex::new(None))
                    );
                }
            }
        }
        "intelligencekit" => {
            match IntelligenceKitProvider::new(app_handle.clone()).await {
                Ok(provider) => {
                    eprintln!("Intelligence: Using IntelligenceKitProvider");
                    return (Arc::new(provider), Arc::new(Mutex::new(None)));
                }
                Err(e) => {
                    eprintln!("Intelligence: IntelligenceKit failed: {}", e);
                    eprintln!("Intelligence: Using NoOpProvider");
                    return (
                        Arc::new(NoOpProvider::new(format!("IntelligenceKit failed: {}", e))),
                        Arc::new(Mutex::new(None))
                    );
                }
            }
        }
        "api" => {
            eprintln!("Intelligence: API provider not yet implemented");
            (
                Arc::new(NoOpProvider::new("API provider not yet implemented".to_string())),
                Arc::new(Mutex::new(None))
            )
        }
        _ => {
            eprintln!("Intelligence: Unknown provider '{}', defaulting to MLX", 
                     provider_name);
            // Recurse with "mlx"
            let mut modified_settings = settings.clone();
            modified_settings.intelligence.provider = "mlx".to_string();
            create_provider(app_handle, &modified_settings, llm_manager).await
        }
    }
}
```

**Key Points:**
- `create_provider` now takes `llm_manager: &LlmModelManager` as a parameter to resolve model paths
- MLX provider initialization directly calls `MlxProvider::new()` with resolved `model_path` and `python_path`
- No helper function needed - settings are resolved inline before calling `MlxProvider::new()`
- `MlxProvider` is wrapped in `Arc` once: `Arc::new(provider)`
- The same `Arc<MlxProvider>` is cloned for both the trait object and the direct reference
- `mlx_ref` type is `Arc<Mutex<Option<Arc<MlxProvider>>>>` - stores an Arc-wrapped provider
- No `Clone` trait needed on `MlxProvider`
- Both managed state entries share the same underlying instance via Arc


**Model Path Resolution:**

```rust
impl LlmModelManager {
    pub fn model_path(&self, model_id: &str) -> PathBuf {
        // Find catalog entry to get repo_id
        let entry = Self::LLM_MODEL_CATALOG
            .iter()
            .find(|e| e.id == model_id)
            .expect("Model ID must be in catalog");
        
        // Extract model directory name from repo_id
        // "mlx-community/Qwen3-8B-4bit" -> "Qwen3-8B-4bit"
        let model_dir = entry.repo_id
            .split('/')
            .last()
            .unwrap_or(model_id);
        
        self.models_dir.join(model_dir)
    }
    
    fn validate_model(&self, model_id: &str) -> bool {
        let model_path = self.model_path(model_id);
        let config_path = model_path.join("config.json");
        config_path.exists()
    }
}
```

**Content Chunking Helper:**

```rust
// Extracted to src/intelligence/utils.rs for reuse by both providers

pub fn split_content(content: &str, max_chars: usize) -> Vec<&str> {
    if content.len() <= max_chars {
        return vec![content];
    }
    
    let mut chunks = Vec::new();
    let mut start = 0;
    
    while start < content.len() {
        if start + max_chars >= content.len() {
            chunks.push(&content[start..]);
            break;
        }
        
        let end = snap_to_char_boundary(content, start + max_chars);
        
        // Try paragraph boundary
        let search_start = snap_to_char_boundary(
            content, 
            if end > start + 500 { end - 500 } else { start }
        );
        
        let break_pos = content[search_start..end]
            .rfind("\n\n")
            .map(|pos| search_start + pos + 2)
            .or_else(|| {
                content[search_start..end]
                    .rfind('\n')
                    .map(|pos| search_start + pos + 1)
            })
            .or_else(|| {
                content[search_start..end]
                    .rfind(' ')
                    .map(|pos| search_start + pos + 1)
            })
            .unwrap_or(end);
        
        chunks.push(&content[start..break_pos]);
        start = break_pos;
    }
    
    chunks
}

fn snap_to_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    let mut i = index;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}
```

**Usage in MlxProvider:**

```rust
use crate::intelligence::utils::split_content;

const MAX_CONTENT_CHARS: usize = 15_000;

// In generate_tags() or summarize()
let chunks = split_content(content, MAX_CONTENT_CHARS);
```

**Note:** This utility is also used by `IntelligenceKitProvider` with `MAX_CONTENT_CHARS = 10_000`. Extracting it to `intelligence/utils.rs` eliminates code duplication and ensures consistent chunking behavior across providers.

### Frontend Implementation

**Settings Component Structure:**

```tsx
// src/components/Settings.tsx
function IntelligenceSettings() {
  const [provider, setProvider] = useState<string>('mlx');
  const [models, setModels] = useState<LlmModelInfo[]>([]);
  const [activeModel, setActiveModel] = useState<string>('');
  
  useEffect(() => {
    loadModels();
  }, []);
  
  useEffect(() => {
    const unlisten = listen('llm-model-download-progress', (event) => {
      updateModelProgress(event.payload);
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);
  
  async function loadModels() {
    const result = await invoke('list_llm_models');
    setModels(result);
  }
  
  async function downloadModel(modelId: string) {
    await invoke('download_llm_model', { modelId });
  }
  
  async function switchModel(modelId: string) {
    await invoke('switch_llm_model', { modelId });
    setActiveModel(modelId);
  }
  
  return (
    <div className="intelligence-settings">
      <h3>Intelligence Provider</h3>
      <ProviderSelector value={provider} onChange={setProvider} />
      
      {provider === 'mlx' && (
        <ModelList
          models={models}
          activeModel={activeModel}
          onDownload={downloadModel}
          onSwitch={switchModel}
          onDelete={deleteModel}
        />
      )}
    </div>
  );
}
```


**Model Card Component:**

```tsx
interface ModelCardProps {
  model: LlmModelInfo;
  isActive: boolean;
  onDownload: () => void;
  onSwitch: () => void;
  onDelete: () => void;
  onCancel: () => void;
}

function ModelCard({ model, isActive, ...handlers }: ModelCardProps) {
  const renderActions = () => {
    switch (model.status.type) {
      case 'not_downloaded':
        return <Button onClick={handlers.onDownload}>Download</Button>;
      
      case 'downloading':
        return (
          <>
            <ProgressBar progress={model.status.progress} />
            <Button onClick={handlers.onCancel}>Cancel</Button>
          </>
        );
      
      case 'downloaded':
        return isActive ? (
          <Badge>Active</Badge>
        ) : (
          <>
            <Button onClick={handlers.onSwitch}>Set Active</Button>
            <Button onClick={handlers.onDelete} variant="danger">Delete</Button>
          </>
        );
      
      case 'error':
        return (
          <>
            <ErrorMessage>{model.status.message}</ErrorMessage>
            <Button onClick={handlers.onDownload}>Retry</Button>
          </>
        );
    }
  };
  
  return (
    <div className="model-card">
      <div className="model-header">
        <h4>{model.display_name}</h4>
        <QualityBadge tier={model.quality_tier} />
      </div>
      <p className="model-description">{model.description}</p>
      <p className="model-size">{model.size_estimate}</p>
      <div className="model-actions">{renderActions()}</div>
    </div>
  );
}
```

## Security Considerations

### Python Execution

**Risk:** Executing arbitrary Python code
**Mitigation:**
- Python path is configurable but defaults to system `python3`
- Sidecar script is bundled with the app (not user-provided)
- No dynamic code execution in sidecar
- Sidecar runs with same privileges as main app (no elevation)

### Model Downloads

**Risk:** Downloading malicious models from HuggingFace
**Mitigation:**
- Catalog is hardcoded (only trusted `mlx-community` repos)
- No user-provided repo IDs accepted
- Models are validated (config.json must exist)
- Downloads use HTTPS

### File System Access

**Risk:** Path traversal or unauthorized file access
**Mitigation:**
- All model paths are resolved relative to `~/.jarvis/models/llm/`
- No user-provided paths accepted for model storage
- Temporary downloads isolated in `.downloads/` subdirectory

### Process Management

**Risk:** Sidecar process leaks or zombie processes
**Mitigation:**
- Sidecar has explicit shutdown command
- Process handle is tracked and cleaned up on app exit
- Timeouts prevent indefinite hangs

## Performance Considerations

### Model Loading Time

- Qwen 8B (4-bit): ~5-10 seconds to load on M1/M2, ~3-5 seconds on M3 Pro
- Llama 3.2 3B (4-bit): ~2-3 seconds on M1/M2, ~1-2 seconds on M3 Pro
- Loading happens once at startup or model switch
- Use 15-second timeout for initialization to accommodate slower hardware

### Inference Time

**M3 Pro (36GB) - measured:**
- Qwen 8B (4-bit): ~30 tok/s → 200 tokens in ~7 seconds, 150 tokens in ~5 seconds
- Llama 3.2 3B (4-bit): ~60 tok/s → 200 tokens in ~3 seconds, 150 tokens in ~2.5 seconds

**M1/M2 (estimated):**
- Tags (200 tokens): ~10-15 seconds with 8B model, ~5-7 seconds with 3B model
- Summary (150 tokens): ~7-10 seconds with 8B model, ~4-5 seconds with 3B model

**General:**
- Use 60-second timeout (2x IntelligenceKit's 30s)
- Chunking large content may take longer (multiple inference passes)

### Memory Usage

- Qwen 8B (4-bit): ~6 GB RAM
- Llama 3.2 3B (4-bit): ~3 GB RAM
- Only one model loaded at a time
- Model stays resident until app exit or model switch

### Download Bandwidth

- Models range from 2 GB to 9 GB
- Progress updates every 1% to avoid event spam
- Downloads are resumable (HuggingFace Hub handles this)

## Deployment Considerations

### Python Dependencies

**User Responsibility:**
- Users must install Python 3.10+ manually
- Users must install MLX dependencies: `pip install mlx mlx-lm huggingface-hub`
- App provides clear error messages if dependencies missing

**Future Enhancement:**
- Bundle Python with the app (pyinstaller or similar)
- Auto-install dependencies on first run

### Disk Space

- Models directory can grow large (up to 50+ GB with all models)
- Users should have at least 20 GB free space
- Settings UI should show total disk usage for models

### macOS Permissions

- No special permissions required (unlike ScreenCaptureKit)
- Models stored in user's home directory
- No sandboxing issues

## Migration Path

### Existing Users

1. App update includes new intelligence settings with defaults
2. Existing `settings.json` files load successfully (backward compatible)
3. Default provider is "mlx" but falls back to IntelligenceKit if unavailable
4. No data migration needed (gems schema unchanged)

### First-Time Setup

1. User opens Settings → Intelligence
2. Sees "No model downloaded" message
3. Downloads recommended model (Qwen 8B 4-bit)
4. Model downloads in background with progress
5. On completion, enrichment becomes available

## Future Enhancements

### Out of Scope for This Iteration

1. **Cloud API Provider:** Placeholder exists, implementation deferred
2. **Streaming Inference:** Token-by-token output to UI
3. **Custom Models:** User-provided model paths
4. **Fine-tuning:** Training custom models
5. **Automatic Python Installation:** Bundled Python runtime
6. **Model Quantization:** On-device quantization of full-precision models
7. **Multi-model Ensemble:** Using multiple models for better results

### Potential Future Work

- **Model Recommendations:** Suggest models based on hardware (RAM, GPU)
- **Inference Caching:** Cache results for identical content
- **Batch Processing:** Enrich multiple gems in parallel
- **Model Benchmarking:** Show inference speed/quality metrics
- **Custom Prompts:** User-configurable prompts for tags/summaries

