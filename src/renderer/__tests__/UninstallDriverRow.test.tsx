import { describe, expect, it, vi, beforeEach } from 'vitest'
import userEvent from '@testing-library/user-event'
import { toast } from 'sonner'
import { renderWithI18n, screen, waitFor } from './test-utils'
import { DangerZoneSection } from '../components/settings-panel/DangerZoneSection'
import { EMPTY_HOST_SNAPSHOT } from '../../shared'

vi.mock('sonner', () => ({ toast: { success: vi.fn(), error: vi.fn() } }))

describe('DangerZoneSection', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('disables the action when no driver is installed', () => {
    renderWithI18n(<DangerZoneSection host={{ ...EMPTY_HOST_SNAPSHOT, status: 'not_installed' }} />)

    expect(screen.getByRole('button', { name: /uninstall driver/i })).toBeDisabled()
    expect(screen.getByText(/no virtual display driver is installed/i)).toBeInTheDocument()
  })

  it('requires confirmation before uninstalling the driver', async () => {
    const user = userEvent.setup()
    vi.mocked(window.easyVirtualDisplay.uninstallDriver).mockResolvedValue(undefined)

    renderWithI18n(<DangerZoneSection host={{ ...EMPTY_HOST_SNAPSHOT, status: 'ok' }} />)

    await user.click(screen.getByRole('button', { name: /uninstall driver/i }))
    expect(window.easyVirtualDisplay.uninstallDriver).not.toHaveBeenCalled()

    await user.click(screen.getByRole('button', { name: /yes, uninstall/i }))

    await waitFor(() => {
      expect(window.easyVirtualDisplay.uninstallDriver).toHaveBeenCalledTimes(1)
    })
    expect(toast.success).toHaveBeenCalledWith('Virtual display driver uninstalled.')
  })

  it('localizes known uninstall errors', async () => {
    const user = userEvent.setup()
    vi.mocked(window.easyVirtualDisplay.uninstallDriver).mockRejectedValue({
      code: 'driver_uninstall_failed'
    })

    renderWithI18n(<DangerZoneSection host={{ ...EMPTY_HOST_SNAPSHOT, status: 'ok' }} />)

    await user.click(screen.getByRole('button', { name: /uninstall driver/i }))
    await user.click(screen.getByRole('button', { name: /yes, uninstall/i }))

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('Failed to uninstall the virtual display driver')
    })
  })
})
