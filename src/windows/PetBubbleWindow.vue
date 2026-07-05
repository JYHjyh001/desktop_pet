<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { emit, listen } from '@tauri-apps/api/event'
import { useWindowOpenAnimation } from '../composables/useWindowOpenAnimation'
import type { DrawerTheme } from '../types/app'

type PetBubbleKind = 'bubble' | 'badge' | 'completion'
type PetBubblePlacement = 'top' | 'bottom' | 'side'
type PetBubbleChannel = 'codex' | 'translation'

interface PetBubbleRenderItem {
  id: string
  channel: PetBubbleChannel | string
  kind: PetBubbleKind | string
  state: string
  message: string
  expanded?: boolean
}

interface PetBubbleRenderPayload {
  kind: PetBubbleKind | string
  state: string
  message: string
  theme: DrawerTheme | string
  items?: PetBubbleRenderItem[]
  placement: PetBubblePlacement | string
  tailX: number
}

interface PetBubblePlacementPayload {
  placement: PetBubblePlacement | string
  tailX: number
}

const visible = ref(false)
const kind = ref<PetBubbleKind>('bubble')
const items = ref<PetBubbleRenderItem[]>([])
const theme = ref<DrawerTheme>('light')
const placement = ref<PetBubblePlacement>('top')
const tailX = ref(150)
const hoveredItemId = ref('')
const { windowOpenAnimationClass } = useWindowOpenAnimation('bubble')
const bubbleHorizontalPadding = 8
const bubbleTailEdgePadding = 18
const hoverRepositionLeaveDistancePx = 4
const hoverRepositionLeaveWindowMs = 1200
let unlistenBubbleUpdate: (() => void) | null = null
let unlistenBubblePlacement: (() => void) | null = null
let hoverOrigin:
  | {
      id: string
      screenX: number
      screenY: number
      startedAt: number
    }
  | null = null

const windowClass = computed(() => [
  'pet-bubble-window',
  `theme-${theme.value}`,
  `kind-${kind.value}`,
  `placement-${placement.value}`,
  windowOpenAnimationClass.value,
])
const visibleItems = computed(() => items.value.filter((item) => item.message.trim()))
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
  const payloadItems = Array.isArray(payload.items) ? payload.items : []
  items.value =
    payloadItems.length > 0
      ? payloadItems.map(normalizeItem).filter((item) => item.message)
      : [
          normalizeItem({
            id: 'codex',
            channel: 'codex',
            kind: kind.value,
            state: payload.state,
            message: payload.message,
          }),
        ].filter((item) => item.message)
  theme.value = payload.theme === 'animal-island' ? 'animal-island' : 'light'
  applyPlacementPayload(payload)
  visible.value = visibleItems.value.length > 0
  if (!visibleItems.value.some((item) => item.id === hoveredItemId.value)) {
    hoveredItemId.value = ''
  }
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

function normalizeItem(item: PetBubbleRenderItem): PetBubbleRenderItem {
  const itemKind =
    item.kind === 'badge' || item.kind === 'completion' ? item.kind : 'bubble'
  const channel = normalizeChannel(item.channel)
  return {
    id: item.id.trim() || `${channel}-${items.value.length}`,
    channel,
    kind: itemKind,
    state: normalizeState(item.state),
    message: item.message.trim(),
    expanded: channel === 'translation' && Boolean(item.expanded),
  }
}

function normalizeChannel(value: string): PetBubbleChannel {
  return value === 'translation' ? 'translation' : 'codex'
}

function isTailItem(index: number) {
  if (visibleItems.value.length <= 0) {
    return false
  }
  if (placement.value === 'bottom') {
    return index === 0
  }
  return index === visibleItems.value.length - 1
}

function bubbleItemClass(item: PetBubbleRenderItem, index: number) {
  const itemKind = item.kind === 'badge' || item.kind === 'completion' ? item.kind : 'bubble'
  return [
    itemKind === 'bubble' ? 'codex-pet-bubble' : 'codex-pet-badge',
    `state-${normalizeState(item.state)}`,
    `channel-${normalizeChannel(item.channel)}`,
    {
      'tail-anchor': itemKind === 'bubble' && isTailItem(index),
      expanded: isExpandedItem(item),
    },
  ]
}

function bubbleTextClass(item: PetBubbleRenderItem) {
  return [
    item.kind === 'bubble' ? 'codex-pet-bubble-text' : '',
    {
      expanded: isExpandedItem(item),
    },
  ]
}

function isExpandedItem(item: PetBubbleRenderItem) {
  return (
    normalizeChannel(item.channel) === 'translation' &&
    Boolean(item.expanded) &&
    hoveredItemId.value === item.id
  )
}

async function handleItemHover(item: PetBubbleRenderItem, hovered: boolean, event?: Event) {
  const channel = normalizeChannel(item.channel)
  if (hovered) {
    hoveredItemId.value = item.id
    if (event instanceof PointerEvent) {
      hoverOrigin = {
        id: item.id,
        screenX: event.screenX,
        screenY: event.screenY,
        startedAt: Date.now(),
      }
    }
  } else if (hoveredItemId.value === item.id) {
    if (shouldIgnorePointerLeaveFromReposition(item, event)) {
      return
    }
    hoveredItemId.value = ''
    if (hoverOrigin?.id === item.id) {
      hoverOrigin = null
    }
  }

  if (channel !== 'translation') {
    return
  }

  try {
    await emit('pet-bubble-item-hover-changed', {
      channel,
      id: item.id,
      hovered,
    })
  } catch (err) {
    console.error(err)
  }
}

function shouldIgnorePointerLeaveFromReposition(item: PetBubbleRenderItem, event?: Event) {
  if (normalizeChannel(item.channel) !== 'translation' || !(event instanceof PointerEvent)) {
    return false
  }
  if (hoverOrigin?.id !== item.id) {
    return false
  }
  if (Date.now() - hoverOrigin.startedAt > hoverRepositionLeaveWindowMs) {
    return false
  }

  const dx = event.screenX - hoverOrigin.screenX
  const dy = event.screenY - hoverOrigin.screenY
  return Math.hypot(dx, dy) <= hoverRepositionLeaveDistancePx
}

async function handleItemClick(item: PetBubbleRenderItem) {
  const channel = normalizeChannel(item.channel)
  if (channel === 'codex') {
    await ackCodexNotifications()
  }

  try {
    await emit('pet-bubble-item-clicked', {
      channel,
      id: item.id,
    })
  } catch (err) {
    console.error(err)
  }
}

async function ackCodexNotifications() {
  try {
    await invoke('ack_codex_notifications')
  } catch (err) {
    console.error(err)
  }
}
</script>

<template>
  <main :class="windowClass" :style="bubbleStyle">
    <TransitionGroup
      v-if="visible && visibleItems.length > 0"
      name="codex-bubble"
      tag="div"
      class="pet-bubble-stack"
    >
      <button
        v-for="(item, index) in visibleItems"
        :key="item.id"
        :class="bubbleItemClass(item, index)"
        type="button"
        role="status"
        aria-live="polite"
        @pointerenter="handleItemHover(item, true, $event)"
        @pointerleave="handleItemHover(item, false, $event)"
        @focus="handleItemHover(item, true, $event)"
        @blur="handleItemHover(item, false, $event)"
        @click.stop="handleItemClick(item)"
        @pointerdown.stop
        @contextmenu.stop.prevent
      >
        <span
          :class="item.kind === 'bubble' ? 'codex-pet-bubble-dot' : 'codex-pet-badge-dot'"
          aria-hidden="true"
        ></span>
        <span :class="bubbleTextClass(item)">
          {{ item.message }}
        </span>
      </button>
    </TransitionGroup>
  </main>
</template>
