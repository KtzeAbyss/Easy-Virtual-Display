import { useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { HostSnapshot } from '../../../shared'
import { toastBridgeError } from '../../lib/bridge-errors'
import { Button } from '../ui/button'
import { ConfirmDialog } from '../ui/dialog'
import { SettingRow } from '../layout/Page'

export function UninstallDriverRow({ host }: { host: HostSnapshot }): React.JSX.Element {
  const { t } = useTranslation()
  const [confirming, setConfirming] = useState(false)
  const isDriverPresent = host.status !== 'not_installed'

  const { mutate: uninstall, isPending } = useMutation({
    mutationFn: () => window.easyVirtualDisplay.uninstallDriver(),
    onSuccess: () => {
      setConfirming(false)
      toast.success(t('driver:uninstall_success'))
    },
    onError: (error: unknown) => {
      toastBridgeError(t, error)
    }
  })

  return (
    <>
      <SettingRow
        label={t('driver:uninstall')}
        description={
          isDriverPresent
            ? t('driver:uninstall_desc')
            : t('common:errors.driver_uninstall_not_installed')
        }
      >
        <Button
          type="button"
          variant="outline"
          disabled={!isDriverPresent || isPending}
          onClick={() => setConfirming(true)}
        >
          {isPending ? t('driver:uninstalling') : t('driver:uninstall_action')}
        </Button>
      </SettingRow>
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={t('driver:uninstall_confirm_title')}
        description={t('driver:uninstall_confirm_body')}
        confirmLabel={isPending ? t('driver:uninstalling') : t('driver:uninstall_action')}
        cancelLabel={t('common:cancel')}
        pending={isPending}
        onConfirm={() => uninstall()}
      />
    </>
  )
}
