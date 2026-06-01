import { useTranslation } from 'react-i18next'
import type { AppSettings } from '../../../shared'
import {
  SectionTitle,
  SettingRow,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue
} from './SettingsPrimitives'

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
    <section>
      <SectionTitle>{t('settings:appearance_title')}</SectionTitle>
      <div className="bg-card border border-border rounded-lg divide-y divide-border">
        <div className="px-4">
          <SettingRow label={t('settings:theme')}>
            <Select
              value={settings.theme}
              onValueChange={(v) => onUpdate({ theme: v as AppSettings['theme'] })}
              disabled={disabled}
            >
              <SelectTrigger className="w-32 h-8 text-xs bg-input border-border">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="system" className="text-xs">
                  {t('common:theme_system')}
                </SelectItem>
                <SelectItem value="light" className="text-xs">
                  {t('common:theme_light')}
                </SelectItem>
                <SelectItem value="dark" className="text-xs">
                  {t('common:theme_dark')}
                </SelectItem>
              </SelectContent>
            </Select>
          </SettingRow>
        </div>
        <div className="px-4">
          <SettingRow label={t('settings:language')}>
            <Select
              value={settings.language}
              onValueChange={(v) => onUpdate({ language: v as AppSettings['language'] })}
              disabled={disabled}
            >
              <SelectTrigger className="w-32 h-8 text-xs bg-input border-border">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="system" className="text-xs">
                  {t('settings:lang_system')}
                </SelectItem>
                <SelectItem value="en" className="text-xs">
                  {t('settings:lang_en')}
                </SelectItem>
                <SelectItem value="zh-CN" className="text-xs">
                  {t('settings:lang_zh_cn')}
                </SelectItem>
              </SelectContent>
            </Select>
          </SettingRow>
        </div>
      </div>
    </section>
  )
}
