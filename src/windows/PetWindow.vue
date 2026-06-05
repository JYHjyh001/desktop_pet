<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import Pet from '../components/Pet.vue'
import type { PetSkinSummary } from '../types/app'
import { resolvePetSkinAnimations } from '../utils/defaultPet'

const petState = ref<'idle' | 'hover' | 'dragging' | 'click'>('idle')
const currentPetSkin = ref<PetSkinSummary | null>(null)
const pointerDown = ref(false)
const dragStarted = ref(false)
const pointerStart = ref({ x: 0, y: 0 })
const protectedUntil = ref(0)
const pointerDownAt = ref(0)
const CLICK_ANIMATION_MS = 1200
let stateTimer: number | null = null
let dragWatchTimer: number | null = null
let unlistenPetImage: (() => void) | null = null
let preloadedPetMedia: Array<HTMLImageElement | HTMLVideoElement> = []

const appWindow = getCurrentWindow()

onMounted(async () => {
  await loadPetSkin()
  unlistenPetImage = await listen('pet-skin-updated', () => {
    void loadPetSkin()
  })
})

onBeforeUnmount(() => {
  unlistenPetImage?.()
  if (stateTimer !== null) {
    window.clearTimeout(stateTimer)
  }
  if (dragWatchTimer !== null) {
    window.clearTimeout(dragWatchTimer)
  }
  clearPreloadedPetMedia()
})

const currentPetImage = computed<string | undefined>(() => {
  const animations = resolvePetSkinAnimations(currentPetSkin.value)

  if (petState.value === 'dragging') {
    return animations.dragging || animations.idle
  }

  if (petState.value === 'click') {
    return animations.click || animations.idle
  }

  if (petState.value === 'hover') {
    return animations.hover || animations.idle
  }

  return animations.idle || undefined
})

async function loadPetSkin() {
  try {
    const skin = await invoke<PetSkinSummary>('get_current_pet_skin')
    currentPetSkin.value = skin
    preloadPetSkinAnimations(skin)
  } catch (err) {
    console.error(err)
    currentPetSkin.value = null
    clearPreloadedPetMedia()
  }
}

function isVideoSource(source: string) {
  return /^data:video\//i.test(source) || /\.(webm|mp4)(?:[?#].*)?$/i.test(source)
}

function preloadPetSkinAnimations(skin: PetSkinSummary | null) {
  clearPreloadedPetMedia()
  const animations = resolvePetSkinAnimations(skin)
  const sources = Array.from(
    new Set(Object.values(animations).filter((source): source is string => Boolean(source))),
  )

  preloadedPetMedia = sources.map((source) => {
    if (isVideoSource(source)) {
      const video = document.createElement('video')
      video.preload = 'auto'
      video.muted = true
      video.playsInline = true
      video.src = source
      video.load()
      return video
    }

    const image = new Image()
    image.decoding = 'async'
    image.src = source
    return image
  })
}

function clearPreloadedPetMedia() {
  for (const media of preloadedPetMedia) {
    if (media instanceof HTMLVideoElement) {
      media.pause()
      media.removeAttribute('src')
      media.load()
    }
  }
  preloadedPetMedia = []
}

async function savePosition() {
  try {
    const position = await appWindow.outerPosition()
    await invoke('save_pet_position', { position: { x: position.x, y: position.y } })
  } catch (err) {
    console.error(err)
  }
}

function isStateProtected() {
  return Date.now() < protectedUntil.value
}

async function showState(
  state: 'idle' | 'hover' | 'dragging' | 'click',
  duration = 0,
  fallback: 'idle' | 'hover' = 'idle',
) {
  if (stateTimer !== null) {
    window.clearTimeout(stateTimer)
    stateTimer = null
  }

  petState.value = state

  if (duration <= 0) {
    protectedUntil.value = 0
    await nextTick()
    await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()))
    return
  }

  protectedUntil.value = Date.now() + duration
  stateTimer = window.setTimeout(() => {
    protectedUntil.value = 0
    petState.value = fallback
    stateTimer = null
  }, duration)

  await nextTick()
  await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()))
}

function handleMouseEnter() {
  if (!isStateProtected()) {
    petState.value = 'hover'
  }
}

function handleMouseLeave() {
  if (!isStateProtected()) {
    petState.value = 'idle'
  }
}

async function watchDragEnd() {
  if (dragWatchTimer !== null) {
    window.clearTimeout(dragWatchTimer)
  }

  const tick = async () => {
    try {
      const isPrimaryButtonPressed = await invoke<boolean>('is_primary_mouse_button_pressed')

      if (!isPrimaryButtonPressed) {
        dragStarted.value = false
        protectedUntil.value = 0
        petState.value = 'idle'
        dragWatchTimer = null
        await savePosition()
        return
      }

      petState.value = 'dragging'
      dragWatchTimer = window.setTimeout(tick, 80)
    } catch (err) {
      console.error(err)
      dragStarted.value = false
      protectedUntil.value = 0
      petState.value = 'idle'
      dragWatchTimer = null
    }
  }

  dragWatchTimer = window.setTimeout(tick, 80)
}

function startPointer(event: PointerEvent) {
  if (event.button !== 0) {
    return
  }

  ;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
  pointerDown.value = true
  dragStarted.value = false
  pointerDownAt.value = Date.now()
  pointerStart.value = { x: event.clientX, y: event.clientY }
  void showState('dragging')
  protectedUntil.value = Number.MAX_SAFE_INTEGER
  void invoke('hide_pet_menu')
}

async function maybeStartDrag(event: PointerEvent) {
  if (!pointerDown.value || dragStarted.value) {
    return
  }

  const dx = event.clientX - pointerStart.value.x
  const dy = event.clientY - pointerStart.value.y
  if (Math.hypot(dx, dy) < 5) {
    return
  }

  dragStarted.value = true
  await showState('dragging')
  protectedUntil.value = Number.MAX_SAFE_INTEGER

  try {
    await appWindow.startDragging()
  } finally {
    pointerDown.value = false
    void watchDragEnd()
  }
}

async function finishPointer(event: PointerEvent) {
  if (!pointerDown.value) {
    return
  }

  try {
    ;(event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId)
  } catch {
    // Pointer capture can be released by the OS-level window drag.
  }
  pointerDown.value = false

  if (dragStarted.value) {
    await savePosition()
    return
  }

  const heldFor = Date.now() - pointerDownAt.value
  const isShortPress = heldFor <= 220
  protectedUntil.value = 0

  if (!isShortPress) {
    petState.value = 'idle'
    return
  }

  await toggleDrawer()
}

function cancelPointer() {
  pointerDown.value = false
  pointerDownAt.value = 0
  if (!dragStarted.value && !isStateProtected()) {
    petState.value = 'idle'
  }
}

async function toggleDrawer() {
  await showState('click', CLICK_ANIMATION_MS)

  try {
    await invoke('hide_pet_menu')
    await invoke('toggle_drawer')
  } catch (err) {
    console.error(err)
  }
}

async function openMenu(event: MouseEvent) {
  event.preventDefault()
  await invoke('show_pet_menu', {
    x: Math.round(event.clientX),
    y: Math.round(event.clientY),
  })
}
</script>

<template>
  <main class="pet-window" @contextmenu="openMenu">
    <button
      class="pet-button"
      type="button"
      @mouseenter="handleMouseEnter"
      @mouseleave="handleMouseLeave"
      @pointerdown="startPointer"
      @pointermove="maybeStartDrag"
      @pointerup="finishPointer"
      @pointercancel="cancelPointer"
      @lostpointercapture="cancelPointer"
      @contextmenu.stop="openMenu"
    >
      <Pet :state="petState" :image-url="currentPetImage" />
    </button>
  </main>
</template>
