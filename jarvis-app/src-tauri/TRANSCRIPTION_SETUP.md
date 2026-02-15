# Transcription Module Setup

## Prerequisites

The transcription module requires the following tools to be installed:

### 1. CMake (Required for whisper-rs)

whisper-rs builds whisper.cpp from source, which requires CMake.

**Install via Homebrew:**
```bash
brew install cmake
```

**Verify installation:**
```bash
cmake --version
```

### 2. Python and uv (Required for silero-vad-rs)

silero-vad-rs uses ONNX Runtime which requires Python build tools.

**Install uv (Python package manager):**
```bash
brew install uv
```

Or follow: https://docs.astral.sh/uv/getting-started/installation/

### 3. Model Files

The transcription system requires three model files:

**Whisper Model (Required - 142MB):**
```bash
mkdir -p ~/.jarvis/models
cd ~/.jarvis/models
# Download ggml-base.en.bin from https://huggingface.co/ggerganov/whisper.cpp
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
```

**Vosk Model (Optional - 40MB):**
```bash
cd ~/.jarvis/models
wget https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip
unzip vosk-model-small-en-us-0.15.zip
```

**Silero VAD Model (Optional - 1.8MB):**
```bash
cd ~/.jarvis/models
wget https://github.com/snakers4/silero-vad/raw/master/files/silero_vad.onnx
```

## Build Instructions

After installing prerequisites:

```bash
cd jarvis-app/src-tauri
cargo build
```

## Graceful Degradation

- **Whisper missing**: Transcription disabled, recording still works
- **Vosk missing**: No instant partials, only Whisper finals
- **VAD missing**: All audio processed (no silence skipping)

## Environment Variables

Override default model paths:

```bash
export JARVIS_WHISPER_MODEL=~/custom/path/to/whisper.bin
export JARVIS_VOSK_MODEL=~/custom/path/to/vosk-model
export JARVIS_VAD_MODEL=~/custom/path/to/silero_vad.onnx
export JARVIS_WHISPER_THREADS=4  # Optional: set thread count
```
