import { describe, expect, it, vi } from 'vitest'
import { createCoalescedPersistence } from './coalescedPersistence'

describe('coalesced persistence', () => {
  it('serializes pending writes and uses the latest snapshot for the trailing save', async () => {
    let resolve!: () => void
    let value = 1
    const snapshots: number[] = []
    const saver = createCoalescedPersistence({ enabled: () => true, onError: vi.fn(), save: async () => {
      snapshots.push(value)
      if (snapshots.length === 1) await new Promise<void>((done) => { resolve = done })
    } })
    const first = saver.flush()
    value = 2
    const second = saver.flush()
    value = 3
    resolve()
    await Promise.all([first, second])
    expect(snapshots).toEqual([1, 3])
    saver.dispose()
  })
  it('prevents late timers and queued retries after disposal', async () => {
    vi.useFakeTimers()
    const save = vi.fn(async () => {})
    const saver = createCoalescedPersistence({ enabled: () => true, save, onError: vi.fn() })
    saver.schedule()
    saver.dispose()
    await vi.runAllTimersAsync()
    await saver.flush()
    expect(save).not.toHaveBeenCalled()
    vi.useRealTimers()
  })
})
