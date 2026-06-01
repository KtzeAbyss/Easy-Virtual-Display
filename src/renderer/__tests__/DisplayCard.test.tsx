import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { waitFor } from '@testing-library/react'
import type { DisplaySummary } from '../../shared'
import { renderWithI18n } from './test-utils'
import { DisplayCard } from '../components/DisplayCard'

const mockDisplay: DisplaySummary = {
  index: 0,
  identifier: 256,
  deviceName: '\\\\.\\DISPLAY1',
  displayName: 'Virtual Display 1',
  active: true,
  currentMode: { width: 1920, height: 1080, hz: 60 },
  currentOrientation: 'landscape',
  supportedResolutions: [
    { width: 1920, height: 1080, refreshRates: [60, 120] },
    { width: 1280, height: 720, refreshRates: [60] }
  ],
  unsupportedCurrentMode: false
}

describe('DisplayCard', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders display name and current mode', () => {
    renderWithI18n(<DisplayCard display={mockDisplay} driverOk={true} />)
    expect(screen.getByText('Virtual Display 1')).toBeInTheDocument()
    expect(screen.getByText(/1920.*1080.*60/)).toBeInTheDocument()
  })

  it('shows unsupported mode badge when unsupportedCurrentMode is true', () => {
    renderWithI18n(
      <DisplayCard display={{ ...mockDisplay, unsupportedCurrentMode: true }} driverOk={true} />
    )
    expect(screen.getByText('unsupported mode')).toBeInTheDocument()
  })

  it('does not show unsupported mode badge normally', () => {
    renderWithI18n(<DisplayCard display={mockDisplay} driverOk={true} />)
    expect(screen.queryByText('unsupported mode')).not.toBeInTheDocument()
  })

  it('calls removeDisplay with correct index when Remove clicked', async () => {
    const user = userEvent.setup()
    vi.mocked(window.easyVirtualDisplay.removeDisplay).mockResolvedValue(undefined)

    renderWithI18n(<DisplayCard display={mockDisplay} driverOk={true} />)
    await user.click(screen.getByRole('button', { name: /remove/i }))

    await waitFor(() => {
      expect(window.easyVirtualDisplay.removeDisplay).toHaveBeenCalledWith(0)
    })
  })

  it('renders three select triggers for resolution, refresh rate, and orientation', () => {
    renderWithI18n(<DisplayCard display={mockDisplay} driverOk={true} />)
    const triggers = screen.getAllByRole('combobox')
    expect(triggers).toHaveLength(3)
  })

  it('disables controls when driver is not ok', () => {
    renderWithI18n(<DisplayCard display={mockDisplay} driverOk={false} />)
    const triggers = screen.getAllByRole('combobox')
    triggers.forEach((s) => expect(s).toBeDisabled())
  })
})
