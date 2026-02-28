# Co-Pilot Agent UX — Analysis & Redesign Direction

## The Problem

The current Co-Pilot panel dumps **everything at once** in the right panel: summary, key points, open questions, decisions, action items, suggested questions, key concepts, and a status footer. All sections are visible simultaneously, all update on every cycle, and there's no hierarchy or progressive disclosure.

During a live recording while watching a YouTube video (or in a real meeting), the user glances at the Co-Pilot tab and is hit with a wall of text across 7+ categories. The cognitive load is too high. You can't tell at a glance:
- What changed since the last cycle?
- What's the one most important insight right now?
- What requires my attention vs. what's just context?

---

## Current UI Inventory

What the Co-Pilot panel currently shows (all at once, all the time):

| Section | Update Pattern | User Value |
|---------|---------------|------------|
| Summary | Replaces every cycle | High — the "what's happening" |
| Key Points | Accumulates, deduplicates | Medium — but grows large fast |
| Open Questions | Accumulates | Medium |
| Decisions & Action Items | Accumulates | High — but rare in most conversations |
| Suggested Questions | Replaces (max 5) | Contextual — useful at specific moments |
| Key Concepts | Accumulates with mention count | Low during live — more useful post-session |
| Status Footer | Updates every cycle | Low — cycle number isn't actionable |

**Problem 1: No hierarchy.** Summary is as visually prominent as key concepts. There's no "most important thing right now" signal.

**Problem 2: No change indication.** When cycle 3 runs, you can't tell what's new vs. what was there in cycle 2. Everything looks the same.

**Problem 3: Too many categories.** 7 sections with bullet lists is a meeting notes document, not a live assistant. Users don't read documents while having a conversation.

**Problem 4: No "glanceability."** The panel requires focused reading to extract value. A live assistant should be glanceable — you look for 1-2 seconds and understand the state.

---

## What the Best Products Do

### Granola — "Invisible AI" (70% retention rate)
- Users write their own notes during the meeting
- AI silently augments notes with transcript context **afterward**
- The user is never shown raw AI output during the meeting
- Philosophy: "The best AI products feel like enhanced versions of tools users already understand"

**Takeaway**: Maybe the Co-Pilot shouldn't show everything live. Show the minimum during recording, save the rest for the gem.

### Fireflies — Dynamic Topic Cards
- Instead of dumping all categories, shows **topic cards** that appear as topics are discussed
- Each card is a self-contained unit: topic name + 1-line summary
- Click to expand for details
- "Catch up (Last 1 min)" button for quick context

**Takeaway**: Cards > bullet lists. Each card is one insight, not a category of insights.

### Microsoft Copilot in Teams — Template-Based
- Lets users choose the **output format** before the meeting starts
- "Speaker Summary", "Executive Summary", "Custom Template"
- Different meeting types produce different outputs
- Not everything is shown — the template filters what matters

**Takeaway**: One-size-fits-all doesn't work. A standup meeting needs different Co-Pilot output than a lecture.

### Otter.ai — Hierarchical: Summary > Transcript > Chat
- Three distinct levels of detail, not shown simultaneously
- Summary is always the primary view
- Transcript available on demand
- Chat for deep queries

**Takeaway**: The summary should be the hero. Everything else is secondary.

---

## Proposed Redesign Direction

### Principle: "One thing at a time, everything on demand"

The Co-Pilot panel should feel like a **smart status bar** during recording, not a **document viewer**.

### Option A: "The Glanceable Summary" (Minimal)

```
┌─────────────────────────────┐
│  Co-Pilot                   │
│                             │
│  "They're discussing the    │
│   challenges of dark web    │
│   scraping and why          │
│   traditional antivirus     │
│   tools aren't enough."     │
│                             │
│  ┌─────┐ ┌─────┐ ┌───────┐ │
│  │ 3 pts│ │ 1 ? │ │2 ideas│ │
│  └─────┘ └─────┘ └───────┘ │
│                             │
│  Cycle 3 · 12s ago          │
└─────────────────────────────┘
```

- **Hero**: Running summary in plain text (2-3 sentences max)
- **Counters**: Small pill badges showing counts (key points: 3, questions: 1, concepts: 2)
- **Click any counter** → expands inline to show that category
- **Status**: Minimal, just cycle + time
- The summary is the only thing that **demands** reading
- Everything else is **available** but not **visible**

### Option B: "The Feed" (Timeline)

```
┌─────────────────────────────┐
│  Co-Pilot                   │
│                             │
│  ● 4:38 — New insight       │
│  "Exploits are sold on      │
│   dark web marketplaces"    │
│   [Key Point]               │
│                             │
│  ● 4:37 — Summary updated   │
│  "Discussion shifted to     │
│   cyber threats..."         │
│                             │
│  ● 4:36 — Question raised   │
│  "How do traditional AV     │
│   tools compare?"           │
│                             │
│  Cycle 3 · processing...    │
└─────────────────────────────┘
```

- Reverse-chronological feed of **individual insights**, not categories
- Each insight is tagged (Key Point, Decision, Question, etc.)
- Natural scroll — latest at top
- Change is obvious — new items appear at the top
- No "wall of text" — each item is 1-2 lines

### Option C: "The Card Stack" (Fireflies-inspired)

```
┌─────────────────────────────┐
│  Co-Pilot                   │
│                             │
│  ┌─────────────────────────┐│
│  │ Cyber Threats         ▼ ││
│  │ Discussion about dark   ││
│  │ web exploits and tools  ││
│  └─────────────────────────┘│
│                             │
│  ┌─────────────────────────┐│
│  │ Key Decision          ▼ ││
│  │ Traditional AV isn't    ││
│  │ sufficient for threats  ││
│  └─────────────────────────┘│
│                             │
│  ┌─────────────────────────┐│
│  │ 💡 Ask about...       ▼ ││
│  │ What alternatives to    ││
│  │ antivirus exist?        ││
│  └─────────────────────────┘│
│                             │
│  Cycle 3 · 12s ago          │
└─────────────────────────────┘
```

- Each card is one insight (not one category)
- Cards can be collapsed (▼) to just the title
- New cards animate in
- Cards can be dismissed (swipe or X)
- Color-coded by type (blue = insight, green = decision, orange = question)

---

## Recommendation

**Option A for v1** — it's the simplest change with the biggest impact:

1. Make the summary the hero (large, readable, always visible)
2. Collapse everything else behind counter badges
3. Show what changed (highlight new items, "NEW" badge on counters)
4. Keep the full detail view for the gem (after recording stops)

**Why not B or C?** They require more architectural changes (event stream for B, individual card state management for C). Option A can be done by reshuffling the existing CoPilotPanel component — same data, different presentation.

---

## Additional UX Improvements (Any Option)

### 1. "What's New" Indicator
When a cycle completes, briefly highlight what changed:
- New key point? Flash the counter
- Summary updated? Subtle background pulse
- New question? Show a brief toast or dot

### 2. Reduce Categories During Live
During recording, show only:
- Summary (always)
- Key Points (counter)
- Open Questions (counter)

Save these for the gem (post-recording):
- Key Concepts
- Decisions & Action Items (unless they actually appear)
- Suggested Questions

### 3. "Catch Up" Button
If the user switches to the transcript tab and comes back, show a "Catch up" button that briefly explains what changed while they were away.

### 4. Calm Mode Toggle
Option to hide the Co-Pilot panel entirely and just show a small floating badge:
```
┌──────────┐
│ 🤖 3 new │
└──────────┘
```
Click to expand when you have a moment.

---

## Post-Recording: The Gem View

The gem detail panel (GemDetailPanel) is the right place for the full document view. After recording:
- Show all categories expanded
- Show the complete analysis
- This is where users **review**, not **glance**
- The cognitive load is appropriate because they're no longer multitasking

The current GemDetailPanel copilot section is actually fine for this — it's the **live panel** that needs the redesign.

---

## Open Questions for Discussion

1. **Should the summary auto-scroll or be static?** If it replaces every cycle, does the user lose context from the previous summary?

2. **Should we show the transcript and co-pilot side by side?** Instead of tabs, maybe the transcript goes left and co-pilot goes right (split the right panel).

3. **Should there be a "meeting type" selector?** Different contexts (lecture, meeting, interview) might benefit from different analysis emphasis.

4. **How do we handle the first cycle?** The model often produces poor output on the first 30s. Should we suppress display until cycle 2?

5. **Should suggested questions be shown as a separate overlay/popover** rather than inline? They're actionable (copy to clipboard) and time-sensitive.
