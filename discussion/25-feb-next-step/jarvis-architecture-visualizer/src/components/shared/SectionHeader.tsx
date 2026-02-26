type SectionHeaderProps = {
  title: string
  description?: string
}

export default function SectionHeader({ title, description }: SectionHeaderProps) {
  return (
    <div className="mb-4">
      <h3 className="text-base font-semibold text-slate-200">{title}</h3>
      {description && <p className="text-xs text-slate-500 mt-1">{description}</p>}
    </div>
  )
}
