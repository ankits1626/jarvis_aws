import { useState } from 'react'
import NodeCard from '../shared/NodeCard.tsx'
import CodeSnippet from '../shared/CodeSnippet.tsx'
import SectionHeader from '../shared/SectionHeader.tsx'
import KeyFiles from '../shared/KeyFiles.tsx'

// ── Data ──

type CommandCategory = {
  category: string
  color: string
  commands: { name: string; params: string; returns: string; caller: string; snippet: string }[]
}

const COMMAND_CATALOG: CommandCategory[] = [
  {
    category: 'Recording',
    color: 'text-emerald-400',
    commands: [
      { name: 'start_recording', params: '()', returns: 'Result<(), String>', caller: 'RecordButton', snippet: '#[tauri::command]\nasync fn start_recording(\n    rec: State<\'_, Mutex<RecordingManager>>,\n    app: AppHandle,\n) -> Result<(), String> {\n    let mut mgr = rec.lock().unwrap();\n    mgr.start(&app).map_err(|e| e.to_string())\n}' },
      { name: 'stop_recording', params: '()', returns: 'Result<Recording, String>', caller: 'RecordButton', snippet: '#[tauri::command]\nasync fn stop_recording(\n    rec: State<\'_, Mutex<RecordingManager>>,\n) -> Result<Recording, String> { ... }' },
      { name: 'get_recordings', params: '()', returns: 'Result<Vec<Recording>, String>', caller: 'App', snippet: '#[tauri::command]\nfn get_recordings(\n    files: State<\'_, FileManager>,\n) -> Result<Vec<Recording>, String> { ... }' },
      { name: 'delete_recording', params: '(id: String)', returns: 'Result<(), String>', caller: 'RecordingRow', snippet: '#[tauri::command]\nfn delete_recording(\n    id: String,\n    files: State<\'_, FileManager>,\n) -> Result<(), String> { ... }' },
    ],
  },
  {
    category: 'Browser',
    color: 'text-violet-400',
    commands: [
      { name: 'get_browser_tabs', params: '()', returns: 'Result<Vec<BrowserTab>, String>', caller: 'BrowserTool', snippet: '#[tauri::command]\nasync fn get_browser_tabs() -> Result<Vec<BrowserTab>, String> {\n    let adapter = ChromeAdapter::new();\n    adapter.get_tabs().await\n}' },
      { name: 'capture_tab', params: '(url: String)', returns: 'Result<PageGist, String>', caller: 'BrowserTool', snippet: '#[tauri::command]\nasync fn capture_tab(url: String) -> Result<PageGist, String> {\n    let gist = extractors::extract(&url).await?;\n    Ok(gist)\n}' },
      { name: 'fetch_youtube_gist', params: '(url: String)', returns: 'Result<YouTubeGist, String>', caller: 'YouTubeSection', snippet: '#[tauri::command]\nasync fn fetch_youtube_gist(url: String) -> Result<YouTubeGist, String> { ... }' },
      { name: 'start_browser_observer', params: '()', returns: 'Result<(), String>', caller: 'App (on mount)', snippet: '#[tauri::command]\nasync fn start_browser_observer(\n    observer: State<\'_, Arc<tokio::sync::Mutex<BrowserObserver>>>,\n    app: AppHandle,\n) -> Result<(), String> { ... }' },
    ],
  },
  {
    category: 'Gems',
    color: 'text-yellow-400',
    commands: [
      { name: 'save_gem', params: '(gem: GemInput)', returns: 'Result<Gem, String>', caller: 'BrowserTool', snippet: '#[tauri::command]\nasync fn save_gem(\n    input: GemInput,\n    store: State<\'_, Arc<dyn GemStore>>,\n    intel: State<\'_, Arc<dyn IntelProvider>>,\n) -> Result<Gem, String> { ... }' },
      { name: 'list_gems', params: '(limit: u32, offset: u32)', returns: 'Result<Vec<Gem>, String>', caller: 'GemsPanel', snippet: '#[tauri::command]\nasync fn list_gems(\n    limit: u32, offset: u32,\n    store: State<\'_, Arc<dyn GemStore>>,\n) -> Result<Vec<Gem>, String> { ... }' },
      { name: 'search_gems', params: '(query: String)', returns: 'Result<Vec<Gem>, String>', caller: 'GemsPanel', snippet: '#[tauri::command]\nasync fn search_gems(\n    query: String,\n    store: State<\'_, Arc<dyn GemStore>>,\n) -> Result<Vec<Gem>, String> { ... }' },
      { name: 'delete_gem', params: '(id: String)', returns: 'Result<(), String>', caller: 'GemsPanel', snippet: '#[tauri::command]\nasync fn delete_gem(\n    id: String,\n    store: State<\'_, Arc<dyn GemStore>>,\n) -> Result<(), String> { ... }' },
    ],
  },
  {
    category: 'Settings',
    color: 'text-slate-400',
    commands: [
      { name: 'get_settings', params: '()', returns: 'Result<Settings, String>', caller: 'Settings', snippet: '#[tauri::command]\nfn get_settings(\n    mgr: State<\'_, Arc<RwLock<SettingsManager>>>,\n) -> Result<Settings, String> { ... }' },
      { name: 'update_settings', params: '(settings: Settings)', returns: 'Result<(), String>', caller: 'Settings', snippet: '#[tauri::command]\nfn update_settings(\n    settings: Settings,\n    mgr: State<\'_, Arc<RwLock<SettingsManager>>>,\n) -> Result<(), String> { ... }' },
    ],
  },
  {
    category: 'Intelligence',
    color: 'text-rose-400',
    commands: [
      { name: 'check_intelligence', params: '()', returns: 'Result<bool, String>', caller: 'App', snippet: '#[tauri::command]\nasync fn check_intelligence(\n    intel: State<\'_, Arc<dyn IntelProvider>>,\n) -> Result<bool, String> {\n    intel.check_availability().await\n}' },
      { name: 'enrich_gem', params: '(id: String)', returns: 'Result<Gem, String>', caller: 'GemsPanel', snippet: '#[tauri::command]\nasync fn enrich_gem(\n    id: String,\n    store: State<\'_, Arc<dyn GemStore>>,\n    intel: State<\'_, Arc<dyn IntelProvider>>,\n) -> Result<Gem, String> { ... }' },
    ],
  },
  {
    category: 'Models',
    color: 'text-blue-400',
    commands: [
      { name: 'get_available_models', params: '()', returns: 'Result<Vec<Model>, String>', caller: 'ModelList', snippet: '#[tauri::command]\nasync fn get_available_models(\n    mgr: State<\'_, Arc<ModelManager>>,\n) -> Result<Vec<Model>, String> { ... }' },
      { name: 'download_model', params: '(name: String)', returns: 'Result<(), String>', caller: 'ModelList', snippet: '#[tauri::command]\nasync fn download_model(\n    name: String,\n    mgr: State<\'_, Arc<ModelManager>>,\n    app: AppHandle,\n) -> Result<(), String> { ... }' },
    ],
  },
]

const EVENTS = [
  { name: 'recording-started', emitter: 'recording.rs', listener: 'App', payload: 'Recording', direction: 'Backend → Frontend' },
  { name: 'recording-stopped', emitter: 'recording.rs', listener: 'App', payload: '()', direction: 'Backend → Frontend' },
  { name: 'transcription-update', emitter: 'transcription/manager.rs', listener: 'TranscriptDisplay', payload: 'TranscriptSegment', direction: 'Backend → Frontend' },
  { name: 'youtube-video-detected', emitter: 'browser/observer.rs', listener: 'YouTubeSection', payload: 'YouTubeGist', direction: 'Backend → Frontend' },
  { name: 'model-download-progress', emitter: 'settings/model_manager.rs', listener: 'ModelList', payload: '{ name, progress, total }', direction: 'Backend → Frontend' },
]

const STATE_OBJECTS = [
  { name: 'FileManager', lockType: 'Direct', typeSignature: 'FileManager', accessedBy: ['get_recordings', 'delete_recording'], color: 'border-slate-600' },
  { name: 'RecordingManager', lockType: 'Mutex', typeSignature: 'Mutex<RecordingManager>', accessedBy: ['start_recording', 'stop_recording'], color: 'border-emerald-500/30' },
  { name: 'TranscriptionManager', lockType: 'tokio::Mutex', typeSignature: 'tokio::sync::Mutex<TranscriptionManager>', accessedBy: ['start_recording', 'stop_recording'], color: 'border-emerald-500/30' },
  { name: 'GemStore', lockType: 'Arc<dyn Trait>', typeSignature: 'Arc<dyn GemStore>', accessedBy: ['save_gem', 'list_gems', 'search_gems', 'delete_gem'], color: 'border-yellow-500/30' },
  { name: 'IntelProvider', lockType: 'Arc<dyn Trait>', typeSignature: 'Arc<dyn IntelProvider>', accessedBy: ['check_intelligence', 'enrich_gem', 'save_gem'], color: 'border-rose-500/30' },
  { name: 'SettingsManager', lockType: 'Arc<RwLock>', typeSignature: 'Arc<RwLock<SettingsManager>>', accessedBy: ['get_settings', 'update_settings'], color: 'border-slate-500/30' },
  { name: 'ModelManager', lockType: 'Arc', typeSignature: 'Arc<ModelManager>', accessedBy: ['get_available_models', 'download_model', 'delete_model'], color: 'border-blue-500/30' },
  { name: 'BrowserObserver', lockType: 'Arc<tokio::Mutex>', typeSignature: 'Arc<tokio::sync::Mutex<BrowserObserver>>', accessedBy: ['start_browser_observer', 'stop_browser_observer'], color: 'border-violet-500/30' },
]

// ── Component ──

export default function TauriBridgeLayer() {
  const [activeCategory, setActiveCategory] = useState(COMMAND_CATALOG[0].category)

  const activeCmds = COMMAND_CATALOG.find(c => c.category === activeCategory)

  return (
    <div className="space-y-10">
      {/* ── Command Catalog ── */}
      <section>
        <SectionHeader
          title="Command Catalog"
          description="37 Tauri commands — grouped by subsystem, click any to see its signature and code"
        />

        {/* Category tabs */}
        <div className="flex flex-wrap gap-1.5 mb-4">
          {COMMAND_CATALOG.map((cat) => (
            <button
              key={cat.category}
              onClick={() => setActiveCategory(cat.category)}
              className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
                activeCategory === cat.category
                  ? `${cat.color} bg-white/5`
                  : 'text-slate-500 hover:text-slate-300 hover:bg-slate-800/50'
              }`}
            >
              {cat.category}
              <span className="ml-1 text-slate-600">{cat.commands.length}</span>
            </button>
          ))}
        </div>

        {/* Commands in active category */}
        {activeCmds && (
          <div className="space-y-2 animate-fade-in">
            {activeCmds.commands.map((cmd) => (
              <NodeCard
                key={cmd.name}
                title={cmd.name}
                icon="⚡"
                description={`${cmd.params} → ${cmd.returns}`}
                color="border-amber-500/20"
                badge={cmd.caller}
                badgeColor="text-amber-500/60"
              >
                <div className="space-y-3">
                  <div className="grid grid-cols-2 gap-3 text-xs">
                    <div>
                      <span className="text-slate-500">Params:</span>
                      <p className="font-mono text-amber-300/80 mt-0.5">{cmd.params}</p>
                    </div>
                    <div>
                      <span className="text-slate-500">Returns:</span>
                      <p className="font-mono text-amber-300/80 mt-0.5">{cmd.returns}</p>
                    </div>
                  </div>
                  <CodeSnippet code={cmd.snippet} language="rust" />
                </div>
              </NodeCard>
            ))}
          </div>
        )}
      </section>

      {/* ── Event Flow ── */}
      <section>
        <SectionHeader
          title="Event System"
          description="Backend emits events, frontend listens — all events flow from Rust to React via Tauri"
        />
        <div className="space-y-1.5">
          {EVENTS.map((ev) => (
            <div key={ev.name} className="flex items-center gap-3 px-3 py-2.5 rounded-lg bg-slate-900/40 border border-slate-800/50 text-xs group hover:bg-slate-800/40 transition-colors">
              <span className="font-mono text-amber-400 font-medium min-w-48">{ev.name}</span>
              <div className="flex items-center gap-2 text-slate-500 flex-1">
                <span className="font-mono text-slate-400">{ev.emitter}</span>
                <span className="text-amber-500/60 animate-pulse">→</span>
                <span className="font-mono text-slate-400">{ev.listener}</span>
              </div>
              <span className="font-mono text-slate-600 hidden group-hover:inline">{ev.payload}</span>
            </div>
          ))}
        </div>
      </section>

      {/* ── State Management Map ── */}
      <section>
        <SectionHeader
          title="State Management"
          description="8 managed state objects — each with a specific concurrency strategy"
        />
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
          {STATE_OBJECTS.map((s) => (
            <NodeCard
              key={s.name}
              title={s.name}
              icon="📦"
              description={s.typeSignature}
              color={s.color}
              badge={s.lockType}
              badgeColor="text-amber-400/70"
            >
              <div className="text-xs space-y-2">
                <div>
                  <span className="text-slate-500">Type:</span>
                  <p className="font-mono text-amber-300/70 mt-0.5">{s.typeSignature}</p>
                </div>
                <div>
                  <span className="text-slate-500">Accessed by:</span>
                  <div className="flex flex-wrap gap-1 mt-1">
                    {s.accessedBy.map((cmd) => (
                      <span key={cmd} className="font-mono px-1.5 py-0.5 rounded bg-amber-500/10 text-amber-300/70 text-[11px]">{cmd}</span>
                    ))}
                  </div>
                </div>
              </div>
            </NodeCard>
          ))}
        </div>
      </section>

      {/* ── Plugin Registry ── */}
      <section>
        <SectionHeader title="Plugin Registry" description="Tauri v2 plugins configured in lib.rs" />
        <div className="flex flex-wrap gap-2">
          {[
            { name: 'tauri-plugin-shell', purpose: 'Sidecar process management' },
            { name: 'tauri-plugin-global-shortcut', purpose: 'Cmd+Shift+R to record' },
            { name: 'tauri-plugin-notification', purpose: 'Native macOS notifications' },
            { name: 'tauri-plugin-opener', purpose: 'Open URLs in default browser' },
          ].map((p) => (
            <div key={p.name} className="px-3 py-2 rounded-lg bg-slate-900/40 border border-slate-800/50 text-xs">
              <span className="font-mono text-amber-400">{p.name}</span>
              <p className="text-slate-500 mt-0.5">{p.purpose}</p>
            </div>
          ))}
        </div>
      </section>

      <KeyFiles
        color="text-amber-400"
        files={[
          { path: 'src-tauri/src/commands.rs', description: '58KB — all 37 Tauri command handlers' },
          { path: 'src-tauri/src/lib.rs', description: 'App builder — plugin init, state management, command registration' },
          { path: 'src-tauri/tauri.conf.json', description: 'Window config, external binaries, permissions' },
        ]}
      />
    </div>
  )
}
