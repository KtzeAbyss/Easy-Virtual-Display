import type { AppSettings, AppSnapshot } from '../../shared'
import { useBridgeCommand } from '../hooks/useBridgeCommand'
import { useAdminConfigForm } from './settings-panel/useAdminConfigForm'
import { BehaviorSettingsSection } from './settings-panel/BehaviorSettingsSection'
import { DisplaySettingsSection } from './settings-panel/DisplaySettingsSection'
import { AppearanceSettingsSection } from './settings-panel/AppearanceSettingsSection'
import { AdvancedSettingsSection } from './settings-panel/AdvancedSettingsSection'
import { DangerZoneSection } from './settings-panel/DangerZoneSection'

interface SettingsPanelProps {
  snapshot: AppSnapshot
}

export function SettingsPanel({ snapshot }: SettingsPanelProps): React.JSX.Element {
  const { settings, host } = snapshot
  const { mutate: updateSetting, isPending: isSettingPending } = useBridgeCommand(
    (patch: Partial<AppSettings>) => window.easyVirtualDisplay.updateSettings(patch),
    'common:settings_update_failed'
  )
  const adminForm = useAdminConfigForm(host)

  return (
    <div className="flex flex-col gap-6">
      <BehaviorSettingsSection
        settings={settings}
        onUpdate={updateSetting}
        disabled={isSettingPending}
      />
      <DisplaySettingsSection
        settings={settings}
        fallbackError={snapshot.fallbackError}
        onUpdate={updateSetting}
        disabled={isSettingPending}
      />
      <AppearanceSettingsSection
        settings={settings}
        onUpdate={updateSetting}
        disabled={isSettingPending}
      />
      <AdvancedSettingsSection adminForm={adminForm} />
      <DangerZoneSection host={host} />
    </div>
  )
}
