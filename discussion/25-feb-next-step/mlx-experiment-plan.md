# MLX Experiment Plan — Hands-On LLM Exploration

**Goal:** Play with real LLMs on your M3 Pro 36GB, see the internals (tokenization, embeddings, attention, weights), and pick a model for Jarvis.

**Philosophy:** You drive. I coach. Every experiment is a Python snippet you run and observe.

---

## Your Hardware

| Spec | Value |
|------|-------|
| Chip | M3 Pro |
| Unified Memory | 36 GB |
| Comfortable model size | 8B (Q4 = ~5 GB) |
| Max model size | 14B (Q4 = ~9 GB), tight but works |
| Sweet spot | **8B Q4** — fast, fits easily, leaves room for KV cache + OS |

---

## Setup (One-Time, ~5 min)

```bash
# Create a virtual environment for our experiments
cd discussion/25-feb-next-step
python3 -m venv mlx-env
source mlx-env/bin/activate

# Install MLX ecosystem
pip install mlx mlx-lm transformers huggingface_hub

# Verify
python3 -c "import mlx; print('MLX version:', mlx.__version__)"
python3 -c "import mlx_lm; print('MLX-LM installed')"
```

---

## Model Choice for Experiments

We'll use **two** models:

### 1. Qwen 3 8B (Q4) — Our Primary Model
- **Why:** Best overall quality at 8B size, great at reasoning, multilingual, instruction-following
- **Size:** ~5 GB in Q4 quantization
- **HuggingFace:** `mlx-community/Qwen3-8B-4bit`
- **Use for:** Jarvis candidate, all experiments

### 2. Llama 3.2 3B (Q4) — Speed/Comparison Model
- **Why:** Tiny, fast, good for quick iteration. Compare against 8B to see quality difference
- **Size:** ~2 GB in Q4 quantization
- **HuggingFace:** `mlx-community/Llama-3.2-3B-Instruct-4bit`
- **Use for:** Speed comparisons, understanding how size affects quality

---

## Experiment 1: Your First Local LLM (15 min)

**Goal:** See a model running on YOUR machine. No cloud, no API, no internet needed after download.

### 1a. CLI — Fastest way to see it work

```bash
# This downloads ~5GB the first time, then it's cached
mlx_lm.generate \
  --model mlx-community/Qwen3-8B-4bit \
  --prompt "What is Rust programming language in one sentence?" \
  --max-tokens 100
```

**What to observe:**
- First run downloads the model (one-time)
- Generation starts token-by-token — you can SEE the autoregressive loop we built in the React app
- Notice the speed: tokens per second printed at the end

### 1b. Python — Programmatic control

```python
from mlx_lm import load, generate

# Load model + tokenizer (cached after first download)
model, tokenizer = load("mlx-community/Qwen3-8B-4bit")

# Generate
response = generate(
    model,
    tokenizer,
    prompt="What is Rust?",
    max_tokens=100,
    verbose=True  # shows tokens/sec
)
print(response)
```

### 1c. Compare small vs big

```python
# Load the small model
model_small, tok_small = load("mlx-community/Llama-3.2-3B-Instruct-4bit")

# Same prompt, both models
prompt = "Explain quantum computing to a 10 year old in 2 sentences."

print("=== 3B Model ===")
print(generate(model_small, tok_small, prompt=prompt, max_tokens=100))

print("\n=== 8B Model ===")
print(generate(model, tokenizer, prompt=prompt, max_tokens=100))
```

**What to notice:** The 8B model gives more coherent, detailed answers. The 3B is faster but sometimes shallow.

---

## Experiment 2: See the Tokenizer (15 min)

**Goal:** Use a REAL tokenizer (not our mock one) and see how it actually splits text.

```python
from mlx_lm import load

model, tokenizer = load("mlx-community/Qwen3-8B-4bit")

# Tokenize a sentence
text = "Rust is a programming language"
token_ids = tokenizer.encode(text)
print(f"Text: {text}")
print(f"Token IDs: {token_ids}")
print(f"Number of tokens: {len(token_ids)}")

# Decode each token individually to see the splits
for tid in token_ids:
    piece = tokenizer.decode([tid])
    print(f"  ID {tid:>6} → '{piece}'")
```

### 2a. Tokenizer surprises

```python
# Try these and see how they get split differently:
tests = [
    "Hello",           # one token? or two?
    "hello",           # lowercase — same token?
    "Rust",            # capital R
    "rust",            # lowercase r
    "tokenization",    # long word — how many pieces?
    "MLX",             # acronym
    "🔥",              # emoji
    " Rust",           # leading space — watch this!
    "Rust is a programming language",  # our standard example
    "Rust on the iron door",           # same "Rust", different context — same token?
]

for t in tests:
    ids = tokenizer.encode(t)
    pieces = [tokenizer.decode([i]) for i in ids]
    print(f"'{t}' → {len(ids)} tokens: {pieces}")
```

**Key insight:** Tokenizer doesn't know meaning. "Rust" (language) and "Rust" (corrosion) get the SAME token ID. It's just text matching. Context comes later, from attention.

### 2b. Vocabulary size

```python
print(f"Vocabulary size: {tokenizer.vocab_size}")
# Qwen3 has ~151,000+ tokens

# Look at some random vocabulary entries
for i in [0, 1, 100, 1000, 10000, 50000, 100000]:
    print(f"  Token {i}: '{tokenizer.decode([i])}'")
```

---

## Experiment 3: See the Embeddings (20 min)

**Goal:** Look at REAL embedding vectors. See that they're just arrays of numbers — exactly like our React visualization, but 4,096 numbers instead of 8.

```python
import mlx.core as mx
from mlx_lm import load

model, tokenizer = load("mlx-community/Qwen3-8B-4bit")

# Get the embedding table (the ACTUAL weight matrix)
embed_table = model.model.embed_tokens

# Check its shape
print(f"Embedding table shape: {embed_table.weight.shape}")
# Expected: (vocab_size, 4096) — one row of 4096 numbers per token

# Tokenize and embed
text = "Rust is a"
token_ids = tokenizer.encode(text)
print(f"\nToken IDs for '{text}': {token_ids}")

# Look up embeddings (this is the table lookup we visualized!)
input_ids = mx.array([token_ids])
embeddings = embed_table(input_ids)
print(f"Embeddings shape: {embeddings.shape}")
# Expected: (1, num_tokens, 4096)

# Peek at the first token's embedding vector
first_embedding = embeddings[0, 0, :10]  # just first 10 of 4096 numbers
print(f"\nFirst 10 values of 'Rust' embedding: {first_embedding}")
```

### 3a. Same word = same embedding (before attention)

```python
# "Rust" in two different contexts
text1 = "Rust is a programming language"
text2 = "Rust on the iron door"

ids1 = tokenizer.encode(text1)
ids2 = tokenizer.encode(text2)

# Find "Rust" token ID in each
print(f"Text 1 first token ID: {ids1[0]}")
print(f"Text 2 first token ID: {ids2[0]}")
# They should be IDENTICAL — same word, same embedding, same ID

# Get embeddings
emb1 = embed_table(mx.array([ids1]))[0, 0, :]
emb2 = embed_table(mx.array([ids2]))[0, 0, :]

# Compare
diff = mx.sum(mx.abs(emb1 - emb2)).item()
print(f"\nDifference between embeddings: {diff}")
# Should be 0.0 — embeddings don't know context yet!
# Context comes from attention layers AFTER embedding
```

**Key insight:** This proves what we learned — embeddings are just a lookup table. "Rust" always maps to the same 4,096 numbers regardless of meaning. The 32 layers of attention + transforms create the contextual understanding.

---

## Experiment 4: See the Weights (20 min)

**Goal:** Open the model file and look at the actual weight groups we visualized in "Inside the File."

```python
from mlx_lm import load

model, tokenizer = load("mlx-community/Qwen3-8B-4bit")

# List ALL weight groups in the model
print("=== Model Architecture ===\n")
total_params = 0
for name, param in model.model.named_parameters():
    size = param.size
    total_params += size
    # Only print a sample (there are hundreds)
    if "layer" not in name or "layers.0." in name or "layers.31." in name:
        print(f"{name:60s} shape={str(param.shape):20s} params={size:>12,}")

print(f"\nTotal parameters: {total_params:,}")
```

### 4a. The weight groups we learned about

```python
# 1. EMBEDDING TABLE
embed = model.model.embed_tokens.weight
print(f"Embedding Table: {embed.shape}")
# (vocab_size, 4096)

# 2. FIRST ATTENTION LAYER (Layer 0)
layer0 = model.model.layers[0]

# Query, Key, Value weight matrices — the attention mechanism!
print(f"\nLayer 0 Attention:")
print(f"  Q weights: {layer0.self_attn.q_proj.weight.shape}")
print(f"  K weights: {layer0.self_attn.k_proj.weight.shape}")
print(f"  V weights: {layer0.self_attn.v_proj.weight.shape}")
print(f"  Output:    {layer0.self_attn.o_proj.weight.shape}")

# Feed-forward (transform) weights
print(f"\nLayer 0 Feed-Forward (Transform):")
print(f"  Gate:  {layer0.mlp.gate_proj.weight.shape}")
print(f"  Up:    {layer0.mlp.up_proj.weight.shape}")
print(f"  Down:  {layer0.mlp.down_proj.weight.shape}")

# 3. HOW MANY LAYERS?
print(f"\nTotal layers: {len(model.model.layers)}")

# 4. PREDICTION HEAD (lm_head)
print(f"\nPrediction head: {model.lm_head.weight.shape}")
# (vocab_size, 4096) — maps final hidden state to vocabulary scores
```

### 4b. Count params by group

```python
def count_params(module):
    return sum(p.size for p in module.parameters())

embed_params = model.model.embed_tokens.weight.size
layer_params = count_params(model.model.layers[0])
total_layer_params = sum(count_params(l) for l in model.model.layers)
head_params = model.lm_head.weight.size

print(f"Embedding Table:  {embed_params:>12,} params")
print(f"Per Layer:        {layer_params:>12,} params")
print(f"All 32 Layers:    {total_layer_params:>12,} params")
print(f"Prediction Head:  {head_params:>12,} params")
print(f"{'─'*45}")
total = embed_params + total_layer_params + head_params
print(f"Total:            {total:>12,} params")
print(f"\nThis is why it's called an '8B' model!")
```

---

## Experiment 5: Watch Attention in Action (25 min)

**Goal:** Run text through the model and see how attention scores look — which words attend to which.

```python
import mlx.core as mx
from mlx_lm import load

model, tokenizer = load("mlx-community/Qwen3-8B-4bit")

text = "Rust is a programming language"
token_ids = tokenizer.encode(text)
tokens = [tokenizer.decode([t]) for t in token_ids]
print(f"Tokens: {tokens}")

# Run through embedding
input_ids = mx.array([token_ids])
hidden = model.model.embed_tokens(input_ids)
print(f"After embedding: {hidden.shape}")

# Run through just the first layer and capture attention
layer0 = model.model.layers[0]

# We need to create a causal mask for attention
seq_len = len(token_ids)
# The mask prevents tokens from looking at future tokens
mask = mx.triu(mx.full((seq_len, seq_len), float('-inf')), k=1)

# Apply RMSNorm (pre-normalization)
normed = layer0.input_layernorm(hidden)

# Get Q, K, V
q = layer0.self_attn.q_proj(normed)
k = layer0.self_attn.k_proj(normed)
v = layer0.self_attn.v_proj(normed)

print(f"\nQuery shape:  {q.shape}")
print(f"Key shape:    {k.shape}")
print(f"Value shape:  {v.shape}")
print("\nThese are the Q, K, V matrices from our attention visualization!")
print("Q = 'what am I looking for?'")
print("K = 'what do I contain?'")
print("V = 'what information do I share?'")
```

---

## Experiment 6: See Prediction Probabilities (20 min)

**Goal:** Give the model a partial sentence and see the probability distribution over next tokens — exactly like our PredictionPanel.

```python
import mlx.core as mx
from mlx_lm import load

model, tokenizer = load("mlx-community/Qwen3-8B-4bit")

# Partial sentence — what comes next?
prompt = "Rust is a programming"

token_ids = tokenizer.encode(prompt)
input_ids = mx.array([token_ids])

# Forward pass through entire model
logits = model(input_ids)
# logits shape: (1, seq_len, vocab_size)

# Get logits for the LAST token position (that's where the prediction is)
last_logits = logits[0, -1, :]  # shape: (vocab_size,)
print(f"Logits shape: {last_logits.shape}")
print(f"These are raw scores for every word in the vocabulary")

# Convert to probabilities with softmax
probs = mx.softmax(last_logits)

# Top 10 predictions
top_indices = mx.argsort(probs)[-10:][::-1]
print(f"\nTop 10 next-token predictions for '{prompt} ___':\n")
for idx in top_indices.tolist():
    token_text = tokenizer.decode([idx])
    prob = probs[idx].item()
    bar = "█" * int(prob * 50)
    print(f"  {prob:6.2%} '{token_text}' {bar}")
```

### 6a. Compare predictions for ambiguous words

```python
prompts = [
    "Rust is a programming",    # → "language" (high confidence)
    "Rust on the iron",         # → "door" / "gate" / "surface"
    "The capital of France is", # → "Paris" (very high confidence)
    "I love eating",            # → many foods (low confidence per token)
]

for prompt in prompts:
    ids = tokenizer.encode(prompt)
    logits = model(mx.array([ids]))
    probs = mx.softmax(logits[0, -1, :])
    top5 = mx.argsort(probs)[-5:][::-1]

    print(f"\n'{prompt} ___'")
    for idx in top5.tolist():
        p = probs[idx].item()
        t = tokenizer.decode([idx])
        print(f"  {p:6.2%} → '{t}'")
```

**Key insight:** When the model is confident, one token dominates (90%+). When uncertain, probability spreads across many tokens. Temperature / top-p sampling controls how we pick from this distribution.

---

## Experiment 7: Jarvis Model Selection (30 min)

**Goal:** Test models specifically for Jarvis use cases (desktop assistant tasks).

```python
from mlx_lm import load, generate

# Test prompts that Jarvis would handle
jarvis_prompts = [
    "Summarize this YouTube video transcript in 3 bullet points: The video explains how transformers work by showing that attention is the key mechanism...",
    "Extract the key takeaways from this Medium article about Rust async programming...",
    "Write a short reply to this email: 'Hey, can we reschedule our meeting from Tuesday to Thursday?'",
    "What are the main topics discussed in this ChatGPT conversation about machine learning?",
]

# Test with Qwen 8B
model, tokenizer = load("mlx-community/Qwen3-8B-4bit")

for prompt in jarvis_prompts:
    print(f"\n{'='*60}")
    print(f"PROMPT: {prompt[:80]}...")
    print(f"{'='*60}")
    response = generate(model, tokenizer, prompt=prompt, max_tokens=150, verbose=True)
    print(response)
```

### 7a. Measure speed

```python
import time

prompt = "Summarize the key points of this article about Rust programming language benefits."

start = time.time()
response = generate(model, tokenizer, prompt=prompt, max_tokens=200, verbose=True)
elapsed = time.time() - start

print(f"\nTotal time: {elapsed:.1f}s")
# On M3 Pro: expect ~30-50 tokens/sec for 8B Q4
# That's fast enough for Jarvis — response in 2-4 seconds
```

---

## Experiment 8: Quantization — See the Difference (15 min)

**Goal:** Understand quantization by comparing Q4 vs higher precision.

```python
from mlx_lm import load
import mlx.core as mx

# Load Q4 model (what we've been using)
model_q4, _ = load("mlx-community/Qwen3-8B-4bit")

# Look at a weight matrix
w = model_q4.model.layers[0].self_attn.q_proj.weight
print(f"Q4 weight dtype: {w.dtype}")
print(f"Q4 weight shape: {w.shape}")
print(f"Sample values: {w[:3, :5]}")
# Notice: values are quantized (fewer unique values)

# Memory comparison
import subprocess
result = subprocess.run(['ps', '-o', 'rss=', '-p', str(__import__('os').getpid())],
                       capture_output=True, text=True)
mem_mb = int(result.stdout.strip()) / 1024
print(f"\nCurrent process memory: {mem_mb:.0f} MB")
```

---

## Recommended Order

| # | Experiment | Time | What You Learn |
|---|-----------|------|----------------|
| 0 | Setup | 5 min | Install MLX ecosystem |
| 1 | First LLM | 15 min | Model running locally, tokens/sec |
| 2 | Tokenizer | 15 min | Real BPE tokenizer, vocab, surprises |
| 3 | Embeddings | 20 min | Real 4096-dim vectors, table lookup |
| 4 | Weights | 20 min | Model architecture, parameter counts |
| 5 | Attention | 25 min | Q/K/V matrices, attention mechanism |
| 6 | Predictions | 20 min | Probability distributions, confidence |
| 7 | Jarvis Model | 30 min | Speed testing, use-case fitness |
| 8 | Quantization | 15 min | Q4 vs full precision, memory |

**Total: ~2.5 hours** (do over multiple sessions)

---

## Model Recommendation for Jarvis

**Primary: `mlx-community/Qwen3-8B-4bit`**
- ~5 GB RAM for weights
- ~2 GB for KV cache (2k context)
- Total: ~7 GB — plenty of room on 36 GB
- Great at summarization, extraction, instruction following
- 30-50 tokens/sec on M3 Pro

**Fallback for speed: `mlx-community/Llama-3.2-3B-Instruct-4bit`**
- ~2 GB RAM
- 60-80 tokens/sec
- Good for simple tasks (email replies, quick summaries)

**Stretch goal: `mlx-community/Qwen3-14B-4bit`**
- ~9 GB RAM — fits on 36 GB but tight with large context
- Best quality, noticeably better at nuanced tasks
- 15-25 tokens/sec

---

## What Connects Back to Our React App

| React App Concept | MLX Experiment |
|-------------------|---------------|
| TokenizerPanel (mock) | Experiment 2 — real BPE tokenizer |
| EmbeddingPanel (8 fake dims) | Experiment 3 — real 4,096 dimensions |
| "Inside the File" weight groups | Experiment 4 — actual model.layers, embed_tokens, lm_head |
| AttentionPanel (mock scores) | Experiment 5 — real Q/K/V matrices |
| PredictionPanel (fake top-5) | Experiment 6 — real probability distribution |
| AutoregressiveDemo | Experiment 1 — watch real token-by-token generation |
| KV Cache Demo (theoretical) | Experiment 1 — see actual tokens/sec (cache is automatic in MLX) |

---

## Resources

- [MLX GitHub](https://github.com/ml-explore/mlx)
- [MLX-LM GitHub](https://github.com/ml-explore/mlx-lm)
- [MLX Community Models (HuggingFace)](https://huggingface.co/mlx-community)
- [MLX Documentation](https://ml-explore.github.io/mlx/)
- [WWDC25: Run LLMs Locally with MLX](https://developer.apple.com/videos/play/wwdc2025/298/)
