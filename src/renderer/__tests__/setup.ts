/// <reference types="vitest/globals" />
import '@testing-library/jest-dom'

const mockBridge: typeof window.easyVirtualDisplay = {
  getSnapshot: vi.fn(),
  subscribeSnapshot: vi.fn(() => vi.fn()),
  onLanguageChanged: vi.fn(() => vi.fn()),
  installDriver: vi.fn().mockResolvedValue(undefined),
  uninstallDriver: vi.fn().mockResolvedValue(undefined),
  addDisplay: vi.fn().mockResolvedValue(undefined),
  removeDisplay: vi.fn().mockResolvedValue(undefined),
  removeAllDisplays: vi.fn().mockResolvedValue(undefined),
  setDisplayMode: vi.fn().mockResolvedValue(undefined),
  applyAdminConfig: vi.fn().mockResolvedValue(undefined),
  updateSettings: vi.fn().mockResolvedValue(undefined),
  openDisplaySettings: vi.fn().mockResolvedValue(undefined),
  showMainWindow: vi.fn().mockResolvedValue(undefined)
}

Object.defineProperty(window, 'easyVirtualDisplay', {
  value: mockBridge,
  writable: true,
  configurable: true
})
