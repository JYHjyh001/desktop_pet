<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import Pet from '../components/Pet.vue'
import { useWindowOpenAnimation } from '../composables/useWindowOpenAnimation'
import type {
  CodexAppServerStatus,
  DrawerTheme,
  PetActionBinding,
  PetAnimationKey,
  PetDrawerConfig,
  PetSkinSummary,
} from '../types/app'
import { petAnimationFields, resolvePetSkinAnimations } from '../utils/defaultPet'

type CodexBubbleState =
  | 'disconnected'
  | 'starting'
  | 'connected'
  | 'running'
  | 'waiting'
  | 'review'
  | 'completed'
  | 'failed'
type PetBubbleKind = 'bubble'

interface PetBubbleWindowPayload {
  kind: PetBubbleKind
  state: CodexBubbleState
  message: string
  theme: DrawerTheme
}

interface PetActionBindings {
  petSingleClick: PetActionBinding
  petDoubleClick: PetActionBinding
  petRightClick: PetActionBinding
}

interface PetActionContext {
  x?: number
  y?: number
  playClick?: boolean
}

const petState = ref<PetAnimationKey>('idle')
const { windowOpenAnimationClass } = useWindowOpenAnimation('pet')
const currentPetSkin = ref<PetSkinSummary | null>(null)
const drawerTheme = ref<DrawerTheme>('light')
const pointerDown = ref(false)
const dragStarted = ref(false)
const pointerStart = ref({ x: 0, y: 0 })
const protectedUntil = ref(0)
const pointerDownAt = ref(0)
const dragDirection = ref<'left' | 'right'>('right')
const petHovered = ref(false)
const codexBubbleText = ref('')
const codexBubbleState = ref<CodexBubbleState>('connected')
const codexBubbleVisible = ref(false)
const codexCompletionHoldActive = ref(false)
const codexCompletionCount = ref(0)
const codexActivityPetState = ref<PetAnimationKey | null>(null)
const latestCodexStatus = ref<CodexAppServerStatus | null>(null)
const petActionBindings = ref<PetActionBindings>(defaultPetActionBindings())
const CLICK_ANIMATION_MS = 1200
const SINGLE_CLICK_DELAY_MS = 260
const DRAG_DIRECTION_THRESHOLD_PX = 2
let stateTimer: number | null = null
let dragWatchTimer: number | null = null
let codexBubbleTimer: number | null = null
let pendingClickTimer: number | null = null
let codexStatusRequestId = 0
let codexBubbleFromHover = false
let suppressNextCodexAckStatus = false
let lastDragPosition: { x: number; y: number } | null = null
let unlistenPetImage: (() => void) | null = null
let unlistenPetAnimationState: (() => void) | null = null
let unlistenCodexStatus: (() => void) | null = null
let unlistenThemeChanged: (() => void) | null = null
let unlistenPetActionBindingsChanged: (() => void) | null = null
let preloadedPetMedia: Array<HTMLImageElement | HTMLVideoElement> = []

const appWindow = getCurrentWindow()
const themeClass = computed(() => `theme-${drawerTheme.value}`)
const showCodexCompletionCounter = computed(
  () => codexCompletionCount.value > 1 && !petHovered.value,
)
const codexCompletionCounterLabel = computed(() => {
  const count = codexCompletionCount.value
  return count > 99 ? '99+' : String(count)
})

onMounted(async () => {
  await loadTheme()
  await loadPetSkin()
  unlistenPetImage = await listen('pet-skin-updated', () => {
    void loadPetSkin()
  })
  unlistenPetAnimationState = await listen<{
    state?: string
    durationMs?: number
    fallback?: string
  }>('pet-animation-state', (event) => {
    const state = event.payload?.state
    if (!isPetAnimationKey(state)) {
      return
    }

    const fallback = isPetAnimationKey(event.payload?.fallback) ? event.payload.fallback : 'idle'
    if (isCodexActivityState(state)) {
      codexActivityPetState.value = state
    }
    if (state === 'jumping' && codexCompletionHoldActive.value) {
      void holdCodexCompletionAnimation()
      return
    }
    void showState(state, event.payload?.durationMs ?? 0, fallback)
  })
  unlistenCodexStatus = await listen<CodexAppServerStatus>('codex-status-updated', (event) => {
    handleCodexStatus(event.payload)
  })
  unlistenThemeChanged = await listen<string>('ui-theme-changed', (event) => {
    drawerTheme.value = normalizeDrawerTheme(event.payload)
    void syncPetBubbleWindow()
  })
  unlistenPetActionBindingsChanged = await listen<PetDrawerConfig['shortcut']>(
    'pet-action-bindings-changed',
    (event) => {
      syncPetActionBindings(event.payload)
    },
  )
})

onBeforeUnmount(() => {
  unlistenPetImage?.()
  unlistenPetAnimationState?.()
  unlistenCodexStatus?.()
  unlistenThemeChanged?.()
  unlistenPetActionBindingsChanged?.()
  if (stateTimer !== null) {
    window.clearTimeout(stateTimer)
  }
  if (dragWatchTimer !== null) {
    window.clearTimeout(dragWatchTimer)
  }
  if (codexBubbleTimer !== null) {
    window.clearTimeout(codexBubbleTimer)
  }
  if (pendingClickTimer !== null) {
    window.clearTimeout(pendingClickTimer)
  }
  void hidePetBubbleWindow()
  clearPreloadedPetMedia()
})

const currentPetImage = computed<string | undefined>(() => {
  const animations = resolvePetSkinAnimations(currentPetSkin.value)
  return animations[petState.value] || animations.idle || undefined
})

async function loadTheme() {
  try {
    const config = await invoke<PetDrawerConfig>('get_config')
    drawerTheme.value = normalizeDrawerTheme(config.drawer.theme)
    syncPetActionBindings(config.shortcut)
  } catch {
    drawerTheme.value = 'light'
    petActionBindings.value = defaultPetActionBindings()
  }
}

function normalizeDrawerTheme(value?: string | null): DrawerTheme {
  return value === 'animal-island' ? 'animal-island' : 'light'
}

function defaultPetActionBindings(): PetActionBindings {
  return {
    petSingleClick: 'smartCodexOrDrawer',
    petDoubleClick: 'toggleDrawer',
    petRightClick: 'petMenu',
  }
}

function syncPetActionBindings(shortcut?: PetDrawerConfig['shortcut'] | null) {
  petActionBindings.value = {
    petSingleClick: normalizePetActionBinding(shortcut?.petSingleClick, 'smartCodexOrDrawer'),
    petDoubleClick: normalizePetActionBinding(shortcut?.petDoubleClick, 'toggleDrawer'),
    petRightClick: normalizePetActionBinding(shortcut?.petRightClick, 'petMenu'),
  }
}

function normalizePetActionBinding(
  value: string | null | undefined,
  fallback: PetActionBinding,
): PetActionBinding {
  switch (value) {
    case 'smartCodexOrDrawer':
    case 'toggleDrawer':
    case 'showDrawer':
    case 'petMenu':
    case 'petChat':
    case 'story':
    case 'music':
    case 'none':
      return value
    default:
      return fallback
  }
}

function isPetAnimationKey(value: unknown): value is PetAnimationKey {
  return typeof value === 'string' && petAnimationFields.some((field) => field.key === value)
}

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

function handleCodexStatus(status: CodexAppServerStatus) {
  latestCodexStatus.value = status
  const shouldSuppressBubble = consumeSuppressedCodexAckStatus(status)
  updateCodexActivityPetState(status)
  updateCodexCompletionReminder(status)

  if (shouldSuppressBubble) {
    return
  }

  if (!petHovered.value) {
    dismissCodexBubble()
    return
  }

  showCodexBubble(status, { respectNotify: false, keepWhileHovered: true })
  if (shouldAcknowledgeCodexCompletionStatus(status)) {
    clearCodexCompletionReminder({ resetAnimation: true })
    void acknowledgeCodexCompletionReminder()
  }
}

function showCodexBubble(
  status: CodexAppServerStatus,
  options: { respectNotify?: boolean; keepWhileHovered?: boolean } = {},
) {
  const respectNotify = options.respectNotify ?? true

  if (respectNotify && status.notify === false) {
    return
  }

  const message = (status.summary?.message || status.message || codexBubbleMessage(status)).trim()
  if (!message) {
    hideCodexBubbleOnly()
    void syncPetBubbleWindow()
    return
  }
  const state = normalizeCodexBubbleState(status.summary?.state || status.state)

  if (!options.keepWhileHovered || !petHovered.value) {
    hideCodexBubbleOnly()
    void syncPetBubbleWindow()
    return
  }

  codexBubbleText.value = message
  codexBubbleState.value = state
  codexBubbleVisible.value = true
  codexBubbleFromHover = Boolean(options.keepWhileHovered)

  if (codexBubbleTimer !== null) {
    window.clearTimeout(codexBubbleTimer)
  }

  codexBubbleTimer = null
  void syncPetBubbleWindow()
}

function dismissCodexBubble() {
  if (codexBubbleTimer !== null) {
    window.clearTimeout(codexBubbleTimer)
    codexBubbleTimer = null
  }
  codexBubbleVisible.value = false
  codexBubbleFromHover = false
  void syncPetBubbleWindow()
}

function hideCodexBubbleOnly() {
  if (codexBubbleTimer !== null) {
    window.clearTimeout(codexBubbleTimer)
    codexBubbleTimer = null
  }
  codexBubbleVisible.value = false
  codexBubbleFromHover = false
}

function updateCodexCompletionReminder(status: CodexAppServerStatus) {
  const summary = status.summary
  const unreadCompletedCount =
    summary?.unreadCompletedCount ?? (normalizeCodexBubbleState(status.state) === 'completed' ? 1 : 0)

  if (unreadCompletedCount <= 0 || hasHigherPriorityCodexAttention(status)) {
    clearCodexCompletionReminder()
    return
  }

  codexCompletionCount.value = unreadCompletedCount
  if (isAllCodexWorkCompleted(status) && !petHovered.value) {
    codexCompletionHoldActive.value = true
    void holdCodexCompletionAnimation()
  } else {
    codexCompletionHoldActive.value = false
  }
}

function isAllCodexWorkCompleted(status: CodexAppServerStatus) {
  const summary = status.summary
  if (!summary) {
    return normalizeCodexBubbleState(status.state) === 'completed'
  }

  return (
    normalizeCodexBubbleState(summary.state) === 'completed' &&
    summary.unreadCompletedCount > 0 &&
    summary.activeCount <= 0 &&
    summary.waitingCount <= 0 &&
    summary.unreadFailedCount <= 0
  )
}

function hasHigherPriorityCodexAttention(status: CodexAppServerStatus) {
  const summary = status.summary
  return Boolean(summary && (summary.waitingCount > 0 || summary.unreadFailedCount > 0))
}

function shouldAcknowledgeCodexCompletionStatus(status: CodexAppServerStatus) {
  const summary = status.summary
  if (!summary) {
    return normalizeCodexBubbleState(status.state) === 'completed'
  }

  return (
    summary.unreadCompletedCount > 0 &&
    summary.waitingCount <= 0 &&
    summary.unreadFailedCount <= 0
  )
}

function hasUnreadCodexCompletionReminder() {
  return codexCompletionHoldActive.value || codexCompletionCount.value > 0
}

function stopCodexCompletionHoldAnimation() {
  codexCompletionHoldActive.value = false

  if (petState.value === 'jumping') {
    if (stateTimer !== null) {
      window.clearTimeout(stateTimer)
      stateTimer = null
    }
    protectedUntil.value = 0
  }
}

function clearCodexCompletionReminder(options: { resetAnimation?: boolean } = {}) {
  codexCompletionHoldActive.value = false
  codexCompletionCount.value = 0

  if (options.resetAnimation && petState.value === 'jumping') {
    if (stateTimer !== null) {
      window.clearTimeout(stateTimer)
      stateTimer = null
    }
    protectedUntil.value = 0
  }
}

async function holdCodexCompletionAnimation() {
  if (!codexCompletionHoldActive.value || petHovered.value || pointerDown.value || dragStarted.value) {
    return
  }

  await showState('jumping', 0, 'jumping')
}

function consumeSuppressedCodexAckStatus(status: CodexAppServerStatus) {
  if (!suppressNextCodexAckStatus) {
    return false
  }

  const summary = status.summary
  if (!summary) {
    suppressNextCodexAckStatus = false
    return normalizeCodexBubbleState(status.state) !== 'completed'
  }

  const hasAttention =
    summary.activeCount > 0 ||
    summary.waitingCount > 0 ||
    summary.unreadCompletedCount > 0 ||
    summary.unreadFailedCount > 0
  if (hasAttention) {
    suppressNextCodexAckStatus = false
    return false
  }

  suppressNextCodexAckStatus = false
  return true
}

async function acknowledgeCodexCompletionReminder() {
  suppressNextCodexAckStatus = true
  try {
    await invoke<CodexAppServerStatus>('ack_codex_notifications')
  } catch (err) {
    suppressNextCodexAckStatus = false
    console.error(err)
  }
}

async function showCurrentCodexBubbleOnHover() {
  const requestId = ++codexStatusRequestId
  try {
    const status = await invoke<CodexAppServerStatus>('get_codex_app_server_status')
    if (requestId !== codexStatusRequestId || !petHovered.value) {
      return
    }
    latestCodexStatus.value = status
    updateCodexActivityPetState(status)
    showCodexBubble(status, { respectNotify: false, keepWhileHovered: true })
  } catch (err) {
    console.error(err)
  }
}

function currentPetBubblePayload(): PetBubbleWindowPayload | null {
  if (codexBubbleVisible.value && codexBubbleText.value.trim()) {
    return {
      kind: 'bubble',
      state: codexBubbleState.value,
      message: codexBubbleText.value,
      theme: drawerTheme.value,
    }
  }

  return null
}

async function syncPetBubbleWindow() {
  const payload = currentPetBubblePayload()

  try {
    if (payload) {
      await invoke('show_pet_bubble', { payload })
    } else {
      await invoke('hide_pet_bubble')
    }
  } catch (err) {
    console.error(err)
  }
}

async function hidePetBubbleWindow() {
  try {
    await invoke('hide_pet_bubble')
  } catch (err) {
    console.error(err)
  }
}

async function repositionPetBubbleWindow() {
  if (!codexBubbleVisible.value) {
    return
  }

  try {
    await invoke('reposition_pet_bubble')
  } catch (err) {
    console.error(err)
  }
}

function normalizeCodexBubbleState(state: string): CodexBubbleState {
  switch (state) {
    case 'disconnected':
    case 'starting':
    case 'connected':
    case 'running':
    case 'waiting':
    case 'review':
    case 'completed':
    case 'failed':
      return state
    default:
      return 'connected'
  }
}

function codexBubbleMessage(status: CodexAppServerStatus) {
  switch (status.state) {
    case 'disconnected':
      return 'Codex 未连接'
    case 'starting':
      return '正在连接 Codex'
    case 'connected':
      return 'Codex 已连接'
    case 'running':
      return 'Codex 正在工作'
    case 'waiting':
      return 'Codex 需要你处理'
    case 'review':
      return 'Codex 正在审查'
    case 'completed':
      return 'Codex 工作完成'
    case 'failed':
      return status.error ? `Codex 失败：${status.error}` : 'Codex 工作失败'
    default:
      return ''
  }
}

async function savePosition() {
  try {
    const position = await appWindow.outerPosition()
    await invoke('save_pet_position', { position: { x: position.x, y: position.y } })
    void repositionPetBubbleWindow()
  } catch (err) {
    console.error(err)
  }
}

function isStateProtected() {
  return Date.now() < protectedUntil.value
}

function isCodexActivityState(state: PetAnimationKey = petState.value) {
  return state === 'running' || state === 'review' || state === 'waiting'
}

function updateCodexActivityPetState(status: CodexAppServerStatus) {
  const summary = status.summary
  if (summary) {
    if (summary.waitingCount > 0) {
      codexActivityPetState.value = 'waiting'
    } else if (summary.runningCount > 0) {
      codexActivityPetState.value = 'running'
    } else if (summary.reviewCount > 0) {
      codexActivityPetState.value = 'review'
    } else {
      codexActivityPetState.value = null
    }
    return
  }

  const state = normalizeCodexBubbleState(status.state)
  codexActivityPetState.value = codexBubbleStateToPetActivityState(state)
}

function codexBubbleStateToPetActivityState(state: CodexBubbleState): PetAnimationKey | null {
  switch (state) {
    case 'running':
    case 'review':
    case 'waiting':
      return state
    default:
      return null
  }
}

function clickAnimationFallbackState(): PetAnimationKey {
  return codexActivityPetState.value ?? 'idle'
}

function canUseHoverState() {
  return !isStateProtected() && !isCodexActivityState()
}

function dragStateForDirection(): PetAnimationKey {
  return dragDirection.value === 'left' ? 'draggingLeft' : 'draggingRight'
}

function setDragDirectionFromDelta(deltaX: number) {
  if (deltaX < -DRAG_DIRECTION_THRESHOLD_PX) {
    dragDirection.value = 'left'
  } else if (deltaX > DRAG_DIRECTION_THRESHOLD_PX) {
    dragDirection.value = 'right'
  }

  petState.value = dragStateForDirection()
}

async function rememberDragPosition() {
  try {
    const position = await appWindow.outerPosition()
    lastDragPosition = { x: position.x, y: position.y }
  } catch {
    lastDragPosition = null
  }
}

async function showState(
  state: PetAnimationKey,
  duration = 0,
  fallback: PetAnimationKey = 'idle',
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
  petHovered.value = true
  if (codexCompletionHoldActive.value) {
    stopCodexCompletionHoldAnimation()
  }

  void showCurrentCodexBubbleOnHover()

  if (canUseHoverState()) {
    petState.value = 'hover'
  }
}

function handleMouseLeave() {
  petHovered.value = false
  codexStatusRequestId += 1
  if (codexBubbleFromHover) {
    if (codexBubbleTimer !== null) {
      window.clearTimeout(codexBubbleTimer)
      codexBubbleTimer = null
    }
    codexBubbleVisible.value = false
    codexBubbleFromHover = false
    void syncPetBubbleWindow()
  }

  if (canUseHoverState()) {
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
        dragWatchTimer = null
        lastDragPosition = null
        if (codexCompletionHoldActive.value && !petHovered.value) {
          await holdCodexCompletionAnimation()
        } else {
          petState.value = 'idle'
        }
        await savePosition()
        return
      }

      const position = await appWindow.outerPosition()
      if (lastDragPosition) {
        setDragDirectionFromDelta(position.x - lastDragPosition.x)
      } else {
        petState.value = dragStateForDirection()
      }
      lastDragPosition = { x: position.x, y: position.y }
      void repositionPetBubbleWindow()
      dragWatchTimer = window.setTimeout(tick, 80)
    } catch (err) {
      console.error(err)
      dragStarted.value = false
      protectedUntil.value = 0
      petState.value = 'idle'
      dragWatchTimer = null
      lastDragPosition = null
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
  if (isCodexActivityState()) {
    codexActivityPetState.value = petState.value
  }
  void showState(dragStateForDirection())
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
  setDragDirectionFromDelta(dx)
  await showState(dragStateForDirection())
  protectedUntil.value = Number.MAX_SAFE_INTEGER
  await rememberDragPosition()

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

  schedulePetClickAction()
}

function cancelPointer() {
  pointerDown.value = false
  pointerDownAt.value = 0
  if (!dragStarted.value && !isStateProtected()) {
    petState.value = 'idle'
  }
}

function schedulePetClickAction() {
  const clickContext = currentPointerActionContext(true)
  if (pendingClickTimer !== null) {
    window.clearTimeout(pendingClickTimer)
    pendingClickTimer = null
    void runConfiguredPetAction(petActionBindings.value.petDoubleClick, clickContext)
    return
  }

  pendingClickTimer = window.setTimeout(() => {
    pendingClickTimer = null
    void runConfiguredPetAction(petActionBindings.value.petSingleClick, clickContext)
  }, SINGLE_CLICK_DELAY_MS)
}

function currentPointerActionContext(playClick = false): PetActionContext {
  return {
    x: pointerStart.value.x,
    y: pointerStart.value.y,
    playClick,
  }
}

async function runConfiguredPetAction(action: PetActionBinding, context: PetActionContext = {}) {
  const normalizedAction = normalizePetActionBinding(action, 'smartCodexOrDrawer')

  if (context.playClick) {
    void showState('click', CLICK_ANIMATION_MS, clickAnimationFallbackState())
  }

  switch (normalizedAction) {
    case 'smartCodexOrDrawer':
      await runSmartCodexOrDrawerAction()
      return
    case 'toggleDrawer':
      await toggleDrawerWindow()
      return
    case 'showDrawer':
      await showDrawerWindow()
      return
    case 'petMenu':
      await showPetMenuAt(context)
      return
    case 'petChat':
      await showWindowCommand('show_pet_chat')
      return
    case 'story':
      await showWindowCommand('show_story')
      return
    case 'music':
      await showWindowCommand('show_music_player')
      return
    case 'none':
      return
  }
}

async function runSmartCodexOrDrawerAction() {
  if (
    hasUnreadCodexCompletionReminder() &&
    (await shouldOpenCodexWindowFromCompletion()) &&
    (await openCodexWindowFromCompletion())
  ) {
    return
  }

  await toggleDrawerWindow()
}

async function shouldOpenCodexWindowFromCompletion() {
  const cachedStatus = latestCodexStatus.value
  if (cachedStatus) {
    updateCodexCompletionReminder(cachedStatus)
    return isCodexCompletionReadyToOpen(cachedStatus)
  }

  try {
    const status = await invoke<CodexAppServerStatus>('get_codex_app_server_status')
    latestCodexStatus.value = status
    updateCodexCompletionReminder(status)
    return isCodexCompletionReadyToOpen(status)
  } catch (err) {
    console.error(err)
    return false
  }
}

function isCodexCompletionReadyToOpen(status: CodexAppServerStatus) {
  const summary = status.summary
  if (!summary) {
    return normalizeCodexBubbleState(status.state) === 'completed'
  }

  return (
    normalizeCodexBubbleState(summary.state) === 'completed' &&
    summary.unreadCompletedCount > 0 &&
    summary.activeCount <= 0 &&
    summary.waitingCount <= 0 &&
    summary.unreadFailedCount <= 0
  )
}

async function openCodexWindowFromCompletion() {
  try {
    await invoke('hide_pet_menu')
    await invoke('open_codex_window')
    clearCodexCompletionReminder({ resetAnimation: true })
    await acknowledgeCodexCompletionReminder()
    return true
  } catch (err) {
    console.info('Codex 窗口未打开，改为打开抽屉。', err)
    return false
  }
}

async function toggleDrawerWindow() {
  try {
    await invoke('hide_pet_menu')
    await invoke('toggle_drawer')
  } catch (err) {
    console.error(err)
  }
}

async function showDrawerWindow() {
  try {
    await invoke('hide_pet_menu')
    await invoke('show_drawer')
  } catch (err) {
    console.error(err)
  }
}

async function showWindowCommand(command: 'show_pet_chat' | 'show_story' | 'show_music_player') {
  try {
    await invoke('hide_pet_menu')
    await invoke(command)
  } catch (err) {
    console.error(err)
  }
}

async function showPetMenuAt(context: PetActionContext) {
  try {
    await invoke('show_pet_menu', {
      x: Math.round(context.x ?? pointerStart.value.x),
      y: Math.round(context.y ?? pointerStart.value.y),
    })
  } catch (err) {
    console.error(err)
  }
}

async function openMenu(event: MouseEvent) {
  event.preventDefault()
  await runConfiguredPetAction(petActionBindings.value.petRightClick, {
    x: event.clientX,
    y: event.clientY,
    playClick: false,
  })
}
</script>

<template>
  <main
    class="pet-window"
    :class="[themeClass, windowOpenAnimationClass]"
    @mouseenter="handleMouseEnter"
    @mouseleave="handleMouseLeave"
    @contextmenu="openMenu"
  >
    <button
      class="pet-button"
      type="button"
      @pointerdown="startPointer"
      @pointermove="maybeStartDrag"
      @pointerup="finishPointer"
      @pointercancel="cancelPointer"
      @lostpointercapture="cancelPointer"
      @contextmenu.stop="openMenu"
    >
      <Pet :state="petState" :image-url="currentPetImage" />
    </button>
    <span
      v-if="showCodexCompletionCounter"
      class="codex-completion-counter"
      role="status"
      aria-live="polite"
      :aria-label="`已完成 ${codexCompletionCount} 个 Codex 任务`"
    >
      <span class="codex-completion-counter-mark" aria-hidden="true">✓</span>
      <span>{{ codexCompletionCounterLabel }}</span>
    </span>
  </main>
</template>
