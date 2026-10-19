import { onBeforeUnmount } from 'vue'
import { listen as rawListen } from '@tauri-apps/api/event'

/** Dispose subscriptions even when their registration finishes after unmount. */
export function useScopedListeners() {
  let disposed = false
  const cleanups = new Set<() => void>()
  function track(promise: Promise<() => void>): Promise<() => void> {
    return promise.then((cleanup) => {
      let active = true
      const release = () => {
        if (!active) return
        active = false
        cleanups.delete(release)
        cleanup()
      }
      if (disposed) release()
      else cleanups.add(release)
      return release
    })
  }
  const listen: typeof rawListen = (event, handler, options) => track(rawListen(event, handler, options))
  onBeforeUnmount(() => {
    disposed = true
    for (const cleanup of cleanups) cleanup()
  })
  return { listen, track, get disposed() { return disposed } }
}
