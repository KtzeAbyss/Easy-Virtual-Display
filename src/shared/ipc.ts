/**
 * Event channel names shared with the Tauri shell (`src-tauri/src/events.rs`).
 *
 * Command invoke names are the snake_case `#[tauri::command]` ids (e.g. `get_snapshot`)
 * and live only on the bridge / Rust side — they are not duplicated here.
 */
const IPC_NAMESPACE = 'easy-virtual-display'

export const rendererEventChannels = {
  snapshotChanged: `${IPC_NAMESPACE}:snapshot-changed`,
  languageChanged: `${IPC_NAMESPACE}:language-changed`
} as const

export type RendererEventChannel =
  (typeof rendererEventChannels)[keyof typeof rendererEventChannels]
