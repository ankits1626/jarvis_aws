# MLX Fundamentals

A developer-focused reference for Apple's MLX framework and MLX-LM library.

---

## 1. What is MLX

MLX is Apple's open-source machine learning framework designed specifically for Apple Silicon. It provides a NumPy-like API that feels familiar to anyone who has worked with Python numerical computing, while leveraging the unique hardware capabilities of M-series chips.

- **Repository**: [github.com/ml-explore/mlx](https://github.com/ml-explore/mlx)
- **Version**: v0.30+ as of February 2026
- **Community**: ~24k GitHub stars
- **Language bindings**: Python (primary), C++, Swift

MLX was created by Apple's machine learning research team to give developers a high-performance ML framework that takes full advantage of Apple Silicon's unified memory architecture -- something that PyTorch and TensorFlow were never designed around.

---

## 2. How MLX Differs from PyTorch and TensorFlow

| Aspect | MLX | PyTorch | TensorFlow/JAX |
|---|---|---|---|
| **Memory model** | Unified (shared CPU/GPU) | Separate CPU & GPU memory, explicit `.to(device)` transfers | Separate memory spaces |
| **Evaluation** | Lazy (deferred until needed) | Eager by default | TF2 eager, JAX lazy |
| **Graph construction** | Dynamic (like PyTorch) | Dynamic | TF1 static, TF2/JAX dynamic |
| **Function transforms** | Composable (`grad`, `vmap`, `jit`) -- JAX-style | `torch.compile`, `autograd` | JAX has composable transforms |
| **Hardware** | Apple Silicon (Metal GPU), CUDA on Linux | CUDA, ROCm, MPS, CPU | CUDA, TPU, CPU |
| **Memory copies** | None between CPU/GPU | Explicit `.cuda()` / `.cpu()` | Explicit device placement |

The key differentiator: **unified memory eliminates the CPU-to-GPU data transfer bottleneck**. On a PyTorch setup with a discrete GPU, moving a 7B model's weights to VRAM is a blocking operation. On MLX with Apple Silicon, the array already lives in memory accessible by both CPU and GPU -- no copy needed.

MLX also borrows the best ideas from JAX (composable function transformations) while keeping PyTorch's imperative, debug-friendly programming model.

---

## 3. Architecture

### Unified Memory Model

The defining architectural feature. On Apple Silicon, CPU and GPU share the same physical memory (DRAM). MLX arrays live in this shared pool:

```
Traditional (PyTorch + CUDA):
  CPU RAM  ──copy──>  GPU VRAM
  [weights]           [weights]    (data duplicated)

MLX on Apple Silicon:
  Unified Memory
  [weights]  <── CPU reads directly
             <── GPU reads directly
             (single copy, zero transfer overhead)
```

This means a machine with 128GB of unified memory can load a 70B parameter model that the GPU operates on directly -- no VRAM limitation separate from system RAM.

### Lazy Evaluation

Computations are not executed immediately. MLX builds a computation graph and only materializes results when explicitly needed (e.g., when you call `mx.eval()` or print a value):

```python
import mlx.core as mx

a = mx.ones((1000, 1000))
b = mx.ones((1000, 1000))
c = a + b          # No computation happens yet
mx.eval(c)         # NOW the addition runs on the GPU
print(c)           # Also triggers evaluation if not yet evaluated
```

This enables graph-level optimizations -- the framework can fuse operations, eliminate dead code, and schedule work efficiently.

### Composable Function Transforms

Like JAX, MLX provides transforms that can be composed freely:

```python
import mlx.core as mx
import mlx.nn as nn

# grad: automatic differentiation
loss_fn = lambda model, x, y: nn.losses.cross_entropy(model(x), y).mean()
grad_fn = mx.grad(loss_fn)

# jit: just-in-time compilation
fast_grad_fn = mx.compile(grad_fn)

# vmap: auto-vectorization (batching)
batched_fn = mx.vmap(some_function)

# These compose:
fast_batched_grad = mx.compile(mx.vmap(mx.grad(loss_fn)))
```

### GPU Backend

- **macOS**: Metal Performance Shaders (MPS) via Apple's Metal API
- **Linux**: CUDA support (newer, added for broader compatibility)

---

## 4. MLX-LM

MLX-LM is the library built on top of MLX specifically for running and fine-tuning large language models.

- **Repository**: [github.com/ml-explore/mlx-lm](https://github.com/ml-explore/mlx-lm)
- **Install**: `pip install mlx-lm`

### Supported Model Families

LLaMA (1/2/3/3.1/3.2), Mistral, Mixtral, Phi (1/2/3/4), Qwen (1/2/2.5), Gemma (1/2), DeepSeek, Cohere Command-R, StarCoder, and many more. Essentially any Hugging Face transformer architecture with a supported implementation.

### Key Features

- **Quantization**: 2-bit, 4-bit, and 8-bit weight quantization for reduced memory usage
- **LoRA / QLoRA fine-tuning**: Parameter-efficient fine-tuning directly on your Mac
- **Prompt caching**: Cache processed prompt prefixes for faster repeated inference
- **KV cache optimization**: Efficient key-value cache management for long contexts
- **Streaming generation**: Token-by-token output for responsive UIs
- **OpenAI-compatible server**: Drop-in replacement for OpenAI API calls

---

## 5. Running Models

### Python API

```python
from mlx_lm import load, generate

# Load a pre-quantized model from Hugging Face
model, tokenizer = load("mlx-community/Llama-3.2-3B-Instruct-4bit")

# Simple generation
response = generate(
    model,
    tokenizer,
    prompt="Explain unified memory in one paragraph.",
    max_tokens=200
)
print(response)
```

### Streaming Generation

```python
from mlx_lm import load, stream_generate

model, tokenizer = load("mlx-community/Llama-3.2-3B-Instruct-4bit")

prompt = "Write a haiku about machine learning."
for token_text in stream_generate(model, tokenizer, prompt=prompt, max_tokens=100):
    print(token_text, end="", flush=True)
print()
```

### Chat with Message Formatting

```python
from mlx_lm import load, generate

model, tokenizer = load("mlx-community/Llama-3.2-3B-Instruct-4bit")

messages = [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "What is MLX?"}
]

# Apply the model's chat template
prompt = tokenizer.apply_chat_template(messages, tokenize=False, add_generation_prompt=True)
response = generate(model, tokenizer, prompt=prompt, max_tokens=300)
print(response)
```

### CLI Usage

```bash
# One-shot generation
mlx_lm.generate \
  --model mlx-community/Llama-3.2-3B-Instruct-4bit \
  --prompt "Hello, how are you?"

# Interactive chat
mlx_lm.chat \
  --model mlx-community/Llama-3.2-3B-Instruct-4bit

# With generation parameters
mlx_lm.generate \
  --model mlx-community/Llama-3.2-3B-Instruct-4bit \
  --prompt "Write a poem" \
  --max-tokens 200 \
  --temp 0.7 \
  --top-p 0.9
```

---

## 6. Server Mode

MLX-LM can run as an OpenAI-compatible HTTP server, making it a local drop-in replacement for the OpenAI API.

### Starting the Server

```bash
mlx_lm.server \
  --model mlx-community/Llama-3.2-3B-Instruct-4bit \
  --port 8080
```

### Calling the API

```bash
curl http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "default",
    "messages": [
      {"role": "system", "content": "You are a helpful assistant."},
      {"role": "user", "content": "Hello"}
    ],
    "max_tokens": 100,
    "temperature": 0.7
  }'
```

### Using with OpenAI Python Client

Because the server is OpenAI-compatible, you can use the standard OpenAI SDK:

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8080/v1",
    api_key="not-needed"  # MLX server doesn't require auth
)

response = client.chat.completions.create(
    model="default",
    messages=[
        {"role": "user", "content": "What is the capital of France?"}
    ],
    max_tokens=100
)

print(response.choices[0].message.content)
```

### Supported Endpoints

- `POST /v1/chat/completions` -- Chat completions (streaming and non-streaming)
- `POST /v1/completions` -- Text completions
- `GET /v1/models` -- List available models

---

## 7. Model Format

### File Structure

MLX models use the same format conventions as Hugging Face models:

```
model-directory/
  config.json           # Model architecture config (hidden_size, num_layers, etc.)
  tokenizer.json        # Tokenizer definition
  tokenizer_config.json # Tokenizer settings
  *.safetensors         # Model weights in safetensors format
  special_tokens_map.json
```

The key difference from standard HF models: weights may be quantized and stored in MLX's expected layout.

### MLX Community on Hugging Face

The [mlx-community](https://huggingface.co/mlx-community) organization on Hugging Face hosts thousands of pre-converted and pre-quantized models ready for use. These are community-maintained conversions of popular models.

Examples:
- `mlx-community/Llama-3.2-3B-Instruct-4bit`
- `mlx-community/Mistral-7B-Instruct-v0.3-4bit`
- `mlx-community/Phi-3-mini-4k-instruct-4bit`
- `mlx-community/Qwen2.5-7B-Instruct-4bit`

### Converting Models Yourself

Convert a Hugging Face model to MLX format with quantization:

```bash
# Convert and quantize to 4-bit
mlx_lm.convert \
  --hf-path meta-llama/Llama-3.2-3B \
  --mlx-path ./mlx-model \
  -q \
  --q-bits 4

# Convert with 8-bit quantization
mlx_lm.convert \
  --hf-path mistralai/Mistral-7B-Instruct-v0.3 \
  --mlx-path ./mistral-8bit \
  -q \
  --q-bits 8

# Convert without quantization (full precision)
mlx_lm.convert \
  --hf-path meta-llama/Llama-3.2-3B \
  --mlx-path ./mlx-model-fp16
```

### Upload to Hugging Face

```bash
mlx_lm.convert \
  --hf-path meta-llama/Llama-3.2-3B \
  --mlx-path ./mlx-model \
  -q --q-bits 4 \
  --upload-repo your-username/Llama-3.2-3B-4bit-mlx
```

---

## 8. Performance on Apple Silicon

### Inference Speed (Approximate, 4-bit Quantized)

| Chip | 7B Model | 13B Model | 70B Model |
|---|---|---|---|
| M1 (8GB) | ~15-20 tok/s | Not enough RAM | -- |
| M1 Pro (16GB) | ~20-25 tok/s | ~10-15 tok/s | -- |
| M2 Max (32GB) | ~35-45 tok/s | ~20-25 tok/s | -- |
| M3 Max (64GB) | ~40-50 tok/s | ~25-30 tok/s | ~10-12 tok/s |
| M4 Max (128GB) | ~50+ tok/s | ~30-35 tok/s | ~15-20 tok/s |

*Benchmarks vary significantly by model architecture, quantization level, prompt length, and generation length. These are rough ballpark numbers for token generation speed.*

### Why MLX Performs Well on Apple Silicon

1. **Zero-copy memory access**: GPU reads weights directly from unified memory
2. **Metal shader optimization**: Custom Metal compute kernels for transformer operations
3. **Quantized matmul kernels**: Specialized kernels for 4-bit and 8-bit matrix multiplication
4. **Memory bandwidth utilization**: Apple Silicon has high memory bandwidth (M4 Max: ~546 GB/s) which is the bottleneck for LLM inference

### Prompt Processing vs. Token Generation

- **Prompt processing (prefill)**: Compute-bound, benefits from GPU parallelism. Typically much faster per token.
- **Token generation (decode)**: Memory-bandwidth-bound, sequential. This is the number reported as "tok/s" above.

---

## 9. Memory Requirements

### Rule of Thumb

For **4-bit quantized** models:

```
Memory needed ~ 0.5 GB per billion parameters (+ ~1-2 GB overhead)
```

| Model Size | 4-bit Memory | 8-bit Memory | FP16 Memory |
|---|---|---|---|
| 1B | ~1 GB | ~1.5 GB | ~2.5 GB |
| 3B | ~2 GB | ~3.5 GB | ~6.5 GB |
| 7B | ~4 GB | ~7.5 GB | ~14.5 GB |
| 13B | ~7 GB | ~13.5 GB | ~26.5 GB |
| 34B | ~18 GB | ~35 GB | ~68 GB |
| 70B | ~35 GB | ~70 GB | ~140 GB |

### Unified Memory Advantage

On a traditional setup (e.g., PyTorch + NVIDIA GPU):
- System has 32GB RAM + 24GB VRAM (e.g., RTX 4090)
- A 70B 4-bit model needs ~35GB -- does not fit in 24GB VRAM
- Must use CPU offloading or model parallelism across GPUs

On Apple Silicon (e.g., M4 Max with 128GB unified memory):
- The full 128GB is available to both CPU and GPU
- A 70B 4-bit model (~35GB) loads directly, GPU operates on it in-place
- No offloading, no splitting, no copies

### Checking Available Memory

```python
import mlx.core as mx

# Check active memory usage
print(f"Active memory: {mx.metal.get_active_memory() / 1e9:.2f} GB")
print(f"Peak memory: {mx.metal.get_peak_memory() / 1e9:.2f} GB")
print(f"Cache memory: {mx.metal.get_cache_memory() / 1e9:.2f} GB")
```

---

## 10. Integration into a Tauri/Rust App

MLX-LM is a Python library, so integrating it into a Tauri (Rust) application requires bridging the language boundary. There are two practical approaches.

### Approach A: HTTP Sidecar (Recommended)

Run `mlx_lm.server` as a subprocess and communicate via the OpenAI-compatible REST API.

```
[Tauri App (Rust)]
       |
       | HTTP requests (localhost:8080)
       |
[mlx_lm.server subprocess]
       |
       | MLX inference
       |
[Apple Silicon GPU via Metal]
```

**Rust side (Tauri command):**

```rust
use std::process::{Command, Child};
use reqwest;

// Start the MLX server as a sidecar process
fn start_mlx_server(model: &str, port: u16) -> std::io::Result<Child> {
    Command::new("python")
        .args(["-m", "mlx_lm.server",
               "--model", model,
               "--port", &port.to_string()])
        .spawn()
}

// Send a chat completion request
async fn chat(prompt: &str) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8080/v1/chat/completions")
        .json(&serde_json::json!({
            "model": "default",
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": 200
        }))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    Ok(response["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string())
}
```

**Pros**: Clean separation, OpenAI-compatible API, easy to swap models, streaming support via SSE.
**Cons**: HTTP overhead (minimal for local), requires managing subprocess lifecycle.

### Approach B: Python Sidecar via stdin/stdout

Bundle a Python script that uses mlx-lm directly. Communicate via stdin/stdout using NDJSON (newline-delimited JSON), similar to how Apple's IntelligenceKit works.

```
[Tauri App (Rust)]
       |
       | stdin/stdout (NDJSON)
       |
[Python sidecar script]
       |
       | mlx-lm Python API
       |
[Apple Silicon GPU via Metal]
```

**Python sidecar script (`mlx_sidecar.py`):**

```python
import sys
import json
from mlx_lm import load, stream_generate

def main():
    model, tokenizer = None, None

    for line in sys.stdin:
        request = json.loads(line.strip())

        if request["type"] == "load":
            model, tokenizer = load(request["model"])
            print(json.dumps({"type": "loaded", "model": request["model"]}), flush=True)

        elif request["type"] == "generate":
            prompt = request["prompt"]
            for token in stream_generate(model, tokenizer, prompt=prompt, max_tokens=request.get("max_tokens", 200)):
                print(json.dumps({"type": "token", "text": token}), flush=True)
            print(json.dumps({"type": "done"}), flush=True)

if __name__ == "__main__":
    main()
```

**Rust side:**

```rust
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader, Write};

fn spawn_sidecar() -> std::io::Result<std::process::Child> {
    Command::new("python")
        .arg("mlx_sidecar.py")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
}
```

**Pros**: Lower latency than HTTP, more control over generation, no port management.
**Cons**: Must handle process I/O carefully, custom protocol, harder to debug.

### Requirements

Both approaches require:
- Python 3.9+ installed on the host machine
- `mlx-lm` package installed (`pip install mlx-lm`)
- macOS with Apple Silicon (M1 or later)
- Sufficient unified memory for the target model

---

## 11. Pros and Cons

### Pros

- **Native Apple Silicon optimization**: Built from the ground up for M-series chips, Metal GPU backend, hand-tuned kernels
- **Unified memory efficiency**: No CPU-GPU copies, full system memory available for models. A 128GB Mac can run 70B models that would require multiple NVIDIA GPUs
- **Active development by Apple**: Regular releases, expanding model support, growing feature set
- **Hugging Face ecosystem**: Thousands of pre-converted models on mlx-community, standard safetensors format
- **OpenAI-compatible server**: Drop-in replacement for OpenAI API, works with existing client libraries
- **Composable transforms**: JAX-style `grad`, `vmap`, `jit` with PyTorch-style eager debugging
- **Fine-tuning support**: LoRA and QLoRA fine-tuning directly on consumer Apple hardware
- **Quantization**: 2/4/8-bit quantization with minimal quality loss, significant memory savings

### Cons

- **Platform limited**: macOS with Apple Silicon only for GPU acceleration. Linux CUDA support is newer and less mature
- **Requires Python runtime**: Cannot be compiled into a standalone binary. A Rust/Tauri app must bundle or depend on a Python environment
- **Not easily embeddable in Rust**: No native Rust bindings. Must use HTTP or subprocess bridges (unlike llama.cpp which has C API bindings)
- **Smaller community than Ollama**: Ollama provides a more turnkey experience with a single binary download. MLX-LM requires Python setup
- **No standalone binary**: Unlike Ollama or llama.cpp, you cannot distribute a single executable. Users need Python + pip + mlx-lm installed
- **Model compatibility**: Not every Hugging Face model has an MLX conversion. Coverage is good for popular models but gaps exist for niche architectures
- **macOS version dependency**: Requires relatively recent macOS versions for full Metal feature support

### When to Choose MLX

- You are building for macOS / Apple Silicon exclusively
- You want maximum performance on Mac hardware
- You need fine-tuning capabilities on consumer hardware
- You are comfortable with Python in your stack
- You want to use the Hugging Face model ecosystem directly

### When to Choose Alternatives

- **Ollama**: When you want a single-binary, zero-config solution with broad OS support
- **llama.cpp**: When you need C/C++ integration, cross-platform support, or Rust bindings (via llama-cpp-rs)
- **PyTorch + Transformers**: When you need NVIDIA GPU support, or are building for cloud/Linux servers
- **vLLM / TGI**: When you need production-grade serving with batching, multi-GPU, and high concurrency

---

## Quick Reference

```bash
# Install
pip install mlx-lm

# Run a model (CLI)
mlx_lm.generate --model mlx-community/Llama-3.2-3B-Instruct-4bit --prompt "Hello"

# Interactive chat
mlx_lm.chat --model mlx-community/Llama-3.2-3B-Instruct-4bit

# Start OpenAI-compatible server
mlx_lm.server --model mlx-community/Llama-3.2-3B-Instruct-4bit --port 8080

# Convert and quantize a model
mlx_lm.convert --hf-path meta-llama/Llama-3.2-3B --mlx-path ./mlx-model -q --q-bits 4
```

```python
# Minimal Python usage
from mlx_lm import load, generate

model, tokenizer = load("mlx-community/Llama-3.2-3B-Instruct-4bit")
print(generate(model, tokenizer, prompt="Hello!", max_tokens=100))
```
