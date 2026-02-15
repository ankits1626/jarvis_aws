# VAD & Vosk Integration Fixes

How we got voice detection (VAD) and instant transcription (Vosk) working on Mac Apple Silicon.

---

## 1. Silero VAD: All Existing Libraries Were Broken

### The problem

VAD (Voice Activity Detection) listens to audio and answers one question: "Is someone speaking right now?" This lets us skip silent audio and only send speech to the expensive Whisper engine.

We needed to use the **Silero VAD** AI model (a tiny 2.2MB file). There were three existing Rust libraries that claimed to wrap this model. All three were broken:

| Library | What went wrong |
|---------|----------------|
| `silero-vad-rust` | Internal dependency conflict — two of its own sub-libraries needed different, incompatible versions of the same thing. Would not compile. |
| `silero-vad-rs` | **Downloads the wrong AI model.** Instead of the tiny 2.2MB voice detector, it downloads a massive 528MB speech-to-text model. This wrong model thinks everything is speech — even silence. |
| `silero-vad-rs` + correct model manually | The library was built for a **different version** of the model. Like trying to play a Blu-ray in a DVD player — the formats don't match. Crashes with "parsing failed". |

### How we fixed it

Since no library worked, we **bypassed them all** and talked to the AI model directly.

Think of it like this: instead of using a broken TV remote (the libraries), we opened the TV's control panel and wired the buttons ourselves.

**Step 1:** Downloaded the correct Silero VAD model (2.2MB) from the official source.

**Step 2:** Used Python to "open the box" and see what the model expects:
- **Input:** 512 audio samples (~32 milliseconds of sound) + memory of what it heard before
- **Output:** A number from 0.0 (silence) to 1.0 (speech)

**Step 3:** Wrote our own Rust code that feeds audio directly to the model using ONNX Runtime (a universal AI model runner), reads back the speech probability, and remembers state between chunks.

**Result:** VAD correctly identifies speech vs silence, loads instantly.

### What changed

- Rewrote `vad.rs` from scratch — talks to the AI model directly instead of through a broken library
- Replaced three broken library dependencies with one working one (`ort` — the ONNX Runtime)

---

## 2. Vosk: Missing Mac ARM64 Library

### The problem

Vosk gives us **instant text previews** while someone is speaking (before Whisper's more accurate but slower result arrives). Think of it as the "gray preview text" you see while typing in a search bar.

The Vosk Rust library is just a thin wrapper — it needs a separate native system library (`libvosk`) to do the actual work. Problem: **Vosk never published a prebuilt version of this library for Mac Apple Silicon (ARM64)**. The only option was to build it from source, which requires installing Kaldi (a massive speech toolkit) and takes 30-60 minutes.

The library file wasn't corrupt or broken — **it simply didn't exist** for our platform.

### How we fixed it

**Key discovery:** The Python version of Vosk (`pip install vosk`) works perfectly on Mac ARM64. When you install it, it bundles the native library inside the Python package. We verified:
- It's a universal binary (works on both Intel and Apple Silicon Macs)
- It's 12.5MB and only depends on standard macOS system libraries
- It exports all 35 functions that the Rust wrapper needs

So we simply **borrowed the library file from the Python package**:

1. **Copied** `libvosk.dyld` from Python's package folder to `~/.jarvis/lib/libvosk.dylib`
2. **Fixed the file's internal name** so macOS knows where to find it at runtime (using `install_name_tool`)
3. **Told the Rust build system** where to find the library (updated `build.rs`)
4. **Replaced the stub** VoskProvider with real code that loads the model and produces instant transcription previews

**Result:** Vosk loads successfully, produces instant text partials in under 100ms.

### What changed

- `vosk_provider.rs` — Real implementation replacing the "always unavailable" stub
- `build.rs` — Tells Rust where to find the Vosk library
- `Cargo.toml` — Enabled the `vosk` dependency
- `~/.jarvis/lib/libvosk.dylib` — The borrowed library file

---

## Summary

| Engine | Problem | Solution |
|--------|---------|----------|
| **Silero VAD** (voice detector) | All 3 existing Rust libraries were broken in different ways | Bypassed them all, talked to the AI model directly using ONNX Runtime |
| **Vosk** (instant previews) | No prebuilt library exists for Mac Apple Silicon | Borrowed the library from the Python version of Vosk, which bundles it |

### Key takeaways

- **VAD:** Sometimes wrapper libraries are more trouble than they're worth. Going direct can be simpler and more reliable.
- **Vosk:** When a native library isn't available for your platform, check if another language's package (Python, Node, etc.) bundles it. Python packages frequently ship native binaries.

---

## Setup for new machines

```bash
# 1. Silero VAD model (2.2MB voice detector)
mkdir -p ~/.jarvis/models
curl -L https://github.com/snakers4/silero-vad/raw/master/src/silero_vad/data/silero_vad.onnx \
  -o ~/.jarvis/models/silero_vad.onnx

# 2. Vosk native library (borrowed from Python package)
pip install vosk
mkdir -p ~/.jarvis/lib
cp $(python -c "import vosk; import os; print(os.path.join(os.path.dirname(vosk.__file__), 'libvosk.dyld'))") \
  ~/.jarvis/lib/libvosk.dylib
install_name_tool -id "@rpath/libvosk.dylib" ~/.jarvis/lib/libvosk.dylib

# 3. Vosk speech model (40MB, for instant previews)
cd ~/.jarvis/models
curl -LO https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip
unzip vosk-model-small-en-us-0.15.zip

# 4. Whisper model (148MB, for accurate final text)
curl -LO https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
```
