<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { PetChatMessage, PetChatReply, PetDrawerConfig } from '../types/app'

type ChatMessage = PetChatMessage & {
  id: string
  local?: boolean
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
const messageListRef = ref<HTMLElement | null>(null)
let unlistenChatOpened: (() => void) | null = null

const aiEnabled = computed(() => Boolean(config.value?.ai?.enabled))
const aiSummary = computed(() => {
  const ai = config.value?.ai
  if (!ai?.enabled) {
    return '未启用 AI 接口'
  }

  const memoryStatus = ai.memoryEnabled === false ? '记忆关闭' : '记忆开启'
  return `${providerLabel(ai.provider)} / ${ai.model || '未设置模型'} / ${memoryStatus}`
})

onMounted(async () => {
  await loadConfig()
  unlistenChatOpened = await listen('pet-chat-opened', () => {
    void loadConfig()
  })
})

onBeforeUnmount(() => {
  unlistenChatOpened?.()
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

function requestMessages() {
  return messages.value
    .filter((message) => !message.local)
    .map(({ role, content }) => ({ role, content }))
}

async function sendMessage() {
  const content = inputText.value.trim()
  if (!content || sending.value) {
    return
  }

  errorMessage.value = ''
  inputText.value = ''
  messages.value.push({
    id: `${Date.now()}-user`,
    role: 'user',
    content,
  })
  sending.value = true

  try {
    const reply = await invoke<PetChatReply>('send_pet_chat_message', {
      messages: requestMessages(),
    })
    messages.value.push({
      id: `${Date.now()}-assistant`,
      role: 'assistant',
      content: reply.message,
    })
  } catch (err) {
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
  <main class="pet-chat-window">
    <header class="pet-chat-header" @pointerdown="startDrag">
      <div>
        <h1>宠物对话</h1>
        <p>{{ aiSummary }}</p>
      </div>
      <button type="button" title="关闭对话" @pointerdown="stopHeaderDrag" @click.stop="hideChat">
        ×
      </button>
    </header>

    <section v-if="!aiEnabled" class="pet-chat-notice">
      <strong>需要先配置 AI 接口</strong>
      <span>打开抽屉设置，在“AI 接口”中启用并填写服务商、模型和 API Key。</span>
      <div>
        <button type="button" @click="openDrawer">打开抽屉</button>
        <button type="button" @click="loadConfig">刷新配置</button>
      </div>
    </section>

    <section ref="messageListRef" class="pet-chat-messages">
      <article
        v-for="message in messages"
        :key="message.id"
        class="pet-chat-message"
        :class="message.role"
      >
        <span>{{ message.role === 'user' ? '你' : '宠物' }}</span>
        <p>{{ message.content }}</p>
      </article>
      <article v-if="sending" class="pet-chat-message assistant">
        <span>宠物</span>
        <p>正在想...</p>
      </article>
    </section>

    <p v-if="errorMessage" class="pet-chat-error">{{ errorMessage }}</p>

    <form class="pet-chat-input" @submit.prevent="sendMessage">
      <textarea
        v-model="inputText"
        :disabled="sending || !aiEnabled"
        rows="2"
        :placeholder="aiEnabled ? '输入消息，Enter 发送' : '先在设置中启用 AI 接口'"
        @keydown="handleInputKeydown"
      />
      <button type="submit" :disabled="sending || !aiEnabled || inputText.trim().length === 0">
        发送
      </button>
    </form>
  </main>
</template>
