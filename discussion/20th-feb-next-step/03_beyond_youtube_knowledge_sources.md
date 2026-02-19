# Beyond YouTube: What Else Can JARVIS Observe?

> **Date**: 2026-02-19
> **Context**: JARVIS can now see YouTube videos in Chrome and prepare gists. What other machine activities can augment the knowledge store?

---

## The Big Picture

JARVIS already has two senses:
- **Hearing** — microphone transcription (Whisper/WhisperKit)
- **Seeing (partial)** — YouTube video detection in Chrome

The question: **what else do people do on their machines that produces knowledge worth capturing?**

---

## 1. Content People Consume Daily

### 1.1 Articles & Blog Posts (HIGH VALUE)
- **What**: Medium, Substack, personal blogs, news sites (HN, TechCrunch, Verge)
- **Signal**: User spends >30 seconds reading an article page
- **Extraction**: Title, author, publication date, key paragraphs, estimated read time
- **How**: Same Chrome AppleScript polling. Detect article pages via `<meta property="og:type" content="article">` or URL patterns. Scrape with `readability`-style extraction (extract main content, strip ads/nav)
- **Value**: Builds a reading log + summarized knowledge base of what the user found interesting

### 1.2 Documentation & Reference (HIGH VALUE)
- **What**: MDN, docs.rs, React docs, AWS docs, language references, API docs
- **Signal**: User browses docs pages (URL patterns: `docs.*`, `developer.*`, `*.readthedocs.io`)
- **Extraction**: Section title, code examples, the specific API/concept being looked up
- **How**: URL pattern matching + page title extraction via AppleScript
- **Value**: "JARVIS, what was that React hook I was reading about yesterday?" — instant recall

### 1.3 GitHub Repositories & PRs (HIGH VALUE)
- **What**: GitHub repo pages, pull requests, issues, code files
- **Signal**: URL matches `github.com/{owner}/{repo}`
- **Extraction**: Repo name, description, language, stars. For PRs: title, author, changed files summary. For code files: filename, language
- **How**: GitHub API (`gh api`) or page scraping. GitHub has excellent structured data
- **Value**: Track what repos the user explores, what PRs they review — builds a "tech radar"

### 1.4 Stack Overflow & Dev Forums (MEDIUM VALUE)
- **What**: StackOverflow, ServerFault, Reddit r/programming, Discord dev channels
- **Signal**: URL matches `stackoverflow.com/questions/`
- **Extraction**: Question title, accepted answer, tags
- **How**: SO has structured HTML with clear selectors. API available too
- **Value**: "What errors have I been debugging this week?" — patterns emerge

### 1.5 Research Papers & PDFs (HIGH VALUE)
- **What**: arXiv, Google Scholar, IEEE, conference papers
- **Signal**: URL matches `arxiv.org/abs/`, `scholar.google.com`, or PDF viewer active
- **Extraction**: Title, authors, abstract, arxiv ID
- **How**: arXiv has a clean API. Google Scholar harder (anti-scraping). PDFs need special handling
- **Value**: Track research interests, build a personal paper library with summaries

### 1.6 Social Media / Microblogging (MEDIUM VALUE)
- **What**: Twitter/X threads, LinkedIn posts, Mastodon
- **Signal**: URL matches `x.com/*/status/`, `linkedin.com/posts/`
- **Extraction**: Author, post text, engagement metrics, thread content
- **How**: Twitter embed API or page scraping (fragile). LinkedIn very hostile to scraping
- **Value**: Capture interesting threads/discussions the user engaged with

### 1.7 Podcasts & Audio Content (MEDIUM VALUE)
- **What**: Spotify, Apple Podcasts, podcast web players
- **Signal**: URL matches podcast platforms, or Spotify desktop app is active
- **Extraction**: Podcast name, episode title, show notes
- **How**: Spotify has AppleScript support (`tell application "Spotify" to name of current track`). Web players via URL
- **Value**: Log what the user listens to — correlate with topics they're researching

### 1.8 Online Courses & Learning Platforms (HIGH VALUE)
- **What**: Udemy, Coursera, YouTube playlists (educational), Khan Academy, Pluralsight
- **Signal**: URL patterns for course platforms + time spent
- **Extraction**: Course title, current lesson, platform
- **How**: URL matching + page title
- **Value**: "What courses am I taking? How far am I?" — learning progress tracker

### 1.9 Email & Calendar (LOW-MEDIUM VALUE, PRIVACY SENSITIVE)
- **What**: Gmail, Outlook, Google Calendar
- **Signal**: User is on mail/calendar tabs
- **Extraction**: Meeting titles (not content), email subjects (not body)
- **How**: Very limited via browser — would need dedicated API integration
- **Value**: Context for "what was I working on Tuesday afternoon?"

### 1.10 AI Chat Sessions (HIGH VALUE)
- **What**: ChatGPT, Claude, Gemini, Perplexity
- **Signal**: URL matches `chat.openai.com`, `claude.ai`, `gemini.google.com`, `perplexity.ai`
- **Extraction**: Conversation topic (from page title), platform used
- **How**: URL + title extraction. Content is private and hard to extract
- **Value**: "I asked an AI about X yesterday" — meta-knowledge about your AI usage

---

## 2. Beyond the Browser — Desktop Activity

### 2.1 Active Application Tracking
- **What**: Track which apps are in focus (VS Code, Figma, Slack, Terminal)
- **How**: AppleScript `tell application "System Events" to name of first application process whose frontmost is true`
- **Value**: Time tracking, context switching detection, productivity insights

### 2.2 IDE Context (VS Code, Xcode, IntelliJ)
- **What**: Which file/project is open, language being used
- **How**: Window title often contains filename + project. VS Code extensions could provide deeper integration
- **Value**: "What code was I working on?" — correlate with browser research

### 2.3 Terminal Commands
- **What**: What the user is building/deploying/debugging
- **How**: Shell history integration (read `~/.zsh_history`), or observe Terminal.app window title
- **Value**: Captures the "doing" side — complements the "reading" side from browser

### 2.4 Meeting Context (Zoom, Teams, Google Meet)
- **What**: Active video call detected
- **How**: Check if Zoom/Teams/Meet process is running + window title for meeting name
- **Value**: "I was in a meeting about X when I looked up Y" — temporal correlation

---

## 3. Competitive Landscape

| Product | Approach | Scope | Privacy |
|---------|----------|-------|---------|
| **Microsoft Recall** | Screenshots every few seconds, OCR + AI indexing | Everything on screen | Local-only (Windows) |
| **Rewind AI / Limitless** | Screen recording + audio, AI search | Everything seen/heard | Local-first, optional cloud |
| **Perplexity Comet** | AI browser with tab awareness | Browser only | Cloud-based |
| **ChatGPT Atlas** | AI-native browser with memory | Browser only | Cloud-based |
| **JARVIS (us)** | Targeted observation + structured extraction | Selective, user-controlled | Local-only |

### Our Differentiator
Unlike Recall/Rewind (which record *everything*), JARVIS takes a **targeted approach**:
- Only observe things with clear knowledge value
- Extract structured data (not screenshots)
- User controls what gets observed
- Knowledge is actionable (gists, summaries, correlations), not just searchable screenshots

---

## 4. Proposed Priority for JARVIS

### Phase 1: Browser Knowledge (Next Sprint)
Extend the YouTube observer pattern to other content types:

| Source | Difficulty | Value | Priority |
|--------|-----------|-------|----------|
| Articles/Blog posts | Medium | High | **P0** |
| Documentation pages | Easy | High | **P0** |
| GitHub repos/PRs | Easy | High | **P0** |
| Stack Overflow | Easy | Medium | **P1** |
| arXiv papers | Easy | High | **P1** |

**Architecture**: The `BrowserObserver` already polls Chrome's active tab URL. We just need a `classify_url()` function that routes to different extractors:

```
URL detected
  → youtube.com/watch    → YouTubeExtractor (existing)
  → github.com/*/        → GitHubExtractor (new)
  → arxiv.org/abs/       → ArxivExtractor (new)
  → docs.*/developer.*   → DocsExtractor (new)
  → article-like page    → ArticleExtractor (new)
  → unknown              → GenericPageExtractor (title + URL + timestamp)
```

### Phase 2: Desktop Context (Future)
- Active app tracking (AppleScript — easy)
- IDE file detection (window title — easy)
- Meeting detection (process check — easy)

### Phase 3: Knowledge Graph (Future)
- Connect observations: "User read about React Server Components on docs, then watched a YouTube video about it, then opened a GitHub repo using it"
- Topic clustering and trend detection
- "What have I been learning about this week?" dashboard

---

## 5. Technical Implementation Path

### Minimal Change for Maximum Coverage

The current `BrowserObserver` polls Chrome every 3 seconds and gets the URL. To support all browser knowledge sources:

1. **URL Router** (`browser/router.rs`) — classify URLs and dispatch to extractors
2. **Extractors** (`browser/extractors/`) — one per source type, each returns a `KnowledgeItem`
3. **Knowledge Store** — append-only local JSON/SQLite for all captured items
4. **Frontend** — extend YouTubeSection into a "Knowledge Feed" showing all captured items

### Data Model

```rust
struct KnowledgeItem {
    id: Uuid,
    source_type: SourceType,  // YouTube, Article, GitHub, Docs, ArXiv, etc.
    url: String,
    title: String,
    summary: Option<String>,
    metadata: serde_json::Value,  // source-specific fields
    captured_at: DateTime<Utc>,
    time_spent_seconds: Option<u32>,  // estimated from consecutive polls
}
```

### What We Already Have

- Chrome URL polling (observer.rs) ✅
- HTTP client (reqwest) ✅
- Regex for URL parsing ✅
- Event emission to frontend ✅
- Notification system ✅
- Settings toggle ✅

**We're 70% there.** The observer infrastructure is built. We just need extractors and a knowledge store.

---

## 6. Competition Context (AWS AIdeas)

For the AWS competition, this positions JARVIS uniquely:
- **Not just another chatbot** — JARVIS observes your digital life passively
- **Not a surveillance tool** — targeted, structured, user-controlled knowledge capture
- **Ambient intelligence** — JARVIS learns what you're interested in without you telling it
- **Multimodal understanding** — hearing (transcription) + seeing (browser/desktop) + knowing (knowledge graph)

The narrative: *"JARVIS doesn't just answer questions. It knows what questions you're likely to ask, because it's been watching what you learn."*

---

## Sources

- [JetBrains State of Developer Ecosystem 2025](https://blog.jetbrains.com/research/2025/10/state-of-developer-ecosystem-2025/)
- [Microsoft Recall Overview](https://learn.microsoft.com/en-us/windows/ai/recall/)
- [Rewind AI / Limitless Guide](https://skywork.ai/skypage/en/Rewind-AI-&-Limitless:-The-Ultimate-Guide-to-Your-Digital-Memory/1976181260991655936)
- [Best AI Desktop Recall Tools 2025](https://usefulai.com/tools/ai-desktop-recall)
- [Personal Knowledge Graphs: Survey & Roadmap (arXiv)](https://arxiv.org/pdf/2304.09572)
- [Knowledge Graph Approaches in Education](https://www.mdpi.com/2079-9292/13/13/2537)
- [AI Browsers: Uses & Top Options 2026](https://seraphicsecurity.com/learn/ai-browser/ai-browsers-uses-pros-cons-and-top-10-options-in-2026/)
- [ChatGPT Atlas by OpenAI](https://openai.com/index/introducing-chatgpt-atlas/)
- [Perplexity Comet & AI Browser Rise](https://www.browserless.io/blog/the-rise-of-the-ai-browser-intelligent-web-tools-2025)
- [macOS AppleScript: Get Active Window Title](https://gist.github.com/timpulver/4753750)
- [AppleScript: Get Browser Tab URL](https://gist.github.com/vitorgalvao/5392178)
- [Apple Mac Automation Scripting Guide](https://developer.apple.com/library/archive/documentation/LanguagesUtilities/Conceptual/MacAutomationScriptingGuide/AutomatetheUserInterface.html)
- [Day in the Life of a Software Developer](https://www.techneeds.com/2025/03/30/what-is-a-day-in-the-life-of-a-software-developer-a-comprehensive-overview/)
- [Global Top Websites by Monthly Visits 2025](https://www.statista.com/statistics/1201880/most-visited-websites-worldwide/)
- [Personal Knowledge Management 2025](https://www.glukhov.org/post/2025/07/personal-knowledge-management/)
- [Knowledge Management Trends 2026](https://enterprise-knowledge.com/top-knowledge-management-trends-2026/)
- [Ambient AI Ecosystem 2025](https://medium.com/@webelightsolutions/the-ambient-ai-ecosystem-in-2025-how-wearables-smartphones-voice-assistants-are-transforming-9ef93fff43f2)
- [HN: Build Personal Knowledge Graph from Content You Consume](https://news.ycombinator.com/item?id=36245557)
