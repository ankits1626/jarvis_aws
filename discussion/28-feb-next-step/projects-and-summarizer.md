# Projects — Gem Organization & Summarizer Agent

> **Depends on:** [Gem Knowledge Files](./gem-knowledge-files.md) → [Gem Search & Knowledge Layer](./gem-search-and-knowledge-layer.md)
>
> The knowledge files generate .md per gem, the search layer indexes them via QMD, and this project system consumes both for gem matching and summarization.

## The Problem

Gems today live in a flat list. As the library grows, there's no way to say "these 12 gems all relate to the same initiative." A user working on something over days/weeks collects recordings, articles, videos, papers, browsing sessions — but nothing ties them together.

Worse: before a follow-up call or meeting, the user has to manually re-read everything to remember where things stand.

---

## Core Concept: Projects as Folders

A **Project** is a named container that groups related gems.

```
Project: "AWS Migration Q1"
├── Call recording (Feb 25) — kickoff with infra team
├── YouTube video (Feb 25) — "ECS vs EKS comparison"
├── Medium article (Feb 26) — "Zero-downtime Postgres migration"
├── Browsing session (Feb 26) — AWS pricing calculator notes
├── Call recording (Feb 27) — vendor demo
└── Paper (Feb 27) — "Cloud Migration Anti-patterns"
```

### Project Properties

| Field | Description |
|---|---|
| `id` | UUID |
| `name` | User-given name ("AWS Migration Q1") |
| `description` | Optional one-liner |
| `objective` | What the user is trying to achieve (agent-captured) |
| `topics` | Key topics/keywords associated with this project (agent-captured) |
| `context` | Any additional context the user provided during setup (agent-captured) |
| `created_at` | Timestamp |
| `updated_at` | Auto-updated when gems added/removed |
| `status` | `active` / `archived` |

### Gem-to-Project Relationship

- A gem can belong to **zero or one** project (simple model, avoids complexity)
- OR: a gem can belong to **multiple projects** (more flexible but adds tagging complexity)
- Unassigned gems remain in a virtual "Inbox" / uncategorized view
- Assigning a gem to a project = setting a `project_id` foreign key on the gem

**Recommendation:** Start with one-to-one (single project per gem). Users can always duplicate a gem if they need it in two places. Keeps the mental model simple — a gem lives in one folder.

---

## Project Setup Agent — Intelligent Project Creation

Projects can start empty, but they don't have to start *blind*. When a user creates a project, Jarvis commissions a **Setup Agent** that interviews the user, builds a project profile, and then mines the existing gem library for relevant matches.

### The Flow

```
User clicks [+ New Project]
        │
        ▼
┌─────────────────────────────────────┐
│  Step 1: Name Your Project          │
│  ┌─────────────────────────────┐    │
│  │ AWS Migration Q1            │    │
│  └─────────────────────────────┘    │
│                          [Next →]   │
└─────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────┐
│  Step 2: Setup Agent Interview      │
│                                     │
│  🤖 "What's the main objective      │
│      of this project?"              │
│  ┌─────────────────────────────┐    │
│  │ Migrate our monolith to AWS │    │
│  │ ECS with zero downtime      │    │
│  └─────────────────────────────┘    │
│                                     │
│  🤖 "What topics or keywords are    │
│      central to this work?"         │
│  ┌─────────────────────────────┐    │
│  │ AWS, ECS, Docker, Postgres, │    │
│  │ migration, infrastructure   │    │
│  └─────────────────────────────┘    │
│                                     │
│  🤖 "Any other context? People      │
│      involved, timeframe, etc?"     │
│  ┌─────────────────────────────┐    │
│  │ Working with infra team,    │    │
│  │ target completion by March  │    │
│  └─────────────────────────────┘    │
│                                     │
│  [Skip] [Create Project →]         │
└─────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────┐
│  Step 3: Gem Suggestions            │
│                                     │
│  Agent scans your 47 existing gems  │
│  and found 8 potential matches:     │
│                                     │
│  ✅ "ECS vs EKS comparison"         │
│     YouTube · Feb 25 · 95% match    │
│     Reason: Directly compares your  │
│     target compute platform         │
│                                     │
│  ✅ "Zero-downtime Postgres migration│
│     Article · Feb 26 · 91% match    │
│     Reason: Covers Postgres         │
│     migration which matches your    │
│     database migration need         │
│                                     │
│  ☐  "Kickoff call with infra team"  │
│     Recording · Feb 25 · 88% match  │
│     Reason: Transcript mentions AWS │
│     migration planning              │
│                                     │
│  ☐  "Kubernetes networking deep dive│
│     Article · Feb 20 · 62% match    │
│     Reason: Related to container    │
│     orchestration but EKS-focused   │
│                                     │
│  ... 4 more suggestions             │
│                                     │
│  [Select All] [Add Selected →]      │
└─────────────────────────────────────┘
        │
        ▼
  Project created with 3 gems
  User can always add more later
```

### How the Setup Agent Works

**Phase 1 — Interview (conversational, 2-4 questions)**

The agent asks structured questions to build a project profile. Questions are not hardcoded — the LLM generates follow-up questions based on previous answers:

| Question | Purpose | Example Answer |
|---|---|---|
| Objective | What is this project trying to achieve? | "Migrate monolith to AWS ECS" |
| Topics | Key subjects, technologies, domains | "AWS, ECS, Docker, Postgres, zero-downtime" |
| Context | People, timeframe, constraints, background | "Working with infra team, March deadline" |
| *(dynamic)* | Agent may ask a follow-up based on answers | "Are you evaluating managed DB options like Aurora?" |

The interview is **skippable** — user can just give a name and go. But the more context the agent gets, the better its gem suggestions will be.

**Phase 2 — Gem Matching (automatic, runs after interview)**

The agent takes the project profile (objective + topics + context) and scores every unassigned gem in the library:

1. **Text matching** — compare project topics/keywords against gem tags, titles, summaries
2. **Semantic matching** — use the LLM to score relevance between the project objective and each gem's content/summary
3. **Temporal hints** — recently created gems may be more relevant than old ones
4. **Source-type weighting** — recordings mentioning project keywords are high-signal (someone was talking about this topic)

Each suggestion includes:
- **Match score** (percentage) — so user understands confidence
- **Reason** (one line) — why the agent thinks this gem belongs, generated by the LLM ("Transcript mentions AWS migration planning with infra team")
- **Pre-selected state** — high-confidence matches (>85%) are pre-checked, lower ones unchecked but visible

**Phase 3 — User Review**

The user sees the ranked suggestions and can:
- Accept suggestions (checkboxes, pre-checked for high confidence)
- Reject suggestions (uncheck)
- Select all / deselect all
- Manually search for gems not suggested (search bar within the dialog)
- Skip entirely and add gems later

### Ongoing Suggestions (Post-Creation)

The setup agent doesn't just run once. After project creation, whenever a **new gem is saved**, the agent can:

1. Check if the new gem matches any active project's profile
2. Surface a lightweight notification: *"This gem looks like it belongs in 'AWS Migration Q1'. Add it?"*
3. User taps yes/no — one action, no friction

This is the "smart project assignment" from Future Extensions, but grounded in the project profile the user already provided.

### Data Model Addition for Setup Agent

The project profile captured during the interview is stored directly on the project record (see updated Project Properties above: `objective`, `topics`, `context`). This profile is reused by:
- The gem matching engine (for ongoing suggestions)
- The summarizer agent (for understanding what matters in this project)
- Future calendar integration (for matching meetings to projects)

---

## The Summarizer Agent

### Scenario

> I started a project. Had a kickoff call 3 days ago. Then spent 2 days reading blogs, watching videos, reading papers, browsing docs. Now I'm about to join a follow-up call. **What's the state of things?**

The **Summarizer Agent** answers this by producing a chronological briefing from all gems in a project.

### How It Works

1. User opens a project
2. Clicks "Prepare Summary" (or it auto-triggers before a detected calendar event — future)
3. Agent collects all gems in the project, ordered by `created_at`
4. For each gem, pulls: title, summary, key points, tags, transcript excerpts (if recording)
5. Produces a structured briefing document

### Summarizer Output Format

```markdown
# Project Briefing: AWS Migration Q1
Generated: Feb 28, 2026 | Gems: 6 | Timespan: Feb 25–27

## Timeline

### Feb 25 (Day 1 — Kickoff)
- **Kickoff Call with Infra Team** (recording, 45 min)
  - Decided on ECS over EKS for initial workloads
  - Action item: Ankit to evaluate RDS vs Aurora pricing
  - Open question: VPC peering vs Transit Gateway

- **"ECS vs EKS comparison"** (YouTube, 22 min)
  - ECS simpler for teams without K8s experience
  - EKS better for multi-cloud portability
  - Reinforces the kickoff decision

### Feb 26 (Day 2 — Research)
- **"Zero-downtime Postgres migration"** (article)
  - Key technique: logical replication + blue-green cutover
  - Relevant to our RDS migration path

- **AWS Pricing Calculator** (browsing session)
  - Estimated $2,400/mo for ECS Fargate (current workload)
  - Aurora Serverless v2 ~$800/mo vs RDS ~$450/mo

### Feb 27 (Day 3 — Vendor & Literature)
- **Vendor Demo: CloudEndure** (recording, 30 min)
  - Offers automated server migration
  - Pricing: $X per server
  - Decision: Not needed — our workloads are already containerized

- **"Cloud Migration Anti-patterns"** (paper)
  - Warns against lift-and-shift without re-architecture
  - Recommends strangler fig pattern for monolith decomposition

## Key Decisions So Far
1. ECS over EKS (Day 1)
2. CloudEndure not needed (Day 3)

## Open Items
- RDS vs Aurora pricing evaluation (assigned: Ankit)
- VPC peering vs Transit Gateway (unresolved)

## Suggested Talking Points for Next Call
- Present pricing comparison (RDS vs Aurora)
- Propose strangler fig approach for monolith
- Get decision on networking (peering vs TGW)
```

### Implementation Approach

- Runs locally using the existing MLX LLM provider (same as enrichment/co-pilot)
- Input: concatenation of gem summaries + key points + transcripts (compressed)
- If total context is too large, use a two-pass approach:
  1. Summarize each gem individually (already done during enrichment)
  2. Feed per-gem summaries into a final synthesis prompt
- Output saved as a special "briefing" gem within the project

---

## UX Flow

### Project Management

```
Left Nav:
  [+] New Project
  ─────────────
  Projects
    > AWS Migration Q1  (3 new gems)
    > Product Launch v2
    > Learning Rust
  ─────────────
  Inbox (unassigned gems)
  All Gems
```

- Creating a project: name → agent interview (skippable) → gem suggestions → done
- Adding gems to a project: from gem detail view ("Move to project...") or drag-and-drop
- Quick-assign during gem creation: "Save to project: [dropdown]"
- Ongoing nudges: when saving a new gem, notification if it matches an active project's profile

### Summarizer UX

```
Project: AWS Migration Q1
┌─────────────────────────────────────────┐
│  [Prepare Briefing]   Last: 2h ago      │
├─────────────────────────────────────────┤
│  6 gems · Feb 25–27 · 2 calls, 1 video │
│  1 article, 1 paper, 1 browsing         │
├─────────────────────────────────────────┤
│  Timeline view (gems listed by date)    │
│  ...                                    │
└─────────────────────────────────────────┘
```

---

## Data Model Changes

### New Table: `projects`

```sql
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    objective TEXT,                 -- agent-captured: what the project aims to achieve
    topics TEXT,                    -- agent-captured: comma-separated keywords/topics
    context TEXT,                   -- agent-captured: people, timeframe, constraints
    status TEXT DEFAULT 'active',   -- active | archived
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### Gem Table Addition

```sql
ALTER TABLE gems ADD COLUMN project_id TEXT REFERENCES projects(id);
CREATE INDEX idx_gems_project ON gems(project_id);
```

### New Table: `briefings`

```sql
CREATE TABLE briefings (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    content TEXT NOT NULL,          -- markdown briefing
    gem_count INTEGER,
    timespan_start TEXT,
    timespan_end TEXT,
    created_at TEXT NOT NULL
);
```

---

## Phased Delivery

The full vision has clear dependency layers. Each phase delivers standalone value — you can ship and use it before starting the next.

### Phase 1 — Projects Foundation (no AI)

**Goal:** Users can manually organize gems into projects.

**Scope:**
- `projects` table + `project_id` FK on gems (DB migration)
- Rust CRUD commands: create project, list projects, update project, archive project, delete project
- Assign/unassign gem to project (update gem's `project_id`)
- List gems by project
- Frontend: "Projects" section in left nav, project list view, project detail view (shows gems grouped by date)
- Frontend: "Move to project..." action on gem detail view
- Frontend: "Save to project" dropdown during gem creation
- "Inbox" virtual view for unassigned gems

**What you get:** A working folder system. Manual, but functional. Users can start organizing immediately.

**Backend deliverables:**
| Command | Description |
|---|---|
| `create_project` | name, description → project |
| `list_projects` | → all projects with gem counts |
| `get_project` | id → project + its gems |
| `update_project` | id, fields → updated project |
| `archive_project` | id → set status=archived |
| `delete_project` | id → delete (unassign gems, don't delete them) |
| `assign_gem_to_project` | gem_id, project_id → update gem |
| `unassign_gem_from_project` | gem_id → clear project_id |

**Frontend deliverables:**
| Component | Description |
|---|---|
| `ProjectList` | Left nav section listing projects with gem counts |
| `ProjectDetailView` | Center panel showing project gems grouped by date |
| `ProjectCreateModal` | Simple form: name + description |
| `AssignToProjectDropdown` | Reusable dropdown on gem detail + gem save flow |
| `InboxView` | Filtered gem list where project_id is null |

---

### Phase 2 — Summarizer Agent

**Goal:** One-click briefing for any project. The headline feature.

**Depends on:** Phase 1 (needs projects with gems in them)

**Scope:**
- `briefings` table (DB migration)
- Summarizer agent: collects project gems → feeds to LLM → produces chronological briefing
- Two-pass strategy for large projects (per-gem summaries → synthesis)
- Rust command: `generate_briefing(project_id)` → briefing markdown
- Rust command: `get_briefings(project_id)` → list past briefings
- Frontend: "Prepare Briefing" button on project detail view
- Frontend: briefing display panel (rendered markdown)
- Frontend: briefing history (past briefings with timestamps)

**What you get:** The core value loop — collect knowledge over days, get a structured catch-up before your next meeting.

**Why Phase 2 and not Phase 3:** This is the user's original pain point. You can manually assign 10 gems to a project in Phase 1 and immediately get a briefing. The setup agent (Phase 3) reduces friction, but the summarizer delivers the value.

---

### Phase 3 — Setup Agent (Intelligent Project Creation)

**Goal:** AI-assisted project creation with interview + gem suggestions.

**Depends on:** Phase 1 (projects exist), existing AI enrichment pipeline (LLM provider)

**Scope:**
- Add `objective`, `topics`, `context` columns to `projects` table (DB migration)
- Setup agent: interview flow (2-4 LLM-generated questions)
- Gem matching engine: score existing gems against project profile (text + semantic + temporal)
- Rust commands: `setup_agent_interview(project_id, answers)`, `suggest_gems_for_project(project_id)`
- Frontend: multi-step project creation wizard (name → interview → suggestions → done)
- Frontend: gem suggestion list with match scores, reasons, checkboxes
- Interview is skippable — falls back to Phase 1's simple create flow

**What you get:** Projects that start smart instead of empty. Lower friction for organizing existing gems into a new project.

**Backend deliverables:**
| Command | Description |
|---|---|
| `start_project_interview` | project_id → first question from LLM |
| `submit_interview_answer` | project_id, answer → next question or completion |
| `suggest_gems_for_project` | project_id → ranked gem suggestions with scores + reasons |
| `bulk_assign_gems` | project_id, gem_ids[] → assign multiple gems at once |

---

### Phase 4 — Ongoing Intelligence (Polish)

**Goal:** Jarvis proactively helps keep projects organized and up-to-date.

**Depends on:** Phase 3 (needs project profiles for matching)

**Scope:**
- **New gem nudge:** when a gem is saved, check against active project profiles, surface notification if match found
- **Delta briefings:** "What's new since last briefing?" instead of full regeneration
- **Profile refinement:** agent suggests updating project topics/objective as more gems are added
- **Quick-assign from notification:** one-tap "Add to project" from the nudge

**What you get:** A living system that maintains itself. Projects stay organized without manual effort.

---

### Phase Summary

```
Phase 1: Projects Foundation          Phase 2: Summarizer Agent
┌──────────────────────────┐          ┌──────────────────────────┐
│ • projects table         │          │ • briefings table        │
│ • CRUD commands          │──────────│ • LLM summarization      │
│ • assign/unassign gems   │          │ • two-pass strategy      │
│ • project nav + views    │          │ • briefing UI            │
│ • inbox view             │          │                          │
└──────────────────────────┘          └──────────────────────────┘
         │                                       │
         ▼                                       │
Phase 3: Setup Agent                             │
┌──────────────────────────┐                     │
│ • interview flow         │                     │
│ • gem matching engine    │                     │
│ • project wizard UI      │                     │
│ • objective/topics/ctx   │                     │
└──────────────────────────┘                     │
         │                                       │
         ▼                                       │
Phase 4: Ongoing Intelligence ◄──────────────────┘
┌──────────────────────────┐
│ • new gem nudges         │
│ • delta briefings        │
│ • profile refinement     │
│ • auto-organize          │
└──────────────────────────┘
```

Each phase is independently shippable. A user on Phase 1 alone already has a useful project system. Phase 2 makes it powerful. Phase 3 makes it smart. Phase 4 makes it effortless.

---

## Future Extensions

1. **Calendar integration** — detect upcoming meetings, auto-trigger briefing for matching project
2. **Project templates** — pre-defined interview scripts for common patterns (e.g., "Research Sprint" asks about hypothesis, "Client Engagement" asks about stakeholders and deliverables, "Learning Path" asks about skill goals)
3. **Shared projects** — export a project + briefing as a portable package for team sharing
4. **Delta briefings** — "What's new since the last briefing?" instead of full re-summarization
5. **Project-level search** — search within a project's gems only
6. **Project profile refinement** — as more gems are added, agent suggests updating the objective/topics ("You've been researching Aurora a lot — should I add 'Aurora' to this project's topics?")
7. **Cross-project detection** — agent notices a gem is relevant to multiple projects and suggests linking

---

## Open Questions

1. **One-to-one vs many-to-many?** Can a gem belong to multiple projects? Simpler to start with one-to-one.
2. **Auto-grouping?** Should Jarvis suggest project groupings based on temporal/topical clustering?
3. **Briefing freshness** — regenerate every time or cache and show "stale" indicator?
4. **Gem ordering within project** — strictly chronological or allow manual reordering?
5. **Project-scoped tags** — should tags be global or can projects have their own tag namespace?
6. **Interview depth** — fixed 3 questions or let the agent ask dynamic follow-ups? More questions = better matching but more friction.
7. **Suggestion threshold** — what's the minimum match score to show a gem suggestion? Too low = noise, too high = missed gems.
8. **Ongoing suggestion frequency** — nudge on every new gem save, or batch suggestions daily?
9. **Profile evolution** — should the project profile auto-update as gems are added, or stay fixed to the original interview?
