# Requirements: MLX Omni Transcription (Local, Private)

## Introduction

Jarvis is a Tauri desktop app (Rust backend + React frontend) that captures audio via JarvisListen (ScreenCaptureKit sidecar) and transcribes it in real-time using Whisper (whisper.cpp/whisper-rs). Recordings are saved as "gems" and enriched with AI-generated tags and summaries via the `IntelProvider` trait backed by a Python MLX sidecar (`mlx-server/server.py`).

This feature adds **local multimodal audio transcription** as a new gem enrichment field — alongside tags and summary. After a recording is saved as a gem, the existing MLX sidecar processes the full audio file using a multimodal model (e.g. Qwen2.5-Omni) and stores an accurate, multilingual transcript on the gem.

### Key Architecture Context

- **`IntelProvider` trait** (`src/intelligence/provider.rs`): Swappable interface — `check_availability()`, `generate_tags(content)`, `summarize(content)`.
- **`MlxProvider`** (`src/intelligence/mlx_provider.rs`): Current implementation. Spawns `sidecars/mlx-server/server.py`, communicates via NDJSON over stdin/stdout with 60s timeouts.
- **`mlx-server/server.py`**: Python sidecar handling `check-availability`, `load-model`, `generate-tags`, `summarize`, `download-model`, `model-info`, `shutdown` commands.
- **`VenvManager`** (`src/intelligence/venv_manager.rs`): Manages `~/.jarvis/venv/mlx/` — creates venv, installs requirements, tracks status via marker file with SHA-256 hash.
- **`LlmModelManager`** (`src/intelligence/llm_model_manager.rs`): Manages model downloads to `~/.jarvis/models/llm/` with catalog, status tracking, and progress events.
- **Gem enrichment flow**: When a gem is saved, `enrich_gem` calls `provider.generate_tags()` and `provider.summarize()`. Results are stored on the gem in SQLite.
- **Recording pipeline**: JarvisListen → s16le 16kHz mono PCM → FIFO → AudioRouter → file on disk (`~/.jarvis/recordings/YYYYMMDD_HHMMSS.pcm`). Whisper provides real-time partials during recording.
- **Sidecar pattern**: `tokio::process::Command` with piped stdin/stdout/stderr, `Arc<Mutex<>>` for state, NDJSON protocol, 60s timeouts.

### What We Proved in Experiments

Using `giangndm/qwen2.5-omni-3b-mlx-8bit` via `mlx-lm-omni` on Apple Silicon:
- Transcribes Hindi audio correctly (outputs Devanagari script)
- Transcribes English (JFK speech) perfectly
- ~12s for 27.6s audio (0.43x realtime), ~5s for 11s audio
- Peak memory: ~5.3 GB, model load: ~1.5s
- Correctly detects spoken language and outputs in native script

Six bugs were found in `mlx-lm-omni` v0.1.3 that must be patched at runtime (see Requirement 1).

### Goals

1. Gems created from recordings get an accurate multilingual transcript as a stored field, alongside tags and summary.
2. The transcript is generated locally using a multimodal MLX model — no data leaves the machine.
3. The MLX sidecar (`mlx-server/server.py`) is extended with a new command — no separate sidecar process.
4. Users can select "MLX Omni (Local, Private)" as a transcription engine in Settings and download multimodal models.
5. Whisper continues to provide real-time partials during recording (unchanged).
6. The system is model-agnostic — any multimodal MLX model that supports audio can be used.

---

## Requirements

### Requirement 1: Patch `mlx-lm-omni` at Runtime

**User Story:** As a developer, I want the MLX sidecar to automatically fix known bugs in `mlx-lm-omni` at startup, so that users don't need to manually patch their Python environment.

#### Acceptance Criteria

1. WHEN the sidecar starts and `mlx-lm-omni` is installed THEN THE SYSTEM SHALL apply runtime patches before any model loading.
2. THE SYSTEM SHALL patch `AudioTower.__call__` to move the reshape operation after the transformer loop (not before), so that each audio chunk processes independently as a batch element through the transformer. This fixes transcription failure for audio longer than ~15 seconds.
3. THE SYSTEM SHALL patch `AudioMel` to use float32 (not float16) for `mel_filters`, `waveform`, and `window` computation, preventing precision loss on quiet audio.
4. THE SYSTEM SHALL patch `ExtendedQuantizedEmbedding.to_quantized()` to accept `**kwargs` for compatibility with MLX 0.30+.
5. THE SYSTEM SHALL patch the `Model` class to add `__getattr__` delegation to the inner tokenizer and a `chat_template` property.
6. WHEN generating transcripts, THE SYSTEM SHALL NOT call `tokenizer.apply_chat_template()` separately to compute token count before calling `generate()`. Each call to `apply_chat_template()` pushes audio embeddings into the `ExtendedEmbedding` queue, and calling it twice (once for token count, once inside `generate()`) causes duplicate embeddings and state corruption. Instead, THE SYSTEM SHALL use a large `prefill_step_size=32768` to ensure the entire prompt (text + audio tokens) is processed in a single prefill step without chunking.
7. WHEN loading a 7B model whose conv weights are in PyTorch layout THEN THE SYSTEM SHALL auto-detect the layout mismatch (`conv1.weight.shape[1] != 3`) and apply `mx.swapaxes(weight, 1, 2)` to both conv1 and conv2.
8. THE SYSTEM SHALL log which patches were applied at startup to stderr for debugging.
9. WHEN upstream `mlx-lm-omni` releases a version that includes these fixes THEN the patches SHALL be version-gated (only applied if `mlx_lm_omni.__version__ <= "0.1.3"` or similar check) so they can be safely removed.

---

### Requirement 2: `generate-transcript` Sidecar Command

**User Story:** As a developer, I want the existing MLX sidecar to accept a `generate-transcript` command that processes an audio file and returns an accurate multilingual transcript, so that the Rust backend can enrich gems with transcripts using the same sidecar process.

#### Acceptance Criteria

1. WHEN the sidecar receives `{"command":"generate-transcript","audio_path":"/path/to/file.pcm"}` and a multimodal model is loaded THEN THE SYSTEM SHALL:
   - Read the PCM file (s16le, 16kHz, mono) and convert to float32 audio
   - Process the audio through the loaded multimodal model with the prompt: "First, identify the language spoken in this audio. Then transcribe the audio verbatim in that original language. Do NOT translate."
   - Respond with `{"type":"response","command":"generate-transcript","language":"Hindi","transcript":"आने वाला था..."}`
2. WHEN the audio file is a `.wav` file THEN THE SYSTEM SHALL load it directly using librosa at 16kHz.
3. WHEN the audio file is a `.pcm` file THEN THE SYSTEM SHALL read it as raw s16le 16kHz mono and convert to float32.
4. WHEN `generate-transcript` is called with no model loaded THEN THE SYSTEM SHALL respond with `{"type":"error","command":"generate-transcript","error":"No model loaded"}`.
5. WHEN `generate-transcript` is called with a non-existent file path THEN THE SYSTEM SHALL respond with `{"type":"error","command":"generate-transcript","error":"Audio file not found: ..."}`.
6. WHEN the audio file is empty or contains no speech THEN THE SYSTEM SHALL respond with `{"type":"response","command":"generate-transcript","language":"unknown","transcript":""}`.
7. THE SYSTEM SHALL use `max_tokens=2000` for transcript generation to handle long recordings.
8. THE SYSTEM SHALL use a timeout of 120 seconds for transcript generation (long audio can take a while on 3B models).
9. THE SYSTEM SHALL clear the `ExtendedEmbedding` queue before each transcription to prevent state leakage between calls.
10. THE SYSTEM SHALL add `mlx-lm-omni`, `librosa`, `soundfile`, and `numpy` to `sidecars/mlx-server/requirements.txt`.

---

### Requirement 3: Multimodal Model Catalog

**User Story:** As a user, I want to download multimodal models that can transcribe audio, so that I can use local AI transcription in my preferred language.

#### Acceptance Criteria

1. THE SYSTEM SHALL add a `MULTIMODAL_MODEL_CATALOG` (or extend the existing `LLM_MODEL_CATALOG`) with at least these models:

   | ID | HuggingFace Repo | Display Name | Size Estimate | Quality | Capabilities |
   |----|-----------------|--------------|---------------|---------|--------------|
   | `qwen-omni-3b-8bit` | `giangndm/qwen2.5-omni-3b-mlx-8bit` | Qwen 2.5 Omni 3B (8-bit) | ~5 GB | good | audio, text |
   | `qwen-omni-7b-4bit` | `giangndm/qwen2.5-omni-7b-mlx-4bit` | Qwen 2.5 Omni 7B (4-bit) | ~8 GB | better | audio, text |

2. THE SYSTEM SHALL store multimodal models at `~/.jarvis/models/llm/<ModelDirName>/` (same location as text LLMs).
3. WHEN a model's capabilities include `"audio"` THEN the Settings UI SHALL show it under the "MLX Omni (Local, Private)" transcription engine section.
4. WHEN a model's capabilities include `"text"` THEN the Settings UI SHALL show it under the existing Intelligence section.
5. WHEN a multimodal model supports both audio and text THEN it SHALL appear in both sections, and using one model for both reduces total memory usage.
6. THE SYSTEM SHALL reuse the existing `LlmModelManager` for downloading, deleting, and tracking multimodal models.

---

### Requirement 4: Transcript as Gem Enrichment Field

**User Story:** As a user, I want my recording gems to have an accurate multilingual transcript stored alongside tags and summary, so that I can read what was said in the original language.

#### Acceptance Criteria

1. THE SYSTEM SHALL add two new fields to the Gem data model:
   - `transcript: Option<String>` — the accurate transcript from the local model
   - `transcript_language: Option<String>` — the detected language (e.g. "Hindi", "English")
2. THE SYSTEM SHALL add `transcript TEXT` and `transcript_language TEXT` columns to the gems SQLite table via a migration.
3. WHEN a gem is created from a recording and enrichment is triggered THEN THE SYSTEM SHALL call `provider.generate_transcript(audio_path)` in addition to `generate_tags()` and `summarize()`.
4. THE SYSTEM SHALL only call `generate_transcript()` for gems that have an associated recording file path. For non-recording gems (YouTube, Medium, etc.), the transcript field remains NULL.
5. WHEN transcript generation fails (model not loaded, audio file missing, timeout) THEN THE SYSTEM SHALL log the error, leave the transcript fields as NULL, and NOT fail the overall enrichment (tags and summary should still be stored).
6. THE SYSTEM SHALL extend the `IntelProvider` trait with a new method:
   ```
   async fn generate_transcript(&self, audio_path: &Path) -> Result<TranscriptResult, String>
   ```
   where `TranscriptResult` contains `language: String` and `transcript: String`.
7. THE SYSTEM SHALL provide a default implementation of `generate_transcript()` on the trait that returns `Err("Transcript generation not supported by this provider")`, so that existing providers (`IntelligenceKitProvider`, `NoOpProvider`) don't need changes.
8. WHEN the Gem is serialized to the frontend THEN THE SYSTEM SHALL include `transcript` and `transcript_language` fields.

---

### Requirement 5: Rust `generate_transcript()` on MlxProvider

**User Story:** As a developer, I want the MlxProvider to support transcript generation using the same sidecar process, so that no additional Python processes are needed.

#### Acceptance Criteria

1. THE SYSTEM SHALL add a `generate_transcript(audio_path: &Path) -> Result<TranscriptResult, String>` method to `MlxProvider`.
2. WHEN `generate_transcript()` is called THEN THE SYSTEM SHALL send `{"command":"generate-transcript","audio_path":"<absolute_path>"}` to the running MLX sidecar via the existing NDJSON stdin/stdout channel.
3. THE SYSTEM SHALL use a 120-second timeout for transcript commands (longer than the 60s timeout for tags/summary).
4. WHEN the sidecar responds with `{"ok":true,"result":{"language":"...","transcript":"..."}}` THEN THE SYSTEM SHALL return `Ok(TranscriptResult { language, transcript })`.
5. WHEN the sidecar responds with `{"ok":false,"error":"..."}` THEN THE SYSTEM SHALL return `Err(error_message)`.
6. THE SYSTEM SHALL NOT spawn a separate sidecar process for transcription — it reuses the existing `MlxProvider` sidecar.

---

### Requirement 6: Settings UI — MLX Omni Transcription Engine

**User Story:** As a user, I want to select "MLX Omni (Local, Private)" as a transcription engine in Settings, download multimodal models, and see model status, so that I can enable local multilingual transcription.

#### Acceptance Criteria

1. THE SYSTEM SHALL add "MLX Omni (Local, Private)" as a new option in the transcription engine dropdown alongside "Whisper (Local)" and "WhisperKit (macOS Native)".
2. WHEN the user selects "MLX Omni (Local, Private)" THEN THE SYSTEM SHALL show:
   - A model picker listing multimodal models from the catalog (those with `"audio"` capability)
   - Each model shows: display name, size estimate, quality badge, download status
   - "Download" button for models not yet downloaded
   - Progress bar during download
   - "Active" badge on the currently selected model
   - Venv status indicator (not set up / ready / needs update)
3. WHEN no multimodal model is downloaded THEN THE SYSTEM SHALL show a message: "Download a multimodal model to enable MLX transcription."
4. THE SYSTEM SHALL add these fields to `TranscriptionSettings`:
   - `transcription_engine: String` — `"whisper-rs"` | `"whisperkit"` | `"mlx-omni"`
   - `mlx_omni_model: String` — HuggingFace repo ID of the selected multimodal model
5. WHEN `transcription_engine` is `"mlx-omni"` THEN the real-time transcription during recording SHALL still use Whisper (for instant partials). The MLX Omni model is used only for post-recording gem enrichment.
6. THE SYSTEM SHALL reuse the existing `LlmModelManager` download infrastructure (progress events, cancel, delete) for multimodal models.

---

### Requirement 7: Frontend Gem Display — Transcript Field

**User Story:** As a user, I want to see the accurate multilingual transcript on my recording gems, so that I can read what was said in the original language.

#### Acceptance Criteria

1. WHEN a gem has a non-null `transcript` field THEN THE SYSTEM SHALL display it in the gem detail view, labeled with the detected language (e.g. "Transcript (Hindi)").
2. WHEN a gem has both a raw Whisper transcript (in `content`) and an MLX transcript (in `transcript`) THEN THE SYSTEM SHALL show the MLX transcript prominently and the Whisper transcript as a secondary/collapsed section.
3. WHEN a gem has no transcript (field is NULL) THEN THE SYSTEM SHALL NOT show a transcript section — no empty placeholders or "pending" states.
4. WHEN a gem's transcript is being generated (enrichment in progress) THEN THE SYSTEM SHALL show a loading indicator in the transcript area.
5. THE SYSTEM SHALL add `transcript` and `transcript_language` to the `Gem` TypeScript interface in `src/state/types.ts`.

---

### Requirement 8: Graceful Degradation

**User Story:** As a user, I want the app to work normally even when multimodal transcription is unavailable, so that I never lose access to my recordings or gems.

#### Acceptance Criteria

1. WHEN `mlx-lm-omni` is not installed in the venv THEN THE SYSTEM SHALL skip transcript generation during enrichment and still generate tags and summary normally.
2. WHEN no multimodal model is downloaded THEN THE SYSTEM SHALL skip transcript generation and log a warning. Tags and summary still work if a text LLM is loaded.
3. WHEN the loaded model does not support audio (text-only LLM) THEN THE SYSTEM SHALL return an error from `generate-transcript` and the enrichment flow SHALL skip the transcript field gracefully.
4. WHEN transcript generation times out (>120s) THEN THE SYSTEM SHALL log the timeout, leave transcript as NULL, and not retry automatically.
5. WHEN the sidecar crashes during transcript generation THEN THE SYSTEM SHALL handle it the same as other sidecar crashes — detect broken pipe, return error, allow re-initialization on next use.
6. WHEN the user has `transcription_engine: "whisper-rs"` (not MLX Omni) THEN THE SYSTEM SHALL NOT attempt transcript generation during enrichment, even if a multimodal model is loaded.

---

## Technical Constraints

1. **macOS only**: MLX requires Apple Silicon. Multimodal models also require Apple Silicon.
2. **Python dependency**: `mlx-lm-omni` plus `librosa`, `soundfile`, `numpy` must be installed in the managed venv (`~/.jarvis/venv/mlx/`).
3. **Memory**: Qwen 2.5 Omni 3B (8-bit) needs ~5.3 GB, 7B (4-bit) needs ~8 GB. Document this in the model catalog.
4. **Single sidecar**: Transcription uses the same `mlx-server` process as tags/summary. No separate process.
5. **Single venv**: `~/.jarvis/venv/mlx/` — expanded requirements.txt triggers venv "needs update" via hash change.
6. **PCM format**: Jarvis recordings are s16le, 16kHz, mono. The sidecar must handle this format directly.
7. **`mlx-lm-omni` patches**: Must be applied at runtime in `server.py` before model loading. Version-gated so they're removed when upstream fixes land.
8. **Backward compatibility**: Existing `settings.json` without `mlx_omni_model` or `transcription_engine: "mlx-omni"` must continue to work via `#[serde(default)]`.
9. **No `tauri_plugin_shell`**: Per ADR-002, sidecar processes use `tokio::process::Command`.

---

## Out of Scope

- Replacing Whisper for real-time transcription (Whisper still handles live partials)
- Streaming transcript tokens to the UI during generation
- Cloud-based transcription
- Speaker diarization (who said what)
- Automatic language translation (we transcribe in the original language)
- Video/image understanding (audio only for now)
- Custom prompt configuration for transcription
