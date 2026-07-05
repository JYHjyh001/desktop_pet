<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useWindowOpenAnimation } from '../composables/useWindowOpenAnimation'
import type { DrawerTheme, PetDrawerConfig } from '../types/app'

type TranslationMode = 'plain' | 'polish'

interface TranslationLanguage {
  value: string
  label: string
}

interface TranslationReply {
  translatedText: string
  detectedLanguage?: string
  provider: string
  model: string
}

interface TranslatorPrefillPayload {
  sourceText?: string
  translatedText?: string
  provider?: string
  model?: string
  sourceLanguage?: string
  targetLanguage?: string
  mode?: string
  statusMessage?: string
}

const translatorWindow = getCurrentWindow()
const { windowOpenAnimationClass } = useWindowOpenAnimation('panel')

const languages: TranslationLanguage[] = [
  { value: 'auto', label: '自动识别' },
  { value: 'zh', label: '中文' },
  { value: 'en', label: '英文' },
  { value: 'ja', label: '日文' },
  { value: 'ko', label: '韩文' },
  { value: 'fr', label: '法文' },
  { value: 'de', label: '德文' },
  { value: 'es', label: '西班牙文' },
  { value: 'ru', label: '俄文' },
  { value: 'it', label: '意大利文' },
  { value: 'pt', label: '葡萄牙文' },
]
const targetLanguages = languages.filter((language) => language.value !== 'auto')

const config = ref<PetDrawerConfig | null>(null)
const sourceLanguage = ref('auto')
const targetLanguage = ref('zh')
const translationMode = ref<TranslationMode>('plain')
const sourceText = ref('')
const translatedText = ref('')
const statusMessage = ref('')
const errorMessage = ref('')
const translating = ref(false)
const copied = ref(false)

let unlistenTranslatorOpened: (() => void) | null = null
let unlistenTranslatorPrefill: (() => void) | null = null
let unlistenThemeChanged: (() => void) | null = null
let copyResetTimer: number | null = null

const drawerTheme = computed<DrawerTheme>(() =>
  config.value?.drawer.theme === 'animal-island' ? 'animal-island' : 'light',
)
const aiEnabled = computed(() => Boolean(config.value?.ai?.enabled))
const aiSummary = computed(() => {
  const ai = config.value?.ai
  if (!ai?.enabled) {
    return 'AI 未启用'
  }

  return `${providerLabel(ai.provider)} / ${ai.model || '未设置模型'}`
})
const sourceCount = computed(() => sourceText.value.trim().length)
const canTranslate = computed(() => aiEnabled.value && sourceCount.value > 0 && !translating.value)
const currentSourceLabel = computed(() => languageLabel(sourceLanguage.value))
const currentTargetLabel = computed(() => languageLabel(targetLanguage.value))

onMounted(async () => {
  await loadConfig()
  unlistenTranslatorOpened = await listen('translator-opened', () => {
    void loadConfig()
  })
  unlistenTranslatorPrefill = await listen<TranslatorPrefillPayload>('translator-prefill', (event) => {
    applyTranslatorPrefill(event.payload)
  })
  unlistenThemeChanged = await listen<string>('ui-theme-changed', (event) => {
    if (config.value) {
      config.value.drawer.theme = event.payload
    }
  })
})

onBeforeUnmount(() => {
  unlistenTranslatorOpened?.()
  unlistenTranslatorPrefill?.()
  unlistenThemeChanged?.()
  if (copyResetTimer !== null) {
    window.clearTimeout(copyResetTimer)
  }
})

async function loadConfig() {
  try {
    config.value = await invoke<PetDrawerConfig>('get_config')
  } catch (err) {
    errorMessage.value = String(err)
  }
}

function applyTranslatorPrefill(payload: TranslatorPrefillPayload) {
  sourceText.value = payload.sourceText?.trim() ?? ''
  translatedText.value = payload.translatedText?.trim() ?? ''
  sourceLanguage.value = normalizeTranslatorLanguage(payload.sourceLanguage, true)
  targetLanguage.value = normalizeTranslatorLanguage(payload.targetLanguage, false)
  translationMode.value = payload.mode === 'polish' ? 'polish' : 'plain'
  statusMessage.value =
    payload.statusMessage?.trim() ||
    (payload.provider ? `${providerLabel(payload.provider)} / ${payload.model || '当前模型'}` : '划选翻译结果')
  errorMessage.value = ''
  copied.value = false
  translating.value = false
}

function normalizeTranslatorLanguage(value: string | undefined, allowAuto: boolean) {
  const normalized = (value || '').trim()
  if (allowAuto && (!normalized || normalized === 'auto' || normalized === '自动识别')) {
    return 'auto'
  }

  const allowedLanguages = allowAuto ? languages : targetLanguages
  if (allowedLanguages.some((language) => language.value === normalized)) {
    return normalized
  }

  return allowAuto ? 'auto' : 'zh'
}

async function startDrag() {
  await translatorWindow.startDragging()
}

async function hideTranslator() {
  await invoke('hide_translator')
}

function swapLanguages() {
  if (sourceLanguage.value === 'auto') {
    sourceLanguage.value = targetLanguage.value
    targetLanguage.value = targetLanguage.value === 'zh' ? 'en' : 'zh'
    return
  }

  const nextSource = targetLanguage.value
  targetLanguage.value = sourceLanguage.value
  sourceLanguage.value = nextSource
}

function clearText() {
  sourceText.value = ''
  translatedText.value = ''
  statusMessage.value = ''
  errorMessage.value = ''
}

async function translate() {
  if (translating.value) {
    return
  }

  errorMessage.value = ''
  statusMessage.value = ''
  copied.value = false
  const text = sourceText.value.trim()
  if (!text) {
    errorMessage.value = '请输入需要翻译的文本。'
    return
  }
  if (!aiEnabled.value) {
    errorMessage.value = '请先在抽屉设置中启用 AI 接口。'
    return
  }

  translating.value = true
  try {
    const reply = await invoke<TranslationReply>('translate_text', {
      request: {
        sourceLanguage: sourceLanguage.value,
        targetLanguage: targetLanguage.value,
        text,
        mode: translationMode.value,
      },
    })
    translatedText.value = reply.translatedText
    statusMessage.value = `${providerLabel(reply.provider)} / ${reply.model || '当前模型'}`
  } catch (err) {
    errorMessage.value = String(err)
  } finally {
    translating.value = false
  }
}

async function copyTranslatedText() {
  if (!translatedText.value.trim()) {
    return
  }

  try {
    await writeClipboardText(translatedText.value)
    copied.value = true
    if (copyResetTimer !== null) {
      window.clearTimeout(copyResetTimer)
    }
    copyResetTimer = window.setTimeout(() => {
      copied.value = false
      copyResetTimer = null
    }, 1400)
  } catch (err) {
    errorMessage.value = `复制失败：${String(err)}`
  }
}

async function writeClipboardText(value: string) {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(value)
    return
  }

  const textarea = document.createElement('textarea')
  textarea.value = value
  textarea.style.position = 'fixed'
  textarea.style.left = '-9999px'
  textarea.style.top = '0'
  document.body.appendChild(textarea)
  textarea.focus()
  textarea.select()
  const ok = document.execCommand('copy')
  document.body.removeChild(textarea)
  if (!ok) {
    throw new Error('当前环境不允许写入剪贴板')
  }
}

function languageLabel(value: string) {
  return languages.find((language) => language.value === value)?.label ?? value
}

function providerLabel(provider: string) {
  const labels: Record<string, string> = {
    openai: 'OpenAI 兼容',
    deepseek: 'DeepSeek',
    anthropic: 'Anthropic',
    gemini: 'Gemini',
    ollama: 'Ollama',
    custom: '自定义',
  }

  return (labels[provider] ?? provider) || 'AI 接口'
}
</script>

<template>
  <main class="translator-window" :class="[`theme-${drawerTheme}`, windowOpenAnimationClass]">
    <header class="translator-header" @pointerdown="startDrag">
      <div>
        <h1>翻译</h1>
        <p>{{ aiSummary }}</p>
      </div>
      <div class="translator-header-actions" @pointerdown.stop>
        <button type="button" class="secondary-button" @click="loadConfig">刷新</button>
        <button type="button" class="window-close" title="关闭" @click="hideTranslator">×</button>
      </div>
    </header>

    <section v-if="!aiEnabled" class="translator-notice">
      <strong>AI 接口未启用</strong>
      <span>请先在抽屉设置中启用 AI 接口。</span>
    </section>

    <section class="translator-language-bar" aria-label="翻译语言">
      <label>
        <span>源语言</span>
        <select v-model="sourceLanguage">
          <option v-for="language in languages" :key="language.value" :value="language.value">
            {{ language.label }}
          </option>
        </select>
      </label>
      <button type="button" class="translator-swap-button" :title="`${currentSourceLabel} ↔ ${currentTargetLabel}`" @click="swapLanguages">
        ⇄
      </button>
      <label>
        <span>目标语言</span>
        <select v-model="targetLanguage">
          <option v-for="language in targetLanguages" :key="language.value" :value="language.value">
            {{ language.label }}
          </option>
        </select>
      </label>
      <div class="translator-mode-toggle" aria-label="翻译模式">
        <button type="button" :class="{ active: translationMode === 'plain' }" @click="translationMode = 'plain'">
          直译
        </button>
        <button type="button" :class="{ active: translationMode === 'polish' }" @click="translationMode = 'polish'">
          润色
        </button>
      </div>
    </section>

    <section class="translator-editor-grid">
      <label class="translator-editor-panel">
        <span>原文</span>
        <textarea
          v-model="sourceText"
          maxlength="6000"
          placeholder="输入要翻译的文本"
          @keydown.ctrl.enter.prevent="translate"
        />
        <small>{{ sourceCount }} / 6000</small>
      </label>

      <section class="translator-editor-panel translator-output-panel" aria-label="译文">
        <span>译文</span>
        <div class="translator-output" :class="{ empty: !translatedText }">
          {{ translatedText || '等待翻译' }}
        </div>
        <small>{{ statusMessage || '准备就绪' }}</small>
      </section>
    </section>

    <p v-if="errorMessage" class="translator-error">{{ errorMessage }}</p>

    <footer class="translator-actions">
      <button type="button" class="translator-ghost-button" :disabled="translating || (!sourceText && !translatedText)" @click="clearText">
        清空
      </button>
      <button type="button" class="translator-ghost-button" :disabled="!translatedText" @click="copyTranslatedText">
        {{ copied ? '已复制' : '复制译文' }}
      </button>
      <button type="button" class="translator-primary-button" :disabled="!canTranslate" @click="translate">
        {{ translating ? '翻译中' : '翻译' }}
      </button>
    </footer>
  </main>
</template>
