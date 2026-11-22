import { screen } from '@testing-library/react'
import type { AppSnapshot } from '../../shared'
import { renderWithI18n } from './test-utils'
import { DriverPage } from '../components/DriverPage'

const makeHost = (overrides: Partial<AppSnapshot['host']> = {}): AppSnapshot['host'] => ({
  revision: 1,
  status: 'ok',
  driverVersion: '4.22.0.0',
  maxDisplays: 4,
  displays: [],
  activeOutputs: 0,
  customModes: [],
  parentGpu: 'auto',
  ...overrides
})

describe('DriverPage', () => {
  it('shows the ok status without an info bar', () => {
    renderWithI18n(<DriverPage host={makeHost({ status: 'ok' })} />)
    expect(screen.getByText('Driver OK')).toBeInTheDocument()
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
  })

  it('explains a pending restart in a warning info bar', () => {
    renderWithI18n(<DriverPage host={makeHost({ status: 'restart_required' })} />)
    expect(screen.getByText('Restart Required')).toBeInTheDocument()
    expect(screen.getByRole('alert')).toHaveTextContent(/restart your PC/i)
  })

  it('offers to install the driver when it is missing', () => {
    renderWithI18n(<DriverPage host={makeHost({ status: 'not_installed' })} />)
    expect(screen.getByText('Not Installed')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /install driver/i })).toBeInTheDocument()
    // Driver-level configuration is meaningless without a driver.
    expect(screen.queryByText('Custom resolutions')).not.toBeInTheDocument()
  })

  it('shows real driver version and display limit', () => {
    renderWithI18n(<DriverPage host={makeHost({ driverVersion: '4.22.0.0', maxDisplays: 8 })} />)
    expect(screen.getByText('4.22.0.0')).toBeInTheDocument()
    expect(screen.getByText('8')).toBeInTheDocument()
  })

  it('shows the active display count', () => {
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
    renderWithI18n(<DriverPage host={host} />)
    expect(screen.getByText('Active displays').nextSibling).toHaveTextContent('1')
  })
})
