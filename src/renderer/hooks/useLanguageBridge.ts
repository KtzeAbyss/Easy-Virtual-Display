import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'

export function useLanguageBridge(): void {
  const { i18n } = useTranslation()

  useEffect(() => {
    const unsubscribe = window.easyVirtualDisplay.onLanguageChanged((language) => {
      i18n.changeLanguage(language)
    })
    return unsubscribe
  }, [i18n])
}
