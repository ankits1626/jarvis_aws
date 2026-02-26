type ComparisonTableProps = {
  leftLabel: string
  rightLabel: string
  rows: { label: string; left: string; right: string }[]
  color?: string
}

export default function ComparisonTable({ leftLabel, rightLabel, rows, color = 'text-indigo-400' }: ComparisonTableProps) {
  return (
    <div className="rounded-lg border border-slate-800 overflow-hidden">
      {/* Header */}
      <div className="grid grid-cols-3 bg-slate-900/60 border-b border-slate-800">
        <div className="px-3 py-2 text-[10px] text-slate-500 uppercase tracking-wider font-medium" />
        <div className={`px-3 py-2 text-[10px] uppercase tracking-wider font-medium ${color}`}>{leftLabel}</div>
        <div className={`px-3 py-2 text-[10px] uppercase tracking-wider font-medium ${color}`}>{rightLabel}</div>
      </div>
      {/* Rows */}
      {rows.map((row, i) => (
        <div
          key={row.label}
          className={`grid grid-cols-3 border-b border-slate-800/50 hover:bg-slate-800/30 transition-colors ${
            i === rows.length - 1 ? 'border-b-0' : ''
          }`}
        >
          <div className="px-3 py-2.5 text-xs text-slate-400 font-medium">{row.label}</div>
          <div className="px-3 py-2.5 text-xs text-slate-300 font-mono">{row.left}</div>
          <div className="px-3 py-2.5 text-xs text-slate-300 font-mono">{row.right}</div>
        </div>
      ))}
    </div>
  )
}
