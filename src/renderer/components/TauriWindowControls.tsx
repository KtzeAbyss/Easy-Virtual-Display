import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { cn } from '@/lib/utils'

/** Segoe Fluent Icons caption glyphs (same code points in Segoe MDL2 Assets on Windows 10). */
const GLYPH = {
  minimize: '\uE921',
  maximize: '\uE922',
  restore: '\uE923',
  close: '\uE8BB'
} as const

/**
 * Caption buttons for the undecorated main window, sized and colored to the Windows 11
 * spec (46×32, red close hover). The maximize glyph tracks the window's maximized state.
 */
export function TauriWindowControls(): React.JSX.Element {
  const { t } = useTranslation()
  const [isMaximized, setIsMaximized] = useState(false)

  useEffect(() => {
    const win = getCurrentWindow()
    let cancelled = false
    let unlisten: (() => void) | undefined

    void win.isMaximized().then((v) => {
      if (!cancelled) setIsMaximized(v)
    })

    void win
      .onResized(async () => {
        const next = await win.isMaximized()
        if (!cancelled) setIsMaximized(next)
      })
      .then((fn) => {
        if (cancelled) {
          fn()
        } else {
          unlisten = fn
        }
      })

    return () => {
      cancelled = true
      unlisten?.()
    }
  }, [])

  const onMinimize = (): void => {
    void getCurrentWindow().minimize()
  }
  const onToggleMaximize = (): void => {
    void getCurrentWindow().toggleMaximize()
  }
  const onClose = (): void => {
    // `close()` fires the `CloseRequested` window event — the Rust side intercepts and
    // either hides the window (close-to-tray) or runs the quit confirmation flow.
    void getCurrentWindow().close()
  }

  // Tauri's drag-region walks up the DOM from the mousedown target; once it sees any
  // ancestor with `data-tauri-drag-region` (and the title-bar row has it), it preventDefaults
  // the mousedown to start a drag — which kills our buttons' click handlers. Opting the
  // entire controls cluster out short-circuits that walk before it reaches the title-bar row.
  return (
    <div className="flex h-full items-stretch" data-tauri-drag-region="false">
      <ControlButton label={t('common:window.minimize')} glyph={GLYPH.minimize} onClick={onMinimize} />
      <ControlButton
        label={t(isMaximized ? 'common:window.restore' : 'common:window.maximize')}
        glyph={isMaximized ? GLYPH.restore : GLYPH.maximize}
        onClick={onToggleMaximize}
      />
      <ControlButton label={t('common:window.close')} glyph={GLYPH.close} onClick={onClose} close />
    </div>
  )
}

function ControlButton({
  label,
  glyph,
  onClick,
  close
}: {
  label: string
  glyph: string
  onClick: () => void
  close?: boolean
}): React.JSX.Element {
  return (
    <button
      type="button"
      tabIndex={-1}
      aria-label={label}
      title={label}
      onClick={onClick}
      className={cn(
        'flex h-8 w-[46px] items-center justify-center text-foreground transition-colors duration-100',
        close
          ? 'hover:bg-caption-close hover:text-white active:bg-caption-close/90 active:text-white/80'
          : 'hover:bg-nav-hover active:bg-nav-selected active:text-muted-foreground'
      )}
    >
      <span aria-hidden className="font-icons text-[10px] leading-none">
        {glyph}
      </span>
    </button>
  )
}
