import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'

type WindowOpenAnimationVariant = 'drawer' | 'panel' | 'menu' | 'bubble' | 'pet'

interface WindowOpenAnimationPayload {
  label?: string
  variant?: WindowOpenAnimationVariant | string
}

const WINDOW_OPEN_ANIMATION_MS = 360

export function useWindowOpenAnimation(variant: WindowOpenAnimationVariant = 'panel') {
  const prepared = ref(variant !== 'pet')
  const opening = ref(false)
  let currentWindowLabel = ''
  let animationTimer: number | null = null
  let animationFrame: number | null = null
  let unlistenWindowOpenAnimation: (() => void) | null = null
  let unlistenWindowOpenPrepare: (() => void) | null = null

  function clearAnimationTimer() {
    if (animationFrame !== null) {
      window.cancelAnimationFrame(animationFrame)
      animationFrame = null
    }

    if (animationTimer !== null) {
      window.clearTimeout(animationTimer)
      animationTimer = null
    }
  }

  function triggerOpenAnimation() {
    clearAnimationTimer()

    if (opening.value) {
      opening.value = false
      animationFrame = window.requestAnimationFrame(() => {
        animationFrame = null
        startOpenAnimation()
      })
      return
    }

    startOpenAnimation()
  }

  function startOpenAnimation() {
    prepared.value = true
    opening.value = true
    animationTimer = window.setTimeout(() => {
      opening.value = false
      prepared.value = false
      animationTimer = null
    }, WINDOW_OPEN_ANIMATION_MS)
  }

  onMounted(async () => {
    try {
      currentWindowLabel = getCurrentWindow().label
    } catch {
      currentWindowLabel = ''
    }

    unlistenWindowOpenAnimation = await listen<WindowOpenAnimationPayload>(
      'window-open-animation',
      (event) => {
        if (event.payload.label && currentWindowLabel && event.payload.label !== currentWindowLabel) {
          return
        }

        triggerOpenAnimation()
      },
    )
    unlistenWindowOpenPrepare = await listen<WindowOpenAnimationPayload>(
      'window-open-prepare',
      (event) => {
        if (event.payload.label && currentWindowLabel && event.payload.label !== currentWindowLabel) {
          return
        }

        if (!opening.value) {
          prepared.value = true
        }
      },
    )
  })

  onBeforeUnmount(() => {
    clearAnimationTimer()
    unlistenWindowOpenAnimation?.()
    unlistenWindowOpenPrepare?.()
  })

  const windowOpenAnimationClass = computed(() => ({
    'window-open-prepared': prepared.value,
    'window-opening': opening.value,
    [`window-opening-${variant}`]: opening.value,
  }))

  return {
    windowOpenAnimationClass,
  }
}
