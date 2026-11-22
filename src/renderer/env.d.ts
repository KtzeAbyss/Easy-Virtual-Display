/// <reference types="vite/client" />

import type { EasyVirtualDisplayBridge } from '../shared'

declare global {
  interface Window {
    easyVirtualDisplay: EasyVirtualDisplayBridge
  }

  /** Injected from package.json by Vite `define`. */
  const __APP_VERSION__: string
}

export {}
