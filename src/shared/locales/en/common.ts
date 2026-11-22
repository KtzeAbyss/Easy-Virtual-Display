import shell from './shell.json'

export default {
  ...shell.common,
  app_title: 'Easy Virtual Display',
  nav_label: 'Main navigation',
  nav: {
    displays: 'Displays',
    driver: 'Driver',
    settings: 'Settings',
    about: 'About'
  },
  window: {
    minimize: 'Minimize',
    maximize: 'Maximize',
    restore: 'Restore',
    close: 'Close'
  },
  retry: 'Retry',
  installing_driver: 'Installing driver…',
  more_actions: 'More actions',
  undo: 'Undo',
  theme_system: 'Use system setting',
  theme_light: 'Light',
  theme_dark: 'Dark',
  connecting: 'Starting background service…',
  connection_failed: 'Could not connect to the background service.',
  startup_failed_title: "Easy Virtual Display couldn't start",
  operation_failed: 'Operation failed',
  settings_update_failed: 'Could not update the setting',
  about: {
    description: 'Create and manage virtual displays on Windows.',
    version: 'Version {{version}}',
    driver_credit: 'Virtual displays are provided by the Parsec Virtual Display Driver.',
    links_title: 'Links',
    repository: 'Project home',
    issues: 'Report a problem',
    releases: 'Release notes',
    shortcuts_title: 'Keyboard shortcuts',
    shortcut_add_display: 'Add a display',
    shortcut_settings: 'Open settings'
  },
  errors: {
    driver_not_installed: 'The driver is not installed',
    driver_disabled: 'The driver is disabled',
    driver_restart_required: 'Restart your PC to finish the driver change',
    driver_error: 'The driver reported an error',
    limit_exceeded: 'The display limit has been reached',
    display_not_found: 'Display not found',
    unsupported_mode: 'Unsupported display mode',
    admin_cancelled: 'Administrator approval was cancelled. Nothing was changed.',
    driver_installer_missing: 'The bundled driver installer is missing',
    driver_uninstall_failed: 'Could not uninstall the virtual display driver',
    driver_uninstall_not_installed: 'The virtual display driver is not installed',
    dotnet_runtime_missing:
      'Microsoft .NET 8 Runtime (x64) is required to start the background service',
    native_host_unavailable: 'The background service is unavailable',
    config_apply_timeout:
      'The configuration was submitted but not confirmed yet. It should take effect shortly.',
    viewer_unsupported: 'Preview is only supported on Windows',
    viewer_start_failed: 'Could not open the preview window',
    launch_on_login_failed: 'Could not change the startup setting',
    settings_save_failed: 'Could not save settings',
    invalid_request: 'Invalid request',
    needs_elevation: 'Administrator approval is required'
  }
}
