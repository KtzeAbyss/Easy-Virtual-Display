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
  'config_apply_timeout'
] as const

export type EasyVirtualDisplayErrorCode = (typeof EASY_VIRTUAL_DISPLAY_ERROR_CODES)[number]

export interface EasyVirtualDisplayError {
  code: EasyVirtualDisplayErrorCode
  message: string
  details?: Record<string, unknown>
}
