# jarvis-app Extension Plan — Local Desktop App

**Date**: Feb 19, 2026
**Current state**: Record → Transcribe → Display (in-memory only)

---

## The Gap

The app transcribes beautifully. But transcripts vanish when you close it. You can't search, export, or build on past sessions. The app **listens** but doesn't **remember**.

The next 4 features turn it from a transcription tool into a local knowledge engine.

---

## Extension Roadmap (Priority Order)

### 1. Persist Transcripts (Foundation — enables everything else)

**Problem**: Transcripts live in `Vec<TranscriptionSegment>` in memory. Gone on app close.

**Solution**: Save transcript JSON alongside each recording.

```
~/.jarvis/recordings/
  20260219_143022.pcm          ← audio (exists today)
  20260219_143022.transcript   ← NEW: transcript JSON
```

**Backend changes**:
- When `transcription-stopped` fires with final transcript, write to `.transcript` file
- `files.rs`: update `RecordingMetadata` to include `has_transcript: bool`
- `commands.rs`: add `get_transcript` command that reads `.transcript` file
- On app load, check which recordings have transcripts

**Frontend changes**:
- Show transcript icon next to recordings that have transcripts
- Clicking a recording loads both audio player AND saved transcript
- TranscriptDisplay works in two modes: live (during recording) and playback (from file)

**Effort**: ~1 day

---

### 2. Copy & Export Transcripts

**Problem**: You can see the transcript but can't get it out of the app.

**Solution**: Copy to clipboard + export to markdown/txt.

**UI**:
```
┌─ Transcript ──────────────────────────┐
│                            [Copy] [⬇] │
│  "So about the pricing for Acme..."   │
│  "I think we discussed this before"   │
│  "Let me check on the timeline"       │
└───────────────────────────────────────┘
```

**Backend**:
- `commands.rs`: add `export_transcript` command
- Formats: plain text, markdown (with timestamps), JSON
- Markdown format:
  ```markdown
  # Transcript — Feb 19, 2026 2:30 PM
  Duration: 5:32

  [00:00] So about the pricing for Acme...
  [00:05] I think we discussed this before
  [00:12] Let me check on the timeline
  ```

**Frontend**:
- Copy button → clipboard (plain text, no timestamps)
- Export dropdown → save as .md or .txt file
- Use Tauri's `dialog.save` API for file picker

**Effort**: ~0.5 day

---

### 3. Search Past Transcripts

**Problem**: You have 50 recordings. Which one mentioned "Acme pricing"?

**Solution**: Full-text search across all saved transcripts.

**Backend**:
- `commands.rs`: add `search_transcripts` command
- Loads all `.transcript` files, searches text content
- Returns: matching recordings + highlighted snippets + timestamps
- Future: sqlite-vec for semantic search (not needed for MVP)

**Frontend**:
```
┌─ Search ───────────────────────────────┐
│ 🔍 [acme pricing                    ] │
│                                        │
│ 📄 20260215_meeting.pcm               │
│    "...the pricing for Acme was $50K  │
│     with a 10% volume discount..."     │
│    [02:15]                             │
│                                        │
│ 📄 20260201_notes.pcm                 │
│    "...Acme wants to renegotiate       │
│     the pricing before Q2..."          │
│    [05:42]                             │
└────────────────────────────────────────┘
```

Click a result → opens that recording at the matching timestamp.

**Effort**: ~1 day

---

### 4. LLM Summary & Analysis

**Problem**: A 30-minute transcript is too long to re-read. You want the gist.

**Solution**: One-click summary via local LLM (Ollama) or API (Claude).

**Backend**:
- New module: `src/intelligence/mod.rs`
- `commands.rs`: add `summarize_transcript` command
- Provider trait (like transcription):
  ```rust
  trait IntelligenceProvider {
      fn summarize(&self, transcript: &str) -> Result<Summary>;
      fn extract_actions(&self, transcript: &str) -> Result<Vec<ActionItem>>;
  }
  ```
- Providers:
  - `OllamaProvider` — local, free, private (llama3, mistral)
  - `ClaudeProvider` — API, better quality, costs money
  - `BedrockProvider` — AWS, bridges to competition version

**Frontend**:
```
┌─ Summary ──────────────────────────────┐
│ 📋 Meeting Summary                     │
│                                        │
│ Discussed Acme deal pricing ($50K      │
│ base with 10% volume discount).        │
│ Implementation timeline estimated at   │
│ 8 weeks. Sarah needs board approval    │
│ for amounts above $75K.                │
│                                        │
│ ─── Action Items ───                   │
│ ☐ Send revised proposal by Friday     │
│ ☐ Check team capacity for Q2          │
│ ? What's the CRM integration plan?    │
│                                        │
│ [Copy Summary] [Copy Actions]          │
└────────────────────────────────────────┘
```

**Effort**: ~2 days

---

## How These Extensions Bridge to the AWS Competition

| Local Feature | AWS Equivalent | Same Story |
|--------------|----------------|------------|
| Persist transcripts | AgentCore Episodic Memory | "Remember everything" |
| Search transcripts | Bedrock Knowledge Bases | "Unified memory" |
| LLM summary | Bedrock Claude | "Augment thinking" |
| Action extraction | Action Agent | "Capture commitments" |

The local app proves the concept works offline. The AWS version scales it to the cloud with multi-agent intelligence. Same narrative, two implementations.

---

## Suggested Build Order

```
                    ┌─────────────────────┐
                    │  1. PERSIST         │  ← Foundation
                    │  Save transcripts   │     (enables 2, 3, 4)
                    └────────┬────────────┘
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
    ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
    │ 2. EXPORT   │  │ 3. SEARCH   │  │ 4. LLM      │
    │ Copy/save   │  │ Find past   │  │ Summarize   │
    │ transcripts │  │ transcripts │  │ + actions   │
    └─────────────┘  └─────────────┘  └─────────────┘
         0.5 day         1 day           2 days
```

Total: ~4.5 days of focused work.

After these 4, the app goes from "transcription demo" to "personal knowledge tool" — and that's the same pitch as the AWS competition version, just running locally.

---

## Optional (Post-Competition)

These are valuable but not urgent:

- **Menu bar / system tray mode** — run as floating overlay, not full window
- **Speaker diarization** — identify who said what
- **Semantic search** — sqlite-vec embeddings for "find conversations ABOUT this topic"
- **Auto-categorization** — tag recordings by topic/project
- **Calendar integration** — link recordings to calendar events
- **Waveform visualization** — show audio waveform in player
