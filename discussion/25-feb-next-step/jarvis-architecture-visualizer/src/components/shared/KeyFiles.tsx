import Tooltip from './Tooltip.tsx'

type KeyFilesProps = {
  files: { path: string; description: string }[]
  color?: string
}

export default function KeyFiles({ files, color = 'text-slate-400' }: KeyFilesProps) {
  return (
    <div className="mt-8 pt-6 border-t border-slate-800/50">
      <h4 className="text-xs font-medium text-slate-500 uppercase tracking-wider mb-3">Key Files</h4>
      <div className="flex flex-wrap gap-2">
        {files.map((f) => (
          <Tooltip key={f.path} content={f.description}>
            <span className={`inline-block font-mono text-xs px-2.5 py-1 rounded-md bg-slate-800/60 border border-slate-700/50 ${color} hover:bg-slate-800 transition-colors cursor-default`}>
              {f.path}
            </span>
          </Tooltip>
        ))}
      </div>
    </div>
  )
}
