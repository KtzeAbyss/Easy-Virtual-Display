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
    <header className="flex min-h-8 flex-wrap items-center justify-between gap-x-3 gap-y-1.5 pb-3">
      <div className="flex min-w-0 items-baseline gap-2.5">
        <h1 className="font-display text-[22px] leading-7 font-semibold">{title}</h1>
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
      {title && <h2 className="pt-2 pb-0.5 text-[13px] font-semibold">{title}</h2>}
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
      <div className="flex min-h-10 items-center gap-3 px-3.5 py-1.5">
        <div className="flex min-w-0 flex-1 flex-col gap-px">
          <span className="text-sm leading-5">{label}</span>
          {description && (
            <span className="truncate text-xs leading-4 text-muted-foreground">{description}</span>
          )}
        </div>
        {children && <div className="flex shrink-0 items-center gap-2">{children}</div>}
      </div>
      {footer}
    </div>
  )
}
