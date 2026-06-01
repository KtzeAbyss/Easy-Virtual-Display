import { beforeEach, describe, expect, test, vi } from 'vitest'

type Listener = (event: { payload: unknown }) => void

interface FakeTauriRuntime {
  invokeCalls: Array<{ cmd: string; args: unknown }>
  listenCalls: Array<{ event: string; listener: Listener }>
  /** Resolve all queued `listen()` calls (simulating Tauri's async setup completing). */
  flushListenPromises(): Promise<void>
  /** Emit a fake event into every listener currently registered for `event`. */
  emit(event: string, payload: unknown): void
  /** Resolved `UnlistenFn`s the bridge has stored; calling them removes the listener. */
  activeUnlisteners: Map<Listener, () => void>
}

let runtime: FakeTauriRuntime
let pendingListens: Array<() => void> = []

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn((cmd: string, args?: unknown) => {
    runtime.invokeCalls.push({ cmd, args })
    return Promise.resolve(undefined)
  })
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn((event: string, listener: Listener) => {
    runtime.listenCalls.push({ event, listener })
    return new Promise<() => void>((resolve) => {
      pendingListens.push(() => {
        const unlisten = (): void => {
          runtime.activeUnlisteners.delete(listener)
        }
        runtime.activeUnlisteners.set(listener, unlisten)
        resolve(unlisten)
      })
    })
  })
}))

async function loadBridge(): Promise<typeof import('../bridge/tauri-bridge')> {
  return await import('../bridge/tauri-bridge')
}

beforeEach(() => {
  runtime = {
    invokeCalls: [],
    listenCalls: [],
    activeUnlisteners: new Map(),
    flushListenPromises: async () => {
      const pending = pendingListens.splice(0)
      for (const resolve of pending) {
        resolve()
      }
      // Yield to microtask queue so .then() callbacks run.
      await Promise.resolve()
    },
    emit: (event, payload) => {
      for (const call of runtime.listenCalls) {
        if (call.event === event) {
          call.listener({ payload })
        }
      }
    }
  }
  pendingListens = []
  vi.clearAllMocks()
})

describe('createTauriBridge → 11-method Seam A surface', () => {
  test('each method maps to the expected snake_case Tauri command', async () => {
    const { createTauriBridge } = await loadBridge()
    const bridge = createTauriBridge()

    await bridge.getSnapshot()
    await bridge.installDriver()
    await bridge.uninstallDriver()
    await bridge.addDisplay()
    await bridge.removeDisplay(2)
    await bridge.removeDisplay()
    await bridge.removeAllDisplays()
    await bridge.setDisplayMode({ index: 0, width: 1920, height: 1080, hz: 60 })
    await bridge.applyAdminConfig({ customModes: [], parentGpu: 'auto' })
    await bridge.updateSettings({ keepScreenOn: true })
    await bridge.openDisplaySettings()
    await bridge.showMainWindow()

    const cmds = runtime.invokeCalls.map((c) => c.cmd)
    expect(cmds).toEqual([
      'get_snapshot',
      'install_driver',
      'uninstall_driver',
      'add_display',
      'remove_display',
      'remove_display',
      'remove_all_displays',
      'set_display_mode',
      'apply_admin_config',
      'update_settings',
      'open_display_settings',
      'show_main_window'
    ])
  })

  test('setDisplayMode wraps the payload under `input`', async () => {
    const { createTauriBridge } = await loadBridge()
    const bridge = createTauriBridge()
    await bridge.setDisplayMode({ index: 1, width: 2560, height: 1440, hz: 144 })
    expect(runtime.invokeCalls).toEqual([
      {
        cmd: 'set_display_mode',
        args: { input: { index: 1, width: 2560, height: 1440, hz: 144 } }
      }
    ])
  })

  test('applyAdminConfig wraps the payload under `input`', async () => {
    const { createTauriBridge } = await loadBridge()
    const bridge = createTauriBridge()
    await bridge.applyAdminConfig({
      customModes: [{ width: 1920, height: 1080, hz: 60 }],
      parentGpu: 'nvidia'
    })
    expect(runtime.invokeCalls[0].args).toEqual({
      input: {
        customModes: [{ width: 1920, height: 1080, hz: 60 }],
        parentGpu: 'nvidia'
      }
    })
  })

  test('updateSettings wraps the payload under `patch`', async () => {
    const { createTauriBridge } = await loadBridge()
    const bridge = createTauriBridge()
    await bridge.updateSettings({ theme: 'dark', language: 'zh-CN' })
    expect(runtime.invokeCalls[0].args).toEqual({
      patch: { theme: 'dark', language: 'zh-CN' }
    })
  })

  test('removeDisplay forwards `index` (or undefined for default)', async () => {
    const { createTauriBridge } = await loadBridge()
    const bridge = createTauriBridge()
    await bridge.removeDisplay(3)
    await bridge.removeDisplay()
    expect(runtime.invokeCalls).toEqual([
      { cmd: 'remove_display', args: { index: 3 } },
      { cmd: 'remove_display', args: { index: undefined } }
    ])
  })
})

describe('subscribeSnapshot / onLanguageChanged — sync unsubscribe adapter', () => {
  test('subscribeSnapshot returns Unsubscribe synchronously and forwards payloads', async () => {
    const { createTauriBridge } = await loadBridge()
    const bridge = createTauriBridge()
    const received: Array<{ x: number }> = []

    const unsub = bridge.subscribeSnapshot(
      (snapshot) => received.push(snapshot as unknown as { x: number })
    )
    expect(typeof unsub).toBe('function')
    expect(runtime.listenCalls).toHaveLength(1)
    expect(runtime.listenCalls[0].event).toBe('easy-virtual-display:snapshot-changed')

    // listen() hasn't resolved yet, but the listener IS already registered (Tauri keeps
    // the handler bound while the Promise is in flight). Emit one event to prove it.
    runtime.emit('easy-virtual-display:snapshot-changed', { x: 1 })
    expect(received).toEqual([{ x: 1 }])

    await runtime.flushListenPromises()
    unsub()
    expect(runtime.activeUnlisteners.size).toBe(0)
  })

  test('unsubscribing BEFORE listen() resolves still cancels the underlying listener', async () => {
    const { createTauriBridge } = await loadBridge()
    const bridge = createTauriBridge()
    const received: unknown[] = []
    const unsub = bridge.subscribeSnapshot((s) => received.push(s))

    // User unsubscribes immediately, before the listen() Promise resolves.
    unsub()
    expect(runtime.activeUnlisteners.size).toBe(0)

    // Now listen() resolves. The adapter must invoke the UnlistenFn right away.
    await runtime.flushListenPromises()
    expect(runtime.activeUnlisteners.size).toBe(0)

    // Any post-cancel event must not reach the consumer (the listener still exists in
    // the fake runtime, but the adapter's wrapper drops the call when `cancelled` is true).
    runtime.emit('easy-virtual-display:snapshot-changed', { ignored: true })
    expect(received).toEqual([])
  })

  test('onLanguageChanged uses the language-changed event', async () => {
    const { createTauriBridge } = await loadBridge()
    const bridge = createTauriBridge()
    const received: string[] = []

    const unsub = bridge.onLanguageChanged((lang) => received.push(lang))
    expect(runtime.listenCalls[0].event).toBe('easy-virtual-display:language-changed')

    runtime.emit('easy-virtual-display:language-changed', 'zh-CN')
    expect(received).toEqual(['zh-CN'])

    await runtime.flushListenPromises()
    unsub()
  })
})
