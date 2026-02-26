import { useState } from 'react'
import NodeCard from '../shared/NodeCard.tsx'
import CodeSnippet from '../shared/CodeSnippet.tsx'
import SectionHeader from '../shared/SectionHeader.tsx'
import KeyFiles from '../shared/KeyFiles.tsx'

// ── Data ──

const EXTRACTORS = [
  {
    name: 'YouTube',
    icon: '▶️',
    domain: 'youtube.com, youtu.be',
    description: 'Scrapes ytInitialPlayerResponse for video metadata',
    fields: ['title', 'author (channel)', 'description', 'duration', 'image_url (thumbnail)'],
    extraJson: '{\n  "channel_id": "UC...",\n  "view_count": "1234567",\n  "duration_seconds": 842,\n  "keywords": ["rust", "tauri"]\n}',
    file: 'browser/extractors/youtube.rs',
  },
  {
    name: 'Medium',
    icon: '📰',
    domain: 'medium.com, *.medium.com',
    description: 'Extracts article content via Chrome adapter get_tab_html()',
    fields: ['title', 'author', 'description (subtitle)', 'content_excerpt', 'published_date'],
    extraJson: '{\n  "read_time": "5 min read",\n  "claps": 1234,\n  "publication": "Towards Data Science"\n}',
    file: 'browser/extractors/medium.rs',
  },
  {
    name: 'Gmail',
    icon: '📧',
    domain: 'mail.google.com',
    description: 'Extracts email subject, sender, and body from Gmail UI',
    fields: ['title (subject)', 'author (sender)', 'content_excerpt (body preview)'],
    extraJson: '{\n  "sender_email": "user@example.com",\n  "thread_id": "abc123",\n  "labels": ["inbox", "important"]\n}',
    file: 'browser/extractors/gmail.rs',
  },
  {
    name: 'ChatGPT',
    icon: '🤖',
    domain: 'chatgpt.com',
    description: 'Extracts conversation history from ChatGPT web UI',
    fields: ['title (conversation name)', 'content_excerpt (last messages)'],
    extraJson: '{\n  "model": "gpt-4",\n  "message_count": 12,\n  "conversation_id": "abc-123"\n}',
    file: 'browser/extractors/chatgpt.rs',
  },
  {
    name: 'Claude Extension',
    icon: '🧠',
    domain: 'Chrome Extension side panel',
    description: 'Reads Claude Chrome Extension via macOS Accessibility API (AXUIElement)',
    fields: ['title', 'content_excerpt (conversation)'],
    extraJson: '{\n  "extraction_method": "accessibility_api",\n  "panel_type": "side_panel"\n}',
    file: 'browser/extractors/claude_extension.rs',
  },
  {
    name: 'Generic',
    icon: '🌍',
    domain: '* (fallback)',
    description: 'Falls back for any unrecognized domain — extracts basic page metadata',
    fields: ['title', 'description (meta tag)', 'content_excerpt'],
    extraJson: '{}',
    file: 'browser/extractors/generic.rs',
  },
]

const OBSERVER_STEPS = [
  { label: 'Poll Chrome', detail: 'AppleScript: tell application "Google Chrome" to get URL of active tab' },
  { label: 'Classify URL', detail: 'Match domain against extractor registry' },
  { label: 'Detect Change', detail: 'Compare with previous URL — deduplicate by video ID' },
  { label: 'Emit Event', detail: 'youtube-video-detected → frontend notification badge' },
  { label: 'Wait 3s', detail: 'tokio::time::sleep(Duration::from_secs(3))' },
]

// ── Component ──

export default function BrowserExtractorLayer() {
  const [activeExtractor, setActiveExtractor] = useState(EXTRACTORS[0].name)
  const [observerStep, setObserverStep] = useState(0)

  const extractor = EXTRACTORS.find(e => e.name === activeExtractor)!

  return (
    <div className="space-y-10">
      {/* ── Observer Cycle ── */}
      <section>
        <SectionHeader
          title="Browser Observer"
          description="Background polling loop — detects when you open a YouTube video in Chrome"
        />
        <div className="flex items-center gap-2 overflow-x-auto pb-2">
          {OBSERVER_STEPS.map((step, i) => (
            <button
              key={step.label}
              onClick={() => setObserverStep(i)}
              className={`shrink-0 px-4 py-3 rounded-xl border text-xs transition-all ${
                observerStep === i
                  ? 'border-violet-500/40 bg-violet-500/10 text-violet-300 scale-105'
                  : 'border-slate-800 bg-slate-900/40 text-slate-500 hover:border-slate-700'
              }`}
            >
              <p className="font-medium">{step.label}</p>
            </button>
          ))}
        </div>
        <div className="mt-3 p-3 rounded-lg bg-slate-900/40 border border-slate-800/50 text-xs text-slate-400 animate-fade-in min-h-12">
          <span className="text-violet-400 font-medium">Step {observerStep + 1}:</span>{' '}
          {OBSERVER_STEPS[observerStep].detail}
        </div>
        <div className="mt-3">
          <CodeSnippet
            language="rust"
            code={`// browser/observer.rs
loop {
    let url = chrome_adapter.get_active_tab_url().await?;
    if is_youtube(&url) {
        let video_id = extract_video_id(&url);
        if !seen_ids.contains(&video_id) {
            seen_ids.insert(video_id);
            app.emit("youtube-video-detected", &gist)?;
        }
    }
    tokio::time::sleep(Duration::from_secs(3)).await;
}`}
          />
        </div>
      </section>

      {/* ── Extractor Router ── */}
      <section>
        <SectionHeader
          title="Extractor Router"
          description="6 specialized extractors — domain-based routing with PageGist unified output"
        />

        {/* Extractor tabs */}
        <div className="flex flex-wrap gap-1.5 mb-4">
          {EXTRACTORS.map((e) => (
            <button
              key={e.name}
              onClick={() => setActiveExtractor(e.name)}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
                activeExtractor === e.name
                  ? 'text-violet-300 bg-violet-500/10'
                  : 'text-slate-500 hover:text-slate-300 hover:bg-slate-800/50'
              }`}
            >
              <span>{e.icon}</span>
              <span>{e.name}</span>
            </button>
          ))}
        </div>

        {/* Active extractor detail */}
        <div className="animate-fade-in p-4 rounded-xl border border-violet-500/20 bg-violet-500/5">
          <div className="flex items-center gap-2 mb-3">
            <span className="text-2xl">{extractor.icon}</span>
            <div>
              <h4 className="text-sm font-semibold text-violet-300">{extractor.name} Extractor</h4>
              <p className="text-[11px] font-mono text-slate-500">{extractor.domain}</p>
            </div>
          </div>
          <p className="text-xs text-slate-400 mb-4">{extractor.description}</p>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {/* Fields populated */}
            <div>
              <p className="text-[10px] text-slate-500 uppercase tracking-wider font-medium mb-2">PageGist fields populated</p>
              <div className="space-y-1">
                {extractor.fields.map((f) => (
                  <div key={f} className="flex items-center gap-2 text-xs">
                    <span className="w-1.5 h-1.5 rounded-full bg-violet-400 shrink-0" />
                    <span className="text-slate-300 font-mono">{f}</span>
                  </div>
                ))}
              </div>
            </div>
            {/* Extra JSON */}
            <div>
              <p className="text-[10px] text-slate-500 uppercase tracking-wider font-medium mb-2">extra: serde_json::Value</p>
              <CodeSnippet code={extractor.extraJson} language="json" />
            </div>
          </div>
          <p className="text-[11px] font-mono text-slate-600 mt-3">{extractor.file}</p>
        </div>
      </section>

      {/* ── PageGist Anatomy ── */}
      <section>
        <SectionHeader
          title="PageGist — Unified Type"
          description="All extractors output the same struct — the universal language of captured content"
        />
        <NodeCard
          title="PageGist"
          icon="📋"
          color="border-violet-500/20"
          defaultExpanded={true}
        >
          <CodeSnippet
            language="rust"
            code={`pub struct PageGist {
    pub url: String,                      // Required — source URL
    pub title: String,                    // Required — page/video/email title
    pub source_type: SourceType,          // YouTube | Article | Chat | Email | Extension | Generic
    pub domain: String,                   // "youtube.com", "medium.com", etc.
    pub author: Option<String>,           // Channel, writer, sender
    pub description: Option<String>,      // Subtitle, meta description
    pub content_excerpt: Option<String>,  // First ~500 chars of content
    pub published_date: Option<String>,   // ISO 8601 if available
    pub image_url: Option<String>,        // Thumbnail, og:image
    pub extra: serde_json::Value,         // Extractor-specific metadata
}`}
          />
        </NodeCard>
      </section>

      <KeyFiles
        color="text-violet-400"
        files={[
          { path: 'src-tauri/src/browser/observer.rs', description: 'Background polling — Chrome active tab detection' },
          { path: 'src-tauri/src/browser/tabs.rs', description: 'Tab enumeration and SourceType classification' },
          { path: 'src-tauri/src/browser/extractors/mod.rs', description: 'Extractor router — domain → extractor mapping' },
          { path: 'src-tauri/src/browser/extractors/youtube.rs', description: 'YouTube scraper — ytInitialPlayerResponse parsing' },
          { path: 'src-tauri/src/browser/extractors/claude_extension.rs', description: 'macOS Accessibility API extraction' },
          { path: 'src-tauri/src/browser/adapters/chrome.rs', description: 'Chrome AppleScript adapter' },
        ]}
      />
    </div>
  )
}
