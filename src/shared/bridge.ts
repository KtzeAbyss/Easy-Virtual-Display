import type {
  AppSettings,
  AppSnapshot,
  DisplayMode,
  EffectiveLanguage,
  Orientation,
  ParentGpu
} from './contracts'

export interface SetDisplayModeInput {
  index: number
  width?: number
  height?: number
  hz?: number
  orientation?: Orientation
}

export interface ApplyAdminConfigInput {
  customModes: DisplayMode[]
  parentGpu: ParentGpu
}

export type SnapshotListener = (snapshot: AppSnapshot) => void

export type Unsubscribe = () => void

/**
 * All mutating commands intentionally resolve to void.
 * Snapshot updates remain the single source of truth.
 */
export interface EasyVirtualDisplayBridge {
  getSnapshot(): Promise<AppSnapshot>
  subscribeSnapshot(listener: SnapshotListener): Unsubscribe
  onLanguageChanged(callback: (language: EffectiveLanguage) => void): Unsubscribe
  installDriver(): Promise<void>
  uninstallDriver(): Promise<void>
  addDisplay(): Promise<void>
  removeDisplay(index?: number): Promise<void>
  removeAllDisplays(): Promise<void>
  setDisplayMode(input: SetDisplayModeInput): Promise<void>
  applyAdminConfig(input: ApplyAdminConfigInput): Promise<void>
  updateSettings(patch: Partial<AppSettings>): Promise<void>
  openDisplaySettings(): Promise<void>
  showMainWindow(): Promise<void>
}
