// Stub — implemented in Phase 1
export default function FlowArrow({ label }: { label?: string }) {
  return (
    <div className="flex items-center gap-1 text-slate-600 text-xs">
      <div className="h-px w-8 bg-slate-700" />
      <span>→</span>
      {label && <span className="text-slate-500">{label}</span>}
      <div className="h-px w-8 bg-slate-700" />
    </div>
  )
}
