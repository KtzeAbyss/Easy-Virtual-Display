import { useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import {
  EASY_VIRTUAL_DISPLAY_ERROR_CODES,
  type EasyVirtualDisplayErrorCode,
  type HostSnapshot
} from '../../../shared'
import { Button, SectionTitle, Trash2 } from './SettingsPrimitives'

interface DangerZoneSectionProps {
  host: HostSnapshot
}

export function DangerZoneSection({ host }: DangerZoneSectionProps): React.JSX.Element {
  const { t } = useTranslation()
  const [confirming, setConfirming] = useState(false)
  const isDriverPresent = host.status !== 'not_installed'

  const { mutate: uninstall, isPending } = useMutation({
    mutationFn: () => window.easyVirtualDisplay.uninstallDriver(),
    onSuccess: () => {
      setConfirming(false)
      toast.success(t('settings:uninstall_driver_success'))
    },
    onError: (error: unknown) => {
      const code = (error as Record<string, unknown> | null)?.code
      if (EASY_VIRTUAL_DISPLAY_ERROR_CODES.includes(code as EasyVirtualDisplayErrorCode)) {
        toast.error(t(`common:errors.${code}`))
      } else {
        toast.error(t('common:operation_failed'))
      }
    }
  })

  return (
    <section>
      <SectionTitle>{t('settings:danger_zone_title')}</SectionTitle>
      <div className="bg-card border border-destructive/30 rounded-lg p-4 flex flex-col gap-3">
        <div className="flex items-start justify-between gap-4">
          <div className="flex flex-col gap-1">
            <p className="text-sm font-medium text-foreground">{t('settings:uninstall_driver')}</p>
            <p className="text-xs text-muted-foreground">{t('settings:uninstall_driver_desc')}</p>
            {!isDriverPresent && (
              <p className="text-xs text-muted-foreground">
                {t('common:errors.driver_uninstall_not_installed')}
              </p>
            )}
          </div>
          {!confirming && (
            <Button
              type="button"
              variant="destructive"
              size="sm"
              className="gap-2 shrink-0"
              disabled={!isDriverPresent || isPending}
              onClick={() => setConfirming(true)}
            >
              <Trash2 className="w-3.5 h-3.5" />
              {t('settings:uninstall_driver_action')}
            </Button>
          )}
        </div>

        {confirming && (
          <div className="border-t border-destructive/30 pt-3 flex flex-col gap-3">
            <p className="text-sm font-medium text-foreground">
              {t('settings:uninstall_driver_confirm_title')}
            </p>
            <p className="text-xs text-muted-foreground">
              {t('settings:uninstall_driver_confirm_body')}
            </p>
            <div className="flex gap-2 self-end">
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={isPending}
                onClick={() => setConfirming(false)}
              >
                {t('common:cancel')}
              </Button>
              <Button
                type="button"
                variant="destructive"
                size="sm"
                disabled={isPending}
                onClick={() => uninstall()}
              >
                {isPending
                  ? t('settings:uninstalling_driver')
                  : t('settings:uninstall_driver_confirm_action')}
              </Button>
            </div>
          </div>
        )}
      </div>
    </section>
  )
}
