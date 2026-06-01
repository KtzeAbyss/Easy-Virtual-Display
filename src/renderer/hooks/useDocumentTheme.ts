import { useEffect } from 'react'
import type { AppTheme } from '../../shared'

function applyThemeClass(theme: AppTheme): (() => void) | undefined {
  const root = document.documentElement

  if (theme === 'system') {
    const mq = window.matchMedia('(prefers-color-scheme: dark)')
    const apply = (): void => {
      root.classList.toggle('dark', mq.matches)
    }
    apply()
    mq.addEventListener('change', apply)
    return () => mq.removeEventListener('change', apply)
  }

  root.classList.toggle('dark', theme === 'dark')
  return undefined
}

export function useDocumentTheme(theme: AppTheme | undefined): void {
  useEffect(() => {
    if (!theme) return undefined
    return applyThemeClass(theme)
  }, [theme])
}
