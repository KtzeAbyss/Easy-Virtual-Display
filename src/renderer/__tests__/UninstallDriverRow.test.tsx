import { describe, expect, it, vi, beforeEach } from 'vitest'
import userEvent from '@testing-library/user-event'
import { toast } from 'sonner'
import { renderWithI18n, screen, waitFor } from './test-utils'
import { UninstallDriverRow } from '../components/driver/UninstallDriverRow'
import { EMPTY_HOST_SNAPSHOT } from '../../shared'

vi.mock('sonner', () => ({ toast: { success: vi.fn(), error: vi.fn() } }))

describe('UninstallDriverRow', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('disables the action when no driver is installed', () => {
    renderWithI18n(<UninstallDriverRow host={{ ...EMPTY_HOST_SNAPSHOT, status: 'not_installed' }} />)

    expect(screen.getByRole('button', { name: /^uninstall$/i })).toBeDisabled()
    expect(screen.getByText(/driver is not installed/i)).toBeInTheDocument()
  })

  it('requires confirmation in a dialog before uninstalling', async () => {
    const user = userEvent.setup()
    vi.mocked(window.easyVirtualDisplay.uninstallDriver).mockResolvedValue(undefined)

    renderWithI18n(<UninstallDriverRow host={{ ...EMPTY_HOST_SNAPSHOT, status: 'ok' }} />)

    await user.click(screen.getByRole('button', { name: /^uninstall$/i }))
    expect(window.easyVirtualDisplay.uninstallDriver).not.toHaveBeenCalled()

    const dialog = await screen.findByRole('dialog')
    expect(dialog).toHaveTextContent('Uninstall the virtual display driver?')
    await user.click(screen.getAllByRole('button', { name: /^uninstall$/i }).at(-1)!)

    await waitFor(() => {
      expect(window.easyVirtualDisplay.uninstallDriver).toHaveBeenCalledTimes(1)
    })
    expect(toast.success).toHaveBeenCalledWith('Virtual display driver uninstalled')
  })

  it('cancelling the dialog leaves the driver alone', async () => {
    const user = userEvent.setup()
    renderWithI18n(<UninstallDriverRow host={{ ...EMPTY_HOST_SNAPSHOT, status: 'ok' }} />)

    await user.click(screen.getByRole('button', { name: /^uninstall$/i }))
    await user.click(await screen.findByRole('button', { name: /cancel/i }))

    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
    expect(window.easyVirtualDisplay.uninstallDriver).not.toHaveBeenCalled()
  })

  it('localizes known uninstall errors', async () => {
    const user = userEvent.setup()
    vi.mocked(window.easyVirtualDisplay.uninstallDriver).mockRejectedValue({
      code: 'driver_uninstall_failed'
    })

    renderWithI18n(<UninstallDriverRow host={{ ...EMPTY_HOST_SNAPSHOT, status: 'ok' }} />)

    await user.click(screen.getByRole('button', { name: /^uninstall$/i }))
    await screen.findByRole('dialog')
    await user.click(screen.getAllByRole('button', { name: /^uninstall$/i }).at(-1)!)

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('Could not uninstall the virtual display driver')
    })
  })
})
