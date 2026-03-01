# NotebookLM Prompt — Generate Jiya & Vikram Meeting Audio

Copy everything below the line into NotebookLM as a source document, then generate an audio overview.

---

## INSTRUCTIONS

Generate a realistic audio conversation between two colleagues on a video call: **Jiya** (Product Lead) and **Vikram** (VP of Engineering) at a mid-stage startup.

This is a project kickoff meeting. It should feel like a real 1:1 — natural, conversational, with "yeah," "makes sense," "exactly" sprinkled in. Not scripted or stiff.

**Length: 8–10 minutes.**

### Character Voices

**Vikram (VP of Engineering):**
- Senior leader. Direct, strategic, asks sharp questions.
- Cares about budget, security, timeline, and business impact.
- Excited about the opportunity but disciplined about execution.
- Uses phrases like "What's the trade-off?", "Make sure that's in the doc", "Start narrow, prove it works."

**Jiya (Product Lead):**
- Well-prepared, references specific research she did that morning.
- Gives concrete numbers — not vague. She says "thirty to forty percent cost premium" not "it costs more."
- Mentions she used ChatGPT to pressure-test her analysis (this is natural for her — she validates her thinking with AI tools).
- Confident but collaborative. Proposes decisions, doesn't wait to be told.

### Backstory (Don't Narrate This — It's Context)

Jiya works at a mid-stage startup. She uses a desktop app called Jarvis to capture and organize her research — articles, YouTube videos, emails, meeting recordings, even ChatGPT conversations. She has two existing projects in Jarvis:

1. **"Q4 Strategy Briefing"** — from a month ago. Contains an internal strategy call recording where the team discussed customer support gaps (68% of Tier 1 tickets are repetitive, 12-minute resolution time, chatbots too rigid, $50K budget floated).

2. **"Tech Trends — 2025 Roundup"** — from two weeks ago. Contains articles and a YouTube video on AWS Bedrock Agents, Strands SDK, agentic AI trends.

This morning, Vikram emailed her: "Leadership wants to green-light the agentic AI platform. Kickoff call at 4 PM. Pull together what we know."

Jiya spent the morning researching — she captured a Medium article on agent costs, an article on orchestration patterns, a YouTube demo of Strands SDK, and had a ChatGPT conversation comparing Bedrock vs. open-source frameworks. She also pulled relevant gems from her two old projects. By 2 PM she generated a research synthesis across all 9 sources and prepared for this call.

### The Conversation Must Cover These Points

Weave these into natural dialogue — don't list them:

**1. The Problem (Vikram frames it):**
- Leadership saw Q4 customer support numbers and they're not happy
- 12-minute average resolution time for Tier 1 tickets
- 68% of those tickets are repetitive — password resets, billing questions, order status
- "We should not need humans for that"

**2. Jiya Already Knows (she references past work):**
- She had the 68% number from the Q4 strategy call
- Team already leaned toward AI agents over chatbots — chatbots are too rigid

**3. Budget & Target:**
- Leadership approved $50,000 for a Q1 prototype
- Goal: handle at least 60% of Tier 1 tickets autonomously
- Not just deflect — actually resolve

**4. Build vs. Buy Decision:**
- Jiya presents two options: AWS Bedrock AgentCore (managed) vs. Strands Agents SDK (open-source, also AWS)
- Trade-off: managed platforms have 30-40% cost premium per call BUT save 2-3 engineering FTEs
- "I also ran this comparison through ChatGPT to pressure-test it" — ChatGPT confirmed managed makes sense for prototype budget
- Migration path is smooth — Strands supports Bedrock as a provider
- **Decision: Bedrock AgentCore for V1.** Evaluate open-source for V2.

**5. Architecture:**
- Jiya explains three orchestration patterns: chains (sequential, too slow), graphs (DAG-based, parallel — recommended), swarms (autonomous, too unpredictable)
- Graphs allow parallel execution: check order status + search knowledge base + pull customer history simultaneously
- Vikram asks about latency — customers will compare to talking with a human
- Bedrock keynote showed sub-second for simple tool calls, 2-4 seconds for multi-step
- **Decision: Under 2 seconds for first response. Stream partial responses.**

**6. Security & Data Privacy:**
- Vikram asks: "What about the data side? Customer data is sensitive."
- Jiya: Bedrock runs in their AWS account, data never leaves VPC
- "I confirmed this in my ChatGPT research too — with open-source you'd need to handle VPC endpoints, network policies, encryption yourself. Bedrock does it natively."
- Vikram: "Make sure that's highlighted in the design doc. Security is the first thing leadership asks about."

**7. Deliverables & Action Items:**
- Jiya commits to a technical design document by next Friday (architecture, agent definitions, tool specs, cost projections, phased rollout)
- Phase 1 scope: three most common ticket types — password resets, billing inquiries, order status. Covers ~40% of volume.
- "Start narrow, prove it works, then expand"
- Jiya will evaluate Strands SDK as fallback and include comparison
- Jiya needs to schedule a follow-up with Arun (engineering lead) for integration with existing ticketing system
- Vikram offers to set up the meeting with Arun
- Jiya asks Vikram to share the current support ticket data export — needs real examples to test agent responses
- Jiya will send her research synthesis (9 sources) to Vikram by end of day

**8. Platform Vision (toward the end):**
- Vikram: "I want this to be a showcase project, not a one-off"
- If it succeeds, roll out to 3 more teams by Q3 — sales, onboarding, internal ops
- "Think about it as a platform"
- Jiya: "I'll design the agent framework to be domain-agnostic — tools and knowledge bases plug in, orchestration layer stays the same"
- **Decision: Domain-agnostic agent platform design.**

**9. Wrap-Up:**
- Check in again Wednesday once Jiya has the design doc outline
- Friendly, natural close — "Talk Wednesday." / "Thanks. Bye."

### What NOT to Do
- Don't mention "NotebookLM," "AI-generated," or "simulation"
- Don't add narration, intro music descriptions, or scene-setting — just the conversation
- Don't add a third speaker — only Jiya and Vikram
- Don't have them summarize the call at the end — it should end naturally like a real meeting
- Don't make it sound like they're reading bullet points — weave information into natural back-and-forth
- Don't skip the ChatGPT references — Jiya naturally mentions she used ChatGPT as a research tool (twice: once during build-vs-buy, once during security discussion)
