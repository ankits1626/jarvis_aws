import { useState } from 'react'

type ConceptCardProps = {
  term: string
  explanation: string
  example?: string
  color?: string
}

export default function ConceptCard({ term, explanation, example, color = 'border-slate-700' }: ConceptCardProps) {
  const [flipped, setFlipped] = useState(false)

  return (
    <div
      className="cursor-pointer"
      style={{ perspective: '800px' }}
      onClick={() => setFlipped(!flipped)}
    >
      {/* Grid overlap: both children in same cell → container takes the taller one's height */}
      <div
        className="grid transition-transform duration-500"
        style={{
          transformStyle: 'preserve-3d',
          transform: flipped ? 'rotateY(180deg)' : 'rotateY(0deg)',
        }}
      >
        {/* Front */}
        <div
          className={`rounded-xl border ${color} bg-slate-900/60 p-5 min-h-32 flex flex-col items-center justify-center text-center`}
          style={{ backfaceVisibility: 'hidden', gridArea: '1 / 1' }}
        >
          <p className="text-base font-semibold text-slate-200 mb-1">{term}</p>
          <p className="text-[11px] text-slate-500">Click to reveal</p>
        </div>

        {/* Back */}
        <div
          className={`rounded-xl border ${color} bg-slate-800/80 p-4 flex flex-col justify-center`}
          style={{ backfaceVisibility: 'hidden', transform: 'rotateY(180deg)', gridArea: '1 / 1' }}
        >
          <p className="text-xs text-slate-300 leading-relaxed mb-2">{explanation}</p>
          {example && (
            <pre className="text-[11px] font-mono text-slate-400 bg-slate-950/60 rounded-md p-2 whitespace-pre-wrap">{example}</pre>
          )}
        </div>
      </div>
    </div>
  )
}
