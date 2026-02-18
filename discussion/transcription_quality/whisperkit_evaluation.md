# WhisperKit (Apple-Native) Evaluation

> Research date: Feb 2026 | Context: Jarvis uses whisper.cpp via whisper-rs. Large-v3-q5_0 model works but inference is slow.

---

## TL;DR

WhisperKit runs the **full Whisper pipeline on Apple Neural Engine** (not just the encoder like whisper.cpp + CoreML). This makes the decoder dramatically faster for large models — exactly our bottleneck. Native streaming means we could replace our entire 3-engine pipeline (Silero VAD + Vosk + whisper.cpp) with a single WhisperKit process.

**Verdict: Worth integrating as a toggleable option via sidecar.**

---

## 1. What Is WhisperKit

- **Maintainer**: Argmax Inc. (founded Nov 2023)
- **Repo**: [github.com/argmaxinc/WhisperKit](https://github.com/argmaxinc/WhisperKit) — ~5,600 stars, 35 contributors
- **License**: MIT (free for commercial use)
- **Current Version**: v0.15.x (34 releases, pre-1.0 but active)
- **Paper**: [ICML 2025](https://arxiv.org/abs/2507.10860) — peer-reviewed
- **Platform**: macOS 14+, iOS 17+, Apple Silicon only
- **WWDC 2025**: Apple's SpeechAnalyzer will integrate into WhisperKit

Not a thin CoreML wrapper — it's a ground-up re-implementation of Whisper inference optimized for ANE.

---

## 2. Why It's Faster Than whisper.cpp

| Aspect | whisper.cpp (our current) | WhisperKit |
|---|---|---|
| Encoder | CoreML/ANE (optional) | CoreML/ANE (native, optimized) |
| **Decoder** | **CPU only** (ANE slower for whisper.cpp) | **ANE with stateful KV cache** |
| Streaming | Chunked re-encoding | Native block-diagonal attention |
| Weight compression | GGML quantization | OD-MBP palettization (ANE-optimized) |
| KV cache | Standard memory alloc | In-place stateful updates |

**The decoder is our bottleneck.** whisper.cpp runs the decoder on CPU even with CoreML enabled. WhisperKit runs both encoder AND decoder on ANE.

### Key Architectural Innovations

1. **Block-Diagonal Attention Masking**: Encoder processes partial audio natively (enables streaming without re-encoding)
2. **Stateful KV Caching**: 45% decoder latency reduction, 75% energy reduction
3. **OD-MBP Compression**: large-v3 compressed from 1.6GB to 0.6GB with only +0.03% WER degradation
4. **Dual-Stream Decoding**: Hypothesis text (~0.45s) + Confirmed text (~1.7s)

---

## 3. Benchmarks

### WhisperKit vs Cloud (ICML 2025, TIMIT dataset)

| System | Hypothesis Latency | Confirmed Latency | WER |
|---|---|---|---|
| **WhisperKit (on-device)** | **0.45s** | **1.7s** | **2.0%** |
| Deepgram nova-3 (cloud) | 0.83s | 1.7s | 2.0% |
| Fireworks large-v3-turbo (cloud) | 0.45s | N/A | 4.72% |
| OpenAI gpt-4o-transcribe (cloud) | N/A | 2.2s+ | 3.2% |

### Component Latency (M3 ANE)

| Component | Before Optimization | After | Reduction |
|---|---|---|---|
| Encoder | 612ms | 218ms | 65% |
| Decoder (per forward pass) | 8.4ms | 4.6ms | 45% |
| Energy per decoder pass | 1.5W | 0.3W | 75% |

### Mac Whisper Speedtest (M4 24GB, batch mode)

| Implementation | Avg Time | Model |
|---|---|---|
| whisper.cpp | 1.23s | large-v3-turbo-q5_0 |
| WhisperKit | 2.22s | large-v3 (full, uncompressed) |

**Caveat**: Not apples-to-apples — WhisperKit used full large-v3 (2.9GB). With turbo or compressed models, WhisperKit would be significantly faster. The key win is in **streaming latency**, not batch throughput.

---

## 4. What This Means for Jarvis

### Current Pipeline (3 engines)

```
Microphone → VAD (Silero) → Vosk (partials) → Whisper (finals)
                                ↓                    ↓
                          gray text (~100ms)    final text (~3-5s)
```

### With WhisperKit (1 engine)

```
Microphone → WhisperKit
                ↓                    ↓
          hypothesis (~0.45s)   confirmed (~1.7s)
```

**Simplification**: WhisperKit's dual-stream replaces both Vosk partials AND Whisper finals. It includes its own VAD, so Silero could be dropped too.

### Performance Comparison (estimated for our use case)

| Metric | Current (whisper.cpp large-v3-q5_0) | WhisperKit (large-v3-turbo compressed) |
|---|---|---|
| First partial | ~100ms (Vosk) | ~450ms (hypothesis) |
| Final text | ~3-5s (5s window + inference) | ~1.7s (confirmed) |
| WER | ~5-6% | ~2-3% |
| Engines | 3 (VAD + Vosk + Whisper) | 1 |
| Energy | Moderate | Low (75% less decoder energy) |

Trade-off: Vosk partials appear faster (100ms vs 450ms), but WhisperKit's hypothesis text is **accurate** while Vosk partials are often wrong.

---

## 5. Model Support

WhisperKit uses **CoreML format** (`.mlmodelc`), NOT GGML. Cannot reuse our existing GGML models.

Models hosted at [huggingface.co/argmaxinc/whisperkit-coreml](https://huggingface.co/argmaxinc/whisperkit-coreml):

| Model | Size | Notes |
|---|---|---|
| openai_whisper-tiny.en | ~75MB | Fast, low accuracy |
| openai_whisper-base.en | ~142MB | Baseline |
| openai_whisper-small.en | ~466MB | Good balance |
| openai_whisper-large-v3_turbo | ~1.5GB | Fast large model |
| openai_whisper-large-v3_turbo (compressed) | ~632MB | Best for our use case |
| openai_whisper-large-v3 (compressed) | ~947MB | Highest accuracy |
| distil-large-v3 | ~1.5GB | Distilled variant |
| distil-large-v3_594MB | ~594MB | Compressed distilled |

**Recommendation**: `openai_whisper-large-v3_turbo` compressed (~632MB) — best speed/accuracy for real-time.

---

## 6. Integration Options (Ranked)

### Option A: whisperkit-cli as Sidecar (RECOMMENDED)

```bash
brew install whisperkit-cli
```

WhisperKit CLI provides:
- **Local HTTP server**: `whisperkit-cli serve --port 8080` (OpenAI-compatible API)
- **SSE streaming**: Real-time results via Server-Sent Events
- **Microphone streaming**: `whisperkit-cli transcribe --stream`

**Integration plan**:
1. Bundle `whisperkit-cli` as a Tauri sidecar binary
2. Start it as a local server on app launch
3. Send audio via HTTP to `localhost:8080/v1/audio/transcriptions`
4. Receive streaming results via SSE
5. Toggle between whisper.cpp and WhisperKit in Settings

**Effort**: 2-3 days | **Risk**: Low

```
Settings toggle:
  [x] WhisperKit (Apple Neural Engine) — Recommended for Apple Silicon
  [ ] whisper.cpp (Metal GPU) — Cross-platform compatible
```

### Option B: Swift FFI via swift-bridge

```rust
#[swift_bridge::bridge]
mod ffi {
    extern "Swift" {
        fn transcribe_audio(audio_path: String) -> String;
    }
}
```

**Effort**: 1-2 weeks | **Risk**: Medium (build system complexity, async bridging)

### Option C: Manual C ABI Bridge

Write thin Objective-C/Swift wrapper exposing C functions, call from Rust via `extern "C"`.

**Effort**: 1 week | **Risk**: Medium

### Why Sidecar Wins

- Already have sidecar pattern (JarvisListen)
- Process isolation (WhisperKit crash doesn't crash app)
- No build system changes (no Swift Package Manager in Cargo build)
- OpenAI-compatible API is well-documented
- Easy to toggle on/off

---

## 7. Risks & Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| macOS only | No Linux/Windows | Keep whisper.cpp as fallback for cross-platform |
| Pre-1.0 API | Breaking changes | Pin CLI version, bundle binary |
| Separate model format | Maintain two model sets | Download CoreML models on first WhisperKit enable |
| Sidecar IPC latency | Audio transfer overhead | Use local HTTP server with streaming (SSE) |
| Binary size | +50MB for CLI | Acceptable for desktop app |
| First-run model compilation | ANE compiles CoreML model (~2-4 min) | Show progress indicator, cache after first run |

---

## 8. Implementation Plan

### Phase 1: Validate (1 day)

```bash
# Install and test
brew install whisperkit-cli

# Test batch transcription
whisperkit-cli transcribe \
  --model "openai_whisper-large-v3_turbo" \
  --audio-path test_audio.wav

# Test streaming
whisperkit-cli transcribe \
  --model "openai_whisper-large-v3_turbo" \
  --stream

# Test local server
whisperkit-cli serve --port 8080
curl -X POST localhost:8080/v1/audio/transcriptions \
  -F file=@test_audio.wav -F model=large-v3-turbo
```

Compare wall-clock time and accuracy against current whisper-rs output.

### Phase 2: Sidecar Integration (2-3 days)

1. Add `whisperkit-cli` as Tauri sidecar binary
2. Create `WhisperKitProvider` that implements `TranscriptionProvider`
3. Provider starts local server, sends audio via HTTP, receives SSE results
4. Add Settings toggle: "Transcription Engine: WhisperKit / whisper.cpp"
5. Update `TranscriptionSettings` and `HybridProvider` to switch engines

### Phase 3: Pipeline Simplification (1-2 days)

When WhisperKit is selected:
- Disable Vosk partials (WhisperKit hypothesis replaces them)
- Optionally disable Silero VAD (WhisperKit has built-in VAD)
- Map WhisperKit dual-stream to existing event system:
  - `hypothesis` → `TranscriptionSegment { is_final: false }`
  - `confirmed` → `TranscriptionSegment { is_final: true }`

### Files to Change

```
jarvis-app/src-tauri/
  src/transcription/
    whisperkit_provider.rs  (NEW - WhisperKit sidecar wrapper)
    hybrid_provider.rs      (MODIFY - toggle engine)
    provider.rs             (MODIFY - add engine enum)
    mod.rs                  (MODIFY - export new provider)
  src/settings/
    manager.rs              (MODIFY - add engine setting)
  src/commands.rs           (MODIFY - expose engine toggle)
  binaries/
    whisperkit-cli-aarch64-apple-darwin  (NEW - sidecar binary)

jarvis-app/src/
  state/types.ts            (MODIFY - add engine setting type)
  components/Settings.tsx   (MODIFY - engine toggle UI)
```

---

## 9. Decision Matrix

| Criterion | whisper.cpp (current) | WhisperKit (proposed) |
|---|---|---|
| Accuracy (WER) | ~5-6% (large-v3-q5_0) | ~2-3% (large-v3-turbo) |
| Final latency | ~3-5s | ~1.7s |
| Partial latency | ~100ms (Vosk, low quality) | ~450ms (hypothesis, high quality) |
| Pipeline complexity | 3 engines | 1 engine |
| Energy consumption | Moderate | Low |
| Cross-platform | Yes | macOS only |
| Integration effort | Already done | 3-5 days |
| Model format | GGML | CoreML (separate download) |

---

## 10. Recommendation

**Integrate WhisperKit as a toggleable option (sidecar approach).**

- Keep whisper.cpp as default/fallback for cross-platform compatibility
- Add WhisperKit as the recommended engine on Apple Silicon
- Start with Phase 1 validation to confirm speedup on target hardware
- The ~2% WER and ~1.7s confirmed latency would be a significant upgrade over current ~5-6% WER and ~3-5s latency

---

## References

- [WhisperKit GitHub](https://github.com/argmaxinc/WhisperKit)
- [ICML 2025 Paper](https://arxiv.org/abs/2507.10860)
- [Argmax Blog](https://www.argmaxinc.com/blog/whisperkit)
- [Apple + Argmax (SpeechAnalyzer)](https://www.argmaxinc.com/blog/apple-and-argmax)
- [CoreML Models on HuggingFace](https://huggingface.co/argmaxinc/whisperkit-coreml)
- [WhisperKit CLI (Homebrew)](https://formulae.brew.sh/formula/whisperkit-cli)
- [WhisperKit Tools (model conversion)](https://github.com/argmaxinc/whisperkittools)
- [Mac Whisper Speedtest Benchmarks](https://github.com/anvanvan/mac-whisper-speedtest)
- [swift-bridge (Rust-Swift FFI)](https://github.com/chinedufn/swift-bridge)
- [Tauri Sidecar Docs](https://v2.tauri.app/develop/sidecar/)
- [Argmax Pricing](https://www.argmaxinc.com/pricing) (MIT free tier sufficient)
