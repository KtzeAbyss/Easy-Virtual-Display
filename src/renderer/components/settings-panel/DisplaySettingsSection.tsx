import { useTranslation } from 'react-i18next'
import type { AppSettings, EasyVirtualDisplayErrorCode } from '../../../shared'
import { SectionTitle, SettingRow, Switch } from './SettingsPrimitives'

interface DisplaySettingsSectionProps {
  settings: AppSettings
  fallbackError: EasyVirtualDisplayErrorCode | null
  onUpdate: (patch: Partial<AppSettings>) => void
  disabled: boolean
}

export function DisplaySettingsSection({
  settings,
  fallbackError,
  onUpdate,
  disabled
}: DisplaySettingsSectionProps): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <section>
      <SectionTitle>{t('settings:display_title')}</SectionTitle>
      <div className="bg-card border border-border rounded-lg divide-y divide-border">
        <div className="px-4">
          <SettingRow
            label={t('settings:fallback_display')}
            description={t('settings:fallback_display_desc')}
          >
            <Switch
              checked={settings.fallbackDisplay}
              onCheckedChange={(v) => onUpdate({ fallbackDisplay: v })}
              disabled={disabled}
            />
          </SettingRow>
          {fallbackError && (
            <p className="pb-2.5 text-xs text-destructive">
              {t('settings:fallback_display_failed', {
                reason: t(`common:errors.${fallbackError}`)
              })}
            </p>
          )}
        </div>
        <div className="px-4">
          <SettingRow
            label={t('settings:keep_screen_on')}
            description={t('settings:keep_screen_on_desc')}
          >
            <Switch
              checked={settings.keepScreenOn}
              onCheckedChange={(v) => onUpdate({ keepScreenOn: v })}
              disabled={disabled}
            />
          </SettingRow>
        </div>
      </div>
    </section>
  )
}
