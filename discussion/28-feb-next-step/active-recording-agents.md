# Active Recording Agents — Concept Elaboration (v2)

## The Big Idea

While Jarvis records a meeting/call/lecture, an **always-on AI agent** produces real-time intelligence — not after the fact, but **while you're still in the conversation**. This turns Jarvis from a passive recorder into an **active co-pilot**.

**Key insight**: Live transcripts (Whisper/WhisperKit) are noisy and low-quality. Instead of feeding the agent text transcripts, we **stream raw audio chunks directly to Qwen Omni** — a multimodal model that can understand audio natively. Qwen Omni produces its own high-quality understanding of what was said, bypassing transcript quality issues entirely.

---

## Architecture: Audio-First, Not Transcript-First

### Why Audio Chunks Instead of Live Transcript

| Approach | Quality | Latency | Complexity |
|---|---|---|---|
| Feed live transcript text to LLM | Low — Whisper real-time is choppy, misses words, no punctuation | Low | Simple |
| **Feed audio chunks to Qwen Omni** | **High — model hears the actual audio, understands tone/context** | Medium (chunk interval) | Medium |

Live transcripts are optimized for speed, not accuracy. Words get dropped, sentences fragment, technical terms get mangled. Qwen Omni as a multimodal model can process audio directly and produce a much richer understanding — including tone, emphasis, and context that text transcripts lose.

### The Processing Pipeline

```
Microphone → PCM Recording (continuous)
                │
                ├──→ Live Transcript (Whisper/WhisperKit)
                │       └──→ [Transcript Tab] — real-time, low-quality, for quick glance
                │
                └──→ Audio Chunk Buffer (every 60-90s)
                        │
                        ▼
                ┌───────────────────────┐
                │  Agent Cycle          │
                │                       │
                │  Input:               │
                │  • Audio chunk (WAV)  │
                │  • Running context    │
                │    (aggregated text)  │
                │                       │
                │  Model: Qwen Omni     │
                │  (MLX, local)         │
                │                       │
                │  Output:              │
                │  • What was discussed │
                │  • Updated summary    │
                │  • Questions          │
                │  • Concepts           │
                └───────────────────────┘
                        │
                        ▼
                ┌───────────────────────┐
                │  Aggregation Layer    │
                │                       │
                │  • Merge new output   │
                │    into running state │
                │  • Compress if needed │
                │  • Emit Tauri events  │
                └───────────────────────┘
                        │
                        ▼
                  [Agent Tab] in UI
```

### Audio Chunking Strategy

1. **Recording runs continuously** — PCM stream saved to file as today
2. **Chunk buffer**: Every 60–90 seconds, extract the latest audio chunk from the PCM stream
3. **Convert to WAV**: Prepend WAV header to the chunk (same as existing `convert_to_wav` logic but for a slice)
4. **Send to Qwen Omni**: Audio chunk + text context → structured output
5. **Aggregate**: Merge new output into running state

```
Timeline:
0:00 ──────── 1:00 ──────── 2:00 ──────── 3:00 ──────── 4:00
     [chunk 1]      [chunk 2]      [chunk 3]      [chunk 4]
         │               │               │               │
         ▼               ▼               ▼               ▼
     Agent Cycle 1   Cycle 2         Cycle 3         Cycle 4
```

### The Aggregation Loop (Core Logic)

This is the heart of the agent. Each cycle:

```
CYCLE N:
  Inputs:
    1. audio_chunk     → last 60-90s of raw audio (WAV)
    2. running_context → aggregated text from ALL previous cycles

  Prompt to Qwen Omni:
    "You are a meeting co-pilot. Here is the conversation context so far,
     followed by a new audio segment. Analyze the new segment and produce
     a structured update."

    CONTEXT SO FAR:
    {running_context}

    [AUDIO: audio_chunk.wav]

    Produce (JSON):
    {
      "new_content_summary": "What was discussed in this segment",
      "updated_full_summary": "Updated summary of entire conversation so far",
      "key_points": ["point 1", "point 2"],
      "decisions": ["decision 1"],
      "action_items": ["action 1"],
      "suggested_questions": [
        {"question": "...", "reason": "..."}
      ],
      "key_concepts": [
        {"term": "...", "context": "why it matters"}
      ],
      "open_questions": ["unresolved topic 1"]
    }

  After inference:
    running_context = response.updated_full_summary
    → This becomes the input for Cycle N+1
```

**The aggregation is self-compressing**: Each cycle, the model reads the old summary + new audio, then produces a new summary that incorporates both. The running context never grows unbounded — it's always the model's latest compressed understanding.

---

## What the Agent Produces

### Stream A: Rolling Summary
- **Updated every cycle** (60–90s)
- Structured sections: Key Points, Decisions, Action Items, Open Questions
- Model sees full audio fidelity — catches things transcript misses
- Self-compressing: old summary + new audio → new summary

### Stream B: Suggested Questions
- 0–3 questions per cycle, only when genuinely useful
- Each has a **reason** explaining why it's worth asking
- Questions from previous cycles **persist** until dismissed or conversation moves on
- Max 5 visible at a time — oldest auto-expire when new ones arrive

### Stream C: Key Concept Alerts
- Technical terms, acronyms, proper nouns worth looking up
- Includes brief context: "mentioned in relation to database migration"
- Deduplicated across cycles — same term flagged only once

---

## UX — Tabbed Right Panel During Recording

### Tab Layout

```
┌─────────────────────────────────────────┐
│  [ Transcript ]  [ Co-Pilot ]           │
├─────────────────────────────────────────┤
```

**Transcript tab** = existing live transcript (Whisper/WhisperKit, real-time, low-quality)
**Co-Pilot tab** = agent output (Qwen Omni, every 60-90s, high-quality analysis)

User switches between tabs freely. Both run simultaneously during recording.

### Co-Pilot Tab Content

```
┌─────────────────────────────────────────┐
│  [ Transcript ]  [• Co-Pilot ]          │
├─────────────────────────────────────────┤
│                                         │
│  SUMMARY                                │
│  ───────                                │
│  • Discussing Q3 migration timeline     │
│  • Team agreed: phased rollout over     │
│    3 sprints, starting April            │
│  • Budget proposed at $50K — pending    │
│    manager approval                     │
│  • ⚠ No rollback plan discussed yet    │
│                                         │
│  DECISIONS                              │
│  ─────────                              │
│  ✓ Phased approach (not big-bang)       │
│  ✓ Start date: April sprint 1          │
│                                         │
│  ACTION ITEMS                           │
│  ────────────                           │
│  • Sarah: draft migration runbook       │
│  • Mike: get budget approval by Fri     │
│                                         │
│  ───────────────────────────────────    │
│                                         │
│  YOU MIGHT ASK                          │
│  ─────────────                          │
│  "What's the rollback strategy if       │
│   phase 1 fails?"                       │
│   → Rollback not discussed despite      │
│     phased approach being agreed     [×]│
│                                         │
│  "Does the $50K include infra costs?"   │
│   → Budget breakdown wasn't specified   │
│     and infra is often overlooked    [×]│
│                                         │
│  ───────────────────────────────────    │
│                                         │
│  LOOK UP                                │
│  ───────                                │
│  • CQRS pattern — mentioned as the      │
│    target architecture (3x)             │
│  • Saga pattern — proposed for          │
│    distributed transactions             │
│                                         │
│  ───────────────────────────────────    │
│  ● Cycle 4 • Last updated 12s ago      │
│  Agent: active                          │
└─────────────────────────────────────────┘
```

### Agent Toggle

- Toggle on the **Record** center panel (not in the tab itself): **"Co-Pilot: ON / OFF"**
- Default: OFF (no compute overhead unless user opts in)
- When ON: Co-Pilot tab appears in right panel, agent cycles start
- When OFF: Tab disappears, only Transcript tab visible

### Tab Indicator

- **Dot indicator** on Co-Pilot tab when new content arrives while user is on Transcript tab
- Subtle pulse animation when agent is processing a new cycle
- "Last updated Xs ago" footer shows freshness

### Interactions

- **[×] on a question** → dismiss it
- **Click a question** → copy to clipboard
- **Click a concept** → future: open search / show definition
- **Pin a key point** → marks it for the final gem

---

## Post-Recording: Enhanced Gem

When recording stops, the agent's accumulated state enriches the gem:

```
Gem: "Team Sync — Feb 28"
├── Source: Recording (20260228_143000.pcm)
├── Live Transcript (Whisper — raw real-time text)
├── Co-Pilot Summary (Qwen Omni — structured, high-quality)
│   ├── Key Points
│   ├── Decisions Made
│   ├── Action Items
│   └── Open Questions
├── Concept Log (all flagged terms with context)
├── Question History (all suggested questions + which were dismissed)
└── AI Tags + One-line Summary (existing enrichment)
```

Two levels of transcript quality in one gem:
1. **Live transcript**: fast, real-time, every word (low quality)
2. **Co-Pilot summary**: structured understanding from actual audio (high quality)

---

## Technical Details

### Audio Chunk Extraction

The recording is a continuous PCM stream (16kHz, mono, 16-bit). To extract a chunk:

```rust
// Pseudocode — extract last N seconds from PCM file
fn extract_audio_chunk(pcm_path: &Path, chunk_duration_secs: u64) -> Vec<u8> {
    let bytes_per_second = 16000 * 2; // 16kHz * 16-bit
    let chunk_bytes = chunk_duration_secs * bytes_per_second;
    let file_size = fs::metadata(pcm_path).len();
    let start = file_size.saturating_sub(chunk_bytes);
    // Read from start..file_size, prepend WAV header
    let pcm_data = read_range(pcm_path, start, file_size);
    prepend_wav_header(pcm_data)
}
```

This reuses existing PCM→WAV logic. No new audio infrastructure needed.

### Concurrency: Whisper vs Qwen Omni

- **Whisper** (transcription): Runs on its own — whisper-rs/WhisperKit/MLX Omni for real-time transcript
- **Qwen Omni** (agent): Runs via MLX sidecar for audio understanding + structured analysis
- These are **different models on different processes** — no resource contention
- If using MLX Omni for BOTH transcription and agent, need sequential scheduling (transcription has priority, agent waits)

### Token/Compute Budget Per Cycle

| Component | Tokens |
|---|---|
| System prompt | ~100 |
| Running context (compressed summary) | ~300–500 |
| Audio chunk (60–90s) | Multimodal — processed as audio, not tokens |
| Output (structured JSON) | ~300–500 |
| **Total text tokens** | **~700–1,100** |

Audio processing is separate from text token count in multimodal models. A 60–90s audio chunk is well within Qwen Omni's capabilities.

### Inference Time Per Cycle

- Qwen Omni on M-series: ~5–15 seconds for audio understanding + text generation
- With 60–90s cycle interval, there's plenty of headroom
- If inference exceeds the cycle interval, skip next cycle (freshness > completeness)

### Failure Handling

- Agent is **purely supplementary** — recording + transcript continue regardless
- If Qwen Omni fails: show "Co-Pilot: paused" indicator, retry next cycle
- If model is slow: show "Co-Pilot: processing..." with cycle number
- No data loss on failure — audio is always saved, transcript always runs

---

## What We Need to Build

### Backend (Rust)

1. **`src/agents/mod.rs`** — Agent module
2. **`src/agents/copilot.rs`** — Co-Pilot agent implementation
   - Audio chunk extraction from active recording
   - Cycle timer (60–90s configurable)
   - Running context state management
   - Qwen Omni inference calls via MLX sidecar
   - JSON response parsing
   - Tauri event emission
3. **New Tauri commands**:
   - `start_copilot_agent` — begin agent cycles (called when user toggles ON)
   - `stop_copilot_agent` — stop agent cycles
   - `get_copilot_state` — fetch current summary/questions/concepts
4. **New Tauri events**:
   - `copilot-updated` — emitted after each cycle with full state
   - `copilot-status` — "processing", "active", "paused", "error"

### Frontend (React)

1. **`src/components/CoPilotPanel.tsx`** — Agent output display
   - Summary section with key points, decisions, action items
   - Suggested questions with dismiss buttons
   - Key concepts list
   - Status footer (cycle count, last updated)
2. **Modify `RightPanel.tsx`** — Add tab switcher during recording
   - `[Transcript]` tab = existing `TranscriptDisplay`
   - `[Co-Pilot]` tab = new `CoPilotPanel`
   - Dot indicator for unread updates
3. **Modify center panel Record section** — Add Co-Pilot toggle switch
4. **`src/hooks/useCoPilot.ts`** — Hook for agent state + Tauri event listener

### Prompt Engineering

1. Design structured prompt for Qwen Omni (audio + text → JSON)
2. Test with different audio: meetings, lectures, 1-on-1s, noisy environments
3. Tune cycle interval — 60s for fast-paced, 90s for slower discussions
4. Calibrate output: how many questions per cycle, summary compression ratio

---

## Implementation Phases

### Phase 1: Audio Chunking + Basic Inference
- Extract audio chunks from active recording
- Send to Qwen Omni with simple prompt
- Log output to console — validate quality
- No UI yet

### Phase 2: Agent Loop + State Management
- Implement cycle timer (60s)
- Build aggregation loop (running context fed back each cycle)
- Tauri events for state updates
- Basic `start_copilot_agent` / `stop_copilot_agent` commands

### Phase 3: Frontend — Co-Pilot Tab
- Tab switcher in right panel during recording
- CoPilotPanel component with summary/questions/concepts sections
- Real-time updates via Tauri event listener
- Co-Pilot toggle on Record screen

### Phase 4: Polish
- Question dismiss/copy interactions
- Status indicators (processing, active, paused)
- Dot indicator on tab for unread updates
- Post-recording: fold agent state into gem

---

## Open Questions

1. **Cycle interval**: Fixed 60s? 90s? Adaptive based on speech density?
2. **Audio chunk overlap**: Should chunks overlap by 10s to avoid missing context at boundaries?
3. **Model selection**: Always Qwen Omni? Or allow user to choose agent model in settings?
4. **Concurrent model use**: If Whisper and Qwen Omni both need MLX, how to schedule?
5. **Gem storage**: Store full agent history or just final summary in gem?
6. **Privacy indicator**: Should there be a visible indicator that audio is being sent to the model? (Even though it's local, users may want to know)
