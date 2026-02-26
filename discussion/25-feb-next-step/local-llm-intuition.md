# Local LLM Intuition Guide

A ground-up explanation of how local LLMs work, what MLX and Ollama are, and which models to use.

---

## Part 1: What Is a "Model" Physically?

A model is a file full of numbers called **weights**. Nothing more.

A 7B model has 7 billion floating-point numbers arranged in matrices. These numbers were learned during training (which costs millions of dollars and thousands of GPUs). You don't train — you download the result and use it.

```
model file = giant matrix of numbers

1B model   ≈ 1,000,000,000 floats  ≈  2 GB on disk (FP16)
3B model   ≈ 3,000,000,000 floats  ≈  6 GB on disk (FP16)
7B model   ≈ 7,000,000,000 floats  ≈ 14 GB on disk (FP16)
13B model  ≈ 13,000,000,000 floats ≈ 26 GB on disk (FP16)
70B model  ≈ 70,000,000,000 floats ≈ 140 GB on disk (FP16)
```

FP16 means each number uses 2 bytes of storage. That's the baseline.

---

## Part 2: What Happens When You "Run" a Model

When you send a prompt to a local LLM, this is what physically happens:

### Step 1: Tokenization

Your text gets split into tokens (roughly word pieces):

```
Input:  "What is Rust?"
Tokens: ["What", " is", " Rust", "?"]
IDs:    [3923, 374, 56461, 30]
```

Each token maps to a number. The model's vocabulary is typically 32,000-128,000 tokens.

### Step 2: Forward Pass (Matrix Multiplication)

The token IDs get converted to vectors (embeddings), then passed through every layer of the model sequentially:

```
Token embeddings [4 tokens × 4096 dimensions]
    │
    ▼
┌──────────────────────────────────┐
│  Layer 1: Self-Attention         │  ← matrix multiply with layer 1 weights
│           Feed-Forward           │  ← matrix multiply with layer 1 weights
└──────────────┬───────────────────┘
               ▼
┌──────────────────────────────────┐
│  Layer 2: Self-Attention         │  ← matrix multiply with layer 2 weights
│           Feed-Forward           │  ← matrix multiply with layer 2 weights
└──────────────┬───────────────────┘
               ▼
              ...
               ▼
┌──────────────────────────────────┐
│  Layer 32: Self-Attention        │  ← matrix multiply with layer 32 weights
│            Feed-Forward          │  ← matrix multiply with layer 32 weights
└──────────────┬───────────────────┘
               ▼
Output: probability distribution over all 32,000 tokens
        → pick the highest probability → "Rust"
```

A 7B model with 32 layers means every token passes through 32 matrix multiplications. Each multiplication touches millions of weights.

### Step 3: Autoregressive Generation (One Token at a Time)

LLMs generate **one token at a time**. To generate a 100-token response, the model runs the forward pass 100 times:

```
Pass 1: "What is Rust?"           → "Rust"
Pass 2: "What is Rust? Rust"      → " is"
Pass 3: "What is Rust? Rust is"   → " a"
Pass 4: "What is Rust? Rust is a" → " systems"
...100 passes total...
```

This is why generation feels slow — each token requires reading ALL model weights from memory.

### Step 4: KV Cache (Speed Optimization)

Without optimization, each new token would reprocess the entire sequence from scratch. The **KV cache** stores intermediate results from previous tokens so only the new token needs full computation:

```
Without KV cache:
  Token 50: process all 50 tokens through all layers (slow)
  Token 51: process all 51 tokens through all layers (slow)

With KV cache:
  Token 50: process only token 50, reuse cached results for 1-49 (fast)
  Token 51: process only token 51, reuse cached results for 1-50 (fast)
```

The KV cache lives in RAM and grows with the conversation length. This is why long conversations use more memory.

---

## Part 3: Why Model Size = RAM Needed

Every token generation requires reading ALL weights from memory. The model can't page weights in and out — it needs random access to every layer.

```
To generate 1 token with a 7B FP16 model:
  → Read 14 GB of weights from RAM
  → Perform matrix multiplication
  → Output 1 token
  → Repeat

To generate 100 tokens:
  → Read 14 GB × 100 = 1.4 TB of data from RAM
  (not 1.4 TB of storage — the same 14 GB read 100 times)
```

If the model doesn't fit in RAM, the OS pages to disk (SSD), and inference becomes 100x slower.

---

## Part 4: Quantization — Making Models Fit

Quantization compresses each weight from high precision to lower precision:

```
FP32 (full float):   4 bytes per weight  → highest quality, huge file
FP16 (half float):   2 bytes per weight  → negligible quality loss, standard
INT8 (8-bit):        1 byte per weight   → ~99% quality, half the FP16 size
Q4_K_M (4-bit):      0.5 bytes per weight → ~95% quality, quarter the FP16 size
Q2 (2-bit):          0.25 bytes per weight → noticeable quality loss
```

### How 4-bit quantization works (simplified)

Instead of storing each weight as a precise float:

```
FP16:  [0.0234, -0.1567, 0.0891, 0.2345, -0.0123, ...]
       Each number can be any value. 2 bytes each.

4-bit: Group 32 weights together.
       Store: min=-0.16, max=0.23, and 32 indices (0-15) into 16 levels.
       [3, 0, 7, 14, 4, ...]
       Each index is 4 bits (half a byte).
       The actual value is reconstructed: value = min + index × (max-min)/15
```

The reconstruction is approximate, hence the small quality loss. But 4x less RAM is a huge win:

```
7B model:
  FP16:    14 GB
  4-bit:   ~4 GB ← fits on an 8 GB Mac

13B model:
  FP16:    26 GB
  4-bit:   ~7 GB ← fits on an 8 GB Mac

70B model:
  FP16:    140 GB
  4-bit:   ~35 GB ← fits on a 64 GB Mac
```

### Q4_K_M explained

You'll see quantization names like Q4_K_M, Q5_K_S, Q8_0. Here's the naming:

```
Q4  = 4 bits per weight
K   = k-quant method (smarter grouping, better quality than naive)
M   = medium (S = small/faster, M = medium/balanced, L = large/higher quality)

Q4_K_M = 4-bit, k-quant, medium quality → the most popular default
Q5_K_M = 5-bit, k-quant, medium → slightly better quality, slightly more RAM
Q8_0   = 8-bit, basic method → near-lossless, double the size of Q4
```

---

## Part 5: The Bottleneck — Memory Bandwidth

Token generation is **memory-bandwidth bound**, NOT compute bound.

This is the most counterintuitive fact about LLM inference. The GPU spends most of its time *waiting for data to arrive from memory*, not doing math.

```
To generate 1 token with a 7B Q4 model:
  Read ~4 GB of weights from memory
  Perform ~7 billion multiply-add operations

Apple M4 Max:
  Memory bandwidth: 546 GB/s → can read 4 GB in 7.3 ms
  Compute: 38 TFLOPS → can do 7B operations in 0.2 ms

  Time to read weights: 7.3 ms  ← BOTTLENECK
  Time to compute:      0.2 ms

  Theoretical max: ~136 tokens/sec
  Real world: ~50 tok/s (KV cache reads, overhead)
```

This is why:
- **More GPU cores barely help** — you're memory-bound
- **Apple Silicon competes with NVIDIA** — Apple has high bandwidth unified memory
- **Quantization helps speed too** — 4-bit = 4x less data to read = 4x faster

### Apple Silicon vs NVIDIA for local inference

```
NVIDIA RTX 4090:
  VRAM: 24 GB (model must fit here)
  VRAM bandwidth: 1,008 GB/s
  → Fast for models that fit in 24 GB
  → Cannot run 70B Q4 (needs 35 GB) without CPU offloading

Apple M4 Max (128 GB):
  Unified memory: 128 GB (model uses this directly)
  Memory bandwidth: 546 GB/s
  → Slower per-token than 4090
  → But can run 70B Q4 entirely in memory (no offloading)
  → No CPU↔GPU copy overhead (unified memory)

Apple M3 Ultra (192 GB):
  Unified memory: 192 GB
  Memory bandwidth: 800 GB/s
  → Can run 70B at full precision (FP16)
```

The Apple advantage: **unified memory means no VRAM limit**. The entire system RAM is your "VRAM."

---

## Part 6: Context Window

The context window is how much text the model can "see" at once — both your input and its output combined.

```
Context window = input tokens + output tokens

If context = 4096 tokens ≈ ~3,000 words total
If context = 32768 tokens ≈ ~25,000 words total
If context = 128k tokens ≈ ~100,000 words total
```

### Why context uses extra RAM (KV Cache)

For each token in the context, the model stores intermediate values (keys and values from the attention mechanism):

```
KV cache size per token ≈ 2 × num_layers × hidden_size × 2 bytes (FP16)

For a 7B model (32 layers, 4096 hidden):
  Per token: 2 × 32 × 4096 × 2 = 524 KB
  4096 context: 524 KB × 4096 = ~2 GB
  32768 context: 524 KB × 32768 = ~16 GB
  128k context: 524 KB × 131072 = ~65 GB

Total RAM = model weights + KV cache
  7B Q4 + 4k context  = 4 GB + 2 GB  = ~6 GB
  7B Q4 + 32k context = 4 GB + 16 GB = ~20 GB
  7B Q4 + 128k context = 4 GB + 65 GB = ~69 GB ← context costs more than the model!
```

This is why "128k context" models aren't free — the memory cost is enormous.

### Prompt processing vs token generation

Two distinct phases with different performance characteristics:

```
Phase 1: Prompt Processing (Prefill)
  - Process all input tokens at once in parallel
  - GPU-compute bound (lots of parallel matrix multiply)
  - Very fast: thousands of tokens/sec
  - Happens once at the start

Phase 2: Token Generation (Decode)
  - Generate one token at a time, sequentially
  - Memory-bandwidth bound (read all weights per token)
  - Slower: 20-50 tokens/sec
  - This is the speed you "feel" as the response streams in
```

When people say "50 tok/s" they mean decode speed. Prefill is 10-50x faster.

---

## Part 7: Model Architecture Families

Not all models are the same architecture. Here's what matters:

### Dense Models (Standard Transformers)

Every token uses ALL parameters. A 7B dense model does 7B operations per token.

```
Examples: Llama 3, Mistral 7B, Phi-3, Gemma, Qwen
```

### Mixture of Experts (MoE)

Only a subset of parameters are active per token. A 47B MoE model might only activate 12B parameters per token.

```
Mixtral 8x7B:
  Total parameters: ~47B
  Active per token: ~12B (2 of 8 experts activated)
  Speed: similar to a 12B dense model
  Quality: closer to a 47B dense model
  RAM: needs all 47B in memory (even though only 12B active per token)
```

MoE gives you better quality-per-compute but still needs full model in RAM.

### Reasoning Models

Models trained specifically for chain-of-thought reasoning. They "think out loud" before answering.

```
DeepSeek R1:
  Generates internal reasoning steps before the final answer
  Better at math, logic, planning
  Slower: generates more tokens (thinking + answer)
  Higher quality for complex tasks
```

---

## Part 8: MLX Deep Dive

### What MLX actually is

MLX is Apple's machine learning framework — think "PyTorch but built for Apple Silicon."

```
┌─────────────────────────────────────┐
│           Your Python Code          │
│   from mlx_lm import load, generate│
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│          MLX-LM Library             │
│   Model loading, tokenization,     │
│   generation loop, KV cache mgmt   │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│          MLX Core Framework         │
│   Array ops, lazy evaluation,      │
│   automatic differentiation,       │
│   Metal shader compilation         │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│     Apple Metal GPU (Hardware)      │
│   Unified memory, GPU compute      │
└─────────────────────────────────────┘
```

### The unified memory advantage

On traditional setups (PyTorch + NVIDIA), data must be copied between CPU RAM and GPU VRAM:

```
Traditional (PyTorch + CUDA):
┌──────────┐    PCIe bus    ┌──────────┐
│ CPU RAM  │ ──── copy ───→ │ GPU VRAM │
│  32 GB   │                │  24 GB   │
└──────────┘                └──────────┘
  Model loaded here first    Must fit here to use GPU
  Then copied to GPU         24 GB = hard limit

MLX on Apple Silicon:
┌──────────────────────────────────────┐
│          Unified Memory (128 GB)     │
│                                      │
│   [model weights]                    │
│      ↑               ↑              │
│      │               │              │
│   CPU reads      GPU reads          │
│   directly       directly           │
│                                      │
│   No copy. No VRAM limit.           │
│   128 GB = your VRAM.               │
└──────────────────────────────────────┘
```

### Lazy evaluation

MLX doesn't compute immediately. It builds a graph and runs it when you ask:

```python
import mlx.core as mx

a = mx.ones((1000, 1000))   # No computation yet
b = mx.ones((1000, 1000))   # No computation yet
c = a + b                    # No computation yet — just records "add a and b"
d = c * 2                    # No computation yet — records "multiply c by 2"

mx.eval(d)                   # NOW: runs a+b, then ×2, on GPU in one fused operation
```

Why this matters: the framework can optimize the entire computation graph before running it. Fuse operations, eliminate redundant work, minimize memory allocation.

### Model format: safetensors

MLX uses Hugging Face's safetensors format:

```
model-directory/
├── config.json            ← architecture (how many layers, hidden size, etc.)
├── tokenizer.json         ← how to split text into tokens
├── tokenizer_config.json  ← tokenizer settings
├── model.safetensors      ← the actual weights (or split into shards)
└── special_tokens_map.json
```

Models come from the `mlx-community` organization on Hugging Face — community-maintained conversions of popular models, pre-quantized and ready to use.

### Running models with MLX-LM

```python
from mlx_lm import load, generate, stream_generate

# Load a 4-bit quantized model (~2 GB download, ~2 GB RAM)
model, tokenizer = load("mlx-community/Llama-3.2-3B-Instruct-4bit")

# Simple generation
response = generate(model, tokenizer, prompt="What is Rust?", max_tokens=200)

# Streaming (token by token)
for token in stream_generate(model, tokenizer, prompt="What is Rust?", max_tokens=200):
    print(token, end="", flush=True)

# Chat format
messages = [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "What is Rust?"}
]
prompt = tokenizer.apply_chat_template(messages, tokenize=False, add_generation_prompt=True)
response = generate(model, tokenizer, prompt=prompt, max_tokens=300)
```

### MLX-LM server mode (OpenAI-compatible)

```bash
# Start server
mlx_lm.server --model mlx-community/Llama-3.2-3B-Instruct-4bit --port 8080

# Call it like OpenAI
curl http://localhost:8080/v1/chat/completions -d '{
  "model": "default",
  "messages": [{"role": "user", "content": "Hello"}],
  "max_tokens": 100
}'
```

This is how you'd integrate with Tauri/Rust — HTTP calls to a Python subprocess.

### Converting any model to MLX format

```bash
# Download from HuggingFace, convert, and quantize to 4-bit
mlx_lm.convert --hf-path meta-llama/Llama-3.2-3B --mlx-path ./my-model -q --q-bits 4
```

### MLX-compatible models

Any Hugging Face transformer model with a supported architecture. The `mlx-community` org has pre-converted versions:

| Model | MLX-Community Name | Size (4-bit) | Quality |
|---|---|---|---|
| Llama 3.2 1B | `mlx-community/Llama-3.2-1B-Instruct-4bit` | ~0.7 GB | Basic |
| Llama 3.2 3B | `mlx-community/Llama-3.2-3B-Instruct-4bit` | ~1.8 GB | Good |
| Llama 3.1 8B | `mlx-community/Llama-3.1-8B-Instruct-4bit` | ~4.5 GB | Very good |
| Mistral 7B | `mlx-community/Mistral-7B-Instruct-v0.3-4bit` | ~4 GB | Good |
| Phi-3 Mini | `mlx-community/Phi-3-mini-4k-instruct-4bit` | ~2 GB | Good for size |
| Phi-3 Medium | `mlx-community/Phi-3-medium-4k-instruct-4bit` | ~7 GB | Very good |
| Qwen 2.5 7B | `mlx-community/Qwen2.5-7B-Instruct-4bit` | ~4 GB | Very good |
| Qwen 2.5 14B | `mlx-community/Qwen2.5-14B-Instruct-4bit` | ~8 GB | Excellent |
| Gemma 2 9B | `mlx-community/gemma-2-9b-it-4bit` | ~5 GB | Very good |
| Gemma 2 27B | `mlx-community/gemma-2-27b-it-4bit` | ~15 GB | Excellent |
| DeepSeek R1 7B | `mlx-community/DeepSeek-R1-Distill-Qwen-7B-4bit` | ~4 GB | Good reasoning |
| Mixtral 8x7B | `mlx-community/Mixtral-8x7B-Instruct-v0.1-4bit` | ~24 GB | Excellent (MoE) |
| Llama 3.1 70B | `mlx-community/Llama-3.1-70B-Instruct-4bit` | ~35 GB | Best open model |
| CodeLlama 34B | `mlx-community/CodeLlama-34B-Instruct-4bit` | ~18 GB | Best for code |

If a model is on Hugging Face but NOT on mlx-community, you can convert it yourself with `mlx_lm.convert`.

### MLX pros and cons

**Pros:**
- Native Apple Silicon optimization — hand-tuned Metal compute shaders
- Unified memory = no VRAM limits, zero-copy
- Can fine-tune models locally (LoRA, QLoRA)
- Access to entire Hugging Face ecosystem (convert anything)
- OpenAI-compatible server for easy integration
- Active development by Apple's ML research team

**Cons:**
- Requires Python — cannot compile to standalone binary
- Must bundle or depend on Python runtime for distribution
- No native Rust bindings — must bridge via HTTP or subprocess
- Smaller community than Ollama
- macOS-only for GPU acceleration (Linux CUDA support is newer)

---

## Part 9: Ollama Deep Dive

### What Ollama actually is

Ollama is a Go binary that wraps **llama.cpp** (a C++ LLM inference engine) and adds model management + HTTP API.

```
┌─────────────────────────────────────┐
│           Your App (Tauri/Rust)     │
│   reqwest::post("localhost:11434")  │
└──────────────┬──────────────────────┘
               │ HTTP
┌──────────────▼──────────────────────┐
│          Ollama Server (Go)         │
│   HTTP routing, model lifecycle,    │
│   concurrent request handling,      │
│   model pulling/caching             │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│       llama.cpp Engine (C++)        │
│   GGUF loading, Metal/CUDA accel,  │
│   quantized inference, KV cache    │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│     Apple Metal GPU (Hardware)      │
└─────────────────────────────────────┘
```

### The Docker analogy

Ollama's mental model maps 1:1 to Docker:

```
Docker                          Ollama
──────                          ──────
docker pull ubuntu              ollama pull llama3.2
docker run ubuntu               ollama run llama3.2
docker images                   ollama list
docker ps                       ollama ps
docker rm                       ollama rm llama3.2
Dockerfile                      Modelfile
Docker Hub                      ollama.com/library
```

### Model format: GGUF

Ollama uses GGUF (GPT-Generated Unified Format), created by the llama.cpp project:

```
model.gguf = single file containing:
  ├── metadata (architecture, vocab size, layers, etc.)
  ├── tokenizer data
  └── quantized weights (already compressed)
```

GGUF is a self-contained format — one file has everything. No separate config.json or tokenizer files.

Ollama stores models as **layers** (like Docker), enabling deduplication:

```
~/.ollama/models/
  manifests/           ← which layers make up each model
  blobs/               ← the actual layer data (shared between models)

If llama3.2 and llama3.2-custom share the same base weights,
the weights are stored once and referenced by both.
```

### Installation

```bash
# macOS (recommended)
brew install ollama

# Start the server (or it auto-starts as a service)
ollama serve
# → Listening on localhost:11434
```

### CLI usage

```bash
# Pull a model
ollama pull llama3.2

# Interactive chat
ollama run llama3.2

# One-shot prompt
ollama run llama3.2 "Explain quicksort in 3 sentences"

# List local models
ollama list

# See loaded/running models
ollama ps

# Remove a model
ollama rm llama3.2

# Show model details
ollama show llama3.2
```

### REST API

All endpoints at `http://localhost:11434`:

```bash
# Chat (non-streaming)
curl http://localhost:11434/api/chat -d '{
  "model": "llama3.2",
  "messages": [{"role": "user", "content": "Hello"}],
  "stream": false
}'
# → {"message": {"role": "assistant", "content": "Hello! How can I help?"}, "done": true}

# Chat (streaming — default)
curl http://localhost:11434/api/chat -d '{
  "model": "llama3.2",
  "messages": [{"role": "user", "content": "Hello"}]
}'
# → one JSON object per token, newline-delimited

# Generate (simpler, no chat format)
curl http://localhost:11434/api/generate -d '{
  "model": "llama3.2",
  "prompt": "What is Rust?",
  "stream": false
}'

# Embeddings
curl http://localhost:11434/api/embed -d '{
  "model": "llama3.2",
  "input": "some text to embed"
}'

# Health check
curl http://localhost:11434/api/version
# → {"version": "0.5.x"}

# List models
curl http://localhost:11434/api/tags

# Currently loaded models
curl http://localhost:11434/api/ps
```

### Modelfile (custom models)

Like a Dockerfile, but for LLMs:

```dockerfile
# Modelfile
FROM llama3.2

PARAMETER temperature 0.2
PARAMETER num_ctx 8192

SYSTEM """You are Jarvis, a content analysis assistant.
Respond in JSON: {"tags": [...], "summary": "..."}"""
```

```bash
# Create and use
ollama create jarvis-tagger -f Modelfile
ollama run jarvis-tagger "Analyze: Apple announces M4 chip"
```

### Calling from Rust (Tauri integration)

```rust
use reqwest;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: Message,
}

async fn ask_ollama(prompt: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let resp = client
        .post("http://localhost:11434/api/chat")
        .json(&ChatRequest {
            model: "llama3.2".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            stream: false,
        })
        .send().await
        .map_err(|e| format!("Ollama request failed: {}", e))?;

    let chat: ChatResponse = resp.json().await
        .map_err(|e| format!("Parse failed: {}", e))?;

    Ok(chat.message.content)
}

async fn is_ollama_running() -> bool {
    reqwest::get("http://localhost:11434/api/version")
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}
```

### Ollama-available models

| Model | Command | Size (Q4) | Strengths |
|---|---|---|---|
| Llama 3.2 1B | `ollama pull llama3.2:1b` | ~0.7 GB | Fastest, basic tasks |
| Llama 3.2 3B | `ollama pull llama3.2` | ~1.8 GB | Good balance |
| Llama 3.1 8B | `ollama pull llama3.1:8b` | ~4.5 GB | Strong general purpose |
| Llama 3.1 70B | `ollama pull llama3.1:70b` | ~40 GB | Best open model |
| Mistral 7B | `ollama pull mistral` | ~4 GB | Fast, efficient |
| Mixtral 8x7B | `ollama pull mixtral` | ~24 GB | MoE, excellent quality |
| Phi-3 Mini 3.8B | `ollama pull phi3` | ~2 GB | Compact, strong |
| Phi-3 Medium 14B | `ollama pull phi3:14b` | ~7 GB | Very capable |
| Qwen 2.5 7B | `ollama pull qwen2.5:7b` | ~4 GB | Multilingual |
| Qwen 2.5 14B | `ollama pull qwen2.5:14b` | ~8 GB | Excellent quality |
| Qwen 2.5 72B | `ollama pull qwen2.5:72b` | ~40 GB | Top tier |
| Gemma 2 9B | `ollama pull gemma2:9b` | ~5 GB | Google's best small |
| Gemma 2 27B | `ollama pull gemma2:27b` | ~15 GB | Very strong |
| DeepSeek R1 7B | `ollama pull deepseek-r1:7b` | ~4 GB | Reasoning |
| DeepSeek R1 14B | `ollama pull deepseek-r1:14b` | ~8 GB | Better reasoning |
| CodeLlama 7B | `ollama pull codellama:7b` | ~4 GB | Code generation |
| CodeLlama 34B | `ollama pull codellama:34b` | ~18 GB | Best for code |

You can also import any GGUF model not in the registry:

```dockerfile
# Modelfile
FROM ./path/to/custom-model.gguf
```

### Ollama pros and cons

**Pros:**
- Zero dependencies — single Go binary, no Python/pip
- Model management built-in (pull, list, rm — like Docker)
- Dead-simple HTTP API — any language can call it
- Great Apple Silicon support (Metal, automatic)
- Streaming built-in — real-time token output
- Embeddings API included
- Modelfile system for customization
- Huge community (130k+ GitHub stars)
- Easy for end users to install (`brew install ollama`)

**Cons:**
- Separate process — must be running alongside your app
- No fine-tuning — run-only, cannot train models
- Cold-start latency — loading a new model takes 5-30s
- Memory overhead — ~500 MB+ even when idle with a model loaded
- Limited configuration vs raw llama.cpp
- GGUF format only (though most models are available)

---

## Part 10: MLX vs Ollama — Head-to-Head

| Aspect | Ollama | MLX-LM |
|---|---|---|
| **Install** | `brew install ollama` | `pip install mlx-lm` + Python 3.9+ |
| **Dependencies** | None | Python, pip, various Python packages |
| **Distribution** | User installs Ollama (common) | Must ensure Python env exists |
| **Model source** | `ollama pull model` (own registry) | Hugging Face downloads |
| **Model format** | GGUF | safetensors |
| **Integration** | HTTP API (any language) | Python API, or HTTP server mode |
| **From Rust/Tauri** | `reqwest` HTTP calls | HTTP to subprocess, or stdin/stdout |
| **Fine-tuning** | Not supported | LoRA and QLoRA supported |
| **Inference engine** | llama.cpp (C++) | MLX (Apple Metal shaders) |
| **Performance** | Excellent (llama.cpp Metal) | Slightly better (native Metal) |
| **Streaming** | Built-in (NDJSON) | Built-in (server mode or Python) |
| **Embeddings** | Built-in API | Separate setup needed |
| **Model availability** | Most popular models, fast updates | Anything on Hugging Face |
| **Custom models** | Modelfile (simple) | Full Python control |
| **Community** | 130k+ stars, huge ecosystem | 24k+ stars, Apple-backed |
| **macOS** | Metal GPU, automatic | Metal GPU, automatic |
| **Linux** | CUDA support | CUDA support (newer) |
| **Windows** | Supported | Not supported |

### For Jarvis: Ollama wins

1. **No Python dependency** — Jarvis is a Tauri/Rust app. Adding Python is a distribution headache.
2. **User may already have Ollama** — it's the most popular local LLM tool.
3. **Simple HTTP integration** — just `reqwest` calls, same pattern as IntelligenceKit's subprocess.
4. **Embeddings included** — useful for future semantic search over gems.
5. **Modelfile** — create a `jarvis-analyzer` custom model with a system prompt baked in.

MLX would be the choice if:
- You were building a Python app
- You needed fine-tuning on user data
- You wanted maximum control over the inference pipeline
- You were doing ML research, not app development

---

## Part 11: Recommended Models for Jarvis

### Tier 1: Fast (2-3 seconds for short tasks)

| Model | RAM | Speed | Use Case |
|---|---|---|---|
| `llama3.2:1b` | ~1 GB | ~40 tok/s | Quick tagging, classification |
| `llama3.2:3b` | ~2 GB | ~30 tok/s | Summaries, tag generation |
| `phi3` (3.8B) | ~2 GB | ~28 tok/s | Compact but capable |

### Tier 2: Balanced (5-10 seconds for analysis)

| Model | RAM | Speed | Use Case |
|---|---|---|---|
| `llama3.1:8b` | ~5 GB | ~20 tok/s | Detailed analysis, structured output |
| `mistral` (7B) | ~4 GB | ~22 tok/s | Good all-rounder |
| `qwen2.5:7b` | ~4 GB | ~22 tok/s | Strong multilingual support |

### Tier 3: High Quality (15-30 seconds)

| Model | RAM | Speed | Use Case |
|---|---|---|---|
| `qwen2.5:14b` | ~8 GB | ~15 tok/s | Excellent structured output |
| `gemma2:27b` | ~15 GB | ~10 tok/s | Very high quality analysis |
| `deepseek-r1:14b` | ~8 GB | ~12 tok/s | Complex reasoning tasks |

### Tier 4: Maximum Quality (30+ seconds, needs lots of RAM)

| Model | RAM | Speed | Use Case |
|---|---|---|---|
| `llama3.1:70b` | ~40 GB | ~6 tok/s | Best open model, near GPT-4 |
| `qwen2.5:72b` | ~40 GB | ~6 tok/s | Strongest multilingual |
| `mixtral` (8x7B) | ~24 GB | ~8 tok/s | MoE, great quality/speed ratio |

### Suggested default for Jarvis

```
IntelligenceKit (Apple Intelligence)
  → instant response (~100ms)
  → basic summary, low quality
  → already integrated

Ollama with llama3.2:3b (or llama3.1:8b)
  → 2-10 second response
  → good quality analysis, tagging, structured JSON output
  → the "heavy" fallback

User controls which to use, or auto-escalate:
  1. Try IntelligenceKit first (fast)
  2. If user wants better quality, run Ollama (accurate)
```

---

## Quick Mental Model

```
You have a gem (article/video/conversation).
You want: tags, summary, key topics, entities.

Option A: Apple Intelligence (IntelligenceKit)
  Speed: instant
  Quality: okay
  RAM: 0 (uses system model)
  Setup: already done

Option B: Ollama + small model (3B)
  Speed: 2-3 seconds
  Quality: good
  RAM: ~2 GB
  Setup: brew install ollama && ollama pull llama3.2

Option C: Ollama + medium model (8B)
  Speed: 5-10 seconds
  Quality: very good
  RAM: ~5 GB
  Setup: brew install ollama && ollama pull llama3.1:8b

Option D: Ollama + large model (70B)
  Speed: 30+ seconds
  Quality: excellent (near GPT-4)
  RAM: ~40 GB
  Setup: brew install ollama && ollama pull llama3.1:70b
```

The beauty of Ollama: switching between models is just changing the `"model"` string in the API call. Same endpoint, same format, different quality/speed tradeoff.
