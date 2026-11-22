import { cn } from '@/lib/utils'

export function PageHeader({
  title,
  meta,
  actions
}: {
  title: string
  meta?: React.ReactNode
  actions?: React.ReactNode
}): React.JSX.Element {
  return (
    <header className="flex min-h-10 flex-wrap items-center justify-between gap-x-4 gap-y-2 pb-5">
      <div className="flex min-w-0 items-baseline gap-3">
        <h1 className="font-display text-[28px] leading-9 font-semibold">{title}</h1>
        {meta}
      </div>
      {actions && <div className="flex items-center gap-2">{actions}</div>}
    </header>
  )
}

export function Section({
  title,
  children,
  className
}: {
  title?: string
  children: React.ReactNode
  className?: string
}): React.JSX.Element {
  return (
    <section className={cn('flex flex-col gap-1', className)}>
      {title && <h2 className="pt-4 pb-1.5 text-sm font-semibold">{title}</h2>}
      {children}
    </section>
  )
}

/** Win11 Settings card: label and description on the left, the control on the right. */
export function SettingRow({
  label,
  description,
  children,
  footer
}: {
  label: string
  description?: string
  children?: React.ReactNode
  footer?: React.ReactNode
}): React.JSX.Element {
  return (
    <div className="rounded-md border bg-card">
      <div className="flex min-h-[60px] items-center gap-4 px-4 py-3">
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <span className="text-sm">{label}</span>
          {description && (
            <span className="text-xs leading-4 text-muted-foreground">{description}</span>
          )}
        </div>
        {children && <div className="flex shrink-0 items-center gap-2">{children}</div>}
      </div>
      {footer}
    </div>
  )
}
