const IPC_NAMESPACE = 'easy-virtual-display'

export const rendererInvokeChannels = {
  getSnapshot: `${IPC_NAMESPACE}:get-snapshot`,
  installDriver: `${IPC_NAMESPACE}:install-driver`,
  uninstallDriver: `${IPC_NAMESPACE}:uninstall-driver`,
  addDisplay: `${IPC_NAMESPACE}:add-display`,
  removeDisplay: `${IPC_NAMESPACE}:remove-display`,
  removeAllDisplays: `${IPC_NAMESPACE}:remove-all-displays`,
  setDisplayMode: `${IPC_NAMESPACE}:set-display-mode`,
  applyAdminConfig: `${IPC_NAMESPACE}:apply-admin-config`,
  updateSettings: `${IPC_NAMESPACE}:update-settings`,
  openDisplaySettings: `${IPC_NAMESPACE}:open-display-settings`,
  showMainWindow: `${IPC_NAMESPACE}:show-main-window`
} as const

export const rendererEventChannels = {
  snapshotChanged: `${IPC_NAMESPACE}:snapshot-changed`,
  languageChanged: `${IPC_NAMESPACE}:language-changed`
} as const

export const hostRpcMethods = {
  getSnapshot: 'host.getSnapshot',
  addDisplay: 'host.addDisplay',
  removeDisplay: 'host.removeDisplay',
  removeAllDisplays: 'host.removeAllDisplays',
  setDisplayMode: 'host.setDisplayMode',
  snapshotChanged: 'host.snapshotChanged'
} as const

export type RendererInvokeChannel =
  (typeof rendererInvokeChannels)[keyof typeof rendererInvokeChannels]

export type RendererEventChannel =
  (typeof rendererEventChannels)[keyof typeof rendererEventChannels]

export type HostRpcMethod = (typeof hostRpcMethods)[keyof typeof hostRpcMethods]
