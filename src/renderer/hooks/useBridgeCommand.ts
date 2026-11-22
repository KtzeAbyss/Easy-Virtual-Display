import { useMutation, type UseMutationResult } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toastBridgeError } from '../lib/bridge-errors'

export function useBridgeCommand<TInput = void>(
  fn: (input: TInput) => Promise<void>,
  fallbackKey = 'common:operation_failed'
): UseMutationResult<void, unknown, TInput> {
  const { t } = useTranslation()

  return useMutation({
    mutationFn: fn,
    onError: (error: unknown) => {
      toastBridgeError(t, error, fallbackKey)
    }
  })
}
