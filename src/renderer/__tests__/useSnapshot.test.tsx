import { renderHook, act, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { AppSnapshot } from '../../shared'
import { SNAPSHOT_QUERY_KEY, useSnapshot } from '../hooks/useSnapshot'

const makeSnapshot = (overrides: Partial<AppSnapshot['host']> = {}, seq = 1): AppSnapshot => ({
  seq,
  host: {
    revision: 1,
    status: 'ok',
    driverVersion: '4.22.0.0',
    maxDisplays: 4,
    displays: [],
    activeOutputs: 0,
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
  effectiveLanguage: 'en',
  openViewers: [],
  fallbackError: null
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
    const snapshot2 = makeSnapshot({ revision: 2, status: 'restart_required' }, 2)

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

  it('keeps an event that lands before an older getSnapshot reply', async () => {
    const stale = makeSnapshot({ revision: 1 }, 1)
    const fresh = makeSnapshot({ revision: 2 }, 2)

    let listener: ((s: AppSnapshot) => void) | null = null
    vi.mocked(window.easyVirtualDisplay.subscribeSnapshot).mockImplementation((cb) => {
      listener = cb
      return vi.fn()
    })
    let resolveReply: (s: AppSnapshot) => void = () => {}
    vi.mocked(window.easyVirtualDisplay.getSnapshot).mockReturnValue(
      new Promise((resolve) => {
        resolveReply = resolve
      })
    )

    const { result } = renderHook(() => useSnapshot(), { wrapper: createWrapper() })
    await waitFor(() => expect(listener).not.toBeNull())

    act(() => {
      listener!(fresh)
    })
    await act(async () => {
      resolveReply(stale)
    })

    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(result.current.data?.host.revision).toBe(2)
  })

  it('ignores an event older than the snapshot it already has', async () => {
    let listener: ((s: AppSnapshot) => void) | null = null
    vi.mocked(window.easyVirtualDisplay.subscribeSnapshot).mockImplementation((cb) => {
      listener = cb
      return vi.fn()
    })
    vi.mocked(window.easyVirtualDisplay.getSnapshot).mockResolvedValue(
      makeSnapshot({ revision: 2 }, 2)
    )

    const { result } = renderHook(() => useSnapshot(), { wrapper: createWrapper() })
    await waitFor(() => expect(result.current.isSuccess).toBe(true))

    act(() => {
      listener!(makeSnapshot({ revision: 1 }, 1))
    })

    expect(result.current.data?.host.revision).toBe(2)
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
