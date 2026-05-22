<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { emit as emitEvent } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { open, save } from '@tauri-apps/plugin-dialog'
import AppCard from '../components/AppCard.vue'
import CategoryList from '../components/CategoryList.vue'
import SearchBar from '../components/SearchBar.vue'
import { useAppStore } from '../stores/appStore'
import type {
  AiProvider,
  AiConnectionTestResult,
  AppDraft,
  AppItemKind,
  PetMemory,
  PetAnimationSet,
  PetApp,
  PetDrawerConfig,
  PetSkinSummary,
  RuntimeInfo,
  UpdateCheckResult,
} from '../types/app'
import { getPetSkinAnimation, getPetSkinPreview } from '../utils/defaultPet'
import {
  appNameFromPath,
  folderNameFromPath,
  normalizeWebsiteUrl,
  parseTags,
  websiteNameFromUrl,
} from '../utils/format'

const store = useAppStore()
const modalVisible = ref(false)
const saving = ref(false)
const iconLoading = ref(false)
const formError = ref('')
const drawerWindow = getCurrentWindow()
const petSkinModalVisible = ref(false)
const settingsModalVisible = ref(false)
const petSkins = ref<PetSkinSummary[]>([])
const currentPetSkin = ref<PetSkinSummary | null>(null)
const selectedPetSkin = ref<PetSkinSummary | null>(null)
const petSkinLoading = ref(false)
const petSkinError = ref('')
const skinImporting = ref(false)
const skinDeleting = ref(false)
const quickSearchTags = ref<string[]>([])
const tagDisplayMode = ref<'compact' | 'detailed'>('compact')
const displayModeSaving = ref(false)
const settingsSaving = ref(false)
const settingsError = ref('')
const updateChecking = ref(false)
const updateInfo = ref<UpdateCheckResult | null>(null)
const updateError = ref('')
const aiTesting = ref(false)
const aiTestMessage = ref('')
const aiTestError = ref('')
const petMemories = ref<PetMemory[]>([])
const petMemoriesLoading = ref(false)
const petMemoryStatus = ref('')
const petMemoryError = ref('')
const runtimeInfo = ref<RuntimeInfo | null>(null)
const runtimeInfoLoading = ref(false)
const runtimeInfoError = ref('')

const skinDraft = reactive({
  name: '',
  idle: '',
  hover: '',
  dragging: '',
  click: '',
})

const animationFields: Array<{
  key: keyof PetAnimationSet
  label: string
  required?: boolean
}> = [
  { key: 'idle', label: '待机动画', required: true },
  { key: 'hover', label: '选中动画' },
  { key: 'click', label: '点击动画' },
  { key: 'dragging', label: '拖动动画' },
]

type SettingsSectionId =
  | 'entries'
  | 'system'
  | 'ai'
  | 'memory'
  | 'window'
  | 'update'
  | 'diagnostics'

const settingsSections: Array<{ id: SettingsSectionId; label: string; description: string }> = [
  { id: 'entries', label: '入口管理', description: '分类和快捷搜索' },
  { id: 'system', label: '系统', description: '自启和常用规则' },
  { id: 'ai', label: 'AI 接口', description: '宠物聊天 API' },
  { id: 'memory', label: '记忆', description: '长期记忆管理' },
  { id: 'window', label: '窗口', description: '置顶行为' },
  { id: 'update', label: '更新', description: '版本检查' },
  { id: 'diagnostics', label: '诊断', description: '运行路径和数据' },
]

const activeSettingsSection = ref<SettingsSectionId>('entries')

const aiProviderOptions: Array<{
  value: AiProvider
  label: string
  baseUrl: string
  model: string
  help: string
}> = [
  {
    value: 'openai',
    label: 'OpenAI 兼容',
    baseUrl: 'https://api.openai.com/v1',
    model: 'gpt-4o-mini',
    help: '适用于 OpenAI 官方接口和大多数兼容 Chat Completions 的服务。',
  },
  {
    value: 'deepseek',
    label: 'DeepSeek',
    baseUrl: 'https://api.deepseek.com',
    model: 'deepseek-chat',
    help: '适用于 DeepSeek 官方 API。',
  },
  {
    value: 'anthropic',
    label: 'Anthropic',
    baseUrl: 'https://api.anthropic.com',
    model: 'claude-3-5-haiku-latest',
    help: '适用于 Claude API，宠物聊天会按该类型适配请求格式。',
  },
  {
    value: 'gemini',
    label: 'Gemini',
    baseUrl: 'https://generativelanguage.googleapis.com',
    model: 'gemini-1.5-flash',
    help: '适用于 Google Gemini API。',
  },
  {
    value: 'ollama',
    label: 'Ollama 本地',
    baseUrl: 'http://127.0.0.1:11434',
    model: 'llama3.1',
    help: '适用于本机 Ollama 服务，通常不需要 API Key。',
  },
  {
    value: 'custom',
    label: '自定义',
    baseUrl: '',
    model: '',
    help: '用于填写其他服务商或自建 OpenAI 兼容服务。',
  },
]

function petSkinPreviewUrl(skin: PetSkinSummary | null | undefined) {
  return getPetSkinPreview(skin)
}

function petSkinAnimationThumbUrl(
  skin: PetSkinSummary | null | undefined,
  key: keyof PetAnimationSet,
) {
  if (!skin) {
    return ''
  }

  if (skin.builtin) {
    return getPetSkinAnimation(skin, key)
  }

  return skin.animations[key] || ''
}

function petSkinAnimationStatus(skin: PetSkinSummary, key: keyof PetAnimationSet) {
  if (skin.animations[key]) {
    return '已配置'
  }

  return skin.builtin ? '内置' : '使用待机动画'
}

const form = reactive({
  id: '',
  name: '',
  itemKind: 'app' as AppItemKind,
  path: '',
  icon: '',
  iconPreview: '',
  category: '其他',
  tags: '',
  favorite: false,
  runAsAdmin: false,
})

const itemKindLabels: Record<AppItemKind, string> = {
  app: '软件',
  folder: '文件夹',
  website: '网站',
}
const shortcutTypeCategoryLabels = new Set([itemKindLabels.folder, itemKindLabels.website])
const editableKindOptions = computed(() =>
  store.itemKindOptions.filter(
    (item): item is { value: AppItemKind; label: string } => item.value !== 'all',
  ),
)

const isEditing = computed(() => Boolean(form.id))
const modalTitle = computed(() => `${isEditing.value ? '编辑' : '添加'}${itemKindLabels[form.itemKind]}`)
const targetLabel = computed(() => {
  if (form.itemKind === 'website') {
    return '网址'
  }

  if (form.itemKind === 'folder') {
    return '文件夹路径'
  }

  return '软件路径'
})
const targetPlaceholder = computed(() => {
  if (form.itemKind === 'website') {
    return 'https://example.com'
  }

  if (form.itemKind === 'folder') {
    return '选择或填写本机文件夹路径'
  }

  return '选择或填写本机 exe 路径'
})

const settingsDraft = reactive({
  categories: [] as string[],
  quickSearchTags: [] as string[],
  newCategory: '',
  newQuickTag: '',
  tagDisplayMode: 'compact' as 'compact' | 'detailed',
  petAlwaysOnTop: true,
  drawerAlwaysOnTop: true,
  startOnBoot: false,
  autoFavoriteEnabled: true,
  aiEnabled: false,
  aiMemoryEnabled: true,
  aiProvider: 'openai' as AiProvider,
  aiApiKey: '',
  aiBaseUrl: 'https://api.openai.com/v1',
  aiModel: 'gpt-4o-mini',
  aiSystemPrompt: '你是一个友好、简洁的桌面宠物助手。',
  aiTemperature: 0.7,
  aiMaxTokens: 800,
})

onMounted(() => {
  void store.loadApps()
  void loadPetSkins()
  void loadDrawerSettings()
})

async function loadDrawerSettings() {
  try {
    const config = await invoke<PetDrawerConfig>('get_config')
    applyDrawerConfig(config)
  } catch (err) {
    console.error(err)
    quickSearchTags.value = []
    tagDisplayMode.value = 'compact'
    store.setConfiguredCategories([])
  }
}

function applyQuickSearchTag(tag: string) {
  store.keyword = tag
}

async function openSettings() {
  settingsModalVisible.value = true
  settingsError.value = ''
  updateError.value = ''
  aiTestError.value = ''
  aiTestMessage.value = ''
  petMemoryError.value = ''
  runtimeInfoError.value = ''
  activeSettingsSection.value = 'entries'
  await Promise.all([
    loadDrawerSettings(),
    loadPetMemories(),
    checkForUpdate(),
    loadRuntimeInfo(),
  ])
}

function applyDrawerConfig(config: PetDrawerConfig) {
  quickSearchTags.value = config.drawer.quickSearchTags ?? []
  tagDisplayMode.value = normalizeTagDisplayMode(config.drawer.tagDisplayMode)
  store.setConfiguredCategories(config.drawer.categories ?? [])
  syncSettingsDraft(config)
}

function syncSettingsDraft(config: PetDrawerConfig) {
  settingsDraft.categories = (config.drawer.categories ?? []).filter(
    (item) => !shortcutTypeCategoryLabels.has(item.trim()),
  )
  settingsDraft.quickSearchTags = [...(config.drawer.quickSearchTags ?? [])]
  settingsDraft.newCategory = ''
  settingsDraft.newQuickTag = ''
  settingsDraft.tagDisplayMode = normalizeTagDisplayMode(config.drawer.tagDisplayMode)
  settingsDraft.petAlwaysOnTop = config.pet.alwaysOnTop
  settingsDraft.drawerAlwaysOnTop = config.drawer.alwaysOnTop
  settingsDraft.startOnBoot = Boolean(config.system?.startOnBoot)
  settingsDraft.autoFavoriteEnabled = config.system?.autoFavoriteEnabled ?? true
  settingsDraft.aiEnabled = Boolean(config.ai?.enabled)
  settingsDraft.aiMemoryEnabled = config.ai?.memoryEnabled ?? true
  settingsDraft.aiProvider = normalizeAiProvider(config.ai?.provider)
  settingsDraft.aiApiKey = config.ai?.apiKey ?? ''
  settingsDraft.aiBaseUrl = config.ai?.baseUrl ?? selectedAiProviderPreset().baseUrl
  settingsDraft.aiModel = config.ai?.model ?? selectedAiProviderPreset().model
  settingsDraft.aiSystemPrompt =
    config.ai?.systemPrompt || '你是一个友好、简洁的桌面宠物助手。'
  settingsDraft.aiTemperature = config.ai?.temperature ?? 0.7
  settingsDraft.aiMaxTokens = config.ai?.maxTokens ?? 800
}

function normalizeTagDisplayMode(value?: string | null): 'compact' | 'detailed' {
  return value === 'detailed' ? 'detailed' : 'compact'
}

function normalizeAiProvider(value?: string | null): AiProvider {
  const matched = aiProviderOptions.find((option) => option.value === value)
  return matched?.value ?? 'custom'
}

function selectedAiProviderPreset() {
  return (
    aiProviderOptions.find((option) => option.value === settingsDraft.aiProvider) ??
    aiProviderOptions[0]
  )
}

function applyAiProviderPreset() {
  const preset = selectedAiProviderPreset()
  settingsDraft.aiBaseUrl = preset.baseUrl
  settingsDraft.aiModel = preset.model
}

function addSettingsCategory() {
  const category = settingsDraft.newCategory.trim()
  settingsError.value = ''

  if (!category) {
    settingsError.value = '请输入分类名称'
    return
  }

  if (shortcutTypeCategoryLabels.has(category)) {
    settingsError.value = '文件夹和网站已作为快捷入口类型，不再作为分类使用'
    return
  }

  if (settingsDraft.categories.some((item) => item.toLowerCase() === category.toLowerCase())) {
    settingsError.value = '分类已存在'
    return
  }

  settingsDraft.categories.push(category)
  settingsDraft.newCategory = ''
}

function removeSettingsCategory(category: string) {
  if (isCoreCategory(category)) {
    return
  }

  settingsDraft.categories = settingsDraft.categories.filter((item) => item !== category)
}

function isCoreCategory(category: string) {
  return ['全部', '常用', '其他'].includes(category)
}

function addSettingsQuickTag() {
  const tag = settingsDraft.newQuickTag.trim()
  settingsError.value = ''

  if (!tag) {
    settingsError.value = '请输入快捷搜索标签'
    return
  }

  if (settingsDraft.quickSearchTags.some((item) => item.toLowerCase() === tag.toLowerCase())) {
    settingsError.value = '快捷搜索标签已存在'
    return
  }

  settingsDraft.quickSearchTags.push(tag)
  settingsDraft.newQuickTag = ''
}

function removeSettingsQuickTag(tag: string) {
  settingsDraft.quickSearchTags = settingsDraft.quickSearchTags.filter((item) => item !== tag)
}

async function saveSettings() {
  settingsSaving.value = true
  settingsError.value = ''

  try {
    const config = await saveDrawerPreferences(settingsDraft.tagDisplayMode)
    applyDrawerConfig(config)
    settingsModalVisible.value = false
  } catch (err) {
    settingsError.value = String(err)
  } finally {
    settingsSaving.value = false
  }
}

function buildDrawerPreferences(tagMode: 'compact' | 'detailed') {
  return {
    categories: [...settingsDraft.categories],
    quickSearchTags: [...settingsDraft.quickSearchTags],
    tagDisplayMode: tagMode,
    petAlwaysOnTop: settingsDraft.petAlwaysOnTop,
    drawerAlwaysOnTop: settingsDraft.drawerAlwaysOnTop,
    startOnBoot: settingsDraft.startOnBoot,
    autoFavoriteEnabled: settingsDraft.autoFavoriteEnabled,
    ai: {
      enabled: settingsDraft.aiEnabled,
      memoryEnabled: settingsDraft.aiMemoryEnabled,
      provider: settingsDraft.aiProvider,
      apiKey: settingsDraft.aiApiKey,
      baseUrl: settingsDraft.aiBaseUrl,
      model: settingsDraft.aiModel,
      systemPrompt: settingsDraft.aiSystemPrompt,
      temperature: safeNumber(settingsDraft.aiTemperature, 0.7),
      maxTokens: safeInteger(settingsDraft.aiMaxTokens, 800),
    },
  }
}

function safeNumber(value: number, fallback: number) {
  return Number.isFinite(value) ? value : fallback
}

function safeInteger(value: number, fallback: number) {
  return Number.isFinite(value) ? Math.round(value) : fallback
}

async function saveDrawerPreferences(tagMode: 'compact' | 'detailed') {
  return invoke<PetDrawerConfig>('save_drawer_preferences', {
    preferences: buildDrawerPreferences(tagMode),
  })
}

async function testAiConnection() {
  aiTesting.value = true
  aiTestMessage.value = ''
  aiTestError.value = ''

  try {
    const result = await invoke<AiConnectionTestResult>('test_ai_connection', {
      settings: buildDrawerPreferences(settingsDraft.tagDisplayMode).ai,
    })
    aiTestMessage.value = result.message
  } catch (err) {
    aiTestError.value = String(err)
  } finally {
    aiTesting.value = false
  }
}

async function loadPetMemories() {
  petMemoriesLoading.value = true
  petMemoryStatus.value = ''
  petMemoryError.value = ''

  try {
    petMemories.value = await invoke<PetMemory[]>('list_pet_memories')
  } catch (err) {
    petMemoryError.value = String(err)
  } finally {
    petMemoriesLoading.value = false
  }
}

async function deletePetMemory(memory: PetMemory) {
  if (!confirm('确认删除这条宠物记忆？')) {
    return
  }

  try {
    petMemoryStatus.value = ''
    petMemoryError.value = ''
    await invoke('delete_pet_memory', { memoryId: memory.id })
    await loadPetMemories()
    petMemoryStatus.value = '已删除选中的长期记忆。'
  } catch (err) {
    petMemoryError.value = String(err)
  }
}

async function clearPetMemories() {
  if (
    !confirm('确认清空所有宠物长期记忆？聊天记录不会显示在这里，但长期记忆会被软删除。')
  ) {
    return
  }

  try {
    petMemoryStatus.value = ''
    petMemoryError.value = ''
    await invoke('clear_pet_memories')
    await loadPetMemories()
    petMemoryStatus.value = '已清空长期记忆。'
  } catch (err) {
    petMemoryError.value = String(err)
  }
}

async function importPetMemory() {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: '宠物记忆 JSON', extensions: ['json'] }],
  })

  if (typeof selected !== 'string') {
    return
  }

  if (!confirm('导入会替换当前本机宠物记忆和最近聊天记录，确认继续？')) {
    return
  }

  petMemoriesLoading.value = true
  petMemoryStatus.value = ''
  petMemoryError.value = ''

  try {
    petMemories.value = await invoke<PetMemory[]>('import_pet_memory', { path: selected })
    petMemoryStatus.value = '记忆导入完成。'
  } catch (err) {
    petMemoryError.value = String(err)
  } finally {
    petMemoriesLoading.value = false
  }
}

async function exportPetMemory() {
  const target = await save({
    defaultPath: 'pet-memory.json',
    filters: [{ name: '宠物记忆 JSON', extensions: ['json'] }],
  })

  if (!target) {
    return
  }

  petMemoryStatus.value = ''
  petMemoryError.value = ''

  try {
    await invoke('export_pet_memory', { path: target })
    petMemoryStatus.value = '记忆导出完成。导出的 JSON 可能包含私人聊天和长期记忆，请妥善保存。'
  } catch (err) {
    petMemoryError.value = String(err)
  }
}

async function openPetMemoryDirectory() {
  petMemoryStatus.value = ''
  petMemoryError.value = ''

  try {
    await invoke('open_pet_memory_dir')
  } catch (err) {
    petMemoryError.value = String(err)
  }
}

function memoryTypeLabel(type: string) {
  const labels: Record<string, string> = {
    preference: '偏好',
    project: '项目',
    event: '事件',
    profile: '画像',
  }

  return labels[type] ?? type
}

async function setTagDisplayMode(mode: 'compact' | 'detailed') {
  if (tagDisplayMode.value === mode || displayModeSaving.value) {
    return
  }

  const previousMode = tagDisplayMode.value
  tagDisplayMode.value = mode
  settingsDraft.tagDisplayMode = mode
  displayModeSaving.value = true

  try {
    const config = await saveDrawerPreferences(mode)
    applyDrawerConfig(config)
  } catch (err) {
    tagDisplayMode.value = previousMode
    settingsDraft.tagDisplayMode = previousMode
    alert(`保存显示方式失败：${String(err)}`)
  } finally {
    displayModeSaving.value = false
  }
}

async function checkForUpdate() {
  updateChecking.value = true
  updateError.value = ''

  try {
    updateInfo.value = await invoke<UpdateCheckResult>('check_for_update')
  } catch (err) {
    updateError.value = String(err)
  } finally {
    updateChecking.value = false
  }
}

async function openUpdatePage() {
  updateError.value = ''

  try {
    await invoke('open_update_page', { url: updateInfo.value?.updateUrl ?? null })
  } catch (err) {
    updateError.value = String(err)
  }
}

async function loadRuntimeInfo() {
  runtimeInfoLoading.value = true
  runtimeInfoError.value = ''

  try {
    runtimeInfo.value = await invoke<RuntimeInfo>('get_runtime_info')
  } catch (err) {
    runtimeInfoError.value = String(err)
  } finally {
    runtimeInfoLoading.value = false
  }
}

async function loadPetSkins() {
  petSkinLoading.value = true
  petSkinError.value = ''

  try {
    const [skins, current] = await Promise.all([
      invoke<PetSkinSummary[]>('list_pet_skins'),
      invoke<PetSkinSummary>('get_current_pet_skin'),
    ])
    petSkins.value = skins
    currentPetSkin.value = current
    selectedPetSkin.value =
      skins.find((skin) => skin.id === selectedPetSkin.value?.id) ??
      skins.find((skin) => skin.id === current.id) ??
      current
  } catch (err) {
    console.error(err)
    petSkinError.value = String(err)
  } finally {
    petSkinLoading.value = false
  }
}

async function openPetSkinManager() {
  petSkinModalVisible.value = true
  await loadPetSkins()
}

async function startDrawerDrag(event: PointerEvent) {
  if (event.button !== 0) {
    return
  }

  const target = event.target
  if (target instanceof HTMLElement && target.closest('button, input, textarea, select')) {
    return
  }

  try {
    await drawerWindow.startDragging()
  } catch (err) {
    console.error(err)
  }
}

function resetForm() {
  form.id = ''
  form.name = ''
  form.itemKind = 'app'
  form.path = ''
  form.icon = ''
  form.iconPreview = ''
  form.category = '其他'
  form.tags = ''
  form.favorite = false
  form.runAsAdmin = false
  formError.value = ''
}

function openAddModal(itemKind: AppItemKind = 'app') {
  resetForm()
  form.itemKind = itemKind
  form.category = '其他'
  modalVisible.value = true
}

function openEditModal(app: PetApp) {
  form.id = app.id
  form.name = app.name
  form.itemKind = app.itemKind
  form.path = app.path
  form.icon = app.icon ?? ''
  form.iconPreview = app.iconDataUrl ?? ''
  form.category = app.category
  form.tags = app.tags.join('，')
  form.favorite = app.favorite
  form.runAsAdmin = app.runAsAdmin
  formError.value = ''
  modalVisible.value = true
}

function setFormItemKind(itemKind: AppItemKind) {
  if (form.itemKind === itemKind) {
    return
  }

  form.itemKind = itemKind
  form.path = ''
  form.icon = ''
  form.iconPreview = ''
  form.runAsAdmin = false
  if (!form.category.trim() || Object.values(itemKindLabels).includes(form.category)) {
    form.category = '其他'
  }
}

async function pickExecutable() {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: 'Windows 可执行程序',
        extensions: ['exe'],
      },
    ],
  })

  if (typeof selected === 'string') {
    form.path = selected
    if (!form.name) {
      form.name = appNameFromPath(selected)
    }
    await autoFillAppIcon(selected)
  }
}

async function pickFolder() {
  const selected = await open({
    multiple: false,
    directory: true,
  })

  if (typeof selected === 'string') {
    form.path = selected
    if (!form.name.trim()) {
      form.name = folderNameFromPath(selected)
    }
  }
}

function normalizeEntryCategory(category: string) {
  const trimmed = category.trim()
  return !trimmed || shortcutTypeCategoryLabels.has(trimmed) ? '其他' : trimmed
}

async function pickTarget() {
  if (form.itemKind === 'folder') {
    await pickFolder()
    return
  }

  if (form.itemKind === 'app') {
    await pickExecutable()
  }
}

async function autoFillAppIcon(path: string) {
  iconLoading.value = true

  try {
    const relativePath = await invoke<string>('import_executable_icon', { path })
    form.icon = relativePath
    form.iconPreview = await invoke<string>('get_image_data_url', { relativePath })
  } catch (err) {
    console.warn('自动获取软件图标失败', err)
  } finally {
    iconLoading.value = false
  }
}

async function selectPetSkin(skin: PetSkinSummary) {
  selectedPetSkin.value = skin

  try {
    currentPetSkin.value = await invoke<PetSkinSummary>('set_pet_skin', { skinId: skin.id })
    selectedPetSkin.value = currentPetSkin.value
    await emitEvent('pet-skin-updated', skin.id)
    await loadPetSkins()
  } catch (err) {
    petSkinError.value = String(err)
  }
}

async function pickPetAnimation(state: keyof PetAnimationSet) {
  const selected = await openImageFile()
  if (typeof selected !== 'string') {
    return
  }

  skinDraft[state] = selected

  if (state === 'idle' && !skinDraft.name.trim()) {
    skinDraft.name = appNameFromPath(selected)
  }
}

function clearPetAnimation(state: keyof PetAnimationSet) {
  skinDraft[state] = ''
}

async function importPetSkin() {
  if (!skinDraft.idle) {
    petSkinError.value = '导入宠物至少需要选择待机动画'
    return
  }

  skinImporting.value = true
  petSkinError.value = ''

  try {
    const imported = await invoke<PetSkinSummary>('import_pet_skin', {
      name: skinDraft.name.trim() || '自定义宠物',
      animations: {
        idle: skinDraft.idle,
        hover: skinDraft.hover || null,
        dragging: skinDraft.dragging || null,
        click: skinDraft.click || null,
      },
    })

    currentPetSkin.value = imported
    selectedPetSkin.value = imported
    skinDraft.name = ''
    skinDraft.idle = ''
    skinDraft.hover = ''
    skinDraft.dragging = ''
    skinDraft.click = ''
    await emitEvent('pet-skin-updated', imported.id)
    await loadPetSkins()
  } catch (err) {
    petSkinError.value = String(err)
  } finally {
    skinImporting.value = false
  }
}

async function deleteSelectedPetSkin() {
  const skin = selectedPetSkin.value
  if (!skin) {
    return
  }

  if (skin.builtin) {
    petSkinError.value = '内置宠物形象不能删除'
    return
  }

  const confirmed = confirm(`确认删除宠物形象「${skin.name}」？此操作会删除本机保存的动画文件。`)
  if (!confirmed) {
    return
  }

  skinDeleting.value = true
  petSkinError.value = ''

  try {
    const nextSkin = await invoke<PetSkinSummary>('delete_pet_skin', { skinId: skin.id })
    currentPetSkin.value = nextSkin
    selectedPetSkin.value = nextSkin
    await emitEvent('pet-skin-updated', nextSkin.id)
    await loadPetSkins()
  } catch (err) {
    petSkinError.value = String(err)
  } finally {
    skinDeleting.value = false
  }
}

async function resetPetImage() {
  try {
    currentPetSkin.value = await invoke<PetSkinSummary>('set_pet_skin', { skinId: 'default' })
    selectedPetSkin.value = currentPetSkin.value
    await emitEvent('pet-skin-updated', 'default')
    await loadPetSkins()
  } catch (err) {
    alert(`恢复默认宠物失败：${String(err)}`)
  }
}

async function pickAppIcon() {
  const selected = await openImageFile()

  if (typeof selected !== 'string') {
    return
  }

  try {
    const relativePath = await invoke<string>('import_app_icon', { path: selected })
    form.icon = relativePath
    form.iconPreview = await invoke<string>('get_image_data_url', { relativePath })
  } catch (err) {
    formError.value = String(err)
  }
}

async function openImageFile() {
  return open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: '图片',
        extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'ico'],
      },
    ],
  })
}

async function saveApp() {
  formError.value = ''
  const targetPath = form.itemKind === 'website' ? normalizeWebsiteUrl(form.path) : form.path.trim()

  if (!targetPath) {
    formError.value =
      form.itemKind === 'website' ? '请填写网址' : `请选择或填写${targetLabel.value}`
    return
  }

  if (form.itemKind === 'website') {
    try {
      const parsed = new URL(targetPath)
      const isIpAddress = /^\d{1,3}(\.\d{1,3}){3}$/.test(parsed.hostname)
      const hasValidHost =
        parsed.hostname === 'localhost' || parsed.hostname.includes('.') || isIpAddress
      if (!['http:', 'https:'].includes(parsed.protocol) || !hasValidHost) {
        throw new Error('invalid url')
      }
    } catch {
      formError.value = '请输入有效的网址'
      return
    }
  }

  const defaultName =
    form.itemKind === 'website'
      ? websiteNameFromUrl(targetPath)
      : form.itemKind === 'folder'
        ? folderNameFromPath(targetPath)
        : appNameFromPath(targetPath)
  const itemName = form.name.trim() || defaultName

  if (!itemName.trim()) {
    formError.value = `请填写${itemKindLabels[form.itemKind]}名称`
    return
  }

  const entryCategory = normalizeEntryCategory(form.category)
  saving.value = true

  const draft: AppDraft = {
    id: form.id || undefined,
    name: itemName,
    itemKind: form.itemKind,
    path: targetPath,
    icon: form.itemKind === 'app' ? form.icon || undefined : undefined,
    category: entryCategory,
    tags: parseTags(form.tags),
    favorite: form.favorite,
    runAsAdmin: form.itemKind === 'app' && form.runAsAdmin,
  }

  try {
    await store.upsertApp(draft)
    modalVisible.value = false
  } catch (err) {
    formError.value = String(err)
  } finally {
    saving.value = false
  }
}

async function removeApp(app: PetApp) {
  if (!confirm(`确认删除 ${app.name}？`)) {
    return
  }

  await store.removeApp(app.id)
}

async function launchApp(app: PetApp) {
  try {
    await store.launchApp(app.id)
  } catch (err) {
    const message = String(err)
    alert(message.startsWith('启动失败：') || message.startsWith('打开') ? message : `打开失败：${message}`)
  }
}

async function toggleAppAdminLaunch(app: PetApp, runAsAdmin: boolean) {
  try {
    await store.setAppRunAsAdmin(app.id, runAsAdmin)
  } catch (err) {
    alert(`保存管理员启动设置失败：${String(err)}`)
  }
}

async function openAppDirectory(app: PetApp) {
  try {
    await store.openAppDirectory(app.id)
  } catch (err) {
    alert(`打开目录失败：${String(err)}`)
  }
}

async function hideDrawer() {
  await invoke('hide_drawer')
}
</script>

<template>
  <main class="drawer-window">
    <header class="drawer-header" @pointerdown="startDrawerDrag">
      <div class="drawer-titlebar">
        <h1>PetDrawer</h1>
        <p>桌面宠物快捷入口抽屉</p>
      </div>
      <div class="header-actions">
        <div class="display-mode-switch" aria-label="入口显示方式">
          <button
            type="button"
            :disabled="displayModeSaving"
            :class="{ active: tagDisplayMode === 'compact' }"
            title="缩略显示"
            @click="setTagDisplayMode('compact')"
          >
            缩略
          </button>
          <button
            type="button"
            :disabled="displayModeSaving"
            :class="{ active: tagDisplayMode === 'detailed' }"
            title="详细显示"
            @click="setTagDisplayMode('detailed')"
          >
            详细
          </button>
        </div>
        <button class="secondary-button" type="button" @click="openSettings">设置</button>
        <button class="window-close" type="button" title="隐藏抽屉" @click="hideDrawer">×</button>
      </div>
    </header>

    <section class="drawer-layout">
      <aside class="drawer-sidebar">
        <section class="pet-preview-panel">
          <div class="pet-preview-frame">
            <img :src="petSkinPreviewUrl(currentPetSkin)" alt="" />
          </div>
          <div class="pet-preview-copy">
            <h2>当前宠物形象</h2>
            <p v-if="currentPetSkin">{{ currentPetSkin.name }}</p>
            <p v-else>内置默认形象</p>
          </div>
          <div class="pet-preview-actions">
            <button class="secondary-button" type="button" @click="openPetSkinManager">
              更换
            </button>
            <button class="secondary-button" type="button" @click="resetPetImage">默认</button>
          </div>
        </section>

        <CategoryList
          :categories="store.categories"
          :active="store.category"
          @select="store.category = $event"
        />

      </aside>

      <section class="drawer-main">
        <SearchBar
          v-model="store.keyword"
          v-model:active-kind="store.itemKindFilter"
          :kind-options="store.itemKindOptions"
          :quick-tags="quickSearchTags"
          @quick-tag="applyQuickSearchTag"
          @add="openAddModal"
        />

        <div class="app-panel">
          <div class="panel-status" v-if="store.loading">正在读取快捷入口列表...</div>
          <div class="panel-status error" v-else-if="store.error">{{ store.error }}</div>
          <div class="empty-state" v-else-if="store.filteredApps.length === 0">
            <h2>还没有匹配的快捷入口</h2>
            <p>可以添加本地软件、常用文件夹或网站，数据会保存在本机 JSON 中。</p>
            <div class="empty-actions">
              <button class="primary-button" type="button" @click="openAddModal()">添加</button>
            </div>
          </div>
          <div class="app-grid" :class="{ compact: tagDisplayMode === 'compact' }" v-else>
            <AppCard
              v-for="app in store.filteredApps"
              :key="app.id"
              :app="app"
              :tag-display-mode="tagDisplayMode"
              @launch="launchApp"
              @edit="openEditModal"
              @remove="removeApp"
              @open-dir="openAppDirectory"
              @toggle-admin="toggleAppAdminLaunch"
            />
          </div>
        </div>
      </section>
    </section>

    <div v-if="modalVisible" class="modal-backdrop" @click.self="modalVisible = false">
      <form class="app-modal" @submit.prevent="saveApp">
        <header>
          <h2>{{ modalTitle }}</h2>
          <button type="button" class="window-close" @click="modalVisible = false">×</button>
        </header>

        <div class="kind-editor" aria-label="入口类型">
          <button
            v-for="option in editableKindOptions"
            :key="option.value"
            type="button"
            :class="{ active: form.itemKind === option.value }"
            @click="setFormItemKind(option.value)"
          >
            {{ option.label }}
          </button>
        </div>

        <label>
          {{ itemKindLabels[form.itemKind] }}名称
          <input v-model="form.name" autocomplete="off" />
        </label>

        <label>
          {{ targetLabel }}
          <div class="path-row">
            <input v-model="form.path" :placeholder="targetPlaceholder" autocomplete="off" />
            <button v-if="form.itemKind !== 'website'" type="button" @click="pickTarget">
              选择
            </button>
          </div>
        </label>

        <label v-if="form.itemKind === 'app'">
          软件图标
          <div class="icon-picker">
            <span class="icon-preview">
              <img v-if="form.iconPreview" :src="form.iconPreview" alt="" />
              <span v-else>默认</span>
            </span>
            <button type="button" :disabled="iconLoading" @click="pickAppIcon">
              {{ iconLoading ? '获取中...' : '选择图标' }}
            </button>
          </div>
        </label>

        <label>
          分类
          <input v-model="form.category" list="category-options" autocomplete="off" />
          <datalist id="category-options">
            <option v-for="category in store.categories" :key="category" :value="category" />
          </datalist>
        </label>

        <label>
          标签
          <input v-model="form.tags" placeholder="多个标签用空格或逗号分隔" autocomplete="off" />
        </label>

        <label class="checkbox-row">
          <input v-model="form.favorite" type="checkbox" />
          设为常用
        </label>

        <p v-if="formError" class="form-error">{{ formError }}</p>

        <footer>
          <button type="button" @click="modalVisible = false">取消</button>
          <button class="primary-button" type="submit" :disabled="saving">
            {{ saving ? '保存中...' : '保存' }}
          </button>
        </footer>
      </form>
    </div>

    <div
      v-if="petSkinModalVisible"
      class="modal-backdrop"
      @click.self="petSkinModalVisible = false"
    >
      <section class="skin-modal">
        <header>
          <div>
            <h2>更换宠物形象</h2>
            <p>从宠物形象文件夹选择，或导入新的多状态宠物。</p>
          </div>
          <button type="button" class="window-close" @click="petSkinModalVisible = false">
            ×
          </button>
        </header>

        <p v-if="petSkinError" class="form-error">{{ petSkinError }}</p>
        <div v-if="petSkinLoading" class="skin-loading">正在搜索宠物形象...</div>

        <div v-else class="skin-manager-layout">
          <div class="skin-grid">
            <button
              v-for="skin in petSkins"
              :key="skin.id"
              type="button"
              class="skin-card"
              :class="{
                active: currentPetSkin?.id === skin.id,
                selected: selectedPetSkin?.id === skin.id,
              }"
              @click="selectPetSkin(skin)"
            >
              <span class="skin-thumb">
                <img
                  v-if="skin.preview || skin.builtin"
                  :src="petSkinPreviewUrl(skin)"
                  alt=""
                />
                <span v-else>默认</span>
              </span>
              <span class="skin-card-name">{{ skin.name }}</span>
              <span class="skin-state-tags">
                <span>待机</span>
                <span v-if="skin.builtin || skin.animations.hover">选中</span>
                <span v-if="skin.builtin || skin.animations.click">点击</span>
                <span v-if="skin.builtin || skin.animations.dragging">拖动</span>
              </span>
            </button>
          </div>

          <aside class="skin-detail-panel">
            <div class="skin-detail-preview">
              <img
                v-if="selectedPetSkin?.preview || selectedPetSkin?.builtin"
                :src="petSkinPreviewUrl(selectedPetSkin)"
                alt=""
              />
              <span v-else>无预览</span>
            </div>
            <h3>{{ selectedPetSkin?.name || '未选择宠物' }}</h3>
            <p>{{ selectedPetSkin?.builtin ? '内置宠物形象' : '已存储宠物形象' }}</p>

            <div class="skin-animation-list" v-if="selectedPetSkin">
              <div v-for="field in animationFields" :key="field.key" class="skin-animation-item">
                <span class="animation-status-thumb">
                  <img
                    v-if="petSkinAnimationThumbUrl(selectedPetSkin, field.key)"
                    :src="petSkinAnimationThumbUrl(selectedPetSkin, field.key)"
                    alt=""
                  />
                  <span v-else>回退</span>
                </span>
                <span>
                  <strong>{{ field.label }}</strong>
                  <small>
                    {{
                      petSkinAnimationStatus(selectedPetSkin, field.key)
                    }}
                  </small>
                </span>
              </div>
            </div>

            <div class="skin-detail-actions" v-if="selectedPetSkin">
              <button
                v-if="!selectedPetSkin.builtin"
                type="button"
                class="skin-delete-button"
                :disabled="skinDeleting"
                @click="deleteSelectedPetSkin"
              >
                {{ skinDeleting ? '删除中...' : '删除形象' }}
              </button>
              <small v-else>内置默认凯蒂会随程序保留，不能从这里删除。</small>
            </div>
          </aside>
        </div>

        <section class="skin-import-panel">
          <h3>导入宠物</h3>
          <label>
            宠物名称
            <input v-model="skinDraft.name" placeholder="例如：小猫助手" autocomplete="off" />
          </label>

          <div class="animation-picker-grid">
            <div v-for="field in animationFields" :key="field.key" class="animation-picker">
              <div>
                <strong>{{ field.label }}</strong>
                <span>{{ field.required ? '必填' : '可选，未设置时使用待机动画' }}</span>
                <p :title="skinDraft[field.key]">
                  {{ skinDraft[field.key] || '未选择图片' }}
                </p>
              </div>
              <div class="animation-picker-actions">
                <button type="button" @click="pickPetAnimation(field.key)">选择</button>
                <button
                  v-if="skinDraft[field.key]"
                  type="button"
                  @click="clearPetAnimation(field.key)"
                >
                  清除
                </button>
              </div>
            </div>
          </div>

          <footer>
            <button type="button" @click="petSkinModalVisible = false">关闭</button>
            <button class="primary-button" type="button" :disabled="skinImporting" @click="importPetSkin">
              {{ skinImporting ? '导入中...' : '导入并使用' }}
            </button>
          </footer>
        </section>
      </section>
    </div>

    <div
      v-if="settingsModalVisible"
      class="modal-backdrop"
      @click.self="settingsModalVisible = false"
    >
      <section class="settings-modal">
        <header>
          <div>
            <h2>设置</h2>
            <p>管理入口、系统启动、AI 接口、宠物记忆、软件更新和运行诊断。</p>
          </div>
          <button type="button" class="window-close" @click="settingsModalVisible = false">
            ×
          </button>
        </header>

        <div class="settings-body">
          <nav class="settings-nav" aria-label="设置分类">
            <button
              v-for="section in settingsSections"
              :key="section.id"
              type="button"
              :class="{ active: activeSettingsSection === section.id }"
              @click="activeSettingsSection = section.id"
            >
              <strong>{{ section.label }}</strong>
              <span>{{ section.description }}</span>
            </button>
          </nav>

          <div class="settings-panels">
            <section v-show="activeSettingsSection === 'entries'" class="settings-section">
              <h3>分类选项</h3>
              <form class="settings-add-row" @submit.prevent="addSettingsCategory">
                <input v-model="settingsDraft.newCategory" placeholder="输入分类名称" />
                <button class="primary-button" type="submit">添加</button>
              </form>
              <div class="settings-chip-list">
                <span
                  v-for="category in settingsDraft.categories"
                  :key="category"
                  class="settings-chip"
                  :class="{ locked: isCoreCategory(category) }"
                >
                  {{ category }}
                  <button
                    v-if="!isCoreCategory(category)"
                    type="button"
                    title="删除分类"
                    @click="removeSettingsCategory(category)"
                  >
                    ×
                  </button>
                </span>
              </div>

              <h3>快捷搜索</h3>
              <form class="settings-add-row" @submit.prevent="addSettingsQuickTag">
                <input
                  v-model="settingsDraft.newQuickTag"
                  placeholder="输入搜索标签，如 VS Code、AI、办公"
                />
                <button class="primary-button" type="submit">添加</button>
              </form>
              <div class="settings-chip-list" v-if="settingsDraft.quickSearchTags.length > 0">
                <span v-for="tag in settingsDraft.quickSearchTags" :key="tag" class="settings-chip">
                  {{ tag }}
                  <button type="button" title="删除标签" @click="removeSettingsQuickTag(tag)">×</button>
                </span>
              </div>
              <p v-else class="settings-empty">还没有快捷搜索标签。</p>
            </section>

            <section v-show="activeSettingsSection === 'system'" class="settings-section">
              <h3>启动和常用</h3>
              <label class="settings-toggle-row">
                <span>
                  <strong>开机自启</strong>
                  <small>登录 Windows 后自动启动 PetDrawer。</small>
                </span>
                <input v-model="settingsDraft.startOnBoot" type="checkbox" />
              </label>
              <label class="settings-toggle-row">
                <span>
                  <strong>自动加入常用</strong>
                  <small>开启后，最近 7 天内打开 2 次及以上的入口会自动进入“常用”。</small>
                </span>
                <input v-model="settingsDraft.autoFavoriteEnabled" type="checkbox" />
              </label>
              <p class="settings-empty">
                关闭自动加入常用后，程序仍会记录打开次数，但不会再根据打开频率自动新增或移出“常用”。
              </p>
            </section>

            <section v-show="activeSettingsSection === 'ai'" class="settings-section">
              <h3>AI 接口</h3>
              <div class="settings-update-panel">
                <div>
                  <strong>接口连接测试</strong>
                  <small>使用当前表单里的服务商、Base URL、模型和 API Key 发起一次短请求。</small>
                </div>
                <div class="settings-update-actions">
                  <button type="button" :disabled="aiTesting" @click="testAiConnection">
                    {{ aiTesting ? '测试中...' : '测试连接' }}
                  </button>
                </div>
              </div>
              <p v-if="aiTestMessage" class="settings-empty">{{ aiTestMessage }}</p>
              <p v-if="aiTestError" class="form-error">{{ aiTestError }}</p>

              <label class="settings-toggle-row">
                <span>
                  <strong>启用宠物聊天 API</strong>
                  <small>配置会保存到本机 config.json，宠物聊天会读取这里的接口信息。</small>
                </span>
                <input v-model="settingsDraft.aiEnabled" type="checkbox" />
              </label>

              <div class="settings-form-grid">
                <label class="settings-field">
                  服务商
                  <select v-model="settingsDraft.aiProvider" @change="applyAiProviderPreset">
                    <option
                      v-for="option in aiProviderOptions"
                      :key="option.value"
                      :value="option.value"
                    >
                      {{ option.label }}
                    </option>
                  </select>
                </label>

                <label class="settings-field">
                  模型
                  <input v-model="settingsDraft.aiModel" placeholder="例如 gpt-4o-mini" />
                </label>

                <label class="settings-field wide">
                  Base URL
                  <input
                    v-model="settingsDraft.aiBaseUrl"
                    placeholder="https://api.example.com/v1"
                    autocomplete="off"
                  />
                </label>

                <label class="settings-field wide">
                  API Key
                  <input
                    v-model="settingsDraft.aiApiKey"
                    type="password"
                    placeholder="留空则只保存服务商和模型"
                    autocomplete="off"
                  />
                </label>

                <label class="settings-field">
                  温度
                  <input
                    v-model.number="settingsDraft.aiTemperature"
                    type="number"
                    min="0"
                    max="2"
                    step="0.1"
                  />
                </label>

                <label class="settings-field">
                  最大输出 Token
                  <input
                    v-model.number="settingsDraft.aiMaxTokens"
                    type="number"
                    min="64"
                    max="32768"
                    step="64"
                  />
                </label>

                <label class="settings-field wide">
                  宠物系统提示词
                  <textarea
                    v-model="settingsDraft.aiSystemPrompt"
                    rows="4"
                    placeholder="定义宠物聊天时的性格、语气和边界"
                  />
                </label>
              </div>

              <p class="settings-empty">{{ selectedAiProviderPreset().help }}</p>
            </section>

            <section v-show="activeSettingsSection === 'memory'" class="settings-section">
              <h3>宠物记忆</h3>
              <label class="settings-toggle-row">
                <span>
                  <strong>启用宠物记忆</strong>
                  <small>开启后会把最近聊天和提取出的长期记忆保存到本机应用数据目录。</small>
                </span>
                <input v-model="settingsDraft.aiMemoryEnabled" type="checkbox" />
              </label>
              <div class="settings-update-panel">
                <div>
                  <strong>长期记忆列表</strong>
                  <small>用于查看、导入、导出、打开目录、删除或清空宠物已经记住的长期信息。</small>
                </div>
                <div class="settings-update-actions">
                  <button type="button" :disabled="petMemoriesLoading" @click="loadPetMemories">
                    {{ petMemoriesLoading ? '读取中...' : '刷新记忆' }}
                  </button>
                  <button type="button" :disabled="petMemoriesLoading" @click="importPetMemory">
                    导入记忆
                  </button>
                  <button type="button" @click="exportPetMemory">导出记忆</button>
                  <button type="button" @click="openPetMemoryDirectory">打开目录</button>
                  <button
                    type="button"
                    :disabled="petMemoriesLoading || petMemories.length === 0"
                    @click="clearPetMemories"
                  >
                    清空记忆
                  </button>
                </div>
              </div>
              <p v-if="petMemoryStatus" class="settings-empty">{{ petMemoryStatus }}</p>
              <div v-if="petMemories.length > 0" class="settings-memory-list">
                <article v-for="memory in petMemories" :key="memory.id" class="settings-memory-row">
                  <div>
                    <strong>
                      {{ memoryTypeLabel(memory.memoryType) }} · 重要度 {{ memory.importance }}
                    </strong>
                    <p>{{ memory.content }}</p>
                    <small v-if="memory.tags.length > 0">标签：{{ memory.tags.join('、') }}</small>
                  </div>
                  <button type="button" title="删除记忆" @click="deletePetMemory(memory)">删除</button>
                </article>
              </div>
              <p v-else class="settings-empty">
                还没有长期记忆。和宠物对话后，重要偏好或项目信息会自动保存到这里。
              </p>
              <p v-if="petMemoryError" class="form-error">{{ petMemoryError }}</p>
            </section>

            <section v-show="activeSettingsSection === 'window'" class="settings-section">
              <h3>窗口置顶</h3>
              <label class="settings-toggle-row">
                <span>
                  <strong>宠物置顶</strong>
                  <small>宠物窗口保持在其他窗口上方</small>
                </span>
                <input v-model="settingsDraft.petAlwaysOnTop" type="checkbox" />
              </label>
              <label class="settings-toggle-row">
                <span>
                  <strong>抽屉置顶</strong>
                  <small>抽屉窗口打开后保持在其他窗口上方</small>
                </span>
                <input v-model="settingsDraft.drawerAlwaysOnTop" type="checkbox" />
              </label>
            </section>

            <section v-show="activeSettingsSection === 'update'" class="settings-section">
              <h3>软件更新</h3>
              <div class="settings-update-panel">
                <div>
                  <strong>当前版本</strong>
                  <small>
                    v{{ updateInfo?.currentVersion || '读取中' }}
                    <template v-if="updateInfo?.latestVersion">
                      / 最新 v{{ updateInfo.latestVersion }}
                    </template>
                  </small>
                </div>
                <div class="settings-update-actions">
                  <button type="button" :disabled="updateChecking" @click="checkForUpdate">
                    {{ updateChecking ? '检查中...' : '检查更新' }}
                  </button>
                  <button
                    v-if="updateInfo?.updateUrl"
                    type="button"
                    class="primary-button"
                    @click="openUpdatePage"
                  >
                    {{ updateInfo.status === 'available' ? '下载新版' : '打开发布页' }}
                  </button>
                </div>
              </div>
              <p v-if="updateInfo?.assetName" class="settings-empty">
                下载文件：{{ updateInfo.assetName }}
              </p>
              <p v-if="updateInfo" class="settings-empty">{{ updateInfo.message }}</p>
              <p v-if="updateError" class="form-error">{{ updateError }}</p>
            </section>

            <section v-show="activeSettingsSection === 'diagnostics'" class="settings-section">
              <h3>运行诊断</h3>
              <div class="settings-update-panel">
                <div>
                  <strong>当前启动信息</strong>
                  <small>用于确认现在打开的是哪个程序和哪份本机数据。</small>
                </div>
                <div class="settings-update-actions">
                  <button type="button" :disabled="runtimeInfoLoading" @click="loadRuntimeInfo">
                    {{ runtimeInfoLoading ? '读取中...' : '刷新诊断' }}
                  </button>
                </div>
              </div>
              <div v-if="runtimeInfo" class="settings-runtime-list">
                <div class="settings-runtime-row">
                  <strong>程序版本</strong>
                  <code>v{{ runtimeInfo.version }}</code>
                </div>
                <div class="settings-runtime-row">
                  <strong>当前 exe</strong>
                  <code :title="runtimeInfo.executablePath">{{ runtimeInfo.executablePath }}</code>
                </div>
                <div class="settings-runtime-row">
                  <strong>数据目录</strong>
                  <code :title="runtimeInfo.dataDir">{{ runtimeInfo.dataDir }}</code>
                </div>
              </div>
              <p v-if="runtimeInfoError" class="form-error">{{ runtimeInfoError }}</p>
            </section>
          </div>
        </div>

        <p v-if="settingsError" class="form-error">{{ settingsError }}</p>

        <footer>
          <button type="button" @click="settingsModalVisible = false">取消</button>
          <button class="primary-button" type="button" :disabled="settingsSaving" @click="saveSettings">
            {{ settingsSaving ? '保存中...' : '保存设置' }}
          </button>
        </footer>
      </section>
    </div>
  </main>
</template>
