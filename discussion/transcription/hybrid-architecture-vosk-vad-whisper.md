# Hybrid Architecture: Silero VAD + Vosk + Whisper

## The Idea

Three engines, each doing what it's best at:

| Engine | Role | Latency | Accuracy |
|--------|------|---------|----------|
| **Silero VAD** | Gate: is anyone speaking? | ~1ms per 30ms chunk | N/A (binary: speech/silence) |
| **Vosk** | Fast partials: show text instantly | <100ms | 10-15% WER (good enough for partials) |
| **Whisper** | Accurate finals: correct the text | 1-2s | 2-5% WER (best-in-class) |

## Pipeline

```
PCM Audio (16kHz mono)
    │
    ▼
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
       │         │    │  (partials)│ ← display immediately in gray text
       │         │    └───────────┘
       │         │
       │         └──→ ┌───────────┐
       │              │  Whisper   │ ← batch on 3s windows, ~1-2s
       │              │  (finals)  │ ← replace gray text with black text
       │              └───────────┘
       │
       ▼
  UI: "Hello, how are you doing today?"
       ^^^^^ gray (Vosk partial) → replaced by black (Whisper final)
```

## What the User Sees

1. **Instant** (~100ms): Gray text appears from Vosk as they speak
2. **1-2 seconds later**: Gray text replaced by accurate black text from Whisper
3. **Feels like magic**: Typing speed transcription with high accuracy

This is the "Whisper-VOSK Loop" pattern documented on Medium.

## Rust Crates (All Exist)

```toml
[dependencies]
silero-vad-rs = "0.1"         # Silero VAD - 1.8MB model, ONNX runtime
vosk = "0.3"                   # Vosk - truly streaming STT
whisper-rs = { version = "0.15", features = ["metal"] }  # Whisper with Metal GPU
```

### silero-vad-rs
- Uses `ort` crate for ONNX inference
- `VADIterator` struct for streaming audio
- 512-sample chunks at 16kHz (~32ms)
- <1ms processing per chunk

### vosk-rs (Bear-03/vosk-rs)
- Safe FFI bindings around Vosk C API
- `Recognizer::partial_result()` — instant streaming partials
- `Recognizer::result()` / `final_result()` — complete utterances
- Returns JSON with words, confidence scores, timestamps
- **Model**: `vosk-model-small-en-us-0.15` — only **40MB**

### whisper-rs
- `WhisperContext` loads GGML model
- `state.full(params, &f32_audio)` runs batch inference
- Metal GPU acceleration: ~3x speedup on Apple Silicon
- **Model**: `ggml-base.en.bin` — **142MB**

## How It Fits Our Provider Trait

The `TranscriptionProvider` trait doesn't need to change. Instead, we create a **composite provider**:

```rust
pub struct HybridProvider {
    vad: SileroVad,
    vosk: VoskRecognizer,      // fast partials
    whisper: WhisperProvider,   // accurate finals
}

impl TranscriptionProvider for HybridProvider {
    fn name(&self) -> &str { "hybrid-vosk-whisper" }

    fn transcribe(&mut self, audio: &[f32]) -> Result<Vec<TranscriptionSegment>> {
        // 1. VAD: check if speech present
        if !self.vad.contains_speech(audio) {
            return Ok(vec![]); // skip silence
        }

        // 2. Vosk: get fast partial (feed i16 samples)
        let vosk_partial = self.vosk.accept_waveform(audio);
        if let Some(partial_text) = vosk_partial {
            // emit partial segment (is_final = false)
        }

        // 3. Whisper: get accurate final
        let whisper_segments = self.whisper.transcribe(audio)?;
        // emit final segments (is_final = true, replaces partials)

        Ok(whisper_segments)
    }
}
```

This means **zero changes to TranscriptionManager** — it just sees a provider that returns segments.

## Concerns and Mitigations

### 1. Vosk macOS Apple Silicon Support
- **Status**: Vosk has ARM builds for iOS (`aarch64-apple-ios`) but macOS ARM64 native support is uncertain
- **Mitigation**: Test early. If Vosk doesn't work on Apple Silicon, fall back to VAD + Whisper only (still a big win)
- **Alternative**: Use Vosk via Rosetta 2 (x86 emulation, adds ~20% latency overhead)

### 2. Three Models to Bundle
- Silero VAD: 1.8MB (tiny)
- Vosk small-en-us: 40MB (small)
- Whisper base.en: 142MB (medium)
- **Total: ~184MB** — acceptable for desktop app

### 3. Memory Usage
- Silero VAD: ~10MB runtime
- Vosk: ~300MB runtime
- Whisper: ~200MB runtime (base.en)
- **Total: ~510MB** — fine for a Mac

### 4. Synchronization Complexity
- Vosk partials and Whisper finals arrive at different times
- Need to track which partials get replaced by which finals
- **Solution**: Use segment timestamps to align and replace

### 5. CPU Usage
- All three engines running simultaneously
- **Mitigation**: VAD gates the other two — during silence, only VAD runs (nearly zero CPU)
- Whisper runs sequentially, not parallel, per our existing spec
- Vosk is very lightweight

## Comparison: Approaches

| Approach | Latency to First Text | Final Accuracy | Complexity | CPU Usage |
|----------|----------------------|----------------|------------|-----------|
| **Whisper only** | 2-3s | Best (2-5% WER) | Low | Medium |
| **VAD + Whisper** | 2-3s | Best | Low-Medium | Low (skips silence) |
| **VAD + Vosk + Whisper** | <100ms | Best | Medium-High | Medium |
| **Vosk only** | <100ms | Lower (10-15% WER) | Low | Low |

## Recommendation

### Phase 1 (Competition MVP): VAD + Whisper
- Add Silero VAD to gate Whisper (biggest bang for buck)
- 4x speed improvement, no hallucinations
- Simple implementation, low risk
- Latency: 2-3s (acceptable for demo)

### Phase 2 (If Time Permits): Add Vosk for Instant Partials
- Add Vosk for <100ms partial results
- Show gray → black text replacement UX
- Test Vosk on macOS Apple Silicon first
- Higher complexity but impressive demo effect

### Phase 3 (Post-Competition): Add Cloud Provider
- AWS Transcribe Streaming as cloud fallback
- Replace Vosk partials with AWS interim results
- Replace Whisper finals with AWS finals
- Same HybridProvider pattern, just different engines

## Impact on Kiro Instructions

If we go with Phase 1 (VAD + Whisper), add to requirements:

```
### Requirement 12: Voice Activity Detection

1. THE System SHALL use Silero VAD to detect speech before transcription
2. WHEN a window contains no speech, THE System SHALL skip it (no Whisper call)
3. THE VAD SHALL process 512-sample chunks (~32ms) in <2ms per chunk
4. THE VAD SHALL maintain a 100ms pre-roll buffer to catch word onsets
5. THE VAD SHALL maintain a 300ms post-roll buffer to catch word endings
```

If we go with Phase 2 (VAD + Vosk + Whisper), additionally:

```
### Requirement 13: Fast Partial Results (Vosk)

1. THE System SHALL provide Vosk-based instant partial transcription (<100ms latency)
2. WHEN Vosk produces a partial result, THE System SHALL emit it as is_final=false
3. WHEN Whisper produces a final result for the same audio, THE System SHALL emit it as is_final=true
4. THE Frontend SHALL replace partial segments with final segments based on timestamp overlap
5. THE Frontend SHALL display partial segments in lighter color, final segments in normal color
```

## Reference Implementations

- [Vosper](https://github.com/appvoid/vosper) — Vosk feedback + Whisper background
- [Keyless](https://github.com/hate/keyless) — Rust pipeline: cpal → VAD → Whisper, local-only
- [Pothook](https://github.com/acknak/pothook) — Tauri + whisper-rs GUI app
- [Recordscript](https://github.com/Recordscript/recordscript) — Tauri screen recorder with whisper-rs
