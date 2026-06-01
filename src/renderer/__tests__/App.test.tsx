import { describe, expect, it, vi } from 'vitest'
import { renderWithI18n, screen } from './test-utils'
import App from '../App'

const { mockUseSnapshot } = vi.hoisted(() => ({
  mockUseSnapshot: vi.fn()
}))

vi.mock('../hooks/useSnapshot', () => ({
  useSnapshot: () => mockUseSnapshot()
}))

vi.mock('../hooks/useLanguageBridge', () => ({
  useLanguageBridge: vi.fn()
}))

vi.mock('../hooks/useDocumentTheme', () => ({
  useDocumentTheme: vi.fn()
}))

vi.mock('../components/TitleBar', () => ({
  TitleBar: () => <div data-testid="title-bar" />
}))

vi.mock('../components/DriverStatusBar', () => ({
  DriverStatusBar: () => <div data-testid="driver-status-bar" />
}))

vi.mock('../components/DisplaysPanel', () => ({
  DisplaysPanel: () => <div data-testid="displays-panel" />
}))

vi.mock('../components/SettingsPanel', () => ({
  SettingsPanel: () => <div data-testid="settings-panel" />
}))

describe('App', () => {
  it('shows the localized runtime-missing message on startup failure', () => {
    mockUseSnapshot.mockReturnValue({
      data: undefined,
      error: { code: 'dotnet_runtime_missing' },
      isLoading: false,
      isError: true,
      refetch: vi.fn()
    })

    renderWithI18n(<App />)

    expect(
      screen.getByText('Microsoft .NET 8 Runtime (x64) is required to start the bundled native host')
    ).toBeInTheDocument()
  })
})
