// ── Guide content block types ──

export type TextBlock = { type: 'text'; body: string }
export type CodeBlock = { type: 'code'; language: string; code: string; caption?: string }
export type InteractiveCodeBlock = {
  type: 'interactive-code'
  language: string
  starterCode: string
  solution: string
  hint: string
  validator: (input: string) => boolean
}
export type ConceptCardBlock = { type: 'concept-card'; term: string; explanation: string; example?: string }
export type ComparisonBlock = {
  type: 'comparison'
  leftLabel: string
  rightLabel: string
  rows: { label: string; left: string; right: string }[]
}
export type QuizBlock = {
  type: 'quiz'
  question: string
  options: string[]
  correctIndex: number
  explanation: string
}
export type DiagramBlock = {
  type: 'diagram'
  nodes: { id: string; label: string; icon?: string; color?: string }[]
  connections: { from: string; to: string; label?: string }[]
}

export type GuideSectionContent =
  | TextBlock
  | CodeBlock
  | InteractiveCodeBlock
  | ConceptCardBlock
  | ComparisonBlock
  | QuizBlock
  | DiagramBlock

export type GuideSection = {
  id: string
  title: string
  content: GuideSectionContent[]
}

export type JarvisConnection = {
  concept: string
  file: string
  description: string
}

export type Guide = {
  id: string
  title: string
  subtitle: string
  color: string           // Tailwind color prefix (e.g., 'orange', 'teal')
  icon: string
  sections: GuideSection[]
  jarvisConnections: JarvisConnection[]
}
