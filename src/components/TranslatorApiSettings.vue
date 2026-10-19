<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { AiConnectionTestResult, TranslationSettingsDraft, TranslationSettingsView } from '../types/app'

const emit = defineEmits<{ close: []; saved: [settings: TranslationSettingsView] }>()
const dialog = ref<HTMLElement | null>(null)
const modeInput = ref<HTMLSelectElement | null>(null)
const loading = ref(true)
const saving = ref(false)
const testing = ref(false)
const error = ref('')
const status = ref('')
const stored = ref<TranslationSettingsView | null>(null)
const useGlobalApi = ref(true)
const provider = ref('openai')
const baseUrl = ref('')
const model = ref('')
const apiKey = ref('')
const clearKey = ref(false)
let active = true
const busy = computed(() => loading.value || saving.value || testing.value)
const testDestination = computed(() => {
  if (useGlobalApi.value) return stored.value?.effective.useGlobalApi ? stored.value.effective.host : '当前全局接口'
  try { return new URL(baseUrl.value).hostname || '当前独立接口' } catch { return '当前独立接口' }
})

onMounted(async () => {
  await nextTick()
  dialog.value?.querySelector<HTMLButtonElement>('button')?.focus()
  await load()
})
onBeforeUnmount(() => { active = false; apiKey.value = '' })

async function load() {
  loading.value = true
  error.value = ''
  try {
    const view = await invoke<TranslationSettingsView>('get_translation_settings')
    if (!active) return
    stored.value = view
    useGlobalApi.value = view.useGlobalApi
    provider.value = view.provider
    baseUrl.value = view.baseUrl
    model.value = view.model
  } catch (err) {
    if (active) error.value = String(err)
  } finally {
    if (active) {
      loading.value = false
      await nextTick()
      modeInput.value?.focus()
    }
  }
}

function draft(): TranslationSettingsDraft {
  const replacement = apiKey.value.trim()
  return { useGlobalApi: useGlobalApi.value, provider: provider.value, baseUrl: baseUrl.value, model: model.value,
    keyAction: clearKey.value ? 'clear' : replacement ? 'replace' : 'keep', apiKey: clearKey.value ? '' : replacement }
}

async function save() {
  if (busy.value || !stored.value) return
  saving.value = true
  error.value = ''
  status.value = ''
  try {
    const view = await invoke<TranslationSettingsView>('save_translation_settings', { settings: draft() })
    if (active) { apiKey.value = ''; emit('saved', view) }
  } catch (err) { if (active) error.value = String(err) }
  finally { if (active) saving.value = false }
}

async function test() {
  if (busy.value || !stored.value) return
  testing.value = true
  error.value = ''
  status.value = ''
  try {
    const result = await invoke<AiConnectionTestResult>('test_translation_connection', { settings: draft() })
    if (active) status.value = result.message
  } catch (err) { if (active) error.value = String(err) }
  finally { if (active) testing.value = false }
}

function close() { if (!saving.value) emit('close') }
function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); close(); return }
  if (event.key !== 'Tab') return
  const controls = Array.from(dialog.value?.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select:not(:disabled), [tabindex="0"]') || [])
  const first = controls[0]
  const last = controls[controls.length - 1]
  if (!first) { event.preventDefault(); dialog.value?.focus(); return }
  if (event.shiftKey && (document.activeElement === first || !controls.includes(document.activeElement as HTMLElement))) {
    event.preventDefault(); last?.focus()
  } else if (!event.shiftKey && (document.activeElement === last || !controls.includes(document.activeElement as HTMLElement))) {
    event.preventDefault(); first.focus()
  }
}
</script>

<template>
  <div class="translator-settings-backdrop" @pointerdown.stop @keydown="onKeydown">
    <section ref="dialog" class="translator-settings-dialog" role="dialog" aria-modal="true" aria-labelledby="translation-settings-title" :aria-busy="busy" tabindex="-1">
      <header>
        <h2 id="translation-settings-title">翻译设置</h2>
        <button type="button" class="translator-ghost-button" aria-label="关闭翻译设置" :disabled="saving" @click="close">×</button>
      </header>
      <p>窗口翻译与 Ctrl+Alt+T 划选翻译共用这些设置。</p>
      <p v-if="loading" role="status">正在读取设置…</p>
      <div v-if="stored" class="translator-settings-fields">
        <label>接口模式
          <select ref="modeInput" v-model="useGlobalApi" :disabled="busy">
            <option :value="true">共用全局 API</option><option :value="false">独立翻译 API</option>
          </select>
        </label>
        <p v-if="useGlobalApi" class="translator-settings-hint">实时使用全局 API 设置。独立配置会保留；不会复制全局密钥。</p>
        <p v-else class="translator-settings-hint">独立接口不受全局 AI 开关影响。失败时不会改用全局接口。</p>
        <fieldset :disabled="busy || useGlobalApi">
          <legend>独立连接</legend>
          <label>接口类型
            <select v-model="provider">
              <option value="openai">OpenAI 兼容</option><option value="deepseek">DeepSeek</option>
              <option value="anthropic">Anthropic</option><option value="gemini">Gemini</option>
              <option value="ollama">Ollama</option><option value="custom">自定义 OpenAI 兼容</option>
            </select>
          </label>
          <label>Base URL<input v-model="baseUrl" type="text" autocomplete="off" spellcheck="false" placeholder="填写完整接口地址" /></label>
          <label>模型<input v-model="model" type="text" autocomplete="off" spellcheck="false" placeholder="填写服务支持的模型名称" /></label>
          <label>API Key <small>{{ stored.hasApiKey ? '已设置密钥；留空保留' : '尚未设置密钥' }}</small>
            <input v-model="apiKey" type="password" autocomplete="new-password" :disabled="clearKey" placeholder="输入新密钥以替换；留空不变" />
          </label>
          <label class="translator-settings-clear-key"><input v-model="clearKey" type="checkbox" />明确清除已保存的独立密钥</label>
        </fieldset>
        <p class="translator-settings-hint">测试只向 {{ testDestination }} 发送固定连通性文本，不发送原文，不保存草稿。配置只保存在本机。</p>
      </div>
      <p v-if="error" class="translator-error" role="alert">{{ error }}</p>
      <p v-if="status" role="status" aria-live="polite">{{ status }}</p>
      <footer>
        <button v-if="!stored && !loading" type="button" class="translator-ghost-button" @click="load">重试读取</button>
        <button type="button" class="translator-ghost-button" :disabled="busy || !stored" @click="test">{{ testing ? '测试中…' : '测试连接' }}</button>
        <button type="button" class="translator-ghost-button" :disabled="saving" @click="close">取消</button>
        <button type="button" class="translator-primary-button" :disabled="busy || !stored" @click="save">{{ saving ? '保存中…' : '保存' }}</button>
      </footer>
    </section>
  </div>
</template>

<style scoped>
.translator-settings-backdrop { position: fixed; inset: 0; z-index: 50; display: grid; place-items: center; padding: 12px; background: rgba(22, 32, 50, .42); }
.translator-settings-dialog { width: min(100%, 520px); max-height: calc(100vh - 24px); overflow-y: auto; box-sizing: border-box; padding: 16px; border: 1px solid #cbd5e1; border-radius: 16px; background: #f8fbff; color: #263754; box-shadow: 0 16px 40px #16203240; font-size: 13px; }
header, footer { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
h2 { margin: 0; font-size: 17px; }
footer { justify-content: flex-end; flex-wrap: wrap; margin-top: 16px; }
.translator-settings-fields, fieldset { display: grid; gap: 10px; }
label { display: grid; gap: 5px; min-width: 0; }
fieldset { min-width: 0; margin: 0; padding: 12px; border: 1px solid #cbd5e1; border-radius: 10px; }
input:not([type="checkbox"]), select { width: 100%; min-width: 0; box-sizing: border-box; border: 1px solid #bac7d8; border-radius: 7px; padding: 8px; color: inherit; background: white; }
input:focus-visible, select:focus-visible, button:focus-visible { outline: 2px solid #4268bc; outline-offset: 2px; }
.translator-settings-clear-key { display: flex; align-items: center; gap: 8px; }
.translator-settings-hint, small { color: #526176; line-height: 1.5; overflow-wrap: anywhere; }
.translator-error { margin: 10px 0; overflow-wrap: anywhere; }
:global(.theme-animal-island .translator-settings-dialog) { background: var(--island-paper, #fff9e9); color: var(--island-ink, #384d39); border: 2px solid var(--island-border-soft, #c9c39a); }
:global(.theme-animal-island .translator-settings-dialog fieldset) { border-color: var(--island-border-soft, #c9c39a); }
:global(.theme-animal-island .translator-settings-dialog input:not([type="checkbox"])), :global(.theme-animal-island .translator-settings-dialog select) { background: #fffdf4; border-color: #c9c39a; }
:global(.theme-animal-island .translator-settings-dialog .translator-settings-hint), :global(.theme-animal-island .translator-settings-dialog small) { color: var(--island-muted, #52624a); }
@media (max-width: 420px) { .translator-settings-dialog { padding: 12px; } }
</style>
