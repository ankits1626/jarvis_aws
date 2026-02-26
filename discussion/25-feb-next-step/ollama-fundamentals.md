# Ollama Fundamentals

A practical, developer-focused guide to running LLMs locally with Ollama.

---

## 1. What is Ollama

Ollama is a local LLM runner built in Go. It uses **llama.cpp** under the hood for inference and ships as a self-contained binary -- no Python, no virtual environments, no dependency hell.

What it handles for you:

- **Model downloads** from its own registry (`ollama.com/library`)
- **Quantization** -- models are pre-quantized and ready to run
- **Serving** -- spins up an HTTP server on `localhost:11434`
- **GPU acceleration** -- Metal on macOS, CUDA on Linux/Windows

With ~130k+ GitHub stars, it has the largest community of any local LLM tool.

**Key insight**: Think of Ollama as "Docker for LLMs." You `pull` models, `run` them, and interact via a standard API. The mental model maps almost 1:1.

---

## 2. Architecture

```
┌─────────────────────────────────────────────────┐
│                   Your App                       │
│         (Tauri, Python, Node, curl)              │
└──────────────────┬──────────────────────────────┘
                   │ HTTP (localhost:11434)
┌──────────────────▼──────────────────────────────┐
│              Ollama Server (Go)                   │
│  ┌─────────────────────────────────────────────┐ │
│  │  Model Manager     │  API Router            │ │
│  │  - Pull / Push     │  - /api/generate       │ │
│  │  - Layer caching   │  - /api/chat           │ │
│  │  - Modelfile       │  - /api/embed          │ │
│  └─────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────┐ │
│  │          llama.cpp (C/C++)                   │ │
│  │  - GGUF model loading                       │ │
│  │  - Metal GPU acceleration (macOS)           │ │
│  │  - CUDA GPU acceleration (Linux/Windows)    │ │
│  │  - Quantized inference (Q4_K_M, Q5, etc.)   │ │
│  └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────┐
│           Model Storage (~/.ollama/models)        │
│  - GGUF format, stored as layers (like Docker)   │
│  - Shared layers between model variants          │
│  - Registry: ollama.com/library                  │
└─────────────────────────────────────────────────┘
```

**Key components**:

- **Go server**: Handles HTTP routing, model lifecycle, concurrent requests
- **llama.cpp**: The actual inference engine, compiled and embedded within Ollama
- **GGUF format**: Models stored as layers, enabling deduplication (a 7B base shared across fine-tunes)
- **Metal / CUDA**: GPU inference is automatic -- Ollama detects your hardware and offloads accordingly

---

## 3. Installation on macOS

### Option A: Homebrew (recommended)

```bash
brew install ollama
```

### Option B: Install script

```bash
curl -fsSL https://ollama.com/install.sh | sh
```

### Option C: DMG download

Download directly from [ollama.com](https://ollama.com) and drag to Applications.

### Starting the server

```bash
# Start the server manually
ollama serve

# Or it runs automatically as a macOS service after install
# Verify it's running:
curl http://localhost:11434/api/version
```

The server listens on `localhost:11434` by default. To change the host/port:

```bash
OLLAMA_HOST=0.0.0.0:8080 ollama serve
```

---

## 4. Available Models

| Model | Sizes | Best For |
|-------|-------|----------|
| `llama3.2` | 1B, 3B | General purpose, fast inference |
| `llama3.1` | 8B, 70B, 405B | General purpose, high quality |
| `mistral` | 7B | Fast with good quality balance |
| `mixtral` | 8x7B, 8x22B | Mixture of Experts, high quality |
| `phi3` | 3.8B, 14B | Compact and efficient |
| `qwen2.5` | 0.5B-72B | Multilingual, wide size range |
| `gemma2` | 2B, 9B, 27B | Google's open model |
| `deepseek-r1` | 1.5B-671B | Reasoning and chain-of-thought |
| `codellama` | 7B, 13B, 34B | Code generation and understanding |

Browse all models: [ollama.com/library](https://ollama.com/library)

**Pulling a specific size**:

```bash
# Default (smallest recommended)
ollama pull llama3.2

# Specific size
ollama pull llama3.1:8b
ollama pull llama3.1:70b

# Specific quantization
ollama pull llama3.1:8b-q5_K_M
```

---

## 5. CLI Usage

```bash
# Pull and run a model (interactive chat)
ollama run llama3.2

# Pull a model without running
ollama pull mistral

# List downloaded models
ollama list

# Start the server (if not running as a service)
ollama serve

# Create a custom model from a Modelfile
ollama create mymodel -f Modelfile

# Show model details (parameters, template, license)
ollama show llama3.2

# Show currently loaded/running models
ollama ps

# Remove a model
ollama rm mistral

# Copy a model (for creating variants)
ollama cp llama3.2 my-llama

# Run with a one-shot prompt (non-interactive)
ollama run llama3.2 "Explain quicksort in 3 sentences"
```

**Useful patterns**:

```bash
# Pipe input to Ollama
echo "Summarize this: $(cat article.txt)" | ollama run llama3.2

# Set system prompt inline
ollama run llama3.2 --system "You are a JSON generator. Output only valid JSON."

# Check version
ollama --version
```

---

## 6. REST API

All endpoints are served at `http://localhost:11434`.

| Method | Endpoint | Purpose |
|--------|----------|---------|
| `POST` | `/api/generate` | Generate a completion |
| `POST` | `/api/chat` | Chat completion (multi-turn) |
| `GET` | `/api/tags` | List local models |
| `POST` | `/api/show` | Get model information |
| `POST` | `/api/pull` | Pull a model from registry |
| `POST` | `/api/embed` | Generate embeddings |
| `GET` | `/api/ps` | List running/loaded models |
| `GET` | `/api/version` | Get Ollama version |
| `DELETE` | `/api/delete` | Delete a model |
| `POST` | `/api/create` | Create a model from Modelfile |

**Default behavior**: All generation endpoints stream by default. Set `"stream": false` for a single JSON response.

---

## 7. API Examples with curl

### Non-streaming generate

```bash
curl http://localhost:11434/api/generate -d '{
  "model": "llama3.2",
  "prompt": "Summarize this article in 3 tags",
  "stream": false
}'
```

Response:

```json
{
  "model": "llama3.2",
  "response": "#technology #AI #local-inference",
  "done": true,
  "total_duration": 1234567890,
  "eval_count": 12,
  "eval_duration": 987654321
}
```

### Streaming chat (default)

```bash
curl http://localhost:11434/api/chat -d '{
  "model": "llama3.2",
  "messages": [{"role": "user", "content": "Hello"}]
}'
```

Returns newline-delimited JSON objects, one per token:

```json
{"model":"llama3.2","message":{"role":"assistant","content":"Hello"},"done":false}
{"model":"llama3.2","message":{"role":"assistant","content":"!"},"done":false}
{"model":"llama3.2","message":{"role":"assistant","content":" How"},"done":false}
...
{"model":"llama3.2","message":{"role":"assistant","content":""},"done":true}
```

### Non-streaming chat

```bash
curl http://localhost:11434/api/chat -d '{
  "model": "llama3.2",
  "messages": [{"role": "user", "content": "Hello"}],
  "stream": false
}'
```

Response:

```json
{
  "model": "llama3.2",
  "message": {
    "role": "assistant",
    "content": "Hello! How can I help you today?"
  },
  "done": true,
  "total_duration": 1034567890
}
```

### Multi-turn chat

```bash
curl http://localhost:11434/api/chat -d '{
  "model": "llama3.2",
  "messages": [
    {"role": "system", "content": "You are a helpful coding assistant."},
    {"role": "user", "content": "What is a closure in Rust?"},
    {"role": "assistant", "content": "A closure in Rust is an anonymous function..."},
    {"role": "user", "content": "Give me an example."}
  ],
  "stream": false
}'
```

### Generate embeddings

```bash
curl http://localhost:11434/api/embed -d '{
  "model": "llama3.2",
  "input": "Ollama is a local LLM runner"
}'
```

Response:

```json
{
  "model": "llama3.2",
  "embeddings": [[0.123, -0.456, 0.789, ...]]
}
```

### List local models

```bash
curl http://localhost:11434/api/tags
```

### Show model info

```bash
curl http://localhost:11434/api/show -d '{
  "name": "llama3.2"
}'
```

---

## 8. Modelfile Customization

A Modelfile is to Ollama what a Dockerfile is to Docker -- it defines a custom model configuration.

### Basic Modelfile

```dockerfile
# Modelfile
FROM llama3.2

PARAMETER temperature 0.3
PARAMETER num_ctx 4096

SYSTEM "You are a content analysis assistant. Generate concise tags and summaries."
```

### Create and use the custom model

```bash
# Create the model
ollama create jarvis-tagger -f Modelfile

# Run it
ollama run jarvis-tagger

# Use via API
curl http://localhost:11434/api/chat -d '{
  "model": "jarvis-tagger",
  "messages": [{"role": "user", "content": "Tag this: Apple releases new M4 chip"}],
  "stream": false
}'
```

### Advanced Modelfile

```dockerfile
FROM llama3.2

# Inference parameters
PARAMETER temperature 0.2
PARAMETER top_p 0.9
PARAMETER top_k 40
PARAMETER num_ctx 8192
PARAMETER repeat_penalty 1.1
PARAMETER stop "<|end|>"
PARAMETER stop "<|user|>"

# System prompt
SYSTEM """You are Jarvis, an intelligent content analysis assistant.

Your capabilities:
- Generate concise tags for articles and videos
- Summarize content in 2-3 sentences
- Extract key entities (people, companies, technologies)

Always respond in valid JSON format:
{
  "tags": ["tag1", "tag2", "tag3"],
  "summary": "...",
  "entities": ["entity1", "entity2"]
}
"""

# Conversation template (for few-shot examples)
MESSAGE user Analyze: "OpenAI announces GPT-5 with improved reasoning"
MESSAGE assistant {"tags": ["#ai", "#openai", "#gpt5"], "summary": "OpenAI has announced GPT-5, featuring enhanced reasoning capabilities.", "entities": ["OpenAI", "GPT-5"]}
```

### Available parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `temperature` | 0.8 | Creativity (0.0 = deterministic, 1.0 = creative) |
| `top_p` | 0.9 | Nucleus sampling threshold |
| `top_k` | 40 | Top-k sampling |
| `num_ctx` | 2048 | Context window size (tokens) |
| `repeat_penalty` | 1.1 | Penalty for repeating tokens |
| `seed` | 0 | Random seed (0 = random, set for reproducibility) |
| `num_predict` | -1 | Max tokens to generate (-1 = infinite) |
| `stop` | - | Stop sequences |

---

## 9. Memory / Hardware Requirements

| Parameters | Quantization | RAM Needed | Example Models |
|-----------|-------------|-----------|----------------|
| 1-3B | Q4_K_M | 2-3 GB | `phi3-mini`, `llama3.2:1b` |
| 7-8B | Q4_K_M | 4-5 GB | `llama3.1:8b`, `mistral` |
| 13B | Q4_K_M | 8-10 GB | `codellama:13b` |
| 34B | Q4_K_M | 20 GB | `codellama:34b` |
| 70B | Q4_K_M | 40 GB | `llama3.1:70b` |

**Rules of thumb**:

- **Q4_K_M** is the default quantization -- best balance of quality vs. size
- A model needs roughly `(parameters in billions) * 0.6 GB` of RAM at Q4 quantization
- You need additional RAM beyond the model size for context window (KV cache)
- Larger context windows (`num_ctx`) consume more memory: 8192 context on a 7B model adds ~1 GB
- If a model does not fit entirely in GPU memory, Ollama will split across GPU + CPU (slower)

**Checking actual memory usage**:

```bash
# See loaded models and their memory usage
ollama ps

# System-level monitoring
# macOS
top -l 1 | grep ollama
```

---

## 10. Performance on Apple Silicon

Ollama uses **Metal** for GPU inference on macOS. All Apple Silicon Macs (M1/M2/M3/M4) have unified memory, meaning the GPU can access all system RAM directly -- no separate VRAM limitation.

| Chip | RAM | Recommended Models | Approx. Speed |
|------|-----|--------------------|---------------|
| M1 (8 GB) | 8 GB | 1B-7B models | ~15-20 tok/s (7B) |
| M1 Pro/Max (16-64 GB) | 16+ GB | 7B-13B comfortably | ~25-30 tok/s (7B) |
| M2/M3 Pro (18 GB) | 18 GB | 7B-13B comfortably | ~30-35 tok/s (7B) |
| M3 Max (36-128 GB) | 36+ GB | Up to 70B models | ~35-40 tok/s (7B) |
| M3 Ultra (64-192 GB) | 64+ GB | 70B+ models | ~40+ tok/s (7B) |

**Key points**:

- All inference runs on GPU via Metal -- no CUDA needed, no configuration required
- Unified memory means no PCIe bottleneck for model loading
- Larger models that fit entirely in memory get full GPU acceleration
- M-series chips have excellent memory bandwidth (100-800 GB/s depending on chip)
- Token generation speed is primarily limited by memory bandwidth, not compute

**Benchmark a model yourself**:

```bash
# The API response includes timing information
curl http://localhost:11434/api/generate -d '{
  "model": "llama3.2",
  "prompt": "Write a haiku about programming",
  "stream": false
}' | python3 -c "
import json, sys
d = json.load(sys.stdin)
tokens = d['eval_count']
duration_s = d['eval_duration'] / 1e9
print(f'{tokens} tokens in {duration_s:.2f}s = {tokens/duration_s:.1f} tok/s')
"
```

---

## 11. Detecting if Ollama is Running

### Health check

```bash
# Simple version check
curl -s http://localhost:11434/api/version
# Returns: {"version":"0.5.x"} if running
# Connection refused if not running
```

### Check with error handling (bash)

```bash
if curl -s --connect-timeout 2 http://localhost:11434/api/version > /dev/null 2>&1; then
    echo "Ollama is running"
else
    echo "Ollama is not running"
fi
```

### Check loaded models

```bash
# Which models are currently loaded in memory?
curl -s http://localhost:11434/api/ps
```

Response:

```json
{
  "models": [
    {
      "name": "llama3.2:latest",
      "model": "llama3.2:latest",
      "size": 2019393189,
      "digest": "a80c4f17acd5...",
      "expires_at": "2025-01-15T10:30:00Z"
    }
  ]
}
```

### Check from Rust (for Tauri integration)

```rust
async fn is_ollama_running() -> bool {
    reqwest::get("http://localhost:11434/api/version")
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}
```

---

## 12. Tauri / Rust Integration

Use the `reqwest` HTTP client to communicate with Ollama from a Tauri backend.

### Cargo.toml dependencies

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
```

### Full implementation

```rust
use reqwest;
use serde::{Deserialize, Serialize};

// --- Request types ---

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct EmbedRequest {
    model: String,
    input: String,
}

// --- Response types ---

#[derive(Deserialize)]
struct ChatResponse {
    message: MessageContent,
}

#[derive(Deserialize)]
struct MessageContent {
    content: String,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f64>>,
}

#[derive(Deserialize)]
struct VersionResponse {
    version: String,
}

// --- Core functions ---

const OLLAMA_BASE: &str = "http://localhost:11434";

/// Check if Ollama is running and return its version
async fn check_ollama() -> Result<String, String> {
    let resp = reqwest::get(format!("{}/api/version", OLLAMA_BASE))
        .await
        .map_err(|e| format!("Ollama not reachable: {}", e))?;

    let version: VersionResponse = resp.json().await
        .map_err(|e| format!("Failed to parse version: {}", e))?;

    Ok(version.version)
}

/// Send a chat message to Ollama and get a response
async fn ask_ollama(prompt: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/api/chat", OLLAMA_BASE))
        .json(&ChatRequest {
            model: "llama3.2".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            stream: false,
        })
        .send()
        .await
        .map_err(|e| format!("Ollama request failed: {}", e))?;

    let chat_resp: ChatResponse = resp.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(chat_resp.message.content)
}

/// Send a chat with system prompt and conversation history
async fn chat_with_context(
    system_prompt: &str,
    messages: Vec<Message>,
) -> Result<String, String> {
    let client = reqwest::Client::new();

    let mut all_messages = vec![Message {
        role: "system".to_string(),
        content: system_prompt.to_string(),
    }];
    all_messages.extend(messages);

    let resp = client
        .post(format!("{}/api/chat", OLLAMA_BASE))
        .json(&ChatRequest {
            model: "llama3.2".to_string(),
            messages: all_messages,
            stream: false,
        })
        .send()
        .await
        .map_err(|e| format!("Ollama request failed: {}", e))?;

    let chat_resp: ChatResponse = resp.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(chat_resp.message.content)
}

/// Generate embeddings for a text
async fn get_embedding(text: &str) -> Result<Vec<f64>, String> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/api/embed", OLLAMA_BASE))
        .json(&EmbedRequest {
            model: "llama3.2".to_string(),
            input: text.to_string(),
        })
        .send()
        .await
        .map_err(|e| format!("Embedding request failed: {}", e))?;

    let embed_resp: EmbedResponse = resp.json().await
        .map_err(|e| format!("Failed to parse embedding: {}", e))?;

    embed_resp.embeddings.into_iter().next()
        .ok_or_else(|| "No embedding returned".to_string())
}
```

### Tauri command integration

```rust
#[tauri::command]
async fn analyze_content(content: String) -> Result<String, String> {
    let system = "You are a content tagger. Respond with JSON: {\"tags\": [...], \"summary\": \"...\"}";
    chat_with_context(system, vec![Message {
        role: "user".to_string(),
        content,
    }]).await
}

#[tauri::command]
async fn ollama_health() -> Result<String, String> {
    check_ollama().await
}
```

---

## 13. Pros and Cons

### Pros

- **Self-contained Go binary** -- no Python, no pip, no conda, no virtual environments
- **Excellent model management** -- pull, list, remove, copy, just like Docker
- **Huge community** -- 130k+ stars, active development, wide model support
- **Dead-simple API** -- one POST request to get a response
- **Great Apple Silicon support** -- Metal acceleration works out of the box
- **Streaming** -- real-time token-by-token output for responsive UIs
- **Embeddings API** -- built-in, same interface, no separate tool needed
- **Modelfile system** -- customize models with system prompts, parameters, templates
- **Wide model support** -- most popular open models available within days of release

### Cons

- **Requires separate install** -- not embeddable as a library, must run as a process
- **No fine-tuning built-in** -- you cannot train or fine-tune models, only run them
- **Model switching has cold-start latency** -- loading a new model takes 5-30s depending on size
- **Memory overhead** -- uses ~500 MB+ RAM even when idle with a model loaded
- **Must be running as a background service** -- your app depends on Ollama being started
- **Single-machine only** -- no built-in clustering or distributed inference
- **Limited configuration** -- fewer knobs compared to raw llama.cpp
- **Model format lock-in** -- only supports GGUF format (though most models are available)

---

## 14. Comparison with Alternatives

| Feature | Ollama | MLX | llama.cpp | LM Studio |
|---------|--------|-----|-----------|-----------|
| **Install** | Binary / brew | pip (Python) | Build from source | GUI app |
| **Language** | Go | Python | C/C++ | Electron |
| **API** | REST (port 11434) | Python / REST | CLI / REST | REST (port 1234) |
| **Models** | Own registry | HuggingFace | Manual GGUF download | GUI browser |
| **GPU** | Metal / CUDA | Metal / CUDA | Metal / CUDA / Vulkan | Metal / CUDA |
| **Embeddings** | Yes (built-in) | Via separate lib | Yes | Yes |
| **Integration** | HTTP (easiest) | Python required | Build/link C library | HTTP |
| **Community** | Largest (~130k stars) | Growing (Apple-backed) | Large (~75k stars) | Medium |
| **Customization** | Modelfile | Python code | Full control | GUI settings |
| **Best for** | App integration | Python/ML workflows | Maximum control | Non-developers |

### When to use what

- **Ollama**: You want the easiest path to integrate local LLMs into any app via HTTP. Best for Tauri/Rust, web apps, or any language with an HTTP client.
- **MLX**: You are in a Python ecosystem and want Apple Silicon optimized inference with full control.
- **llama.cpp**: You need maximum performance, want to embed inference in a C/C++ application, or need the most bleeding-edge features.
- **LM Studio**: You want a GUI to experiment with models before writing code.

---

## Quick Start Cheatsheet

```bash
# Install
brew install ollama

# Start server
ollama serve

# Pull and run a model
ollama run llama3.2

# One-shot prompt
ollama run llama3.2 "What is Rust's ownership model?"

# API call from any language
curl http://localhost:11434/api/chat -d '{
  "model": "llama3.2",
  "messages": [{"role": "user", "content": "Hello!"}],
  "stream": false
}'

# Check status
curl http://localhost:11434/api/version
curl http://localhost:11434/api/ps
ollama list
```
