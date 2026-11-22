import { useTranslation } from 'react-i18next'
import { LoaderCircle } from 'lucide-react'
import type { DriverStatus, HostSnapshot, ParentGpu } from '../../shared'
import { cn } from '@/lib/utils'
import { AlertTriangle, Monitor, Layers, Zap, Cpu } from 'lucide-react'
import { useBridgeCommand } from '../hooks/useBridgeCommand'
import { Button } from './ui/button'
import type { TFunction } from 'i18next'

interface DriverStatusBarProps {
  host: HostSnapshot
}

const STATUS_COLOR_MAP: Record<string, { color: string; dot: string; ok: boolean }> = {
  ok: { color: 'text-status-ok', dot: 'bg-status-ok', ok: true },
  restart_required: { color: 'text-status-warn', dot: 'bg-status-warn', ok: false }
}

const DEFAULT_STATUS_COLOR = {
  color: 'text-status-error',
  dot: 'bg-status-error',
  ok: false
}

function getStatusConfig(status: DriverStatus): { color: string; dot: string; ok: boolean } {
  return STATUS_COLOR_MAP[status] ?? DEFAULT_STATUS_COLOR
}

function getParentGpuLabel(t: TFunction, gpu: ParentGpu): string {
  const labels: Record<string, string> = {
    auto: t('driver:gpu_options.auto'),
    nvidia: t('driver:gpu_options.nvidia'),
    amd: t('driver:gpu_options.amd')
  }
  return labels[gpu] ?? gpu
}

function DriverStatusSummary({
  host,
  config
}: {
  host: HostSnapshot
  config: { color: string; dot: string; ok: boolean }
}): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <div className="flex items-center justify-between px-4 py-3 bg-card border border-border rounded-lg">
      <div className="flex items-center gap-2.5">
        <span className="relative flex h-2 w-2 shrink-0">
          <span
            className={cn(
              'absolute inline-flex h-full w-full rounded-full opacity-75',
              config.ok ? 'animate-ping' : '',
              config.dot
            )}
          />
          <span className={cn('relative inline-flex rounded-full h-2 w-2', config.dot)} />
        </span>
        <span className={cn('text-sm font-medium', config.color)}>
          {t('driver:labels.driver_status')}: {t(`driver:status.${host.status}`)}
        </span>
      </div>
      {!config.ok && (
        <div className="flex items-center gap-1.5 px-2.5 py-1 bg-status-warn/10 border border-status-warn/20 rounded-md">
          <AlertTriangle className="w-3.5 h-3.5 text-status-warn" />
          <span className="text-xs text-status-warn font-medium">
            {t('driver:warnings.driver_not_ready')}
          </span>
        </div>
      )}
    </div>
  )
}

function DriverInstallBanner(): React.JSX.Element {
  const { t } = useTranslation()
  const installDriver = useBridgeCommand<void>(() => window.easyVirtualDisplay.installDriver())
  return (
    <div className="flex items-center justify-between gap-3 px-4 py-3 bg-card border border-border rounded-lg">
      <div className="flex flex-col gap-1">
        <span className="text-sm font-medium text-foreground">
          {t('common:install_driver_message')}
        </span>
        <span className="text-xs text-muted-foreground">{t('common:install_driver_detail')}</span>
      </div>
      <Button
        type="button"
        onClick={() => installDriver.mutate()}
        disabled={installDriver.isPending}
      >
        {installDriver.isPending ? <LoaderCircle className="animate-spin" /> : null}
        {installDriver.isPending
          ? t('common:installing_driver')
          : t('common:install_driver_action')}
      </Button>
    </div>
  )
}

function DriverMetricsGrid({ host }: { host: HostSnapshot }): React.JSX.Element {
  const { t } = useTranslation()
  const activeCount = host.displays.filter((d) => d.active).length
  return (
    <div className="grid grid-cols-4 gap-2">
      <MetricCard
        icon={<Monitor className="w-3.5 h-3.5" />}
        label={t('driver:labels.active_displays')}
        value={String(activeCount)}
        suffix={`/ ${host.maxDisplays}`}
      />
      <MetricCard
        icon={<Layers className="w-3.5 h-3.5" />}
        label={t('driver:labels.max_displays')}
        value={String(host.maxDisplays)}
      />
      <MetricCard
        icon={<Zap className="w-3.5 h-3.5" />}
        label={t('driver:labels.driver_version')}
        value={host.driverVersion}
        mono
      />
      <MetricCard
        icon={<Cpu className="w-3.5 h-3.5" />}
        label={t('driver:labels.parent_gpu')}
        value={getParentGpuLabel(t, host.parentGpu)}
        mono
      />
    </div>
  )
}

export function DriverStatusBar({ host }: DriverStatusBarProps): React.JSX.Element {
  const config = getStatusConfig(host.status)
  return (
    <div className="flex flex-col gap-2">
      <DriverStatusSummary host={host} config={config} />
      {host.status === 'not_installed' && <DriverInstallBanner />}
      <DriverMetricsGrid host={host} />
    </div>
  )
}

function MetricCard({
  icon,
  label,
  value,
  suffix,
  mono
}: {
  icon: React.ReactNode
  label: string
  value: string
  suffix?: string
  mono?: boolean
}): React.JSX.Element {
  return (
    <div className="flex flex-col gap-1 px-3 py-2.5 bg-metric-bg border border-border rounded-lg">
      <div className="flex items-center gap-1.5 text-muted-foreground">
        {icon}
        <span className="text-xs">{label}</span>
      </div>
      <div className="flex items-baseline gap-1">
        <span className={cn('text-sm font-semibold text-foreground', mono && 'font-mono')}>
          {value}
        </span>
        {suffix && <span className="text-xs text-muted-foreground font-mono">{suffix}</span>}
      </div>
    </div>
  )
}
