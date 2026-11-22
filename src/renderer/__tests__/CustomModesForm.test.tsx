import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { waitFor } from '@testing-library/react'
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

async function addMode(user: ReturnType<typeof userEvent.setup>, item: RegExp): Promise<void> {
  await user.click(screen.getByRole('button', { name: /add mode/i }))
  await user.click(await screen.findByRole('menuitem', { name: item }))
}

describe('CustomModesForm (driver page)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('keeps Apply disabled until something changes', () => {
    renderWithI18n(<DriverPage host={makeHost()} />)
    expect(screen.getByRole('button', { name: /apply/i })).toBeDisabled()
    expect(screen.queryByRole('button', { name: /discard/i })).not.toBeInTheDocument()
  })

  it('hides the add mode menu when 5 modes exist', () => {
    const host = makeHost({
      customModes: Array.from({ length: 5 }, () => ({ width: 1920, height: 1080, hz: 60 }))
    })
    renderWithI18n(<DriverPage host={host} />)
    expect(screen.queryByRole('button', { name: /add mode/i })).not.toBeInTheDocument()
  })

  it('applies a preset mode', async () => {
    const user = userEvent.setup()
    vi.mocked(window.easyVirtualDisplay.applyAdminConfig).mockResolvedValue(undefined)

    renderWithI18n(<DriverPage host={makeHost()} />)
    await addMode(user, /2560 × 1440 @ 60/)
    await user.click(screen.getByRole('button', { name: /apply/i }))

    await waitFor(() => {
      expect(window.easyVirtualDisplay.applyAdminConfig).toHaveBeenCalledWith({
        customModes: [{ width: 2560, height: 1440, hz: 60 }],
        parentGpu: 'auto'
      })
    })
  })

  it('discard restores the applied configuration', async () => {
    const user = userEvent.setup()
    renderWithI18n(<DriverPage host={makeHost()} />)
    await addMode(user, /custom/i)
    expect(screen.getByPlaceholderText('Width')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: /discard/i }))
    expect(screen.queryByPlaceholderText('Width')).not.toBeInTheDocument()
  })

  it('shows an inline error when a mode has an invalid (zero) width', async () => {
    const user = userEvent.setup()

    renderWithI18n(<DriverPage host={makeHost()} />)
    await addMode(user, /custom/i)

    const widthInput = screen.getByPlaceholderText('Width')
    await user.clear(widthInput)
    await user.type(widthInput, '0')

    await user.click(screen.getByRole('button', { name: /apply/i }))

    await waitFor(() => {
      expect(screen.getByText(/positive whole numbers/i)).toBeInTheDocument()
    })
    expect(window.easyVirtualDisplay.applyAdminConfig).not.toHaveBeenCalled()
  })
})
