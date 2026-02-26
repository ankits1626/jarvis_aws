import { useState } from 'react'

type InteractiveCodeProps = {
  starterCode: string
  solution: string
  hint: string
  language?: string
  validator: (input: string) => boolean
}

export default function InteractiveCode({ starterCode, solution, hint, language, validator }: InteractiveCodeProps) {
  const [code, setCode] = useState(starterCode)
  const [result, setResult] = useState<'pass' | 'fail' | null>(null)
  const [showHint, setShowHint] = useState(false)
  const [showSolution, setShowSolution] = useState(false)

  function handleRun() {
    setResult(validator(code) ? 'pass' : 'fail')
  }

  function handleReset() {
    setCode(starterCode)
    setResult(null)
    setShowHint(false)
    setShowSolution(false)
  }

  return (
    <div className="rounded-lg border border-slate-800 overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-1.5 bg-slate-900/60 border-b border-slate-800/50">
        <span className="text-[10px] text-slate-500 uppercase tracking-wider font-medium">
          {language ?? 'code'} — interactive
        </span>
        <div className="flex items-center gap-1.5">
          <button onClick={handleReset} className="text-[10px] text-slate-500 hover:text-slate-300 transition-colors px-1.5 py-0.5 rounded hover:bg-slate-800">Reset</button>
          <button onClick={() => setShowHint(!showHint)} className="text-[10px] text-amber-500/70 hover:text-amber-400 transition-colors px-1.5 py-0.5 rounded hover:bg-slate-800">Hint</button>
          <button onClick={() => setShowSolution(!showSolution)} className="text-[10px] text-slate-500 hover:text-slate-300 transition-colors px-1.5 py-0.5 rounded hover:bg-slate-800">Solution</button>
        </div>
      </div>

      {/* Editor */}
      <textarea
        value={code}
        onChange={(e) => { setCode(e.target.value); setResult(null) }}
        className="w-full bg-slate-950 text-slate-300 font-mono text-xs p-3 resize-y min-h-28 outline-none border-none leading-relaxed"
        spellCheck={false}
      />

      {/* Run + result */}
      <div className="flex items-center gap-3 px-3 py-2 bg-slate-900/40 border-t border-slate-800/50">
        <button onClick={handleRun} className="px-3 py-1 rounded-md bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-medium transition-colors">Run</button>
        {result === 'pass' && <span className="text-xs text-emerald-400 font-medium animate-fade-in">Correct!</span>}
        {result === 'fail' && <span className="text-xs text-red-400 font-medium animate-fade-in">Not quite — try again or check the hint</span>}
      </div>

      {showHint && (
        <div className="px-3 py-2 bg-amber-500/5 border-t border-amber-500/20 animate-fade-in">
          <p className="text-xs text-amber-300/80">{hint}</p>
        </div>
      )}

      {showSolution && (
        <div className="border-t border-slate-800/50 animate-fade-in">
          <div className="px-3 py-1.5 bg-slate-900/60">
            <span className="text-[10px] text-slate-500 uppercase tracking-wider font-medium">Solution</span>
          </div>
          <pre className="px-3 py-2 bg-slate-950 font-mono text-xs text-emerald-300/80 whitespace-pre-wrap">{solution}</pre>
        </div>
      )}
    </div>
  )
}
