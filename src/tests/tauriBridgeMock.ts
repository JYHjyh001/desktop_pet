import { clearMocks, mockIPC, mockWindows } from '@tauri-apps/api/mocks'
import { vi } from 'vitest'

export const tauriCommandMock = vi.fn()

export function installTauriBridgeMock() {
  mockWindows('translator')
  mockIPC(
    (command, payload) => tauriCommandMock(command, payload),
    { shouldMockEvents: true },
  )
}

export function resetTauriBridgeMock() {
  clearMocks()
  tauriCommandMock.mockReset()
  installTauriBridgeMock()
}

export function clearTauriBridgeMock() {
  clearMocks()
}
