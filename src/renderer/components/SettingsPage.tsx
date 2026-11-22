import { useTranslation } from 'react-i18next'
import type { AppSettings, AppSnapshot } from '../../shared'
import { useBridgeCommand } from '../hooks/useBridgeCommand'
import { PageHeader } from './layout/Page'
import { BehaviorSettingsSection } from './settings-panel/BehaviorSettingsSection'
import { DisplaySettingsSection } from './settings-panel/DisplaySettingsSection'
import { AppearanceSettingsSection } from './settings-panel/AppearanceSettingsSection'

/** App preferences only; every control applies immediately. Driver config lives on the driver page. */
export function SettingsPage({ snapshot }: { snapshot: AppSnapshot }): React.JSX.Element {
  const { t } = useTranslation()
  const { settings } = snapshot
  const { mutate: updateSetting, isPending } = useBridgeCommand(
    (patch: Partial<AppSettings>) => window.easyVirtualDisplay.updateSettings(patch),
    'common:settings_update_failed'
  )

  return (
    <>
      <PageHeader title={t('settings:title')} />
      <div className="flex flex-col gap-1">
        <BehaviorSettingsSection settings={settings} onUpdate={updateSetting} disabled={isPending} />
        <DisplaySettingsSection
          settings={settings}
          fallbackError={snapshot.fallbackError}
          onUpdate={updateSetting}
          disabled={isPending}
        />
        <AppearanceSettingsSection
          settings={settings}
          onUpdate={updateSetting}
          disabled={isPending}
        />
      </div>
    </>
  )
}
