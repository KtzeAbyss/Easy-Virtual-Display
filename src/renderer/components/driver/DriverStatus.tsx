import { useTranslation } from 'react-i18next'
import { LoaderCircle, Shield } from 'lucide-react'
import type { DriverStatus } from '../../../shared'
import { cn } from '@/lib/utils'
import { HEALTH_DOT, driverHealth, driverInfoBar } from '../../lib/driver-status'
import { useBridgeCommand } from '../../hooks/useBridgeCommand'
import { Button } from '../ui/button'
import { InfoBar } from '../ui/info-bar'

export function DriverStatusChip({
  status,
  className
}: {
  status: DriverStatus
  className?: string
}): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <span className={cn('inline-flex items-center gap-1.5 text-xs text-muted-foreground', className)}>
      <span className={cn('size-2 shrink-0 rounded-full', HEALTH_DOT[driverHealth(status)])} />
      {t(`driver:status.${status}`)}
    </span>
  )
}

export function InstallDriverButton({
  variant = 'default'
}: {
  variant?: 'default' | 'outline'
}): React.JSX.Element {
  const { t } = useTranslation()
  const installDriver = useBridgeCommand<void>(() => window.easyVirtualDisplay.installDriver())
  return (
    <Button
      type="button"
      variant={variant}
      onClick={() => installDriver.mutate()}
      disabled={installDriver.isPending}
    >
      {installDriver.isPending ? <LoaderCircle className="animate-spin" /> : <Shield />}
      {installDriver.isPending ? t('common:installing_driver') : t('common:install_driver_action')}
    </Button>
  )
}

/** Explains why display actions are unavailable and offers the next step. */
export function DriverStatusInfoBar({
  status,
  onOpenDriverPage
}: {
  status: DriverStatus
  onOpenDriverPage?: () => void
}): React.JSX.Element | null {
  const { t } = useTranslation()
  const info = driverInfoBar(status)
  if (!info) return null

  let action: React.ReactNode = null
  if (status === 'not_installed') {
    action = <InstallDriverButton />
  } else if (onOpenDriverPage) {
    action = (
      <Button type="button" variant="outline" onClick={onOpenDriverPage}>
        {t('driver:open_driver_page')}
      </Button>
    )
  }

  return (
    <InfoBar
      severity={info.severity}
      title={t(`driver:infobar.${info.key}.title`)}
      message={t(`driver:infobar.${info.key}.message`)}
      actions={action}
    />
  )
}
