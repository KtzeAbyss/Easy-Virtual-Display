import { useEffect, useState } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Minus, Square, Copy, X } from 'lucide-react'
import { cn } from '@/lib/utils'

/**
 * Custom minimize / maximize / close buttons rendered when the Tauri shell strips native
 * decorations. Sits on the right side of the title bar, following the Windows
 * window-control convention. Updates the maximize icon when the window enters/leaves
 * maximized state.
 */
export function TauriWindowControls(): React.JSX.Element {
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
    <div className="flex items-center gap-0" data-tauri-drag-region="false">
      <ControlButton aria-label="minimize" onClick={onMinimize}>
        <Minus className="h-3.5 w-3.5" />
      </ControlButton>
      <ControlButton aria-label="maximize" onClick={onToggleMaximize}>
        {isMaximized ? <Copy className="h-3 w-3" /> : <Square className="h-3 w-3" />}
      </ControlButton>
      <ControlButton aria-label="close" onClick={onClose} variant="close">
        <X className="h-4 w-4" />
      </ControlButton>
    </div>
  )
}

function ControlButton({
  children,
  variant,
  ...props
}: React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: 'close'
}): React.JSX.Element {
  return (
    <button
      {...props}
      type="button"
      className={cn(
        'flex h-9 w-11 items-center justify-center text-foreground transition-colors',
        variant === 'close'
          ? 'hover:bg-destructive hover:text-destructive-foreground'
          : 'hover:bg-muted'
      )}
    >
      {children}
    </button>
  )
}
