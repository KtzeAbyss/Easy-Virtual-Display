import { renderHook, act, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { AppSnapshot } from '../../shared'
import { SNAPSHOT_QUERY_KEY, useSnapshot } from '../hooks/useSnapshot'

const makeSnapshot = (overrides: Partial<AppSnapshot['host']> = {}): AppSnapshot => ({
  host: {
    revision: 1,
    status: 'ok',
    driverVersion: '4.22.0.0',
    maxDisplays: 4,
    displays: [],
    customModes: [],
    parentGpu: 'auto',
    ...overrides
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

const createWrapper = (): React.FC<{ children: React.ReactNode }> => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } }
  })
  const Wrapper = ({ children }: { children: React.ReactNode }): React.JSX.Element => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  )
  return Wrapper
}

describe('useSnapshot', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('loads initial snapshot via getSnapshot', async () => {
    const snapshot = makeSnapshot()
    vi.mocked(window.easyVirtualDisplay.getSnapshot).mockResolvedValue(snapshot)

    const { result } = renderHook(() => useSnapshot(), { wrapper: createWrapper() })

    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(result.current.data).toEqual(snapshot)
    expect(window.easyVirtualDisplay.getSnapshot).toHaveBeenCalledOnce()
  })

  it('updates data when subscribeSnapshot fires', async () => {
    const snapshot1 = makeSnapshot({ revision: 1 })
    const snapshot2 = makeSnapshot({ revision: 2, status: 'restart_required' })

    let listener: ((s: AppSnapshot) => void) | null = null
    vi.mocked(window.easyVirtualDisplay.subscribeSnapshot).mockImplementation((cb) => {
      listener = cb
      return vi.fn()
    })
    vi.mocked(window.easyVirtualDisplay.getSnapshot).mockResolvedValue(snapshot1)

    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    const Wrapper = ({ children }: { children: React.ReactNode }): React.JSX.Element => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    )
    const { result } = renderHook(() => useSnapshot(), { wrapper: Wrapper })

    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(result.current.data?.host.revision).toBe(1)

    act(() => {
      listener!(snapshot2)
    })

    await waitFor(() => expect(result.current.data?.host.revision).toBe(2))
    expect(result.current.data?.host.status).toBe('restart_required')
  })

  it('unsubscribes on unmount', async () => {
    const snapshot = makeSnapshot()
    const unsubscribe = vi.fn()
    vi.mocked(window.easyVirtualDisplay.subscribeSnapshot).mockReturnValue(unsubscribe)
    vi.mocked(window.easyVirtualDisplay.getSnapshot).mockResolvedValue(snapshot)

    const { unmount } = renderHook(() => useSnapshot(), { wrapper: createWrapper() })
    await waitFor(() => {
      expect(window.easyVirtualDisplay.subscribeSnapshot).toHaveBeenCalled()
    })

    unmount()
    expect(unsubscribe).toHaveBeenCalled()
  })

  it('commands return void (not a new snapshot)', async () => {
    const result = await window.easyVirtualDisplay.addDisplay()
    expect(result).toBeUndefined()
  })

  it('snapshot query key is stable', () => {
    expect(SNAPSHOT_QUERY_KEY).toEqual(['snapshot'])
  })
})
