import { useTranslation } from 'react-i18next'
import type { AppSettings } from '../../../shared'
import { Section, SettingRow } from '../layout/Page'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select'

interface AppearanceSettingsSectionProps {
  settings: AppSettings
  onUpdate: (patch: Partial<AppSettings>) => void
  disabled: boolean
}

export function AppearanceSettingsSection({
  settings,
  onUpdate,
  disabled
}: AppearanceSettingsSectionProps): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <Section title={t('settings:appearance_title')}>
      <SettingRow label={t('settings:theme')}>
        <Select
          value={settings.theme}
          onValueChange={(v) => onUpdate({ theme: v as AppSettings['theme'] })}
          disabled={disabled}
        >
          <SelectTrigger className="w-40" aria-label={t('settings:theme')}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="system">{t('common:theme_system')}</SelectItem>
            <SelectItem value="light">{t('common:theme_light')}</SelectItem>
            <SelectItem value="dark">{t('common:theme_dark')}</SelectItem>
          </SelectContent>
        </Select>
      </SettingRow>
      <SettingRow label={t('settings:language')}>
        <Select
          value={settings.language}
          onValueChange={(v) => onUpdate({ language: v as AppSettings['language'] })}
          disabled={disabled}
        >
          <SelectTrigger className="w-40" aria-label={t('settings:language')}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="system">{t('settings:lang_system')}</SelectItem>
            <SelectItem value="en">{t('settings:lang_en')}</SelectItem>
            <SelectItem value="zh-CN">{t('settings:lang_zh_cn')}</SelectItem>
          </SelectContent>
        </Select>
      </SettingRow>
    </Section>
  )
}
