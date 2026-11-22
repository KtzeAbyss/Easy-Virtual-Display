export default {
  title: 'Displays',
  count_of_max: '{{count}} of {{max}}',
  add_display: 'Add display',
  limit_reached: 'The driver supports up to {{max}} displays',
  remove: 'Remove',
  remove_all: 'Remove all',
  remove_all_confirm_title: 'Remove all virtual displays?',
  remove_all_confirm_body:
    '{{count}} virtual displays will be removed. Windows on them will move to your other displays.',
  open_display_settings: 'System display settings',
  state_active: 'Active',
  state_inactive: 'Inactive',
  device_name: 'Device: {{name}}',
  unsupported_mode: 'Unsupported mode',
  resolution: 'Resolution',
  refresh_rate: 'Refresh rate',
  orientation: 'Orientation',
  orientations: {
    landscape: 'Landscape',
    portrait: 'Portrait',
    landscape_flipped: 'Landscape (flipped)',
    portrait_flipped: 'Portrait (flipped)'
  },
  hz: '{{value}} Hz',
  mode_changed: 'Display mode changed',
  empty_title: 'No virtual displays yet',
  empty_description: 'Select "Add display" or press Ctrl+N to create one.',
  empty_no_driver_title: 'Install the driver to get started',
  empty_no_driver_description:
    'Virtual displays are provided by the Parsec Virtual Display Driver. You only need to install it once.',
  open_viewer: 'Preview',
  close_viewer: 'Stop preview',
  viewer_hint:
    'Mirror this display in a window. Click inside it to take over the mouse and keyboard; the hotkey shown in the title bar gives them back. If an app on the display swallows that hotkey (e.g. full-screen Remote Desktop), press Ctrl+Alt+Del, then Esc.'
}
