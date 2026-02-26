# Requirements: MLX Intelligence Provider

## Introduction

Jarvis is a Tauri desktop app (Rust backend + React frontend) that captures content from various sources (YouTube, Medium, email, ChatGPT conversations) and stores them as "gems." Each gem can be enriched with AI-generated tags and a one-sentence summary via the `IntelProvider` trait.

Currently, AI enrichment uses `IntelligenceKitProvider` — a Swift sidecar that talks to Apple Foundation Models (3B, macOS 26+ only). This feature adds `MlxProvider`, a **new `IntelProvider` implementation** backed by a Python sidecar running [MLX-LM](https://github.com/ml-explore/mlx-lm) on Apple Silicon. It also adds an LLM model management system (download, switch, delete) that mirrors the existing Whisper `ModelManager` pattern.

### Key Architecture Context

- **`IntelProvider` trait** (`src/intelligence/provider.rs`): The swappable interface with three methods — `check_availability()`, `generate_tags(content)`, `summarize(content)`.
- **Existing implementations**: `IntelligenceKitProvider` (Swift sidecar, NDJSON over stdin/stdout) and `NoOpProvider` (graceful fallback).
- **Existing model management**: `ModelManager` (`src/settings/model_manager.rs`) manages Whisper model downloads with a static catalog, progress events, cancel/delete support, and `ModelStatus` enum.
- **Settings**: `SettingsManager` (`src/settings/manager.rs`) persists to `~/.jarvis/settings.json`. Currently has `transcription` and `browser` sections.
- **Sidecar pattern**: Rust spawns a child process, communicates via NDJSON over stdin/stdout with 30s timeouts.

### Goals

1. Users can run local LLM inference for gem enrichment using MLX on Apple Silicon.
2. Users can download, switch between, and delete LLM models — same UX as Whisper models.
3. The existing `IntelProvider` trait and all commands/frontend that use it remain unchanged.
4. If MLX is unavailable, the system falls back gracefully to IntelligenceKit, then NoOpProvider.
5. All inference is local and private — no data leaves the machine.

---

## Requirements

### Requirement 1: Python MLX Sidecar Server

**User Story:** As a developer, I want a Python sidecar that loads MLX-LM models and responds to NDJSON commands over stdin/stdout, so that the Rust backend can delegate LLM inference without embedding Python in the Rust process.

#### Acceptance Criteria

1. WHEN the sidecar is spawned with no arguments THEN THE SYSTEM SHALL start listening on stdin for NDJSON commands and respond on stdout with NDJSON responses, one JSON object per line.
2. WHEN the sidecar receives a `{"command":"check-availability"}` message THEN THE SYSTEM SHALL respond with `{"ok":true,"available":true}` if `mlx_lm` is importable, or `{"ok":true,"available":false,"reason":"..."}` if not.
3. WHEN the sidecar receives a `{"command":"load-model","model_path":"/path/to/model"}` message THEN THE SYSTEM SHALL load the model and tokenizer from the given local path using `mlx_lm.load()` and respond with `{"ok":true,"loaded":true,"model":"<model_dir_name>"}`.
4. WHEN `load-model` is called with a path that does not exist or does not contain a valid MLX model THEN THE SYSTEM SHALL respond with `{"ok":false,"error":"..."}` and remain in its previous state (no model loaded, or previous model still loaded).
5. WHEN the sidecar receives `{"command":"generate-tags","content":"..."}` and a model is loaded THEN THE SYSTEM SHALL generate 3-5 topic tags using the loaded model with `/no_think` appended to suppress Qwen thinking mode, parse the result as a JSON array, and respond with `{"ok":true,"result":["tag1","tag2",...]}`.
6. WHEN the sidecar receives `{"command":"summarize","content":"..."}` and a model is loaded THEN THE SYSTEM SHALL generate a one-sentence summary using the loaded model with `/no_think` appended, and respond with `{"ok":true,"result":"summary text"}`.
7. WHEN `generate-tags` or `summarize` is called with no model loaded THEN THE SYSTEM SHALL respond with `{"ok":false,"error":"No model loaded"}`.
8. WHEN the sidecar receives `{"command":"download-model","repo_id":"mlx-community/...","target_dir":"/path/to/dir"}` THEN THE SYSTEM SHALL download the model using `huggingface_hub.snapshot_download()` to the specified directory, emitting `{"ok":true,"progress":<0-100>,"downloaded_mb":<float>}` lines periodically, and `{"ok":true,"complete":true}` when finished.
9. WHEN a download fails or the repo_id is invalid THEN THE SYSTEM SHALL respond with `{"ok":false,"error":"..."}`.
10. WHEN the sidecar receives `{"command":"model-info"}` and a model is loaded THEN THE SYSTEM SHALL respond with `{"ok":true,"model":"<name>","params":"<count>"}`.
11. WHEN the sidecar receives `{"command":"shutdown"}` THEN THE SYSTEM SHALL respond with `{"ok":true}` and exit cleanly.
12. WHEN the sidecar encounters malformed JSON on stdin THEN THE SYSTEM SHALL respond with `{"ok":false,"error":"Invalid JSON: ..."}` and continue listening (not crash).
13. THE SYSTEM SHALL place the sidecar script at `src-tauri/sidecars/mlx-server/server.py` with a `requirements.txt` listing `mlx`, `mlx-lm`, and `huggingface-hub` as dependencies.
14. WHEN generating tags or summaries, THE SYSTEM SHALL use `max_tokens=200` for tags and `max_tokens=150` for summaries to bound inference time.

---

### Requirement 2: LLM Model Manager (Rust)

**User Story:** As a user, I want to download, list, switch between, and delete LLM models from a curated catalog, so that I can choose the best model for my hardware and quality needs.

#### Acceptance Criteria

1. THE SYSTEM SHALL define a static `LLM_MODEL_CATALOG` with at least these models:

   | ID | HuggingFace Repo | Display Name | Size Estimate | Quality |
   |----|-----------------|--------------|---------------|---------|
   | `qwen3-8b-4bit` | `mlx-community/Qwen3-8B-4bit` | Qwen 3 8B (Q4) | ~5 GB | great |
   | `llama-3.2-3b-4bit` | `mlx-community/Llama-3.2-3B-Instruct-4bit` | Llama 3.2 3B (Q4) | ~2 GB | good |
   | `qwen3-4b-4bit` | `mlx-community/Qwen3-4B-Instruct-4bit` | Qwen 3 4B (Q4) | ~2.5 GB | good |
   | `qwen3-14b-4bit` | `mlx-community/Qwen3-14B-4bit` | Qwen 3 14B (Q4) | ~9 GB | best |

2. THE SYSTEM SHALL store LLM models at `~/.jarvis/models/llm/<ModelDirName>/` where each model is a directory containing `config.json`, `tokenizer.json`, and `.safetensors` files.
3. THE SYSTEM SHALL create the `~/.jarvis/models/llm/` directory on `LlmModelManager::new()` if it does not exist.
4. WHEN `list_llm_models()` is called THEN THE SYSTEM SHALL return a `Vec<LlmModelInfo>` for every catalog entry with status: `Downloaded` (if directory exists and contains `config.json`), `Downloading` (if download is in progress with current progress percentage), `Error` (if previous download failed), or `NotDownloaded`.
5. WHEN `download_llm_model(model_id)` is called with a valid catalog ID THEN THE SYSTEM SHALL spawn a background task that sends a `download-model` command to the Python sidecar, emits `llm-model-download-progress` Tauri events with `{model_id, progress, downloaded_mb}`, and emits `llm-model-download-complete` on success.
6. WHEN `download_llm_model()` is called with a model that is already downloading THEN THE SYSTEM SHALL return an error `"Model <id> is already being downloaded"`.
7. WHEN `cancel_llm_download(model_id)` is called THEN THE SYSTEM SHALL cancel the in-progress download, clean up any partial files in `~/.jarvis/models/llm/.downloads/`, and remove the model from the download queue.
8. WHEN `delete_llm_model(model_id)` is called THEN THE SYSTEM SHALL remove the entire model directory from `~/.jarvis/models/llm/` and clear any error state for that model.
9. WHEN `delete_llm_model()` is called for the currently active model THEN THE SYSTEM SHALL return an error `"Cannot delete the active model"`.
10. THE SYSTEM SHALL reuse the existing `ModelStatus` enum from `model_manager.rs` for LLM model status tracking.
11. THE SYSTEM SHALL validate a downloaded model by checking that `config.json` exists in the model directory.
12. THE SYSTEM SHALL use a `.downloads/` temporary directory inside `~/.jarvis/models/llm/` for in-progress downloads and atomically rename to the final location on completion.

---

### Requirement 3: MlxProvider (Rust IntelProvider Implementation)

**User Story:** As a user, I want my gems to be enriched with tags and summaries using a local MLX model, so that I get AI-powered organization without sending my data to the cloud.

#### Acceptance Criteria

1. THE SYSTEM SHALL implement `MlxProvider` as a new struct implementing the `IntelProvider` trait in `src/intelligence/mlx_provider.rs`.
2. WHEN `MlxProvider::new(app_handle, model_path, python_path)` is called THEN THE SYSTEM SHALL spawn the Python MLX sidecar using `tokio::process::Command` with stdin/stdout piped, send a `check-availability` command, and if available send a `load-model` command with the given model path.
3. WHEN `MlxProvider::new()` fails to spawn the Python process (python not found, mlx not installed) THEN THE SYSTEM SHALL return `Err(String)` with a descriptive error message.
4. WHEN `check_availability()` is called THEN THE SYSTEM SHALL return `AvailabilityResult { available: true, reason: None }` if the sidecar is running and a model is loaded, or `{ available: false, reason: Some("...") }` otherwise.
5. WHEN `generate_tags(content)` is called THEN THE SYSTEM SHALL split content into chunks at paragraph boundaries if it exceeds 15,000 characters (MLX models have 8K+ token context), send each chunk to the sidecar via `generate-tags` command, deduplicate tags case-insensitively, and return at most 5 tags.
6. WHEN `summarize(content)` is called THEN THE SYSTEM SHALL split content into chunks if needed, send each chunk to the sidecar via `summarize` command, and if multiple chunks produced summaries, combine them by sending the concatenated summaries back for a final summarization pass.
7. WHEN the sidecar process dies unexpectedly THEN THE SYSTEM SHALL detect the broken pipe on the next command attempt and return an error `"MLX sidecar process terminated"` rather than hanging.
8. THE SYSTEM SHALL use a 60-second timeout for generate-tags and summarize commands (longer than IntelligenceKit's 30s because local LLM inference is slower).
9. THE SYSTEM SHALL implement a `switch_model(model_path)` method that sends a `load-model` command to the running sidecar, enabling hot-swapping models without restarting the process.
10. WHEN `switch_model()` fails (invalid path, load error) THEN THE SYSTEM SHALL return an error and the sidecar SHALL remain loaded with the previous model.

---

### Requirement 4: Provider Selection and Fallback Chain

**User Story:** As a user, I want Jarvis to automatically select the best available intelligence provider based on my settings, so that AI enrichment works seamlessly regardless of my system configuration.

#### Acceptance Criteria

1. THE SYSTEM SHALL modify `intelligence/mod.rs` to accept settings and select a provider based on `settings.intelligence.provider`:
   - `"mlx"` → attempt `MlxProvider::new()`, fall back to IntelligenceKit, then NoOp
   - `"intelligencekit"` → attempt `IntelligenceKitProvider::new()`, fall back to NoOp
   - `"api"` → return `NoOpProvider` with message "API provider not yet implemented"
   - any other value → default to `"mlx"` behavior
2. WHEN the MLX provider is selected but fails to initialize (Python missing, mlx not installed, no model downloaded) THEN THE SYSTEM SHALL log the error, attempt IntelligenceKit as fallback, and if that also fails, use NoOpProvider.
3. WHEN the intelligence provider is selected THEN THE SYSTEM SHALL log which provider was successfully initialized (e.g., `"Intelligence: Using MlxProvider with model Qwen3-8B-4bit"`).
4. THE SYSTEM SHALL update `create_provider()` in `mod.rs` to accept `&Settings` as a parameter in addition to `AppHandle`.
5. THE SYSTEM SHALL update `lib.rs` to pass settings to `create_provider()` during app initialization.

---

### Requirement 5: Intelligence Settings

**User Story:** As a user, I want to configure which intelligence provider and model to use in my settings, so that I can switch between MLX, IntelligenceKit, and future API providers.

#### Acceptance Criteria

1. THE SYSTEM SHALL add an `IntelligenceSettings` struct to `settings/manager.rs`:
   ```
   IntelligenceSettings {
       provider: String,       // "mlx" | "intelligencekit" | "api"
       active_model: String,   // catalog ID, e.g. "qwen3-8b-4bit"
       python_path: String,    // "python3" or absolute path to python binary
   }
   ```
2. THE SYSTEM SHALL add `#[serde(default)] pub intelligence: IntelligenceSettings` to the `Settings` struct.
3. THE SYSTEM SHALL provide sensible defaults: `provider: "mlx"`, `active_model: "qwen3-8b-4bit"`, `python_path: "python3"`.
4. WHEN settings are loaded from an existing `settings.json` that does not contain the `intelligence` key THEN THE SYSTEM SHALL use the default `IntelligenceSettings` (backward compatibility via `#[serde(default)]`).
5. THE SYSTEM SHALL add validation in `SettingsManager::validate()`:
   - `provider` must be one of `"mlx"`, `"intelligencekit"`, `"api"`
   - `active_model` must not be empty
   - `python_path` must not be empty
6. THE SYSTEM SHALL persist the intelligence settings to `~/.jarvis/settings.json` alongside existing transcription and browser settings.

---

### Requirement 6: New Tauri Commands

**User Story:** As a frontend developer, I want Tauri commands for managing LLM models and switching providers, so that I can build the settings UI.

#### Acceptance Criteria

1. THE SYSTEM SHALL add these new Tauri commands to `commands.rs` and register them in `lib.rs`:

   | Command | Parameters | Returns | Description |
   |---------|-----------|---------|-------------|
   | `list_llm_models` | none | `Vec<LlmModelInfo>` | List all catalog models with download status |
   | `download_llm_model` | `model_id: String` | `()` | Start async download, emits progress events |
   | `cancel_llm_download` | `model_id: String` | `()` | Cancel in-progress download |
   | `delete_llm_model` | `model_id: String` | `()` | Remove model from disk |
   | `switch_llm_model` | `model_id: String` | `()` | Change active model, tell sidecar to reload |

2. WHEN `switch_llm_model(model_id)` is called THEN THE SYSTEM SHALL:
   - Verify the model is downloaded
   - Update `settings.intelligence.active_model` to the new model_id
   - Resolve the model path from `~/.jarvis/models/llm/` and the catalog repo name
   - Send `load-model` to the running MLX sidecar
   - Persist the updated settings to disk
3. WHEN `switch_llm_model()` is called with a model that is not downloaded THEN THE SYSTEM SHALL return an error `"Model <id> is not downloaded"`.
4. THE SYSTEM SHALL add `LlmModelManager` to Tauri managed state in `lib.rs`, wrapped in `Arc`, following the same pattern as the existing `ModelManager`.
5. WHEN `enrich_gem` is called THEN THE SYSTEM SHALL continue to work exactly as before — it uses the `IntelProvider` trait, which now resolves to whichever provider is active. No changes to `enrich_gem` or `check_intel_availability` commands.

---

### Requirement 7: Frontend Settings UI — Intelligence Section

**User Story:** As a user, I want a settings panel where I can choose my intelligence provider and manage LLM models (download, switch, delete), so that I can control how my gems are enriched.

#### Acceptance Criteria

1. THE SYSTEM SHALL add an "Intelligence" section to the settings panel, below the existing transcription settings.
2. THE SYSTEM SHALL display a provider selector with three options: "Local MLX (Recommended)", "Apple IntelligenceKit", and "Cloud API (coming soon)" where Cloud API is disabled/grayed out.
3. WHEN the user selects "Local MLX" THEN THE SYSTEM SHALL show a model list below the provider selector.
4. THE SYSTEM SHALL display each model from the catalog as a card showing: display name, size estimate, quality tier badge, description, and current status (Downloaded/Downloading/Not Downloaded).
5. WHEN a model has status `NotDownloaded` THEN THE SYSTEM SHALL show a "Download" button on its card.
6. WHEN the user clicks "Download" THEN THE SYSTEM SHALL invoke `download_llm_model`, show a progress bar on the card, and listen for `llm-model-download-progress` events to update the bar.
7. WHEN a model is downloading THEN THE SYSTEM SHALL show a "Cancel" button that invokes `cancel_llm_download`.
8. WHEN a model has status `Downloaded` and is the active model THEN THE SYSTEM SHALL show an "Active" badge on its card.
9. WHEN a model has status `Downloaded` and is NOT the active model THEN THE SYSTEM SHALL show a "Set Active" button that invokes `switch_llm_model` and a "Delete" button that invokes `delete_llm_model`.
10. WHEN model download completes THEN THE SYSTEM SHALL refresh the model list by re-invoking `list_llm_models`.
11. WHEN the user switches providers THEN THE SYSTEM SHALL invoke `update_settings` with the new provider value.

---

### Requirement 8: Graceful Degradation and Error Handling

**User Story:** As a user, I want the app to continue working even when AI enrichment is unavailable, so that I never lose access to my gems.

#### Acceptance Criteria

1. WHEN Python is not installed or `mlx_lm` is not installed THEN THE SYSTEM SHALL fall back to IntelligenceKit (if available) or NoOpProvider, and the app SHALL continue to function for all non-AI features.
2. WHEN no LLM model has been downloaded yet THEN THE SYSTEM SHALL show a message in the gems panel: "AI enrichment unavailable. Download a model in Settings." and the "Enrich" button SHALL be disabled with a tooltip explaining why.
3. WHEN the MLX sidecar crashes during enrichment THEN THE SYSTEM SHALL return an error to the frontend and the frontend SHALL display a toast notification with the error, not crash or hang.
4. WHEN a model download fails THEN THE SYSTEM SHALL clean up partial files from the `.downloads/` directory, set the model status to `Error` with the failure message, and allow the user to retry.
5. WHEN settings.json has `"provider": "mlx"` but the active model's directory does not exist (user manually deleted it) THEN THE SYSTEM SHALL detect this during provider initialization, log a warning, and fall back through the provider chain.
6. THE SYSTEM SHALL never block the app startup for more than 5 seconds on intelligence provider initialization. If the sidecar takes longer than 5 seconds to respond to `check-availability`, THE SYSTEM SHALL timeout and use NoOpProvider.

---

## Technical Constraints

1. **macOS only**: MLX requires Apple Silicon. The sidecar should check `platform.machine()` and refuse to load on non-ARM64 machines.
2. **Python dependency**: The user must have Python 3.10+ with `mlx`, `mlx-lm`, and `huggingface-hub` installed. The sidecar script should fail gracefully with a clear error if imports fail.
3. **Memory**: Qwen 8B (4-bit) needs ~6 GB RAM. The model catalog should document memory requirements so users can make informed choices.
4. **No `tauri_plugin_shell`**: Per ADR-002 in `docs/project_notes/decisions.md`, sidecar processes use `tokio::process::Command` (not Tauri's shell plugin) for direct stdin/stdout access.
5. **Backward compatibility**: Existing `settings.json` files without the `intelligence` key must continue to work via `#[serde(default)]`.

---

## Out of Scope

- Cloud API provider implementation (placeholder only in this iteration)
- Frontend GemsPanel changes (already works with any IntelProvider)
- Automatic Python/MLX installation
- Model fine-tuning or custom model training
- Streaming token output to the UI during enrichment
