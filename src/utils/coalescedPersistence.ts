/** Serial, debounced snapshots. Disposal cancels timers and prevents late retries. */
export function createCoalescedPersistence(options: {
  enabled: () => boolean
  save: () => Promise<void>
  onError: (error: unknown) => void
  delayMs?: number
}) {
  let timer: ReturnType<typeof setTimeout> | null = null
  let running: Promise<void> | null = null
  let queued = false
  let disposed = false
  function clear() { if (timer !== null) { clearTimeout(timer); timer = null } }
  function flush(): Promise<void> {
    clear()
    if (disposed || !options.enabled()) return Promise.resolve()
    if (running) { queued = true; return running }
    running = (async () => {
      do {
        queued = false
        try { await options.save() }
        catch (error) { options.onError(error); break }
      } while (queued && !disposed && options.enabled())
    })().finally(() => { running = null })
    return running
  }
  return {
    flush, clear,
    schedule() {
      if (disposed || !options.enabled()) return
      clear()
      timer = setTimeout(() => { timer = null; void flush() }, options.delayMs ?? 260)
    },
    dispose() { disposed = true; queued = false; clear() },
  }
}
