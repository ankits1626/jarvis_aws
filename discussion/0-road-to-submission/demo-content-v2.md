# Jarvis Demo — Content Plan v2

## 1. The Research Topic

**Topic: Building an Agentic AI Platform for Customer Support**

**New project title:** "Agentic AI Platform — Customer Support"
**Description:** "Research and plan an AI agent platform to automate Tier 1 customer support using multi-agent orchestration."
**Objective:** "Technical design doc + cost estimate for a Q2 prototype that handles 60% of support tickets autonomously."

---

## 2. Pre-Existing Projects (Already in Jarvis Before Demo Day)

These projects should be created days/weeks before the demo so timestamps look realistic.

### Project 1: "Q4 Strategy Briefing" (created ~1 month ago, status: Completed)

| # | Gem Title | Source | Content Summary |
|---|-----------|--------|-----------------|
| 1 | "Q4 All-Hands — Strategy & Priorities" | Email | VP's email outlining Q4 priorities: customer retention, support efficiency, AI exploration. Mentions leadership frustration with support resolution times. |
| 2 | "Q4 Strategy Call — AI Roadmap & Support Gaps" | Recording | Internal call. Key facts: 68% of Tier 1 tickets are repetitive (password resets, billing, order status). 12-min avg resolution. Chatbots too rigid. VP floated $50K budget if someone owns the initiative. Vendor lock-in concerns raised. |
| 3 | "What Are AI Agents? A Practical Introduction" | Article | Overview of AI agents vs. chatbots. Covers tool use, memory, reasoning loops, ReAct pattern. Explains why agents handle edge cases better than rule-based chatbots. |

**Tags across gems:** `strategy`, `customer-support`, `ai-agents`, `chatbots`, `budget`, `q4`

### Project 2: "Tech Trends — 2025 Roundup" (created ~2 weeks ago, status: Active)

| # | Gem Title | Source | Content Summary |
|---|-----------|--------|-----------------|
| 1 | "Top 10 AI/ML Trends Reshaping Enterprise Software" | Medium | Covers: agentic AI, multimodal models, on-device inference, RAG maturation, AI observability. Highlights agentic AI as #1 trend for 2025-26. |
| 2 | "The Rise of Automation: RPA to Agentic AI" | Article | Evolution from simple RPA bots to cognitive agents. Key insight: agents differ from RPA because they can reason, use tools, and adapt to novel situations. |
| 3 | "AWS re:Invent 2025 — Amazon Bedrock Agents Deep Dive" | YouTube | Bedrock AgentCore walkthrough — managed memory, tool execution, session persistence, Agent Collaboration for multi-agent routing. Sub-second latency for simple tool calls. |
| 4 | "Strands Agents SDK — Open Source AI Agent Framework" | Article | AWS open-source Python framework. Model-agnostic, OpenTelemetry observability, simple decorator syntax for tools. Builds a customer support agent in 50 lines. |

**Tags across gems:** `ai-trends`, `agentic-ai`, `bedrock`, `strands-sdk`, `automation`, `aws`, `open-source`

---

## 3. Gems Captured During Demo Day

### Gem — VP Email (8:30 AM)
**Title:** "RE: Agentic AI Platform — Green Light from Leadership"
**Source:** Gmail
**Content:** "Hey Jiya — remember that agentic AI platform idea we discussed last month? Leadership wants to green-light it. Can you pull together what we know and have a kickoff call with me at 4 PM today? Let's move fast. We've got budget flexibility this quarter. — Vikram"
**Tags:** `project-kickoff`, `agentic-ai`, `leadership`
**Summary:** "VP green-lights agentic AI platform — kickoff call at 4 PM, budget available."

### Gem — Medium Article (9:15 AM)
**Title:** "The Real Cost of Running Multi-Agent Systems in Production"
**Source:** Medium
**Content:** Production cost breakdown for LLM-based agents. Managed (Bedrock, Azure) vs. self-hosted (Strands, LangGraph). Key finding: managed has 30-40% cost premium but saves 2-3 FTEs. Per-call: $0.002-0.008 per invocation.
**Tags:** `ai-agents`, `cost-analysis`, `multi-agent`, `production`
**Summary:** "Managed agent platforms cost 30-40% more per call but save 2-3 engineering headcount vs. self-hosted."

### Gem — Article (10:00 AM)
**Title:** "Agent Orchestration Patterns: Chains, Graphs, and Swarms"
**Source:** Article
**Content:** Compares three patterns. Chains: sequential, simple, limited. Graphs: DAG-based, parallel execution (LangGraph). Swarms: autonomous negotiation (unpredictable). Recommends graphs for customer support — parallel info retrieval while maintaining control flow.
**Tags:** `agent-orchestration`, `architecture`, `patterns`
**Summary:** "Graph-based DAG orchestration recommended for customer support — enables parallel execution with controllable flow."

### Gem — YouTube Video (10:45 AM)
**Title:** "Building Production AI Agents with Strands SDK — AWS Summit 2025"
**Source:** YouTube
**Content:** Live demo building a customer support agent in 50 lines of Python. Shows tool definition, agent loop, multi-agent orchestration. Model-agnostic, OpenTelemetry built in.
**Tags:** `strands-sdk`, `ai-agents`, `aws`, `python`
**Summary:** "AWS Summit demo of Strands SDK — open-source Python framework for AI agents with multi-agent orchestration."

### Gem — ChatGPT Conversation (11:30 AM)
**Title:** "ChatGPT — Bedrock AgentCore vs Open Source Comparison"
**Source:** ChatGPT
**Content:**

> **Jiya:** Compare AWS Bedrock AgentCore with open-source agent frameworks like Strands SDK and LangGraph for building a customer support automation platform. I need to make a build-vs-buy decision for a $50K Q1 prototype.
>
> **ChatGPT:** Here's a structured comparison:
>
> **Bedrock AgentCore (Managed)**
> - Pros: Zero infrastructure, managed memory & sessions, VPC-contained (data never leaves AWS), built-in tool execution, sub-second latency for simple calls
> - Cons: 30-40% cost premium, less customization, tied to AWS ecosystem
> - Best for: Fast prototyping, teams without dedicated ML infra engineers
>
> **Strands SDK (Open Source, AWS)**
> - Pros: Model-agnostic (Bedrock, OpenAI, local), full control, great observability (OpenTelemetry), simple Python decorators
> - Cons: You own the infrastructure, need to build memory/session management, more engineering investment upfront
> - Best for: Teams that want flexibility and plan to scale beyond AWS
>
> **LangGraph (Open Source, LangChain)**
> - Pros: Graph-based orchestration (DAG), parallel execution, large ecosystem, good for complex multi-step workflows
> - Cons: Heavier abstraction layer, steeper learning curve, LangChain dependency
> - Best for: Complex workflows requiring conditional branching and parallel execution
>
> **My recommendation for your case:** Start with Bedrock AgentCore for the prototype — it minimizes infrastructure work and your $50K budget is better spent on agent design than ops. Evaluate Strands for V2 if you need cost optimization at scale. The migration path from Bedrock to Strands is relatively smooth since Strands supports Bedrock as a provider.
>
> **Jiya:** What about latency? Our customers expect near-instant responses.
>
> **ChatGPT:** Bedrock AgentCore shows sub-second for simple tool calls. For multi-step resolution (order lookup + knowledge search + action), expect 2-4 seconds. Key strategy: stream the first response immediately while background tools execute. Set a hard target — I'd suggest under 2 seconds for first visible response.
>
> **Jiya:** And data privacy? Customer data is sensitive.
>
> **ChatGPT:** This is actually Bedrock's strongest argument. Everything runs in your VPC — customer data never touches third-party infrastructure. With open-source options, you'd need to set up VPC endpoints, configure network policies, manage encryption at rest and in transit yourself. Bedrock handles all of that natively.

**Tags:** `bedrock`, `strands-sdk`, `langraph`, `comparison`, `architecture`
**Summary:** "ChatGPT analysis comparing Bedrock AgentCore vs Strands SDK vs LangGraph — recommends Bedrock for prototype, Strands for scale."

---

## 4. Gems Transferred from Old Projects (the nudge)

When Jiya creates the new project, Jarvis recommends these gems from her existing projects:

**From "Q4 Strategy Briefing":**
- "Q4 Strategy Call — AI Roadmap & Support Gaps" (Recording) — 92% relevant (mentions AI agents, support metrics, budget)
- "What Are AI Agents? A Practical Introduction" (Article) — 78% relevant (foundational context on agents)

**From "Tech Trends — 2025 Roundup":**
- "AWS re:Invent 2025 — Bedrock Agents Deep Dive" (YouTube) — 95% relevant (exact platform under consideration)
- "Strands Agents SDK — Open Source AI Agent Framework" (Article) — 88% relevant (the alternative being evaluated)

She selects all four. They're added to the new project.

---

## 5. Full Gem List in Project at Summary Time (before the call)

| # | Gem | Source | When Added |
|---|-----|--------|------------|
| 1 | VP email — green light | Gmail | 8:30 AM (captured) |
| 2 | Q4 Strategy Call — support gaps | Recording | Transferred from Q4 project |
| 3 | What Are AI Agents? | Article | Transferred from Q4 project |
| 4 | Bedrock Agents Deep Dive | YouTube | Transferred from Tech Trends |
| 5 | Strands SDK overview | Article | Transferred from Tech Trends |
| 6 | Cost of Multi-Agent Systems | Medium | 9:15 AM (captured) |
| 7 | Orchestration Patterns | Article | 10:00 AM (captured) |
| 8 | Strands SDK — AWS Summit demo | YouTube | 10:45 AM (captured) |
| 9 | ChatGPT — Bedrock vs Open Source | ChatGPT | 11:30 AM (captured) |

**9 gems. 4 transferred from old projects. 5 captured fresh. 1 ChatGPT conversation.**

---

## 6. Project Summary Output (Act 4)

> **Agentic AI Platform — Research Synthesis**
>
> **Goal:** Build an AI agent platform to automate Tier 1 customer support, targeting 60% autonomous ticket resolution within a $50K Q1 budget.
>
> **Key Findings Across 9 Sources:**
>
> - **The problem is well-defined.** 68% of Tier 1 tickets are repetitive (password resets, billing, order status). Average resolution: 12 minutes. The Q4 strategy call already established that chatbots are too rigid — the team wants agents that can reason and adapt. (Q4 recording, VP email)
>
> - **Graph-based orchestration is the consensus.** Three sources converge: DAG-based orchestration enables parallel execution (check order + search KB + pull customer history simultaneously). Sequential chains are too slow for real-time support. (Orchestration patterns article, Strands demo, Bedrock keynote)
>
> - **Bedrock AgentCore is the fastest path to prototype.** Managed memory, tool execution, VPC-contained data flow, sub-second latency for simple calls. The ChatGPT analysis and cost article both confirm: managed platforms have a 30-40% premium but save 2-3 FTEs — for a $50K prototype, infrastructure savings outweigh per-call costs. (Bedrock keynote, ChatGPT comparison, cost article)
>
> - **Strands SDK is the scale play.** Model-agnostic, full control, better long-term cost profile. Migration path from Bedrock to Strands is smooth — Strands supports Bedrock as a provider. Worth evaluating for V2. (Strands article, Summit demo, ChatGPT comparison)
>
> - **Cost range: $600-2,400/month at 10K tickets/day.** Per-call cost $0.002-0.008 depending on model and context length. Manageable within prototype budget. (Cost article)
>
> **Open Questions for Kickoff Call:**
> 1. Build vs. buy for orchestration layer? (Leaning: buy for V1, build for V2)
> 2. Latency target for first agent response?
> 3. Human escalation path when agent can't resolve?
> 4. Success metrics beyond resolution rate (CSAT, confidence thresholds)?

---

## 7. The Call Transcript — Jiya & Vikram

**Duration:** ~8 min (edited for demo — represents a 35-min call)
**Participants:** Jiya (Product Lead), Vikram (VP of Engineering)

---

**[00:00]**

**Vikram:** Hey Jiya, thanks for jumping on this so quickly. I know it's short notice.

**Jiya:** No worries. I actually spent the morning pulling together research — I've got a pretty good picture of the landscape already.

**Vikram:** Great, that's exactly what I was hoping. So let me give you the context from leadership. They saw the Q4 customer support numbers and they're not happy. Twelve minutes average resolution time for Tier 1 tickets is way too high. And what really got their attention is that sixty-eight percent of those tickets are repetitive — password resets, billing questions, order status. We should not need humans for that.

**Jiya:** Yeah, I actually had that number from our Q4 strategy call. I pulled up my notes from that meeting. The team was already leaning toward AI agents over traditional chatbots because the chatbots are too rigid — they can't handle the edge cases.

**Vikram:** Exactly. So here's what leadership approved: fifty thousand dollars for a Q1 prototype. The goal is to build something that can handle at least sixty percent of Tier 1 tickets autonomously. Not just deflect them — actually resolve them.

**[01:30]**

**Jiya:** Okay, fifty K and sixty percent resolution. That's ambitious but doable. I've been looking at two main approaches. Option one is AWS Bedrock AgentCore — it's managed infrastructure, handles agent memory, tool execution, the whole orchestration layer. Option two is going open-source with the Strands Agents SDK, which is also from AWS but gives us more control.

**Vikram:** What's the trade-off?

**Jiya:** Cost versus control. I found an article that breaks this down, and I also ran the comparison through ChatGPT to pressure-test it — managed platforms like Bedrock have about a thirty to forty percent cost premium per agent call. But they save you two to three engineering headcount because you're not building the infrastructure yourself. For a prototype with fifty K budget, I think managed makes more sense. We can always migrate later — Strands actually supports Bedrock as a provider, so the migration path is smooth.

**Vikram:** I agree. Let's go with Bedrock AgentCore for the first iteration. We can evaluate open-source options for V2 if we need to optimize costs at scale.

**Jiya:** Good. Decision made — Bedrock AgentCore for V1.

**[03:00]**

**Vikram:** Now what about the architecture? How would these agents actually work?

**Jiya:** So I looked at three orchestration patterns — chains, graphs, and swarms. For customer support, the recommendation from everything I've read is graph-based orchestration. It lets us run things in parallel — while one agent checks order status, another searches the knowledge base, and a third looks up customer history. All at the same time. Chains would force sequential processing, which is too slow for real-time support.

**Vikram:** Makes sense. What's the latency looking like? Our customers are going to compare this against chatting with a human. If the agent takes thirty seconds to respond, they'll hate it.

**Jiya:** The Bedrock keynote from re:Invent showed sub-second latency for simple tool calls. For a multi-step resolution — like checking an order and issuing a refund — I'd estimate two to four seconds end-to-end. But we should set a hard target.

**Vikram:** Let's say under two seconds for the first response. The customer should see something within two seconds, even if the full resolution takes longer. We can stream partial responses.

**Jiya:** Got it. Latency target: under two seconds for first agent response, streaming enabled.

**[04:30]**

**Vikram:** What about the data side? Our customer data is sensitive. I don't want any of it going through third-party models without controls.

**Jiya:** Bedrock runs within our AWS account — the data doesn't leave our VPC. That's actually one of the biggest advantages over the other managed options. I confirmed this in my ChatGPT research too — with open-source, you'd need to set up VPC endpoints, network policies, encryption yourself. Bedrock handles all of that natively.

**Vikram:** Good. Make sure that's highlighted in the design doc. Security is going to be the first thing leadership asks about.

**Jiya:** Noted. I'll include an architecture diagram showing the data flow stays within our AWS boundary.

**[05:30]**

**Vikram:** Okay, let's talk deliverables and timeline. What do you need from me?

**Jiya:** I'll put together a technical design document by next Friday. It'll cover the architecture, agent definitions, tool specifications, cost projections, and a phased rollout plan. Phase one would be the three most common ticket types — password resets, billing inquiries, and order status checks. That alone covers about forty percent of volume.

**Vikram:** That's smart. Start narrow, prove it works, then expand. What else?

**Jiya:** I want to evaluate Strands SDK as our fallback option, in case Bedrock's pricing doesn't work at scale. I'll include a comparison in the design doc. And I need to schedule a follow-up with the engineering lead — probably Arun — to go over integration points with our existing ticketing system.

**Vikram:** I'll set up that meeting. Anything else?

**Jiya:** Can you share the current support ticket data export? I need real examples to test the agent responses against. And I'll send you my research synthesis by end of day — I've got nine sources covering the technical landscape, cost analysis, and architectural patterns.

**Vikram:** Perfect. Send it over. I'll review tonight and we can iterate.

**[07:00]**

**Vikram:** One more thing — I want this to be a showcase project. If we pull this off, we'll roll it out to three more teams by Q3. So think about it not as a one-off but as a platform. The agents we build for support should be reusable patterns for sales, onboarding, internal ops.

**Jiya:** Understood. I'll design the agent framework to be domain-agnostic — the tools and knowledge bases plug in, but the orchestration layer stays the same.

**Vikram:** Love it. Alright, this was great Jiya. Really glad you hit the ground running today. Let's check in again Wednesday once you've got the design doc outline.

**Jiya:** Sounds good. Talk Wednesday.

**Vikram:** Thanks. Bye.

**[07:45]**

---

## 8. Co-Pilot Cards During the Call

| Time | Type | Card Text |
|------|------|-----------|
| ~01:30 | Key Point | VP confirms $50K budget for Q1 prototype |
| ~01:45 | Key Point | Target: 60% autonomous resolution of Tier 1 tickets |
| ~02:30 | Decision | Use Bedrock AgentCore for V1 (managed over open-source) |
| ~03:30 | Key Point | Graph-based orchestration recommended for parallel agent execution |
| ~04:15 | Decision | Latency target: under 2s for first response, streaming enabled |
| ~05:00 | Key Point | Bedrock data stays within AWS VPC — no third-party exposure |
| ~05:45 | Action Item | Prepare technical design document by next Friday |
| ~06:00 | Key Point | Phase 1: password resets, billing, order status (~40% of volume) |
| ~06:15 | Action Item | Evaluate Strands SDK as fallback — include comparison in doc |
| ~06:30 | Action Item | Schedule follow-up with Arun (engineering lead) |
| ~06:45 | Action Item | Send research synthesis to Vikram by EOD |
| ~07:15 | Decision | Design agent framework as domain-agnostic platform |
| ~07:30 | Key Point | Q3 expansion target: roll out to 3 more teams |

---

## 9. Post-Call Chat Q&A

**Q1: "List every commitment I made during this call"**

> Based on the transcript, you committed to:
> 1. Prepare a technical design document by next Friday — architecture, agent definitions, tool specs, cost projections, phased rollout
> 2. Evaluate Strands SDK as fallback and include comparison in the design doc
> 3. Schedule follow-up with Arun (engineering lead) for integration review
> 4. Send research synthesis to Vikram by end of day
> 5. Design the agent framework as domain-agnostic for reuse across teams

**Q2: "What did we NOT discuss that we should have?"**

> Based on your pre-meeting research and the call transcript:
> 1. **Multi-model routing** — your orchestration patterns research mentions using different models for different tasks. Not discussed.
> 2. **Detailed cost projections** — $50K budget confirmed but per-call costs ($0.002-0.008) weren't mapped to expected ticket volume.
> 3. **Human escalation path** — no plan for when the agent can't resolve a ticket.
> 4. **Success metrics beyond resolution rate** — CSAT, false resolution rate, and confidence thresholds weren't addressed.
