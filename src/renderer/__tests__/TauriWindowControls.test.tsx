import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'

interface WindowMock {
  minimize: ReturnType<typeof vi.fn>
  toggleMaximize: ReturnType<typeof vi.fn>
  close: ReturnType<typeof vi.fn>
  isMaximized: ReturnType<typeof vi.fn>
  onResized: ReturnType<typeof vi.fn>
  /** Trigger the registered onResized callback (so the test can simulate maximize/restore). */
  emitResize: () => Promise<void>
  /** Resolved UnlistenFn from `onResized()` — calling it tears the listener down. */
  unlisten: ReturnType<typeof vi.fn>
}

let win: WindowMock

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => win
}))

beforeEach(() => {
  let resizedHandler: (() => Promise<void> | void) | null = null
  win = {
    minimize: vi.fn().mockResolvedValue(undefined),
    toggleMaximize: vi.fn().mockResolvedValue(undefined),
    close: vi.fn().mockResolvedValue(undefined),
    isMaximized: vi.fn().mockResolvedValue(false),
    unlisten: vi.fn(),
    onResized: vi.fn((cb: () => Promise<void> | void) => {
      resizedHandler = cb
      return Promise.resolve(win.unlisten)
    }),
    emitResize: async () => {
      if (resizedHandler) await resizedHandler()
    }
  }
})

afterEach(() => {
  vi.clearAllMocks()
})

async function loadComponent(): Promise<typeof import('../components/TauriWindowControls')> {
  return await import('../components/TauriWindowControls')
}

describe('TauriWindowControls', () => {
  test('minimize button calls window.minimize()', async () => {
    const { TauriWindowControls } = await loadComponent()
    render(<TauriWindowControls />)
    fireEvent.click(screen.getByLabelText('minimize'))
    expect(win.minimize).toHaveBeenCalledTimes(1)
  })

  test('maximize button calls window.toggleMaximize()', async () => {
    const { TauriWindowControls } = await loadComponent()
    render(<TauriWindowControls />)
    fireEvent.click(screen.getByLabelText('maximize'))
    expect(win.toggleMaximize).toHaveBeenCalledTimes(1)
  })

  test('close button calls window.close() (Rust intercepts CloseRequested)', async () => {
    const { TauriWindowControls } = await loadComponent()
    render(<TauriWindowControls />)
    fireEvent.click(screen.getByLabelText('close'))
    expect(win.close).toHaveBeenCalledTimes(1)
  })

  test('initializes isMaximized state from window.isMaximized()', async () => {
    win.isMaximized.mockResolvedValueOnce(true)
    const { TauriWindowControls } = await loadComponent()
    render(<TauriWindowControls />)
    await waitFor(() => {
      expect(win.isMaximized).toHaveBeenCalled()
    })
  })

  test('updates maximize state when window emits resized', async () => {
    const { TauriWindowControls } = await loadComponent()
    render(<TauriWindowControls />)
    await waitFor(() => expect(win.onResized).toHaveBeenCalled())

    // Window goes maximized: emit resize event with isMaximized → true.
    win.isMaximized.mockResolvedValue(true)
    await act(async () => {
      await win.emitResize()
    })

    // The component re-renders; this assertion verifies that the listener pulled the
    // latest state from window.isMaximized().
    await waitFor(() => {
      expect(win.isMaximized).toHaveBeenCalledTimes(2)
    })
  })

  test('cleans up the resize listener on unmount', async () => {
    const { TauriWindowControls } = await loadComponent()
    const { unmount } = render(<TauriWindowControls />)
    await waitFor(() => expect(win.onResized).toHaveBeenCalled())
    unmount()
    expect(win.unlisten).toHaveBeenCalledTimes(1)
  })

  test('each control button renders its icon glyph', async () => {
    // Regression guard: ControlButton must render {children}. A self-closing <button/>
    // dropped the lucide icons, leaving clickable-but-glyphless (invisible) buttons.
    const { TauriWindowControls } = await loadComponent()
    render(<TauriWindowControls />)
    for (const label of ['minimize', 'maximize', 'close']) {
      expect(screen.getByLabelText(label).querySelector('svg')).not.toBeNull()
    }
  })

  test('controls wrapper opts out of the Tauri drag region', async () => {
    // Regression guard: without this attribute, Tauri's drag-region walk reaches the
    // title-bar row and preventDefault()s the mousedown, killing the button clicks.
    const { TauriWindowControls } = await loadComponent()
    const { container } = render(<TauriWindowControls />)
    const wrapper = container.querySelector('[data-tauri-drag-region="false"]')
    expect(wrapper).not.toBeNull()
    // The minimize / maximize / close buttons must be descendants of that opt-out wrapper.
    expect(wrapper?.querySelector('[aria-label="minimize"]')).not.toBeNull()
    expect(wrapper?.querySelector('[aria-label="maximize"]')).not.toBeNull()
    expect(wrapper?.querySelector('[aria-label="close"]')).not.toBeNull()
  })
})
