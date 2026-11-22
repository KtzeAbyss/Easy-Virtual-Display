import { useTranslation } from 'react-i18next'
import { Cpu, Info, Monitor, Settings } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import type { DriverStatus } from '../../shared'
import { cn } from '@/lib/utils'
import { HEALTH_DOT, driverHealth } from '../lib/driver-status'

export type AppPage = 'displays' | 'driver' | 'settings' | 'about'

const PRIMARY: { page: AppPage; icon: LucideIcon }[] = [
  { page: 'displays', icon: Monitor },
  { page: 'driver', icon: Cpu }
]

const FOOTER: { page: AppPage; icon: LucideIcon }[] = [
  { page: 'settings', icon: Settings },
  { page: 'about', icon: Info }
]

function NavItem({
  page,
  icon: Icon,
  selected,
  badge,
  onSelect
}: {
  page: AppPage
  icon: LucideIcon
  selected: boolean
  badge?: string
  onSelect: (page: AppPage) => void
}): React.JSX.Element {
  const { t } = useTranslation()
  const label = t(`common:nav.${page}`)
  return (
    <li>
      <button
        type="button"
        aria-current={selected ? 'page' : undefined}
        aria-label={label}
        title={label}
        onClick={() => onSelect(page)}
        className={cn(
          'relative flex h-9 w-full items-center gap-3 rounded-md px-3 text-sm outline-none transition-colors',
          'focus-visible:ring-2 focus-visible:ring-ring',
          selected ? 'bg-nav-selected' : 'hover:bg-nav-hover active:text-muted-foreground'
        )}
      >
        {selected && (
          <span className="absolute top-1/2 left-0 h-4 w-[3px] -translate-y-1/2 rounded-full bg-primary" />
        )}
        <span className="relative">
          <Icon className="size-4" aria-hidden />
          {badge && (
            <span className={cn('absolute -top-0.5 -right-0.5 size-2 rounded-full', badge)} />
          )}
        </span>
        <span className="hidden truncate min-[840px]:inline">{label}</span>
      </button>
    </li>
  )
}

/** Fluent NavigationView pane: labels collapse to icons when the window gets narrow. */
export function NavigationRail({
  page,
  onNavigate,
  driverStatus
}: {
  page: AppPage
  onNavigate: (page: AppPage) => void
  driverStatus: DriverStatus | undefined
}): React.JSX.Element {
  const { t } = useTranslation()
  const health = driverStatus ? driverHealth(driverStatus) : 'ok'
  const driverBadge = health === 'ok' ? undefined : HEALTH_DOT[health]
  return (
    <nav
      aria-label={t('common:nav_label')}
      className="flex w-14 shrink-0 flex-col justify-between px-1.5 pt-1 pb-2 min-[840px]:w-[200px]"
    >
      <ul className="flex flex-col gap-0.5">
        {PRIMARY.map(({ page: p, icon }) => (
          <NavItem
            key={p}
            page={p}
            icon={icon}
            selected={page === p}
            badge={p === 'driver' ? driverBadge : undefined}
            onSelect={onNavigate}
          />
        ))}
      </ul>
      <ul className="flex flex-col gap-0.5">
        {FOOTER.map(({ page: p, icon }) => (
          <NavItem key={p} page={p} icon={icon} selected={page === p} onSelect={onNavigate} />
        ))}
      </ul>
    </nav>
  )
}
