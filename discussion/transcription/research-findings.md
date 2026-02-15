# Transcription Architecture Research Findings

## Executive Summary

Our current plan uses **whisper-rs (Rust bindings to whisper.cpp)** with **file-based PCM tailing** and **5-second sliding windows**. After extensive research, this is a solid starting point but has important caveats. The biggest discovery: **VAD (Voice Activity Detection) is critical** for production quality, and **whisper.cpp is NOT truly streaming** — it's repeated batch inference on windows.

---

## 1. whisper.cpp Reality Check

### It's NOT Streaming
whisper.cpp has no native streaming mode. The `stream` example is repeated batch inference on overlapping windows. Every call to `whisper_full()` processes a complete audio buffer from scratch.

### How "Real-Time" Actually Works
1. Audio captured into ring buffer
2. Every N milliseconds (default 3000ms, configurable to ~300ms), extract chunk
3. Prepend overlap from previous iteration
4. Run full batch inference via `whisper_full()`
5. If `keep_context` enabled, use previous tokens as prompt

### Latency Numbers on Apple Silicon

| Model | Hardware | Inference Time |
|-------|----------|---------------|
| base.en | M1/M2 | ~0.5-1s per 5s window |
| small.en | M1/M2 | ~1-2s per 5s window |
| large-v3-turbo (q5) + CoreML | Apple Silicon | ~1.23s |
| medium | M2 Max | ~3.3s (too slow for real-time) |

**Practical end-to-end latency with base.en + Metal**: **1.5-3 seconds**

### whisper-rs Crate Status
- **Current version**: 0.15.1 (Sept 2025, actively maintained)
- **Metal support**: `features = ["metal"]` — gives ~3x speedup
- **CoreML**: NOT exposed in whisper-rs (notable gap vs raw whisper.cpp)
- **VAD support**: New `WhisperVadContext` / `WhisperVadParams` available
- **No streaming API**: Must implement windowing yourself
- **Thread safety**: `WhisperContext` is Send but not Sync

---

## 2. The VAD Discovery — This is Critical

### Why VAD Matters
Without VAD, our system will:
- Transcribe silence (wasting CPU)
- Get hallucinations from whisper.cpp (it generates phantom text when fed silence/noise)
- Process windows during pauses in conversation unnecessarily

### Silero VAD Specs
- **Model size**: 1.8MB
- **Speed**: ~1ms for 30ms chunks (essentially free)
- **Accuracy**: Superior to webrtc-vad
- **Integration**: whisper-rs now has `WhisperVadContext`

### Impact on Our Architecture
With VAD, the flow becomes:
1. PCM tailer reads new bytes
2. **VAD checks if speech is present** (1ms per 30ms chunk)
3. Only feed windows containing speech to whisper.cpp
4. Skip silent windows entirely

**Result: 4x speed improvement + no hallucinations** (per WhisperX + Silero-VAD benchmarks)

### Recommendation
**Add VAD to our spec.** whisper-rs has built-in VAD support now. This is not optional for production quality.

---

## 3. Alternative Architectures Considered

### Option A: Whisper-VOSK Hybrid (from Medium)
- **VOSK** provides instant partial results (truly streaming, ~50ms latency)
- **Whisper** runs in background for high-accuracy corrections
- User sees fast VOSK text immediately, then it gets silently corrected by Whisper
- **Verdict**: Interesting but too complex for our timeline. Requires managing two engines.

### Option B: sherpa-onnx (Next-Gen Kaldi)
- **Truly streaming** via transducer/CTC models (zipformer)
- **No Rust bindings** — would need C FFI wrapper
- Built-in VAD, speaker diarization, multiple languages
- **Verdict**: Best streaming quality, but integration effort too high for competition deadline.

### Option C: In-Process Audio Capture (cpal crate)
Replace JarvisListen sidecar with `cpal` crate directly in Rust:
```
cpal audio capture → mpsc::channel → transcription thread → Tauri events
```
- Eliminates disk I/O and file coordination
- Zero-copy data transfer
- **Verdict**: Would be ideal but requires major refactoring of the working Listen module. Not worth it for competition.

### Option D: Cloud — AWS Transcribe Streaming
- `aws-sdk-transcribestreaming` Rust crate exists
- 300-500ms latency for interim results
- ~$0.024/minute
- **Verdict**: Perfect for the "cloud provider" behind our `TranscriptionProvider` trait. NOT Bedrock — AWS Transcribe is the right service.

### Option E: Deepgram Nova-3
- <300ms latency, cheapest ($4.30/1000 min)
- WebSocket streaming
- **Verdict**: Best cloud option if we ever need cloud transcription.

---

## 4. AWS Bedrock — Important Correction

### Bedrock Does NOT Do Real-Time Transcription
AWS Bedrock Data Automation can process audio files for transcription, but it is **batch-oriented**. For real-time streaming, you need **Amazon Transcribe Streaming**.

### The Right AWS Service
- **Amazon Transcribe Streaming**: HTTP/2 or WebSocket, 300-500ms interim results
- **Rust SDK**: `aws-sdk-transcribestreaming` crate on docs.rs
- **Pricing**: ~$0.024/minute

### Impact on Our Provider Trait
Our `TranscriptionProvider` trait should be designed with **AWS Transcribe** (not Bedrock) in mind as the future cloud provider. The trait already works for this since `transcribe()` takes audio chunks and returns segments.

---

## 5. Production Architecture Patterns (from Medium)

### Pattern 1: VAD-Gated Windowing (Recommended)
```
Audio → VAD → [only speech chunks] → Buffer → Whisper → Events
```
- Silero VAD filters out silence
- Only speech-containing audio reaches whisper
- Prevents hallucinations, saves CPU

### Pattern 2: Overlapping Windows with Context
- Keep 200-500ms overlap between windows
- Pass previous segment tokens as context prompt to next inference
- Reduces word-boundary errors

### Pattern 3: Pre-Roll Buffer
- Keep 100-200ms of audio before VAD triggers
- Catches word onsets that VAD might miss
- Important for natural-sounding transcription starts

### Pattern 4: Confidence-Based Display
- Show high-confidence results immediately as "final"
- Show low-confidence results in lighter text as "partial"
- Replace partials with finals as more context arrives

---

## 6. Model Selection Recommendation

### For Competition (Development)
**`ggml-base.en.bin`** (~142MB)
- Fastest inference (~0.5-1s per 5s window on Apple Silicon)
- English-only (fine for competition)
- Acceptable accuracy for demo purposes

### For Production
**`ggml-small.en.bin`** (~466MB) or **`ggml-large-v3-turbo`** (quantized)
- Better accuracy
- Still real-time capable with Metal acceleration
- large-v3-turbo: "6x faster than Large V2, same accuracy"

### Quantization
- Use quantized models (q5_0 or q8_0) for faster inference
- ~19% speed improvement with minimal accuracy loss

---

## 7. Validation of Our Current Architecture

### What's Good
1. **Provider trait pattern** — correct, enables swapping whisper for AWS Transcribe
2. **File-based PCM tailing** — pragmatic given existing sidecar architecture
3. **Sliding window approach** — standard for whisper.cpp "streaming"
4. **tokio background task** — correct for non-blocking transcription
5. **Error isolation** — transcription failures don't affect recording

### What's Missing or Wrong
1. **No VAD** — MUST add Silero VAD to skip silence and prevent hallucinations
2. **AWS "Bedrock" in intro** — should say "AWS Transcribe Streaming" as the future cloud provider
3. **Default thread count** — spec says "auto-detect" in R9.5 but KIRO_INSTRUCTIONS says "default: 1". Should be 1 for minimal CPU impact, let user override.
4. **No context carryover** — should pass previous segment tokens as prompt to next inference for continuity
5. **5s window may be too large** — Medium articles suggest 2-3s windows for better responsiveness. 5s with 1s overlap means ~4s between updates minimum.
6. **No model download guidance** — should include a command or script to download the model

### Specific Gaps in requirements.md

| Gap | Severity | Recommendation |
|-----|----------|----------------|
| No VAD requirement | HIGH | Add Requirement 12: Voice Activity Detection |
| No context carryover | MEDIUM | Add to R2 acceptance criteria |
| "Bedrock" mentioned | LOW | Change to "AWS Transcribe Streaming" |
| Window size default | MEDIUM | Consider 3s instead of 5s for lower latency |
| No model download | LOW | Add acceptance criteria to R8 |
| Metal GPU acceleration | MEDIUM | Add to R9: MUST enable Metal on macOS |

---

## 8. Recommended Changes to Our Spec

### Add: VAD Requirement (HIGH PRIORITY)
```
### Requirement 12: Voice Activity Detection

1. THE System SHALL use Silero VAD to detect speech segments before transcription
2. WHEN a PCM window contains no detected speech, THE System SHALL skip transcription for that window
3. WHEN VAD detects speech onset, THE System SHALL include a 200ms pre-roll buffer
4. WHEN VAD detects speech end, THE System SHALL include a 300ms post-roll buffer
5. THE VAD SHALL process audio in 30ms chunks with < 2ms latency per chunk
```

### Modify: Window Size
Change default from 5 seconds to 3 seconds for lower latency. This gives ~2s between transcription updates with a 0.5s overlap (vs ~4s with current 5s/1s config).

### Add: Context Carryover
In R4 (WhisperProvider), add:
```
9. WHEN transcribing sequential windows, THE Whisper_Provider SHALL pass the last
   segment's tokens as a prompt to the next inference for continuity
```

### Add: Metal Acceleration
In R9 (Performance), add:
```
6. THE System SHALL enable Metal GPU acceleration on macOS via the whisper-rs "metal" feature flag
```

### Modify: Cloud Provider Reference
Change all "AWS Bedrock" references to "AWS Transcribe Streaming" — Bedrock doesn't support real-time STT.

---

## 9. Existing Tauri Transcription Apps (Reference)

### Vibe
- Tauri app for multilingual audio transcription
- Uses whisper bindings directly
- Cross-platform (macOS, Linux, Windows)
- Source: [thewh1teagle/vibe](https://github.com/thewh1teagle/vibe)

### Handy
- Fully local STT app built with Tauri (Rust + React/TS)
- Global hotkey triggering, on-device processing
- Silence filtering (VAD-like)
- Source: Referenced in Medium articles

### Taurscribe
- Tauri-based transcription app
- Direct whisper.cpp integration
- Source: [machowdh/taurscribe](https://github.com/machowdh/taurscribe)

These are good reference implementations to study for patterns.

---

## 10. Final Recommendation

### For the Competition (Ship Fast)
1. Use **whisper-rs with Metal** (`features = ["metal"]`)
2. Add **Silero VAD** via whisper-rs `WhisperVadContext`
3. Use **3-second windows** with 0.5s overlap
4. Use **ggml-base.en** model (fast, English-only)
5. Keep **file-based PCM tailing** (pragmatic, works with existing sidecar)
6. Implement **context carryover** (pass previous tokens as prompt)
7. Design provider trait for **AWS Transcribe Streaming** (not Bedrock) as cloud swap

### Post-Competition (Production)
1. Replace file-based with in-process `cpal` audio capture
2. Add AWS Transcribe Streaming as cloud provider
3. Use larger model (small.en or large-v3-turbo quantized)
4. Consider sherpa-onnx for truly streaming results
5. Add speaker diarization

---

## Sources

### Technical Documentation
- [whisper.cpp stream example](https://github.com/ggml-org/whisper.cpp/blob/master/examples/stream/README.md)
- [whisper-rs docs.rs](https://docs.rs/whisper-rs/latest/whisper_rs/)
- [whisper-rs Codeberg](https://codeberg.org/tazz4843/whisper-rs)
- [aws-sdk-transcribestreaming](https://docs.rs/aws-sdk-transcribestreaming)
- [Silero VAD](https://github.com/snakers4/silero-vad)
- [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx)

### Medium Articles (Key Insights)
- [WhisperX + Silero-VAD: 4x speed, sub-500ms latency](https://medium.com/@aidenkoh/how-to-implement-high-speed-voice-recognition-in-chatbot-systems-with-whisperx-silero-vad-cdd45ea30904)
- [Whisper-VOSK Hybrid: self-correcting real-time model](https://medium.com/@aarathi.ajith01/the-whisper-vosk-loop-a-hybrid-transcription-model-that-self-corrects-in-real-time-bd25cbb8dac7)
- [How to Make Whisper STT Real-Time (3-part series)](https://medium.com/@pcb.it18/how-to-make-whisper-stt-real-time-transcription-part-3-10395d124c73)
- [Vibe: Tauri multilingual transcription](https://medium.com/@thewh1teagle/creating-vibe-multilingual-audio-transcription-872ab6d9dbb0)
- [Handy: Fully local STT with Tauri](https://agentnativedev.medium.com/fastest-devs-talk-to-llms-meet-fully-local-stt-app-handy-d14f9403b948)
- [Local Go STT with Silero VAD + whisper.cpp](https://medium.com/@etolkachev93/local-all-in-one-go-speech-to-text-solution-with-silero-vad-and-whisper-cpp-server-94a69fa51b04)
- [Whisper Large V3 Turbo: 6x faster, same accuracy](https://medium.com/@bnjmn_marie/whisper-large-v3-turbo-as-good-as-large-v2-but-6x-faster-97f0803fa933)
- [Quantizing Whisper: 30% faster, 64% less memory](https://medium.com/@daniel-klitzke/quantizing-openais-whisper-with-the-huggingface-optimum-library-30-faster-inference-64-36d9815190e0)
- [Building a Streaming Whisper WebSocket Service](https://medium.com/@david.richards.tech/how-to-build-a-streaming-whisper-websocket-service-1528b96b1235)
- [Deepgram vs Whisper vs ElevenLabs comparison](https://girishkurup21.medium.com/heres-a-detailed-comparison-of-deepgram-whisper-and-elevenlabs-across-various-aspects-c200512d8b23)

### Benchmarks
- [Apple Silicon Whisper Performance](https://www.voicci.com/blog/apple-silicon-whisper-performance.html)
- [mac-whisper-speedtest](https://github.com/anvanvan/mac-whisper-speedtest)
- [AssemblyAI: Top APIs for real-time STT 2026](https://www.assemblyai.com/blog/best-api-models-for-real-time-speech-recognition-and-transcription)
