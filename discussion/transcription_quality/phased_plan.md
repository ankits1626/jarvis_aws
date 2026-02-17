# Transcription Quality Improvement Plan

> Constraint: All processing must remain **on-device**. No cloud APIs.

---

## Current Baseline

| Aspect | Value | Problem |
|---|---|---|
| Model | `ggml-base.en.bin` (74MB) / `ggml-medium.en.bin` (1.5GB) | base ~10% WER, medium ~7.5% WER but slow |
| Decoding | `Greedy { best_of: 1 }` | Lowest quality decoding — no hypothesis exploration |
| Window | 3s fixed, 0.5s overlap | Short context hurts accuracy, mid-word splits |
| Acceleration | Metal GPU | Not using Apple Neural Engine (ANE) |
| Hallucination filtering | Silero VAD only | No post-transcription filtering |
| Partials | Vosk | Low accuracy, user never sees them (visibility bug) |

**Target**: Get WER below 5% with inference under 1s per window on Apple Silicon (M1+, 16GB RAM).

---

## Phase 1: Quick Wins (No Architecture Change)

**Goal**: Improve quality significantly with config/model changes only.
**Effort**: ~2-4 hours. **Risk**: Low.

### 1.1 Upgrade Whisper Model

Replace `ggml-base.en.bin` with **`ggml-large-v3-turbo-q5_0.bin`** (~600MB).

- 809M params (vs base's 74M) — dramatically better accuracy
- Quantized to q5_0 to keep memory reasonable
- WER: ~7.75% (vs base's ~10%)
- 6x faster than full large-v3 (decoder pruned from 32 to 4 layers)
- Download: `whisper.cpp` model download script or HuggingFace

**Fallback**: If too slow on target hardware, use **`ggml-distil-large-v3.bin`** (~1.5GB fp16, or quantize to q5_0). Same accuracy, optimized for long-form.

**Changes required**:
- Update default model path in `TranscriptionConfig::from_env()` and `from_settings()`
- Update `SettingsManager` default `whisper_model` value
- Add model download instructions or auto-download logic

### 1.2 Improve Decoding Parameters

Current: `Greedy { best_of: 1 }` — the absolute minimum quality setting.

**Option A** (quality + moderate speed cost):
```rust
// Beam search with 5 beams
let mut params = FullParams::new(SamplingStrategy::BeamSearch {
    beam_size: 5,
    patience: 1.0,
});
```

**Option B** (quality + less speed cost):
```rust
// Greedy with 5 candidates — cheaper than beam search
let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 5 });
```

**Recommendation**: Start with Option B (`best_of: 5`). If quality still insufficient, move to Option A.

### 1.3 Set Anti-Hallucination Parameters

These are **free** — they filter bad output, no compute cost:

```rust
// Reject segments where model thinks no speech is present
params.set_no_speech_thold(0.4); // default 0.6, lower = stricter

// Reject segments with low confidence
params.set_entropy_thold(2.4); // default, filters low-confidence gibberish

// Temperature fallback: retry with higher temp on failed decode
// whisper-rs handles this if set
params.set_temperature(0.0);
params.set_temperature_inc(0.2); // fallback: 0.0, 0.2, 0.4, 0.6, 0.8
```

### 1.4 Increase Window Duration

Change from 3s to **5s**:
- More context per window = better accuracy
- Only adds 2s to initial latency (5s vs 3s before first result)
- Overlap stays at 0.5s

```rust
// In manager.rs
let mut audio_buffer = AudioBuffer::new(5.0, 0.5, 16000);
```

Also make this configurable via `TranscriptionSettings`.

### Phase 1 Expected Outcome

| Metric | Before | After |
|---|---|---|
| WER | ~10% (base) | ~5-6% (turbo-q5_0 + beam/best_of) |
| Inference time | ~1.5s/3s window | ~1-2s/5s window (larger model but quantized) |
| Hallucinations | Frequent on silence | Reduced via thresholds |

---

## Phase 2: CoreML / ANE Acceleration

**Goal**: Cut inference time in half by leveraging Apple Neural Engine.
**Effort**: ~1-2 days. **Risk**: Medium (build system complexity).

### 2.1 Generate CoreML Encoder Model

The Whisper encoder is the bottleneck (~80% of inference time). Running it on ANE gives **3x speedup**.

Steps:
1. Install `coremltools` and `ane_transformers` Python packages
2. Convert large-v3-turbo encoder to CoreML format
3. Place `.mlmodelc` alongside the GGML model
4. Build whisper.cpp with `-DWHISPER_COREML=1`

**Caveat**: First run has a ~4 minute compilation penalty (ANE compiles model). Subsequent runs use cache.

**Alternative**: Use Apple's **MLX framework** instead of CoreML — avoids the ANECompilerService slow first-load. Requires a different integration path (mlx-whisper).

### 2.2 Update whisper-rs Build

Ensure the `whisper-rs` crate is compiled with CoreML support:
- Check if `whisper-rs` supports the `coreml` feature flag
- If not, may need to vendor whisper.cpp and build with CMake flags
- Update `Cargo.toml` and build script accordingly

### Phase 2 Expected Outcome

| Metric | Before (Phase 1) | After |
|---|---|---|
| Encoder time | ~800ms | ~250ms (ANE) |
| Total inference | ~1-2s/5s window | ~0.5-1s/5s window |
| First-run penalty | None | ~4 min (cached after) |

---

## Phase 3: Architecture Improvements

**Goal**: Smarter audio handling and better partial transcription quality.
**Effort**: ~3-5 days. **Risk**: Medium.

### 3.1 Adaptive VAD-Based Windowing

Instead of fixed 3s/5s windows, **accumulate audio until speech pause**:

```
Current (fixed):
  [====3s====][====3s====][====3s====]
  Words get cut at boundaries

Proposed (adaptive):
  [=== sentence 1 (2.1s) ===][=== sentence 2 (4.3s) ===]
  Natural boundaries, no mid-word splits
```

Implementation:
- Run Silero VAD continuously on incoming chunks
- When VAD detects silence > 300ms after speech, extract the accumulated speech segment
- Minimum segment: 1s. Maximum segment: 10s (force-flush).
- This gives Whisper complete utterances = much better accuracy

### 3.2 Replace Vosk with sherpa-onnx Streaming ASR

Vosk is low accuracy and the partials are never visible anyway (React batching issue). Replace with **sherpa-onnx**:

- **Streaming Zipformer** model: RTF 0.05 (20x faster than real-time)
- 45MB RAM footprint
- Already uses ONNX Runtime (same dependency as Silero VAD)
- Much better partial quality than Vosk
- True streaming: feed audio continuously, get updated partials in real-time

This also **fixes the Vosk visibility problem** — sherpa-onnx partials stream continuously on a separate thread, not blocked by Whisper inference.

Architecture change:
```
Current:
  AudioRouter → TranscriptionManager → HybridProvider (VAD → Vosk → Whisper sequential)

Proposed:
  AudioRouter → TranscriptionManager
                  ├── Stream 1: sherpa-onnx (continuous partials, separate thread)
                  └── Stream 2: Whisper (final transcription on VAD-segmented utterances)
```

### 3.3 Post-Transcription Hallucination Filtering

After Whisper returns segments, apply filters:

1. **Compression ratio check**: If `len(tokens) / len(text)` is abnormally high, likely hallucination
2. **Repetition detection**: If segment text repeats previous segment, likely hallucination
3. **no_speech probability**: whisper-rs exposes per-segment no_speech_prob — reject if > 0.6
4. **Confidence scoring**: Log-probability threshold on segments

### Phase 3 Expected Outcome

| Metric | Before (Phase 2) | After |
|---|---|---|
| WER | ~5-6% | ~3-4% (natural boundaries + filtering) |
| Partial quality | Poor (Vosk) / invisible | Good (sherpa-onnx) / visible in real-time |
| User experience | 5s wait, then text appears | Streaming partials immediately, finals on pause |

---

## Phase 4: Next-Gen Models (Exploratory)

**Goal**: Evaluate alternative ASR engines for potentially superior quality/speed tradeoff.
**Effort**: ~1-2 weeks (R&D). **Risk**: High (new integrations).

### 4.1 Moonshine ASR

- 27M params (Tiny) / 61M params (Base)
- **5x less compute** than Whisper tiny at same WER
- Variable-length processing (no fixed 30s padding) — ideal for adaptive windowing
- [moonshine.cpp](https://github.com/royshil/moonshine.cpp) — standalone C++ with ONNX Runtime
- Would replace whisper-rs entirely
- Paper: [arxiv.org/html/2410.15608v1](https://arxiv.org/html/2410.15608v1)

**Tradeoff**: Moonshine Base WER (~10%) is comparable to Whisper base.en, not large-v3-turbo. So this is a **speed play**, not a quality play. Best if latency is more important than accuracy.

Moonshine v2 adds **true streaming** with sliding-window self-attention — bounded latency for real-time applications.

### 4.2 WhisperKit (Apple-Native)

- Swift framework, runs Whisper natively on Apple Neural Engine
- **0.46s latency**, **2.2% WER** — best published on-device result
- [ICML 2025 paper](https://arxiv.org/abs/2507.10860)
- Caveat: Swift-only. Would need Swift-to-Rust FFI bridge (via C ABI)
- Significant integration effort but potentially the best possible on-device result

### 4.3 Parakeet TDT 1.1B (NVIDIA NeMo)

- ~8% WER, **RTF >2000x** (absurdly fast)
- 1.1B params, ~4GB VRAM
- ONNX export available for non-NVIDIA platforms
- English-only, CTC-based (no autoregressive decoder)
- Could work via ONNX Runtime on Apple Silicon

### Phase 4 Decision Matrix

| Engine | WER | Latency | Integration Effort | Best For |
|---|---|---|---|---|
| Whisper large-v3-turbo + CoreML | ~5% | ~0.5s | Low (Phase 1-2) | **Quality-first** |
| Moonshine Base (ONNX) | ~10% | ~0.1s | Medium | **Speed-first** |
| WhisperKit | ~2.2% | ~0.46s | High (Swift FFI) | **Best possible quality** |
| Parakeet TDT (ONNX) | ~8% | <0.1s | Medium | **Ultra-low-latency** |
| sherpa-onnx Zipformer | ~8-10% | Real-time | Medium | **Streaming partials** |

---

## Implementation Priority

```
Phase 1 (Days 1-2)     Phase 2 (Days 3-5)     Phase 3 (Week 2)        Phase 4 (Week 3+)
┌─────────────────┐    ┌─────────────────┐    ┌──────────────────┐    ┌──────────────────┐
│ 1.1 Model upgrade│    │ 2.1 CoreML model│    │ 3.1 Adaptive VAD │    │ 4.x Evaluate     │
│ 1.2 Beam search  │───>│ 2.2 Build flags │───>│ 3.2 sherpa-onnx  │───>│     alternatives │
│ 1.3 Thresholds   │    │                 │    │ 3.3 Hallucin.    │    │                  │
│ 1.4 5s window    │    │                 │    │     filtering    │    │                  │
└─────────────────┘    └─────────────────┘    └──────────────────┘    └──────────────────┘
  ~10% → ~5-6% WER      ~1s → ~0.5s            ~5% → ~3-4% WER        Evaluate tradeoffs
```

---

## Files That Change Per Phase

### Phase 1
- `whisper_provider.rs` — decoding params (beam search, thresholds)
- `provider.rs` — `TranscriptionConfig` defaults (window duration)
- `manager.rs` — `AudioBuffer::new()` window duration
- `settings/manager.rs` — default whisper model name
- Model download documentation

### Phase 2
- `Cargo.toml` — whisper-rs feature flags / build configuration
- `build.rs` or `CMakeLists.txt` — CoreML compile flags
- Model generation script (Python)

### Phase 3
- `manager.rs` — adaptive windowing logic, dual-stream architecture
- `hybrid_provider.rs` — decouple Vosk, add sherpa-onnx
- New file: `sherpa_provider.rs` — sherpa-onnx streaming wrapper
- `vad.rs` — continuous VAD for segmentation (not just gating)
- `provider.rs` — hallucination filter post-processing

### Phase 4
- Potentially new provider implementations
- FFI bridges if using WhisperKit

---

## References

- [Whisper Large V3 Turbo (HuggingFace)](https://huggingface.co/openai/whisper-large-v3-turbo) — 809M params, 6x faster
- [Distil-Whisper GGML Models](https://huggingface.co/distil-whisper/distil-large-v3-ggml) — 5x faster, 0.8% WER delta
- [Moonshine ASR Paper](https://arxiv.org/html/2410.15608v1) — 5x less compute at same WER
- [WhisperKit (ICML 2025)](https://arxiv.org/abs/2507.10860) — 0.46s latency, 2.2% WER on Apple Silicon
- [whisper.cpp CoreML Setup](https://panjas.com/blog/2024-11-26/coreml-models-for-whisper-cpp) — ANE encoder acceleration
- [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) — streaming ASR, RTF 0.05
- [moonshine.cpp](https://github.com/royshil/moonshine.cpp) — C++ ONNX implementation
- [Whisper Hallucination Research](https://arxiv.org/html/2501.11378v1) — VAD pre-filtering effectiveness
- [Beam Search + Min Lookahead](https://arxiv.org/html/2309.10299) — 2.26% avg WER reduction
- [Calm-Whisper: Hallucination Reduction](https://arxiv.org/html/2505.12969v1) — attention head calming
- [2025 Edge STT Benchmark (Ionio)](https://www.ionio.ai/blog/2025-edge-speech-to-text-model-benchmark-whisper-vs-competitors)
- [Best Open Source STT 2026 (Northflank)](https://northflank.com/blog/best-open-source-speech-to-text-stt-model-in-2026-benchmarks)
