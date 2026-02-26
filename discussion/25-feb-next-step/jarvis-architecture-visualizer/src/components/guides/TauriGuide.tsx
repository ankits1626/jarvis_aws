import GuideShell from './GuideShell.tsx'
import { tauriGuide } from '../../data/guides/tauriData.ts'

export default function TauriGuide() {
  return <GuideShell guide={tauriGuide} />
}
