# Jarvis Demo — Content Plan

## 1. The Research Topic

**Topic: Building an Agentic AI Platform for Customer Support**

Why this topic works:
- Timely — everyone in tech is talking about AI agents (March 2026)
- Naturally produces YouTube videos, articles, and real discussions
- Has depth — architecture decisions, cost trade-offs, tooling choices
- Relatable to any technical audience watching the demo

**Project title:** "Agentic AI Platform — Customer Support Automation"
**Project description:** "Research and plan an AI agent platform to automate Tier 1 customer support using multi-agent orchestration."
**Project objective:** "Deliver a technical design doc and cost estimate for a Q2 prototype that handles 60% of support tickets autonomously."

---

## 2. Pre-Existing Gems (Already in Library Before Demo Day)

These gems should be captured days/weeks before the demo recording so they have realistic timestamps. They represent Riya's past knowledge that becomes relevant when the email arrives.

### Gem A — YouTube Video (captured ~3 weeks ago)
**Title:** "AWS re:Invent 2025 — Amazon Bedrock Agents Deep Dive"
**Source:** YouTube
**Content:** Keynote covering Bedrock AgentCore, managed agent memory, tool execution, session persistence. Mentions sub-second latency for simple tool calls, multi-turn conversation support, and the new Agent Collaboration feature for multi-agent routing.
**Tags:** `bedrock`, `aws`, `ai-agents`, `multi-agent`, `re:invent`
**Summary:** "AWS keynote introducing Bedrock AgentCore — managed infrastructure for building AI agents with persistent memory, tool execution, and multi-agent collaboration."

### Gem B — Medium Article (captured ~2 weeks ago)
**Title:** "The Real Cost of Running Multi-Agent Systems in Production"
**Source:** Medium / Article
**Content:** Breakdown of production costs for LLM-based agent systems. Compares managed (Bedrock, Azure AI) vs. self-hosted (Strands, LangGraph). Key finding: managed platforms have 30-40% cost premium but save 2-3 engineering headcount. Includes per-call cost analysis: $0.002-0.008 per agent invocation depending on model and context length.
**Tags:** `ai-agents`, `cost-analysis`, `multi-agent`, `production`
**Summary:** "Cost analysis of multi-agent systems — managed platforms (Bedrock, Azure) cost 30-40% more per call but save 2-3 engineering FTEs vs. self-hosted (Strands, LangGraph)."

### Gem C — Recording Transcript (captured ~1 month ago)
**Title:** "Q4 Strategy Call — AI Roadmap & Customer Support Gaps"
**Source:** Recording
**Content:** Internal strategy call discussing current customer support metrics. Key points: 68% of Tier 1 tickets are repetitive (password resets, billing inquiries, status checks). Average resolution time 12 minutes. Team explored using chatbots but found them too rigid — agents could handle the nuance. VP mentioned $50K budget flexibility for Q1-Q2 if someone owns the initiative. Concerns raised about vendor lock-in and data privacy.
**Tags:** `strategy`, `customer-support`, `ai-roadmap`, `budget`
**Summary:** "Q4 strategy call identifying customer support automation as top priority — 68% of Tier 1 tickets are repetitive, team wants flexible AI agents over rigid chatbots."

---

## 3. Gems Captured During Research Sprint (Demo Day)

### Gem D — YouTube Video
**Title:** "Building Production AI Agents with Strands SDK — AWS Summit 2025"
**Source:** YouTube
**Content:** Walkthrough of Strands Agents SDK — open-source Python framework for building agents. Shows tool definition, agent loop, multi-agent orchestration. Key selling points: model-agnostic (works with Bedrock, OpenAI, local models), built-in observability with OpenTelemetry, simple Python decorator syntax for tools. Demo builds a customer support agent in 50 lines of code.
**Tags:** `strands-sdk`, `ai-agents`, `aws`, `open-source`, `python`
**Summary:** "AWS Summit demo of Strands Agents SDK — open-source Python framework for building AI agents with model-agnostic tool execution and multi-agent orchestration."

### Gem E — Article
**Title:** "Agent Orchestration Patterns: Chains, Graphs, and Swarms"
**Source:** Article / Blog
**Content:** Technical deep-dive comparing orchestration patterns. Chains (sequential, simple, limited flexibility), Graphs (DAG-based, parallel execution, LangGraph), Swarms (autonomous agents negotiating tasks, OpenAI Swarm). Recommends graphs for production customer support — allows parallel info retrieval (check order status + search knowledge base simultaneously) while maintaining control flow.
**Tags:** `agent-orchestration`, `langraph`, `patterns`, `architecture`
**Summary:** "Comparison of agent orchestration patterns — recommends graph-based (DAG) orchestration for customer support due to parallel execution and controllable flow."

### Gem F — Email from VP (the inciting email)
**Title:** "RE: Agentic AI Platform — Green Light from Leadership"
**Source:** Gmail
**Content:** "Hey Riya — remember that agentic AI platform idea we discussed last month? Leadership wants to green-light it. Can you pull together what we know and have a kickoff call with me at 4 PM today? Let's move fast. We've got budget flexibility this quarter. — Vikram"
**Tags:** `project-kickoff`, `agentic-ai`, `leadership`
**Summary:** "VP green-lights agentic AI platform initiative — kickoff call at 4 PM today, budget available this quarter."

---

## 4. The Call Transcript — Riya & Vikram (VP)

**Duration:** ~8 minutes (edited for demo — represents a 35-minute call)
**Participants:** Riya (Product Lead), Vikram (VP of Engineering)

---

### TRANSCRIPT

**[00:00]**

**Vikram:** Hey Riya, thanks for jumping on this so quickly. I know it's short notice.

**Riya:** No worries. I actually spent the morning pulling together research — I've got a pretty good picture of the landscape already.

**Vikram:** Great, that's exactly what I was hoping. So let me give you the context from leadership. They saw the Q4 customer support numbers and they're not happy. Twelve minutes average resolution time for Tier 1 tickets is way too high. And what really got their attention is that sixty-eight percent of those tickets are repetitive — password resets, billing questions, order status. We should not need humans for that.

**Riya:** Yeah, I actually had that number from our Q4 strategy call. I pulled up my notes from that meeting. The team was already leaning toward AI agents over traditional chatbots because the chatbots are too rigid — they can't handle the edge cases.

**Vikram:** Exactly. So here's what leadership approved: fifty thousand dollars for a Q1 prototype. The goal is to build something that can handle at least sixty percent of Tier 1 tickets autonomously. Not just deflect them — actually resolve them.

**[01:30]**

**Riya:** Okay, fifty K and sixty percent resolution. That's ambitious but doable. I've been looking at two main approaches. Option one is using AWS Bedrock AgentCore — it's managed infrastructure, handles agent memory, tool execution, the whole orchestration layer. Option two is going open-source with something like the Strands Agents SDK, which is also from AWS but gives us more control.

**Vikram:** What's the trade-off?

**Riya:** Cost versus control. I found an article that breaks this down — managed platforms like Bedrock have about a thirty to forty percent cost premium per agent call. But they save you two to three engineering headcount because you're not building the infrastructure yourself. For a prototype with fifty K budget, I think managed makes more sense. We can always migrate later.

**Vikram:** I agree. Let's go with Bedrock AgentCore for the first iteration. We can evaluate open-source options for V2 if we need to optimize costs at scale.

**Riya:** Good. Decision made — Bedrock AgentCore for V1.

**[03:00]**

**Vikram:** Now what about the architecture? How would these agents actually work?

**Riya:** So I looked at three orchestration patterns — chains, graphs, and swarms. For customer support, the recommendation from everything I've read is graph-based orchestration. It lets us run things in parallel — so while one agent checks the order status, another searches the knowledge base, and a third looks up the customer's history. All at the same time. Chains would force sequential processing, which is too slow.

**Vikram:** Makes sense. What's the latency looking like? Our customers are going to compare this against chatting with a human. If the agent takes thirty seconds to respond, they'll hate it.

**Riya:** The Bedrock keynote from re:Invent showed sub-second latency for simple tool calls. For a multi-step resolution — like checking an order and issuing a refund — I'd estimate two to four seconds end-to-end. But we should set a hard target.

**Vikram:** Let's say under two seconds for the first response. The customer should see something within two seconds, even if the full resolution takes longer. We can stream partial responses.

**Riya:** Got it. Latency target: under two seconds for first agent response, streaming enabled.

**[04:30]**

**Vikram:** What about the data side? Our customer data is sensitive. I don't want any of it going through third-party models without controls.

**Riya:** Bedrock runs within our AWS account — the data doesn't leave our VPC. That's actually one of the biggest advantages over the other managed options. And for the knowledge base, we can use Bedrock Knowledge Bases with our existing support documentation. No need to export data anywhere.

**Vikram:** Good. Make sure that's highlighted in the design doc. Security is going to be the first thing leadership asks about.

**Riya:** Noted. I'll include an architecture diagram showing the data flow stays within our AWS boundary.

**[05:30]**

**Vikram:** Okay, let's talk deliverables and timeline. What do you need from me?

**Riya:** I'll put together a technical design document by next Friday. It'll cover the architecture, agent definitions, tool specifications, cost projections, and a phased rollout plan. Phase one would be the three most common ticket types — password resets, billing inquiries, and order status checks. That alone covers about forty percent of volume.

**Vikram:** That's smart. Start narrow, prove it works, then expand. What else?

**Riya:** I want to evaluate Strands SDK as our fallback option, in case Bedrock's pricing doesn't work at scale. I'll include a comparison in the design doc. And I need to schedule a follow-up with the engineering lead — probably Arun — to go over the integration points with our existing ticketing system.

**Vikram:** I'll set up that meeting. Anything else?

**Riya:** Can you share the current support ticket data export? I need real examples to test the agent responses against. And I'll send you my research synthesis by end of day — I've pulled together eight sources covering the technical landscape, cost analysis, and architectural patterns.

**Vikram:** Perfect. Send it over. I'll review tonight and we can iterate.

**[07:00]**

**Vikram:** One more thing — I want this to be a showcase project. If we pull this off, we'll roll it out to three more teams by Q3. So think about it not as a one-off but as a platform. The agents we build for support should be reusable patterns for sales, onboarding, internal ops.

**Riya:** Understood. I'll design the agent framework to be domain-agnostic — the tools and knowledge bases plug in, but the orchestration layer stays the same.

**Vikram:** Love it. Alright, this was great Riya. Really glad you hit the ground running today. Let's check in again Wednesday once you've got the design doc outline.

**Riya:** Sounds good. Talk Wednesday.

**Vikram:** Thanks. Bye.

**[07:45]**

---

## 5. Co-Pilot Cards That Should Appear During the Call

These are the insight cards the Co-Pilot would surface in real-time:

| Timestamp | Type | Card Text |
|-----------|------|-----------|
| ~01:30 | Key Point | VP confirms $50K budget for Q1 prototype |
| ~01:45 | Key Point | Target: 60% autonomous resolution of Tier 1 tickets |
| ~02:30 | Decision | Use Bedrock AgentCore for V1 prototype (managed over open-source) |
| ~03:30 | Key Point | Graph-based orchestration recommended for parallel agent execution |
| ~04:15 | Decision | Latency target: under 2 seconds for first agent response, streaming enabled |
| ~05:00 | Key Point | Bedrock data stays within AWS VPC — no third-party data exposure |
| ~05:45 | Action Item | Riya to prepare technical design document by next Friday |
| ~06:00 | Key Point | Phase 1 scope: password resets, billing inquiries, order status (40% of volume) |
| ~06:15 | Action Item | Evaluate Strands SDK as fallback option — include comparison in design doc |
| ~06:30 | Action Item | Schedule follow-up with engineering lead (Arun) for integration review |
| ~06:45 | Action Item | Riya to send research synthesis to VP by end of day |
| ~07:15 | Decision | Design agent framework as domain-agnostic platform (reusable for sales, onboarding, ops) |
| ~07:30 | Key Point | Q3 expansion target: roll out to 3 more teams if prototype succeeds |

**Summary: 5 key points, 3 decisions, 4 action items**

---

## 6. Post-Call Chat Q&A (scripted)

**Q1: "List every commitment I made during this call"**

> Based on the transcript, you committed to:
> 1. Prepare a technical design document by next Friday — covering architecture, agent definitions, tool specs, cost projections, and phased rollout
> 2. Evaluate Strands SDK as a fallback option and include a comparison in the design doc
> 3. Schedule a follow-up with Arun (engineering lead) for integration review
> 4. Send research synthesis to Vikram by end of day
> 5. Design the agent framework as domain-agnostic for reuse across teams

**Q2: "What did we NOT discuss that we should have?"**

> Based on your pre-meeting research and the call transcript, these topics were not addressed:
> 1. **Multi-model routing** — Your research on orchestration patterns mentions using different models for different agent tasks (cheaper models for classification, stronger models for reasoning). This wasn't discussed.
> 2. **Detailed cost projections** — The $50K budget was confirmed but per-call costs ($0.002-0.008 per invocation from your research) weren't mapped to expected ticket volume.
> 3. **Fallback to human agents** — No escalation path was defined for when the AI agent can't resolve a ticket. What's the handoff experience?
> 4. **Success metrics beyond resolution rate** — 60% resolution target was set, but customer satisfaction (CSAT), false resolution rate, and agent confidence thresholds weren't discussed.

---

## 7. Research Agent Topics (for the demo)

When Jarvis's Research Agent analyzes the project, it should suggest these topics:

1. **"AWS Bedrock AgentCore customer support use cases"** — find real implementations
2. **"Strands Agents SDK getting started tutorial"** — evaluate the open-source alternative
3. **"Multi-agent orchestration patterns for customer service"** — architecture reference
4. **"LLM agent cost optimization production"** — cost management strategies
5. **"AI agent customer support resolution rate benchmarks"** — industry baselines

---

## 8. Project Summary Output (for Act 4)

When Riya generates a project summary before the call:

> **Agentic AI Platform — Research Synthesis**
>
> **Project Goal:** Build an AI agent platform to automate Tier 1 customer support, targeting 60% autonomous ticket resolution.
>
> **Key Themes Across 8 Sources:**
> - **Agent orchestration is shifting from chains to graphs.** Three sources (re:Invent keynote, orchestration patterns article, Strands SDK demo) converge on graph-based DAG orchestration for production workloads. Parallel execution enables simultaneous knowledge retrieval, order lookup, and customer history checks.
> - **Cost is the primary production concern.** Managed platforms (Bedrock, Azure) carry a 30-40% premium per agent call but eliminate 2-3 FTEs of infrastructure work. At $0.002-0.008 per invocation, a system handling 10K tickets/day costs $600-2,400/month in inference alone.
> - **AWS Bedrock AgentCore offers the fastest path to prototype.** Managed memory, tool execution, multi-agent collaboration, and VPC-contained data flow. Sub-second latency for simple tool calls demonstrated at re:Invent.
> - **Open-source (Strands SDK) offers more flexibility at scale.** Model-agnostic, built-in observability, simple Python tooling. Better long-term cost profile but requires more engineering investment upfront.
>
> **Internal Context:**
> - 68% of current Tier 1 tickets are repetitive (password resets, billing, order status)
> - 12-minute average resolution time — leadership wants this dramatically reduced
> - $50K budget approved for Q1-Q2 prototype
> - Team previously rejected rigid chatbots in favor of flexible AI agents
>
> **Open Questions for Kickoff:**
> 1. Build vs. buy for the orchestration layer?
> 2. Single-model vs. multi-model agent routing?
> 3. What's the escalation path when agents can't resolve?
> 4. Success metrics beyond resolution rate (CSAT, confidence thresholds)?
