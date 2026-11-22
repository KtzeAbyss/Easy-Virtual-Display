import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { CircleX } from 'lucide-react'
import { useSnapshot } from './hooks/useSnapshot'
import { useLanguageBridge } from './hooks/useLanguageBridge'
import { useDocumentTheme } from './hooks/useDocumentTheme'
import { useBridgeCommand } from './hooks/useBridgeCommand'
import type { AppSnapshot } from '../shared'
import { parseBridgeError } from '../shared/errors'
import { TitleBar } from './components/TitleBar'
import { NavigationRail, type AppPage } from './components/NavigationRail'
import { DisplaysPage, canAddDisplay, type AddDisplayMutation } from './components/DisplaysPage'
import { DriverPage } from './components/DriverPage'
import { SettingsPage } from './components/SettingsPage'
import { AboutPage } from './components/AboutPage'
import { Button } from './components/ui/button'

function LoadingSkeleton(): React.JSX.Element {
  const { t } = useTranslation()
  return (
    <div role="status" aria-label={t('common:connecting')} className="flex flex-col gap-2">
      <div className="mb-3 h-9 w-40 animate-pulse rounded-md bg-muted" />
      {[0, 1].map((i) => (
        <div key={i} className="h-[124px] animate-pulse rounded-lg border bg-card" />
      ))}
      <span className="pt-2 text-xs text-muted-foreground">{t('common:connecting')}</span>
    </div>
  )
}

function StartupError({ error, onRetry }: { error: unknown; onRetry: () => void }): React.JSX.Element {
  const { t } = useTranslation()
  const code = parseBridgeError(error)
  const message = code
    ? t(`common:errors.${code}`)
    : error instanceof Error && error.message.length > 0
      ? error.message
      : t('common:connection_failed')

  return (
    <div role="alert" className="flex flex-col items-center gap-3 px-6 py-20 text-center">
      <CircleX className="size-10 text-status-error" strokeWidth={1.5} aria-hidden />
      <p className="font-display text-xl font-semibold">{t('common:startup_failed_title')}</p>
      <p className="selectable max-w-md text-sm text-muted-foreground">{message}</p>
      <Button className="mt-3 min-w-28" onClick={onRetry}>
        {t('common:retry')}
      </Button>
    </div>
  )
}

/** Ctrl+N adds a display, Ctrl+, opens settings. Also stops WebView2 opening a browser window on Ctrl+N. */
function useAppShortcuts(
  snapshot: AppSnapshot | undefined,
  addDisplay: AddDisplayMutation,
  navigate: (page: AppPage) => void
): void {
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent): void => {
      if (!e.ctrlKey || e.altKey || e.shiftKey || e.metaKey) return
      if (e.key === ',') {
        e.preventDefault()
        navigate('settings')
      } else if (e.key.toLowerCase() === 'n') {
        e.preventDefault()
        if (snapshot && canAddDisplay(snapshot) && !addDisplay.isPending) {
          navigate('displays')
          addDisplay.mutate()
        }
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [snapshot, addDisplay, navigate])
}

function App(): React.JSX.Element {
  const { data: snapshot, error, isLoading, isError, refetch } = useSnapshot()
  const [page, setPage] = useState<AppPage>('displays')
  const addDisplay = useBridgeCommand<void>(() => window.easyVirtualDisplay.addDisplay())

  useLanguageBridge()
  useDocumentTheme(snapshot?.settings.theme ?? 'system')
  useAppShortcuts(snapshot, addDisplay, setPage)

  let content: React.ReactNode
  if (isLoading) {
    content = <LoadingSkeleton />
  } else if (isError || !snapshot) {
    content = <StartupError error={error} onRetry={() => void refetch()} />
  } else if (page === 'displays') {
    content = (
      <DisplaysPage
        snapshot={snapshot}
        addDisplay={addDisplay}
        onOpenDriverPage={() => setPage('driver')}
      />
    )
  } else if (page === 'driver') {
    content = <DriverPage host={snapshot.host} />
  } else if (page === 'settings') {
    content = <SettingsPage snapshot={snapshot} />
  } else {
    content = <AboutPage />
  }

  return (
    <div className="flex h-full flex-col">
      <TitleBar />
      <div className="flex min-h-0 flex-1">
        <NavigationRail page={page} onNavigate={setPage} driverStatus={snapshot?.host.status} />
        <main
          key={page}
          className="min-h-0 flex-1 overflow-y-auto rounded-tl-lg border-t border-l bg-layer"
        >
          <div className="mx-auto w-full max-w-[880px] animate-in px-8 pt-6 pb-10 duration-200 fade-in-0 slide-in-from-bottom-1">
            {content}
          </div>
        </main>
      </div>
    </div>
  )
}

export default App
