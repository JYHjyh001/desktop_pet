import { afterEach, beforeEach } from 'vitest'
import { clearTauriBridgeMock, resetTauriBridgeMock } from './tauriBridgeMock'

beforeEach(() => {
  resetTauriBridgeMock()
})

afterEach(() => {
  clearTauriBridgeMock()
})
