# JARVIS — 20-Day Sprint Plan (Feb 19 → Mar 13)

## The Honest Scoping

The full vision has 5 agents, GraphRAG, episodic memory, and 15 AWS services. That's a 3-month product. We have 20 days and one developer. Here's what actually ships.

### The Rule: Build 3 things that tell the full story

The competition judges an **article with a prototype**, not a production app. We need:

1. A **working demo** that creates a visceral "wow" moment
2. An **architecture diagram** that shows the full vision
3. A **narrative** that makes judges think "this person gets it"

The prototype proves we can build it. The article sells the vision.

---

## Competition Submission = 3 Deliverables

| Deliverable | What | Deadline |
|-------------|------|----------|
| **Prototype** | Working web app: mic → transcribe → 3 agents → context panel | Mar 8 |
| **Demo Video** | 2-3 min screen recording showing the "aha" moment | Mar 10 |
| **Article** | Published on AWS Builder Center with architecture + story | Mar 12 |

---

## Feature Set — What Ships

### The 3-Agent Slice (not 5)

Drop Research Agent and Insight Synthesizer. They're nice-to-have but don't change the story. Keep the 3 agents that each prove a different capability:

```
Browser Mic → Transcribe Streaming → Topic Detector
                                          │
                    ┌─────────────────────┼──────────────────────┐
                    ▼                     ▼                      ▼
              MEMORY AGENT       ANTICIPATION AGENT        ACTION AGENT
              "Here's what       "They'll ask about        "You just committed
               you discussed      pricing next —            to sending the
               last time"         here's your data"         proposal by Friday"
                    │                     │                      │
                    └─────────────────────┼──────────────────────┘
                                          ▼
                                    React UI Panel
                                  [Memory | Predict | Actions]
```

| Agent | What it proves | AWS Service | Complexity |
|-------|---------------|-------------|------------|
| **Memory Agent** | "I remember your past" | Bedrock Knowledge Bases | Medium — core RAG |
| **Anticipation Agent** | "I predict your future" | Bedrock Claude Sonnet | Low — single prompt |
| **Action Agent** | "I capture your commitments" | Bedrock Claude Haiku | Low — single prompt |

**Why these 3**: Memory is the core value. Anticipation is the wow factor. Actions are practical utility. Together they tell the complete "digital shadow" story without needing 5 agents.

### What's IN

- Browser mic capture (MediaRecorder API)
- Amazon Transcribe Streaming (WebSocket via Lambda proxy)
- Live transcript display
- Pre-seeded S3 knowledge base (8-10 curated documents)
- Bedrock Knowledge Base with semantic chunking
- Memory Agent — retrieves relevant past knowledge
- Anticipation Agent — predicts next topics/questions
- Action Agent — captures commitments, decisions, open questions
- Tabbed context panel (Memory | Predict | Actions)
- Agent latency display in status bar
- CloudWatch logging for the agent loop

### What's OUT (mentioned in article as "roadmap")

- Research Agent (web search for current context)
- Insight Synthesizer (merges all agent outputs)
- GraphRAG / Neptune Analytics (article mentions it as next phase)
- AgentCore Memory / Episodic Memory (article mentions it)
- Strands Agents SDK (use direct Lambda + Bedrock API instead — simpler)
- Speaker diarization
- Knowledge ingestion pipeline (we pre-seed manually)
- User auth (single-user prototype)
- Session persistence / history
- Desktop overlay

### The Trade-off

The article describes the **full 5-agent architecture with GraphRAG and episodic memory** as the vision. The prototype demonstrates **3 agents with standard RAG** as proof of concept. Judges understand MVPs — showing you know what to build next is as valuable as having built it.

---

## Knowledge Base — The Make-or-Break Investment

The demo is only as good as the knowledge base. If the Memory Agent surfaces irrelevant results, the demo falls flat. Invest 2-3 hours curating these documents carefully.

### Documents to seed (8-10 total)

Create **realistic but controlled** documents that guarantee good retrieval:

| Document | Content | Why |
|----------|---------|-----|
| `meeting-acme-oct15.md` | Pricing discussion with Acme Corp — $50K base, 10% volume discount, Sarah needs board approval above $75K | Demo scenario: "the pricing deal" |
| `meeting-acme-nov02.md` | Follow-up call — implementation timeline discussed, 8-week estimate, team at 70% capacity | Links to the Acme thread |
| `chatgpt-competitor-research.md` | Exported ChatGPT conversation analyzing competitor pricing, market positioning | Shows cross-tool memory |
| `article-ai-market-trends.md` | Summary of an article about AI market trends in enterprise | Background knowledge |
| `meeting-quarterly-review.md` | Q3 quarterly review — revenue targets, product roadmap priorities | Business context |
| `claude-product-strategy.md` | Exported Claude conversation about product strategy decisions | Another LLM chat source |
| `meeting-hiring-jan.md` | Hiring discussion — need 2 engineers, interview pipeline | Different topic thread |
| `notes-conference-keynote.md` | Notes from a conference keynote about industry trends | Broad knowledge |

### Demo Script (rehearse this)

The demo conversation should be **scripted** to hit topics covered in the knowledge base:

1. "Let's talk about the Acme deal..." → Memory Agent surfaces Oct 15 + Nov 2 meeting notes
2. "I think we discussed pricing..." → Memory Agent: "$50K base, 10% volume discount"
3. (Pause) → Anticipation Agent: "They'll likely ask about implementation timeline. Your last estimate was 8 weeks."
4. "I'll send them the revised proposal..." → Action Agent: "Commitment: send revised proposal"
5. "What about the competitor landscape?" → Memory Agent surfaces ChatGPT research export

This demo shows: past memory, prediction, action capture, and cross-tool knowledge — all in 2 minutes.

---

## Day-by-Day Plan

### Phase 1: Foundation (Days 1-7, Feb 19-25)

**Goal**: Audio in → live transcript → Memory Agent surfaces context

| Day | Tasks | Done |
|-----|-------|------|
| **Day 1-2** (Feb 19-20) | AWS setup: account, Bedrock access, budget alerts. Kiro IDE install + init project with spec-driven mode. Generate requirements.md. | |
| **Day 3-4** (Feb 21-22) | React app (Vite + TS). Browser mic capture → PCM chunks. Lambda proxy for Transcribe auth (Cognito or presigned URL). | |
| **Day 5** (Feb 23) | Transcribe Streaming WebSocket connection working. Live transcript displayed in UI. Verify latency <2s. | |
| **Day 6** (Feb 24) | S3 bucket + upload 8-10 knowledge documents. Create Bedrock Knowledge Base. Configure semantic chunking. Start sync. | |
| **Day 7** (Feb 25) | Memory Agent Lambda: takes transcript chunk → calls RetrieveAndGenerate → returns context. Wire to UI. **Milestone: speak about Acme → see past meeting notes appear.** | |

**Phase 1 exit criteria**: "I say 'Acme pricing' and my Oct 15 meeting notes appear in the context panel."

### Phase 2: Agent Swarm (Days 8-13, Feb 26 - Mar 3)

**Goal**: All 3 agents working in parallel, tabbed UI

| Day | Tasks | Done |
|-----|-------|------|
| **Day 8** (Feb 26) | Topic Detector: Lambda + Claude Haiku — extracts topics/entities from transcript chunk. Triggers agents only when meaningful topics detected. | |
| **Day 9** (Feb 27) | Anticipation Agent Lambda: takes full transcript + topics → Claude Sonnet predicts next questions/needs. | |
| **Day 10** (Feb 28) | Action Agent Lambda: takes transcript chunks → Claude Haiku detects commitments, decisions, questions. | |
| **Day 11** (Mar 1) | Parallel execution: Step Functions Express OR simple Promise.all from frontend calling 3 Lambda APIs concurrently. Wire all agents to UI. | |
| **Day 12** (Mar 2) | Tabbed context panel: [Memory] [Predict] [Actions] tabs. Status bar with agent latency. Loading states. | |
| **Day 13** (Mar 3) | Integration testing. Fix edge cases: no relevant context, empty transcript, topic detector false positives. CloudWatch logging. | |

**Phase 2 exit criteria**: "I have a live conversation. Memory tab shows past context. Predict tab anticipates next topic. Actions tab captures my commitments. All update in real-time."

### Phase 3: Demo + Article (Days 14-20, Mar 4-12)

**Goal**: Publishable article with embedded demo

| Day | Tasks | Done |
|-----|-------|------|
| **Day 14** (Mar 4) | UI polish — clean layout, professional typography, JARVIS branding. Visual indicators: listening state, agent processing, relevance. | |
| **Day 15** (Mar 5) | Rehearse demo script 3-4 times. Tune knowledge base documents if retrieval isn't good enough. Adjust agent prompts. | |
| **Day 16** (Mar 6) | Record demo video (2-3 min). Show problem → start conversation → Memory surfaces context → Anticipation predicts → Actions capture. | |
| **Day 17-18** (Mar 7-8) | Write article draft. Hook → What JARVIS Does → Agent Swarm Architecture → AWS Services → Kiro Workflow → Demo → ROI → What's Next. | |
| **Day 19** (Mar 9) | Create architecture diagrams (clean versions). Add screenshots/GIFs. Include Kiro screenshots (requirements.md, design.md). | |
| **Day 20** (Mar 10) | Review, polish, publish on AWS Builder Center. Share on LinkedIn/Twitter. | |
| **Buffer** (Mar 11-12) | Fix anything broken. Engage with community votes. | |

---

## AWS Services — The Actual Build

Only use services you actually need. Don't add complexity for the sake of listing services.

| Service | What for | Must-have? |
|---------|----------|-----------|
| **Transcribe Streaming** | Real-time speech-to-text | Yes |
| **Bedrock (Claude Haiku)** | Topic Detector, Action Agent | Yes |
| **Bedrock (Claude Sonnet)** | Anticipation Agent | Yes |
| **Bedrock Knowledge Bases** | Memory Agent (managed RAG) | Yes |
| **S3** | Knowledge document storage | Yes |
| **Lambda** | Agent compute (4 functions) | Yes |
| **API Gateway** | REST API for frontend → agents | Yes |
| **Cognito** | Browser → Transcribe auth | Yes |
| **CloudWatch** | Agent loop logging | Yes |
| **Step Functions Express** | Parallel agent orchestration | Nice-to-have (can use Promise.all instead) |

**9 services used.** Article mentions 6 more as roadmap (AgentCore, Neptune, DynamoDB, EventBridge, Strands SDK, CloudFront).

---

## The Article Strategy

### Structure

```
1. THE HOOK (200 words)
   "You're in a meeting. Someone mentions a decision from 3 weeks ago..."
   Pain: context switching costs $450B/year, 40% productivity loss

2. WHAT JARVIS DOES (200 words + architecture diagram)
   One-liner: "A swarm of AI agents that gives you superhuman memory,
   predictive intelligence, and automatic action tracking during
   live conversations."

3. THE AGENT SWARM (500 words + code snippets)
   - Topic Detector → parallel fan-out
   - Memory Agent: Bedrock Knowledge Bases retrieves your past
   - Anticipation Agent: Claude predicts what you'll need next
   - Action Agent: Claude captures commitments in real-time
   - Show Lambda function snippet for each agent
   - Show the parallel execution flow

4. BUILDING WITH KIRO (200 words + screenshots)
   - Screenshot of requirements.md Kiro generated
   - Screenshot of design.md with architecture
   - How spec-driven development structured the build

5. THE DEMO (embedded video or GIF walkthrough)
   - 2-3 min video showing the "magic moment"
   - Annotated screenshots of each agent's output

6. THE FULL VISION (300 words + full architecture diagram)
   - What's next: Research Agent, GraphRAG, Episodic Memory
   - The "digital shadow" narrative — beyond meetings
   - Cross-source ingestion: LLM chats, YouTube, articles, Slack

7. ROI (100 words)
   - $10,375/employee/year in context switching costs
   - JARVIS reduces this by 60-80%
   - Runs on AWS Free Tier (<$5/month at scale with Haiku)

Total: ~1,500-1,800 words
```

### Community Voting Tips

The article needs to be **shareable**. What makes people vote:

1. **Visual** — clean architecture diagrams, demo GIFs, not walls of text
2. **Relatable problem** — everyone has felt "I know I discussed this before but can't remember where"
3. **Clear demo moment** — one GIF showing the Anticipation Agent predicting the next question
4. **Technical credibility** — code snippets that show you actually built this, not just described it
5. **Ambitious but grounded** — show the 3-agent MVP AND the 5-agent vision

---

## Risk Mitigation

| Risk | Days lost | Mitigation |
|------|-----------|------------|
| Transcribe WebSocket auth | 2 days | Follow AWS sample repo exactly. Fallback: use Web Speech API for demo (browser-native, no AWS) |
| Bedrock KB indexing slow | 1 day | Create KB on Day 6, docs already uploaded. 24h buffer before needing it. |
| Agent latency too high (>10s) | 1 day | Use Claude Haiku for everything except Anticipation. Reduce transcript chunk size. |
| Credits exhausted | Blocks all | Budget alert at $10. Use Bedrock Playground for prompt dev. Cache responses during testing. |
| UI not polished enough | Hurts votes | Allocate full Day 14 for polish. Use a clean CSS framework (Tailwind). |
| Demo doesn't show "wow" moment | Hurts votes | Rehearse 3-4 times. Script the conversation to hit knowledge base topics perfectly. |

### The Nuclear Fallback

If AWS Transcribe integration takes too long (auth issues):
- Use the **Web Speech API** (built into Chrome) for transcription
- Still send transcript to Bedrock agents via Lambda
- Mention Transcribe Streaming in the article as the "production architecture"
- Demo still works, judges understand the substitution

---

## Success = 3 Things

1. **I speak. JARVIS remembers.** (Memory Agent works)
2. **I pause. JARVIS predicts.** (Anticipation Agent works)
3. **I commit. JARVIS captures.** (Action Agent works)

That's the demo. That's the article. That's the pitch.

Everything else is in the "What's Next" section.
