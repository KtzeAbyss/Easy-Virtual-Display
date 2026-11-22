import { useTranslation } from 'react-i18next'
import { CircleX } from 'lucide-react'
import type { AppSettings, EasyVirtualDisplayErrorCode } from '../../../shared'
import { Section, SettingRow } from '../layout/Page'
import { Switch } from '../ui/switch'

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
    <Section title={t('settings:display_title')}>
      <SettingRow
        label={t('settings:fallback_display')}
        description={t('settings:fallback_display_desc')}
        footer={
          fallbackError && (
            <p
              role="alert"
              className="flex items-center gap-2 border-t bg-infobar-error px-4 py-2.5 text-xs"
            >
              <CircleX className="size-3.5 shrink-0 text-status-error" aria-hidden />
              {t('settings:fallback_display_failed', {
                reason: t(`common:errors.${fallbackError}`)
              })}
            </p>
          )
        }
      >
        <Switch
          aria-label={t('settings:fallback_display')}
          checked={settings.fallbackDisplay}
          onCheckedChange={(v) => onUpdate({ fallbackDisplay: v })}
          disabled={disabled}
        />
      </SettingRow>
      <SettingRow
        label={t('settings:keep_screen_on')}
        description={t('settings:keep_screen_on_desc')}
      >
        <Switch
          aria-label={t('settings:keep_screen_on')}
          checked={settings.keepScreenOn}
          onCheckedChange={(v) => onUpdate({ keepScreenOn: v })}
          disabled={disabled}
        />
      </SettingRow>
    </Section>
  )
}
