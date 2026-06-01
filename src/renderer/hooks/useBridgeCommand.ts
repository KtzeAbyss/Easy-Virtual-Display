import { useMutation, type UseMutationResult } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { EASY_VIRTUAL_DISPLAY_ERROR_CODES, type EasyVirtualDisplayErrorCode } from '../../shared'

export function useBridgeCommand<TInput = void>(
  fn: (input: TInput) => Promise<void>,
): UseMutationResult<void, unknown, TInput> {
  const { t } = useTranslation()

  return useMutation({
    mutationFn: fn,
    onError: (error: unknown) => {
      const code = (error as Record<string, unknown>)?.code
      if (EASY_VIRTUAL_DISPLAY_ERROR_CODES.includes(code as EasyVirtualDisplayErrorCode)) {
        toast.error(t(`common:errors.${code}`))
      } else {
        toast.error(t('common:operation_failed'))
      }
    }
  })
}
