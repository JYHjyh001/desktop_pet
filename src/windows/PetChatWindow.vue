<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { emit as emitEvent, listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type {
  Companion,
  CompanionStatus,
  DrawerTheme,
  FavorabilityLog,
  PetChatMessage,
  PetChatReply,
  PetDrawerConfig,
  PetMemoryMessage,
  PetSkinSummary,
} from '../types/app'
import { getPetSkinPreview } from '../utils/defaultPet'
import {
  twemojiItems,
  tokenizeTwemojiText,
  type TwemojiItem,
  type TwemojiTextToken,
} from '../utils/twemoji'

type ChatMessage = PetChatMessage & {
  id: string
  memoryId?: number
  local?: boolean
  visibleContent?: string
  typing?: boolean
}

type ChatMessagePart = {
  type: 'dialogue' | 'narration'
  text: string
}

type NarrationDelimiter = {
  open: string
  close: string
}

type MessageContextMenu = {
  messageId: number
  x: number
  y: number
}

type CharacterContextMenu = {
  x: number
  y: number
}

type MessageDeleteRequest = {
  messageIds: number[]
  summary: string
}

type WechatForwardRole = 'user' | 'assistant'

const WECHAT_INTEGRATION_ENABLED: boolean = false
const chatWindow = getCurrentWindow()
const messages = ref<ChatMessage[]>([
  {
    id: 'welcome',
    role: 'assistant',
    content: '我在，想聊点什么？',
    createdAt: nowSeconds(),
    local: true,
  },
])
const inputRef = ref<HTMLTextAreaElement | null>(null)
const inputText = ref('')
const emojiPickerVisible = ref(false)
const chatSettingsVisible = ref(false)
const chatSettingsSaving = ref(false)
const chatSettingsError = ref('')
const chatSettingsDraft = ref({
  typewriterEnabled: true,
  narrationEnabled: false,
})
const sending = ref(false)
const errorMessage = ref('')
const config = ref<PetDrawerConfig | null>(null)
const currentCompanion = ref<Companion | null>(null)
const currentPetSkin = ref<PetSkinSummary | null>(null)
const companionStatus = ref<CompanionStatus | null>(null)
const companionStatusLoading = ref(false)
const companionStatusError = ref('')
const characterContextMenu = ref<CharacterContextMenu | null>(null)
const characterStatusVisible = ref(false)
const favorabilityLogsVisible = ref(false)
const favorabilityLogs = ref<FavorabilityLog[]>([])
const favorabilityLogsLoading = ref(false)
const manualFavorabilityVisible = ref(false)
const manualFavorabilityDraft = ref(0)
const messageListRef = ref<HTMLElement | null>(null)
const messageContextMenu = ref<MessageContextMenu | null>(null)
const selectionMode = ref(false)
const selectedMessageIds = ref<number[]>([])
const deleteRequest = ref<MessageDeleteRequest | null>(null)
const deletingMessages = ref(false)
let unlistenChatOpened: (() => void) | null = null
let unlistenThemeChanged: (() => void) | null = null
let unlistenChatDisplayChanged: (() => void) | null = null
let unlistenChatNarrationChanged: (() => void) | null = null
let unlistenCompanionChanged: (() => void) | null = null
let typewriterTimer: number | null = null
let activeTypewriterMessageId: string | null = null

const typewriterBaseDelayMs = 18
const typewriterMaxSteps = 260
const narrationDelimiters: NarrationDelimiter[] = [
  { open: '(', close: ')' },
  { open: '（', close: '）' },
  { open: '[', close: ']' },
  { open: '［', close: '］' },
  { open: '【', close: '】' },
  { open: '〔', close: '〕' },
  { open: '*', close: '*' },
]

const aiEnabled = computed(() => Boolean(config.value?.ai?.enabled))
const typewriterEnabled = computed(() => config.value?.drawer.chatTypewriterEnabled !== false)
const narrationEnabled = computed(() => config.value?.drawer.chatNarrationEnabled === true)
const drawerTheme = computed<DrawerTheme>(() =>
  config.value?.drawer.theme === 'animal-island' ? 'animal-island' : 'light',
)
const persistedMessages = computed(() =>
  messages.value.filter((message) => typeof message.memoryId === 'number'),
)
const selectedMessageCount = computed(() => selectedMessageIds.value.length)
const aiSummary = computed(() => {
  const ai = config.value?.ai
  if (!ai?.enabled) {
    return '未启用 AI 接口'
  }

  const memoryStatus = ai.memoryEnabled === false ? '记忆关闭' : '记忆开启'
  const model = currentCompanion.value?.model
    ? `${currentCompanion.value.model}（伴侣覆盖）`
    : ai.model || '未设置模型'
  return `${providerLabel(ai.provider)} / ${model} / ${memoryStatus}`
})
const companionAvatarSrc = computed(() => currentCompanion.value?.avatar || getPetSkinPreview(currentPetSkin.value))
const favorabilitySummary = computed(() => {
  const status = companionStatus.value
  if (!status) {
    return '好感度状态读取中'
  }
  if (!status.favorabilityEnabled) {
    return '好感度系统关闭'
  }
  return `${status.relationshipStageName} / 好感度 ${status.favorability}`
})
const favorabilityStateLabel = computed(() => {
  const status = companionStatus.value
  if (!status) {
    return '读取中'
  }
  return status.favorabilityEnabled ? '已开启' : '未开启'
})
const favorabilityToggleLabel = computed(() =>
  companionStatus.value?.favorabilityEnabled ? '关闭好感度系统' : '开启好感度系统',
)
const chatInputPlaceholder = computed(() => {
  if (selectionMode.value) {
    return '请先完成或取消多选'
  }
  if (!aiEnabled.value) {
    return '先在设置中启用 AI 接口'
  }
  return '输入消息，Enter 发送'
})

onMounted(async () => {
  await Promise.all([loadConfig(), loadConversation(), loadCurrentPetSkin(), loadCompanionStatus()])
  unlistenChatOpened = await listen('pet-chat-opened', () => {
    void Promise.all([loadConfig(), loadConversation(), loadCurrentPetSkin(), loadCompanionStatus()])
  })
  unlistenThemeChanged = await listen<string>('ui-theme-changed', (event) => {
    if (config.value) {
      config.value.drawer.theme = event.payload
    }
  })
  unlistenChatDisplayChanged = await listen<boolean>('ui-chat-display-changed', (event) => {
    if (config.value) {
      config.value.drawer.chatTypewriterEnabled = event.payload
    }
    if (!event.payload) {
      finishTypewriter()
    }
  })
  unlistenChatNarrationChanged = await listen<boolean>('ui-chat-narration-changed', (event) => {
    if (config.value) {
      config.value.drawer.chatNarrationEnabled = event.payload
    }
  })
  unlistenCompanionChanged = await listen('companion-changed', () => {
    finishTypewriter()
    void Promise.all([loadConfig(), loadConversation(), loadCurrentPetSkin(), loadCompanionStatus()])
  })
})

onBeforeUnmount(() => {
  unlistenChatOpened?.()
  unlistenThemeChanged?.()
  unlistenChatDisplayChanged?.()
  unlistenChatNarrationChanged?.()
  unlistenCompanionChanged?.()
  clearTypewriterTimer()
})

watch(
  messages,
  async () => {
    await nextTick()
    scrollToBottom()
  },
  { deep: true },
)

async function loadConfig() {
  try {
    config.value = await invoke<PetDrawerConfig>('get_config')
  } catch (err) {
    errorMessage.value = String(err)
  }
}

async function loadCurrentPetSkin() {
  try {
    currentPetSkin.value = await invoke<PetSkinSummary>('get_current_pet_skin')
  } catch {
    currentPetSkin.value = null
  }
}

async function loadCompanionStatus() {
  companionStatusLoading.value = true
  companionStatusError.value = ''
  try {
    companionStatus.value = await invoke<CompanionStatus>('get_current_companion_status')
    manualFavorabilityDraft.value = companionStatus.value.favorability
  } catch (err) {
    companionStatusError.value = String(err)
  } finally {
    companionStatusLoading.value = false
  }
}

function welcomeMessage(companion?: Companion | null): ChatMessage {
  const firstMessage = companion?.firstMessage?.trim()
  return {
    id: 'welcome',
    role: 'assistant',
    content: firstMessage || (companion ? `我是${companion.name}，想聊点什么？` : '我在，想聊点什么？'),
    createdAt: nowSeconds(),
    local: true,
  }
}

async function loadConversation() {
  try {
    const [companion, history] = await Promise.all([
      invoke<Companion>('get_current_companion'),
      invoke<PetMemoryMessage[]>('get_companion_messages'),
    ])
    currentCompanion.value = companion
    messages.value =
      history.length > 0
        ? history.map((message) => ({
            id: `history-${message.id}`,
            memoryId: message.id,
            role: message.role,
            content: message.content,
            createdAt: message.createdAt,
          }))
        : [welcomeMessage(companion)]
    errorMessage.value = ''
  } catch (err) {
    errorMessage.value = String(err)
  }
}

function providerLabel(provider?: string) {
  const labels: Record<string, string> = {
    openai: 'OpenAI 兼容',
    deepseek: 'DeepSeek',
    anthropic: 'Anthropic',
    gemini: 'Gemini',
    ollama: 'Ollama 本地',
    custom: '自定义',
  }

  return labels[(provider ?? '').toLowerCase()] ?? '自定义'
}

function scrollToBottom() {
  const element = messageListRef.value
  if (element) {
    element.scrollTop = element.scrollHeight
  }
}

async function startDrag() {
  await chatWindow.startDragging()
}

async function hideChat() {
  await invoke('hide_pet_chat')
}

function stopHeaderDrag(event: PointerEvent) {
  event.stopPropagation()
}

async function openDrawer() {
  await invoke('show_drawer')
}

function canManageMessage(message: ChatMessage) {
  return typeof message.memoryId === 'number' && !message.local
}

function closeMessageContextMenu() {
  messageContextMenu.value = null
}

function closeCharacterContextMenu() {
  characterContextMenu.value = null
}

function closeFloatingPanels() {
  closeMessageContextMenu()
  closeCharacterContextMenu()
  emojiPickerVisible.value = false
}

function syncChatSettingsDraft() {
  chatSettingsDraft.value = {
    typewriterEnabled: config.value?.drawer.chatTypewriterEnabled !== false,
    narrationEnabled: config.value?.drawer.chatNarrationEnabled === true,
  }
}

function openChatSettings() {
  finishTypewriter()
  closeFloatingPanels()
  syncChatSettingsDraft()
  chatSettingsError.value = ''
  chatSettingsVisible.value = true
}

function closeChatSettings() {
  if (!chatSettingsSaving.value) {
    chatSettingsVisible.value = false
  }
}

async function saveChatSettings() {
  chatSettingsSaving.value = true
  chatSettingsError.value = ''

  try {
    const updatedConfig = await invoke<PetDrawerConfig>('save_chat_display_preferences', {
      chatTypewriterEnabled: chatSettingsDraft.value.typewriterEnabled,
      chatNarrationEnabled: chatSettingsDraft.value.narrationEnabled,
    })
    config.value = updatedConfig
    await Promise.all([
      emitEvent('ui-chat-display-changed', updatedConfig.drawer.chatTypewriterEnabled ?? true),
      emitEvent('ui-chat-narration-changed', updatedConfig.drawer.chatNarrationEnabled ?? false),
    ])
    chatSettingsVisible.value = false
  } catch (err) {
    chatSettingsError.value = String(err)
  } finally {
    chatSettingsSaving.value = false
  }
}

function openMessageContextMenu(event: MouseEvent, message: ChatMessage) {
  if (!canManageMessage(message) || sending.value || deletingMessages.value) {
    return
  }

  finishTypewriter()
  closeCharacterContextMenu()
  const messageId = message.memoryId as number
  const menuWidth = 136
  const menuHeight = 84
  messageContextMenu.value = {
    messageId,
    x: Math.max(8, Math.min(event.clientX, window.innerWidth - menuWidth - 8)),
    y: Math.max(8, Math.min(event.clientY, window.innerHeight - menuHeight - 8)),
  }
}

async function openCharacterContextMenu(event: MouseEvent) {
  finishTypewriter()
  closeMessageContextMenu()
  await loadCompanionStatus()
  const menuWidth = 190
  const menuHeight = 190
  characterContextMenu.value = {
    x: Math.max(8, Math.min(event.clientX, window.innerWidth - menuWidth - 8)),
    y: Math.max(8, Math.min(event.clientY, window.innerHeight - menuHeight - 8)),
  }
}

async function openCharacterStatus() {
  closeCharacterContextMenu()
  manualFavorabilityVisible.value = false
  await loadCompanionStatus()
  characterStatusVisible.value = true
}

function closeCharacterStatus() {
  characterStatusVisible.value = false
  manualFavorabilityVisible.value = false
}

async function openFavorabilityLogs() {
  closeCharacterContextMenu()
  favorabilityLogsVisible.value = true
  await loadFavorabilityLogs()
}

function closeFavorabilityLogs() {
  favorabilityLogsVisible.value = false
}

async function loadFavorabilityLogs() {
  favorabilityLogsLoading.value = true
  companionStatusError.value = ''
  try {
    favorabilityLogs.value = await invoke<FavorabilityLog[]>('list_current_companion_favorability_logs')
  } catch (err) {
    companionStatusError.value = String(err)
  } finally {
    favorabilityLogsLoading.value = false
  }
}

function showManualFavorability() {
  closeCharacterContextMenu()
  manualFavorabilityDraft.value = companionStatus.value?.favorability ?? 0
  characterStatusVisible.value = true
  manualFavorabilityVisible.value = true
}

async function setManualFavorability() {
  companionStatusLoading.value = true
  companionStatusError.value = ''
  try {
    companionStatus.value = await invoke<CompanionStatus>('set_current_companion_favorability', {
      value: safeInteger(manualFavorabilityDraft.value, 0),
    })
    manualFavorabilityDraft.value = companionStatus.value.favorability
    manualFavorabilityVisible.value = false
    await loadFavorabilityLogs()
  } catch (err) {
    companionStatusError.value = String(err)
  } finally {
    companionStatusLoading.value = false
  }
}

async function resetFavorability() {
  closeCharacterContextMenu()
  companionStatusLoading.value = true
  companionStatusError.value = ''
  try {
    companionStatus.value = await invoke<CompanionStatus>('reset_current_companion_favorability')
    manualFavorabilityDraft.value = 0
    await loadFavorabilityLogs()
  } catch (err) {
    companionStatusError.value = String(err)
  } finally {
    companionStatusLoading.value = false
  }
}

async function toggleFavorabilityEnabled() {
  closeCharacterContextMenu()
  companionStatusLoading.value = true
  companionStatusError.value = ''
  try {
    const latestStatus = await invoke<CompanionStatus>('get_current_companion_status')
    companionStatus.value = latestStatus
    companionStatus.value = await invoke<CompanionStatus>(
      'set_current_companion_favorability_enabled',
      {
        enabled: !latestStatus.favorabilityEnabled,
      },
    )
    manualFavorabilityDraft.value = companionStatus.value.favorability
    await loadFavorabilityLogs()
  } catch (err) {
    companionStatusError.value = String(err)
  } finally {
    companionStatusLoading.value = false
  }
}

function isMessageSelected(message: ChatMessage) {
  return typeof message.memoryId === 'number' && selectedMessageIds.value.includes(message.memoryId)
}

function enterSelectionMode(messageId?: number) {
  closeMessageContextMenu()
  finishTypewriter()
  selectionMode.value = true
  selectedMessageIds.value = messageId ? [messageId] : []
}

function leaveSelectionMode() {
  selectionMode.value = false
  selectedMessageIds.value = []
  closeMessageContextMenu()
}

function toggleMessageSelection(message: ChatMessage) {
  if (!selectionMode.value || !canManageMessage(message)) {
    return
  }

  const messageId = message.memoryId as number
  selectedMessageIds.value = selectedMessageIds.value.includes(messageId)
    ? selectedMessageIds.value.filter((id) => id !== messageId)
    : [...selectedMessageIds.value, messageId]
}

function selectAllMessages() {
  selectedMessageIds.value = persistedMessages.value
    .map((message) => message.memoryId)
    .filter((messageId): messageId is number => typeof messageId === 'number')
}

function requestDeleteMessages(messageIds: number[]) {
  const uniqueIds = [...new Set(messageIds)]
  if (uniqueIds.length === 0) {
    return
  }

  closeMessageContextMenu()
  deleteRequest.value = {
    messageIds: uniqueIds,
    summary:
      uniqueIds.length === 1
        ? '确定删除这条聊天消息吗？删除后它不会再参与后续对话。'
        : `确定删除已选择的 ${uniqueIds.length} 条聊天消息吗？删除后它们不会再参与后续对话。`,
  }
}

function requestContextMessageDelete() {
  const messageId = messageContextMenu.value?.messageId
  if (messageId) {
    requestDeleteMessages([messageId])
  }
}

function requestSelectedMessagesDelete() {
  requestDeleteMessages(selectedMessageIds.value)
}

function cancelDeleteMessages() {
  if (!deletingMessages.value) {
    deleteRequest.value = null
  }
}

async function confirmDeleteMessages() {
  const request = deleteRequest.value
  if (!request || deletingMessages.value) {
    return
  }

  deletingMessages.value = true
  finishTypewriter()
  try {
    await invoke<number>('delete_companion_messages', { messageIds: request.messageIds })
    deleteRequest.value = null
    leaveSelectionMode()
    await loadConversation()
  } catch (err) {
    errorMessage.value = String(err)
  } finally {
    deletingMessages.value = false
  }
}

function requestMessages() {
  return messages.value
    .filter((message) => !message.local)
    .map((message) => ({
      role: message.role,
      content: requestMessageContent(message),
      createdAt: message.createdAt,
      timeContext: formatMessageContextTime(message.createdAt),
    }))
    .filter((message) => message.content.length > 0)
}

function safeInteger(value: number, fallback: number) {
  return Number.isFinite(value) ? Math.round(value) : fallback
}

function displayedContent(message: ChatMessage) {
  return stripInternalTimeLabels(message.visibleContent ?? message.content)
}

function renderedTextTokens(text: string): TwemojiTextToken[] {
  return tokenizeTwemojiText(text)
}

function stripInternalTimeLabels(content: string) {
  return content.replace(/\[(?:用户发送时间|宠物生成时间)[:：][^\]]*\]\s*/g, '')
}

function requestMessageContent(message: ChatMessage) {
  const content = stripInternalTimeLabels(message.content)
  if (narrationEnabled.value) {
    return content.trim()
  }

  return dialogueOnlyContent(content)
}

function renderedMessageParts(message: ChatMessage) {
  const parts = splitMessageContent(displayedContent(message))
  return narrationEnabled.value ? parts : parts.filter((part) => part.type === 'dialogue')
}

function splitMessageContent(content: string): ChatMessagePart[] {
  const parts: ChatMessagePart[] = []
  const characters = Array.from(content)
  let dialogue = ''
  let narration = ''
  const narrationCloseStack: string[] = []

  const flushDialogue = () => {
    const text = dialogue.trim()
    if (text) {
      parts.push({ type: 'dialogue', text })
    }
    dialogue = ''
  }

  const flushNarration = () => {
    const text = narration.trim()
    if (text) {
      parts.push({ type: 'narration', text })
    }
    narration = ''
  }

  for (let index = 0; index < characters.length; index += 1) {
    const character = characters[index]
    const expectedClose = narrationCloseStack[narrationCloseStack.length - 1]

    if (expectedClose && character === expectedClose) {
      narrationCloseStack.pop()
      if (narrationCloseStack.length === 0) {
        flushNarration()
      } else {
        narration += character
      }
      continue
    }

    const opening = findNarrationOpening(characters, index, narrationCloseStack.length > 0)
    if (opening) {
      if (narrationCloseStack.length === 0) {
        flushDialogue()
      } else {
        narration += character
      }
      narrationCloseStack.push(opening.close)
      continue
    }

    if (narrationCloseStack.length > 0) {
      narration += character
    } else {
      dialogue += character
    }
  }

  if (narrationCloseStack.length > 0) {
    dialogue += narrationDelimiters.find((delimiter) => delimiter.close === narrationCloseStack[0])?.open ?? ''
    dialogue += narration
    narration = ''
  }
  flushDialogue()

  return parts
}

function findNarrationOpening(
  characters: string[],
  index: number,
  insideNarration: boolean,
): NarrationDelimiter | null {
  if (insideNarration) {
    return null
  }

  const delimiter = narrationDelimiters.find((item) => characters[index] === item.open)
  if (!delimiter || !hasClosingNarrationDelimiter(characters, index, delimiter)) {
    return null
  }

  return delimiter
}

function hasClosingNarrationDelimiter(
  characters: string[],
  index: number,
  delimiter: NarrationDelimiter,
) {
  if (
    delimiter.open === '*' &&
    (characters[index - 1] === '*' || characters[index + 1] === '*' || !characters[index + 1]?.trim())
  ) {
    return false
  }

  return characters.slice(index + 1).some((character, offset) => {
    if (character !== delimiter.close) {
      return false
    }

    if (
      delimiter.open === '*' &&
      (characters[index + offset] === '*' || !characters[index + offset]?.trim())
    ) {
      return false
    }

    return characters.slice(index + 1, index + offset + 1).some((item) => item.trim())
  })
}

function dialogueOnlyContent(content: string) {
  return splitMessageContent(content)
    .filter((part) => part.type === 'dialogue')
    .map((part) => part.text)
    .join('\n')
    .trim()
}

function nowSeconds() {
  return Math.floor(Date.now() / 1000).toString()
}

function shouldForwardWechatClawbot(role: WechatForwardRole) {
  if (!WECHAT_INTEGRATION_ENABLED) {
    return false
  }

  const settings = config.value?.wechatClawbot
  if (!settings?.enabled || !settings.target?.trim()) {
    return false
  }

  return role === 'user'
    ? Boolean(settings.forwardUserMessages)
    : settings.forwardAssistantMessages !== false
}

function formatWechatClawbotMessage(role: WechatForwardRole, content: string) {
  const roleName = role === 'user' ? '用户' : currentCompanion.value?.name || 'PetDrawer'
  return `[PetDrawer/${roleName}]\n${content.trim()}`
}

async function forwardWechatClawbotMessage(role: WechatForwardRole, content: string) {
  const message = content.trim()
  if (!message || !shouldForwardWechatClawbot(role)) {
    return
  }

  try {
    await invoke('send_wechat_clawbot_message', {
      message: formatWechatClawbotMessage(role, message),
    })
  } catch (err) {
    console.warn('微信 ClawBot 同步失败', err)
  }
}

function timestampDate(value?: string | null) {
  if (!value) {
    return null
  }

  const trimmed = value.trim()
  const numericValue = Number(trimmed)
  const date = Number.isFinite(numericValue)
    ? new Date(numericValue > 9999999999 ? numericValue : numericValue * 1000)
    : new Date(trimmed)

  return Number.isNaN(date.getTime()) ? null : date
}

function formatMessageClock(value?: string | null) {
  const date = timestampDate(value)
  if (!date) {
    return ''
  }

  const now = new Date()
  const sameDay =
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate()
  const options: Intl.DateTimeFormatOptions = sameDay
    ? { hour: '2-digit', minute: '2-digit', hour12: false }
    : { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', hour12: false }

  return date.toLocaleString('zh-CN', options)
}

function formatMessageContextTime(value?: string | null) {
  const date = timestampDate(value)
  if (!date) {
    return ''
  }

  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  })
}

function messageTimeText(message: ChatMessage) {
  const clock = formatMessageClock(message.createdAt)
  if (!clock) {
    return ''
  }

  return clock
}

function messageDateTimeAttribute(message: ChatMessage) {
  return timestampDate(message.createdAt)?.toISOString()
}

function logSourceLabel(source: string) {
  const labels: Record<string, string> = {
    dialogue: '对话',
    manual: '手动设置',
    reset: '重置',
    system: '系统',
  }
  return labels[source] ?? source
}

function relationshipStageLabel(stage: string) {
  const labels: Record<string, string> = {
    hostile: '敌对',
    dislike: '讨厌',
    guarded: '戒备',
    neutral: '初始',
    acquaintance: '初识',
    familiar: '熟悉',
    friend: '朋友',
    close: '亲近',
    dependent: '依赖',
    bond: '羁绊',
  }
  return labels[stage] ?? stage
}

function signedValue(value: number) {
  return value > 0 ? `+${value}` : `${value}`
}

function statusTimeText(value?: string | null) {
  return value ? formatMessageContextTime(value) : '暂无'
}

function isTimedMessagePart(message: ChatMessage, partIndex: number) {
  return Boolean(messageTimeText(message)) && partIndex === renderedMessageParts(message).length - 1
}

function isBubbleMessagePart(message: ChatMessage, part: ChatMessagePart) {
  return message.role === 'assistant' && part.type === 'dialogue'
}

function isTypingMessagePart(message: ChatMessage, partIndex: number) {
  return Boolean(message.typing) && partIndex === renderedMessageParts(message).length - 1
}

function clearTypewriterTimer() {
  if (typewriterTimer !== null) {
    window.clearTimeout(typewriterTimer)
    typewriterTimer = null
  }
}

function finishTypewriter(messageId = activeTypewriterMessageId) {
  if (!messageId || messageId !== activeTypewriterMessageId) {
    return
  }

  const message = messages.value.find((item) => item.id === messageId)
  if (message) {
    message.visibleContent = message.content
    message.typing = false
  }

  clearTypewriterTimer()
  activeTypewriterMessageId = null
}

function typewriterDelay(character: string) {
  if (['。', '！', '？', '!', '?', '\n'].includes(character)) {
    return typewriterBaseDelayMs * 5
  }

  if (['，', '、', ',', '：', ':', '；', ';'].includes(character)) {
    return typewriterBaseDelayMs * 2
  }

  return typewriterBaseDelayMs
}

function startTypewriter(messageId: string) {
  finishTypewriter()
  const message = messages.value.find((item) => item.id === messageId)
  if (!message) {
    return
  }

  const characters = Array.from(message.content)
  if (
    characters.length === 0 ||
    !typewriterEnabled.value ||
    window.matchMedia('(prefers-reduced-motion: reduce)').matches
  ) {
    return
  }

  message.visibleContent = ''
  message.typing = true
  activeTypewriterMessageId = message.id
  let index = 0
  const charactersPerStep = Math.max(1, Math.ceil(characters.length / typewriterMaxSteps))

  const revealNext = () => {
    if (!message.typing || activeTypewriterMessageId !== message.id) {
      return
    }

    const nextCharacters = characters.slice(index, index + charactersPerStep).join('')
    message.visibleContent = `${message.visibleContent ?? ''}${nextCharacters}`
    index += charactersPerStep
    if (index >= characters.length) {
      message.typing = false
      activeTypewriterMessageId = null
      typewriterTimer = null
      return
    }

    typewriterTimer = window.setTimeout(
      revealNext,
      typewriterDelay(nextCharacters[nextCharacters.length - 1] ?? ''),
    )
  }

  revealNext()
}

async function sendMessage() {
  const content = inputText.value.trim()
  if (!content || sending.value) {
    return
  }
  const outgoingContent = narrationEnabled.value ? content : dialogueOnlyContent(content)
  if (!outgoingContent) {
    errorMessage.value = '旁白功能关闭时，旁白标记中的内容不会发送。请输入旁白外的对话内容，或在设置中开启旁白功能。'
    return
  }

  finishTypewriter()
  errorMessage.value = ''
  inputText.value = ''
  emojiPickerVisible.value = false
  messages.value.push({
    id: `${Date.now()}-user`,
    role: 'user',
    content: stripInternalTimeLabels(outgoingContent),
    createdAt: nowSeconds(),
  })
  void forwardWechatClawbotMessage('user', stripInternalTimeLabels(outgoingContent))
  const previousMessageIds = new Set(
    messages.value
      .map((message) => message.memoryId)
      .filter((messageId): messageId is number => typeof messageId === 'number'),
  )
  sending.value = true

  try {
    const reply = await invoke<PetChatReply>('send_pet_chat_message', {
      messages: requestMessages(),
    })
    await loadConversation()
    const assistantMessage =
      [...messages.value]
        .reverse()
        .find(
          (message) =>
            message.role === 'assistant' &&
            typeof message.memoryId === 'number' &&
            !previousMessageIds.has(message.memoryId),
        ) ??
      ({
        id: `${Date.now()}-assistant`,
        role: 'assistant',
        content: stripInternalTimeLabels(reply.message),
        createdAt: nowSeconds(),
        local: true,
      } satisfies ChatMessage)
    if (assistantMessage.local) {
      messages.value.push(assistantMessage)
    }
    startTypewriter(assistantMessage.id)
    void forwardWechatClawbotMessage('assistant', reply.message)
    if (reply.favorabilityChange?.status) {
      companionStatus.value = reply.favorabilityChange.status
      manualFavorabilityDraft.value = reply.favorabilityChange.status.favorability
    }
    if (reply.memoryWarning) {
      errorMessage.value = `本次回复已生成，但部分本机状态未能保存：${reply.memoryWarning}`
    }
  } catch (err) {
    await loadConversation()
    errorMessage.value = String(err)
  } finally {
    sending.value = false
  }
}

function handleInputKeydown(event: KeyboardEvent) {
  if (event.key !== 'Enter' || event.shiftKey) {
    return
  }

  event.preventDefault()
  void sendMessage()
}

function toggleEmojiPicker() {
  closeMessageContextMenu()
  closeCharacterContextMenu()
  emojiPickerVisible.value = !emojiPickerVisible.value
}

async function insertTwemoji(item: TwemojiItem) {
  const input = inputRef.value
  const insertText = item.emoji

  if (!input) {
    inputText.value += insertText
    return
  }

  const start = input.selectionStart
  const end = input.selectionEnd
  inputText.value = `${inputText.value.slice(0, start)}${insertText}${inputText.value.slice(end)}`

  await nextTick()
  input.focus()
  const cursor = start + insertText.length
  input.setSelectionRange(cursor, cursor)
}
</script>

<template>
  <main class="pet-chat-window" :class="`theme-${drawerTheme}`" @click="closeFloatingPanels">
    <header class="pet-chat-header" @pointerdown="startDrag">
      <div>
        <h1>和 {{ currentCompanion?.name || '宠物' }} 对话</h1>
        <p>{{ aiSummary }}</p>
      </div>
      <div class="pet-chat-header-actions" @pointerdown="stopHeaderDrag">
        <button
          v-if="persistedMessages.length > 0 && !selectionMode"
          class="pet-chat-manage-button"
          type="button"
          title="批量选择消息"
          @click.stop="enterSelectionMode()"
        >
          多选
        </button>
        <button type="button" title="对话设置" @click.stop="openChatSettings">⚙</button>
        <button type="button" title="关闭对话" @click.stop="hideChat">×</button>
      </div>
    </header>

    <section v-if="!aiEnabled" class="pet-chat-notice">
      <strong>需要先配置 AI 接口</strong>
      <span>打开抽屉设置，在“AI 接口”中启用并填写服务商、模型和 API Key。</span>
      <div>
        <button type="button" @click="openDrawer">打开抽屉</button>
        <button type="button" @click="loadConfig">刷新配置</button>
      </div>
    </section>

    <section v-if="selectionMode" class="pet-chat-selection-toolbar" @click.stop>
      <span>已选择 {{ selectedMessageCount }} 条</span>
      <button type="button" @click="selectAllMessages">全选</button>
      <button
        class="danger-button"
        type="button"
        :disabled="selectedMessageCount === 0 || deletingMessages"
        @click="requestSelectedMessagesDelete"
      >
        删除
      </button>
      <button type="button" @click="leaveSelectionMode">取消</button>
    </section>

    <section ref="messageListRef" class="pet-chat-messages" @scroll.passive="closeMessageContextMenu">
      <template v-for="message in messages" :key="message.id">
        <article
          v-if="renderedMessageParts(message).length > 0"
          class="pet-chat-message"
          :class="[
            message.role,
            {
              selecting: selectionMode && canManageMessage(message),
              selected: isMessageSelected(message),
            },
          ]"
          @click="toggleMessageSelection(message)"
          @contextmenu.prevent="openMessageContextMenu($event, message)"
        >
          <input
            v-if="selectionMode && canManageMessage(message)"
            class="pet-chat-message-selector"
            type="checkbox"
            :checked="isMessageSelected(message)"
            aria-label="选择聊天消息"
            @click.stop
            @change="toggleMessageSelection(message)"
          />
          <div class="pet-chat-message-row">
            <button
              v-if="message.role === 'assistant'"
              class="pet-chat-avatar-button"
              type="button"
              :title="`查看 ${currentCompanion?.name || '宠物'} 状态`"
              @click.stop="openCharacterStatus"
              @contextmenu.stop.prevent="openCharacterContextMenu"
            >
              <img :src="companionAvatarSrc" :alt="currentCompanion?.name || '宠物'" draggable="false" />
            </button>
            <div class="pet-chat-message-content">
              <span class="pet-chat-message-author">
                {{ message.role === 'user' ? '你' : currentCompanion?.name || '宠物' }}
              </span>
              <template
                v-for="(part, partIndex) in renderedMessageParts(message)"
                :key="`${message.id}-${partIndex}`"
              >
                <p
                  v-if="part.type === 'dialogue'"
                  :class="{ 'pet-chat-message-bubble': isBubbleMessagePart(message, part) }"
                >
                  <span
                    class="pet-chat-message-text"
                    :class="{ typing: isTypingMessagePart(message, partIndex) }"
                  >
                    <template
                      v-for="(token, tokenIndex) in renderedTextTokens(part.text)"
                      :key="`${message.id}-${partIndex}-${tokenIndex}`"
                    >
                      <img
                        v-if="token.type === 'emoji'"
                        class="twemoji-inline"
                        :src="token.src"
                        :alt="token.text"
                        :title="token.label"
                        draggable="false"
                      />
                      <span v-else>{{ token.text }}</span>
                    </template>
                  </span>
                  <time
                    v-if="isTimedMessagePart(message, partIndex)"
                    class="pet-chat-message-time"
                    :datetime="messageDateTimeAttribute(message)"
                    :title="formatMessageContextTime(message.createdAt)"
                  >
                    {{ messageTimeText(message) }}
                  </time>
                </p>
                <aside v-else class="pet-chat-message-narration">
                  <span
                    class="pet-chat-message-narration-text"
                    :class="{ typing: isTypingMessagePart(message, partIndex) }"
                  >
                    <template
                      v-for="(token, tokenIndex) in renderedTextTokens(part.text)"
                      :key="`${message.id}-${partIndex}-narration-${tokenIndex}`"
                    >
                      <img
                        v-if="token.type === 'emoji'"
                        class="twemoji-inline"
                        :src="token.src"
                        :alt="token.text"
                        :title="token.label"
                        draggable="false"
                      />
                      <span v-else>{{ token.text }}</span>
                    </template>
                  </span>
                  <time
                    v-if="isTimedMessagePart(message, partIndex)"
                    class="pet-chat-message-time"
                    :datetime="messageDateTimeAttribute(message)"
                    :title="formatMessageContextTime(message.createdAt)"
                  >
                    {{ messageTimeText(message) }}
                  </time>
                </aside>
              </template>
            </div>
          </div>
        </article>
      </template>
      <article v-if="sending" class="pet-chat-message assistant">
        <div class="pet-chat-message-row">
          <button
            class="pet-chat-avatar-button"
            type="button"
            :title="`查看 ${currentCompanion?.name || '宠物'} 状态`"
            @click.stop="openCharacterStatus"
            @contextmenu.stop.prevent="openCharacterContextMenu"
          >
            <img :src="companionAvatarSrc" :alt="currentCompanion?.name || '宠物'" draggable="false" />
          </button>
          <div class="pet-chat-message-content">
            <span class="pet-chat-message-author">{{ currentCompanion?.name || '宠物' }}</span>
            <p>正在想...</p>
          </div>
        </div>
      </article>
    </section>

    <p v-if="errorMessage" class="pet-chat-error">{{ errorMessage }}</p>

    <form class="pet-chat-input" @submit.prevent="sendMessage" @click.stop>
      <textarea
        ref="inputRef"
        v-model="inputText"
        :disabled="sending || !aiEnabled || selectionMode"
        rows="2"
        :placeholder="chatInputPlaceholder"
        @keydown="handleInputKeydown"
      />
      <div class="pet-chat-input-actions">
        <button
          class="pet-chat-emoji-toggle"
          type="button"
          :disabled="sending || !aiEnabled || selectionMode"
          :aria-expanded="emojiPickerVisible"
          title="添加表情"
          @click="toggleEmojiPicker"
        >
          ☺
        </button>
        <button
          type="submit"
          :disabled="sending || !aiEnabled || selectionMode || inputText.trim().length === 0"
        >
          发送
        </button>
      </div>
      <div v-if="emojiPickerVisible" class="pet-chat-emoji-panel" @click.stop>
        <button
          v-for="item in twemojiItems"
          :key="item.code"
          type="button"
          :title="item.label"
          @click="insertTwemoji(item)"
        >
          <img class="twemoji-picker-icon" :src="item.src" :alt="item.emoji" draggable="false" />
        </button>
      </div>
    </form>

    <menu
      v-if="messageContextMenu"
      class="pet-chat-context-menu"
      :style="{ left: `${messageContextMenu.x}px`, top: `${messageContextMenu.y}px` }"
      @click.stop
    >
      <button type="button" @click="requestContextMessageDelete">删除</button>
      <button type="button" @click="enterSelectionMode(messageContextMenu.messageId)">多选</button>
    </menu>

    <menu
      v-if="characterContextMenu"
      class="pet-chat-context-menu pet-chat-character-menu"
      :style="{ left: `${characterContextMenu.x}px`, top: `${characterContextMenu.y}px` }"
      @click.stop
    >
      <li class="pet-chat-character-menu-status">当前：{{ favorabilityStateLabel }}</li>
      <button type="button" @click="openCharacterStatus">查看当前状态</button>
      <button type="button" @click="openFavorabilityLogs">查看关系变化记录</button>
      <button type="button" @click="showManualFavorability">设置好感度</button>
      <button type="button" @click="resetFavorability">重置好感度为 0</button>
      <button type="button" @click="toggleFavorabilityEnabled">{{ favorabilityToggleLabel }}</button>
    </menu>

    <div
      v-if="deleteRequest"
      class="pet-chat-delete-backdrop"
      role="presentation"
      @click.self="cancelDeleteMessages"
    >
      <section class="pet-chat-delete-dialog" role="alertdialog" aria-modal="true">
        <h2>删除聊天记录</h2>
        <p>{{ deleteRequest.summary }}</p>
        <footer>
          <button type="button" :disabled="deletingMessages" @click="cancelDeleteMessages">取消</button>
          <button
            class="danger-button"
            type="button"
            :disabled="deletingMessages"
            @click="confirmDeleteMessages"
          >
            {{ deletingMessages ? '删除中...' : '删除' }}
          </button>
        </footer>
      </section>
    </div>

    <div
      v-if="characterStatusVisible"
      class="pet-chat-settings-backdrop"
      role="presentation"
      @click.self="closeCharacterStatus"
    >
      <section class="pet-chat-character-dialog" role="dialog" aria-modal="true" @click.stop>
        <header>
          <div class="pet-chat-character-heading">
            <img :src="companionAvatarSrc" :alt="currentCompanion?.name || '宠物'" draggable="false" />
            <div>
              <h2>{{ currentCompanion?.name || '宠物' }}</h2>
              <p>{{ favorabilitySummary }}</p>
            </div>
          </div>
          <button type="button" title="关闭状态" @click="closeCharacterStatus">×</button>
        </header>

        <div v-if="companionStatus" class="pet-chat-status-grid">
          <div>
            <span>好感度系统</span>
            <strong>{{ favorabilityStateLabel }}</strong>
          </div>
          <div>
            <span>当前关系</span>
            <strong>{{ companionStatus.relationshipStageName }}</strong>
          </div>
          <div>
            <span>好感度</span>
            <strong>{{ companionStatus.favorability }}</strong>
          </div>
          <div>
            <span>心情</span>
            <strong>{{ companionStatus.mood }}</strong>
          </div>
          <div>
            <span>信任度</span>
            <strong>{{ companionStatus.trust }}</strong>
          </div>
          <div>
            <span>亲密度</span>
            <strong>{{ companionStatus.intimacy }}</strong>
          </div>
        </div>

        <p class="pet-chat-character-note">
          最近变化：{{ companionStatus?.lastChangeReason || '暂无关系变化记录' }}
        </p>
        <p class="pet-chat-character-note">
          最近互动：{{ statusTimeText(companionStatus?.lastInteractionTime) }}
        </p>
        <p v-if="!companionStatus?.favorabilityEnabled" class="pet-chat-character-note">
          当前状态不会影响角色回复；开启后会恢复已保存的好感度数据。
        </p>

        <div v-if="manualFavorabilityVisible" class="pet-chat-manual-favorability">
          <label>
            设置好感度
            <input v-model.number="manualFavorabilityDraft" type="number" min="-9999" max="9999" />
          </label>
          <button type="button" :disabled="companionStatusLoading" @click="setManualFavorability">
            保存数值
          </button>
        </div>

        <p v-if="companionStatusError" class="pet-chat-error in-dialog">{{ companionStatusError }}</p>

        <footer>
          <button type="button" :disabled="companionStatusLoading" @click="toggleFavorabilityEnabled">
            {{ favorabilityToggleLabel }}
          </button>
          <button type="button" :disabled="companionStatusLoading" @click="showManualFavorability">
            设置好感度
          </button>
          <button type="button" :disabled="companionStatusLoading" @click="resetFavorability">
            重置
          </button>
        </footer>
      </section>
    </div>

    <div
      v-if="favorabilityLogsVisible"
      class="pet-chat-settings-backdrop"
      role="presentation"
      @click.self="closeFavorabilityLogs"
    >
      <section class="pet-chat-character-dialog pet-chat-log-dialog" role="dialog" aria-modal="true" @click.stop>
        <header>
          <div>
            <h2>关系变化记录</h2>
            <p>{{ currentCompanion?.name || '宠物' }} 的最近好感度日志</p>
          </div>
          <button type="button" title="关闭记录" @click="closeFavorabilityLogs">×</button>
        </header>
        <div v-if="favorabilityLogsLoading" class="pet-chat-log-empty">正在读取...</div>
        <div v-else-if="favorabilityLogs.length === 0" class="pet-chat-log-empty">
          暂无关系变化记录。
        </div>
        <div v-else class="pet-chat-log-list">
          <article v-for="log in favorabilityLogs" :key="log.id" class="pet-chat-log-row">
            <div>
              <strong>{{ signedValue(log.changeValue) }} / {{ logSourceLabel(log.source) }}</strong>
              <small>{{ statusTimeText(log.createdAt) }}</small>
            </div>
            <p>
              {{ log.oldFavorability }} -> {{ log.newFavorability }}，
              {{ relationshipStageLabel(log.oldStage) }} -> {{ relationshipStageLabel(log.newStage) }}
            </p>
            <small>{{ log.reason || '无原因说明' }}</small>
          </article>
        </div>
        <p v-if="companionStatusError" class="pet-chat-error in-dialog">{{ companionStatusError }}</p>
        <footer>
          <button type="button" :disabled="favorabilityLogsLoading" @click="loadFavorabilityLogs">
            刷新
          </button>
          <button type="button" @click="closeFavorabilityLogs">关闭</button>
        </footer>
      </section>
    </div>

    <div
      v-if="chatSettingsVisible"
      class="pet-chat-settings-backdrop"
      role="presentation"
      @click.self="closeChatSettings"
    >
      <section class="pet-chat-settings-dialog" role="dialog" aria-modal="true">
        <header>
          <div>
            <h2>对话设置</h2>
            <p>这些设置只影响聊天窗口的显示方式。</p>
          </div>
          <button type="button" title="关闭设置" @click="closeChatSettings">×</button>
        </header>
        <label class="pet-chat-settings-toggle">
          <span>
            <strong>逐字显示回复</strong>
            <small>开启后，伴侣回复会以打字机效果显示。</small>
          </span>
          <input v-model="chatSettingsDraft.typewriterEnabled" type="checkbox" />
        </label>
        <label class="pet-chat-settings-toggle">
          <span>
            <strong>开启旁白功能</strong>
            <small>开启后，括号、【】或 *动作* 中的内容会作为旁白显示；关闭后只显示双方对话。</small>
          </span>
          <input v-model="chatSettingsDraft.narrationEnabled" type="checkbox" />
        </label>
        <p v-if="chatSettingsError" class="pet-chat-error">{{ chatSettingsError }}</p>
        <footer>
          <button type="button" :disabled="chatSettingsSaving" @click="closeChatSettings">取消</button>
          <button
            class="primary-button"
            type="button"
            :disabled="chatSettingsSaving"
            @click="saveChatSettings"
          >
            {{ chatSettingsSaving ? '保存中...' : '保存' }}
          </button>
        </footer>
      </section>
    </div>
  </main>
</template>
