# MLX Multimodal Omni-Model Experiments

**Date**: 2026-02-26
**Goal**: Find a single multimodal LLM that can handle audio, images, and video — all via MLX on Apple Silicon

---

## The Idea

Instead of separate tools for each modality (Whisper for audio, VLM for images, etc.), use **one omni-model** that natively understands all input types:

> "Give it audio → it transcribes. Give it an image → it describes. Give it video → it summarizes."

---

## Candidate Models

Three families of omni-models work on MLX today:

### 1. Qwen2.5-Omni (Recommended Starting Point)

**What**: End-to-end multimodal model. Text + Audio + Image + Video input → Text + Speech output.
- [GitHub](https://github.com/QwenLM/Qwen2.5-Omni) | [HuggingFace](https://huggingface.co/Qwen/Qwen2.5-Omni-7B)
- Uses "Thinker-Talker" architecture: Thinker processes all modalities, Talker generates speech
- SOTA on OmniBench (multimodal benchmark)

| Model | Quant | Size | RAM | Library |
|-------|-------|------|-----|---------|
| [giangndm/qwen2.5-omni-3b-mlx-8bit](https://huggingface.co/giangndm/qwen2.5-omni-3b-mlx-8bit) | 8-bit | ~6 GB | ~8 GB | `mlx-lm-omni` |
| [giangndm/qwen2.5-omni-7b-mlx-4bit](https://huggingface.co/giangndm/qwen2.5-omni-7b-mlx-4bit) | 4-bit | ~4 GB | ~8 GB | `mlx-lm-omni` |

**Pros**: True omni — single model handles ALL modalities. Audio is a first-class input (not just vision).
**Cons**: Uses a community library (`mlx-lm-omni`), not official `mlx-lm`. Newer, less battle-tested.

### 2. Gemma-3n (Google)

**What**: Natively multimodal — text, image, audio, video inputs → text output.
- [HuggingFace Blog](https://huggingface.co/blog/gemma3n) | Day-0 MLX support via `mlx-vlm`
- Designed for on-device inference (efficient)

| Model | Quant | Size | RAM | Library |
|-------|-------|------|-----|---------|
| `mlx-community/gemma-3n-E2B-it-4bit` | 4-bit | ~1.5 GB | ~4 GB | `mlx-vlm` |
| `google/gemma-3n-E4B-it` | fp16 | ~8 GB | ~12 GB | `mlx-vlm` |

**Pros**: Official `mlx-vlm` support (stable library). Lightweight E2B variant is tiny. Audio+Image+Video all supported.
**Cons**: Smaller model may have lower quality than Qwen 7B. Newer model family.

### 3. Qwen3-Omni (Latest, Largest)

**What**: Next-gen omni model. Text + Audio + Image + Video → Text + Speech. MoE architecture.
- [GitHub](https://github.com/QwenLM/Qwen3-Omni) | SOTA on 32/36 audio+video benchmarks
- 119 text languages, 19 speech input languages

| Model | Quant | Size | RAM | Library |
|-------|-------|------|-----|---------|
| [pherber3/Qwen3-Omni-30B-A3B-Instruct-4bit-mlx](https://huggingface.co/pherber3/Qwen3-Omni-30B-A3B-Instruct-4bit-mlx) | 4-bit | ~17 GB | ~32 GB | `mlx-lm-omni` |

**Pros**: Best quality. MoE = only 3B active params despite 30B total (fast).
**Cons**: Needs 32GB+ RAM. Large download.

### Comparison Matrix

| Feature | Qwen2.5-Omni-3B | Qwen2.5-Omni-7B | Gemma-3n-E2B | Qwen3-Omni-30B-A3B |
|---------|-----------------|-----------------|--------------|---------------------|
| Audio → Text | Yes | Yes | Yes | Yes |
| Image → Text | Yes | Yes | Yes | Yes |
| Video → Text | Yes | Yes | Yes | Yes |
| Text → Speech | Yes | Yes | No | Yes |
| Min RAM | ~8 GB | ~8 GB | ~4 GB | ~32 GB |
| MLX Library | mlx-lm-omni | mlx-lm-omni | mlx-vlm | mlx-lm-omni |
| Maturity | Community port | Community port | Official support | Community port |

---

## Experiment Plan (Jupyter Notebook)

### Setup

```python
# Cell 1: Install dependencies
!pip install mlx mlx-vlm librosa soundfile

# For Qwen2.5-Omni specifically
!pip install mlx-lm-omni
# OR install from git for latest:
# !pip install git+https://github.com/giangndm/mlx-lm-omni.git
```

### Experiment 1: Audio Transcription (Qwen2.5-Omni)

```python
# Cell 2: Load Qwen2.5-Omni
from mlx_lm_omni import load, generate
import librosa
import time

model, tokenizer = load("giangndm/qwen2.5-omni-3b-mlx-8bit")

# Cell 3: Transcribe an audio file
audio, sr = librosa.load("test_audio.wav", sr=16000)

messages = [
    {"role": "system", "content": "You are a helpful assistant."},
    {
        "role": "user",
        "content": "Transcribe this audio accurately.",
        "audio": audio,
    },
]

prompt = tokenizer.apply_chat_template(messages, add_generation_prompt=True)

start = time.time()
text = generate(model, tokenizer, prompt=prompt, verbose=True)
elapsed = time.time() - start

print(f"\nTranscription: {text}")
print(f"Time: {elapsed:.2f}s")

# Cell 4: Try with a Jarvis recording
import glob
recordings = sorted(glob.glob("/Users/ankit/.jarvis/recordings/*.wav"))
if recordings:
    latest = recordings[-1]
    print(f"Using: {latest}")
    audio, sr = librosa.load(latest, sr=16000)
    messages = [
        {"role": "user", "content": "Transcribe this audio word for word.", "audio": audio},
    ]
    prompt = tokenizer.apply_chat_template(messages, add_generation_prompt=True)
    text = generate(model, tokenizer, prompt=prompt, verbose=True)
    print(f"\n{text}")
```

### Experiment 2: Image Understanding (Qwen2.5-Omni)

```python
# Cell 5: Image understanding with same model
# Note: Check if mlx-lm-omni supports image input, otherwise use mlx-vlm

# Option A: If mlx-lm-omni supports images (check docs)
messages = [
    {
        "role": "user",
        "content": "Describe this image in detail.",
        "image": "test_image.jpg",
    },
]
prompt = tokenizer.apply_chat_template(messages, add_generation_prompt=True)
text = generate(model, tokenizer, prompt=prompt, verbose=True)
print(text)

# Option B: Use mlx-vlm for image (Gemma-3n handles audio+image)
from mlx_vlm import load as vlm_load, generate as vlm_generate
from mlx_vlm.prompt_utils import apply_chat_template as vlm_chat_template
from mlx_vlm.utils import load_config

model_path = "mlx-community/gemma-3n-E2B-it-4bit"
vlm_model, vlm_processor = vlm_load(model_path)
vlm_config = vlm_model.config

image = ["test_image.jpg"]
prompt = "Describe this image in detail."
formatted_prompt = vlm_chat_template(vlm_processor, vlm_config, prompt, num_images=1)
output = vlm_generate(vlm_model, vlm_processor, formatted_prompt, image, verbose=False, max_tokens=500)
print(output)
```

### Experiment 3: Image Understanding (Gemma-3n via mlx-vlm)

```python
# Cell 6: Gemma-3n — audio understanding
!python -m mlx_vlm.generate \
    --model mlx-community/gemma-3n-E2B-it-4bit \
    --max-tokens 200 \
    --prompt "Transcribe the following speech segment in English:" \
    --audio test_audio.wav

# Cell 7: Gemma-3n — image understanding
!python -m mlx_vlm.generate \
    --model mlx-community/gemma-3n-E2B-it-4bit \
    --max-tokens 200 \
    --prompt "Describe what you see in this image." \
    --image test_image.jpg

# Cell 8: Gemma-3n — combined image + audio
!python -m mlx_vlm.generate \
    --model mlx-community/gemma-3n-E2B-it-4bit \
    --max-tokens 200 \
    --prompt "Describe what you see and hear." \
    --image test_image.jpg \
    --audio test_audio.wav
```

### Experiment 4: Video Understanding

```python
# Cell 9: Video with Qwen2.5-VL (proven video support)
!python -m mlx_vlm.video_generate \
    --model mlx-community/Qwen2.5-VL-7B-Instruct-4bit \
    --max-tokens 500 \
    --prompt "Describe what happens in this video step by step." \
    --video test_video.mp4 \
    --max-pixels 360 360 \
    --fps 1.0

# Cell 10: Video with Gemma-3n (if supported)
# Gemma-3n natively supports video — test if mlx-vlm exposes it
!python -m mlx_vlm.video_generate \
    --model mlx-community/gemma-3n-E2B-it-4bit \
    --max-tokens 500 \
    --prompt "Summarize this video." \
    --video test_video.mp4 \
    --fps 1.0
```

### Experiment 5: Head-to-Head Comparison

```python
# Cell 11: Compare transcription quality across models
import time

test_audio = "test_audio.wav"

results = {}

# --- Qwen2.5-Omni (via mlx-lm-omni) ---
from mlx_lm_omni import load as omni_load, generate as omni_generate
model_omni, tok_omni = omni_load("giangndm/qwen2.5-omni-3b-mlx-8bit")
audio, sr = librosa.load(test_audio, sr=16000)
msgs = [{"role": "user", "content": "Transcribe this audio.", "audio": audio}]
prompt = tok_omni.apply_chat_template(msgs, add_generation_prompt=True)

start = time.time()
results["qwen2.5-omni-3b"] = omni_generate(model_omni, tok_omni, prompt=prompt)
results["qwen2.5-omni-3b_time"] = time.time() - start

# Free memory
del model_omni, tok_omni
import gc; gc.collect()
import mlx.core as mx; mx.metal.clear_cache()

# --- Gemma-3n (via mlx-vlm) ---
# ... similar pattern ...

# Cell 12: Print comparison
for key, val in results.items():
    if not key.endswith("_time"):
        print(f"\n{'='*60}")
        print(f"Model: {key}")
        print(f"Time:  {results[key + '_time']:.2f}s")
        print(f"Output: {val}")
```

---

## Experiment Sequence

### Phase 1: Qwen2.5-Omni-3B (Quick Win)
1. `pip install mlx-lm-omni`
2. Load `giangndm/qwen2.5-omni-3b-mlx-8bit` (~6GB download)
3. Test audio transcription on a Jarvis recording
4. Test image description
5. **Evaluate**: Quality vs dedicated Whisper? Latency?

### Phase 2: Gemma-3n-E2B (Lightweight Alternative)
1. Already have `mlx-vlm` from Jarvis venv
2. Load `mlx-community/gemma-3n-E2B-it-4bit` (~1.5GB download)
3. Test audio, image, and combined audio+image
4. **Evaluate**: Is the tiny model good enough? Speed?

### Phase 3: Video (Both Models)
1. Test video understanding with Qwen2.5-VL-7B (proven)
2. Test video with Gemma-3n (if supported)
3. Try different FPS settings (0.5, 1.0, 2.0)
4. **Evaluate**: How long a video can each handle?

### Phase 4: Pick a Winner
Based on results, decide:
- **Best audio transcription**: Omni model vs dedicated Whisper
- **Best image understanding**: Which model, which size
- **Best video**: Which model handles it best
- **Best all-rounder**: If one model is "good enough" at everything

---

## What This Unlocks for Jarvis

If an omni model works well:

| Use Case | How |
|----------|-----|
| Transcribe recordings | Feed audio directly to omni model (replace/complement Whisper) |
| Screenshot → Gem | Paste screenshot, model extracts text + describes UI |
| Meeting video → Gem | Record screen, model summarizes what happened |
| Audio + Screen | Simultaneous: "What are they saying AND showing?" |
| Document analysis | Photo of whiteboard/document → structured notes |

The big win: **one model download, one venv, all modalities**. No separate Whisper, no separate VLM.

---

## Hardware Requirements

| Model | Min RAM | Download Size | Notes |
|-------|---------|---------------|-------|
| Qwen2.5-Omni-3B (8-bit) | 8 GB | ~6 GB | Good starting point |
| Qwen2.5-Omni-7B (4-bit) | 8 GB | ~4 GB | Better quality |
| Gemma-3n-E2B (4-bit) | 4 GB | ~1.5 GB | Smallest, fastest |
| Gemma-3n-E4B (fp16) | 12 GB | ~8 GB | Better quality |
| Qwen3-Omni-30B-A3B (4-bit) | 32 GB | ~17 GB | Best quality, needs beefy Mac |

---

## Dependencies

```bash
# Core
pip install mlx jupyter ipywidgets

# Qwen2.5-Omni support
pip install mlx-lm-omni    # or: pip install git+https://github.com/giangndm/mlx-lm-omni.git
pip install librosa soundfile

# Gemma-3n / Qwen2.5-VL / Video support
pip install mlx-vlm

# Video frame extraction (optional, for manual approach)
pip install opencv-python Pillow
```

---

## Sources
- [Qwen2.5-Omni (GitHub)](https://github.com/QwenLM/Qwen2.5-Omni)
- [Qwen2.5-Omni-7B (HuggingFace)](https://huggingface.co/Qwen/Qwen2.5-Omni-7B)
- [giangndm/qwen2.5-omni-3b-mlx-8bit](https://huggingface.co/giangndm/qwen2.5-omni-3b-mlx-8bit)
- [giangndm/qwen2.5-omni-7b-mlx-4bit](https://huggingface.co/giangndm/qwen2.5-omni-7b-mlx-4bit)
- [mlx-lm-omni (GitHub)](https://github.com/giangndm/mlx-lm-omni)
- [Qwen3-Omni (GitHub)](https://github.com/QwenLM/Qwen3-Omni)
- [Gemma-3n (HuggingFace Blog)](https://huggingface.co/blog/gemma3n)
- [mlx-vlm (GitHub)](https://github.com/Blaizzy/mlx-vlm)
- [mlx-vlm (PyPI)](https://pypi.org/project/mlx-vlm/)
- [Qwen2.5-Omni Apple Silicon Demo](https://huggingface.co/spaces/Jimmi42/Qwen2.5-Omni-Apple-silicon)
- [vllm-mlx (GitHub)](https://github.com/waybarrios/vllm-mlx)
- [MLX Framework](https://mlx-framework.org/)
