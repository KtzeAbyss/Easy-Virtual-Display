import type { DriverStatus } from '../../shared'
import type { InfoBarSeverity } from '../components/ui/info-bar'

export type DriverHealth = 'ok' | 'warning' | 'error'

export function driverHealth(status: DriverStatus): DriverHealth {
  if (status === 'ok') return 'ok'
  if (status === 'restart_required' || status === 'not_installed') return 'warning'
  return 'error'
}

export const HEALTH_DOT: Record<DriverHealth, string> = {
  ok: 'bg-status-ok',
  warning: 'bg-status-warn',
  error: 'bg-status-error'
}

type InfoBarKey = 'not_installed' | 'restart_required' | 'disabled' | 'unavailable'

export function driverInfoBar(
  status: DriverStatus
): { key: InfoBarKey; severity: InfoBarSeverity } | null {
  switch (status) {
    case 'ok':
      return null
    case 'not_installed':
      return { key: 'not_installed', severity: 'info' }
    case 'restart_required':
      return { key: 'restart_required', severity: 'warning' }
    case 'disabled':
    case 'disabled_service':
      return { key: 'disabled', severity: 'error' }
    default:
      return { key: 'unavailable', severity: 'error' }
  }
}
