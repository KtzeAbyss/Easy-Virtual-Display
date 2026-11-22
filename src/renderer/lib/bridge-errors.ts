import { toast } from 'sonner'
import { parseBridgeError, type EasyVirtualDisplayErrorCode } from '../../shared'

type Translate = (key: string) => string

/**
 * Show a toast for a bridge/command rejection. Known codes use `common:errors.*`;
 * otherwise the provided fallback i18n key is used.
 */
export function toastBridgeError(
  t: Translate,
  error: unknown,
  fallbackKey = 'common:operation_failed'
): EasyVirtualDisplayErrorCode | undefined {
  const code = parseBridgeError(error)
  if (code) {
    toast.error(t(`common:errors.${code}`))
    return code
  }
  toast.error(t(fallbackKey))
  return undefined
}
