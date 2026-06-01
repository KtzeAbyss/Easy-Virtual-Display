import i18next from 'i18next'
import type { i18n as I18nInstance } from 'i18next'
import { resources } from '../shared/locales'
import type { EffectiveLanguage } from '../shared'

export async function initRendererI18n(): Promise<I18nInstance> {
  const i18n = i18next.createInstance({
    resources,
    fallbackLng: 'en',
    initImmediate: false
  })
  i18n.init()

  try {
    const snapshot = await window.easyVirtualDisplay.getSnapshot()
    const language = snapshot.effectiveLanguage as EffectiveLanguage
    await i18n.changeLanguage(language)
  } catch {
    // Bridge unavailable — fallback to 'en', let React mount normally
  }

  return i18n
}
