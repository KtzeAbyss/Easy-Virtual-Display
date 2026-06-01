/// <reference types="vite/client" />

import type { EasyVirtualDisplayBridge } from '../shared'

declare global {
  interface Window {
    easyVirtualDisplay: EasyVirtualDisplayBridge
  }
}

export {}
