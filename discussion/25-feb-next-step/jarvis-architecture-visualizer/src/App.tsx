import { useState } from 'react'
import Sidebar from './components/Sidebar.tsx'
import LayerExplorer from './components/LayerExplorer.tsx'
import RustGuide from './components/guides/RustGuide.tsx'
import TauriGuide from './components/guides/TauriGuide.tsx'
import ReactTSGuide from './components/guides/ReactTSGuide.tsx'
import SpecDrivenGuide from './components/guides/SpecDrivenGuide.tsx'
import SidecarGuide from './components/guides/SidecarGuide.tsx'
import { DEFAULT_VIEW, NAV_SECTIONS } from './navigation.ts'
import type { ActiveView } from './navigation.ts'

// ── Home ──

function Home({ onNavigate }: { onNavigate: (view: ActiveView) => void }) {
  return (
    <div className="animate-fade-in">
      <div className="mb-12">
        <h1 className="text-4xl font-bold text-slate-100 mb-3">
          Jarvis Architecture Visualizer
        </h1>
        <p className="text-lg text-slate-400 max-w-2xl">
          Explore the architecture. Learn the stack. Understand the system.
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 mb-12">
        {NAV_SECTIONS.map((section) => (
          <button
            key={section.part}
            onClick={() => onNavigate({ part: section.part, section: section.items[0].id })}
            className="group text-left p-5 rounded-xl bg-slate-900/60 border border-slate-800 hover:border-slate-700 hover:bg-slate-800/60 transition-all"
          >
            <div className="flex items-center gap-3 mb-3">
              <span className="text-2xl">{section.icon}</span>
              <div>
                <p className="text-xs text-slate-500 font-medium">Part {section.part}</p>
                <h3 className="text-sm font-semibold text-slate-200 group-hover:text-slate-100">
                  {section.label}
                </h3>
              </div>
            </div>
            <p className="text-xs text-slate-500">
              {section.items.length} {section.items.length === 1 ? 'section' : 'sections'}
            </p>
          </button>
        ))}
      </div>

      <div className="flex flex-wrap gap-6 text-sm text-slate-500">
        <span><strong className="text-slate-300">6</strong> layers</span>
        <span><strong className="text-slate-300">37</strong> commands</span>
        <span><strong className="text-slate-300">13</strong> specs</span>
        <span><strong className="text-slate-300">8</strong> guides</span>
        <span><strong className="text-slate-300">14</strong> components</span>
      </div>

      <div className="mt-10 p-4 rounded-lg bg-slate-900/40 border border-slate-800/50">
        <p className="text-xs text-slate-500 mb-2 font-medium uppercase tracking-wider">Suggested path</p>
        <div className="flex items-center gap-2 text-sm text-slate-400 flex-wrap">
          <span className="text-indigo-400">Big Picture</span>
          <span className="text-slate-600">&rarr;</span>
          <span className="text-amber-400">Layers</span>
          <span className="text-slate-600">&rarr;</span>
          <span className="text-emerald-400">Data Flows</span>
          <span className="text-slate-600">&rarr;</span>
          <span className="text-violet-400">Spec Map</span>
          <span className="text-slate-600">&rarr;</span>
          <span className="text-rose-400">101 Guides</span>
        </div>
      </div>
    </div>
  )
}

// ── Placeholder (for parts not yet built) ──

function Placeholder({ title, icon, color }: { title: string; icon: string; color: string }) {
  return (
    <div className="animate-fade-in flex flex-col items-center justify-center py-32">
      <span className="text-5xl mb-4">{icon}</span>
      <h2 className={`text-2xl font-bold mb-2 ${color}`}>{title}</h2>
      <p className="text-slate-500 text-sm">Coming in the next phase</p>
    </div>
  )
}

// ── Layer IDs that are wired up ──
const LAYER_IDS = new Set(['frontend', 'tauri-bridge', 'audio', 'browser', 'intelligence', 'gems'])

// ── Content resolver ──

function resolveContent(view: ActiveView, onNavigate: (view: ActiveView) => void) {
  if (view.part === 0) {
    return <Home onNavigate={onNavigate} />
  }

  // Part 2 — Layer Explorer (fully built)
  if (view.part === 2 && LAYER_IDS.has(view.section)) {
    return <LayerExplorer activeLayer={view.section} />
  }

  // Part 5 — Tech Stack 101 Guides
  if (view.part === 5) {
    switch (view.section) {
      case 'guide-rust': return <RustGuide />
      case 'guide-tauri': return <TauriGuide />
      case 'guide-react': return <ReactTSGuide />
      case 'guide-spec': return <SpecDrivenGuide />
      case 'guide-sidecar': return <SidecarGuide />
    }
  }

  // Everything else — placeholder
  const navSection = NAV_SECTIONS.find(s => s.part === view.part)
  const navItem = navSection?.items.find(i => i.id === view.section)

  if (!navSection || !navItem) {
    return <Placeholder title="Not Found" icon="❓" color="text-slate-400" />
  }

  return <Placeholder title={navItem.label} icon={navItem.icon} color={navItem.color} />
}

// ── App ──

export default function App() {
  const [activeView, setActiveView] = useState<ActiveView>(DEFAULT_VIEW)

  return (
    <div className="flex min-h-screen">
      <Sidebar activeView={activeView} onNavigate={setActiveView} />

      <main className="flex-1 ml-70 min-h-screen">
        <div className="max-w-5xl mx-auto px-8 py-10">
          {resolveContent(activeView, setActiveView)}
        </div>
      </main>
    </div>
  )
}
