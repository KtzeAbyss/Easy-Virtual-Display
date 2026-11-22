import { useEffect } from 'react'
import type { AppTheme } from '../../shared'

let materialRequest = 0

/** Mirror the theme onto the native frame; only go transparent once Mica is confirmed. */
function syncWindowMaterial(dark: boolean): void {
  const root = document.documentElement
  const request = ++materialRequest
  window.easyVirtualDisplay
    .applyWindowMaterial(dark)
    .then((mica) => {
      if (request === materialRequest) root.classList.toggle('mica', mica)
    })
    .catch(() => {
      if (request === materialRequest) root.classList.remove('mica')
    })
}

function applyThemeClass(theme: AppTheme): (() => void) | undefined {
  const root = document.documentElement
  const apply = (dark: boolean): void => {
    root.classList.toggle('dark', dark)
    syncWindowMaterial(dark)
  }

  if (theme === 'system') {
    const mq = window.matchMedia('(prefers-color-scheme: dark)')
    const onChange = (): void => apply(mq.matches)
    onChange()
    mq.addEventListener('change', onChange)
    return () => mq.removeEventListener('change', onChange)
  }

  apply(theme === 'dark')
  return undefined
}

export function useDocumentTheme(theme: AppTheme): void {
  useEffect(() => applyThemeClass(theme), [theme])
}
