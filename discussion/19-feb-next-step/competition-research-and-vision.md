# JARVIS Competition Research — The Bigger Vision

**Date**: Feb 19, 2026
**Purpose**: Research findings, competitive landscape, tech scouting, and the real vision.

---

## The Problem Is Bigger Than Meetings

The original pitch — "JARVIS helps you in meetings" — is too narrow. Every competitor will build a meeting assistant. The real problem:

> **Knowledge workers lose 40% of their productive time to context switching — toggling between 1,200 apps per day, losing 23 minutes of focus per switch, costing organizations $450 billion annually.**

Sources:
- Harvard Business Review: average digital worker toggles between apps/sites **1,200 times per day**
- UC Irvine: **23 minutes 15 seconds** to regain focus after interruption
- 2024 studies: **2.1 hours/day** lost to distractions, costing **$10,375 per employee annually**
- **40% of knowledge workers** never get a single uninterrupted 30-minute block in their workday
- Heavy multitasking drops IQ by **10 points** and increases errors by **50%**

The problem isn't "I forget things in meetings." The problem is:

> **Every time you switch between ChatGPT, Slack, email, browser tabs, documents, and meetings — you lose context. Your knowledge is scattered across dozens of tools, and no tool follows you across all of them.**

---

## The Real Vision: Digital Shadow

JARVIS shouldn't be a meeting assistant. It should be an **ambient cognitive layer** — a digital shadow that:

1. **Follows you everywhere** — meetings, LLM chats, browser sessions, documents, Slack conversations
2. **Captures everything** — not just audio, but the knowledge artifacts you create and consume
3. **Connects everything** — builds a knowledge graph of relationships between topics, people, decisions, and commitments across all contexts
4. **Surfaces everything** — proactively brings relevant context from ANY past interaction when you need it, regardless of which tool it came from

This is not a note-taker. This is a **second brain** that never forgets.

---

## Competitive Landscape

### TwinMind (Direct Competitor — $5.7M Seed, Sequoia Capital)

- Founded March 2024 by ex-Google X team (Daniel George, Sunny Tang, Mahi Karim)
- **What they do**: Ambient listening app that captures speech all day, builds a personal knowledge graph
- **How**: Runs in background on phone, 16-17 hours battery life, processes audio on-device
- **Speech model**: Ear-3 — 5.26% word error rate, 140+ languages, $0.23/hour
- **Privacy**: Audio deleted on-fly, only transcribed text stored locally
- **Strength**: Pure mobile, always-on, privacy-first
- **Weakness**: Phone-only, no desktop/browser, no cross-app integration, no AWS

**JARVIS differentiator vs TwinMind**:
- JARVIS is AWS-native (Bedrock, Knowledge Bases, AgentCore)
- JARVIS ingests knowledge from MULTIPLE sources (not just audio) — LLM chats, articles, YouTube, documents
- JARVIS uses multi-agent swarm (not single pipeline)
- JARVIS has GraphRAG for connected knowledge (not flat vector search)
- JARVIS has Anticipation Agent (proactive, not reactive)

### Other Competitors

| Solution | What they do | JARVIS advantage |
|----------|-------------|-----------------|
| Otter.ai / Fireflies | Cloud transcription + summaries | JARVIS has unified cross-platform memory |
| Granola | Meeting notes with AI | JARVIS works beyond meetings |
| Notion AI | Document-level AI assistant | JARVIS connects knowledge across tools |
| Recall.ai | Meeting bot that joins calls | JARVIS is ambient, no bot needed |
| Rewind.ai (now Limitless) | Screen recording + search | JARVIS is semantic, not screenshot-based |
| Second Brain (.io) | Saves content from social/web | JARVIS processes and connects, not just saves |

### Why JARVIS Wins

None of these competitors have ALL of:
1. Real-time ambient listening
2. Cross-source knowledge ingestion (LLM chats, articles, meetings, videos)
3. Knowledge graph with relationship mapping (GraphRAG)
4. Multi-agent swarm for parallel intelligence
5. Proactive anticipation (not just reactive retrieval)
6. AWS-native (serverless, scalable, cheap)

---

## Tech Stack — What Makes It Outstanding

### 1. Strands Agents SDK (AWS's official agent framework)

**What**: Python SDK for building multi-agent systems with Amazon Bedrock
**Why it matters for competition**: This is AWS's own agent SDK — using it shows alignment with AWS's vision

Key features:
- `@tool` decorator to wrap agents as tools for other agents
- Supervisor + collaborator pattern for multi-agent orchestration
- Native integration with AgentCore Memory
- Supports Bedrock, Anthropic, OpenAI, Ollama model providers

```python
from strands import Agent
from strands.tools import tool

@tool
def memory_agent(query: str) -> str:
    """Searches personal knowledge base for relevant context."""
    # Bedrock Knowledge Bases RetrieveAndGenerate
    ...

@tool
def anticipation_agent(transcript: str) -> str:
    """Predicts what the user will need next based on conversation flow."""
    ...

supervisor = Agent(
    tools=[memory_agent, anticipation_agent, research_agent, action_agent],
    system_prompt="You are JARVIS, coordinating specialized agents..."
)
```

**Source**: [Strands Agents SDK](https://github.com/strands-agents/sdk-python) | [Multi-Agent Example](https://strandsagents.com/latest/documentation/docs/examples/python/multi_agent_example/multi_agent_example/)

### 2. Bedrock AgentCore Memory (Episodic Memory)

**What**: Managed service that gives agents the ability to remember past interactions
**Why it's a game-changer**: Your agents LEARN from experience

Memory types:
- **Short-term memory**: Turn-by-turn within a session (conversation context)
- **Long-term memory**: Cross-session persistence (user preferences, facts, summaries)
- **Episodic memory**: Captures meaningful "episodes" from interactions — automatically detects when an episode is complete and consolidates it into structured records

Built-in strategies:
- `summaryMemoryStrategy` — summarizes conversation sessions
- `userPreferenceMemoryStrategy` — learns and stores user preferences
- `semanticMemoryStrategy` — extracts and stores factual information

**For JARVIS**: Each meeting/conversation becomes an "episode" that the agents can recall later. "In your October 15 episode with the Acme team, the key decision was..."

**Source**: [AgentCore Memory Docs](https://docs.aws.amazon.com/bedrock-agentcore/latest/devguide/memory.html) | [Episodic Memory](https://docs.aws.amazon.com/bedrock-agentcore/latest/devguide/episodic-memory-strategy.html)

### 3. GraphRAG with Neptune Analytics

**What**: Knowledge graph + vector search combined. GA since March 2025.
**Why it's better than flat RAG**: Understands RELATIONSHIPS between pieces of knowledge

Standard RAG: "Find chunks similar to this query" → returns isolated text snippets
GraphRAG: "Find chunks similar to this query AND traverse the relationships" → returns connected knowledge

Example:
- Standard RAG: "What did we discuss about pricing?" → Returns the pricing paragraph
- GraphRAG: "What did we discuss about pricing?" → Returns the pricing paragraph + the competitor analysis that influenced it + the budget constraint from the board meeting + Sarah's objection from the follow-up call

**For JARVIS**: When the Memory Agent searches your knowledge, it doesn't just find matching text — it traverses the graph of connected topics, people, decisions, and commitments to surface the FULL context.

**Source**: [Bedrock GraphRAG with Neptune](https://aws.amazon.com/blogs/machine-learning/build-graphrag-applications-using-amazon-bedrock-knowledge-bases/) | [Neptune Analytics](https://aws.amazon.com/blogs/database/using-knowledge-graphs-to-build-graphrag-applications-with-amazon-bedrock-and-amazon-neptune/)

### 4. Amazon Transcribe Streaming (WebSocket)

**What**: Real-time speech-to-text via WebSocket from browser
**Latency**: 50-200ms chunks, partial result stabilization
**Free Tier**: 60 minutes/month
**Auth**: AWS Signature V4 → needs Lambda proxy or Cognito

Reference implementations:
- [amazon-transcribe-websocket-static](https://github.com/amazon-archives/amazon-transcribe-websocket-static) — browser-only demo
- [amazon-transcribe-websocket](https://github.com/aws-samples/amazon-transcribe-websocket) — Node.js backend

### 5. Kiro IDE (Required by competition)

Best practices for the competition:
- Start with **spec-driven workflow** — requirements.md → design.md → tasks
- Configure **steering files** immediately (product.md, structure.md, tech.md)
- Use **MCP servers**: AWS Documentation MCP, Context7
- Show the Kiro workflow in the article (judges want to see you used it meaningfully)
- Use **agent hooks** for automated testing

**Source**: [Kiro Best Practices](https://kiro.dev/docs/specs/best-practices/) | [Kiro Specs Docs](https://kiro.dev/docs/specs/)

---

## Competition Strategy — What Judges Want

From research on the competition:

> "The setup favors thoughtful designs over flashy prototypes. You win by being specific, not loud."

### What judges evaluate:

1. **Tight problem** — not "AI for everything" but "AI for one painful, specific workflow"
2. **Clear agent loop** — context → plan → act → verify → log
3. **Real users in sight** — who uses this and why
4. **Crisp story** — the article matters as much as the prototype
5. **AWS service usage** — breadth and depth of AWS integration
6. **Kiro usage** — must use Kiro for at least part of development
7. **Free Tier compliance** — stays within budget
8. **Community votes** — article quality + shareability

### Our advantages:

| Factor | How JARVIS scores |
|--------|------------------|
| Problem specificity | "Context loss across tools" — universally felt, measurable ($450B/year) |
| Agent loop | 5-agent swarm with supervisor, parallel execution, episodic memory |
| Real users | Every knowledge worker in meetings, interviews, negotiations |
| Story | "Your digital shadow" — memorable, shareable concept |
| AWS depth | Transcribe + Bedrock + AgentCore + Knowledge Bases + GraphRAG + Neptune + Lambda + Step Functions + S3 + CloudWatch |
| Kiro | Spec-driven development visible in article |
| Cost | Free Tier + $200 credits |

---

## The Narrative That Wins

### Don't say: "I built a meeting assistant"
**Everyone** will say that. There will be 50+ meeting assistants in the Top 1,000.

### Say: "I built a digital shadow — an ambient AI swarm that follows you across every tool and conversation, building a living knowledge graph of everything you've ever learned, and proactively surfacing it when you need it."

### The article hook:

> "You're in a meeting. The client mentions a competitor you researched three weeks ago in a ChatGPT conversation. The pricing data is in a spreadsheet you opened on your laptop. The decision from the last call is buried in an email thread. You can't recall any of it.
>
> Now imagine a swarm of AI agents working in the background — one recalling your ChatGPT research, another pulling the pricing data, a third predicting the client's next question, and a fourth capturing every commitment you make. All in real-time. All without you asking.
>
> That's JARVIS. Not a meeting bot. A digital shadow."

---

## Revised Architecture — The Full Vision

```
┌─────────────────────────────────────────────────────────────────────┐
│                    KNOWLEDGE INGESTION LAYER                        │
│                                                                     │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ │
│  │ LLM Chat │ │ Articles │ │ YouTube  │ │ Meeting  │ │ Documents│ │
│  │ Exports  │ │ (Medium) │ │ Transcr. │ │ Notes    │ │ (PDF/MD) │ │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ │
│       └──────┬─────┴──────┬─────┴──────┬─────┴──────┬─────┘       │
│              ▼            ▼            ▼            ▼              │
│        ┌─────────────────────────────────────────────┐             │
│        │              Amazon S3 Bucket                │             │
│        │         (unified knowledge store)            │             │
│        └───────────────────┬─────────────────────────┘             │
│                            ▼                                        │
│        ┌─────────────────────────────────────────────┐             │
│        │     Bedrock Knowledge Bases + GraphRAG       │             │
│        │     (Neptune Analytics knowledge graph)      │             │
│        │     Entities → Relationships → Embeddings    │             │
│        └─────────────────────────────────────────────┘             │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                    REAL-TIME AGENT SWARM                             │
│                                                                     │
│  Browser Mic ──→ Amazon Transcribe Streaming ──→ Live Transcript    │
│                          │                                          │
│                          ▼                                          │
│              ┌──────────────────────┐                               │
│              │   SUPERVISOR AGENT   │ ← Bedrock AgentCore           │
│              │   (Strands SDK)      │                               │
│              └──────────┬───────────┘                               │
│                         │                                           │
│           ┌─────────────┼─────────────┬─────────────┐              │
│           ▼             ▼             ▼             ▼              │
│    ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐    │
│    │  MEMORY    │ │  RESEARCH  │ │ ANTICIPATE │ │  ACTION    │    │
│    │  AGENT     │ │  AGENT     │ │  AGENT     │ │  AGENT     │    │
│    │            │ │            │ │            │ │            │    │
│    │ GraphRAG   │ │ Web Search │ │ Predicts   │ │ Captures   │    │
│    │ Knowledge  │ │ Current    │ │ next need  │ │ commitments│    │
│    │ Bases +    │ │ context    │ │ before you │ │ questions  │    │
│    │ Neptune    │ │ real-time  │ │ ask        │ │ decisions  │    │
│    │ Analytics  │ │ data       │ │            │ │ follow-ups │    │
│    └─────┬──────┘ └─────┬──────┘ └─────┬──────┘ └─────┬──────┘    │
│          └──────────────┴──────────────┴──────────────┘            │
│                              │                                      │
│                              ▼                                      │
│              ┌──────────────────────┐                               │
│              │  INSIGHT SYNTHESIZER │                               │
│              │  (Bedrock Claude)    │                               │
│              └──────────┬───────────┘                               │
│                         │                                           │
│              ┌──────────▼───────────┐                               │
│              │  AgentCore Memory    │                               │
│              │  (Episodic Memory)   │                               │
│              │  This conversation   │                               │
│              │  becomes a future    │                               │
│              │  knowledge source    │                               │
│              └──────────────────────┘                               │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                    CONTEXT OVERLAY (React UI)                        │
│                                                                     │
│  ┌─────────────────┐  ┌──────────────────────────────────────────┐ │
│  │                  │  │ [Memory] [Research] [Predict] [Actions]  │ │
│  │  Live Transcript │  │                                          │ │
│  │                  │  │  Your Oct 15 meeting with Acme:          │ │
│  │  "So about the   │  │  - Agreed on $50K base                  │ │
│  │   pricing for    │  │  - 10% volume discount                  │ │
│  │   the Acme       │  │  - Sarah needs board approval >$75K     │ │
│  │   deal..."       │  │                                          │ │
│  │                  │  │  PREDICTION: They'll ask about timeline. │ │
│  │                  │  │  Your last estimate was 8 weeks.         │ │
│  │                  │  │                                          │ │
│  │  ● Listening     │  │  ACTION: Send revised proposal by Fri   │ │
│  └─────────────────┘  └──────────────────────────────────────────┘ │
│  Transcribe: 1.2s │ Agents: 2.8s │ Graph: 847 nodes │ us-east-1  │
└─────────────────────────────────────────────────────────────────────┘
```

### The Closed Loop (What Makes It a True Shadow)

The magic: **today's conversations become tomorrow's knowledge**.

1. You have a meeting → Transcribe captures it → agents process it
2. AgentCore Episodic Memory stores it as a structured episode
3. Next week, you mention the same topic → Memory Agent retrieves it via GraphRAG
4. The knowledge graph grows with every interaction

No manual note-taking. No manual knowledge entry. The system learns from your life.

---

## AWS Services — Full Stack

| Service | Component | Why (for judges) |
|---------|-----------|-----------------|
| **Bedrock (Claude)** | All agents | Core reasoning engine |
| **Bedrock AgentCore** | Supervisor + multi-agent | AWS's newest agent platform |
| **AgentCore Memory** | Episodic memory | Agents that learn over time |
| **Strands Agents SDK** | Agent code | AWS's official agent SDK |
| **Bedrock Knowledge Bases** | Memory Agent RAG | Managed retrieval |
| **GraphRAG + Neptune Analytics** | Knowledge graph | Connected knowledge (not flat) |
| **Transcribe Streaming** | Real-time audio | WebSocket speech-to-text |
| **S3** | Knowledge store | Document storage |
| **Lambda** | Agent compute | Serverless, free tier |
| **Step Functions Express** | Parallel orchestration | Agent swarm coordination |
| **API Gateway WebSocket** | Frontend ↔ Backend | Real-time communication |
| **Cognito** | Auth | Browser → Transcribe auth |
| **DynamoDB** | Session state | Conversation history |
| **CloudWatch** | Observability | Agent latency + traces |
| **EventBridge** | Event routing | Knowledge ingestion triggers |

That's **15 AWS services** used meaningfully. Most competitors will use 3-5.

---

## What Makes This a $250K Winner

| Dimension | Our Edge |
|-----------|----------|
| **Problem** | Universal ($450B/year context switching cost) |
| **Vision** | "Digital shadow" — not a meeting tool, a cognitive layer |
| **Architecture** | Multi-agent swarm with GraphRAG + episodic memory |
| **AWS Depth** | 15 services, using the latest (AgentCore, GraphRAG, Strands) |
| **Demo Moment** | Anticipation Agent predicts what you need before you ask |
| **Narrative** | "Your AI swarm that never forgets" — shareable, memorable |
| **Technical Credibility** | Local prototype already works (Tauri + Whisper.cpp) |
| **ROI** | Quantifiable: 40% productivity gain, $10K/employee/year saved |

---

## Research Sources

- [TwinMind — Ex-Google X "Second Brain" (TechCrunch)](https://techcrunch.com/2025/09/10/ex-google-x-trio-wants-their-ai-to-be-your-second-brain-and-they-just-raised-6m-to-make-it-happen/)
- [TwinMind Ear-3 Speech Model (Maginative)](https://www.maginative.com/article/twinmind-aims-to-be-your-second-brain-raises-5-7m-and-announces-breakthrough-speech-model/)
- [Amazon Bedrock AgentCore](https://aws.amazon.com/bedrock/agentcore/)
- [AgentCore Episodic Memory (AWS Blog)](https://aws.amazon.com/blogs/machine-learning/build-agents-to-learn-from-experiences-using-amazon-bedrock-agentcore-episodic-memory/)
- [Strands Agents SDK (GitHub)](https://github.com/strands-agents/sdk-python)
- [Multi-Agent Orchestration on AWS](https://aws.amazon.com/solutions/guidance/multi-agent-orchestration-on-aws/)
- [Bedrock GraphRAG with Neptune Analytics](https://aws.amazon.com/blogs/machine-learning/build-graphrag-applications-using-amazon-bedrock-knowledge-bases/)
- [GraphRAG GA Announcement](https://aws.amazon.com/blogs/machine-learning/announcing-general-availability-of-amazon-bedrock-knowledge-bases-graphrag-with-amazon-neptune-analytics/)
- [Context Switching Costs — Productivity Report](https://productivityreport.org/2025/04/11/how-much-time-do-we-lose-task-switching/)
- [Context Switching — $450B Cost (Conclude)](https://conclude.io/blog/context-switching-is-killing-your-productivity/)
- [Ambient Agents — Always-On AI (DigitalOcean)](https://www.digitalocean.com/community/tutorials/ambient-agents-context-aware-ai)
- [Ambient Agents — Future of Intelligence (Medium)](https://medium.com/@fahey_james/ambient-agents-and-the-future-of-always-on-intelligence-85c21137d070)
- [AWS Multi-Agent Collaboration (VentureBeat)](https://venturebeat.com/ai/aws-brings-multi-agent-orchestration-to-bedrock)
- [Kiro Best Practices](https://kiro.dev/docs/specs/best-practices/)
- [Kiro Spec-Driven Development](https://kiro.dev/docs/specs/)
- [AWS 10000 AIdeas Competition](https://builder.aws.com/connect/events/10000aideas)
- [Amazon Transcribe WebSocket Demo (GitHub)](https://github.com/amazon-archives/amazon-transcribe-websocket-static)
- [Strands Multi-Agent Example](https://strandsagents.com/latest/documentation/docs/examples/python/multi_agent_example/multi_agent_example/)
- [AgentCore Memory Docs](https://docs.aws.amazon.com/bedrock-agentcore/latest/devguide/memory.html)
- [The 2026 Guide to Bedrock AgentCore](https://www.goml.io/blog/amazon-bedrock-agentcore)
