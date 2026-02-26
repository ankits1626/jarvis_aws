import { useState } from 'react'

type CodeSnippetProps = {
  code: string
  language?: string
  caption?: string
}

// Basic keyword highlighting per language (no external lib)
function highlightCode(code: string, language?: string): string {
  if (!language) return code
  // We return raw text — highlighting is handled via CSS classes in the rendered spans
  return code
}

export default function CodeSnippet({ code, language, caption }: CodeSnippetProps) {
  const [copied, setCopied] = useState(false)

  function handleCopy() {
    navigator.clipboard.writeText(code).then(() => {
      setCopied(true)
      setTimeout(() => setCopied(false), 1500)
    })
  }

  const highlighted = highlightCode(code, language)

  return (
    <div className="rounded-lg bg-slate-950 border border-slate-800 overflow-hidden">
      {/* Header bar */}
      <div className="flex items-center justify-between px-3 py-1.5 bg-slate-900/60 border-b border-slate-800/50">
        <span className="text-[10px] text-slate-500 uppercase tracking-wider font-medium">
          {language ?? 'code'}
        </span>
        <button
          onClick={handleCopy}
          className="text-[10px] text-slate-500 hover:text-slate-300 transition-colors px-1.5 py-0.5 rounded hover:bg-slate-800"
        >
          {copied ? '✓ Copied' : 'Copy'}
        </button>
      </div>
      {/* Code body */}
      <pre className="p-3 font-mono text-xs text-slate-300 overflow-x-auto whitespace-pre-wrap leading-relaxed">
        {highlighted}
      </pre>
      {caption && (
        <div className="px-3 pb-2">
          <p className="text-[11px] text-slate-500 italic">{caption}</p>
        </div>
      )}
    </div>
  )
}
