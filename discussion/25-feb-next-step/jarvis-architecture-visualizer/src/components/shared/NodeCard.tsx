import type { ReactNode } from 'react'
import { useState } from 'react'

type NodeCardProps = {
  title: string
  icon?: string
  description?: string
  color?: string             // border color class
  badge?: string             // small tag in the header
  badgeColor?: string        // badge text color
  defaultExpanded?: boolean
  children?: ReactNode
}

export default function NodeCard({
  title,
  icon,
  description,
  color = 'border-slate-700',
  badge,
  badgeColor = 'text-slate-500',
  defaultExpanded = false,
  children,
}: NodeCardProps) {
  const [expanded, setExpanded] = useState(defaultExpanded)

  return (
    <div className={`rounded-lg border ${color} bg-slate-900/50 transition-all`}>
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center gap-3 p-3 text-left hover:bg-slate-800/30 transition-colors rounded-lg"
      >
        {icon && <span className="text-lg shrink-0">{icon}</span>}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <p className="text-sm font-medium text-slate-200 truncate">{title}</p>
            {badge && (
              <span className={`text-[10px] px-1.5 py-0.5 rounded-full bg-slate-800 font-medium ${badgeColor}`}>
                {badge}
              </span>
            )}
          </div>
          {description && !expanded && (
            <p className="text-xs text-slate-500 truncate mt-0.5">{description}</p>
          )}
        </div>
        <span className={`text-xs text-slate-600 transition-transform duration-200 shrink-0 ${expanded ? 'rotate-90' : ''}`}>
          ▸
        </span>
      </button>
      {expanded && children && (
        <div className="px-3 pb-3 pt-1 border-t border-slate-800/50 animate-fade-in">
          {children}
        </div>
      )}
    </div>
  )
}
