import { useState } from 'react'
import CodeSnippet from '../shared/CodeSnippet.tsx'
import SectionHeader from '../shared/SectionHeader.tsx'
import KeyFiles from '../shared/KeyFiles.tsx'

// ── Data ──

const NDJSON_COMMANDS = [
  {
    name: 'check-availability',
    description: 'Verify Apple Intelligence is enabled on this Mac',
    request: '{"command":"check-availability"}',
    response: '{"ok":true,"available":true}',
  },
  {
    name: 'open-session',
    description: 'Start a new inference session with the 3B model',
    request: '{"command":"open-session"}',
    response: '{"ok":true,"session_id":"550e8400-e29b-41d4-a716-446655440000"}',
  },
  {
    name: 'message (tags)',
    description: 'Generate 3-5 topic tags from content',
    request: '{"command":"message","session_id":"550e8400-...","prompt":"Generate 3-5 topic tags for this content.","content":"Article about Rust ownership...","output_format":"string_list"}',
    response: '{"ok":true,"result":["rust","ownership","memory-safety","systems-programming"]}',
  },
  {
    name: 'message (summary)',
    description: 'Generate a one-sentence summary',
    request: '{"command":"message","session_id":"550e8400-...","prompt":"Summarize in one sentence.","content":"Article about Rust ownership...","output_format":"text"}',
    response: `{"ok":true,"result":"This article explains how Rust's ownership model ensures memory safety without garbage collection."}`,
  },
  {
    name: 'close-session',
    description: 'End the inference session and free resources',
    request: '{"command":"close-session","session_id":"550e8400-..."}',
    response: '{"ok":true}',
  },
]

const PROVIDERS = [
  {
    name: 'IntelligenceKitProvider',
    status: 'active',
    description: 'Spawns IntelligenceKit Swift sidecar, communicates via NDJSON over stdin/stdout',
    tech: 'Apple Foundation Models · 3B parameter · on-device',
    requirements: 'macOS 26+, Apple Silicon, Apple Intelligence enabled',
    color: 'border-rose-500/40 bg-rose-500/5',
    textColor: 'text-rose-400',
  },
  {
    name: 'NoOpProvider',
    status: 'fallback',
    description: 'Returns "unavailable" for all operations — used when AI is not available',
    tech: 'No dependencies',
    requirements: 'Always available (it does nothing)',
    color: 'border-slate-600 bg-slate-800/30',
    textColor: 'text-slate-400',
  },
  {
    name: 'MLXProvider',
    status: 'planned',
    description: 'Python MLX sidecar for local LLM inference — Qwen 3 (4B/8B), Llama 3.2 (3B)',
    tech: 'Python · MLX · Apple Silicon GPU',
    requirements: 'macOS, Apple Silicon, Python 3.11+',
    color: 'border-slate-700 bg-slate-900/30',
    textColor: 'text-slate-500',
  },
]

// ── Component ──

export default function IntelligenceLayer() {
  const [activeCommand, setActiveCommand] = useState(0)
  const [aiAvailable, setAiAvailable] = useState(true)

  return (
    <div className="space-y-10">
      {/* ── Provider Architecture ── */}
      <section>
        <SectionHeader
          title="Provider Architecture"
          description="trait IntelProvider → swappable implementations with graceful fallback"
        />

        {/* Trait definition */}
        <div className="mb-4">
          <CodeSnippet
            language="rust"
            code={`// intelligence/provider.rs
#[async_trait]
pub trait IntelProvider: Send + Sync {
    async fn check_availability(&self) -> Result<bool, String>;
    async fn generate_tags(&self, content: &str) -> Result<Vec<String>, String>;
    async fn summarize(&self, content: &str) -> Result<String, String>;
}`}
          />
        </div>

        {/* Provider cards */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
          {PROVIDERS.map((p) => (
            <div key={p.name} className={`p-4 rounded-xl border ${p.color} transition-all`}>
              <div className="flex items-center gap-2 mb-2">
                <span className={`text-sm font-semibold ${p.textColor}`}>{p.name}</span>
                <span className={`text-[10px] px-1.5 py-0.5 rounded-full border ${
                  p.status === 'active' ? 'border-rose-500/30 text-rose-400 bg-rose-500/10' :
                  p.status === 'planned' ? 'border-slate-600 text-slate-500 bg-slate-800' :
                  'border-slate-600 text-slate-500 bg-slate-800'
                }`}>
                  {p.status}
                </span>
              </div>
              <p className="text-xs text-slate-400 mb-3">{p.description}</p>
              <p className="text-[11px] text-slate-500"><span className="text-slate-600">Tech:</span> {p.tech}</p>
              <p className="text-[11px] text-slate-500 mt-1"><span className="text-slate-600">Requires:</span> {p.requirements}</p>
            </div>
          ))}
        </div>
      </section>

      {/* ── NDJSON Protocol ── */}
      <section>
        <SectionHeader
          title="NDJSON Protocol"
          description="Step through the request/response cycle — click each command to see the JSON exchange"
        />

        <div className="flex flex-wrap gap-1.5 mb-4">
          {NDJSON_COMMANDS.map((cmd, i) => (
            <button
              key={cmd.name}
              onClick={() => setActiveCommand(i)}
              className={`px-3 py-1.5 rounded-lg text-xs font-mono transition-colors ${
                activeCommand === i
                  ? 'text-rose-300 bg-rose-500/10'
                  : 'text-slate-500 hover:text-slate-300 hover:bg-slate-800/50'
              }`}
            >
              {cmd.name}
            </button>
          ))}
        </div>

        <div className="animate-fade-in space-y-3">
          <p className="text-xs text-slate-400">{NDJSON_COMMANDS[activeCommand].description}</p>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            <div>
              <p className="text-[10px] text-rose-400 uppercase tracking-wider font-medium mb-1.5">Request (stdin →)</p>
              <CodeSnippet code={NDJSON_COMMANDS[activeCommand].request} language="json" />
            </div>
            <div>
              <p className="text-[10px] text-emerald-400 uppercase tracking-wider font-medium mb-1.5">Response (← stdout)</p>
              <CodeSnippet code={NDJSON_COMMANDS[activeCommand].response} language="json" />
            </div>
          </div>
        </div>

        {/* Step indicator */}
        <div className="flex items-center justify-center gap-1 mt-4">
          {NDJSON_COMMANDS.map((_, i) => (
            <button
              key={i}
              onClick={() => setActiveCommand(i)}
              className={`w-2 h-2 rounded-full transition-colors ${
                activeCommand === i ? 'bg-rose-400' : 'bg-slate-700 hover:bg-slate-600'
              }`}
            />
          ))}
        </div>
      </section>

      {/* ── Graceful Degradation ── */}
      <section>
        <SectionHeader
          title="Graceful Degradation"
          description="Toggle AI availability — see how the system adapts"
        />

        <div className="flex items-center gap-3 mb-4">
          <button
            onClick={() => setAiAvailable(!aiAvailable)}
            className={`relative w-12 h-6 rounded-full transition-colors ${
              aiAvailable ? 'bg-rose-500' : 'bg-slate-700'
            }`}
          >
            <span className={`absolute top-0.5 w-5 h-5 rounded-full bg-white transition-transform ${
              aiAvailable ? 'left-6' : 'left-0.5'
            }`} />
          </button>
          <span className={`text-sm font-medium ${aiAvailable ? 'text-rose-400' : 'text-slate-500'}`}>
            Apple Intelligence {aiAvailable ? 'Enabled' : 'Disabled'}
          </span>
        </div>

        <div className={`p-4 rounded-xl border transition-all ${
          aiAvailable ? 'border-rose-500/20 bg-rose-500/5' : 'border-slate-700 bg-slate-800/30'
        }`}>
          {aiAvailable ? (
            <div className="space-y-2 text-xs animate-fade-in">
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-rose-400" />
                <span className="text-slate-300">IntelligenceKitProvider active</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-rose-400" />
                <span className="text-slate-300">Gems auto-enriched on save (tags + summary)</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-rose-400" />
                <span className="text-slate-300">Manual enrich button available in GemsPanel</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-rose-400" />
                <span className="text-slate-300">ai_enrichment JSON populated in gem records</span>
              </div>
            </div>
          ) : (
            <div className="space-y-2 text-xs animate-fade-in">
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-slate-600" />
                <span className="text-slate-400">NoOpProvider active — all AI calls return "unavailable"</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-slate-600" />
                <span className="text-slate-400">Gems saved without enrichment — ai_enrichment is null</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-slate-600" />
                <span className="text-slate-400">App works normally — no errors, no degraded UX</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-slate-600" />
                <span className="text-slate-400">Can enrich later when AI becomes available</span>
              </div>
            </div>
          )}
        </div>
      </section>

      <KeyFiles
        color="text-rose-400"
        files={[
          { path: 'src-tauri/src/intelligence/mod.rs', description: 'Provider factory — checks availability, selects implementation' },
          { path: 'src-tauri/src/intelligence/provider.rs', description: 'IntelProvider trait definition' },
          { path: 'src-tauri/src/intelligence/intelligencekit_provider.rs', description: 'Sidecar spawn, NDJSON communication, session management' },
          { path: 'src-tauri/src/intelligence/noop_provider.rs', description: 'Fallback — returns unavailability for all operations' },
          { path: 'intelligence-kit/', description: 'Swift server — Foundation Models, NDJSON protocol' },
          { path: '.kiro/specs/mlx-intelligence-provider/', description: 'Spec for planned MLX Python sidecar' },
        ]}
      />
    </div>
  )
}
