<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useWindowOpenAnimation } from '../composables/useWindowOpenAnimation'
import type { DrawerTheme } from '../types/app'

type PetBubbleKind = 'bubble' | 'badge' | 'completion'
type PetBubblePlacement = 'top' | 'bottom' | 'side'

interface PetBubbleRenderPayload {
  kind: PetBubbleKind | string
  state: string
  message: string
  theme: DrawerTheme | string
  placement: PetBubblePlacement | string
  tailX: number
}

interface PetBubblePlacementPayload {
  placement: PetBubblePlacement | string
  tailX: number
}

const visible = ref(false)
const kind = ref<PetBubbleKind>('bubble')
const state = ref('connected')
const message = ref('')
const theme = ref<DrawerTheme>('light')
const placement = ref<PetBubblePlacement>('top')
const tailX = ref(150)
const { windowOpenAnimationClass } = useWindowOpenAnimation('bubble')
const bubbleHorizontalPadding = 8
const bubbleTailEdgePadding = 18
let unlistenBubbleUpdate: (() => void) | null = null
let unlistenBubblePlacement: (() => void) | null = null

const windowClass = computed(() => [
  'pet-bubble-window',
  `theme-${theme.value}`,
  `kind-${kind.value}`,
  `placement-${placement.value}`,
  windowOpenAnimationClass.value,
])
const bubbleStyle = computed(() => {
  const tailPosition = Math.round(tailX.value - bubbleHorizontalPadding)
  return {
    '--tail-x':
      kind.value === 'bubble'
        ? `clamp(${bubbleTailEdgePadding}px, ${tailPosition}px, calc(100% - ${bubbleTailEdgePadding}px))`
        : `${tailX.value}px`,
  }
})

onMounted(async () => {
  unlistenBubbleUpdate = await listen<PetBubbleRenderPayload>('pet-bubble-updated', (event) => {
    applyRenderPayload(event.payload)
  })
  unlistenBubblePlacement = await listen<PetBubblePlacementPayload>(
    'pet-bubble-placement-updated',
    (event) => {
      applyPlacementPayload(event.payload)
    },
  )
})

onBeforeUnmount(() => {
  unlistenBubbleUpdate?.()
  unlistenBubblePlacement?.()
})

function applyRenderPayload(payload: PetBubbleRenderPayload) {
  kind.value =
    payload.kind === 'badge' || payload.kind === 'completion' ? payload.kind : 'bubble'
  state.value = normalizeState(payload.state)
  message.value = payload.message.trim()
  theme.value = payload.theme === 'animal-island' ? 'animal-island' : 'light'
  applyPlacementPayload(payload)
  visible.value = Boolean(message.value)
}

function applyPlacementPayload(payload: PetBubblePlacementPayload) {
  placement.value =
    payload.placement === 'bottom' || payload.placement === 'side' ? payload.placement : 'top'
  tailX.value = Number.isFinite(payload.tailX) ? payload.tailX : 150
}

function normalizeState(value: string) {
  switch (value) {
    case 'disconnected':
    case 'starting':
    case 'connected':
    case 'running':
    case 'waiting':
    case 'review':
    case 'completed':
    case 'failed':
      return value
    default:
      return 'connected'
  }
}

async function ackCodexNotifications() {
  try {
    await invoke('ack_codex_notifications')
  } catch (err) {
    console.error(err)
  } finally {
    visible.value = false
    try {
      await invoke('hide_pet_bubble')
    } catch (err) {
      console.error(err)
    }
  }
}
</script>

<template>
  <main :class="windowClass" :style="bubbleStyle">
    <Transition name="codex-bubble">
      <button
        v-if="visible && kind === 'bubble'"
        class="codex-pet-bubble"
        :class="`state-${state}`"
        type="button"
        role="status"
        aria-live="polite"
        @click.stop="ackCodexNotifications"
        @pointerdown.stop
        @contextmenu.stop.prevent
      >
        <span class="codex-pet-bubble-dot" aria-hidden="true"></span>
        <span class="codex-pet-bubble-text">{{ message }}</span>
      </button>
      <button
        v-else-if="visible"
        class="codex-pet-badge"
        :class="`state-${state}`"
        type="button"
        role="status"
        aria-live="polite"
        @click.stop="ackCodexNotifications"
        @pointerdown.stop
        @contextmenu.stop.prevent
      >
        <span class="codex-pet-badge-dot" aria-hidden="true"></span>
        <span>{{ message }}</span>
      </button>
    </Transition>
  </main>
</template>
