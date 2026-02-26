import { useState } from 'react'

type QuizBlockProps = {
  question: string
  options: string[]
  correctIndex: number
  explanation: string
}

export default function QuizBlock({ question, options, correctIndex, explanation }: QuizBlockProps) {
  const [selected, setSelected] = useState<number | null>(null)
  const answered = selected !== null
  const correct = selected === correctIndex

  function handleSelect(i: number) {
    if (answered) return
    setSelected(i)
  }

  return (
    <div className="rounded-lg border border-slate-800 bg-slate-900/40 overflow-hidden">
      <div className="px-4 py-3 border-b border-slate-800/50">
        <p className="text-sm text-slate-200 font-medium">{question}</p>
      </div>
      <div className="p-3 space-y-1.5">
        {options.map((opt, i) => {
          let style = 'border-slate-800 bg-slate-900/30 hover:bg-slate-800/50 text-slate-300'
          if (answered) {
            if (i === correctIndex) style = 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300'
            else if (i === selected) style = 'border-red-500/40 bg-red-500/10 text-red-300'
            else style = 'border-slate-800/50 bg-slate-900/20 text-slate-500'
          }
          return (
            <button
              key={i}
              onClick={() => handleSelect(i)}
              disabled={answered}
              className={`w-full text-left px-3 py-2 rounded-lg border text-xs transition-all ${style} ${!answered ? 'cursor-pointer' : 'cursor-default'}`}
            >
              <span className="text-slate-500 mr-2">{String.fromCharCode(65 + i)}.</span>
              {opt}
            </button>
          )
        })}
      </div>
      {answered && (
        <div className={`px-4 py-3 border-t animate-fade-in ${correct ? 'border-emerald-500/20 bg-emerald-500/5' : 'border-red-500/20 bg-red-500/5'}`}>
          <p className={`text-xs font-medium mb-1 ${correct ? 'text-emerald-400' : 'text-red-400'}`}>{correct ? 'Correct!' : 'Not quite'}</p>
          <p className="text-xs text-slate-400">{explanation}</p>
        </div>
      )}
    </div>
  )
}
