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

  it('renders the display name, state and current mode once', () => {
    renderWithI18n(<DisplayCard display={mockDisplay} driverOk={true} viewerOpen={false} />)
    expect(screen.getByText('Virtual Display 1')).toBeInTheDocument()
    expect(screen.getByText('Active')).toBeInTheDocument()
    expect(screen.getByText(/1920 × 1080 · 60 Hz · Landscape/)).toBeInTheDocument()
    // The raw device path lives in a tooltip, not in the card body.
    expect(screen.queryByText('\\\\.\\DISPLAY1')).not.toBeInTheDocument()
  })

  it('shows unsupported mode badge when unsupportedCurrentMode is true', () => {
    renderWithI18n(
      <DisplayCard
        display={{ ...mockDisplay, unsupportedCurrentMode: true }}
        driverOk={true}
        viewerOpen={false}
      />
    )
    expect(screen.getByText('Unsupported mode')).toBeInTheDocument()
  })

  it('does not show unsupported mode badge normally', () => {
    renderWithI18n(<DisplayCard display={mockDisplay} driverOk={true} viewerOpen={false} />)
    expect(screen.queryByText('Unsupported mode')).not.toBeInTheDocument()
  })

  it('removes the display from the overflow menu', async () => {
    const user = userEvent.setup()
    vi.mocked(window.easyVirtualDisplay.removeDisplay).mockResolvedValue(undefined)

    renderWithI18n(<DisplayCard display={mockDisplay} driverOk={true} viewerOpen={false} />)
    await user.click(screen.getByRole('button', { name: /more actions/i }))
    await user.click(await screen.findByRole('menuitem', { name: /^remove$/i }))

    await waitFor(() => {
      expect(window.easyVirtualDisplay.removeDisplay).toHaveBeenCalledWith(0)
    })
  })

  it('opens system display settings from the overflow menu', async () => {
    const user = userEvent.setup()
    renderWithI18n(<DisplayCard display={mockDisplay} driverOk={true} viewerOpen={false} />)
    await user.click(screen.getByRole('button', { name: /more actions/i }))
    await user.click(await screen.findByRole('menuitem', { name: /system display settings/i }))
    expect(window.easyVirtualDisplay.openDisplaySettings).toHaveBeenCalledTimes(1)
  })

  it('renders labelled selects for resolution, refresh rate, and orientation', () => {
    renderWithI18n(<DisplayCard display={mockDisplay} driverOk={true} viewerOpen={false} />)
    expect(screen.getAllByRole('combobox')).toHaveLength(3)
    expect(screen.getByLabelText('Resolution')).toBeInTheDocument()
    expect(screen.getByLabelText('Refresh rate')).toBeInTheDocument()
    expect(screen.getByLabelText('Orientation')).toBeInTheDocument()
  })

  it('opens the mirror window for this display when Preview is clicked', async () => {
    const user = userEvent.setup()

    renderWithI18n(<DisplayCard display={mockDisplay} driverOk={true} viewerOpen={false} />)
    await user.click(screen.getByRole('button', { name: /^preview$/i }))

    await waitFor(() => {
      expect(window.easyVirtualDisplay.openDisplayViewer).toHaveBeenCalledWith(0)
    })
  })

  it('closes the mirror window when one is already open', async () => {
    const user = userEvent.setup()

    renderWithI18n(<DisplayCard display={mockDisplay} driverOk={true} viewerOpen={true} />)
    await user.click(screen.getByRole('button', { name: /stop preview/i }))

    await waitFor(() => {
      expect(window.easyVirtualDisplay.closeDisplayViewer).toHaveBeenCalledWith(0)
    })
  })

  it('can still close an open viewer while the driver is not ok', async () => {
    const user = userEvent.setup()

    renderWithI18n(<DisplayCard display={mockDisplay} driverOk={false} viewerOpen={true} />)
    const close = screen.getByRole('button', { name: /stop preview/i })
    expect(close).toBeEnabled()
    await user.click(close)

    await waitFor(() => {
      expect(window.easyVirtualDisplay.closeDisplayViewer).toHaveBeenCalledWith(0)
    })
  })

  it('cannot open a viewer for an inactive display', () => {
    renderWithI18n(
      <DisplayCard display={{ ...mockDisplay, active: false }} driverOk={true} viewerOpen={false} />
    )
    expect(screen.getByText('Inactive')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /^preview$/i })).toBeDisabled()
  })

  it('disables controls when driver is not ok', () => {
    renderWithI18n(<DisplayCard display={mockDisplay} driverOk={false} viewerOpen={false} />)
    const triggers = screen.getAllByRole('combobox')
    triggers.forEach((s) => expect(s).toBeDisabled())
  })
})
