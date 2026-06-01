import enCommon from './en/common'
import enDriver from './en/driver'
import enDisplays from './en/displays'
import enSettings from './en/settings'
import enTray from './en/tray'
import zhCommon from './zh-CN/common'
import zhDriver from './zh-CN/driver'
import zhDisplays from './zh-CN/displays'
import zhSettings from './zh-CN/settings'
import zhTray from './zh-CN/tray'

export const resources = {
  en: {
    common: enCommon,
    driver: enDriver,
    displays: enDisplays,
    settings: enSettings,
    tray: enTray
  },
  'zh-CN': {
    common: zhCommon,
    driver: zhDriver,
    displays: zhDisplays,
    settings: zhSettings,
    tray: zhTray
  }
} as const

export type LocaleNamespace = keyof typeof resources.en
