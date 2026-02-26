# MLX Multimodal Integration into Jarvis — Research Findings

## Goal

Evolve the existing `mlx-server` sidecar into a unified local intelligence engine.
A gem gets enriched with three fields from the local model:
- **tags** (existing)
- **summary** (existing)
- **transcript** (new — from local multimodal model processing the recording audio)

Whisper continues to provide real-time partials during recording.
After recording stops, the local MLX model generates an accurate multilingual transcript that gets stored on the gem — same as tags and summary.

Currently tested with Qwen2.5-Omni, but the sidecar is model-agnostic.

### Future Direction
The current `mlx-server` sidecar evolves into a single local intelligence engine. One sidecar, one venv, one multimodal model handling all gem enrichment:
- **transcript** — from audio (multimodal models like Qwen2.5-Omni)
- **tags** — from text content
- **summary** — from text content

Eventually the same multimodal model handles all three, replacing separate text-only LLMs.

---

## What We Proved in Experiments

### Model: `giangndm/qwen2.5-omni-3b-mlx-8bit`
- Transcribes Hindi audio correctly (Devanagari output)
- Transcribes English (JFK) perfectly
- ~12s to process 27.6s audio on Apple Silicon (0.43x realtime)
- ~5s to process 11s audio
- Peak memory: ~5.3 GB
- Model load time: ~1.5s

### Model: `giangndm/qwen2.5-omni-7b-mlx-4bit`
- Better quality, similar speed
- Higher memory (~8+ GB)
- Conv weights ship in PyTorch layout — need `mx.swapaxes(w, 1, 2)` fix

### Bugs Found & Fixed in `mlx-lm-omni` v0.1.3
| Bug | File | Fix |
|-----|------|-----|
| Audio chunks merged before transformer (breaks >15s audio) | `audio_tower.py` | Move reshape after transformer loop |
| float16 mel computation (precision loss on quiet audio) | `audio_mel.py` | Use float32 for mel_filters, waveform, window |
| Prefill chunking splits audio token region | `generate()` call | Pass `prefill_step_size=len(prompt)` |
| Conv weight layout mismatch (7B only) | `utils.py` | Auto-detect and swapaxes |
| Missing `__getattr__` on tokenizer wrapper | `model.py` | Delegate to inner tokenizer |
| `to_quantized()` missing `**kwargs` (mlx 0.30) | `tokenizer.py` | Add `**kwargs` parameter |

---

## Current Jarvis Transcription Architecture

### Pipeline
```
JarvisListen (Swift sidecar, ScreenCaptureKit)
    → s16le 16kHz mono PCM → FIFO pipe
    → AudioRouter → AudioBuffer (3s windows, 0.5s overlap)
    → TranscriptionProvider (trait)
        ├─ HybridProvider (default): VAD + Vosk partials + Whisper finals
        ├─ WhisperKitProvider: macOS native whisperkit-cli
        └─ WhisperProvider: standalone whisper.cpp
    → Tauri events → React frontend
```

### Key trait
```rust
pub trait TranscriptionProvider: Send + Sync {
    fn name(&self) -> &str;
    fn initialize(&mut self, config: &TranscriptionConfig) -> Result<()>;
    fn transcribe(&mut self, audio: &[f32]) -> Result<Vec<TranscriptionSegment>>;
}
```

### Recording stop flow
1. `stop_recording()` → sends SIGTERM to JarvisListen
2. TranscriptionManager drains remaining audio buffers
3. Emits `transcription-stopped` with full Whisper transcript
4. Emits `recording-stopped` (PCM file verified on disk)
5. Frontend updates state, shows "Save Gem" button

### PCM files stored at
```
~/.jarvis/recordings/YYYYMMDD_HHMMSS.pcm
```

---

## Existing MLX Sidecar Pattern

### Protocol: NDJSON over stdin/stdout
```
Rust → stdin:  {"command": "generate-tags", "content": "..."}\n
Python → stdout: {"type": "response", "command": "generate-tags", "tags": [...]}\n
```

### Lifecycle
1. `tokio::process::Command::new(python_path).arg(server.py)` with piped stdin/stdout/stderr
2. Stderr monitored in background tokio task
3. `Arc<Mutex<>>` protects state for thread-safe access
4. 60s timeouts on send/receive
5. Shutdown: send `{"command": "shutdown"}` → wait 3s → SIGKILL

### Venv system
```
~/.jarvis/venv/mlx/
├── bin/python3
└── .jarvis-setup-complete  (SHA-256 hash of requirements.txt)
```
- `VenvManager.resolve_python_path()`: returns venv Python if ready, else system Python
- Status: NotCreated | Ready | NeedsUpdate

---

## Proposed Design: Transcript as Gem Enrichment Field

### How it fits with existing gem enrichment
```
Gem enrichment pipeline (all via mlx-server sidecar):
  ├─ generate-tags     → gem.tags       (existing)
  ├─ generate-summary  → gem.summary    (existing)
  └─ generate-transcript → gem.transcript (new)
```

The transcript is just another enrichment field on a gem, like tags and summary.
When a recording is saved as a gem, the enrichment pipeline runs all three.

### Architecture
```
[During recording]
    → Whisper provides real-time partials (existing, unchanged)

[Recording saved as gem]
    → Gem created with raw Whisper transcript
    → Enrichment kicks in (same as existing flow):
        1. generate-tags      → stored on gem
        2. generate-summary   → stored on gem
        3. generate-transcript → stored on gem (language + native text)
    → Frontend updates gem display with enriched fields
```

### Gem Data Model Change
```rust
// Existing gem fields
pub struct Gem {
    pub id: String,
    pub content: String,          // raw content / Whisper transcript
    pub tags: Vec<String>,        // from MLX enrichment
    pub summary: Option<String>,  // from MLX enrichment
    // NEW:
    pub transcript: Option<String>,        // accurate transcript from local model
    pub transcript_language: Option<String>, // detected language (e.g. "Hindi", "English")
}
```

### Evolve `sidecars/mlx-server/server.py`

The existing sidecar gets a new command. Same process, same model, same venv.

Commands (existing + new):
- `check-availability` — verify mlx packages importable
- `load-model` — load any MLX model (text LLM or multimodal)
- `generate-tags` — gem enrichment (existing)
- `generate-summary` — gem enrichment (existing)
- **`generate-transcript`** — gem enrichment (new): takes audio file path, returns `{ language, transcript }`
- `shutdown` — graceful exit

Dependencies (`requirements.txt` — expanded):
```
mlx>=0.22.0
mlx-lm>=0.22.0
mlx-lm-omni>=0.1.3      # multimodal support
librosa>=0.10.0           # audio loading
soundfile>=0.12.0         # WAV I/O
numpy>=1.24.0
huggingface-hub>=0.20.0
```

Note: `mlx-lm-omni` needs our patches. Options:
1. Fork and publish patched version
2. Ship patched files alongside the sidecar
3. Submit PR upstream and wait for merge

### Rust Side: Extend existing `intelligence/mlx_provider.rs`

No new Rust module needed — add `generate_transcript()` to existing `MlxProvider`:
- Same NDJSON protocol, same sidecar process
- New method: `generate_transcript(audio_path: &Path) -> Result<TranscriptResult>`
- Timeout: 120s (long audio can take a while)
- Returns `{ language: String, transcript: String }`

### Enrichment Flow for Recording Gems

In `gems/store.rs` or wherever gem enrichment is triggered:
```
async fn enrich_gem(gem: &mut Gem, provider: &dyn IntelProvider) {
    // Existing:
    gem.tags = provider.generate_tags(&gem.content).await?;
    gem.summary = provider.summarize(&gem.content).await?;

    // New — only for gems with a recording:
    if let Some(recording_path) = &gem.recording_path {
        let result = provider.generate_transcript(recording_path).await?;
        gem.transcript = Some(result.transcript);
        gem.transcript_language = Some(result.language);
    }
}
```

### Single Venv (shared — no new venv)
```
~/.jarvis/venv/mlx/       ← same venv, just expanded requirements
├── bin/python3
└── .jarvis-setup-complete
```

requirements.txt grows to include audio deps. Existing `VenvManager` handles it — hash changes trigger "needs update".

---

### Settings UI: New Transcription Engine Option

The existing transcription engine dropdown (`whisper-rs` | `whisperkit`) gets a new option:

```
Transcription Engine:
  ├─ Whisper (Local)           ← existing, whisper.cpp via whisper-rs
  ├─ WhisperKit (macOS Native) ← existing, whisperkit-cli
  └─ MLX Omni (Local, Private) ← NEW: multimodal model via mlx-lm-omni
```

When "MLX Omni (Local, Private)" is selected:
- Shows model picker dropdown (e.g. `qwen2.5-omni-3b-mlx-8bit`, `qwen2.5-omni-7b-mlx-4bit`)
- "Download Model" button (reuses existing `LlmModelManager` infrastructure)
- Shows model status: not downloaded / downloading / ready
- Shows venv status: not set up / ready / needs update

This mirrors the existing "MLX (Local, Private)" option in the Intelligence settings panel, but for transcription models instead of text LLMs.

```rust
TranscriptionSettings {
    // ... existing fields ...
    transcription_engine: String,             // "whisper-rs" | "whisperkit" | "mlx-omni"
    mlx_omni_model: String,                   // HuggingFace repo ID
}
```

---

## Migration Path

### Phase 1 (Now)
- Add `generate-transcript` command to `mlx-server/server.py`
- Add audio deps to `requirements.txt`
- Add `transcript` + `transcript_language` fields to gem data model
- Extend `MlxProvider` with `generate_transcript()` method
- Trigger transcript generation during gem enrichment for recording gems

### Phase 2 (Future)
- All three enrichment fields (tags, summary, transcript) generated by the same multimodal model
- A single Qwen2.5-Omni (or successor) handles everything
- Decommission separate text-only LLM models
- One model download, one model in memory

---

## Files to Modify

| File | Change |
|------|--------|
| `sidecars/mlx-server/server.py` | Add `generate-transcript` command (audio → text) |
| `sidecars/mlx-server/requirements.txt` | Add audio deps (librosa, soundfile, mlx-lm-omni) |
| `src-tauri/src/intelligence/mlx_provider.rs` | Add `generate_transcript()` method |
| `src-tauri/src/intelligence/mod.rs` | Extend `IntelProvider` trait with `generate_transcript()` |
| `src-tauri/src/gems/store.rs` | Add `transcript`, `transcript_language` to gem schema |
| `src-tauri/src/gems/sqlite_store.rs` | Add columns + enrichment trigger for recording gems |
| `src/state/types.ts` | Add transcript fields to Gem type |
| `src/components/GemsPanel.tsx` | Display transcript + language on gem cards |

---

## Open Questions

1. **Upstream patches**: Submit PR to `mlx-lm-omni` or maintain our own fork?
2. **Memory budget**: 3B (5.3GB) vs 7B (8GB+) — can both Whisper and multimodal model fit in memory?
3. **Gem display**: Show both Whisper (raw) and MLX (enriched) transcripts? Or only MLX when available?
4. **Sidecar lifecycle**: Keep model loaded permanently, or load/unload per enrichment to save memory?
5. **Enrichment timing**: Enrich immediately when gem is saved, or batch enrich in background?

---

## Performance Summary

| Metric | 3B-8bit | 7B-4bit |
|--------|---------|---------|
| Model load | 1.5s | ~3s |
| 11s audio | 2.2s | 1.5s |
| 27s audio | 4.2s | ~4s |
| 90s audio | ~12s | ~10s |
| Peak memory | 5.3 GB | ~8 GB |
| Hindi support | Yes | Yes |
| Quality | Good | Better |
