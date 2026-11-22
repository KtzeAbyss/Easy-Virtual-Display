export default {
  title: 'Driver',
  status: {
    ok: 'Driver OK',
    not_installed: 'Not Installed',
    disabled: 'Disabled',
    disabled_service: 'Service Disabled',
    restart_required: 'Restart Required',
    inaccessible: 'Inaccessible',
    driver_error: 'Driver Error',
    unknown_problem: 'Unknown Problem',
    unknown: 'Unknown'
  },
  labels: {
    status: 'Status',
    driver_version: 'Driver version',
    max_displays: 'Display limit',
    active_displays: 'Active displays',
    parent_gpu: 'Render GPU'
  },
  gpu_options: {
    auto: 'Automatic',
    nvidia: 'NVIDIA',
    amd: 'AMD'
  },
  infobar: {
    not_installed: {
      title: 'Driver not installed',
      message: 'Install it to start creating virtual displays.'
    },
    restart_required: {
      title: 'Restart required',
      message: 'The driver change takes effect after you restart your PC.'
    },
    disabled: {
      title: 'Driver disabled',
      message: 'Enable "Parsec Virtual Display Adapter" in Device Manager.'
    },
    unavailable: {
      title: 'Driver unavailable',
      message: 'Display actions are paused. Reinstalling the driver may help.'
    }
  },
  open_driver_page: 'View driver',
  status_title: 'Status',
  modes_title: 'Custom modes',
  custom_resolutions: 'Custom resolutions',
  custom_modes_hint: 'Up to 5 extra modes, offered by every virtual display.',
  no_custom_modes: 'No custom modes',
  add_mode: 'Add mode',
  add_custom_mode: 'Custom…',
  invalid_values: 'Enter positive whole numbers',
  placeholder_width: 'Width',
  placeholder_height: 'Height',
  placeholder_hz: 'Hz',
  gpu_selection_desc: 'Which graphics card renders the virtual displays',
  uac_note: 'Applying changes requires administrator approval.',
  apply: 'Apply',
  applying: 'Applying…',
  discard: 'Discard',
  config_saved: 'Configuration applied',
  maintenance_title: 'Maintenance',
  uninstall: 'Uninstall driver',
  uninstall_desc: 'Remove the Parsec Virtual Display Driver from this PC. You can reinstall it at any time.',
  uninstall_action: 'Uninstall',
  uninstalling: 'Uninstalling…',
  uninstall_confirm_title: 'Uninstall the virtual display driver?',
  uninstall_confirm_body:
    'All virtual displays will be removed and the driver will be uninstalled. Administrator approval is required.',
  uninstall_success: 'Virtual display driver uninstalled'
}
