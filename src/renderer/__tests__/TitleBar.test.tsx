import { describe, expect, test, vi } from 'vitest'
import { renderWithI18n, screen } from './test-utils'
import { TitleBar } from '../components/TitleBar'

vi.mock('../components/TauriWindowControls', () => ({
  TauriWindowControls: () => <div data-testid="tauri-window-controls" />
}))

const props = {
  activeTab: 'displays' as const,
  onTabChange: vi.fn(),
  theme: 'system' as const,
  onThemeChange: vi.fn()
}

describe('TitleBar', () => {
  test('renders the custom Tauri window controls and a deep-mode drag row', () => {
    const { container } = renderWithI18n(<TitleBar {...props} />)
    expect(screen.getByTestId('tauri-window-controls')).toBeInTheDocument()
    // `deep` is the value that makes the logo / title / spacer draggable; a bare attribute
    // would only trigger when the mousedown lands on the row element itself.
    const dragRow = container.querySelector('[data-tauri-drag-region="deep"]') as HTMLElement | null
    expect(dragRow).not.toBeNull()
    expect(dragRow?.className).toContain('pr-0')
  })

  test('theme switcher buttons opt out of the Tauri drag region', () => {
    const { container } = renderWithI18n(<TitleBar {...props} />)
    const optOuts = container.querySelectorAll('[data-tauri-drag-region="false"]')
    // 3 theme buttons (system/light/dark).
    expect(optOuts.length).toBe(3)
  })
})
