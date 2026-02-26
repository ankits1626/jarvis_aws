import GuideShell from './GuideShell.tsx'
import { specDrivenGuide } from '../../data/guides/specDrivenData.ts'

export default function SpecDrivenGuide() {
  return <GuideShell guide={specDrivenGuide} />
}
