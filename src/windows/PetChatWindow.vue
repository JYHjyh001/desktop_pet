<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type {
  Companion,
  DrawerTheme,
  PetChatMessage,
  PetChatReply,
  PetDrawerConfig,
  PetMemoryMessage,
} from '../types/app'

type ChatMessage = PetChatMessage & {
  id: string
  memoryId?: number
  local?: boolean
  visibleContent?: string
  typing?: boolean
}

type MessageContextMenu = {
  messageId: number
  x: number
  y: number
}

type MessageDeleteRequest = {
  messageIds: number[]
  summary: string
}

const chatWindow = getCurrentWindow()
const messages = ref<ChatMessage[]>([
  {
    id: 'welcome',
    role: 'assistant',
    content: '我在，想聊点什么？',
    local: true,
  },
])
const inputText = ref('')
const sending = ref(false)
const errorMessage = ref('')
const config = ref<PetDrawerConfig | null>(null)
const currentCompanion = ref<Companion | null>(null)
const messageListRef = ref<HTMLElement | null>(null)
const messageContextMenu = ref<MessageContextMenu | null>(null)
const selectionMode = ref(false)
const selectedMessageIds = ref<number[]>([])
const deleteRequest = ref<MessageDeleteRequest | null>(null)
const deletingMessages = ref(false)
let unlistenChatOpened: (() => void) | null = null
let unlistenThemeChanged: (() => void) | null = null
let unlistenChatDisplayChanged: (() => void) | null = null
let unlistenCompanionChanged: (() => void) | null = null
let typewriterTimer: number | null = null
let activeTypewriterMessageId: string | null = null

const typewriterBaseDelayMs = 18
const typewriterMaxSteps = 260

const aiEnabled = computed(() => Boolean(config.value?.ai?.enabled))
const typewriterEnabled = computed(() => config.value?.drawer.chatTypewriterEnabled !== false)
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

onMounted(async () => {
  await Promise.all([loadConfig(), loadConversation()])
  unlistenChatOpened = await listen('pet-chat-opened', () => {
    void Promise.all([loadConfig(), loadConversation()])
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
  unlistenCompanionChanged = await listen('companion-changed', () => {
    finishTypewriter()
    void Promise.all([loadConfig(), loadConversation()])
  })
})

onBeforeUnmount(() => {
  unlistenChatOpened?.()
  unlistenThemeChanged?.()
  unlistenChatDisplayChanged?.()
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

function welcomeMessage(companion?: Companion | null): ChatMessage {
  return {
    id: 'welcome',
    role: 'assistant',
    content: companion ? `我是${companion.name}，想聊点什么？` : '我在，想聊点什么？',
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

function openMessageContextMenu(event: MouseEvent, message: ChatMessage) {
  if (!canManageMessage(message) || sending.value || deletingMessages.value) {
    return
  }

  finishTypewriter()
  const messageId = message.memoryId as number
  const menuWidth = 136
  const menuHeight = 84
  messageContextMenu.value = {
    messageId,
    x: Math.max(8, Math.min(event.clientX, window.innerWidth - menuWidth - 8)),
    y: Math.max(8, Math.min(event.clientY, window.innerHeight - menuHeight - 8)),
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
    .map(({ role, content }) => ({ role, content }))
}

function displayedContent(message: ChatMessage) {
  return message.visibleContent ?? message.content
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

  finishTypewriter()
  errorMessage.value = ''
  inputText.value = ''
  messages.value.push({
    id: `${Date.now()}-user`,
    role: 'user',
    content,
  })
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
        content: reply.message,
        local: true,
      } satisfies ChatMessage)
    if (assistantMessage.local) {
      messages.value.push(assistantMessage)
    }
    startTypewriter(assistantMessage.id)
    if (reply.memoryWarning) {
      errorMessage.value = `本次回复已生成，但宠物记忆未能保存：${reply.memoryWarning}`
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
</script>

<template>
  <main class="pet-chat-window" :class="`theme-${drawerTheme}`" @click="closeMessageContextMenu">
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
      <article
        v-for="message in messages"
        :key="message.id"
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
        <span>{{ message.role === 'user' ? '你' : currentCompanion?.name || '宠物' }}</span>
        <p :class="{ typing: message.typing }">{{ displayedContent(message) }}</p>
      </article>
      <article v-if="sending" class="pet-chat-message assistant">
        <span>{{ currentCompanion?.name || '宠物' }}</span>
        <p>正在想...</p>
      </article>
    </section>

    <p v-if="errorMessage" class="pet-chat-error">{{ errorMessage }}</p>

    <form class="pet-chat-input" @submit.prevent="sendMessage">
      <textarea
        v-model="inputText"
        :disabled="sending || !aiEnabled || selectionMode"
        rows="2"
        :placeholder="
          selectionMode
            ? '请先完成或取消多选'
            : aiEnabled
              ? '输入消息，Enter 发送'
              : '先在设置中启用 AI 接口'
        "
        @keydown="handleInputKeydown"
      />
      <button
        type="submit"
        :disabled="sending || !aiEnabled || selectionMode || inputText.trim().length === 0"
      >
        发送
      </button>
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
  </main>
</template>
