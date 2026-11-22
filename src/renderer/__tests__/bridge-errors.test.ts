import { describe, expect, test, vi } from 'vitest'
import { parseBridgeError } from '../../shared/errors'
import { toastBridgeError } from '../lib/bridge-errors'

vi.mock('sonner', () => ({
  toast: {
    error: vi.fn()
  }
}))

describe('parseBridgeError', () => {
  test('returns known codes', () => {
    expect(parseBridgeError({ code: 'native_host_unavailable' })).toBe('native_host_unavailable')
    expect(parseBridgeError({ code: 'invalid_request' })).toBe('invalid_request')
    expect(parseBridgeError({ code: 'needs_elevation' })).toBe('needs_elevation')
  })

  test('rejects unknown shapes', () => {
    expect(parseBridgeError(null)).toBeUndefined()
    expect(parseBridgeError({ code: 'not_a_real_code' })).toBeUndefined()
    expect(parseBridgeError(new Error('boom'))).toBeUndefined()
  })
})

describe('toastBridgeError', () => {
  test('uses common:errors for known codes', async () => {
    const { toast } = await import('sonner')
    const t = vi.fn((key: string) => key)
    toastBridgeError(t, { code: 'driver_not_installed' })
    expect(t).toHaveBeenCalledWith('common:errors.driver_not_installed')
    expect(toast.error).toHaveBeenCalledWith('common:errors.driver_not_installed')
  })

  test('uses fallback key for unknown errors', async () => {
    const { toast } = await import('sonner')
    const t = vi.fn((key: string) => key)
    toastBridgeError(t, new Error('nope'), 'common:settings_update_failed')
    expect(toast.error).toHaveBeenCalledWith('common:settings_update_failed')
  })
})
