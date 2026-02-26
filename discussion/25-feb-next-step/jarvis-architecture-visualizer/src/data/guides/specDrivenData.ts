import type { Guide } from './types'

export const specDrivenGuide: Guide = {
  id: 'guide-spec',
  title: 'Spec-Driven Development 101',
  subtitle: 'Requirements first, code second — how Jarvis builds features without guessing',
  color: 'indigo',
  icon: '📝',

  sections: [
    // ── 1. The Problem ──
    {
      id: 'the-problem',
      title: '1 — The Problem: Building Without a Map',
      content: [
        {
          type: 'text',
          body: `Imagine you're building a house. You could just start nailing boards together and figure it out as you go. Or you could draw a blueprint first.\n\nMost software starts the first way — someone says "add a search feature," a developer jumps into code, and three days later everyone argues about whether it should search titles only or full content too. The feature gets rewritten twice.\n\n**Spec-driven development is the blueprint approach.** Before you write a single line of code, you write down:\n- What the user needs (requirements)\n- How the system will do it (design)\n- What to build first, second, third (tasks)\n\nThis isn't busywork. It's how Jarvis ships features that work the first time.`,
        },
        {
          type: 'concept-card',
          term: 'Spec-Driven Development',
          explanation: 'Write down what you\'re building before you build it. A spec is a blueprint — it describes the WHAT and HOW before any code exists.',
          example: 'Before building the Gems feature, Jarvis wrote 9 requirements, a design doc with architecture diagrams, and 12 implementation tasks — all before touching Rust or TypeScript.',
        },
        {
          type: 'concept-card',
          term: 'Why not just start coding?',
          explanation: 'Without a spec, you discover requirements WHILE coding. That means rewriting, breaking things, and arguing about scope. With a spec, you discover problems on paper where fixes are free.',
          example: '"Should search_gems return full content or a preview?" — Answering this in a spec takes 30 seconds. Answering it after building takes refactoring the API, the frontend, and the tests.',
        },
      ],
    },

    // ── 2. The Three Files ──
    {
      id: 'three-files',
      title: '2 — The Three Files: requirements → design → tasks',
      content: [
        {
          type: 'text',
          body: `Every feature in Jarvis lives in a folder with exactly three files. Think of them as three layers of zoom:\n\n**requirements.md** — The "WHAT" (zoom out)\nWhat does the user need? What should happen when they click a button? This is written from the user's perspective.\n\n**design.md** — The "HOW" (zoom in)\nWhat structs, traits, and components will we build? What's the architecture? This is written from the developer's perspective.\n\n**tasks.md** — The "DO" (hands on keyboard)\nA checklist of implementation steps in dependency order. Each task references which requirements it satisfies.`,
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'req', label: 'requirements.md', icon: '📋', color: '#818cf8' },
            { id: 'des', label: 'design.md', icon: '📐', color: '#6366f1' },
            { id: 'tasks', label: 'tasks.md', icon: '✅', color: '#4f46e5' },
            { id: 'code', label: 'Code', icon: '💻', color: '#22c55e' },
          ],
          connections: [
            { from: 'req', to: 'des', label: 'informs' },
            { from: 'des', to: 'tasks', label: 'breaks into' },
            { from: 'tasks', to: 'code', label: 'guides' },
          ],
        },
        {
          type: 'text',
          body: `In Jarvis, these live at:\n\`\`\`\n.kiro/specs/\n├── jarvis-gems/\n│   ├── requirements.md    ← What the user needs\n│   ├── design.md           ← How we'll build it\n│   └── tasks.md            ← Step-by-step checklist\n├── jarvis-browser-vision/\n│   ├── requirements.md\n│   ├── design.md\n│   └── tasks.md\n├── jarvis-listen/\n│   └── ...\n└── (13 specs total)\n\`\`\`\n\nEvery feature — gems, browser vision, transcription, settings, the Claude extension extractor — has its own spec folder. No exceptions.`,
        },
      ],
    },

    // ── 3. Writing Requirements (The WHAT) ──
    {
      id: 'requirements',
      title: '3 — Writing Requirements: The WHAT',
      content: [
        {
          type: 'text',
          body: `A requirement answers one question: **"What should happen from the user's perspective?"**\n\nEvery requirement in Jarvis follows the same template:\n\n1. **User Story** — One sentence: "As a [who], I want [what], so that [why]"\n2. **Acceptance Criteria** — A numbered list of SHALL/SHALL NOT rules\n\nLet's look at a real requirement from the Jarvis Gems feature:`,
        },
        {
          type: 'code',
          language: 'markdown',
          code: `### Requirement 3: Save Gem

**User Story:** As a JARVIS user, I want to save an extracted
gist as a Gem with one click, so that I can build my personal
knowledge collection.

#### Acceptance Criteria

1. THE System SHALL expose a \`save_gem\` Tauri command that
   accepts a PageGist and persists it via the GemStore trait
2. THE System SHALL generate a UUID v4 as the gem's \`id\`
3. THE System SHALL record the current timestamp as
   \`captured_at\` in ISO 8601 format
4. WHEN a gem with the same \`source_url\` already exists,
   THE System SHALL update (upsert) rather than duplicate
5. THE \`save_gem\` command SHALL return the saved Gem to
   the frontend
6. WHEN the save fails, THE System SHALL return a
   descriptive error message`,
          caption: 'From .kiro/specs/jarvis-gems/requirements.md — Requirement 3',
        },
        {
          type: 'text',
          body: `Notice the pattern:\n\n- **SHALL** = must do this, non-negotiable\n- **WHEN...SHALL** = if this happens, then do that\n- Each criterion is testable — you can check it off when the code does exactly this\n\nThe user story tells you WHY. The acceptance criteria tell you WHAT, precisely. No ambiguity.`,
        },
        {
          type: 'concept-card',
          term: 'User Story',
          explanation: '"As a [role], I want [capability], so that [benefit]." Forces you to think about WHO needs this and WHY — not just what to build.',
          example: '"As a JARVIS user, I want to search my gems by keyword, so that I can quickly find relevant knowledge."',
        },
        {
          type: 'concept-card',
          term: 'Acceptance Criteria',
          explanation: 'Numbered SHALL statements that define exactly what "done" means. If all criteria pass, the requirement is met. No opinions, no grey areas.',
          example: '"THE search SHALL match against title, description, and content fields" — you know exactly which fields to index.',
        },
        {
          type: 'concept-card',
          term: 'SHALL vs SHOULD vs MAY',
          explanation: 'SHALL = mandatory, must implement. SHOULD = recommended, implement if possible. MAY = optional, nice to have. Jarvis uses SHALL for everything critical.',
        },
        {
          type: 'quiz',
          question: 'Why does Requirement 3 say "upsert rather than duplicate" for same source_url?',
          options: [
            'To save disk space',
            'To prevent the user from having two copies of the same article — update the existing one instead',
            'Because SQLite can\'t handle duplicates',
            'To make the code simpler',
          ],
          correctIndex: 1,
          explanation: 'If you save a gem from the same YouTube video twice, you want the latest version — not two entries cluttering your collection. The requirement catches this edge case before anyone writes code.',
        },
      ],
    },

    // ── 4. Writing Design Docs (The HOW) ──
    {
      id: 'design',
      title: '4 — Writing Design Docs: The HOW',
      content: [
        {
          type: 'text',
          body: `Requirements say WHAT. The design doc says HOW.\n\nA design doc translates user-facing requirements into developer-facing architecture. It answers:\n- What structs/types will we create?\n- How do the pieces connect?\n- What's the data flow?\n- What are the error cases?\n\nHere's how the Gems design doc breaks down Requirement 1 (Storage Trait Abstraction):`,
        },
        {
          type: 'code',
          language: 'markdown',
          code: `## Architecture

### Layer Responsibilities

**Frontend Layer**
- GistCard: Displays extracted gist with "Save Gem" button
- GemsPanel: Browsable/searchable list of saved gems

**Command Layer**
- Tauri commands depend on GemStore trait (via Arc<dyn GemStore>)
- Commands map between frontend types (PageGist) and
  storage types (Gem)
- Commands handle UUID generation, timestamp recording

**Storage Abstraction Layer**
- GemStore trait: Defines async interface for all operations
- Gem struct: Backend-agnostic data model

**Implementation Layer**
- SqliteGemStore: Default implementation using SQLite + FTS5`,
          caption: 'From .kiro/specs/jarvis-gems/design.md',
        },
        {
          type: 'text',
          body: `See how the design doc introduces **layers**? The requirement just said "save a gem." The design doc says:\n\n1. The **frontend** calls a Tauri command\n2. The **command** converts a PageGist → Gem and calls a trait method\n3. The **trait** is abstract — any backend can implement it\n4. The **SQLite implementation** is just one option\n\nThis layering means you can swap SQLite for a cloud API later without touching the frontend or commands. The design doc is where you make these decisions — not mid-coding.`,
        },
        {
          type: 'comparison',
          leftLabel: 'requirements.md',
          rightLabel: 'design.md',
          rows: [
            { label: 'Audience', left: 'Product owner / user', right: 'Developer building it' },
            { label: 'Language', left: 'User stories + SHALL rules', right: 'Structs, traits, data flows' },
            { label: 'Answers', left: 'WHAT should happen?', right: 'HOW will we build it?' },
            { label: 'Example', left: '"Save a gem with one click"', right: 'GistCard → save_gem cmd → GemStore trait → SqliteGemStore' },
            { label: 'Changes when...', left: 'User needs change', right: 'Architecture changes' },
          ],
        },
      ],
    },

    // ── 5. Writing Tasks (The DO) ──
    {
      id: 'tasks',
      title: '5 — Writing Tasks: The DO',
      content: [
        {
          type: 'text',
          body: `Tasks are the checklist you follow with your hands on the keyboard. Each task:\n- Describes exactly what to create or change\n- References which requirements it satisfies\n- Is ordered by dependency (build the foundation first)\n\nHere's a real snippet from the Gems tasks:`,
        },
        {
          type: 'code',
          language: 'markdown',
          code: `- [x] 1. Set up Gems module structure and dependencies
  - Create \`src/gems/\` directory with mod.rs, store.rs,
    sqlite_store.rs
  - Add dependencies to Cargo.toml: async-trait, uuid,
    rusqlite, dirs, chrono
  - Export gems module in src/lib.rs
  - _Requirements: 1.1, 1.6, 2.1_

- [x] 2. Implement GemStore trait and core data types
  - [x] 2.1 Define Gem and GemPreview structs in store.rs
  - [x] 2.2 Define GemStore trait in store.rs
    - Add async_trait annotation
    - Define methods: save, get, list, search, delete
    - _Requirements: 1.1, 1.2_

- [x] 3. Implement SqliteGemStore
  - [x] 3.1 Create SqliteGemStore struct
  - [x] 3.2 Implement schema initialization
  - [x] 3.3 Write unit tests for schema
  - [x] 3.4 Implement GemStore::save method`,
          caption: 'From .kiro/specs/jarvis-gems/tasks.md',
        },
        {
          type: 'text',
          body: `Notice the pattern:\n\n- **Dependency order**: You can't implement save (task 3.4) before defining the trait (task 2.2)\n- **Traceability**: Each task says which requirement it fulfills (_Requirements: 1.1, 1.6_)\n- **Checkboxes**: As you finish each task, you check it off. Progress is visible.\n- **Sub-tasks**: Complex tasks break into numbered sub-steps (3.1, 3.2, 3.3...)\n\nWhen you come back to this spec a month later, the tasks file tells you exactly where you left off.`,
        },
        {
          type: 'concept-card',
          term: 'Traceability',
          explanation: 'Every task links back to a requirement number. This means you can trace any line of code back to a user need. Nothing gets built "because it seemed cool" — everything exists for a reason.',
          example: 'Task 3.2 (schema initialization) links to Requirements 2.3 and 2.4. If someone asks "why do we have an FTS5 table?" you can trace it back to Requirement 5: full-text search.',
        },
        {
          type: 'concept-card',
          term: 'Dependency Order',
          explanation: 'Tasks are ordered so each one builds on the previous. You never start a task that depends on something not yet built.',
          example: 'You can\'t implement the GemsPanel UI (task 6) until the Tauri commands exist (task 4) and the SQLite store works (task 3).',
        },
      ],
    },

    // ── 6. The Real Workflow: Gems from Zero ──
    {
      id: 'real-workflow',
      title: '6 — Real Workflow: How Gems Was Built',
      content: [
        {
          type: 'text',
          body: `Let's walk through exactly how the Gems feature went from idea to working code using specs. This is the actual sequence that happened:`,
        },
        {
          type: 'text',
          body: `**Step 1: Write requirements.md**\n\nSomeone said: "I want to save extracted gists so I can find them later."\n\nThat one sentence became 9 requirements:\n- Req 1: Storage trait abstraction (so we can swap backends later)\n- Req 2: SQLite implementation (default backend)\n- Req 3: Save gem\n- Req 4: List gems\n- Req 5: Search gems\n- Req 6: Delete gem\n- Req 7: Save button in Browser Tool\n- Req 8: Gems Panel UI\n- Req 9: Don't break existing features\n\nEach with precise acceptance criteria. This took maybe an hour. It saved days of "wait, should search include content or just titles?" debates later.`,
        },
        {
          type: 'text',
          body: `**Step 2: Write design.md**\n\nThe design doc decided:\n- Use a trait so backends are swappable → GemStore trait\n- SQLite + FTS5 for the default → SqliteGemStore\n- Commands map PageGist → Gem (don't change existing types)\n- Frontend gets a GemsPanel component\n\nKey decision captured in the design: "Commands depend on the GemStore trait via Arc<dyn GemStore>, NOT on any concrete implementation." This one sentence shaped the entire architecture.`,
        },
        {
          type: 'text',
          body: `**Step 3: Write tasks.md**\n\n12 tasks in dependency order. Start with module structure, then trait definition, then SQLite implementation, then Tauri commands, then frontend. Each task references requirements.\n\n**Step 4: Code**\n\nNow — and only now — open the editor. Follow the tasks top to bottom. Check off each one. When you're done, every requirement is satisfied because every task traces back to one.`,
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'idea', label: 'Idea: "save gists"', icon: '💡', color: '#fbbf24' },
            { id: 'reqs', label: '9 Requirements', icon: '📋', color: '#818cf8' },
            { id: 'design', label: 'Architecture', icon: '📐', color: '#6366f1' },
            { id: 'tasks', label: '12 Tasks', icon: '✅', color: '#4f46e5' },
            { id: 'code', label: 'Working Code', icon: '💻', color: '#22c55e' },
            { id: 'ship', label: 'Ship It', icon: '🚀', color: '#10b981' },
          ],
          connections: [
            { from: 'idea', to: 'reqs', label: '~1 hour' },
            { from: 'reqs', to: 'design', label: '~1 hour' },
            { from: 'design', to: 'tasks', label: '~30 min' },
            { from: 'tasks', to: 'code', label: 'follow checklist' },
            { from: 'code', to: 'ship', label: 'all tasks ✓' },
          ],
        },
      ],
    },

    // ── 7. Spec Evolution: v1 → v2 ──
    {
      id: 'spec-evolution',
      title: '7 — Spec Evolution: When Requirements Change',
      content: [
        {
          type: 'text',
          body: `Features evolve. The first version ships, users give feedback, and you need v2. Spec-driven development handles this gracefully — you don't throw away the old spec, you write a new one that references it.\n\nJarvis Browser Vision is the perfect example:`,
        },
        {
          type: 'comparison',
          leftLabel: 'Browser Vision v1',
          rightLabel: 'Browser Vision v2',
          rows: [
            { label: 'Observer start', left: 'Manual — user clicks "Start" in YouTube section', right: 'Auto — starts on app launch, runs in background' },
            { label: 'YouTube detection', left: 'Detect URL, fetch full page HTML (~2-3s)', right: 'Detect URL, quick oEmbed fetch (~200ms), then full page on demand' },
            { label: 'Notification', left: '"YouTube video detected"', right: '"You\'re watching: [Title]. Would you like a gist?"' },
            { label: 'Dedup', left: 'Compare against last URL only', right: 'HashSet of seen video IDs — never re-notify for same video' },
            { label: 'User action', left: 'Switch to YouTube section manually', right: 'Click notification → Jarvis foregrounds with section open' },
            { label: 'Settings', left: 'None', right: 'Toggle observer on/off, persisted to settings.json' },
          ],
        },
        {
          type: 'text',
          body: `Both versions have their own spec folder:\n\`\`\`\n.kiro/specs/\n├── jarvis-browser-vision/       ← v1 spec\n│   ├── requirements.md\n│   ├── design.md\n│   └── tasks.md\n├── jarvis-browser-vision-v2/    ← v2 spec (references v1)\n│   ├── requirements.md\n│   ├── design.md\n│   └── tasks.md\n\`\`\`\n\nThe v2 requirements.md starts by explaining what changed from v1:\n\n> "In v1, the user had to manually open the YouTube section and start the observer. In v2, JARVIS automatically observes Chrome in the background from the moment the app launches."\n\nThis gives anyone reading the spec full context: what existed before, what changed, and why.`,
        },
        {
          type: 'concept-card',
          term: 'Spec Versioning',
          explanation: 'When a feature evolves significantly, create a new spec folder (v2) rather than rewriting v1. This preserves the history of decisions and makes it easy to understand how the feature grew.',
          example: 'jarvis-browser-vision/ stays unchanged as a historical record. jarvis-browser-vision-v2/ describes the evolution. Anyone can read both to understand the full journey.',
        },
      ],
    },

    // ── 8. The Glossary Pattern ──
    {
      id: 'glossary',
      title: '8 — Naming Things: The Glossary',
      content: [
        {
          type: 'text',
          body: `Every Jarvis spec starts with a **Glossary** section. This isn't just decoration — it's one of the most important parts.\n\nWhen you're building software, confusion about names causes real bugs. If one developer calls it a "gist" and another calls it an "extraction" and a third calls it a "summary," the code becomes inconsistent — and eventually, someone uses the wrong variable.\n\nThe glossary establishes the official vocabulary for a feature:`,
        },
        {
          type: 'code',
          language: 'markdown',
          code: `## Glossary

- **Gem**: A persistent knowledge unit extracted from a
  browser source. Contains structured metadata (title,
  author, source type, domain) plus the full extracted
  content.

- **GemStore**: A Rust trait defining the storage interface
  for gems. Operations: save, list, search, delete, get.
  Implementations are swappable.

- **PageGist**: The existing in-memory struct returned by
  extractors. A PageGist becomes a Gem when the user
  saves it.

- **FTS5**: SQLite's full-text search extension. Enables
  fast keyword search across gem titles, descriptions,
  and content.`,
          caption: 'From .kiro/specs/jarvis-gems/requirements.md — Glossary',
        },
        {
          type: 'text',
          body: `Now everyone on the team — including AI tools — knows:\n- A **PageGist** is temporary (in-memory)\n- A **Gem** is persistent (saved to database)\n- A PageGist *becomes* a Gem when saved\n- **GemStore** is the abstract interface, not a concrete database\n\nNo confusion. No "did you mean the gist or the gem?" conversations. The code mirrors the glossary: the struct is literally called \`Gem\`, the trait is literally called \`GemStore\`.`,
        },
        {
          type: 'quiz',
          question: 'Why is the glossary at the TOP of the spec, before requirements?',
          options: [
            'Alphabetical order',
            'So you understand the vocabulary before reading the rules that use it',
            'It\'s optional, doesn\'t matter where it goes',
            'To make the document longer',
          ],
          correctIndex: 1,
          explanation: 'If you read "THE System SHALL expose a save_gem command that accepts a PageGist and persists it via the GemStore trait" without knowing what PageGist or GemStore mean, the requirement is gibberish. The glossary defines the language first.',
        },
      ],
    },

    // ── 9. Try It: Write a Spec ──
    {
      id: 'try-it',
      title: '9 — Try It: Write a Mini Spec',
      content: [
        {
          type: 'text',
          body: `Let's practice. Imagine you're adding a **"Favorites"** feature to Jarvis — users can star their favorite gems for quick access.\n\nHere's your exercise: Write a requirement for the "favorite a gem" action using the pattern you've learned.`,
        },
        {
          type: 'interactive-code',
          language: 'markdown',
          starterCode: `### Requirement: Favorite a Gem

**User Story:** As a ______, I want to ______,
so that ______.

#### Acceptance Criteria

1. THE System SHALL ...
2. WHEN ______, THE System SHALL ...
3. THE ______ command SHALL return ...`,
          solution: `### Requirement: Favorite a Gem

**User Story:** As a JARVIS user, I want to mark a gem
as a favorite, so that I can quickly find my most
important knowledge.

#### Acceptance Criteria

1. THE System SHALL expose a \`toggle_favorite\` Tauri
   command that accepts a gem \`id\` and a \`favorite\`
   boolean
2. WHEN favorite is true, THE System SHALL mark the gem
   as favorited in the GemStore
3. WHEN favorite is false, THE System SHALL remove the
   favorite mark
4. THE \`toggle_favorite\` command SHALL return the
   updated Gem to the frontend
5. THE \`list_gems\` command SHALL accept an optional
   \`favorites_only\` filter parameter`,
          hint: 'Follow the pattern: User Story (who/what/why), then SHALL statements for each behavior. Think about: what command name? what happens when you favorite? what happens when you un-favorite? how do you find favorites later?',
          validator: (input: string) => {
            const lower = input.toLowerCase()
            return lower.includes('user story') &&
                   lower.includes('shall') &&
                   lower.includes('acceptance criteria')
          },
        },
        {
          type: 'text',
          body: `Now try writing a task that implements your requirement:`,
        },
        {
          type: 'interactive-code',
          language: 'markdown',
          starterCode: `## Task: Implement Favorite Gem

- [ ] 1. Add \`favorite\` field to ...
  - _Requirements: ..._

- [ ] 2. Implement \`toggle_favorite\` ...
  - _Requirements: ..._

- [ ] 3. Update frontend ...
  - _Requirements: ..._`,
          solution: `## Task: Implement Favorite Gem

- [ ] 1. Add \`favorite\` field to Gem struct and database
  - Add \`is_favorite: bool\` to Gem struct (default false)
  - Add \`is_favorite\` INTEGER column to gems table
  - Run migration to add column to existing databases
  - _Requirements: 1_

- [ ] 2. Implement toggle_favorite Tauri command
  - Add \`toggle_favorite(id: String, favorite: bool)\`
    command in commands.rs
  - Update the gem's is_favorite field in SqliteGemStore
  - Return the updated Gem
  - _Requirements: 1, 2, 3, 4_

- [ ] 3. Update list_gems to support favorites filter
  - Add optional \`favorites_only: bool\` parameter
  - When true, add WHERE is_favorite = 1 to query
  - _Requirements: 5_

- [ ] 4. Add favorite button to gem card in frontend
  - Add star icon toggle to each gem card
  - Call toggle_favorite on click
  - Add "Favorites" filter toggle to GemsPanel
  - _Requirements: 1, 5_`,
          hint: 'Tasks should be in dependency order: database change first, then backend command, then frontend. Each task should reference which requirement number it satisfies.',
          validator: (input: string) => {
            const lower = input.toLowerCase()
            return lower.includes('requirements') &&
                   (lower.includes('- [') || lower.includes('-[')) &&
                   lower.includes('favorite')
          },
        },
      ],
    },

    // ── 10. All 13 Jarvis Specs ──
    {
      id: 'all-specs',
      title: '10 — The Full Picture: All 13 Jarvis Specs',
      content: [
        {
          type: 'text',
          body: `Every feature in Jarvis was built with a spec. Here's the complete map — 13 spec folders, each with requirements → design → tasks:`,
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'app', label: 'jarvis-app', icon: '🏠', color: '#6366f1' },
            { id: 'listen', label: 'jarvis-listen', icon: '🎤', color: '#f59e0b' },
            { id: 'transcribe', label: 'jarvis-transcribe', icon: '📝', color: '#f59e0b' },
            { id: 'whisper', label: 'jarvis-whisperkit', icon: '🤖', color: '#f59e0b' },
            { id: 'browser', label: 'browser-vision', icon: '🌐', color: '#3b82f6' },
            { id: 'browserv2', label: 'browser-vision-v2', icon: '🌐', color: '#3b82f6' },
            { id: 'gems', label: 'jarvis-gems', icon: '💎', color: '#8b5cf6' },
            { id: 'medium', label: 'medium-extractor', icon: '📰', color: '#10b981' },
            { id: 'claude', label: 'claude-extension', icon: '🧠', color: '#10b981' },
            { id: 'settings', label: 'jarvis-settings', icon: '⚙️', color: '#64748b' },
            { id: 'intel', label: 'intelligence-kit', icon: '🧠', color: '#ec4899' },
            { id: 'inteli', label: 'intel-integration', icon: '🔌', color: '#ec4899' },
            { id: 'mlx', label: 'mlx-intelligence', icon: '🤖', color: '#ec4899' },
          ],
          connections: [
            { from: 'listen', to: 'transcribe', label: 'audio → text' },
            { from: 'transcribe', to: 'whisper', label: 'uses' },
            { from: 'browser', to: 'browserv2', label: 'evolved to' },
            { from: 'browserv2', to: 'gems', label: 'saves to' },
            { from: 'medium', to: 'gems', label: 'saves to' },
            { from: 'claude', to: 'gems', label: 'saves to' },
            { from: 'intel', to: 'inteli', label: 'integrates via' },
            { from: 'inteli', to: 'mlx', label: 'provides' },
          ],
        },
        {
          type: 'text',
          body: `**Audio pipeline**: jarvis-listen → jarvis-transcribe → jarvis-whisperkit\nThree specs that together define how Jarvis captures and transcribes audio.\n\n**Browser pipeline**: browser-vision → browser-vision-v2\nHow Jarvis observes Chrome and detects content. v2 adds auto-start and smart notifications.\n\n**Extractors**: medium-extractor, claude-extension-extractor\nEach extractor has its own spec defining how it parses a specific content source.\n\n**Storage**: jarvis-gems\nThe persistence layer where all extracted knowledge is saved and searched.\n\n**Intelligence**: intelligence-kit → intelligence-kit-integration → mlx-intelligence\nThe AI layer — how Jarvis processes and understands content.\n\n**Core**: jarvis-app, jarvis-settings\nThe application shell and user preferences.\n\nEvery box in this diagram has a folder in \`.kiro/specs/\` with three files. That's 13 features × 3 files = 39 documents guiding the entire codebase.`,
        },
        {
          type: 'quiz',
          question: 'If you wanted to add a "Twitter/X extractor" to Jarvis, what would you do FIRST?',
          options: [
            'Start writing the Rust code for scraping tweets',
            'Create .kiro/specs/jarvis-twitter-extractor/ with requirements.md, design.md, and tasks.md',
            'Ask a designer to mockup the UI',
            'Add a new Tauri command called extract_tweet',
          ],
          correctIndex: 1,
          explanation: 'Spec first, code second. Create the spec folder, write the requirements (what tweets to extract, what metadata to capture), the design (how to scrape, what structs), and the tasks (implementation checklist). Then code.',
        },
      ],
    },
  ],

  jarvisConnections: [
    {
      concept: 'Requirements Document',
      file: '.kiro/specs/jarvis-gems/requirements.md',
      description: 'Real example: 9 requirements with user stories and acceptance criteria for the Gems feature',
    },
    {
      concept: 'Design Document',
      file: '.kiro/specs/jarvis-gems/design.md',
      description: 'Real example: Architecture with trait abstraction, layer diagram, and data models',
    },
    {
      concept: 'Tasks Checklist',
      file: '.kiro/specs/jarvis-gems/tasks.md',
      description: 'Real example: 12 ordered tasks with sub-steps and requirement traceability',
    },
    {
      concept: 'Spec Evolution (v1)',
      file: '.kiro/specs/jarvis-browser-vision/requirements.md',
      description: 'Browser Vision v1: manual observer start, basic YouTube detection',
    },
    {
      concept: 'Spec Evolution (v2)',
      file: '.kiro/specs/jarvis-browser-vision-v2/requirements.md',
      description: 'Browser Vision v2: auto-start, oEmbed, smart notifications — see how specs evolve',
    },
  ],
}
