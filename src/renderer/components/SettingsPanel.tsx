import { useMutation, type UseMutationResult } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import {
  EASY_VIRTUAL_DISPLAY_ERROR_CODES,
  type EasyVirtualDisplayErrorCode,
  type AppSettings,
  type AppSnapshot
} from '../../shared'
import { useAdminConfigForm } from './settings-panel/useAdminConfigForm'
import { BehaviorSettingsSection } from './settings-panel/BehaviorSettingsSection'
import { DisplaySettingsSection } from './settings-panel/DisplaySettingsSection'
import { AppearanceSettingsSection } from './settings-panel/AppearanceSettingsSection'
import { AdvancedSettingsSection } from './settings-panel/AdvancedSettingsSection'
import { DangerZoneSection } from './settings-panel/DangerZoneSection'

interface SettingsPanelProps {
  snapshot: AppSnapshot
}

/* ── useUpdateSettings hook ──────────────────────────────────── */

function useUpdateSettings(): UseMutationResult<void, unknown, Partial<AppSettings>> {
  const { t } = useTranslation()

  return useMutation({
    mutationFn: (patch: Partial<AppSettings>) => window.easyVirtualDisplay.updateSettings(patch),
    onError: (error: unknown) => {
      const code = (error as Record<string, unknown>)?.code
      if (EASY_VIRTUAL_DISPLAY_ERROR_CODES.includes(code as EasyVirtualDisplayErrorCode)) {
        toast.error(t(`common:errors.${code}`))
      } else {
        toast.error(t('common:settings_update_failed'))
      }
    }
  })
}

/* ── Main component ──────────────────────────────────────────── */

export function SettingsPanel({ snapshot }: SettingsPanelProps): React.JSX.Element {
  const { settings, host } = snapshot
  const { mutate: updateSetting, isPending: isSettingPending } = useUpdateSettings()
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
