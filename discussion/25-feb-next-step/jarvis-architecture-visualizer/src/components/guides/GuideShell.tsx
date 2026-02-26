import { useState, useRef, useEffect } from 'react'
import type { Guide, GuideSectionContent } from '../../data/guides/types.ts'
import CodeSnippet from '../shared/CodeSnippet.tsx'
import InteractiveCode from '../shared/InteractiveCode.tsx'
import QuizBlock from '../shared/QuizBlock.tsx'
import ConceptCard from '../shared/ConceptCard.tsx'
import ComparisonTable from '../shared/ComparisonTable.tsx'
import Tooltip from '../shared/Tooltip.tsx'

// ── Color map for guide accents ──
const COLOR_MAP: Record<string, { border: string; text: string; bg: string; dot: string }> = {
  orange:  { border: 'border-orange-500/40', text: 'text-orange-400', bg: 'bg-orange-500/5', dot: 'bg-orange-400' },
  teal:    { border: 'border-teal-500/40',   text: 'text-teal-400',   bg: 'bg-teal-500/5',   dot: 'bg-teal-400' },
  blue:    { border: 'border-blue-500/40',   text: 'text-blue-400',   bg: 'bg-blue-500/5',   dot: 'bg-blue-400' },
  yellow:  { border: 'border-yellow-500/40', text: 'text-yellow-400', bg: 'bg-yellow-500/5', dot: 'bg-yellow-400' },
  slate:   { border: 'border-slate-500/40',  text: 'text-slate-400',  bg: 'bg-slate-500/5',  dot: 'bg-slate-400' },
  indigo:  { border: 'border-indigo-500/40', text: 'text-indigo-400', bg: 'bg-indigo-500/5', dot: 'bg-indigo-400' },
  cyan:    { border: 'border-cyan-500/40',   text: 'text-cyan-400',   bg: 'bg-cyan-500/5',   dot: 'bg-cyan-400' },
}

function getColors(color: string) {
  return COLOR_MAP[color] ?? COLOR_MAP.indigo
}

// ── Block renderer ──

function renderBlock(block: GuideSectionContent, index: number, guideColor: string) {
  const colors = getColors(guideColor)

  switch (block.type) {
    case 'text':
      return <p key={index} className="text-sm text-slate-300 leading-relaxed">{block.body}</p>

    case 'code':
      return <CodeSnippet key={index} code={block.code} language={block.language} caption={block.caption} />

    case 'interactive-code':
      return (
        <InteractiveCode
          key={index}
          starterCode={block.starterCode}
          solution={block.solution}
          hint={block.hint}
          language={block.language}
          validator={block.validator}
        />
      )

    case 'concept-card':
      return (
        <ConceptCard
          key={index}
          term={block.term}
          explanation={block.explanation}
          example={block.example}
          color={colors.border}
        />
      )

    case 'comparison':
      return (
        <ComparisonTable
          key={index}
          leftLabel={block.leftLabel}
          rightLabel={block.rightLabel}
          rows={block.rows}
          color={colors.text}
        />
      )

    case 'quiz':
      return (
        <QuizBlock
          key={index}
          question={block.question}
          options={block.options}
          correctIndex={block.correctIndex}
          explanation={block.explanation}
        />
      )

    case 'diagram':
      return (
        <div key={index} className="flex items-center gap-2 flex-wrap py-2">
          {block.nodes.map((node, ni) => (
            <div key={node.id} className="flex items-center gap-2">
              <div className={`px-3 py-2 rounded-lg border ${colors.border} ${colors.bg} text-xs`}>
                {node.icon && <span className="mr-1">{node.icon}</span>}
                <span className="text-slate-300 font-medium">{node.label}</span>
              </div>
              {ni < block.nodes.length - 1 && block.connections.find(c => c.from === node.id) && (
                <div className="flex items-center gap-1 text-slate-600 text-xs shrink-0">
                  <span>→</span>
                  {block.connections.find(c => c.from === node.id)?.label && (
                    <span className="text-[10px] text-slate-500">
                      {block.connections.find(c => c.from === node.id)!.label}
                    </span>
                  )}
                </div>
              )}
            </div>
          ))}
        </div>
      )

    default:
      return null
  }
}

// ── GuideShell ──

export default function GuideShell({ guide }: { guide: Guide }) {
  const colors = getColors(guide.color)
  const [activeSection, setActiveSection] = useState(guide.sections[0]?.id ?? '')
  const sectionRefs = useRef<Record<string, HTMLElement | null>>({})

  // Scrollspy: track which section is visible
  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            setActiveSection(entry.target.id)
          }
        }
      },
      { rootMargin: '-20% 0px -60% 0px' }
    )

    for (const ref of Object.values(sectionRefs.current)) {
      if (ref) observer.observe(ref)
    }

    return () => observer.disconnect()
  }, [guide.id])

  function scrollTo(sectionId: string) {
    sectionRefs.current[sectionId]?.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }

  return (
    <div className="animate-fade-in">
      {/* Guide header */}
      <div className={`rounded-xl border ${colors.border} ${colors.bg} p-5 mb-8`}>
        <div className="flex items-center gap-3 mb-1">
          <span className="text-3xl">{guide.icon}</span>
          <div>
            <h2 className={`text-xl font-bold ${colors.text}`}>{guide.title}</h2>
            <p className="text-sm text-slate-400 mt-0.5">{guide.subtitle}</p>
          </div>
        </div>
      </div>

      {/* Layout: TOC + Content */}
      <div className="flex gap-8">
        {/* Mini TOC */}
        <nav className="hidden lg:block w-48 shrink-0 sticky top-10 self-start">
          <p className="text-[10px] text-slate-600 uppercase tracking-wider font-medium mb-3">Sections</p>
          <div className="space-y-1">
            {guide.sections.map((section) => (
              <button
                key={section.id}
                onClick={() => scrollTo(section.id)}
                className={`w-full text-left flex items-center gap-2 px-2 py-1.5 rounded text-xs transition-colors ${
                  activeSection === section.id
                    ? `${colors.text} bg-white/5`
                    : 'text-slate-500 hover:text-slate-300'
                }`}
              >
                <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${
                  activeSection === section.id ? colors.dot : 'bg-slate-700'
                }`} />
                <span className="truncate">{section.title}</span>
              </button>
            ))}
            <button
              onClick={() => scrollTo('jarvis-connections')}
              className={`w-full text-left flex items-center gap-2 px-2 py-1.5 rounded text-xs transition-colors ${
                activeSection === 'jarvis-connections' ? `${colors.text} bg-white/5` : 'text-slate-500 hover:text-slate-300'
              }`}
            >
              <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${activeSection === 'jarvis-connections' ? colors.dot : 'bg-slate-700'}`} />
              <span>In Jarvis</span>
            </button>
          </div>
        </nav>

        {/* Content */}
        <div className="flex-1 min-w-0 space-y-12">
          {guide.sections.map((section) => (
            <section
              key={section.id}
              id={section.id}
              ref={(el) => { sectionRefs.current[section.id] = el }}
            >
              <h3 className={`text-lg font-semibold ${colors.text} mb-4 pb-2 border-b ${colors.border}`}>
                {section.title}
              </h3>
              <div className="space-y-4">
                {/* Render concept cards in a grid */}
                {(() => {
                  const groups: GuideSectionContent[][] = []
                  let currentGroup: GuideSectionContent[] = []

                  for (const block of section.content) {
                    if (block.type === 'concept-card') {
                      currentGroup.push(block)
                    } else {
                      if (currentGroup.length > 0) {
                        groups.push(currentGroup)
                        currentGroup = []
                      }
                      groups.push([block])
                    }
                  }
                  if (currentGroup.length > 0) groups.push(currentGroup)

                  return groups.map((group, gi) => {
                    if (group.length > 1 || group[0]?.type === 'concept-card') {
                      return (
                        <div key={gi} className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
                          {group.map((block, bi) => renderBlock(block, bi, guide.color))}
                        </div>
                      )
                    }
                    return renderBlock(group[0], gi, guide.color)
                  })
                })()}
              </div>
            </section>
          ))}

          {/* Jarvis Connections */}
          <section
            id="jarvis-connections"
            ref={(el) => { sectionRefs.current['jarvis-connections'] = el }}
          >
            <h3 className={`text-lg font-semibold ${colors.text} mb-4 pb-2 border-b ${colors.border}`}>
              How Jarvis Uses This
            </h3>
            <div className="space-y-2">
              {guide.jarvisConnections.map((conn) => (
                <div key={conn.file} className="flex items-start gap-3 p-3 rounded-lg bg-slate-900/40 border border-slate-800/50">
                  <span className={`w-2 h-2 rounded-full shrink-0 mt-1.5 ${colors.dot}`} />
                  <div className="text-xs">
                    <span className={`font-medium ${colors.text}`}>{conn.concept}</span>
                    <Tooltip content={conn.description}>
                      <span className="font-mono text-slate-500 ml-2 hover:text-slate-300 transition-colors cursor-default">
                        {conn.file}
                      </span>
                    </Tooltip>
                    <p className="text-slate-400 mt-0.5">{conn.description}</p>
                  </div>
                </div>
              ))}
            </div>
          </section>
        </div>
      </div>
    </div>
  )
}
