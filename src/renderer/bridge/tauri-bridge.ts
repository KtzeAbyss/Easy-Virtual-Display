import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import {
  rendererEventChannels,
  type AppSettings,
  type AppSnapshot,
  type ApplyAdminConfigInput,
  type EasyVirtualDisplayBridge,
  type EffectiveLanguage,
  type SetDisplayModeInput,
  type SnapshotListener,
  type Unsubscribe
} from '../../shared'

/**
 * Seam A contract requires `subscribeSnapshot` / `onLanguageChanged` to return an
 * `Unsubscribe` synchronously — but Tauri's `listen()` is async. This adapter returns
 * immediately and handles the race where the caller unsubscribes before `listen()`
 * resolves: in that case we invoke the `UnlistenFn` as soon as it arrives.
 */
function syncSubscribe<T>(event: string, callback: (payload: T) => void): Unsubscribe {
  let unlistenFn: UnlistenFn | null = null
  let cancelled = false

  listen<T>(event, (e) => {
    if (cancelled) {
      return
    }
    callback(e.payload)
  })
    .then((fn) => {
      if (cancelled) {
        fn()
      } else {
        unlistenFn = fn
      }
    })
    .catch(() => {
      // Tauri internals unavailable — nothing to clean up.
    })

  return () => {
    cancelled = true
    if (unlistenFn) {
      unlistenFn()
      unlistenFn = null
    }
  }
}

export function createTauriBridge(): EasyVirtualDisplayBridge {
  return {
    getSnapshot: () => invoke<AppSnapshot>('get_snapshot'),
    subscribeSnapshot: (listener: SnapshotListener) =>
      syncSubscribe<AppSnapshot>(rendererEventChannels.snapshotChanged, listener),
    onLanguageChanged: (callback) =>
      syncSubscribe<EffectiveLanguage>(rendererEventChannels.languageChanged, callback),
    installDriver: () => invoke<void>('install_driver'),
    uninstallDriver: () => invoke<void>('uninstall_driver'),
    addDisplay: () => invoke<void>('add_display'),
    removeDisplay: (index?: number) => invoke<void>('remove_display', { index }),
    removeAllDisplays: () => invoke<void>('remove_all_displays'),
    setDisplayMode: (input: SetDisplayModeInput) => invoke<void>('set_display_mode', { input }),
    applyAdminConfig: (input: ApplyAdminConfigInput) =>
      invoke<void>('apply_admin_config', { input }),
    updateSettings: (patch: Partial<AppSettings>) => invoke<void>('update_settings', { patch }),
    openDisplaySettings: () => invoke<void>('open_display_settings'),
    showMainWindow: () => invoke<void>('show_main_window')
  }
}
