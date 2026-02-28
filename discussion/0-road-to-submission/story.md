# Jarvis — User Journey Story

**Persona:** Riya, Product Lead at a mid-stage startup
**Setting:** One full workday — from morning email to post-meeting synthesis
**Theme:** Knowledge doesn't disappear. It becomes a Gem.

---

## Act 1: The Spark (8:30 AM)

Riya opens her inbox. There's an email from her VP:

> "Hey Riya — remember that agentic AI platform idea we discussed last month?
> Leadership wants to green-light it. Can you pull together what we know and
> have a kickoff call with me at 4 PM today? Let's move fast."

One email. Eight hours. She needs to go from zero to a credible project brief.

**Jarvis action:** Riya opens Jarvis and captures the email as a **Gem**.
The email is now searchable, tagged, and part of her knowledge base.

---

## Act 2: Project Setup (8:45 AM)

Riya creates a new **Project** in Jarvis: "Agentic AI Platform Research."

Jarvis asks a few setup questions:
- What's this project about?
- What are the key topics?
- Any deadlines?

Based on her answers, Jarvis scans her existing Gems library and recommends:

> **3 relevant Gems found from previous work:**
>
> | Gem | Source | Suitability |
> |-----|--------|-------------|
> | "AWS re:Invent 2025 — Bedrock AgentCore keynote" | YouTube | 92% match |
> | "Multi-Agent Orchestration Patterns" | Medium article | 87% match |
> | "Q4 Strategy Call — AI roadmap discussion" | Recording transcript | 78% match |

Riya selects all three. Months-old knowledge she'd forgotten about — instantly relevant again.

*[Features: Projects (planned), Gem search + relevance scoring (planned)]*

---

## Act 3: Research Sprint (9:00 AM – 12:00 PM)

### Email → Gem
The VP's email is already captured. Jarvis auto-enriches it: tags it
`#agentic-ai`, `#project-kickoff`, summarizes it as
*"Green light for agentic AI platform — kickoff call at 4 PM."*

### AI-Assisted Discovery
Based on the project context, Jarvis suggests research topics:

> **Suggested topics for "Agentic AI Platform Research":**
> - Agent orchestration frameworks (Strands, LangGraph, CrewAI)
> - Tool-use patterns in production agents
> - Cost optimization for multi-agent systems
> - Bedrock AgentCore vs. open-source agent runtimes
>
> **Recommended sources:**
> - [YouTube] "Building Production AI Agents — AWS Summit 2025"
> - [Medium] "The Real Cost of Running Multi-Agent Systems"
> - [Article] "Strands Agents SDK — Getting Started Guide"

*[Feature: AI-powered topic suggestions + web search via Tavily (planned)]*

### YouTube → Gem
Riya opens the suggested YouTube video in Chrome. Jarvis's **background
observer** detects it automatically — a notification badge appears on the
YouTube tab.

She clicks "Capture" — Jarvis extracts the title, channel, duration,
description, and full transcript. Saved as a Gem. Auto-tagged, auto-summarized.

### Article → Gem
She opens the Medium article. Switches to Jarvis's Browser tab — sees all
her open Chrome tabs listed with auto-classified source types. One click:
the article content is extracted via the Readability algorithm and saved as
a Gem.

### More Research
She opens two more articles, watches another short video. Each one → captured
as a Gem in under 5 seconds. No copy-pasting. No Notion. No bookmarks that
die.

**By noon, Riya has 8 Gems in her project** — the original email, 3 old
Gems she'd forgotten about, 2 YouTube videos, and 2 articles. All searchable.
All tagged. All enriched with AI summaries.

*[Features: YouTube auto-detection, Chrome tab listing, content extraction,
Gems library, AI enrichment — ALL BUILT]*

---

## Act 4: Pre-Meeting Preparation (2:00 PM)

The 4 PM call is in two hours. Riya has 8 research Gems but she needs a
coherent briefing, not a pile of bookmarks.

### Synthesis Gem
Riya asks Jarvis to **generate a summary** across all project Gems.

Jarvis creates a **Synthesis Gem** — a new Gem whose content is an AI-generated
overview that pulls together the key themes, contradictions, and open questions
from all 8 sources:

> **Agentic AI Platform — Research Synthesis**
>
> Key themes across 8 sources:
> - Agent orchestration is moving from chains to graphs (3 sources agree)
> - Cost per agent call is the #1 production concern (Medium article + Q4 call)
> - AWS Bedrock AgentCore offers managed memory + tool execution
> - Open-source alternatives (Strands, LangGraph) offer more flexibility
>
> Open questions for kickoff:
> - Build vs. buy for the orchestration layer?
> - Single-model vs. multi-model agent routing?
> - On-device inference for latency-sensitive agents?

*[Feature: Cross-gem synthesis (planned)]*

### Chat with the Synthesis
Riya opens the **Chat** panel on the Synthesis Gem and prepares for the call:

> **Riya:** "What are the strongest arguments for using Bedrock AgentCore
> vs building our own orchestration?"
>
> **Jarvis:** "Based on your research, three sources point to AgentCore's
> managed episodic memory as the key differentiator — it handles session
> persistence without custom infrastructure. However, the Medium article
> notes a 40% cost premium over self-hosted Strands for high-throughput
> workloads. The Q4 strategy call transcript shows your team already has
> concerns about vendor lock-in..."

She asks three more questions. Each answer draws from her actual research,
not generic AI knowledge. The Chat session is logged as a markdown file —
she can reference it during the call.

*[Features: Chat with Gems (planned, trait already built), session logging — BUILT]*

---

## Act 5: The Call (4:00 PM)

Riya joins the call with her VP. She hits **Record** in Jarvis.

### Live Transcription
Words appear in the **Transcript** tab in real-time as they speak.

### Co-Pilot Intelligence
The **Co-Pilot** tab lights up. Every 60 seconds, Jarvis analyzes the raw
audio and surfaces insight cards:

> **[Key Point]** VP confirms budget of $50K for Q1 prototype
>
> **[Decision]** Team will use Bedrock AgentCore for the first iteration
>
> **[Action Item]** Riya to prepare a technical design doc by next Friday
>
> **[Question]** No one addressed: what's the latency requirement for
> agent responses? (Suggested: ask about this)

Cards slide in with animation. Color-coded by type. Auto-collapse after a
few seconds so they don't distract. Riya glances at the suggested question
and asks it — the VP says "under 2 seconds for the first response."

A new card appears: **[Decision]** Latency target: <2s for first agent response.

**The call runs for 35 minutes.** The Co-Pilot captured 4 decisions, 6 action
items, 3 key points, and 2 open questions — in real-time. Riya didn't take
a single note.

*[Features: Recording, live transcription, Co-Pilot agent, card stack UX — ALL BUILT]*

---

## Act 6: Post-Meeting Intelligence (4:45 PM)

The call ends. Riya stops recording.

### Session Summary
A **Session Summary Card** appears — aggregating everything the Co-Pilot
captured across the entire call:

> **Meeting Summary:** 35-minute kickoff for Agentic AI Platform. Budget
> confirmed at $50K. Bedrock AgentCore chosen as foundation. Technical
> design doc due next Friday. Latency target <2s.

### Transcript + Chat
Riya clicks on the recording. The full transcript is available. She opens
**Chat** and does her post-meeting analysis:

> **Riya:** "List every commitment I made during this call"
>
> **Jarvis:** "Based on the transcript, you committed to:
> 1. Prepare a technical design document by next Friday
> 2. Evaluate Strands SDK as a fallback option
> 3. Schedule a follow-up with the engineering lead
> 4. Share the research synthesis document with the VP by EOD"

> **Riya:** "What did we NOT discuss that we should have?"
>
> **Jarvis:** "Based on the open questions from your pre-meeting research,
> these topics were not addressed:
> - Multi-model routing strategy
> - Cost projections for the prototype phase
> - On-device inference for latency-sensitive use cases"

*[Features: Session summary, transcription, chat with recordings — ALL BUILT]*

---

## Act 7: Closing the Loop (5:15 PM)

### Save as Gem
Riya saves the recording as a **Gem** — the transcript, Co-Pilot analysis,
and all metadata are preserved together. It joins the 8 research Gems in
her project.

### The Project Now Has 10 Gems:

| # | Gem | Source | When |
|---|-----|--------|------|
| 1 | VP kickoff email | Gmail | 8:30 AM |
| 2 | AWS re:Invent keynote | YouTube (old) | 3 weeks ago |
| 3 | Multi-Agent Patterns article | Medium (old) | 2 weeks ago |
| 4 | Q4 AI Roadmap call | Recording (old) | Last month |
| 5 | AWS Summit agents video | YouTube | 9:15 AM |
| 6 | Cost of Multi-Agent article | Medium | 10:00 AM |
| 7 | Strands SDK guide | Article | 10:45 AM |
| 8 | Second agents video | YouTube | 11:30 AM |
| 9 | Research synthesis | AI-generated | 2:00 PM |
| 10 | Kickoff call recording | Recording | 4:00 PM |

**From one email at 8:30 AM to a fully documented project with 10 knowledge
artifacts by 5:15 PM.** Every conversation captured. Every source preserved.
Every insight searchable. Nothing forgotten.

---

## The Line That Sells It

> **Other tools transcribe your meetings. Jarvis remembers your entire day.**

---

## Feature Map — What's Built vs Planned

| Feature | Status | Used In |
|---------|--------|---------|
| Email extraction → Gem | BUILT | Act 1 |
| YouTube auto-detection + capture | BUILT | Act 3 |
| Article/Medium extraction | BUILT | Act 3 |
| Chrome tab listing | BUILT | Act 3 |
| AI enrichment (tags, summaries) | BUILT | Act 3 |
| Gems library with FTS5 search | BUILT | Act 2, 3, 7 |
| Audio recording + live transcription | BUILT | Act 5 |
| Co-Pilot (real-time analysis) | BUILT | Act 5 |
| Card stack UX (animated insights) | BUILT | Act 5 |
| Session summary card | BUILT | Act 6 |
| Chat with recordings | BUILT | Act 6 |
| Save recording as Gem | BUILT | Act 7 |
| Projects (grouping Gems) | PLANNED | Act 2 |
| Gem relevance scoring + recommendations | PLANNED | Act 2 |
| AI topic suggestions + web search (Tavily) | PLANNED | Act 3 |
| Cross-gem synthesis | PLANNED | Act 4 |
| Chat with Gems | PLANNED (trait ready) | Act 4 |

**13 of 17 features shown in the story are already built and working.**
