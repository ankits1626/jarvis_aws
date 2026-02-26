import FrontendLayer from './layers/FrontendLayer.tsx'
import TauriBridgeLayer from './layers/TauriBridgeLayer.tsx'
import AudioPipelineLayer from './layers/AudioPipelineLayer.tsx'
import BrowserExtractorLayer from './layers/BrowserExtractorLayer.tsx'
import IntelligenceLayer from './layers/IntelligenceLayer.tsx'
import GemsLayer from './layers/GemsLayer.tsx'

type LayerConfig = {
  id: string
  title: string
  icon: string
  description: string
  color: string           // border/accent color class
  textColor: string       // text color class
  bgColor: string         // subtle background
  component: React.ComponentType
}

const LAYERS: LayerConfig[] = [
  {
    id: 'frontend',
    title: 'Frontend',
    icon: '🖥️',
    description: 'React 19 + TypeScript — 14 components, useReducer state, Tauri IPC hooks',
    color: 'border-cyan-500/40',
    textColor: 'text-cyan-400',
    bgColor: 'bg-cyan-500/5',
    component: FrontendLayer,
  },
  {
    id: 'tauri-bridge',
    title: 'Tauri Bridge',
    icon: '🌉',
    description: '37 IPC commands, event system, plugin registry, managed state objects',
    color: 'border-amber-500/40',
    textColor: 'text-amber-400',
    bgColor: 'bg-amber-500/5',
    component: TauriBridgeLayer,
  },
  {
    id: 'audio',
    title: 'Audio Pipeline',
    icon: '🎙️',
    description: 'JarvisListen sidecar → FIFO → hybrid transcription (VAD + Vosk + Whisper)',
    color: 'border-emerald-500/40',
    textColor: 'text-emerald-400',
    bgColor: 'bg-emerald-500/5',
    component: AudioPipelineLayer,
  },
  {
    id: 'browser',
    title: 'Browser & Extractors',
    icon: '🌐',
    description: 'Chrome observer, 6 content extractors, PageGist unified type',
    color: 'border-violet-500/40',
    textColor: 'text-violet-400',
    bgColor: 'bg-violet-500/5',
    component: BrowserExtractorLayer,
  },
  {
    id: 'intelligence',
    title: 'Intelligence / AI',
    icon: '🧠',
    description: 'IntelProvider trait, IntelligenceKit sidecar, NDJSON protocol, graceful degradation',
    color: 'border-rose-500/40',
    textColor: 'text-rose-400',
    bgColor: 'bg-rose-500/5',
    component: IntelligenceLayer,
  },
  {
    id: 'gems',
    title: 'Gems — Knowledge Base',
    icon: '💎',
    description: 'SQLite + FTS5 storage, CRUD operations, AI enrichment pipeline',
    color: 'border-yellow-500/40',
    textColor: 'text-yellow-400',
    bgColor: 'bg-yellow-500/5',
    component: GemsLayer,
  },
]

type LayerExplorerProps = {
  activeLayer: string
}

export default function LayerExplorer({ activeLayer }: LayerExplorerProps) {
  const layer = LAYERS.find(l => l.id === activeLayer)

  if (!layer) {
    return <div className="text-slate-500 py-20 text-center">Layer not found</div>
  }

  const LayerComponent = layer.component

  return (
    <div className="animate-fade-in">
      {/* Layer header */}
      <div className={`rounded-xl border ${layer.color} ${layer.bgColor} p-5 mb-8`}>
        <div className="flex items-center gap-3 mb-2">
          <span className="text-3xl">{layer.icon}</span>
          <div>
            <h2 className={`text-xl font-bold ${layer.textColor}`}>{layer.title}</h2>
            <p className="text-sm text-slate-400 mt-0.5">{layer.description}</p>
          </div>
        </div>
      </div>

      {/* Layer content */}
      <LayerComponent />
    </div>
  )
}
