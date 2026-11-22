import { useTranslation } from 'react-i18next'
import type { HostSnapshot } from '../../shared'
import { PageHeader, Section } from './layout/Page'
import { DriverStatusChip, DriverStatusInfoBar } from './driver/DriverStatus'
import { CustomModesForm } from './driver/CustomModesForm'
import { UninstallDriverRow } from './driver/UninstallDriverRow'
import { useAdminConfigForm } from './driver/useAdminConfigForm'

function Fact({ label, value }: { label: string; value: React.ReactNode }): React.JSX.Element {
  return (
    <div className="flex min-w-0 flex-col gap-0.5 px-3.5 py-2">
      <dt className="text-xs text-muted-foreground">{label}</dt>
      <dd className="selectable truncate text-sm tabular-nums">{value}</dd>
    </div>
  )
}

function DriverFacts({ host }: { host: HostSnapshot }): React.JSX.Element {
  const { t } = useTranslation()
  const installed = host.status !== 'not_installed'
  const activeCount = host.displays.filter((d) => d.active).length
  return (
    <dl className="grid grid-cols-2 divide-x divide-y rounded-md border bg-card sm:grid-cols-4 sm:divide-y-0">
      <Fact label={t('driver:labels.status')} value={<DriverStatusChip status={host.status} />} />
      <Fact label={t('driver:labels.driver_version')} value={installed ? host.driverVersion : '—'} />
      <Fact
        label={t('driver:labels.max_displays')}
        value={installed ? String(host.maxDisplays) : '—'}
      />
      <Fact label={t('driver:labels.active_displays')} value={String(activeCount)} />
    </dl>
  )
}

export function DriverPage({ host }: { host: HostSnapshot }): React.JSX.Element {
  const { t } = useTranslation()
  const adminForm = useAdminConfigForm(host)
  const installed = host.status !== 'not_installed'

  return (
    <>
      <PageHeader title={t('driver:title')} />
      <div className="flex flex-col gap-1">
        <DriverStatusInfoBar status={host.status} />
        <Section title={t('driver:status_title')}>
          <DriverFacts host={host} />
        </Section>
        {installed && (
          <Section title={t('driver:modes_title')}>
            <CustomModesForm adminForm={adminForm} />
          </Section>
        )}
        <Section title={t('driver:maintenance_title')}>
          <UninstallDriverRow host={host} />
        </Section>
      </div>
    </>
  )
}
