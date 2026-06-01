import { screen } from '@testing-library/react'
import type { AppSnapshot } from '../../shared'
import { renderWithI18n } from './test-utils'
import { DriverStatusBar } from '../components/DriverStatusBar'

const makeHost = (overrides: Partial<AppSnapshot['host']> = {}): AppSnapshot['host'] => ({
  revision: 1,
  status: 'ok',
  driverVersion: '4.22.0.0',
  maxDisplays: 4,
  displays: [],
  customModes: [],
  parentGpu: 'auto',
  ...overrides
})

describe('DriverStatusBar', () => {
  it('renders is-ok status when driver is ok', () => {
    renderWithI18n(<DriverStatusBar host={makeHost({ status: 'ok' })} />)
    expect(screen.getByText(/Driver OK/)).toBeInTheDocument()
  })

  it('renders warning status for restart_required', () => {
    renderWithI18n(<DriverStatusBar host={makeHost({ status: 'restart_required' })} />)
    expect(screen.getByText(/Restart Required/)).toBeInTheDocument()
  })

  it('renders error status for non-ok non-warning status', () => {
    renderWithI18n(<DriverStatusBar host={makeHost({ status: 'not_installed' })} />)
    expect(screen.getByText(/Not Installed/)).toBeInTheDocument()
  })

  it('shows warning banner when driver is not ok', () => {
    renderWithI18n(<DriverStatusBar host={makeHost({ status: 'not_installed' })} />)
    expect(screen.getByText(/not ready/i)).toBeInTheDocument()
  })

  it('does not show warning banner when driver is ok', () => {
    renderWithI18n(<DriverStatusBar host={makeHost({ status: 'ok' })} />)
    expect(screen.queryByText(/not ready/i)).not.toBeInTheDocument()
  })

  it('shows real driver version and max displays', () => {
    renderWithI18n(
      <DriverStatusBar host={makeHost({ driverVersion: '4.22.0.0', maxDisplays: 8 })} />
    )
    expect(screen.getByText('4.22.0.0')).toBeInTheDocument()
    expect(screen.getByText('8')).toBeInTheDocument()
  })

  it('shows active display count', () => {
    const host = makeHost({
      displays: [
        {
          index: 0,
          identifier: 256,
          deviceName: '\\\\.\\DISPLAY1',
          displayName: 'Virtual Display 1',
          active: true,
          currentMode: null,
          currentOrientation: 'landscape',
          supportedResolutions: [],
          unsupportedCurrentMode: false
        }
      ]
    })
    renderWithI18n(<DriverStatusBar host={host} />)
    expect(screen.getByText('1')).toBeInTheDocument()
  })
})
