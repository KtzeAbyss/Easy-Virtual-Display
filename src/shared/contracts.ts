export const DRIVER_STATUSES = [
  'ok',
  'inaccessible',
  'unknown',
  'unknown_problem',
  'disabled',
  'driver_error',
  'restart_required',
  'disabled_service',
  'not_installed'
] as const

export type DriverStatus = (typeof DRIVER_STATUSES)[number]

export const PARENT_GPUS = ['auto', 'nvidia', 'amd'] as const

export type ParentGpu = (typeof PARENT_GPUS)[number]

export const ORIENTATIONS = [
  'landscape',
  'portrait',
  'landscape_flipped',
  'portrait_flipped'
] as const

export type Orientation = (typeof ORIENTATIONS)[number]

export const APP_THEMES = ['system', 'light', 'dark'] as const

export type AppTheme = (typeof APP_THEMES)[number]

export const APP_LANGUAGES = ['system', 'en', 'zh-CN'] as const
export type AppLanguage = (typeof APP_LANGUAGES)[number]
export type EffectiveLanguage = 'en' | 'zh-CN'

export interface DisplayMode {
  width: number
  height: number
  hz: number
}

export interface SupportedResolution {
  width: number
  height: number
  refreshRates: number[]
}

export interface DisplaySummary {
  index: number
  identifier: number
  deviceName: string
  displayName: string
  active: boolean
  currentMode: DisplayMode | null
  currentOrientation: Orientation
  supportedResolutions: SupportedResolution[]
  unsupportedCurrentMode: boolean
}

export interface HostSnapshot {
  revision: number
  status: DriverStatus
  driverVersion: string
  maxDisplays: number
  displays: DisplaySummary[]
  customModes: DisplayMode[]
  parentGpu: ParentGpu
}

export interface AppSettings {
  launchOnLogin: boolean
  closeToTray: boolean
  startMinimized: boolean
  fallbackDisplay: boolean
  keepScreenOn: boolean
  theme: AppTheme
  language: AppLanguage
}

export interface AppSnapshot {
  host: HostSnapshot
  settings: AppSettings
  effectiveLanguage: EffectiveLanguage
}

export const DEFAULT_APP_SETTINGS: AppSettings = {
  launchOnLogin: false,
  closeToTray: true,
  startMinimized: false,
  fallbackDisplay: false,
  keepScreenOn: false,
  theme: 'system',
  language: 'system'
}

export const EMPTY_HOST_SNAPSHOT: HostSnapshot = {
  revision: 0,
  status: 'unknown',
  driverVersion: 'pending',
  maxDisplays: 0,
  displays: [],
  customModes: [],
  parentGpu: 'auto'
}

export const EMPTY_APP_SNAPSHOT: AppSnapshot = {
  host: EMPTY_HOST_SNAPSHOT,
  settings: DEFAULT_APP_SETTINGS,
  effectiveLanguage: 'en'
}
