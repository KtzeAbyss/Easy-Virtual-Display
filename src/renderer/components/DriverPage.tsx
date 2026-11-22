import { useTranslation } from 'react-i18next'
import type { HostSnapshot } from '../../shared'
import { PageHeader, Section } from './layout/Page'
import { DriverStatusChip, DriverStatusInfoBar } from './driver/DriverStatus'
import { CustomModesForm } from './driver/CustomModesForm'
import { UninstallDriverRow } from './driver/UninstallDriverRow'
import { useAdminConfigForm } from './driver/useAdminConfigForm'

function Fact({ label, value }: { label: string; value: React.ReactNode }): React.JSX.Element {
  return (
    <div className="flex items-center justify-between gap-4 px-4 py-2.5">
      <dt className="text-sm text-muted-foreground">{label}</dt>
      <dd className="selectable text-sm tabular-nums">{value}</dd>
    </div>
  )
}

function DriverFacts({ host }: { host: HostSnapshot }): React.JSX.Element {
  const { t } = useTranslation()
  const installed = host.status !== 'not_installed'
  const activeCount = host.displays.filter((d) => d.active).length
  return (
    <dl className="divide-y rounded-md border bg-card">
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
