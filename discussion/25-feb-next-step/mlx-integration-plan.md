# MLX Integration Plan — Replacing IntelligenceKit with MLX-LM

## Current State

Jarvis already has the perfect architecture for this:

```
Frontend (React)
    │
    │  invoke('enrich_gem', ...)
    │  invoke('check_intel_availability', ...)
    ▼
commands.rs
    │
    │  intel_provider.generate_tags(content)
    │  intel_provider.summarize(content)
    ▼
IntelProvider trait  ← THE INTERFACE (provider.rs)
    │
    ├── IntelligenceKitProvider  ← Current: Swift sidecar → Apple Foundation Models (3B)
    ├── NoOpProvider             ← Fallback: returns errors gracefully
    └── ??? MlxProvider          ← NEW: Python sidecar → MLX-LM (switchable models)
```

**Nothing in commands.rs or the frontend needs to change.** We just add a new `IntelProvider` implementation + a model manager.

---

## The Interface (Already Exists)

```rust
// provider.rs — this stays EXACTLY as-is
#[async_trait]
pub trait IntelProvider: Send + Sync {
    async fn check_availability(&self) -> AvailabilityResult;
    async fn generate_tags(&self, content: &str) -> Result<Vec<String>, String>;
    async fn summarize(&self, content: &str) -> Result<String, String>;
}
```

Any backend that can check readiness, generate tags, and summarize — plugs right in.

---

## What We Build

### 1. LLM Model Manager (Mirrors Whisper's ModelManager)

New file: `src/intelligence/llm_model_manager.rs`

Follows the **exact same pattern** as `settings/model_manager.rs` (Whisper models):

```rust
/// Static catalog of available LLM models
const LLM_MODEL_CATALOG: &[LlmModelEntry] = &[
    LlmModelEntry {
        id: "qwen3-8b-4bit",
        repo_id: "mlx-community/Qwen3-8B-4bit",
        display_name: "Qwen 3 8B (Q4)",
        description: "Best overall quality. Great for summarization and tagging. Recommended.",
        size_estimate: "~5 GB",
        quality_tier: "great",
        context_window: 8192,
        speed_tier: "moderate",    // ~30 tok/s on M3 Pro
    },
    LlmModelEntry {
        id: "llama-3.2-3b-4bit",
        repo_id: "mlx-community/Llama-3.2-3B-Instruct-4bit",
        display_name: "Llama 3.2 3B (Q4)",
        description: "Fast and lightweight. Good for simple tasks.",
        size_estimate: "~2 GB",
        quality_tier: "good",
        context_window: 8192,
        speed_tier: "fast",        // ~60 tok/s on M3 Pro
    },
    LlmModelEntry {
        id: "qwen3-4b-4bit",
        repo_id: "mlx-community/Qwen3-4B-Instruct-4bit",
        display_name: "Qwen 3 4B (Q4)",
        description: "Balanced speed and quality. Good middle ground.",
        size_estimate: "~2.5 GB",
        quality_tier: "good",
        context_window: 8192,
        speed_tier: "fast",
    },
    LlmModelEntry {
        id: "qwen3-14b-4bit",
        repo_id: "mlx-community/Qwen3-14B-4bit",
        display_name: "Qwen 3 14B (Q4)",
        description: "Highest quality. Needs 36GB+ RAM. Slower inference.",
        size_estimate: "~9 GB",
        quality_tier: "best",
        context_window: 8192,
        speed_tier: "slow",        // ~15 tok/s on M3 Pro
    },
];
```

**What it does (same as Whisper ModelManager):**

| Feature | Whisper ModelManager | LLM ModelManager |
|---------|---------------------|------------------|
| Catalog | `MODEL_CATALOG` (static entries) | `LLM_MODEL_CATALOG` (static entries) |
| Storage | `~/.jarvis/models/` | `~/.jarvis/models/llm/` |
| List models | `list_models()` → `Vec<ModelInfo>` | `list_llm_models()` → `Vec<LlmModelInfo>` |
| Download | `download_model()` with progress events | `download_llm_model()` with progress events |
| Cancel | `cancel_download()` | `cancel_llm_download()` |
| Delete | `delete_model()` | `delete_llm_model()` |
| Validate | `validate_ggml_file()` (magic bytes) | `validate_llm_model()` (check config.json exists) |
| Active model | Via `settings.transcription.whisper_model` | Via `settings.intelligence.active_model` |
| Status enum | `Downloaded / Downloading / Error / NotDownloaded` | Same `ModelStatus` enum (reuse!) |
| Progress events | `model-download-progress` | `llm-model-download-progress` |

**Download mechanism:**
- Whisper uses `reqwest` to download single `.bin` files from HuggingFace
- LLM uses `huggingface_hub` snapshot_download via the Python sidecar (models are multi-file: config.json + tokenizer.json + model-*.safetensors)
- Alternative: Rust calls `huggingface-hub` crate directly, or shells out to `huggingface-cli download`

### 2. Python MLX Server (Sidecar)

A small Python script that:
- Receives model path as argument
- Loads the specified model on startup
- Listens on stdin for NDJSON commands
- Responds on stdout with NDJSON responses
- Can be restarted with a different model (via `load-model` command)

```
jarvis-app/src-tauri/sidecars/
└── mlx-server/
    ├── server.py          # NDJSON stdin/stdout server
    ├── requirements.txt   # mlx, mlx-lm
    └── README.md
```

**NDJSON Protocol:**

```json
// Rust → Python
{"command":"check-availability"}
{"command":"load-model","model_path":"/Users/.../.jarvis/models/llm/Qwen3-8B-4bit"}
{"command":"generate-tags","content":"Rust is a programming language..."}
{"command":"summarize","content":"Rust is a programming language..."}
{"command":"model-info"}
{"command":"shutdown"}

// Python → Rust
{"ok":true,"available":true,"model":"Qwen3-8B-4bit","params":"8.0B"}
{"ok":true,"loaded":true,"model":"Qwen3-8B-4bit"}
{"ok":true,"result":["Rust","Programming","Systems"]}
{"ok":true,"result":"Rust is a systems programming language..."}
{"ok":true,"model":"Qwen3-8B-4bit","params":"8,030,261,248","vocab":151936}
{"ok":true}
```

Key addition: **`load-model` command** — lets Rust tell the Python server to switch models without restarting the process. This enables hot-swapping from the settings UI.

### 3. Rust MlxProvider

New file: `src/intelligence/mlx_provider.rs`

Almost identical to `intelligencekit_provider.rs` but:
- Spawns Python instead of Swift binary
- Sends tag/summary prompts formatted for Qwen (with `/no_think`)
- Larger context window (8K+ tokens vs 4K) — bigger content chunks
- Has `switch_model(model_path)` method that sends `load-model` command
- No session management needed (stateless per-request)

### 4. Provider Selection (in mod.rs)

```rust
pub async fn create_provider(app_handle: tauri::AppHandle, settings: &Settings) -> Arc<dyn IntelProvider> {
    match settings.intelligence.provider.as_str() {
        "mlx" => {
            let model_path = resolve_active_model_path(&settings.intelligence);
            match MlxProvider::new(app_handle, &model_path).await {
                Ok(provider) => Arc::new(provider),
                Err(e) => {
                    eprintln!("MLX provider failed: {}, trying IntelligenceKit", e);
                    try_intelligencekit(app_handle).await
                }
            }
        }
        "intelligencekit" => try_intelligencekit(app_handle).await,
        "api" => Arc::new(NoOpProvider::new("API provider not yet implemented".into())),
        _ => try_intelligencekit(app_handle).await,
    }
}
```

---

## Settings Changes

### Current Settings Structure
```rust
pub struct Settings {
    pub transcription: TranscriptionSettings,
    pub browser: BrowserSettings,
}
```

### New Settings Structure
```rust
pub struct Settings {
    pub transcription: TranscriptionSettings,
    pub browser: BrowserSettings,
    #[serde(default)]
    pub intelligence: IntelligenceSettings,  // NEW
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceSettings {
    pub provider: String,       // "mlx" | "intelligencekit" | "api"
    pub active_model: String,   // "qwen3-8b-4bit" (catalog ID)
    pub python_path: String,    // "python3" or "/path/to/venv/bin/python"
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
```

### Settings JSON on disk
```json
{
  "transcription": { ... },
  "browser": { ... },
  "intelligence": {
    "provider": "mlx",
    "active_model": "qwen3-8b-4bit",
    "python_path": "python3"
  }
}
```

---

## New Tauri Commands

Mirror the Whisper model commands:

```rust
// LLM model management (mirrors Whisper model commands)
commands::list_llm_models,          // → Vec<LlmModelInfo> with download status
commands::download_llm_model,       // → starts async download, emits progress events
commands::cancel_llm_download,      // → cancels in-progress download
commands::delete_llm_model,         // → removes model from disk
commands::switch_llm_model,         // → changes active model, tells MLX server to reload

// Existing (unchanged)
commands::check_intel_availability, // → still works, now returns MLX model info
commands::enrich_gem,               // → still works, uses whatever provider is active
```

---

## Frontend Changes

### Settings Panel — New "Intelligence" Section

Mirrors the existing Whisper model selector UI:

```
┌─────────────────────────────────────────────────┐
│ Intelligence Provider                           │
│                                                 │
│ ○ Local MLX (Recommended)                       │
│ ○ Apple Intelligence Kit                        │
│ ○ Cloud API (coming soon)                       │
│                                                 │
│ ─────────────────────────────────────────────── │
│                                                 │
│ Available Models              Active: Qwen 8B   │
│                                                 │
│ ┌─────────────────────────────────────────────┐ │
│ │ ★ Qwen 3 8B (Q4)              ~5 GB  great │ │
│ │   Best overall quality. Recommended.        │ │
│ │   [Downloaded ✓] [● Active]                 │ │
│ ├─────────────────────────────────────────────┤ │
│ │   Llama 3.2 3B (Q4)           ~2 GB  good  │ │
│ │   Fast and lightweight.                     │ │
│ │   [Download]                                │ │
│ ├─────────────────────────────────────────────┤ │
│ │   Qwen 3 4B (Q4)              ~2.5 GB good │ │
│ │   Balanced speed and quality.               │ │
│ │   [Download]                                │ │
│ ├─────────────────────────────────────────────┤ │
│ │   Qwen 3 14B (Q4)             ~9 GB  best  │ │
│ │   Highest quality. Needs 36GB+ RAM.         │ │
│ │   [Download]                                │ │
│ └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
```

**Interactions (same as Whisper model UI):**
- Click "Download" → progress bar appears, emits `llm-model-download-progress` events
- Click "Cancel" during download → stops and cleans up
- Click "Active" on a downloaded model → switches active model
- Click "Delete" on downloaded (non-active) model → removes from disk
- Download progress bar shows MB downloaded / total

### GemsPanel — No Changes
Already shows tags and summary from `ai_enrichment`. Doesn't care which provider generated them.

---

## Model Storage Layout

```
~/.jarvis/
├── gems.db
├── settings.json
└── models/
    ├── ggml-base.en.bin              # Existing: Whisper models (flat files)
    ├── ggml-large-v3-turbo-q5_0.bin
    ├── whisperkit/                    # Existing: WhisperKit models
    └── llm/                           # NEW: LLM models
        ├── Qwen3-8B-4bit/            # Each model is a directory
        │   ├── config.json
        │   ├── tokenizer.json
        │   ├── tokenizer_config.json
        │   └── model-00001-of-00002.safetensors
        ├── Llama-3.2-3B-Instruct-4bit/
        │   └── ...
        └── .downloads/               # Temp dir for in-progress downloads
```

---

## Download Mechanism

Two options for downloading HuggingFace models:

### Option A: Python sidecar handles download (Simpler)
The MLX server has a `download-model` command:
```json
{"command":"download-model","repo_id":"mlx-community/Qwen3-8B-4bit","target_dir":"~/.jarvis/models/llm/Qwen3-8B-4bit"}
```
Python uses `huggingface_hub.snapshot_download()` and reports progress on stdout.
Rust just forwards events to the frontend.

**Pro:** Already works (we used this in the notebook). **Con:** Requires Python to be available for downloads too.

### Option B: Rust handles download directly
Use the `hf-hub` Rust crate or shell out to `huggingface-cli download`.

**Pro:** No Python dependency for downloads. **Con:** More Rust code, different download pattern.

**Recommendation:** Option A. Python is already required for inference, so it's available for downloads too. Keep it simple.

---

## What Changes vs What Stays the Same

| Component | Changes? | Details |
|-----------|----------|---------|
| `provider.rs` (trait) | NO | Interface stays identical |
| `commands.rs` | ADD | New LLM model management commands |
| `lib.rs` | SMALL | Init LlmModelManager + provider selection |
| `mod.rs` | YES | New provider factory with MLX option |
| `intelligencekit_provider.rs` | NO | Stays as fallback option |
| `noop_provider.rs` | NO | Stays as final fallback |
| `settings/manager.rs` | SMALL | Add `IntelligenceSettings` + validation |
| Frontend (GemsPanel) | NO | Still calls same Tauri commands |
| Frontend (Settings) | ADD | New intelligence section with model list |
| **NEW: `llm_model_manager.rs`** | NEW | ~300 lines, mirrors Whisper ModelManager |
| **NEW: `mlx_provider.rs`** | NEW | ~200 lines, mirrors IK provider |
| **NEW: `mlx-server/server.py`** | NEW | ~200 lines, NDJSON server + download |

---

## Implementation Order

| Step | What | Details |
|------|------|---------|
| 1 | Python MLX server | NDJSON server: load model, generate tags, summarize, download, model-info |
| 2 | `LlmModelManager` | Catalog, list/download/delete, mirrors Whisper pattern |
| 3 | `MlxProvider` | Spawn Python, NDJSON protocol, tag/summarize prompts |
| 4 | Settings + wiring | `IntelligenceSettings`, provider factory, new commands in lib.rs |
| 5 | Frontend settings | Model list UI, provider selector, download progress |
| 6 | Testing | Download model → switch → enrich gem → verify tags/summary |

---

## Fallback Chain

```
User selects "MLX" in settings
    │
    ├── Python found? MLX installed? Model downloaded?
    │   YES → MlxProvider (Qwen 8B, local, private, fast)
    │   NO  ↓
    │
    ├── IntelligenceKit binary available? macOS 26+?
    │   YES → IntelligenceKitProvider (Apple 3B, on-device)
    │   NO  ↓
    │
    └── NoOpProvider (app works, no AI enrichment)
        Frontend shows: "AI enrichment unavailable. Download a model in Settings."
```

---

## Why This Design

**Same pattern as Whisper** — you already have model download/switch for transcription. LLM models work the same way. Consistent UX.

**Swappable** — tomorrow add `"api"` provider (OpenAI/Anthropic). Write one file, add to settings. Zero changes to commands or frontend.

**Play with models** — download Qwen 8B, Llama 3B, Qwen 14B. Switch between them in settings. See which gives best tags/summaries for your content. Delete the ones you don't want.

**Privacy** — all local. Emails, meeting notes, conversations never leave the machine.
