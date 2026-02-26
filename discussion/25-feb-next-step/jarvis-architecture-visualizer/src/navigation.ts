// ── Navigation types & data ──

export type NavSection = {
  part: number
  label: string
  icon: string
  items: NavItem[]
}

export type NavItem = {
  id: string
  label: string
  icon: string
  color: string          // Tailwind color class for active state
}

export type ActiveView = {
  part: number
  section: string        // item id within the part
}

export const NAV_SECTIONS: NavSection[] = [
  {
    part: 1,
    label: 'The Big Picture',
    icon: '🔭',
    items: [
      { id: 'overview', label: 'System Overview', icon: '🗺️', color: 'text-indigo-400' },
    ],
  },
  {
    part: 2,
    label: 'Layer Explorer',
    icon: '🧱',
    items: [
      { id: 'frontend',     label: 'Frontend',            icon: '🖥️', color: 'text-cyan-400' },
      { id: 'tauri-bridge',  label: 'Tauri Bridge',        icon: '🌉', color: 'text-amber-400' },
      { id: 'audio',        label: 'Audio Pipeline',      icon: '🎙️', color: 'text-emerald-400' },
      { id: 'browser',      label: 'Browser & Extractors', icon: '🌐', color: 'text-violet-400' },
      { id: 'intelligence', label: 'Intelligence / AI',   icon: '🧠', color: 'text-rose-400' },
      { id: 'gems',         label: 'Gems',                icon: '💎', color: 'text-yellow-400' },
    ],
  },
  {
    part: 3,
    label: 'Data Flows',
    icon: '🔀',
    items: [
      { id: 'flow-recording', label: 'Recording Flow',       icon: '⏺️', color: 'text-emerald-400' },
      { id: 'flow-browser',   label: 'Browser Capture Flow', icon: '🌐', color: 'text-violet-400' },
      { id: 'flow-gem',       label: 'Gem Lifecycle',        icon: '💎', color: 'text-yellow-400' },
    ],
  },
  {
    part: 4,
    label: 'Spec Map',
    icon: '📋',
    items: [
      { id: 'specs', label: 'Kiro Specifications', icon: '📐', color: 'text-indigo-400' },
    ],
  },
  {
    part: 5,
    label: 'Tech Stack 101',
    icon: '📚',
    items: [
      { id: 'guide-rust',     label: 'Rust 101',            icon: '🦀', color: 'text-orange-400' },
      { id: 'guide-tauri',    label: 'Tauri 101',           icon: '⚡', color: 'text-teal-400' },
      { id: 'guide-swift',    label: 'Swift 101',           icon: '🐦', color: 'text-orange-400' },
      { id: 'guide-react',    label: 'React + TS 101',      icon: '⚛️', color: 'text-blue-400' },
      { id: 'guide-sqlite',   label: 'SQLite + FTS5 101',   icon: '🗄️', color: 'text-yellow-400' },
      { id: 'guide-macos',    label: 'macOS APIs 101',      icon: '🍎', color: 'text-slate-400' },
      { id: 'guide-spec',     label: 'Spec-Driven Dev 101', icon: '📝', color: 'text-indigo-400' },
      { id: 'guide-sidecar',  label: 'Sidecar Pattern 101', icon: '🚗', color: 'text-teal-400' },
    ],
  },
]

export const DEFAULT_VIEW: ActiveView = { part: 0, section: 'home' }
