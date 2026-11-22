import { useTranslation } from 'react-i18next'
import type { AppSettings } from '../../../shared'
import { Section, SettingRow } from '../layout/Page'
import { Switch } from '../ui/switch'

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
    <Section title={t('settings:behavior_title')}>
      <SettingRow
        label={t('settings:launch_on_login')}
        description={t('settings:launch_on_login_desc')}
      >
        <Switch
          aria-label={t('settings:launch_on_login')}
          checked={settings.launchOnLogin}
          onCheckedChange={(v) => onUpdate({ launchOnLogin: v })}
          disabled={disabled}
        />
      </SettingRow>
      <SettingRow
        label={t('settings:close_to_tray')}
        description={t('settings:close_to_tray_desc')}
      >
        <Switch
          aria-label={t('settings:close_to_tray')}
          checked={settings.closeToTray}
          onCheckedChange={(v) => onUpdate({ closeToTray: v })}
          disabled={disabled}
        />
      </SettingRow>
      <SettingRow
        label={t('settings:start_minimized')}
        description={t('settings:start_minimized_desc')}
      >
        <Switch
          aria-label={t('settings:start_minimized')}
          checked={settings.startMinimized}
          onCheckedChange={(v) => onUpdate({ startMinimized: v })}
          disabled={disabled}
        />
      </SettingRow>
    </Section>
  )
}
