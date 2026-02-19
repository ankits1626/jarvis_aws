# JARVIS Agent Swarm — AWS Competition Architecture

**Date**: Feb 19, 2026
**Context**: Semi-finalist (Top 1,000). Deadline Mar 13. ~3.5 weeks remaining.

---

## Current State

Local JARVIS desktop app (Tauri + Rust) is working:
- Audio capture via sidecar (JarvisListen)
- Hybrid transcription pipeline: VAD (Silero) → Vosk (instant partials) → Whisper (accurate finals)
- WhisperKit integration (Apple Neural Engine) as alternative engine
- Settings UI with configurable window duration, engine selection, model management
- libvosk.dylib now bundled in .app for portability

**What's proven**: Real-time listen → transcribe → display works locally.

---

## The Pivot for AWS Competition

The local app is the long-term product. For the competition, build an **AWS-native agent swarm** that demonstrates the same core value using AWS services — but goes further with multi-agent intelligence.

### Why a swarm wins

Most competitors will build: `mic → Transcribe → Bedrock RAG → display`. That's a single pipeline. JARVIS runs **5 specialized agents in parallel** that each bring a different dimension of intelligence to the conversation.

---

## Agent Swarm Architecture

```
                              ┌─ Memory Agent ──────── Bedrock Knowledge Bases
                              │                        (your past meetings, notes, chats)
                              │
Browser   Amazon              │
  Mic  → Transcribe ──→ Topic ├─ Research Agent ───── Bedrock + Lambda
         Streaming    Detector │                       (current info about discussed topics)
         (WebSocket)    Agent  │
                              ├─ Anticipation Agent ── Bedrock Claude
                              │                        (predicts what you'll need next)
                              │
                              └─ Action Agent ─────── Bedrock Claude
                                                      (captures commitments & follow-ups)
                                        │
                                        ▼
                                Insight Synthesizer ──→ React UI
                                (Bedrock Claude)        (tabbed context panel)
```

### Agent Orchestration: Step Functions Express

```json
{
  "StartAt": "TopicDetector",
  "States": {
    "TopicDetector": {
      "Type": "Task",
      "Resource": "arn:aws:lambda:...:topic-detector",
      "Next": "ParallelAgents"
    },
    "ParallelAgents": {
      "Type": "Parallel",
      "Branches": [
        { "StartAt": "MemoryAgent", "States": { "MemoryAgent": { "Type": "Task", "Resource": "arn:aws:lambda:...:memory-agent", "End": true } } },
        { "StartAt": "ResearchAgent", "States": { "ResearchAgent": { "Type": "Task", "Resource": "arn:aws:lambda:...:research-agent", "End": true } } },
        { "StartAt": "AnticipationAgent", "States": { "AnticipationAgent": { "Type": "Task", "Resource": "arn:aws:lambda:...:anticipation-agent", "End": true } } },
        { "StartAt": "ActionAgent", "States": { "ActionAgent": { "Type": "Task", "Resource": "arn:aws:lambda:...:action-agent", "End": true } } }
      ],
      "Next": "InsightSynthesizer"
    },
    "InsightSynthesizer": {
      "Type": "Task",
      "Resource": "arn:aws:lambda:...:insight-synthesizer",
      "End": true
    }
  }
}
```

**Why Step Functions Express**: Runs in <30s, costs $1 per million executions, returns all parallel results together.

---

## The 5 Agents — Detail

### 1. Topic Detector Agent

**Input**: Last 30 seconds of transcript text
**Output**: Extracted topics, entities, intent
**AWS Service**: Lambda + Bedrock Claude Haiku (fast, cheap)

```
Prompt: "Extract the key topics, named entities (people, companies, products),
and conversational intent from this transcript chunk. Return JSON."
```

**Why it matters**: Gates the other agents. No point searching knowledge base if the conversation is small talk.

### 2. Memory Agent

**Input**: Topics from Topic Detector
**Output**: Relevant past knowledge with source attribution
**AWS Service**: Bedrock Knowledge Bases (RetrieveAndGenerate API)

```
Knowledge sources in S3:
- Past meeting notes (markdown)
- LLM chat exports (ChatGPT, Claude conversations)
- Article highlights (Medium, blog posts)
- YouTube transcript summaries
```

**Example output**: "Last time you discussed pricing with the Acme team (Jan 15 meeting), you agreed on $50K/year with a 10% volume discount. Sarah mentioned they'd need board approval above $75K."

**Why it matters**: This is the core value — perfect recall of your scattered knowledge.

### 3. Research Agent

**Input**: Topics from Topic Detector
**Output**: Current, relevant external information
**AWS Service**: Lambda + Bedrock Claude + web search (or pre-seeded current-events S3 bucket for demo)

**Example output**: "Acme Corp announced Q4 earnings yesterday — revenue up 15%. Their CTO mentioned AI investment in the earnings call."

**Why it matters**: Goes beyond your personal knowledge. Brings the outside world into your conversation context.

### 4. Anticipation Agent

**Input**: Full conversation transcript so far + topics
**Output**: Predicted next topics + pre-fetched context
**AWS Service**: Bedrock Claude Sonnet

```
Prompt: "Given this conversation flow, predict what topics or questions
are likely to come up in the next 2-3 minutes. For each prediction,
explain why and what context the user would need."
```

**Example output**: "Based on the pricing discussion, they'll likely ask about implementation timeline. Your last proposal to a similar client estimated 8 weeks. Prepare: your team's current capacity is at 70%."

**Why it matters**: This is the "wow" moment. JARVIS doesn't just react — it anticipates. No other competitor will have this.

### 5. Action Agent

**Input**: Transcript chunks
**Output**: Detected commitments, questions, follow-ups, decisions
**AWS Service**: Bedrock Claude Haiku

```
Prompt: "Identify any commitments, action items, open questions,
or decisions made in this transcript. Attribute to speaker if possible."
```

**Example output**:
- Commitment: "You agreed to send the revised proposal by Friday"
- Question (open): "What's the integration timeline with their existing CRM?"
- Decision: "Go with Option B — phased rollout starting Q2"

**Why it matters**: Practical value. After the meeting, you have a clean list of what was decided and what you owe.

---

## UI Design — Tabbed Context Panel

```
┌────────────────────────────────────────────────────────────────┐
│  JARVIS                                            [Settings]  │
├───────────────────────────────┬────────────────────────────────┤
│                               │  [Memory] [Research] [Predict] │
│   Live Transcript             │  [Actions]                     │
│                               │────────────────────────────────│
│   "So about the pricing       │  MEMORY                        │
│    for the Acme deal..."      │                                │
│                               │  Jan 15 Meeting with Acme:     │
│   "I think we discussed       │  - Agreed $50K/year base       │
│    this last month"           │  - 10% volume discount         │
│                               │  - Sarah needs board approval  │
│   "What was the number        │    above $75K                  │
│    we landed on?"             │                                │
│                               │  ─────────────────────────     │
│                               │  PREDICTION                    │
│                               │  They'll likely ask about      │
│                               │  implementation timeline.      │
│                               │  Your last estimate: 8 weeks.  │
│                               │                                │
│   ● Listening...              │  ─────────────────────────     │
│                               │  ACTIONS                       │
│                               │  ☐ Send revised proposal (Fri) │
│                               │  ? Integration timeline w/ CRM │
├───────────────────────────────┴────────────────────────────────┤
│  Transcribe: 1.2s │ Agents: 2.8s │ 5 docs indexed │ us-east-1│
└────────────────────────────────────────────────────────────────┘
```

---

## AWS Services Map

| Service | Agent/Component | Free Tier | Purpose |
|---------|----------------|-----------|---------|
| Transcribe Streaming | Listener | 60 min/mo | Real-time speech-to-text |
| Bedrock (Claude Haiku) | Topic Detector, Action Agent | Credits | Fast, cheap classification |
| Bedrock (Claude Sonnet) | Anticipation, Synthesizer | Credits | Deep reasoning |
| Bedrock Knowledge Bases | Memory Agent | With Bedrock | Managed RAG over S3 docs |
| S3 | Knowledge store | 5 GB free | Meeting notes, chat exports, articles |
| Lambda | All agents | 1M invocations | Serverless compute |
| Step Functions Express | Orchestration | 4K transitions | Parallel agent execution |
| API Gateway WebSocket | Frontend ↔ Backend | 1M messages | Real-time communication |
| Cognito | Auth | 50K MAU | Browser → Transcribe auth |
| CloudWatch | Monitoring | 5 GB logs | Agent latency tracking |
| DynamoDB | Session state | 25 GB free | Conversation history |

**Estimated cost**: <$25 total using Free Tier + $200 new account credits.

---

## 3-Week Build Plan (Revised)

### Week 1: Feb 19-25 — Transcribe + Memory Agent

**Goal**: Audio in → live transcript → knowledge surfaced

- [ ] Set up AWS account (us-east-1), enable Bedrock, budget alerts
- [ ] Install Kiro IDE, initialize project with spec-driven mode
- [ ] React app (Vite + TypeScript) with browser mic capture
- [ ] Lambda proxy for Transcribe Streaming WebSocket auth
- [ ] Connect browser → Lambda → Transcribe → live transcript in UI
- [ ] Create S3 bucket with 5-10 knowledge documents
- [ ] Create Bedrock Knowledge Base, wait for indexing
- [ ] Build Topic Detector (Lambda + Claude Haiku)
- [ ] Build Memory Agent (Lambda + Bedrock KB RetrieveAndGenerate)
- [ ] Wire: transcript → topic detector → memory agent → UI context panel
- [ ] Test: speak about a topic in your knowledge base → verify context appears

### Week 2: Feb 26 - Mar 4 — Remaining Agents + Polish

**Goal**: Full swarm running in parallel

- [ ] Build Research Agent (Lambda + Claude, pre-seeded current docs)
- [ ] Build Anticipation Agent (Lambda + Claude Sonnet)
- [ ] Build Action Agent (Lambda + Claude Haiku)
- [ ] Build Insight Synthesizer (Lambda + Claude Sonnet)
- [ ] Create Step Functions Express state machine for parallel execution
- [ ] Wire all agents: transcript chunk → Step Functions → UI tabs
- [ ] Build tabbed UI: Memory | Research | Predict | Actions
- [ ] End-to-end latency optimization (target: <5s for full swarm response)
- [ ] Add status bar: transcribe latency, agent latency, docs indexed

### Week 3: Mar 5-12 — Demo + Article + Submit

**Goal**: Publish prototype article on AWS Builder Center

- [ ] UI polish — clean, professional layout
- [ ] Error handling and edge cases
- [ ] Record demo video (2-3 min):
  - Show the problem (cognitive overload)
  - Start a live conversation about a topic in the knowledge base
  - Show Memory Agent surfacing past context
  - Show Anticipation Agent predicting the next question (the "wow" moment)
  - Show Action Agent capturing commitments
- [ ] Write article (structure below)
- [ ] Create clean architecture diagrams
- [ ] Peer review
- [ ] Publish on AWS Builder Center by Mar 12

---

## Article Narrative — The Winning Story

### Hook
"You're in a meeting. Someone references a decision from three weeks ago. You can't remember the details. You fumble. The moment passes. What if you had an AI swarm working for you — one agent recalling your past, another researching the present, a third predicting what comes next, and a fourth capturing every commitment?"

### Core Message
"JARVIS isn't a chatbot. It's a **swarm of specialized AI agents** that augment your cognitive abilities during live conversations. Each agent brings a different dimension of intelligence — memory, research, prediction, and action tracking — working in parallel so you can focus on being present."

### Technical Depth
- Show the Step Functions state machine (parallel execution)
- Show Bedrock Knowledge Bases (unified memory from scattered sources)
- Show the Topic Detector → Parallel Agents → Synthesizer pipeline
- Include latency metrics (Transcribe: <2s, Agent Swarm: <5s)

### The Differentiator
"Most AI meeting tools transcribe and summarize. JARVIS **anticipates**. The Anticipation Agent watches conversation flow and surfaces context before you need it. This is the difference between a note-taker and a cognitive partner."

---

## Key Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Transcribe WebSocket auth from browser | Blocks Week 1 | Lambda proxy with Cognito identity pool |
| Step Functions latency too high | Swarm feels slow | Use Express workflows + Claude Haiku for fast agents |
| Anticipation Agent predictions are wrong | Undermines the "wow" | Seed good knowledge docs, rehearse demo conversation |
| Credits run out | Can't iterate | Set $10/$25 budget alerts, use Haiku for dev, Sonnet for demo |
| Too ambitious for 3 weeks | Incomplete prototype | Memory Agent alone is a viable MVP, other agents are additive |

---

## Fallback Plan

If time runs short, the **minimum viable swarm** is:

1. Transcribe Streaming → live transcript
2. Topic Detector → Memory Agent → context panel

That's still better than most competitors. Each additional agent (Research, Anticipation, Action) makes it more impressive but isn't required for a working demo.

---

## The Pitch in One Line

> "A swarm of AI agents that gives you superhuman memory, real-time research, predictive intelligence, and automatic action tracking — all during a live conversation, all on AWS."
