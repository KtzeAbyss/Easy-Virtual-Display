import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { waitFor } from '@testing-library/react'
import type { AppSnapshot } from '../../shared'
import { renderWithI18n } from './test-utils'
import { SettingsPanel } from '../components/SettingsPanel'

const makeSnapshot = (hostOverrides: Partial<AppSnapshot['host']> = {}): AppSnapshot => ({
  host: {
    revision: 1,
    status: 'ok',
    driverVersion: '4.22.0.0',
    maxDisplays: 4,
    displays: [],
    customModes: [],
    parentGpu: 'auto',
    ...hostOverrides
  },
  settings: {
    launchOnLogin: false,
    closeToTray: true,
    startMinimized: false,
    fallbackDisplay: false,
    keepScreenOn: false,
    theme: 'system',
    language: 'system'
  },
  effectiveLanguage: 'en'
})

describe('SettingsPanel (Advanced section)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders save button', () => {
    renderWithI18n(<SettingsPanel snapshot={makeSnapshot()} />)
    expect(screen.getByRole('button', { name: /save configuration/i })).toBeInTheDocument()
  })

  it('renders add mode button when fewer than 5 modes', () => {
    renderWithI18n(<SettingsPanel snapshot={makeSnapshot()} />)
    expect(screen.getByRole('button', { name: /add mode/i })).toBeInTheDocument()
  })

  it('hides add mode button when 5 modes exist', () => {
    const snapshot = makeSnapshot({
      customModes: Array.from({ length: 5 }, () => ({ width: 1920, height: 1080, hz: 60 }))
    })
    renderWithI18n(<SettingsPanel snapshot={snapshot} />)
    expect(screen.queryByRole('button', { name: /add mode/i })).not.toBeInTheDocument()
  })

  it('calls applyAdminConfig with empty modes on submit', async () => {
    const user = userEvent.setup()
    vi.mocked(window.easyVirtualDisplay.applyAdminConfig).mockResolvedValue(undefined)

    renderWithI18n(<SettingsPanel snapshot={makeSnapshot()} />)
    await user.click(screen.getByRole('button', { name: /save configuration/i }))

    await waitFor(() => {
      expect(window.easyVirtualDisplay.applyAdminConfig).toHaveBeenCalledWith({
        customModes: [],
        parentGpu: 'auto'
      })
    })
  })

  it('shows error when mode has invalid (zero) width', async () => {
    const user = userEvent.setup()

    renderWithI18n(<SettingsPanel snapshot={makeSnapshot()} />)
    await user.click(screen.getByRole('button', { name: /add mode/i }))

    const widthInput = screen.getByPlaceholderText('Width')
    await user.clear(widthInput)
    await user.type(widthInput, '0')

    await user.click(screen.getByRole('button', { name: /save configuration/i }))

    await waitFor(() => {
      expect(screen.getByText(/invalid values/i)).toBeInTheDocument()
    })
    expect(window.easyVirtualDisplay.applyAdminConfig).not.toHaveBeenCalled()
  })
})
