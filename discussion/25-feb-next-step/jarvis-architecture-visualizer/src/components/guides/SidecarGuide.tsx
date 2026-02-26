import GuideShell from './GuideShell.tsx'
import { sidecarGuide } from '../../data/guides/sidecarData.ts'

export default function SidecarGuide() {
  return <GuideShell guide={sidecarGuide} />
}
