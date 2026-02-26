import { useState } from 'react'
import NodeCard from '../shared/NodeCard.tsx'
import CodeSnippet from '../shared/CodeSnippet.tsx'
import SectionHeader from '../shared/SectionHeader.tsx'
import KeyFiles from '../shared/KeyFiles.tsx'

// ── Data ──

const GEM_FIELDS = [
  { name: 'id', type: 'String', required: true, description: 'UUID v4 — unique identifier' },
  { name: 'source_type', type: 'String', required: true, description: '"YouTube", "Article", "Chat", "Email", "Extension", "Generic"' },
  { name: 'source_url', type: 'String', required: true, description: 'Unique constraint — prevents duplicate captures' },
  { name: 'domain', type: 'String', required: true, description: '"youtube.com", "medium.com", etc.' },
  { name: 'title', type: 'String', required: true, description: 'Page title, video title, email subject' },
  { name: 'author', type: 'Option<String>', required: false, description: 'Channel name, article writer, email sender' },
  { name: 'description', type: 'Option<String>', required: false, description: 'Subtitle, meta description, preview' },
  { name: 'content', type: 'Option<String>', required: false, description: 'Full or partial content text' },
  { name: 'source_meta', type: 'serde_json::Value', required: true, description: 'Extractor-specific fields (from PageGist.extra)' },
  { name: 'captured_at', type: 'String', required: true, description: 'ISO 8601 timestamp of capture' },
  { name: 'ai_enrichment', type: 'Option<serde_json::Value>', required: false, description: 'AI-generated tags + summary (null if no AI)' },
]

const CRUD_OPERATIONS = [
  {
    name: 'Save (Upsert)',
    icon: '💾',
    description: 'Insert or replace by source_url uniqueness — prevents duplicate gems',
    sql: 'INSERT OR REPLACE INTO gems\n  (id, source_type, source_url, domain, title, author,\n   description, content, source_meta, captured_at, ai_enrichment)\nVALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)',
    rust: 'pub async fn save(&self, gem: &Gem) -> Result<(), String> {\n    self.conn.execute(\n        "INSERT OR REPLACE INTO gems ...",\n        params![gem.id, gem.source_type, gem.source_url, ...],\n    )?;\n    Ok(())\n}',
  },
  {
    name: 'List',
    icon: '📋',
    description: 'Paginated listing ordered by captured_at DESC (newest first)',
    sql: 'SELECT * FROM gems\n  ORDER BY captured_at DESC\n  LIMIT ?1 OFFSET ?2',
    rust: 'pub async fn list(&self, limit: u32, offset: u32) -> Result<Vec<Gem>, String> {\n    let mut stmt = self.conn.prepare("SELECT * FROM gems ORDER BY ...")?;\n    let gems = stmt.query_map(params![limit, offset], |row| { ... })?;\n    Ok(gems)\n}',
  },
  {
    name: 'Search (FTS5)',
    icon: '🔍',
    description: 'Full-text search across title, description, and content via FTS5',
    sql: 'SELECT gems.* FROM gems\n  JOIN gems_fts ON gems.id = gems_fts.rowid\n  WHERE gems_fts MATCH ?1\n  ORDER BY rank',
    rust: 'pub async fn search(&self, query: &str) -> Result<Vec<Gem>, String> {\n    let mut stmt = self.conn.prepare("SELECT ... JOIN gems_fts ...")?;\n    let gems = stmt.query_map(params![query], |row| { ... })?;\n    Ok(gems)\n}',
  },
  {
    name: 'Filter by Tag',
    icon: '🏷️',
    description: 'Exact match on AI-generated tags in the ai_enrichment JSON',
    sql: "SELECT * FROM gems\n  WHERE json_extract(ai_enrichment, '$.tags') LIKE '%\"rust\"%'\n  ORDER BY captured_at DESC",
    rust: 'pub async fn filter_by_tag(&self, tag: &str) -> Result<Vec<Gem>, String> {\n    let pattern = format!("%\\\"{}\\\"%" , tag);\n    let mut stmt = self.conn.prepare("SELECT ... WHERE json_extract ...")?;\n    ...\n}',
  },
  {
    name: 'Delete',
    icon: '🗑️',
    description: 'Remove a gem by its UUID — cascades to FTS5 via trigger',
    sql: 'DELETE FROM gems WHERE id = ?1',
    rust: 'pub async fn delete(&self, id: &str) -> Result<(), String> {\n    self.conn.execute("DELETE FROM gems WHERE id = ?1", params![id])?;\n    Ok(())\n}',
  },
]

const ENRICHMENT_STEPS = [
  { label: 'Gem saved', detail: 'PageGist converted to Gem with UUID + timestamp', active: true },
  { label: 'Check AI', detail: 'intel.check_availability() — is Apple Intelligence enabled?', active: true },
  { label: 'Open session', detail: 'NDJSON: {"command":"open-session"} → session_id', active: true },
  { label: 'Generate tags', detail: 'Send content → receive 3-5 topic tags', active: true },
  { label: 'Summarize', detail: 'Send content → receive one-sentence summary', active: true },
  { label: 'Merge + save', detail: 'ai_enrichment = { tags, summary, provider, enriched_at }', active: true },
]

// ── Component ──

export default function GemsLayer() {
  const [activeCrud, setActiveCrud] = useState(CRUD_OPERATIONS[0].name)
  const [enrichStep, setEnrichStep] = useState(0)
  const [showInsertAnim, setShowInsertAnim] = useState(false)

  const crud = CRUD_OPERATIONS.find(c => c.name === activeCrud)!

  function triggerInsert() {
    setShowInsertAnim(true)
    setTimeout(() => setShowInsertAnim(false), 2000)
  }

  return (
    <div className="space-y-10">
      {/* ── Gem Data Model ── */}
      <section>
        <SectionHeader
          title="Gem Data Model"
          description="12 fields — required fields in yellow, optional in gray"
        />
        <div className="space-y-1">
          {GEM_FIELDS.map((f) => (
            <div
              key={f.name}
              className="flex items-center gap-3 px-3 py-2 rounded-lg bg-slate-900/40 border border-slate-800/50 text-xs hover:bg-slate-800/40 transition-colors group"
            >
              <span className={`w-2 h-2 rounded-full shrink-0 ${f.required ? 'bg-yellow-400' : 'bg-slate-600'}`} />
              <span className="font-mono text-yellow-300 min-w-32">{f.name}</span>
              <span className="font-mono text-slate-500 min-w-36">{f.type}</span>
              <span className="text-slate-400 hidden group-hover:inline">{f.description}</span>
            </div>
          ))}
        </div>

        {/* ai_enrichment example */}
        <div className="mt-4">
          <CodeSnippet
            language="json"
            caption="Example ai_enrichment JSON"
            code={`{
  "tags": ["rust", "ownership", "memory-safety"],
  "summary": "Explains how Rust's ownership model ensures memory safety.",
  "provider": "intelligencekit",
  "enriched_at": "2024-03-15T14:30:22Z"
}`}
          />
        </div>
      </section>

      {/* ── SQLite Schema ── */}
      <section>
        <SectionHeader
          title="SQLite Schema"
          description="Two tables — gems (main) + gems_fts (FTS5 virtual table for full-text search)"
        />

        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          <NodeCard title="gems" icon="📊" color="border-yellow-500/20" badge="main table" badgeColor="text-yellow-500/60" defaultExpanded={true}>
            <CodeSnippet
              language="sql"
              code={`CREATE TABLE gems (
    id TEXT PRIMARY KEY,
    source_type TEXT NOT NULL,
    source_url TEXT NOT NULL UNIQUE,
    domain TEXT NOT NULL,
    title TEXT NOT NULL,
    author TEXT,
    description TEXT,
    content TEXT,
    source_meta TEXT NOT NULL DEFAULT '{}',
    captured_at TEXT NOT NULL,
    ai_enrichment TEXT
);`}
            />
          </NodeCard>
          <NodeCard title="gems_fts" icon="🔍" color="border-yellow-500/20" badge="FTS5 virtual" badgeColor="text-yellow-500/60" defaultExpanded={true}>
            <CodeSnippet
              language="sql"
              code={`-- Full-text search index
CREATE VIRTUAL TABLE gems_fts
  USING fts5(
    title,
    description,
    content,
    content=gems,
    content_rowid=rowid
);

-- Auto-sync triggers
CREATE TRIGGER gems_ai AFTER INSERT ON gems
  BEGIN INSERT INTO gems_fts(...) ... END;
CREATE TRIGGER gems_ad AFTER DELETE ON gems
  BEGIN INSERT INTO gems_fts(...) ... END;`}
            />
          </NodeCard>
        </div>

        {/* Insert animation */}
        <div className="mt-4 text-center">
          <button
            onClick={triggerInsert}
            className="px-4 py-2 rounded-lg bg-yellow-500/10 border border-yellow-500/30 text-yellow-400 text-xs font-medium hover:bg-yellow-500/20 transition-colors"
          >
            Simulate: Insert a gem
          </button>
          {showInsertAnim && (
            <div className="mt-3 animate-fade-in text-xs space-y-1.5">
              <div className="flex items-center justify-center gap-2 text-yellow-400">
                <span>INSERT INTO gems</span>
                <span className="animate-pulse">→</span>
                <span className="text-slate-400">row added</span>
              </div>
              <div className="flex items-center justify-center gap-2 text-yellow-400/70">
                <span>trigger: gems_ai</span>
                <span className="animate-pulse">→</span>
                <span className="text-slate-400">INSERT INTO gems_fts</span>
                <span className="animate-pulse">→</span>
                <span className="text-slate-400">indexed</span>
              </div>
            </div>
          )}
        </div>
      </section>

      {/* ── CRUD Operations ── */}
      <section>
        <SectionHeader
          title="CRUD Operations"
          description="Click an operation to see its SQL query and Rust implementation"
        />

        <div className="flex flex-wrap gap-1.5 mb-4">
          {CRUD_OPERATIONS.map((op) => (
            <button
              key={op.name}
              onClick={() => setActiveCrud(op.name)}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
                activeCrud === op.name
                  ? 'text-yellow-300 bg-yellow-500/10'
                  : 'text-slate-500 hover:text-slate-300 hover:bg-slate-800/50'
              }`}
            >
              <span>{op.icon}</span>
              <span>{op.name}</span>
            </button>
          ))}
        </div>

        <div className="animate-fade-in space-y-3">
          <p className="text-xs text-slate-400">{crud.description}</p>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            <div>
              <p className="text-[10px] text-yellow-400 uppercase tracking-wider font-medium mb-1.5">SQL</p>
              <CodeSnippet code={crud.sql} language="sql" />
            </div>
            <div>
              <p className="text-[10px] text-yellow-400 uppercase tracking-wider font-medium mb-1.5">Rust</p>
              <CodeSnippet code={crud.rust} language="rust" />
            </div>
          </div>
        </div>
      </section>

      {/* ── Enrichment Pipeline ── */}
      <section>
        <SectionHeader
          title="AI Enrichment Pipeline"
          description="Step through the enrichment flow — from gem save to AI-powered tags and summary"
        />
        <div className="flex items-center gap-1.5 overflow-x-auto pb-2">
          {ENRICHMENT_STEPS.map((step, i) => (
            <div key={step.label} className="flex items-center">
              <button
                onClick={() => setEnrichStep(i)}
                className={`shrink-0 px-3 py-2 rounded-lg border text-xs transition-all ${
                  enrichStep === i
                    ? 'border-yellow-500/40 bg-yellow-500/10 text-yellow-300 scale-105'
                    : i <= enrichStep
                    ? 'border-yellow-500/20 bg-yellow-500/5 text-yellow-400/60'
                    : 'border-slate-800 bg-slate-900/40 text-slate-500'
                }`}
              >
                {step.label}
              </button>
              {i < ENRICHMENT_STEPS.length - 1 && (
                <span className={`mx-1 text-xs ${i < enrichStep ? 'text-yellow-500/40' : 'text-slate-700'}`}>→</span>
              )}
            </div>
          ))}
        </div>
        <div className="mt-3 p-3 rounded-lg bg-slate-900/40 border border-slate-800/50 text-xs text-slate-400 animate-fade-in">
          <span className="text-yellow-400 font-medium">Step {enrichStep + 1}:</span>{' '}
          {ENRICHMENT_STEPS[enrichStep].detail}
        </div>
      </section>

      <KeyFiles
        color="text-yellow-400"
        files={[
          { path: 'src-tauri/src/gems/mod.rs', description: 'Module exports' },
          { path: 'src-tauri/src/gems/store.rs', description: 'GemStore trait — swappable storage backend' },
          { path: 'src-tauri/src/gems/sqlite_store.rs', description: 'SQLite + FTS5 implementation with full-text search' },
          { path: '~/.jarvis/gems.db', description: 'SQLite database file on disk' },
        ]}
      />
    </div>
  )
}
