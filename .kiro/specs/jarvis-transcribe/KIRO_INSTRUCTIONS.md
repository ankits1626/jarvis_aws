# Kiro Instructions: JarvisTranscribe Module

## What to Generate

Generate a complete spec (requirements.md, design.md, tasks.md) for the JarvisTranscribe module following the exact same format as `.kiro/specs/jarvis-listen/`. This is the "Transcribe" step in the JARVIS pipeline: Listen -> **Transcribe** -> Augment -> Display.

---

## What This Module Does

JarvisTranscribe adds **real-time speech-to-text** to the existing JARVIS desktop app. As JarvisListen captures audio, the Rust backend routes PCM chunks to both a recording file and the transcription pipeline in real-time (zero-delay streaming via named pipe). The transcription pipeline is a **three-engine hybrid**:

1. **Silero VAD** — detects if anyone is speaking (gates the pipeline, prevents hallucinations)
2. **Vosk** — provides instant partial text (<100ms latency, displayed as gray text)
3. **Whisper** — provides accurate final text (1-2s latency, replaces gray text with black)

The user sees text appear instantly as they speak (Vosk), then it silently gets corrected to high accuracy (Whisper). This is the "Whisper-VOSK Loop" pattern.

---

## Architecture Decision: Hybrid VAD + Vosk + Whisper

Three engines, each doing what it's best at:

| Engine | Role | Latency | Accuracy |
|--------|------|---------|----------|
| **Silero VAD** | Gate: is anyone speaking? | ~1ms per 32ms chunk | N/A (binary: speech/silence) |
| **Vosk** | Fast partials: show text instantly | <100ms | 10-15% WER (good enough for partials) |
| **Whisper** | Accurate finals: correct the text | 1-2s | 2-5% WER (best-in-class) |

### Pipeline Diagram

```
JarvisListen Sidecar (Swift)
    │
    │ writes PCM to named pipe (FIFO) via --output flag
    │ (sidecar doesn't know it's a FIFO — treats it as a regular file)
    ▼
┌──────────────────────┐
│  AudioRouter (Rust)   │  ← reads FIFO, routes chunks in real-time
│  (new component)      │  ← replaces PcmTailer — zero polling delay
└──────┬───────┬────────┘
       │       │
       │       └──→ PCM File (recording.pcm) — written by Rust for playback
       │
       ▼ mpsc channel (zero-copy)
┌─────────────┐
│ Silero VAD   │  ← 512-sample chunks (~32ms), <1ms processing
│ (gatekeeper) │  ← 100ms pre-roll buffer for word onsets
└──────┬───────┘
       │ speech detected?
       │
       ├── NO → skip (save CPU, prevent Whisper hallucinations)
       │
       ├── YES ──┬──→ ┌───────────┐
       │         │    │   Vosk     │ ← truly streaming, <100ms
       │         │    │  (partials)│ ← emit as is_final=false (gray text)
       │         │    └───────────┘
       │         │
       │         └──→ ┌───────────┐
       │              │  Whisper   │ ← batch on 3s windows, ~1-2s
       │              │  (finals)  │ ← emit as is_final=true (replaces gray)
       │              └───────────┘
       │
       ▼
  UI: "Hello, how are you doing today?"
       ^^^^^ gray (Vosk partial) → replaced by black (Whisper final)
```

### Why Named Pipe (FIFO) Instead of File Tailing

**Problem with the old approach**: Sidecar writes to file → transcription polls file every 200ms → 200ms delay + disk I/O overhead.

**Problem with stdout**: Tauri's shell plugin splits binary data on newline bytes (0x0A), which corrupts PCM audio data. This is documented in `recording.rs` line 124-126.

**Solution — Named Pipe (FIFO)**:
1. Before spawning sidecar, Rust creates a FIFO at a temp path using `nix::unistd::mkfifo()`
2. Sidecar is spawned with `--output <fifo_path>` — it treats the FIFO as a regular file
3. Rust opens the FIFO for reading in a tokio blocking task
4. As chunks arrive (every 100ms from sidecar), Rust immediately:
   - Writes to the actual PCM file (for recording/playback)
   - Sends via `tokio::sync::mpsc` channel to TranscriptionManager
5. TranscriptionManager receives chunks with **zero delay** — no polling, no disk reads
6. When sidecar exits, the FIFO reader gets EOF — clean shutdown

**Zero changes to the JarvisListen sidecar.** It already writes to whatever `--output` path it's given. The `nix` crate (already in Cargo.toml) provides `mkfifo()`.

### Why This Architecture

- **Silero VAD** gates the pipeline — during silence, only VAD runs (nearly zero CPU), prevents Whisper hallucinations on silence/noise
- **Vosk** provides instant feedback (<100ms) — users see text as they speak
- **Whisper** corrects to high accuracy (2-5% WER) — the final text is best-in-class
- All three are local, offline, privacy-preserving — no cloud dependency for competition
- **Provider-swappable**: The hybrid pipeline is wrapped in a single `HybridProvider` implementing `TranscriptionProvider` trait — TranscriptionManager doesn't know or care about the internals

### Phased Implementation

- **Phase 1 (Competition MVP)**: VAD + Vosk + Whisper — full hybrid pipeline. VAD gates silence, Vosk provides instant partials (<100ms gray text), Whisper corrects to high accuracy (black text). This is the competition demo.
- **Phase 2 (Post-Competition)**: Add AWS Transcribe Streaming as cloud provider — replace local engines with cloud

Design the provider trait so the HybridProvider can be swapped for an AwsTranscribeProvider without changing TranscriptionManager.

---

## Rust Crates

```toml
[dependencies]
silero-vad-rs = "0.1"                                    # Silero VAD - 1.8MB model, ONNX runtime
vosk = "0.3"                                             # Vosk - truly streaming STT, instant partials
whisper-rs = { version = "0.15", features = ["metal"] }  # Whisper with Metal GPU acceleration
```

### Crate Details

#### silero-vad-rs
- Uses `ort` crate for ONNX inference
- `VADIterator` struct for streaming audio
- 512-sample chunks at 16kHz (~32ms)
- <1ms processing per chunk
- Model size: 1.8MB

#### vosk-rs
- Safe FFI bindings around Vosk C API
- `Recognizer::partial_result()` — instant streaming partials
- `Recognizer::result()` / `final_result()` — complete utterances
- Returns JSON with words, confidence scores, timestamps
- Model: `vosk-model-small-en-us-0.15` — only **40MB**
- **Caveat**: macOS Apple Silicon native support is uncertain (has iOS ARM but desktop ARM64 unverified). Test early. If doesn't work, fall back to Phase 1 (VAD + Whisper only) or try Rosetta 2 (~20% latency overhead)

#### whisper-rs
- `WhisperContext` loads GGML model
- `state.full(params, &f32_audio)` runs batch inference
- Metal GPU acceleration: ~3x speedup on Apple Silicon
- Model: `ggml-base.en.bin` — **142MB**
- NOT truly streaming — it's repeated batch inference on windows
- Returns timestamps in centiseconds (multiply by 10 for milliseconds)

### Model Sizes
- Silero VAD: 1.8MB (model) + ~10MB runtime
- Vosk small-en-us: 40MB (model) + ~300MB runtime
- Whisper base.en: 142MB (model) + ~200MB runtime
- **Total: ~184MB models, ~510MB runtime** — acceptable for desktop app on Mac

---

## How It Integrates with the Existing App

### Current Recording Flow (already implemented)
1. User clicks "Start Recording" in React UI
2. Tauri backend (`recording.rs`) spawns JarvisListen sidecar with `--mono --sample-rate 16000 --output <filepath>`
3. JarvisListen writes PCM chunks (3200 bytes per 100ms) to the output file
4. User clicks "Stop Recording" -> backend sends SIGTERM -> sidecar flushes and exits
5. Recording appears in the UI list

### New Recording + Transcription Flow (replaces current)
1. User clicks "Start Recording"
2. `RecordingManager` creates a **named pipe (FIFO)** at a temp path
3. `RecordingManager` spawns sidecar with `--output <fifo_path>` (sidecar writes to FIFO, thinking it's a file)
4. `RecordingManager` spawns an **AudioRouter** tokio task that:
   a. Opens the FIFO for reading
   b. As chunks arrive (every 100ms): writes to actual PCM file AND sends via `mpsc` channel to TranscriptionManager
5. `TranscriptionManager` receives chunks from the channel (zero delay) and:
   a. **Runs Silero VAD** — skips silence
   b. **Feeds speech to Vosk** for instant partial text (is_final=false)
   c. **Buffers audio** into 3-second windows for Whisper
   d. **Feeds each window to Whisper** for accurate final text (is_final=true)
   e. **Emits Tauri events** (`transcription-update`) with segments
6. React frontend receives events:
   - Partial segments (from Vosk): displayed in **light gray** text
   - Final segments (from Whisper): **replace** matching partials with normal black text
7. User clicks "Stop Recording" -> SIGTERM to sidecar -> sidecar flushes and closes FIFO -> AudioRouter gets EOF -> TranscriptionManager drains remaining audio -> `transcription-stopped` event emitted

---

## Existing Code You Must Integrate With

### Rust Backend Files (read these for context)
- **`src-tauri/src/lib.rs`**: Tauri app setup. Uses `.manage()` for state, `.invoke_handler()` for commands. You'll add TranscriptionManager to managed state here.
- **`src-tauri/src/recording.rs`**: `RecordingManager` struct. Uses `std::sync::Mutex`. Spawns sidecar, sends SIGTERM for stop. You'll trigger transcription start/stop from `start_recording()` and `stop_recording()`.
- **`src-tauri/src/commands.rs`**: Tauri command handlers. Pattern: `State<'_, Mutex<RecordingManager>>`. You'll add `get_transcript` and `get_transcription_status` commands.
- **`src-tauri/src/files.rs`**: `FileManager` with `get_recordings_dir()`. Constants: `SAMPLE_RATE=16000`, `BYTES_PER_SAMPLE=2`, `CHANNELS=1`.
- **`src-tauri/Cargo.toml`**: Current deps include tauri 2, tokio, serde, nix, dirs, chrono. You'll add `whisper-rs`, `silero-vad-rs`, and optionally `vosk`.

### Frontend Files (read these for context)
- **`src/state/types.ts`**: `AppState`, `AppAction` union type, event payload interfaces. You'll add transcription-related types here.
- **`src/state/reducer.ts`**: `appReducer` function. You'll add transcription action handlers.
- **`src/hooks/useRecording.ts`**: Uses `useTauriEvent` for event listeners, `invoke` for commands. You'll add transcription event listeners.
- **`src/App.tsx`**: Main component. You'll add a TranscriptDisplay component.

### Audio Format (from JarvisListen)
- **16kHz, 16-bit signed little-endian (s16le), mono**
- **3200 bytes per 100ms chunk** (16000 samples/sec * 2 bytes/sample * 0.1 sec)
- **32000 bytes per second** of audio
- whisper-rs expects **32-bit float at 16kHz mono** — use `whisper_rs::convert_integer_to_float_audio(&[i16])` to convert
- Vosk expects **i16 samples at 16kHz mono** — can feed s16le bytes directly
- Silero VAD expects **f32 samples at 16kHz mono** in 512-sample chunks

---

## Key Technical Details for the Design

### TranscriptionProvider Trait (Rust)
```rust
pub trait TranscriptionProvider: Send + Sync {
    fn name(&self) -> &str;
    fn initialize(&mut self, config: &TranscriptionConfig) -> Result<(), Box<dyn Error>>;
    fn transcribe(&mut self, audio: &[f32]) -> Result<Vec<TranscriptionSegment>, Box<dyn Error>>;
}
```
- Must be object-safe (usable as `Box<dyn TranscriptionProvider>`)
- `transcribe()` takes f32 audio (16kHz mono), returns text segments with timestamps
- `TranscriptionSegment`: `{ text: String, start_ms: i64, end_ms: i64, is_final: bool }`
- No engine-specific types in the trait interface
- The `is_final` field is critical: Vosk partials use `is_final=false`, Whisper finals use `is_final=true`

### HybridProvider (Composite Provider)
```rust
pub struct HybridProvider {
    vad: SileroVad,               // gatekeeper — always active
    vosk: Option<VoskRecognizer>, // instant partials (Option for graceful degradation if macOS ARM fails)
    whisper: WhisperProvider,     // accurate finals
}

impl TranscriptionProvider for HybridProvider {
    fn name(&self) -> &str { "hybrid-vad-whisper" }

    fn transcribe(&mut self, audio: &[f32]) -> Result<Vec<TranscriptionSegment>> {
        // 1. VAD: check if speech present
        if !self.vad.contains_speech(audio) {
            return Ok(vec![]); // skip silence — saves CPU, prevents hallucinations
        }

        let mut segments = vec![];

        // 2. Vosk: get fast partial
        if let Some(vosk) = &mut self.vosk {
            let vosk_partial = vosk.accept_waveform(audio);
            if let Some(partial_text) = vosk_partial {
                segments.push(TranscriptionSegment {
                    text: partial_text,
                    start_ms: ...,
                    end_ms: ...,
                    is_final: false, // partial — displayed as gray text
                });
            }
        }

        // 3. Whisper: get accurate final (batched on 3s windows)
        let whisper_segments = self.whisper.transcribe(audio)?;
        for seg in whisper_segments {
            segments.push(TranscriptionSegment {
                is_final: true, // final — replaces matching gray partials
                ..seg
            });
        }

        Ok(segments)
    }
}
```

**Key**: TranscriptionManager sees just a `Box<dyn TranscriptionProvider>`. Whether it's HybridProvider, WhisperProvider alone, or a future AwsTranscribeProvider — TranscriptionManager doesn't change.

### WhisperProvider Implementation
- Uses `whisper_rs::WhisperContext::new_with_params(model_path, WhisperContextParameters::default())`
- Creates state via `ctx.create_state()`
- Runs inference: `state.full(params, &f32_audio)` with `SamplingStrategy::Greedy { best_of: 1 }`
- Extracts segments: `state.full_n_segments()`, `state.full_get_segment_text(i)`, `state.full_get_segment_t0(i)`, `state.full_get_segment_t1(i)`
- Whisper returns timestamps in centiseconds (multiply by 10 for milliseconds)
- Default model path: `~/.jarvis/models/ggml-base.en.bin`
- Override via `JARVIS_WHISPER_MODEL` environment variable
- MUST enable Metal GPU acceleration on macOS via the `features = ["metal"]` flag (~3x speedup)
- Pass previous segment tokens as prompt to next inference for continuity (context carryover)

### Voice Activity Detection (Silero VAD)
- Process 512-sample chunks at 16kHz (~32ms per chunk)
- <1ms processing per chunk (essentially free)
- Maintain a **100ms pre-roll buffer** to catch word onsets that VAD might miss
- Maintain a **300ms post-roll buffer** to catch word endings
- When no speech detected: skip all downstream processing (no Vosk, no Whisper)
- **Result: 4x speed improvement + zero hallucinations** on silence

### VoskProvider
- Uses `vosk::Recognizer` with `vosk::Model`
- `recognizer.accept_waveform(&i16_samples)` feeds audio
- `recognizer.partial_result()` returns instant partial text
- `recognizer.result()` / `recognizer.final_result()` returns complete utterances
- Model path: `~/.jarvis/models/vosk-model-small-en-us-0.15/`
- Override via `JARVIS_VOSK_MODEL` environment variable
- **Caveat**: If Vosk doesn't work on macOS Apple Silicon, gracefully degrade to VAD + Whisper only

### AudioRouter (New Component — Replaces PcmTailer)
- Created by `RecordingManager` when recording starts
- Creates a FIFO using `nix::unistd::mkfifo()` at a temp path (e.g., `/tmp/jarvis_audio_<session_id>.fifo`)
- Uses `tokio::net::unix::pipe::OpenOptions::new().open_receiver(fifo_path)` for async FIFO reading
- **Startup sequence** (critical for avoiding race conditions):
  1. Rust creates FIFO with `mkfifo()`
  2. Rust spawns a tokio task that opens FIFO for reading (blocks until writer connects)
  3. Rust spawns Swift sidecar with `--output <fifo_path>` (sidecar opens FIFO for writing, unblocks reader)
- Reads chunks as they arrive from the sidecar (3200 bytes per 100ms)
- For each chunk received:
  1. Appends to the actual PCM file (for recording/playback later)
  2. Sends via `tokio::sync::mpsc::Sender<Vec<u8>>` to TranscriptionManager
- When sidecar closes the FIFO (EOF), AudioRouter signals completion
- Cleans up (unlinks) the FIFO file on drop
- **macOS caveat**: Do NOT use `read_write(true)` on FIFO open — that's Linux-only. Use read-only mode.
- **macOS pipe buffer**: 16KB default (expandable to 64KB by kernel). At 32KB/s throughput, this is more than sufficient.
- **Backpressure**: If Rust reader falls behind, sidecar's `write()` blocks automatically — natural flow control, no data loss
- **SIGPIPE safety**: JarvisListen already ignores SIGPIPE (main.swift line 85: `Darwin.signal(SIGPIPE, SIG_IGN)`), so if reader crashes, sidecar gets `EPIPE` instead of terminating
- **Result**: Transcription receives audio with **zero delay** instead of 200ms polling

### TranscriptionManager
- Uses `tokio::sync::Mutex` (NOT `std::sync::Mutex`) because it holds locks across async await points
- Owns the provider via `Arc<TokioMutex<Box<dyn TranscriptionProvider>>>`
- Has a `start(rx: mpsc::Receiver<Vec<u8>>)` method that spawns a tokio background task
- Receives PCM chunks from the `mpsc` channel (sent by AudioRouter)
- Has a `stop()` method that signals the task to finish remaining audio and exit
- Uses `tokio::sync::watch` channel for the stop signal
- Accumulates full transcript in `Vec<TranscriptionSegment>`
- **Does NOT know about VAD, Vosk, or Whisper** — it just calls `provider.transcribe(audio)` and emits events
- **Does NOT know about FIFO or file I/O** — it just reads from a channel

### Audio Windowing
- Accumulate PCM bytes in an `AudioBuffer`
- When buffer reaches **3 seconds** (96,000 bytes): extract window, transcribe, advance
- Sliding window: advance by **2.5 seconds** (80,000 bytes), keep **0.5 second overlap**
- On recording stop: transcribe remaining if >= 1 second
- Convert s16le bytes -> i16 samples -> f32 via whisper_rs helper
- **Why 3s instead of 5s**: Lower latency — gives ~2s between transcription updates (vs ~4s with 5s windows). Better responsiveness for real-time feel.

### Tauri Events to Emit
- `transcription-started` — when transcription begins for a recording
- `transcription-update` — `{ segment: { text, start_ms, end_ms, is_final } }` — per segment
- `transcription-stopped` — `{ transcript: TranscriptionSegment[] }` — full transcript
- `transcription-error` — `{ message: string }` — non-fatal errors

### New Tauri Commands
- `get_transcript` -> `Vec<TranscriptionSegment>` — returns accumulated transcript
- `get_transcription_status` -> `TranscriptionStatus` (idle/active/error/disabled)

### Frontend State Additions
Add to `AppState`:
```typescript
transcriptionStatus: "idle" | "active" | "error" | "disabled";
transcript: TranscriptionSegment[];
transcriptionError: string | null;
```

Add `TranscriptionSegment` type:
```typescript
interface TranscriptionSegment {
  text: string;
  start_ms: number;
  end_ms: number;
  is_final: boolean;
}
```

Add to `AppAction`:
```typescript
| { type: "TRANSCRIPTION_STARTED" }
| { type: "TRANSCRIPTION_UPDATE"; segment: TranscriptionSegment }
| { type: "TRANSCRIPTION_STOPPED"; transcript: TranscriptionSegment[] }
| { type: "TRANSCRIPTION_ERROR"; message: string }
| { type: "CLEAR_TRANSCRIPT" }
```

### TranscriptDisplay Component
- Scrollable container showing transcript segments
- Auto-scrolls to latest text
- **Final segments** (`is_final=true`): normal text color (Whisper output)
- **Partial segments** (`is_final=false`): lighter/gray text color (Vosk output)
- When a final segment arrives with overlapping timestamps, it **replaces** the matching partial segments
- Shows "Transcribing..." indicator when active
- Shows error indicator if transcription fails
- Clears on new recording start

### Partial → Final Replacement Logic (Frontend)
When a `transcription-update` event arrives with `is_final=true`:
1. Find any existing partial segments (`is_final=false`) with overlapping time ranges
2. Remove those partials from the display
3. Insert the final segment in their place
4. This creates the gray→black text replacement UX

---

## Critical Constraints

1. **Recording NEVER blocked by transcription**: Transcription errors must never prevent or interrupt recording. If models are missing, recording works — just no transcription.
2. **Graceful degradation**: If Whisper model missing → status `Disabled`, skip all transcription. If Vosk model missing → skip Vosk, use VAD + Whisper only. If VAD model missing → skip VAD, use Whisper only. App startup must not fail.
3. **Background thread**: All transcription runs in a tokio background task. Never block the Tauri main thread or the UI.
4. **Sequential Whisper processing**: Process Whisper windows one at a time (not parallel) to limit CPU usage. Vosk runs inline with audio chunks (lightweight).
5. **VAD gates everything**: During silence, only VAD runs — nearly zero CPU. Vosk and Whisper only process speech-containing audio.
6. **All errors to stderr**: Transcription errors go to stderr, never stdout.
7. **whisper-rs requires cmake**: The `whisper-rs` dependency requires cmake to build whisper.cpp from source.
8. **Metal GPU acceleration**: MUST enable on macOS via `features = ["metal"]` for ~3x Whisper speedup.

---

## Rust Module Structure

```
src-tauri/src/transcription/
├── mod.rs                  # Module exports
├── provider.rs             # TranscriptionProvider trait + TranscriptionSegment + TranscriptionConfig types
├── audio_router.rs         # AudioRouter (FIFO reader → PCM file writer + mpsc sender)
├── vad.rs                  # Silero VAD wrapper (speech detection, pre-roll/post-roll buffers)
├── whisper_provider.rs     # WhisperProvider implementation (batch inference on windows)
├── vosk_provider.rs        # VoskProvider implementation (instant partials)
├── hybrid_provider.rs      # HybridProvider composite (VAD + Vosk + Whisper behind TranscriptionProvider trait)
├── manager.rs              # TranscriptionManager (orchestration, event emission — engine-agnostic)
└── audio_buffer.rs         # AudioBuffer struct (accumulation, windowing, s16le->f32 conversion)
```

---

## Requirements to Cover

1. **Audio Routing via Named Pipe** — FIFO creation, sidecar writes to FIFO, Rust reads and routes to PCM file + mpsc channel, zero-delay streaming, clean EOF shutdown
2. **Audio Windowing** — 3s default window (configurable 2-30s), sliding with 0.5s overlap, drain on stop
3. **TranscriptionProvider Trait** — object-safe, Send+Sync, initialize/transcribe/name, no engine-specific types, `is_final` field in segments
4. **WhisperProvider** — whisper-rs crate, GGML model, s16le->f32, Greedy sampling, Metal GPU, context carryover, configurable model path
5. **Event Emission** — transcription-update per segment (with is_final), transcription-started, transcription-stopped, transcription-error
6. **Frontend Display** — scrollable transcript, auto-scroll, partial (gray) vs final (normal) styling, partial→final replacement by timestamp overlap, error indicator
7. **Tauri Integration** — auto-start with recording, get_transcript command, get_transcription_status command
8. **Model Management** — default paths (~/.jarvis/models/), env var overrides for each model, missing model = graceful degradation
9. **Performance** — background tokio task, sequential Whisper windows, configurable thread count via JARVIS_WHISPER_THREADS, Metal GPU enabled
10. **Error Handling** — never block recording, graceful degradation per engine, retry transient IO errors, all errors to stderr
11. **Audio Format** — 16kHz s16le mono, byte calculations, format conversion for each engine
12. **Voice Activity Detection** — Silero VAD gating, 512-sample chunks, <2ms per chunk, 100ms pre-roll, 300ms post-roll, skip silence
13. **Fast Partial Results** — Vosk instant partials (<100ms), emit as is_final=false, Whisper finals replace partials, macOS ARM caveat and fallback

---

## Model Default Paths and Environment Variables

| Model | Default Path | Env Var Override | Size |
|-------|-------------|------------------|------|
| Whisper | `~/.jarvis/models/ggml-base.en.bin` | `JARVIS_WHISPER_MODEL` | 142MB |
| Silero VAD | `~/.jarvis/models/silero_vad.onnx` | `JARVIS_VAD_MODEL` | 1.8MB |
| Vosk | `~/.jarvis/models/vosk-model-small-en-us-0.15/` | `JARVIS_VOSK_MODEL` | 40MB |

---

## Follow the Same Spec Format As

Look at `.kiro/specs/jarvis-listen/` for the exact format:
- **requirements.md**: Introduction, Glossary, numbered Requirements with User Stories + Acceptance Criteria using RFC-2119 keywords (WHEN, THE System SHALL, IF, THEN)
- **design.md**: Overview, Architecture diagram, Component descriptions, Data Models (with actual Rust/TS code), Algorithms (step-by-step), Error Handling categories, Correctness Properties (20+ formal properties with "Validates: Requirements X.Y"), Testing Strategy
- **tasks.md**: Checkbox task list, sub-tasks with requirement references, property tests as optional sub-tasks marked with `*`, checkpoints for validation

---

## Reference Implementations

These are existing open-source projects that use similar patterns — study for inspiration:

- [Vosper](https://github.com/appvoid/vosper) — Vosk feedback + Whisper background (the hybrid pattern)
- [Keyless](https://github.com/hate/keyless) — Rust pipeline: cpal → VAD → Whisper, local-only
- [Pothook](https://github.com/acknak/pothook) — Tauri + whisper-rs GUI app
- [Recordscript](https://github.com/Recordscript/recordscript) — Tauri screen recorder with whisper-rs
- [Vibe](https://github.com/thewh1teagle/vibe) — Tauri multilingual transcription app
