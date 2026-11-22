export const EASY_VIRTUAL_DISPLAY_ERROR_CODES = [
  'driver_not_installed',
  'driver_disabled',
  'driver_restart_required',
  'driver_error',
  'limit_exceeded',
  'display_not_found',
  'unsupported_mode',
  'admin_cancelled',
  'driver_installer_missing',
  'driver_uninstall_failed',
  'driver_uninstall_not_installed',
  'dotnet_runtime_missing',
  'native_host_unavailable',
  'config_apply_timeout',
  'viewer_unsupported',
  'viewer_start_failed',
  'launch_on_login_failed',
  'settings_save_failed',
  'invalid_request',
  'needs_elevation'
] as const

export type EasyVirtualDisplayErrorCode = (typeof EASY_VIRTUAL_DISPLAY_ERROR_CODES)[number]

export interface EasyVirtualDisplayError {
  code: EasyVirtualDisplayErrorCode
  message: string
  details?: Record<string, unknown>
}

/** Lift a thrown bridge/command rejection into a known app error code when possible. */
export function parseBridgeError(error: unknown): EasyVirtualDisplayErrorCode | undefined {
  if (!error || typeof error !== 'object') {
    return undefined
  }
  const code = (error as { code?: unknown }).code
  if (typeof code !== 'string') {
    return undefined
  }
  return EASY_VIRTUAL_DISPLAY_ERROR_CODES.includes(code as EasyVirtualDisplayErrorCode)
    ? (code as EasyVirtualDisplayErrorCode)
    : undefined
}
