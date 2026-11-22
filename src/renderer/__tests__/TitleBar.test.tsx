import { describe, expect, test, vi } from 'vitest'
import { renderWithI18n, screen } from './test-utils'
import { TitleBar } from '../components/TitleBar'

vi.mock('../components/TauriWindowControls', () => ({
  TauriWindowControls: () => <div data-testid="tauri-window-controls" />
}))

describe('TitleBar', () => {
  test('renders the app title, window controls and a deep-mode drag row', () => {
    const { container } = renderWithI18n(<TitleBar />)
    expect(screen.getByTestId('tauri-window-controls')).toBeInTheDocument()
    expect(screen.getByText('Easy Virtual Display')).toBeInTheDocument()
    // `deep` is the value that makes the icon / title / spacer draggable; a bare attribute
    // would only trigger when the mousedown lands on the row element itself.
    expect(container.querySelector('[data-tauri-drag-region="deep"]')).not.toBeNull()
  })

  test('holds no interactive controls besides the window buttons', () => {
    renderWithI18n(<TitleBar />)
    expect(screen.queryAllByRole('button')).toHaveLength(0)
  })
})
