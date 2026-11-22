import { describe, expect, it, vi, beforeEach } from 'vitest'
import userEvent from '@testing-library/user-event'
import { renderWithI18n, screen, waitFor } from './test-utils'
import type { AppSnapshot } from '../../shared'
import { DEFAULT_APP_SETTINGS } from '../../shared'
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

vi.mock('../components/DisplaysPage', () => ({
  DisplaysPage: () => <div data-testid="displays-page" />,
  canAddDisplay: (s: AppSnapshot) =>
    s.host.status === 'ok' && s.host.displays.length < s.host.maxDisplays
}))

vi.mock('../components/DriverPage', () => ({
  DriverPage: () => <div data-testid="driver-page" />
}))

vi.mock('../components/SettingsPage', () => ({
  SettingsPage: () => <div data-testid="settings-page" />
}))

vi.mock('../components/AboutPage', () => ({
  AboutPage: () => <div data-testid="about-page" />
}))

const snapshot: AppSnapshot = {
  seq: 1,
  host: {
    revision: 1,
    status: 'ok',
    driverVersion: '4.22.0.0',
    maxDisplays: 4,
    displays: [],
    activeOutputs: 0,
    customModes: [],
    parentGpu: 'auto'
  },
  settings: DEFAULT_APP_SETTINGS,
  effectiveLanguage: 'en',
  openViewers: [],
  fallbackError: null
}

function loaded(): void {
  mockUseSnapshot.mockReturnValue({
    data: snapshot,
    error: null,
    isLoading: false,
    isError: false,
    refetch: vi.fn()
  })
}

describe('App', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

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
      screen.getByText('Microsoft .NET 8 Runtime (x64) is required to start the background service')
    ).toBeInTheDocument()
    // The frame stays usable so the window can still be moved and closed.
    expect(screen.getByTestId('title-bar')).toBeInTheDocument()
  })

  it('navigates between pages from the navigation pane', async () => {
    const user = userEvent.setup()
    loaded()
    renderWithI18n(<App />)

    expect(screen.getByTestId('displays-page')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Displays' })).toHaveAttribute('aria-current', 'page')

    await user.click(screen.getByRole('button', { name: 'Driver' }))
    expect(screen.getByTestId('driver-page')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'About' }))
    expect(screen.getByTestId('about-page')).toBeInTheDocument()
  })

  it('Ctrl+, opens settings and Ctrl+N adds a display', async () => {
    const user = userEvent.setup()
    loaded()
    renderWithI18n(<App />)

    await user.keyboard('{Control>},{/Control}')
    expect(screen.getByTestId('settings-page')).toBeInTheDocument()

    await user.keyboard('{Control>}n{/Control}')
    await waitFor(() => expect(window.easyVirtualDisplay.addDisplay).toHaveBeenCalledTimes(1))
    expect(screen.getByTestId('displays-page')).toBeInTheDocument()
  })
})
