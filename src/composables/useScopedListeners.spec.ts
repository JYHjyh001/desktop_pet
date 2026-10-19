import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import { useScopedListeners } from './useScopedListeners'

describe('scoped listeners', () => {
  it('disposes a registration that resolves after unmount exactly once', async () => {
    let resolve!: (cleanup: () => void) => void
    let release!: Promise<() => void>
    const cleanup = vi.fn()
    const wrapper = mount(defineComponent({ setup() {
      release = useScopedListeners().track(new Promise((done) => { resolve = done }))
      return () => null
    } }))
    wrapper.unmount()
    resolve(cleanup)
    const unlisten = await release
    unlisten()
    expect(cleanup).toHaveBeenCalledTimes(1)
  })
})
