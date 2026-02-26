import { useState } from 'react'
import type { ActiveView, NavSection } from '../navigation.ts'
import { NAV_SECTIONS } from '../navigation.ts'

type SidebarProps = {
  activeView: ActiveView
  onNavigate: (view: ActiveView) => void
}

export default function Sidebar({ activeView, onNavigate }: SidebarProps) {
  const [collapsed, setCollapsed] = useState(false)
  const [expandedParts, setExpandedParts] = useState<Set<number>>(new Set([activeView.part]))

  function togglePart(part: number) {
    setExpandedParts(prev => {
      const next = new Set(prev)
      if (next.has(part)) next.delete(part)
      else next.add(part)
      return next
    })
  }

  const isHome = activeView.part === 0

  return (
    <aside
      className={`fixed top-0 left-0 h-screen bg-slate-900/80 backdrop-blur-sm border-r border-slate-800 flex flex-col z-50 transition-sidebar ${
        collapsed ? 'w-16' : 'w-70'
      }`}
    >
      {/* Header */}
      <div className={`flex items-center border-b border-slate-800 h-14 shrink-0 ${collapsed ? 'justify-center px-2' : 'justify-between px-4'}`}>
        {!collapsed && (
          <button
            onClick={() => onNavigate({ part: 0, section: 'home' })}
            className="flex items-center gap-2 hover:opacity-80 transition-opacity"
          >
            <span className="text-lg">⚡</span>
            <span className="font-semibold text-sm text-slate-100 tracking-tight">Jarvis Visualizer</span>
          </button>
        )}
        <button
          onClick={() => setCollapsed(!collapsed)}
          className="p-1.5 rounded-md hover:bg-slate-800 text-slate-400 hover:text-slate-200 transition-colors"
          title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        >
          {collapsed ? '▶' : '◀'}
        </button>
      </div>

      {/* Navigation */}
      <nav className="flex-1 overflow-y-auto py-2">
        {/* Home */}
        <button
          onClick={() => onNavigate({ part: 0, section: 'home' })}
          className={`w-full flex items-center gap-3 px-4 py-2 text-sm transition-colors ${
            isHome
              ? 'text-indigo-400 bg-indigo-400/10'
              : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/50'
          } ${collapsed ? 'justify-center px-2' : ''}`}
        >
          <span className="text-base shrink-0">🏠</span>
          {!collapsed && <span>Home</span>}
        </button>

        <div className="h-px bg-slate-800 my-2" />

        {/* Sections */}
        {NAV_SECTIONS.map((section) => (
          <SidebarSection
            key={section.part}
            section={section}
            collapsed={collapsed}
            expanded={expandedParts.has(section.part)}
            activeView={activeView}
            onToggle={() => togglePart(section.part)}
            onNavigate={onNavigate}
          />
        ))}
      </nav>

      {/* Footer */}
      {!collapsed && (
        <div className="border-t border-slate-800 px-4 py-3">
          <p className="text-[10px] text-slate-600 leading-tight">
            Jarvis AWS Architecture
          </p>
        </div>
      )}
    </aside>
  )
}

// ── Section ──

type SidebarSectionProps = {
  section: NavSection
  collapsed: boolean
  expanded: boolean
  activeView: ActiveView
  onToggle: () => void
  onNavigate: (view: ActiveView) => void
}

function SidebarSection({ section, collapsed, expanded, activeView, onToggle, onNavigate }: SidebarSectionProps) {
  const isActivePart = activeView.part === section.part
  const isSingleItem = section.items.length === 1

  // For single-item sections, clicking the header navigates directly
  function handleHeaderClick() {
    if (isSingleItem) {
      onNavigate({ part: section.part, section: section.items[0].id })
    } else {
      onToggle()
    }
  }

  return (
    <div className="mb-1">
      {/* Section header */}
      <button
        onClick={handleHeaderClick}
        className={`w-full flex items-center gap-3 px-4 py-2 text-xs font-medium uppercase tracking-wider transition-colors ${
          isActivePart
            ? 'text-slate-200'
            : 'text-slate-500 hover:text-slate-300'
        } ${collapsed ? 'justify-center px-2' : ''}`}
      >
        <span className="text-base shrink-0">{section.icon}</span>
        {!collapsed && (
          <>
            <span className="flex-1 text-left">{section.label}</span>
            {!isSingleItem && (
              <span className={`text-[10px] transition-transform duration-200 ${expanded ? 'rotate-90' : ''}`}>
                ▸
              </span>
            )}
          </>
        )}
      </button>

      {/* Sub-items */}
      {!collapsed && expanded && !isSingleItem && (
        <div className="animate-fade-in-left">
          {section.items.map((item) => {
            const isActive = activeView.part === section.part && activeView.section === item.id
            return (
              <button
                key={item.id}
                onClick={() => onNavigate({ part: section.part, section: item.id })}
                className={`w-full flex items-center gap-2.5 pl-10 pr-4 py-1.5 text-sm transition-colors ${
                  isActive
                    ? `${item.color} bg-white/5`
                    : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/50'
                }`}
              >
                <span className="text-xs shrink-0">{item.icon}</span>
                <span className="truncate">{item.label}</span>
              </button>
            )
          })}
        </div>
      )}
    </div>
  )
}
