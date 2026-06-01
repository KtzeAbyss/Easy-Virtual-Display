import shell from './shell.json'

export default {
  ...shell.common,
  app_title: 'Virtual Display Manager',
  tab_displays: 'Displays',
  tab_settings: 'Settings',
  retry: 'Retry',
  installing_driver: 'Installing driver...',
  remove: 'Remove',
  save: 'Save Configuration',
  saving: 'Saving...',
  add: 'Add',
  theme_system: 'System',
  theme_light: 'Light',
  theme_dark: 'Dark',
  connecting: 'Connecting to native host...',
  connection_failed: 'Failed to connect to the native host.',
  operation_failed: 'Operation failed',
  settings_update_failed: 'Failed to update settings',
  errors: {
    driver_not_installed: 'Driver is not installed',
    driver_disabled: 'Driver is disabled',
    driver_restart_required: 'Driver restart is required',
    driver_error: 'Driver error occurred',
    limit_exceeded: 'Display limit exceeded',
    display_not_found: 'Display not found',
    unsupported_mode: 'Unsupported display mode',
    admin_cancelled: 'UAC was cancelled — operation unchanged',
    driver_installer_missing: 'Bundled driver installer is missing',
    driver_uninstall_failed: 'Failed to uninstall the virtual display driver',
    driver_uninstall_not_installed: 'No virtual display driver is installed to uninstall',
    dotnet_runtime_missing:
      'Microsoft .NET 8 Runtime (x64) is required to start the bundled native host',
    native_host_unavailable: 'Native host is unavailable',
    config_apply_timeout:
      'Configuration submitted but not yet confirmed — the change should take effect shortly'
  }
}
