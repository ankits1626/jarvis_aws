// Stub — implemented in Phase 4
type StatusBadgeProps = {
  status: 'implemented' | 'in-progress' | 'planned'
}

const styles = {
  'implemented': 'bg-emerald-500/15 text-emerald-400 border-emerald-500/30',
  'in-progress': 'bg-yellow-500/15 text-yellow-400 border-yellow-500/30',
  'planned':     'bg-slate-500/15 text-slate-400 border-slate-500/30',
} as const

const icons = {
  'implemented': '✓',
  'in-progress': '◔',
  'planned':     '○',
} as const

export default function StatusBadge({ status }: StatusBadgeProps) {
  return (
    <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium border ${styles[status]}`}>
      <span>{icons[status]}</span>
      <span className="capitalize">{status}</span>
    </span>
  )
}
