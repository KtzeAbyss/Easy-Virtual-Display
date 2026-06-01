import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { useSnapshot } from './hooks/useSnapshot'
import { useLanguageBridge } from './hooks/useLanguageBridge'
import { useDocumentTheme } from './hooks/useDocumentTheme'
import {
  EASY_VIRTUAL_DISPLAY_ERROR_CODES,
  type AppTheme,
  type EasyVirtualDisplayErrorCode
} from '../shared'
import { TitleBar } from './components/TitleBar'
import { DriverStatusBar } from './components/DriverStatusBar'
import { DisplaysPanel } from './components/DisplaysPanel'
import { SettingsPanel } from './components/SettingsPanel'
import { Button } from './components/ui/button'

function LoadingScreen(): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <div className="min-h-screen bg-background font-sans flex items-center justify-center">
      <div className="flex flex-col items-center justify-center gap-4 text-muted-foreground">
        <span>{t('common:connecting')}</span>
      </div>
    </div>
  )
}

function ConnectionErrorScreen({
  onRetry,
  error,
}: {
  onRetry: () => void
  error?: unknown
}): React.JSX.Element {
  const { t } = useTranslation()
  const code = (error as Record<string, unknown> | null)?.code
  const message = EASY_VIRTUAL_DISPLAY_ERROR_CODES.includes(code as EasyVirtualDisplayErrorCode)
    ? t(`common:errors.${code}`)
    : error instanceof Error && error.message.length > 0
      ? error.message
      : t('common:connection_failed')

  return (
    <div className="min-h-screen bg-background font-sans flex items-center justify-center">
      <div className="flex flex-col items-center justify-center gap-4 text-muted-foreground">
        <p>{message}</p>
        <Button variant="default" onClick={onRetry}>
          {t('common:retry')}
        </Button>
      </div>
    </div>
  )
}

function App(): React.JSX.Element {
  const { data: snapshot, error, isLoading, isError, refetch } = useSnapshot()
  const { t } = useTranslation()
  const [activeTab, setActiveTab] = useState<'displays' | 'settings'>('displays')

  useLanguageBridge()
  useDocumentTheme(snapshot?.settings.theme)

  if (isLoading) return <LoadingScreen />
  if (isError || !snapshot) {
    return <ConnectionErrorScreen error={error} onRetry={() => refetch()} />
  }

  const handleThemeChange = (theme: AppTheme): void => {
    window.easyVirtualDisplay.updateSettings({ theme }).catch((error: unknown) => {
      const code = (error as Record<string, unknown>)?.code
      if (EASY_VIRTUAL_DISPLAY_ERROR_CODES.includes(code as EasyVirtualDisplayErrorCode)) {
        toast.error(t(`common:errors.${code}`))
      } else {
        toast.error(t('common:settings_update_failed'))
      }
    })
  }

  return (
    <div className="min-h-screen bg-background font-sans">
      <TitleBar
        activeTab={activeTab}
        onTabChange={setActiveTab}
        theme={snapshot.settings.theme}
        onThemeChange={handleThemeChange}
      />
      <main className="max-w-2xl mx-auto px-4 py-5 flex flex-col gap-5">
        <DriverStatusBar host={snapshot.host} />
        {activeTab === 'displays' && <DisplaysPanel snapshot={snapshot} />}
        {activeTab === 'settings' && <SettingsPanel snapshot={snapshot} />}
      </main>
    </div>
  )
}

export default App
