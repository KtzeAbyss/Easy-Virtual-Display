import { useTranslation } from 'react-i18next'
import type { AppSettings } from '../../../shared'
import { SectionTitle, SettingRow, Switch } from './SettingsPrimitives'

interface BehaviorSettingsSectionProps {
  settings: AppSettings
  onUpdate: (patch: Partial<AppSettings>) => void
  disabled: boolean
}

export function BehaviorSettingsSection({
  settings,
  onUpdate,
  disabled
}: BehaviorSettingsSectionProps): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <section>
      <SectionTitle>{t('settings:behavior_title')}</SectionTitle>
      <div className="bg-card border border-border rounded-lg divide-y divide-border">
        <div className="px-4">
          <SettingRow
            label={t('settings:launch_on_login')}
            description={t('settings:launch_on_login_desc')}
          >
            <Switch
              checked={settings.launchOnLogin}
              onCheckedChange={(v) => onUpdate({ launchOnLogin: v })}
              disabled={disabled}
            />
          </SettingRow>
        </div>
        <div className="px-4">
          <SettingRow
            label={t('settings:close_to_tray')}
            description={t('settings:close_to_tray_desc')}
          >
            <Switch
              checked={settings.closeToTray}
              onCheckedChange={(v) => onUpdate({ closeToTray: v })}
              disabled={disabled}
            />
          </SettingRow>
        </div>
        <div className="px-4">
          <SettingRow
            label={t('settings:start_minimized')}
            description={t('settings:start_minimized_desc')}
          >
            <Switch
              checked={settings.startMinimized}
              onCheckedChange={(v) => onUpdate({ startMinimized: v })}
              disabled={disabled}
            />
          </SettingRow>
        </div>
      </div>
    </section>
  )
}
