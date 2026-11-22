import { CircleAlert, CircleCheck, CircleX, Info } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import { cn } from '@/lib/utils'

export type InfoBarSeverity = 'info' | 'success' | 'warning' | 'error'

const SEVERITY: Record<InfoBarSeverity, { icon: LucideIcon; surface: string; tint: string }> = {
  info: { icon: Info, surface: 'bg-infobar-info', tint: 'text-status-info' },
  success: { icon: CircleCheck, surface: 'bg-infobar-ok', tint: 'text-status-ok' },
  warning: { icon: CircleAlert, surface: 'bg-infobar-warn', tint: 'text-status-warn' },
  error: { icon: CircleX, surface: 'bg-infobar-error', tint: 'text-status-error' }
}

/** Fluent InfoBar: a persistent, inline status message with optional actions. */
export function InfoBar({
  severity,
  title,
  message,
  actions,
  className
}: {
  severity: InfoBarSeverity
  title: string
  message?: string
  actions?: React.ReactNode
  className?: string
}): React.JSX.Element {
  const { icon: Icon, surface, tint } = SEVERITY[severity]
  return (
    <div
      role={severity === 'error' || severity === 'warning' ? 'alert' : 'status'}
      className={cn(
        'flex items-center gap-3 rounded-md border px-4 py-3',
        surface,
        className
      )}
    >
      <Icon className={cn('size-4 shrink-0', tint)} aria-hidden />
      <div className="flex min-w-0 flex-1 flex-wrap items-baseline gap-x-3 gap-y-0.5">
        <span className="text-sm font-semibold">{title}</span>
        {message && <span className="text-sm text-foreground/85">{message}</span>}
      </div>
      {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
    </div>
  )
}
