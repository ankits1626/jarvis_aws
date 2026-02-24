# JarvisIntel: Local Intelligence for Jarvis

## The Big Picture

Jarvis already captures knowledge — browser articles, YouTube transcripts, audio recordings.
But a pile of raw text isn't useful unless you can *understand* what's in it.

JarvisIntel adds a brain. It reads each gem and extracts meaning:
tags, summaries, topics — so you can glance at a gem card and know what's inside
without reading 2000 words of transcript.

The key constraint: **everything runs on-device, data never leaves your Mac.**

---

## Why Apple Foundation Models (and not something else)

| Option | Size | Quality | Cost | Privacy | Offline |
|--------|------|---------|------|---------|---------|
| Claude API | 0 | Best | $/call | Cloud | No |
| Bundled LLM (Phi-3, 3.8B) | ~2GB download | Good | Free | Local | Yes |
| keyword-extraction-rs (TF-IDF/RAKE) | 0 | Basic | Free | Local | Yes |
| **Apple Foundation Models** | **0 (built into OS)** | **Good** | **Free** | **Local** | **Yes** |

Apple's model wins because:
1. **Zero download** — the 3B model ships with macOS 26. No model management.
2. **Hardware optimized** — 2-bit quantization, KV-cache sharing, Apple Neural Engine.
   Same silicon advantage you already use with WhisperKit.
3. **Structured output guaranteed** — guided generation means "give me tags as `[String]`"
   and the model *cannot* produce anything else. No parsing, no retries.
4. **Free** — no API key, no per-call cost, no rate limits (foreground).

The tradeoff: it's only available on macOS 26+ with Apple Silicon (M1+).
For Jarvis, that's fine — you already target macOS with Apple Silicon for WhisperKit.

---

## The Core Technologies (Building Intuition)

### 1. The On-Device Model (~3B Parameters)

**What it is:**
A ~3 billion parameter language model, baked into the operating system.
Think of it as a small but capable brain that lives on your Mac's Neural Engine.

**How big is 3B?**
- GPT-4: ~1.8 trillion parameters (600x larger)
- Claude Sonnet: unknown but much larger
- Llama 3 8B: 2.7x larger
- Apple's model: optimized to punch above its weight on *specific tasks*

**What it's good at (sweet spot for Jarvis):**
- Summarization — "condense this 2000-word article into 2 sentences"
- Entity extraction — "what topics are discussed here?"
- Classification — "is this about AI, cooking, or finance?"
- Instruction following — "extract exactly 5 tags as a JSON array"

**What it's NOT good at:**
- General world knowledge (don't ask it "what's the capital of France")
- Complex reasoning chains
- Creative writing at GPT-4 level
- Being a general chatbot

**Mental model:** Think of it as a specialist, not a generalist.
It's like a very fast librarian who can tag and categorize books
but can't write a novel.

### 2. Guided Generation (Constrained Decoding)

This is the killer feature for Jarvis. Here's the intuition:

**The problem with normal LLMs:**
You ask: "Give me 5 tags for this article"
LLM responds: "Sure! Here are some tags: 1. AI 2. Machine Learning..."
Now you have to *parse* that text. What if it says "Here are the tags:" first?
What if it uses bullets instead of numbers? What if it adds explanations?

**How guided generation solves it:**

```swift
@Generable
struct TagResult {
    @Guide(description: "3-5 topic tags, each 1-3 words")
    var tags: [String]
}
```

When you ask the model to generate `TagResult`, three things happen:

1. **Compile time**: The Swift `@Generable` macro converts your struct into a
   JSON schema specification. This happens at build time, not runtime.

2. **Prompt injection**: The framework automatically adds the schema to the prompt,
   telling the model "your response must conform to this structure."

3. **Token masking**: During inference, the OS-level decoder *masks out invalid tokens*.
   If the schema expects a `[`, the model literally cannot produce `{` or text.
   It's not "hoping" the model follows instructions — it's *mechanically impossible*
   for the model to produce invalid output.

**Result:** `response.tags` is always `[String]`. Always. No parsing. No retries.
Type-safe at the language level.

**Why this matters for Jarvis:**
- `generate_tags` returns `[String]` — guaranteed
- `generate_summary` returns `String` — guaranteed
- `classify_source` returns an enum value — guaranteed
- Zero error handling for malformed responses needed

### 3. LanguageModelSession (The Conversation Manager)

**What it is:**
A stateful object that manages a conversation with the model.

```swift
let session = LanguageModelSession()

// First call
let tags = try await session.respond(to: "Tag this: ...", generating: TagResult.self)

// Second call — session remembers the first
let refined = try await session.respond(to: "Make them more specific")
```

**Key properties:**

| Property | Value | Implication for Jarvis |
|----------|-------|----------------------|
| Context window | 4,096 tokens | ~3000 words input+output combined |
| Statefulness | Multi-turn within session | Can refine results conversationally |
| Concurrency | Thread-safe | Can process multiple gems in parallel |
| Instructions | System prompt per session | Set once: "You are a content tagger" |

**4,096 tokens — is that enough?**

For tagging and summarization, yes:
- Instructions: ~100 tokens
- Gem content (truncated): ~3,500 tokens (~2,600 words)
- Output (5 tags): ~50 tokens

For long transcripts (10+ minute recordings), you'd truncate to first ~2,500 words.
The model sees enough to extract topics even from partial content.

**Mental model:** Think of a session as a notepad.
The model can see everything written on the notepad (the transcript).
The notepad has a fixed size (4096 tokens).
Once full, you need a new notepad (new session).

### 4. Tool Calling (Future: RAG over Gems)

Not needed for tagging, but powerful for a future "Ask Jarvis" feature:

```swift
struct SearchGemsTool: Tool {
    let description = "Search the user's saved gems by keyword"

    @Generable
    struct Arguments {
        @Guide(description: "Search query")
        var query: String
    }

    func call(arguments: Arguments) async throws -> String {
        // Call your SQLite FTS5 search
        let results = searchGems(query: arguments.query)
        return formatResults(results)
    }
}
```

The model can *autonomously decide* to search your gems when answering a question.
You: "What did I save about AI agents?"
Model: *calls SearchGemsTool("AI agents")* → reads results → answers with context.

This turns Jarvis into a personal knowledge assistant. But that's Phase 2.

### 5. Guardrails (Content Safety)

**What they are:**
Built-in filters that block harmful content in both input and output.
You cannot disable them.

**What gets blocked:**
- Self-harm, violence, adult content
- Prompt injection attempts (70.4% blocked in testing)

**What does NOT get blocked (relevant for Jarvis):**
- Technical content, code snippets
- News articles, podcast transcripts
- Normal educational/informational content

**Known issue:** Some developers report false positives — the guardrails
occasionally block innocent content (e.g., book summaries). Apple is
actively fixing this. For Jarvis's use case (tagging/summarizing
technical and general content), this should rarely trigger.

**Fallback strategy:** If guardrails block a gem's content,
fall back to `keyword-extraction-rs` (TF-IDF) for that gem.

---

## Architecture: How JarvisIntel Fits

```
┌──────────────────────────────────────────────────────────┐
│                    Jarvis App (Tauri)                     │
│                                                          │
│  ┌──────────┐  ┌──────────┐  ┌───────────────┐          │
│  │ GemsPanel│  │BrowserTool│  │TranscriptDisp.│          │
│  └────┬─────┘  └────┬─────┘  └──────┬────────┘          │
│       └──────────────┼───────────────┘                   │
│                      │                                   │
│              ┌───────▼────────┐                          │
│              │  Rust Backend  │                          │
│              │   (commands)   │                          │
│              └───────┬────────┘                          │
│                      │                                   │
│    ┌─────────────────┼──────────────────────┐            │
│    │                 │                      │            │
│  ┌─▼───────┐  ┌─────▼──────┐  ┌────────────▼─────────┐  │
│  │ SQLite  │  │JarvisListen│  │  Arc<dyn IntelProvider>│  │
│  │(gems.db)│  │ (sidecar)  │  │  ┌─────────────────┐  │  │
│  └─────────┘  └────────────┘  │  │ Provider impl:  │  │  │
│                               │  │                 │  │  │
│                               │  │ Foundation ─────┼──┤  │
│                               │  │   Models        │  │  │
│                               │  │ (JarvisIntel    │  │  │
│                               │  │  sidecar)       │  │  │
│                               │  │       │         │  │  │
│                               │  │       ▼         │  │  │
│                               │  │  Apple 3B LLM   │  │  │
│                               │  │  (on-device)    │  │  │
│                               │  │                 │  │  │
│                               │  │ ─── OR ───      │  │  │
│                               │  │                 │  │  │
│                               │  │ Keyword ────────┤  │  │
│                               │  │  (TF-IDF/RAKE)  │  │  │
│                               │  │  pure Rust      │  │  │
│                               │  │                 │  │  │
│                               │  │ ─── OR ───      │  │  │
│                               │  │                 │  │  │
│                               │  │ Claude API ─────┼──┼──┼─→ api.anthropic.com
│                               │  │  (reqwest)      │  │  │
│                               │  └─────────────────┘  │  │
│                               └───────────────────────┘  │
└──────────────────────────────────────────────────────────┘
```

The key: **commands talk to `Arc<dyn IntelProvider>`** — they don't know
which implementation is behind the trait. Swap providers via settings,
no code changes in commands or frontend.

### Data Flow: Gem Save → Auto-Tag

```
1. User clicks "Save Gem" (BrowserTool or TranscriptDisplay)
   │
2. invoke('save_gem', { gist }) → Rust saves to SQLite
   │
3. Rust spawns JarvisIntel sidecar:
   │   JarvisIntel --generate-tags
   │   stdin ← { content: "...", title: "..." }
   │
4. JarvisIntel (Swift):
   │   let session = LanguageModelSession(instructions: "...")
   │   let result = try await session.respond(to: prompt, generating: TagResult.self)
   │   stdout → { "tags": ["AI agents", "productivity", "automation"] }
   │
5. Rust reads stdout, updates gem in SQLite:
   │   UPDATE gems SET tags = '["AI agents","productivity","automation"]' WHERE id = ?
   │
6. Frontend refreshes gem card → tags appear as badges
```

### Communication Protocol: JSON over stdio

Same pattern as how many CLI tools work. Simple, no sockets, no HTTP.

```
Request (stdin):
{
  "command": "generate_tags",
  "content": "The article discusses how AI agents are...",
  "title": "The Rise of AI Agents",
  "source_type": "Article"
}

Response (stdout):
{
  "ok": true,
  "tags": ["AI agents", "autonomous systems", "productivity"]
}

Error (stdout):
{
  "ok": false,
  "error": "guardrail_blocked",
  "fallback_tags": ["general"]
}
```

---

## The Provider Abstraction (Swappable Intelligence)

This is the most important design decision. The intelligence layer must be
**provider-agnostic** — same pattern Jarvis already uses for transcription:

```
TranscriptionProvider trait
├── WhisperKitProvider  (Apple Neural Engine, local)
├── HybridProvider      (whisper-rs + Vosk, local)
└── (future) CloudProvider (API-based)
```

JarvisIntel follows the exact same pattern:

```
IntelProvider trait
├── FoundationModelProvider  (Apple on-device 3B, local)
├── KeywordProvider          (TF-IDF/RAKE, local, no model)
├── ClaudeProvider           (Anthropic API, cloud)
└── (future) OllamaProvider  (local LLM server)
```

### The Trait

```rust
use async_trait::async_trait;

/// Result of intelligence operations on a gem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelResult {
    pub tags: Vec<String>,
    pub summary: Option<String>,
}

/// Provider-agnostic intelligence interface.
/// Implementations can be local (Foundation Models, TF-IDF) or remote (Claude API).
#[async_trait]
pub trait IntelProvider: Send + Sync {
    /// Human-readable name (e.g., "apple-foundation-models", "claude-api", "tfidf")
    fn name(&self) -> &str;

    /// Check if this provider is available on the current system
    fn is_available(&self) -> bool;

    /// Why this provider is unavailable (None if available)
    fn unavailable_reason(&self) -> Option<&str>;

    /// Generate tags for the given content
    async fn generate_tags(
        &self,
        title: &str,
        content: &str,
        source_type: &str,
    ) -> Result<Vec<String>, String>;

    /// Generate a short summary (optional — not all providers support this)
    async fn generate_summary(
        &self,
        title: &str,
        content: &str,
        source_type: &str,
    ) -> Result<Option<String>, String> {
        // Default: no summary support
        Ok(None)
    }
}
```

### Why This Shape

**Mirrors `TranscriptionProvider`:**
- `name()` — identifies the provider in logs/settings
- `is_available()` — runtime check (is Foundation Models on this Mac? is API key set?)
- `generate_tags()` — the core operation, like `transcribe()` for audio

**Mirrors `GemStore`:**
- `async_trait` — providers may do I/O (spawn sidecar, call API)
- `Send + Sync` — safe to share across async tasks
- `Result<T, String>` — consistent error type with the rest of Jarvis

### Provider Implementations

#### 1. FoundationModelProvider (Primary — Local)

```rust
pub struct FoundationModelProvider {
    app_handle: AppHandle,
    available: bool,
}

#[async_trait]
impl IntelProvider for FoundationModelProvider {
    fn name(&self) -> &str { "apple-foundation-models" }
    fn is_available(&self) -> bool { self.available }

    async fn generate_tags(&self, title: &str, content: &str, source_type: &str)
        -> Result<Vec<String>, String>
    {
        // Spawns JarvisIntel sidecar (Swift binary)
        // Sends JSON over stdin, reads JSON from stdout
        // Returns structured tags from guided generation
    }

    async fn generate_summary(&self, title: &str, content: &str, source_type: &str)
        -> Result<Option<String>, String>
    {
        // Same sidecar, different command flag
        // Returns 1-2 sentence summary
    }
}
```

**Communicates with:** `JarvisIntel` Swift sidecar → Apple Foundation Models framework
**Availability:** macOS 26+, Apple Silicon, Apple Intelligence enabled

#### 2. KeywordProvider (Fallback — Local, No Model)

```rust
pub struct KeywordProvider;

#[async_trait]
impl IntelProvider for KeywordProvider {
    fn name(&self) -> &str { "keyword-extraction" }
    fn is_available(&self) -> bool { true }  // Always available

    async fn generate_tags(&self, _title: &str, content: &str, _source_type: &str)
        -> Result<Vec<String>, String>
    {
        // Uses keyword-extraction-rs crate
        // YAKE or TF-IDF algorithm — pure Rust, no model, instant
        let keywords = yake::extract(content, 5);
        Ok(keywords)
    }
    // No generate_summary — returns Ok(None) via default impl
}
```

**Communicates with:** Nothing external. Pure Rust computation.
**Availability:** Always. Every platform. Zero dependencies.

#### 3. ClaudeProvider (Future — Cloud API)

```rust
pub struct ClaudeProvider {
    api_key: Option<String>,
    client: reqwest::Client,
}

#[async_trait]
impl IntelProvider for ClaudeProvider {
    fn name(&self) -> &str { "claude-api" }
    fn is_available(&self) -> bool { self.api_key.is_some() }
    fn unavailable_reason(&self) -> Option<&str> {
        if self.api_key.is_none() { Some("No API key configured") } else { None }
    }

    async fn generate_tags(&self, title: &str, content: &str, source_type: &str)
        -> Result<Vec<String>, String>
    {
        // POST to https://api.anthropic.com/v1/messages
        // Uses tool_use for structured output
        // Highest quality but requires API key + costs money
    }

    async fn generate_summary(&self, title: &str, content: &str, source_type: &str)
        -> Result<Option<String>, String>
    {
        // Same API, different prompt
        // Claude produces the best summaries
        Ok(Some(summary))
    }
}
```

**Communicates with:** Anthropic API over HTTPS
**Availability:** When API key is configured in settings

### Provider Selection (in lib.rs setup)

Same pattern as transcription engine selection:

```rust
// In settings
pub struct IntelSettings {
    pub intel_engine: String,  // "foundation-models" | "keyword" | "claude-api"
    pub claude_api_key: Option<String>,
}

// In lib.rs setup
let provider: Arc<dyn IntelProvider> = match settings.intel.intel_engine.as_str() {
    "foundation-models" => {
        let fm = FoundationModelProvider::new(app.handle().clone());
        if fm.is_available() {
            Arc::new(fm)
        } else {
            eprintln!("Foundation Models unavailable: {}", fm.unavailable_reason().unwrap_or("unknown"));
            eprintln!("Falling back to keyword extraction");
            Arc::new(KeywordProvider)
        }
    }
    "claude-api" => {
        let claude = ClaudeProvider::new(settings.intel.claude_api_key.clone());
        if claude.is_available() {
            Arc::new(claude)
        } else {
            eprintln!("Claude API unavailable: no API key. Falling back to keyword extraction");
            Arc::new(KeywordProvider)
        }
    }
    _ => Arc::new(KeywordProvider),
};

app.manage(provider);
```

### Settings UI Extension

```
Intel Engine: [Foundation Models (Local)] ▾
                Foundation Models (Local)     ← macOS 26+, free, private
                Keyword Extraction (Local)    ← any OS, basic quality
                Claude API (Cloud)            ← highest quality, requires key
                ─────────────────────────
                API Key: [sk-ant-...]         ← only shown for Claude
```

### The Key Insight

Every command in the Rust backend uses `State<'_, Arc<dyn IntelProvider>>`:

```rust
#[tauri::command]
async fn generate_tags(
    gem_id: String,
    gem_store: State<'_, Arc<dyn GemStore>>,
    intel: State<'_, Arc<dyn IntelProvider>>,  // ← provider-agnostic
) -> Result<Vec<String>, String> {
    let gem = gem_store.get(&gem_id).await?.ok_or("Gem not found")?;
    let tags = intel.generate_tags(
        &gem.title,
        gem.content.as_deref().unwrap_or(""),
        &gem.source_type,
    ).await?;
    // Update gem with tags
    gem_store.update_tags(&gem_id, &tags).await?;
    Ok(tags)
}
```

This command works identically whether the provider is Foundation Models,
TF-IDF, or Claude API. The frontend doesn't know or care which is active.

---

## Design Decisions

### Decision 1: Sidecar vs FFI (libai)

**Sidecar (chosen):**
- Same pattern as JarvisListen — known, working, tested
- Process isolation — if the model crashes, Jarvis continues
- No complex build system changes (no C FFI, no bridging headers)
- Easy to test independently: `echo '{}' | JarvisIntel --generate-tags`

**FFI via libai (rejected for now):**
- Lower latency (no process spawn)
- But: adds build complexity, C bindings, less isolation
- Can migrate to FFI later if sidecar latency is an issue

### Decision 2: When to Generate Tags

**Option A: Synchronous (on save)** — User waits for tags before gem is saved.
  - Bad UX. Model takes 1-3 seconds. User sees spinner on "Save Gem".

**Option B: Async (after save)** — Gem saved immediately, tags appear later.
  - Good UX. "Save Gem" is instant. Tags fade in when ready.
  - Frontend polls or listens for a Tauri event: `gem-tags-updated`.

**Chosen: Option B.** Save is instant. Tagging happens in background.

### Decision 3: Tags Storage

**Option A: Separate `tags` table (normalized)**
```sql
CREATE TABLE gem_tags (gem_id TEXT, tag TEXT);
```
- Proper relational design. Enables "show all gems with tag X".
- More complex queries for listing gems with their tags.

**Option B: JSON array column on gems table**
```sql
ALTER TABLE gems ADD COLUMN tags TEXT DEFAULT '[]';
-- Store as: '["AI agents","productivity"]'
```
- Simple. One query to get gem + tags.
- Can still search: `WHERE tags LIKE '%AI agents%'`
- Include in FTS5 for full-text search.

**Chosen: Option B.** Simpler. FTS5 handles searchability.
Can normalize later if filtering-by-tag becomes important.

### Decision 4: Context Window Management

The 4096 token limit means long content must be truncated.

**Strategy:**
- Instructions (system prompt): ~150 tokens (fixed)
- Title + source_type: ~20 tokens
- Content: first 3,500 tokens (~2,600 words)
- Output budget: ~100 tokens (enough for 5 tags)

For long transcripts, 2,600 words covers ~13 minutes of speech
(at ~200 words/minute). That's enough to extract topics.

### Decision 5: Fallback for Unsupported Systems

If Foundation Models is unavailable (older macOS, Intel Mac):
1. Check availability at startup: `SystemLanguageModel.default.isAvailable`
2. If unavailable: use `keyword-extraction-rs` (TF-IDF/RAKE) as fallback
3. Tags will be lower quality but still useful

---

## The Swift Sidecar: JarvisIntel

### Package Structure

```
jarvis-intel/
├── Package.swift                       # Swift 6.2, macOS 26+
├── Sources/JarvisIntel/
│   ├── main.swift                      # CLI entry point, argument parsing
│   ├── TagGenerator.swift              # @Generable types + tag generation
│   ├── Summarizer.swift                # Summary generation (Phase 2)
│   └── ModelAvailability.swift         # Check if Foundation Models available
└── Tests/JarvisIntelTests/
    └── TagGeneratorTests.swift
```

### Core Swift Code (Conceptual)

```swift
import FoundationModels

// === Structured output types ===

@Generable
struct TagResult {
    @Guide(description: "3 to 5 topic tags. Each tag is 1-3 words. Lowercase.")
    var tags: [String]
}

@Generable
struct SummaryResult {
    @Guide(description: "A 1-2 sentence summary of the content")
    var summary: String

    @Guide(description: "3 to 5 topic tags, each 1-3 words, lowercase")
    var tags: [String]
}

// === Tag generation ===

func generateTags(title: String, content: String, sourceType: String) async throws -> [String] {
    let session = LanguageModelSession(
        instructions: """
        You are a content tagger for a personal knowledge base.
        Extract topic tags from the provided content.
        Tags should be specific and descriptive (e.g., "AI agents" not "technology").
        """
    )

    let prompt = """
    Source type: \(sourceType)
    Title: \(title)
    Content:
    \(content.prefix(10_000))
    """

    let result = try await session.respond(to: prompt, generating: TagResult.self)
    return result.tags
}
```

**Key insight:** The `@Generable` macro + `session.respond(generating:)` is all you need.
No JSON parsing. No regex. No "please format as JSON". The framework handles everything.

### CLI Interface

```swift
// main.swift
@main
struct JarvisIntel {
    static func main() async {
        // 1. Parse arguments (--generate-tags, --summarize, --check-availability)
        // 2. Read JSON from stdin
        // 3. Call appropriate function
        // 4. Write JSON to stdout
        // 5. Handle signals (SIGTERM/SIGINT) for graceful shutdown
    }
}
```

### Availability Check

```swift
func checkAvailability() -> Bool {
    let model = SystemLanguageModel.default
    return model.isAvailable
    // Also check: model.isReady (downloaded and ready)
    // vs model.needsDownload (Apple Intelligence not set up)
}
```

This lets the Rust side know whether to use JarvisIntel or fall back to TF-IDF.

---

## Rust Integration

### New Module: `src-tauri/src/intel.rs`

```rust
pub struct IntelManager {
    app_handle: AppHandle,
    available: bool,  // cached availability check
}

impl IntelManager {
    pub fn new(app_handle: AppHandle) -> Self { ... }

    /// Check if JarvisIntel (Foundation Models) is available
    pub async fn check_availability(&self) -> bool { ... }

    /// Generate tags for a gem (spawns sidecar, reads result)
    pub async fn generate_tags(&self, content: &str, title: &str, source_type: &str)
        -> Result<Vec<String>, String> { ... }
}
```

### Spawn Pattern (mirrors recording.rs)

```rust
let sidecar = self.app_handle
    .shell()
    .sidecar("JarvisIntel")
    .args(["--generate-tags"]);

let (rx, child) = sidecar.spawn()?;

// Write request to stdin
let request = serde_json::json!({
    "title": title,
    "content": content,
    "source_type": source_type,
});
child.write(serde_json::to_vec(&request)?)?;

// Read response from stdout
for event in rx {
    match event {
        CommandEvent::Stdout(line) => {
            let response: TagResponse = serde_json::from_str(&line)?;
            return Ok(response.tags);
        }
        CommandEvent::Terminated { .. } => break,
        _ => {}
    }
}
```

### Integration with Gem Save Flow

```rust
// In save_gem command (commands.rs), after saving:
let gem = gem_store.save(gem).await?;

// Spawn background tagging (don't block the save)
let intel = app_handle.state::<IntelManager>();
let gem_id = gem.id.clone();
tauri::async_runtime::spawn(async move {
    if let Ok(tags) = intel.generate_tags(&content, &title, &source_type).await {
        gem_store.update_tags(&gem_id, &tags).await.ok();
        app_handle.emit("gem-tags-updated", &gem_id).ok();
    }
});

Ok(gem) // Return immediately — tags come later
```

---

## Frontend Updates

### GemCard: Display Tags

```tsx
// In GemCard component
{gem.tags && gem.tags.length > 0 && (
  <div className="gem-tags">
    {gem.tags.map(tag => (
      <span key={tag} className="gem-tag">{tag}</span>
    ))}
  </div>
)}
```

### Listen for Tag Updates

```tsx
// In GemsPanel
useTauriEvent('gem-tags-updated', (gemId) => {
  // Refresh the specific gem or the whole list
  fetchGems(searchQuery);
});
```

### CSS: Tag Badges

```css
.gem-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-top: 6px;
}

.gem-tag {
  background: #eef2ff;
  color: #4338ca;
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 12px;
  font-weight: 500;
}
```

---

## DB Schema Changes

```sql
-- Add tags column
ALTER TABLE gems ADD COLUMN tags TEXT DEFAULT '[]';

-- Update FTS5 to include tags (requires rebuild)
DROP TABLE IF EXISTS gems_fts;
CREATE VIRTUAL TABLE gems_fts USING fts5(
    title, author, description, content, tags,
    content='gems', content_rowid='rowid'
);

-- Rebuild triggers to include tags
-- (same pattern as existing triggers, add tags column)
```

---

## Phased Rollout

### Phase 1a: Trait + KeywordProvider (build the skeleton)
- Define `IntelProvider` trait in `src-tauri/src/intel/provider.rs`
- Implement `KeywordProvider` using `keyword-extraction-rs`
- Add `tags TEXT DEFAULT '[]'` column to gems table
- Add `generate_tags` / `get_intel_status` Tauri commands
- Wire up provider selection in `lib.rs` (same as transcription)
- Tag badges on gem cards in frontend
- Tags included in FTS5 search
- **This works on all macOS versions immediately.**

### Phase 1b: FoundationModelProvider (add the Apple brain)
- Build `JarvisIntel` Swift sidecar (Package.swift, guided generation)
- Implement `FoundationModelProvider` (spawns sidecar, JSON stdio)
- Add to `tauri.conf.json` externalBin + capabilities
- Auto-detect availability, fall back to KeywordProvider
- **Now macOS 26 users get high-quality tags, others get keyword tags.**

### Phase 1c: ClaudeProvider (add the cloud option)
- Implement `ClaudeProvider` using `reqwest`
- Add API key field to settings
- Settings UI: intel engine selector dropdown
- **Users can now choose: local (free) or cloud (best quality).**

### Phase 2: Summaries
- Add `generate_summary()` to trait (already stubbed with default impl)
- Add `summary TEXT` column to gems table
- Foundation Models + Claude providers return summaries
- Frontend: show summary instead of truncated content_preview

### Phase 3: "Ask Jarvis" Chat
- New `ChatProvider` trait (long-running, multi-turn, streaming)
- Tool calling: model can search gems via FTS5 autonomously
- Chat panel in the UI (like YouTube/Browser/Gems panels)
- RAG over your personal knowledge base
- Foundation Models (4096 tokens) for quick lookups
- Claude API for deep analysis across many gems

---

## Hardware & OS Requirements

| Requirement | Value |
|-------------|-------|
| macOS version | 26 (Tahoe) or later |
| Chip | Apple Silicon (M1 or later) |
| Apple Intelligence | Must be enabled in Settings |
| Disk space | 0 extra (model ships with OS) |
| Network | Not required (fully offline) |

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| macOS 26 not yet released (ships fall 2025) | Can't test on production OS | Develop on macOS 26 beta / Xcode 26 beta |
| Guardrails block legitimate content | Some gems get no tags | Fall back to TF-IDF for blocked content |
| 4096 token limit too small for long transcripts | Partial content analysis | Truncate to first ~2,600 words (covers most topics) |
| Sidecar spawn latency (~500ms) | Slight delay before tagging starts | Tagging is async — user doesn't wait |
| Model quality lower than Claude/GPT-4 | Tags may be less nuanced | Good enough for topic extraction; can add API option later |

---

## References

- [Apple Foundation Models Documentation](https://developer.apple.com/documentation/FoundationModels)
- [Meet the Foundation Models Framework (WWDC25)](https://developer.apple.com/videos/play/wwdc2025/286/)
- [Deep Dive into Foundation Models (WWDC25)](https://developer.apple.com/videos/play/wwdc2025/301/)
- [Managing the Context Window (TN3193)](https://developer.apple.com/documentation/technotes/tn3193-managing-the-on-device-foundation-model-s-context-window)
- [Foundation Models Tech Report 2025](https://machinelearning.apple.com/research/apple-foundation-models-tech-report-2025)
- [Acceptable Use Requirements](https://developer.apple.com/apple-intelligence/acceptable-use-requirements-for-the-foundation-models-framework/)
- [libai — C bridge for non-Swift access](https://github.com/6over3/libai)
- [Guided Generation Best Practices](https://datawizz.ai/blog/apple-foundations-models-framework-10-best-practices-for-developing-ai-apps)
- [Guardrails Discussion (Developer Forums)](https://developer.apple.com/forums/thread/787736)
- [Token Usage Tracking](https://artemnovichkov.com/blog/tracking-token-usage-in-foundation-models)
