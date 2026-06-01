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
  AiConnectionProfile,
  AiProvider,
  AiConnectionTestResult,
  AppDraft,
  AppItemKind,
  ChatEmojiFrequency,
  Companion,
  CompanionDraft,
  DrawerTheme,
  PetMemory,
  PetMemoryDraft,
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

type ImportantConfirmation = {
  title: string
  message: string
  detail: string
  confirmLabel: string
  variant: 'danger' | 'warning'
}

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
const companions = ref<Companion[]>([])
const currentCompanion = ref<Companion | null>(null)
const companionLoading = ref(false)
const companionStatus = ref('')
const companionError = ref('')
const editingCompanionId = ref<string | null>(null)
const petSkinLoading = ref(false)
const petSkinError = ref('')
const skinImporting = ref(false)
const skinDeleting = ref(false)
const editingPetSkinId = ref<string | null>(null)
const clearedPetAnimationStates = ref<Array<keyof PetAnimationSet>>([])
const quickSearchTags = ref<string[]>([])
const tagDisplayMode = ref<'compact' | 'detailed'>('compact')
const drawerTheme = ref<DrawerTheme>('light')
const displayModeSaving = ref(false)
const settingsSaving = ref(false)
const settingsError = ref('')
const updateChecking = ref(false)
const updateInfo = ref<UpdateCheckResult | null>(null)
const updateError = ref('')
const aiTesting = ref(false)
const aiTestMessage = ref('')
const aiTestError = ref('')
const aiProfileStatus = ref('')
const aiProfileError = ref('')
const petMemories = ref<PetMemory[]>([])
const petMemoriesLoading = ref(false)
const petMemoryStatus = ref('')
const petMemoryError = ref('')
const editingMemoryId = ref<number | null>(null)
const runtimeInfo = ref<RuntimeInfo | null>(null)
const runtimeInfoLoading = ref(false)
const runtimeInfoError = ref('')
const importantConfirmation = ref<ImportantConfirmation | null>(null)
let resolveImportantConfirmation: ((confirmed: boolean) => void) | null = null

const skinDraft = reactive({
  name: '',
  idle: '',
  hover: '',
  dragging: '',
  click: '',
})
const companionDraft = reactive({
  name: '',
  personaPrompt: '',
  systemPrompt: '',
  model: '',
  voiceId: '',
  skinId: 'default',
  favorability: 0,
  intimacy: 0,
  mood: '',
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

const memoryTypeOptions = [
  { value: 'nickname', label: '称呼' },
  { value: 'preference', label: '偏好' },
  { value: 'dislike', label: '不喜欢' },
  { value: 'relationship', label: '关系' },
  { value: 'emotion', label: '情绪' },
  { value: 'habit', label: '习惯' },
  { value: 'life_event', label: '生活事件' },
  { value: 'important_person', label: '重要人物' },
  { value: 'interest', label: '兴趣' },
  { value: 'goal', label: '目标' },
  { value: 'boundary', label: '边界' },
  { value: 'instruction', label: '回复要求' },
  { value: 'other', label: '其他' },
]

const memoryDraft = reactive({
  memoryType: 'preference',
  content: '',
  importance: 5,
  tags: '',
  confidence: 0.8,
})

type SettingsSectionId =
  | 'entries'
  | 'system'
  | 'appearance'
  | 'companion'
  | 'ai'
  | 'memory'
  | 'window'
  | 'update'
  | 'diagnostics'
  | 'about'

const settingsSections: Array<{ id: SettingsSectionId; label: string; description: string }> = [
  { id: 'entries', label: '入口管理', description: '分类和快捷搜索' },
  { id: 'system', label: '系统', description: '自启和常用规则' },
  { id: 'appearance', label: '外观', description: '界面主题风格' },
  { id: 'companion', label: '伴侣', description: '角色与切换' },
  { id: 'ai', label: 'AI 接口', description: '宠物聊天 API' },
  { id: 'memory', label: '记忆', description: '长期记忆管理' },
  { id: 'window', label: '窗口', description: '置顶行为' },
  { id: 'update', label: '更新', description: '版本检查' },
  { id: 'diagnostics', label: '诊断', description: '运行路径和数据' },
  { id: 'about', label: '关于', description: '开源许可' },
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

const maxAiProfileCount = 20
const themeOptions: Array<{
  id: DrawerTheme
  name: string
  description: string
}> = [
  {
    id: 'light',
    name: '清爽默认',
    description: '简洁蓝灰色面板，保留当前应用风格。',
  },
  {
    id: 'animal-island',
    name: '动物岛',
    description: '暖色纸感底色，搭配薄荷绿、果橙和圆润按钮。',
  },
]

const chatEmojiFrequencyOptions: Array<{
  value: ChatEmojiFrequency
  label: string
  description: string
}> = [
  { value: 'none', label: '不主动使用', description: '回复里不主动加入 emoji，除非用户明确要求。' },
  { value: 'low', label: '少量', description: '偶尔使用，适合更克制的陪伴语气。' },
  { value: 'normal', label: '自然', description: '只在能增强语气时使用，默认每条 0 到 1 个。' },
  { value: 'high', label: '较多', description: '更活泼亲近，但仍避免堆叠刷屏。' },
]

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
  drawerTheme: 'light' as DrawerTheme,
  chatTypewriterEnabled: true,
  chatNarrationEnabled: false,
  aiEnabled: false,
  aiMemoryEnabled: true,
  aiShortMemorySummaryEnabled: true,
  aiShortMemoryRecentTurns: 10,
  aiShortMemoryCompressionTriggerTurns: 12,
  aiProvider: 'openai' as AiProvider,
  aiApiKey: '',
  aiBaseUrl: 'https://api.openai.com/v1',
  aiModel: 'gpt-4o-mini',
  aiSystemPrompt: '请遵循当前伴侣档案中的身份与表达方式，尊重用户隐私，回复自然且清晰。',
  aiTemperature: 0.7,
  aiMaxTokens: 800,
  aiEmojiFrequency: 'normal' as ChatEmojiFrequency,
  aiActiveProfileId: '',
  aiProfiles: [] as AiConnectionProfile[],
  newAiProfileLabel: '',
})

const selectedAiProfile = computed(() =>
  settingsDraft.aiProfiles.find((profile) => profile.id === settingsDraft.aiActiveProfileId),
)
const isEditingPetSkin = computed(() => Boolean(editingPetSkinId.value))
const canManageSelectedPetSkin = computed(
  () =>
    Boolean(
      selectedPetSkin.value &&
        !selectedPetSkin.value.builtin &&
        selectedPetSkin.value.id !== 'legacy_custom',
    ),
)
const previewDrawerTheme = computed(() =>
  settingsModalVisible.value ? settingsDraft.drawerTheme : drawerTheme.value,
)

onMounted(() => {
  void store.loadApps()
  void loadPetSkins()
  void loadCompanions()
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
  aiProfileError.value = ''
  aiProfileStatus.value = ''
  companionError.value = ''
  companionStatus.value = ''
  petMemoryError.value = ''
  runtimeInfoError.value = ''
  activeSettingsSection.value = 'entries'
  await Promise.all([
    loadDrawerSettings(),
    loadCompanions(),
    loadPetMemories(),
    loadRuntimeInfo(),
  ])
  checkForUpdate()
}

function applyDrawerConfig(config: PetDrawerConfig) {
  quickSearchTags.value = config.drawer.quickSearchTags ?? []
  tagDisplayMode.value = normalizeTagDisplayMode(config.drawer.tagDisplayMode)
  drawerTheme.value = normalizeDrawerTheme(config.drawer.theme)
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
  settingsDraft.drawerTheme = normalizeDrawerTheme(config.drawer.theme)
  settingsDraft.chatTypewriterEnabled = config.drawer.chatTypewriterEnabled ?? true
  settingsDraft.chatNarrationEnabled = config.drawer.chatNarrationEnabled ?? false
  settingsDraft.petAlwaysOnTop = config.pet.alwaysOnTop
  settingsDraft.drawerAlwaysOnTop = config.drawer.alwaysOnTop
  settingsDraft.startOnBoot = Boolean(config.system?.startOnBoot)
  settingsDraft.autoFavoriteEnabled = config.system?.autoFavoriteEnabled ?? true
  settingsDraft.aiEnabled = Boolean(config.ai?.enabled)
  settingsDraft.aiMemoryEnabled = config.ai?.memoryEnabled ?? true
  settingsDraft.aiShortMemorySummaryEnabled = config.ai?.shortMemorySummaryEnabled ?? true
  settingsDraft.aiShortMemoryRecentTurns = clampInteger(config.ai?.shortMemoryRecentTurns ?? 10, 2, 40)
  settingsDraft.aiShortMemoryCompressionTriggerTurns = clampInteger(
    config.ai?.shortMemoryCompressionTriggerTurns ?? 12,
    4,
    80,
  )
  settingsDraft.aiProvider = normalizeAiProvider(config.ai?.provider)
  settingsDraft.aiApiKey = config.ai?.apiKey ?? ''
  settingsDraft.aiBaseUrl = config.ai?.baseUrl ?? selectedAiProviderPreset().baseUrl
  settingsDraft.aiModel = config.ai?.model ?? selectedAiProviderPreset().model
  settingsDraft.aiSystemPrompt =
    config.ai?.systemPrompt ||
    '请遵循当前伴侣档案中的身份与表达方式，尊重用户隐私，回复自然且清晰。'
  settingsDraft.aiTemperature = config.ai?.temperature ?? 0.7
  settingsDraft.aiMaxTokens = config.ai?.maxTokens ?? 800
  settingsDraft.aiEmojiFrequency = normalizeChatEmojiFrequency(config.ai?.emojiFrequency)
  settingsDraft.aiProfiles = (config.ai?.profiles ?? []).map((profile) => ({
    id: profile.id,
    label: profile.label,
    provider: normalizeAiProvider(profile.provider),
    apiKey: profile.apiKey ?? '',
    baseUrl: profile.baseUrl ?? '',
    model: profile.model ?? '',
  }))
  settingsDraft.aiActiveProfileId = settingsDraft.aiProfiles.some(
    (profile) => profile.id === config.ai?.activeProfileId,
  )
    ? (config.ai?.activeProfileId ?? '')
    : ''
  settingsDraft.newAiProfileLabel = ''
}

function normalizeTagDisplayMode(value?: string | null): 'compact' | 'detailed' {
  return value === 'detailed' ? 'detailed' : 'compact'
}

function normalizeDrawerTheme(value?: string | null): DrawerTheme {
  return value === 'animal-island' ? 'animal-island' : 'light'
}

function normalizeChatEmojiFrequency(value?: string | null): ChatEmojiFrequency {
  return chatEmojiFrequencyOptions.some((option) => option.value === value)
    ? (value as ChatEmojiFrequency)
    : 'normal'
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

function makeAiProfileId() {
  return `api_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`
}

function connectionProfileFromDraft(id: string, label: string): AiConnectionProfile {
  return {
    id,
    label,
    provider: settingsDraft.aiProvider,
    apiKey: settingsDraft.aiApiKey.trim(),
    baseUrl: settingsDraft.aiBaseUrl.trim(),
    model: settingsDraft.aiModel.trim(),
  }
}

function selectAiProfile(profile: AiConnectionProfile) {
  settingsDraft.aiActiveProfileId = profile.id
  settingsDraft.aiProvider = normalizeAiProvider(profile.provider)
  settingsDraft.aiApiKey = profile.apiKey
  settingsDraft.aiBaseUrl = profile.baseUrl
  settingsDraft.aiModel = profile.model
  aiProfileError.value = ''
  aiProfileStatus.value = `已切换到「${profile.label}」，保存设置后宠物聊天将使用该配置。`
}

function selectAiProfileFromList(event: Event) {
  const id = (event.target as HTMLSelectElement).value
  const profile = settingsDraft.aiProfiles.find((item) => item.id === id)
  if (profile) {
    selectAiProfile(profile)
    return
  }
  settingsDraft.aiActiveProfileId = ''
  aiProfileStatus.value = '当前表单未绑定保存的 API 配置标签。'
}

function addAiProfile() {
  const label = settingsDraft.newAiProfileLabel.trim()
  aiProfileError.value = ''
  aiProfileStatus.value = ''

  if (!label) {
    aiProfileError.value = '请输入 API 配置标签名称。'
    return
  }

  if (label.length > 40) {
    aiProfileError.value = 'API 配置标签名称不能超过 40 个字符。'
    return
  }

  if (settingsDraft.aiProfiles.length >= maxAiProfileCount) {
    aiProfileError.value = `最多保存 ${maxAiProfileCount} 个 API 配置标签。`
    return
  }

  if (settingsDraft.aiProfiles.some((profile) => profile.label.toLowerCase() === label.toLowerCase())) {
    aiProfileError.value = 'API 配置标签名称已存在。'
    return
  }

  const profile = connectionProfileFromDraft(makeAiProfileId(), label)
  settingsDraft.aiProfiles.push(profile)
  settingsDraft.newAiProfileLabel = ''
  settingsDraft.aiActiveProfileId = profile.id
  aiProfileStatus.value = `已添加「${label}」，点击“保存设置”后写入本机配置。`
}

function updateSelectedAiProfile() {
  const profile = selectedAiProfile.value
  aiProfileError.value = ''
  aiProfileStatus.value = ''
  if (!profile) {
    aiProfileError.value = '请先选择一个已有标签，或添加新的 API 配置标签。'
    return
  }

  Object.assign(profile, connectionProfileFromDraft(profile.id, profile.label))
  aiProfileStatus.value = `已更新「${profile.label}」，点击“保存设置”后生效。`
}

async function removeAiProfile(profile: AiConnectionProfile) {
  if (
    !(await requestImportantConfirmation({
      title: '删除 API 配置标签',
      message: `确认删除 API 配置标签「${profile.label}」？`,
      detail: '该标签将从列表中移除；若当前表单仍保留同一连接，保存后它仍会作为当前聊天连接保存在本机。',
      confirmLabel: '删除标签',
      variant: 'danger',
    }))
  ) {
    return
  }

  settingsDraft.aiProfiles = settingsDraft.aiProfiles.filter((item) => item.id !== profile.id)
  if (settingsDraft.aiActiveProfileId === profile.id) {
    settingsDraft.aiActiveProfileId = ''
  }
  aiProfileError.value = ''
  aiProfileStatus.value = `已移除「${profile.label}」，点击“保存设置”后写入本机配置。`
}

function removeSelectedAiProfile() {
  if (selectedAiProfile.value) {
    void removeAiProfile(selectedAiProfile.value)
  }
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
    void emitEvent('ui-theme-changed', config.drawer.theme)
    void emitEvent('ui-chat-display-changed', config.drawer.chatTypewriterEnabled ?? true)
    void emitEvent('ui-chat-narration-changed', config.drawer.chatNarrationEnabled ?? false)
    settingsModalVisible.value = false
  } catch (err) {
    settingsError.value = String(err)
  } finally {
    settingsSaving.value = false
  }
}

function buildDrawerPreferences(tagMode: 'compact' | 'detailed') {
  const profiles = settingsDraft.aiProfiles.map((profile) =>
    profile.id === settingsDraft.aiActiveProfileId
      ? connectionProfileFromDraft(profile.id, profile.label)
      : { ...profile },
  )

  return {
    categories: [...settingsDraft.categories],
    quickSearchTags: [...settingsDraft.quickSearchTags],
    tagDisplayMode: tagMode,
    theme: settingsDraft.drawerTheme,
    chatTypewriterEnabled: settingsDraft.chatTypewriterEnabled,
    chatNarrationEnabled: settingsDraft.chatNarrationEnabled,
    petAlwaysOnTop: settingsDraft.petAlwaysOnTop,
    drawerAlwaysOnTop: settingsDraft.drawerAlwaysOnTop,
    startOnBoot: settingsDraft.startOnBoot,
    autoFavoriteEnabled: settingsDraft.autoFavoriteEnabled,
    ai: {
      enabled: settingsDraft.aiEnabled,
      memoryEnabled: settingsDraft.aiMemoryEnabled,
      shortMemorySummaryEnabled: settingsDraft.aiShortMemorySummaryEnabled,
      shortMemoryRecentTurns: clampInteger(settingsDraft.aiShortMemoryRecentTurns, 2, 40),
      shortMemoryCompressionTriggerTurns: clampInteger(
        settingsDraft.aiShortMemoryCompressionTriggerTurns,
        4,
        80,
      ),
      provider: settingsDraft.aiProvider,
      apiKey: settingsDraft.aiApiKey,
      baseUrl: settingsDraft.aiBaseUrl,
      model: settingsDraft.aiModel,
      systemPrompt: settingsDraft.aiSystemPrompt,
      temperature: safeNumber(settingsDraft.aiTemperature, 0.7),
      maxTokens: safeInteger(settingsDraft.aiMaxTokens, 800),
      emojiFrequency: settingsDraft.aiEmojiFrequency,
      activeProfileId: settingsDraft.aiActiveProfileId,
      profiles,
    },
  }
}

function safeNumber(value: number, fallback: number) {
  return Number.isFinite(value) ? value : fallback
}

function safeInteger(value: number, fallback: number) {
  return Number.isFinite(value) ? Math.round(value) : fallback
}

function clampInteger(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, safeInteger(value, min)))
}

function requestImportantConfirmation(confirmation: ImportantConfirmation) {
  resolveImportantConfirmation?.(false)
  importantConfirmation.value = confirmation

  return new Promise<boolean>((resolve) => {
    resolveImportantConfirmation = resolve
  })
}

function settleImportantConfirmation(confirmed: boolean) {
  const resolve = resolveImportantConfirmation
  importantConfirmation.value = null
  resolveImportantConfirmation = null
  resolve?.(confirmed)
}

function confirmAdministratorLaunch(appName: string) {
  return requestImportantConfirmation({
    title: '启用管理员启动',
    message: `确认允许「${appName}」以管理员身份启动？`,
    detail: '之后启动该软件时，系统可能弹出 Windows 管理员权限授权提示。',
    confirmLabel: '允许启用',
    variant: 'warning',
  })
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

function resetMemoryDraft() {
  editingMemoryId.value = null
  memoryDraft.memoryType = 'preference'
  memoryDraft.content = ''
  memoryDraft.importance = 5
  memoryDraft.tags = ''
  memoryDraft.confidence = 0.8
}

function buildPetMemoryDraft(): PetMemoryDraft {
  return {
    memoryType: memoryDraft.memoryType,
    content: memoryDraft.content.trim(),
    importance: safeInteger(memoryDraft.importance, 5),
    tags: parseTags(memoryDraft.tags),
    confidence: safeNumber(memoryDraft.confidence, 0.8),
  }
}

async function savePetMemoryDraft() {
  const draft = buildPetMemoryDraft()
  if (!draft.content) {
    petMemoryError.value = '请先填写记忆内容。'
    return
  }

  petMemoriesLoading.value = true
  petMemoryStatus.value = ''
  petMemoryError.value = ''

  try {
    let successMessage = ''
    if (editingMemoryId.value) {
      await invoke<PetMemory>('update_pet_memory', {
        memoryId: editingMemoryId.value,
        draft,
      })
      successMessage = '记忆已更新。'
    } else {
      await invoke<PetMemory>('add_pet_memory', { draft })
      successMessage = '记忆已添加。'
    }
    resetMemoryDraft()
    await loadPetMemories()
    petMemoryStatus.value = successMessage
  } catch (err) {
    petMemoryError.value = String(err)
  } finally {
    petMemoriesLoading.value = false
  }
}

function editPetMemory(memory: PetMemory) {
  editingMemoryId.value = memory.id
  memoryDraft.memoryType = memory.memoryType
  memoryDraft.content = memory.content
  memoryDraft.importance = memory.importance
  memoryDraft.tags = memory.tags.join('、')
  memoryDraft.confidence = memory.confidence ?? 0.8
}

async function deletePetMemory(memory: PetMemory) {
  if (
    !(await requestImportantConfirmation({
      title: '删除长期记忆',
      message: '确认删除这条宠物长期记忆？',
      detail: '该信息将从本机数据库中永久删除，并且不会继续参与宠物回复。',
      confirmLabel: '永久删除',
      variant: 'danger',
    }))
  ) {
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
    !(await requestImportantConfirmation({
      title: '清空长期记忆',
      message: '确认清空当前伴侣的长期记忆？聊天记录不会被删除。',
      detail: '当前伴侣的长期记忆将从本机数据库永久删除，且此操作无法撤销。',
      confirmLabel: '永久清空',
      variant: 'danger',
    }))
  ) {
    return
  }

  try {
    petMemoryStatus.value = ''
    petMemoryError.value = ''
    await invoke('clear_pet_memories')
    await loadPetMemories()
    petMemoryStatus.value = '已清空当前伴侣的长期记忆。'
  } catch (err) {
    petMemoryError.value = String(err)
  }
}

async function clearPetMemoryMessages() {
  if (
    !(await requestImportantConfirmation({
      title: '清空短期聊天记录',
      message: '确认清空当前伴侣的聊天记录？长期记忆不会被删除。',
      detail: '清空后当前伴侣不会再读取这些最近对话作为上下文，且无法撤销。',
      confirmLabel: '永久清空',
      variant: 'danger',
    }))
  ) {
    return
  }

  petMemoriesLoading.value = true
  try {
    petMemoryStatus.value = ''
    petMemoryError.value = ''
    await invoke('clear_pet_memory_messages')
    await emitEvent('companion-changed', currentCompanion.value?.id ?? 'default')
    petMemoryStatus.value = '已清空当前伴侣的聊天记录。'
  } catch (err) {
    petMemoryError.value = String(err)
  } finally {
    petMemoriesLoading.value = false
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

  if (
    !(await requestImportantConfirmation({
      title: '覆盖本机记忆',
      message: '导入会替换当前伴侣的长期记忆和聊天记录，确认继续？',
      detail: '当前伴侣的本机记忆将被所选 JSON 文件中的内容覆盖，原数据无法从应用恢复。',
      confirmLabel: '覆盖并导入',
      variant: 'danger',
    }))
  ) {
    return
  }

  petMemoriesLoading.value = true
  petMemoryStatus.value = ''
  petMemoryError.value = ''

  try {
    petMemories.value = await invoke<PetMemory[]>('import_pet_memory', { path: selected })
    await emitEvent('companion-changed', currentCompanion.value?.id ?? 'default')
    petMemoryStatus.value = '记忆导入完成。'
  } catch (err) {
    petMemoryError.value = String(err)
  } finally {
    petMemoriesLoading.value = false
  }
}

async function exportPetMemory() {
  if (
    !(await requestImportantConfirmation({
      title: '导出隐私数据',
      message: '确认导出宠物记忆？导出的 JSON 可能包含私人聊天和长期记忆。',
      detail: '请只保存到你信任的位置，并避免将文件分享给无关人员。',
      confirmLabel: '继续导出',
      variant: 'warning',
    }))
  ) {
    return
  }

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
    nickname: '称呼',
    preference: '偏好',
    dislike: '不喜欢',
    relationship: '关系',
    emotion: '情绪',
    habit: '习惯',
    life_event: '生活事件',
    important_person: '重要人物',
    interest: '兴趣',
    goal: '目标',
    boundary: '边界',
    instruction: '回复要求',
    short_term_summary: '短期摘要',
    other: '其他',
    project: '项目',
    event: '事件',
    profile: '画像',
  }

  return labels[type] ?? type
}

function confidencePercent(value?: number) {
  return `${Math.round((value ?? 0.8) * 100)}%`
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

function fillCompanionDraft(companion?: Companion | null) {
  editingCompanionId.value = companion?.id ?? null
  companionDraft.name = companion?.name ?? ''
  companionDraft.personaPrompt = companion?.personaPrompt ?? ''
  companionDraft.systemPrompt = companion?.systemPrompt ?? ''
  companionDraft.model = companion?.model ?? ''
  companionDraft.voiceId = companion?.voiceId ?? ''
  companionDraft.skinId = companion?.skinId ?? currentPetSkin.value?.id ?? 'default'
  companionDraft.favorability = companion?.relationshipState.favorability ?? 0
  companionDraft.intimacy = companion?.relationshipState.intimacy ?? 0
  companionDraft.mood = companion?.relationshipState.mood ?? ''
}

async function loadCompanions() {
  companionLoading.value = true
  companionError.value = ''

  try {
    const [items, current] = await Promise.all([
      invoke<Companion[]>('list_companions'),
      invoke<Companion>('get_current_companion'),
    ])
    companions.value = items
    currentCompanion.value = current
    if (!editingCompanionId.value) {
      fillCompanionDraft(current)
    }
  } catch (err) {
    companionError.value = String(err)
  } finally {
    companionLoading.value = false
  }
}

function startNewCompanion() {
  fillCompanionDraft()
  companionDraft.skinId = currentPetSkin.value?.id ?? 'default'
  companionDraft.personaPrompt = '你是一个有独特性格的桌面伴侣，请自然、真诚地陪伴用户交流。'
}

function editCompanion(companion: Companion) {
  fillCompanionDraft(companion)
  companionStatus.value = `正在编辑「${companion.name}」的档案。`
}

function companionSkinLabel(companion: Companion) {
  return petSkins.value.find((skin) => skin.id === companion.skinId)?.name ?? '默认形象'
}

function buildCompanionDraft(): CompanionDraft {
  return {
    ...(editingCompanionId.value ? { id: editingCompanionId.value } : {}),
    name: companionDraft.name.trim(),
    personaPrompt: companionDraft.personaPrompt.trim(),
    systemPrompt: companionDraft.systemPrompt.trim(),
    model: companionDraft.model.trim(),
    voiceId: companionDraft.voiceId.trim(),
    skinId: companionDraft.skinId || 'default',
    relationshipState: {
      favorability: safeInteger(companionDraft.favorability, 0),
      intimacy: safeInteger(companionDraft.intimacy, 0),
      mood: companionDraft.mood.trim(),
    },
  }
}

async function saveCompanion() {
  if (!companionDraft.name.trim() || !companionDraft.personaPrompt.trim()) {
    companionError.value = '伴侣名称和角色设定不能为空。'
    return
  }

  companionLoading.value = true
  companionError.value = ''
  companionStatus.value = ''
  try {
    const isNewCompanion = !editingCompanionId.value
    const saved = await invoke<Companion>('upsert_companion', { draft: buildCompanionDraft() })
    editingCompanionId.value = saved.id
    if (isNewCompanion || currentCompanion.value?.id === saved.id) {
      const current = await invoke<Companion>('switch_companion', { companionId: saved.id })
      currentCompanion.value = current
      await emitEvent('pet-skin-updated', current.skinId)
      await emitEvent('companion-changed', current.id)
      await Promise.all([loadPetSkins(), loadPetMemories()])
    }
    await loadCompanions()
    fillCompanionDraft(companions.value.find((companion) => companion.id === saved.id) ?? saved)
    companionStatus.value = `伴侣「${saved.name}」已保存到本机。`
  } catch (err) {
    companionError.value = String(err)
  } finally {
    companionLoading.value = false
  }
}

async function activateCompanion(companion: Companion) {
  companionLoading.value = true
  companionError.value = ''
  companionStatus.value = ''
  try {
    const current = await invoke<Companion>('switch_companion', { companionId: companion.id })
    currentCompanion.value = current
    fillCompanionDraft(current)
    await emitEvent('pet-skin-updated', current.skinId)
    await emitEvent('companion-changed', current.id)
    await Promise.all([loadPetSkins(), loadPetMemories(), loadCompanions()])
    companionStatus.value = `已切换到「${current.name}」，聊天、记忆与形象已同步。`
  } catch (err) {
    companionError.value = String(err)
  } finally {
    companionLoading.value = false
  }
}

function selectCompanionFromList(event: Event) {
  const id = (event.target as HTMLSelectElement).value
  const companion = companions.value.find((item) => item.id === id)
  if (companion && companion.id !== currentCompanion.value?.id) {
    void activateCompanion(companion)
  }
}

async function removeCompanion(companion: Companion) {
  if (
    !(await requestImportantConfirmation({
      title: '删除伴侣档案',
      message: `确认删除伴侣「${companion.name}」？`,
      detail: '该伴侣的聊天记录与长期记忆将从本机数据库永久删除，无法恢复。',
      confirmLabel: '永久删除',
      variant: 'danger',
    }))
  ) {
    return
  }

  companionLoading.value = true
  companionError.value = ''
  try {
    const current = await invoke<Companion>('delete_companion', { companionId: companion.id })
    currentCompanion.value = current
    fillCompanionDraft(current)
    await emitEvent('pet-skin-updated', current.skinId)
    await emitEvent('companion-changed', current.id)
    await Promise.all([loadPetSkins(), loadPetMemories(), loadCompanions()])
    companionStatus.value = `已删除「${companion.name}」。`
  } catch (err) {
    companionError.value = String(err)
  } finally {
    companionLoading.value = false
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
  resetPetSkinDraft()
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
  if (isEditingPetSkin.value && editingPetSkinId.value !== skin.id) {
    resetPetSkinDraft()
  }
  selectedPetSkin.value = skin

  try {
    currentPetSkin.value = await invoke<PetSkinSummary>('set_pet_skin', { skinId: skin.id })
    selectedPetSkin.value = currentPetSkin.value
    await emitEvent('pet-skin-updated', skin.id)
    await Promise.all([loadPetSkins(), loadCompanions()])
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
  clearedPetAnimationStates.value = clearedPetAnimationStates.value.filter((item) => item !== state)

  if (state === 'idle' && !skinDraft.name.trim()) {
    skinDraft.name = appNameFromPath(selected)
  }
}

function clearPetAnimation(state: keyof PetAnimationSet) {
  skinDraft[state] = ''
  if (
    isEditingPetSkin.value &&
    state !== 'idle' &&
    selectedPetSkin.value?.id === editingPetSkinId.value &&
    selectedPetSkin.value.animations[state]
  ) {
    if (!clearedPetAnimationStates.value.includes(state)) {
      clearedPetAnimationStates.value.push(state)
    }
  }
}

function resetPetSkinDraft() {
  editingPetSkinId.value = null
  clearedPetAnimationStates.value = []
  skinDraft.name = ''
  skinDraft.idle = ''
  skinDraft.hover = ''
  skinDraft.dragging = ''
  skinDraft.click = ''
}

function editSelectedPetSkin() {
  const skin = selectedPetSkin.value
  if (!skin || !canManageSelectedPetSkin.value) {
    petSkinError.value =
      skin?.id === 'legacy_custom'
        ? '旧版单图形象请重新导入后再编辑动画'
        : '内置宠物形象不能编辑'
    return
  }

  editingPetSkinId.value = skin.id
  clearedPetAnimationStates.value = []
  skinDraft.name = skin.name
  skinDraft.idle = ''
  skinDraft.hover = ''
  skinDraft.dragging = ''
  skinDraft.click = ''
  petSkinError.value = ''
}

function isPetAnimationCleared(state: keyof PetAnimationSet) {
  return clearedPetAnimationStates.value.includes(state)
}

function restorePetAnimation(state: keyof PetAnimationSet) {
  clearedPetAnimationStates.value = clearedPetAnimationStates.value.filter((item) => item !== state)
}

function canClearPetAnimation(state: keyof PetAnimationSet) {
  if (skinDraft[state]) {
    return true
  }

  return (
    isEditingPetSkin.value &&
    state !== 'idle' &&
    Boolean(selectedPetSkin.value?.animations[state]) &&
    !isPetAnimationCleared(state)
  )
}

function petAnimationDraftLabel(state: keyof PetAnimationSet) {
  if (skinDraft[state]) {
    return skinDraft[state]
  }

  if (!isEditingPetSkin.value) {
    return '未选择图片'
  }

  if (isPetAnimationCleared(state)) {
    return '保存后移除此动画，改用待机动画'
  }

  return state === 'idle' ? '未选择替换素材，将保留当前待机动画' : '未选择替换素材，将保留当前设置'
}

async function savePetSkin() {
  if (!isEditingPetSkin.value && !skinDraft.idle) {
    petSkinError.value = '导入宠物至少需要选择待机动画'
    return
  }

  skinImporting.value = true
  petSkinError.value = ''

  try {
    const editingId = editingPetSkinId.value
    const animations = {
      idle: skinDraft.idle || null,
      hover: skinDraft.hover || null,
      dragging: skinDraft.dragging || null,
      click: skinDraft.click || null,
    }
    const saved = editingId
      ? await invoke<PetSkinSummary>('update_pet_skin', {
          skinId: editingId,
          name: skinDraft.name.trim(),
          animations,
          clearedStates: clearedPetAnimationStates.value,
        })
      : await invoke<PetSkinSummary>('import_pet_skin', {
          name: skinDraft.name.trim() || '自定义宠物',
          animations,
        })

    const refreshCurrentSkin = !editingId || currentPetSkin.value?.id === saved.id
    if (refreshCurrentSkin) {
      currentPetSkin.value = saved
      await emitEvent('pet-skin-updated', saved.id)
    }
    selectedPetSkin.value = saved
    resetPetSkinDraft()
    await Promise.all([loadPetSkins(), loadCompanions()])
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

  const confirmed = await requestImportantConfirmation({
    title: '删除宠物形象',
    message: `确认删除宠物形象「${skin.name}」？`,
    detail: '本机保存的动画文件将被删除；之后需要重新导入素材才能再次使用该形象。',
    confirmLabel: '永久删除',
    variant: 'danger',
  })
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
    await Promise.all([loadPetSkins(), loadCompanions()])
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
    await Promise.all([loadPetSkins(), loadCompanions()])
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

  const storedApp = form.id ? store.apps.find((app) => app.id === form.id) : undefined
  if (
    form.itemKind === 'app' &&
    form.runAsAdmin &&
    (!storedApp || !storedApp.runAsAdmin) &&
    !(await confirmAdministratorLaunch(itemName))
  ) {
    form.runAsAdmin = Boolean(storedApp?.runAsAdmin)
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
  if (
    !(await requestImportantConfirmation({
      title: '删除快捷入口',
      message: `确认删除快捷入口「${app.name}」？`,
      detail: '该入口会从抽屉列表中移除；如需恢复，需要重新添加。',
      confirmLabel: '确认删除',
      variant: 'danger',
    }))
  ) {
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
  if (runAsAdmin && !app.runAsAdmin && !(await confirmAdministratorLaunch(app.name))) {
    return
  }

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
  <main class="drawer-window" :class="`theme-${previewDrawerTheme}`">
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
            <h2>当前伴侣</h2>
            <p>{{ currentCompanion?.name || '默认伴侣' }}</p>
            <small>{{ currentPetSkin?.name || '内置默认形象' }}</small>
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
            <p>选择现有形象，或导入、编辑本机多状态宠物。</p>
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
                v-if="canManageSelectedPetSkin"
                type="button"
                class="secondary-button skin-edit-button"
                @click="editSelectedPetSkin"
              >
                编辑动画
              </button>
              <button
                v-if="canManageSelectedPetSkin"
                type="button"
                class="skin-delete-button"
                :disabled="skinDeleting"
                @click="deleteSelectedPetSkin"
              >
                {{ skinDeleting ? '删除中...' : '删除形象' }}
              </button>
              <small v-else-if="selectedPetSkin.builtin">内置默认凯蒂会随程序保留，不能从这里编辑或删除。</small>
              <small v-else>旧版单图形象请在下方重新导入为多状态宠物后编辑。</small>
            </div>
          </aside>
        </div>

        <section class="skin-import-panel">
          <h3>{{ isEditingPetSkin ? '编辑宠物形象' : '导入宠物' }}</h3>
          <p v-if="isEditingPetSkin" class="settings-empty">
            仅选择需要替换的素材；未选择的动画会继续保留当前配置。
          </p>
          <label>
            宠物名称
            <input v-model="skinDraft.name" placeholder="例如：小猫助手" autocomplete="off" />
          </label>

          <div class="animation-picker-grid">
            <div v-for="field in animationFields" :key="field.key" class="animation-picker">
              <div>
                <strong>{{ field.label }}</strong>
                <span>
                  {{
                    isEditingPetSkin
                      ? field.required
                        ? '可替换，未选择时保留当前动画'
                        : '可替换或移除，未选择时保留当前设置'
                      : field.required
                        ? '必填'
                        : '可选，未设置时使用待机动画'
                  }}
                </span>
                <p :title="skinDraft[field.key]">
                  {{ petAnimationDraftLabel(field.key) }}
                </p>
              </div>
              <div class="animation-picker-actions">
                <button type="button" @click="pickPetAnimation(field.key)">选择</button>
                <button
                  v-if="canClearPetAnimation(field.key)"
                  type="button"
                  @click="clearPetAnimation(field.key)"
                >
                  {{ isEditingPetSkin && field.key !== 'idle' && selectedPetSkin?.animations[field.key] ? '移除' : '清除' }}
                </button>
                <button
                  v-if="isPetAnimationCleared(field.key)"
                  type="button"
                  @click="restorePetAnimation(field.key)"
                >
                  保留原动画
                </button>
              </div>
            </div>
          </div>

          <footer>
            <button type="button" @click="isEditingPetSkin ? resetPetSkinDraft() : (petSkinModalVisible = false)">
              {{ isEditingPetSkin ? '取消编辑' : '关闭' }}
            </button>
            <button class="primary-button" type="button" :disabled="skinImporting" @click="savePetSkin">
              {{ skinImporting ? (isEditingPetSkin ? '保存中...' : '导入中...') : (isEditingPetSkin ? '保存修改' : '导入并使用') }}
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
            <p>管理入口、伴侣档案、AI 接口、宠物记忆、软件更新、运行诊断和开源许可。</p>
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

            <section v-show="activeSettingsSection === 'appearance'" class="settings-section">
              <h3>界面主题</h3>
              <p class="settings-empty">
                选择后可立即预览，点击“保存设置”后将应用到抽屉、对话窗口与右键菜单。
              </p>
              <div class="theme-choice-grid" role="radiogroup" aria-label="界面主题">
                <button
                  v-for="option in themeOptions"
                  :key="option.id"
                  class="theme-choice-card"
                  :class="{ active: settingsDraft.drawerTheme === option.id, [`preview-${option.id}`]: true }"
                  type="button"
                  role="radio"
                  :aria-checked="settingsDraft.drawerTheme === option.id"
                  @click="settingsDraft.drawerTheme = option.id"
                >
                  <span class="theme-choice-preview">
                    <i></i>
                    <i></i>
                    <i></i>
                  </span>
                  <strong>{{ option.name }}</strong>
                  <small>{{ option.description }}</small>
                </button>
              </div>
              <label class="settings-toggle-row">
                <span>
                  <strong>逐字显示聊天回复</strong>
                  <small>关闭后宠物回复会立即完整显示；开启时长回复会自动加速完成展示。</small>
                </span>
                <input v-model="settingsDraft.chatTypewriterEnabled" type="checkbox" />
              </label>
              <label class="settings-toggle-row">
                <span>
                  <strong>开启旁白功能</strong>
                  <small>开启后，括号、【】或 *动作* 中的内容会作为旁白显示；关闭后聊天只显示双方对话。</small>
                </span>
                <input v-model="settingsDraft.chatNarrationEnabled" type="checkbox" />
              </label>
            </section>

            <section v-show="activeSettingsSection === 'companion'" class="settings-section">
              <label class="settings-field wide">
                表情使用频率
                <select v-model="settingsDraft.aiEmojiFrequency">
                  <option
                    v-for="option in chatEmojiFrequencyOptions"
                    :key="option.value"
                    :value="option.value"
                  >
                    {{ option.label }} - {{ option.description }}
                  </option>
                </select>
              </label>
              <div class="settings-companion-heading">
                <div>
                  <h3>伴侣切换</h3>
                  <p class="settings-empty">
                    每位伴侣拥有独立人设、聊天记录、长期记忆和形象绑定。
                  </p>
                </div>
                <button class="secondary-button" type="button" @click="startNewCompanion">
                  添加伴侣
                </button>
              </div>
              <div v-if="companionLoading && companions.length === 0" class="settings-empty">
                正在读取伴侣档案...
              </div>
              <div v-else class="settings-companion-picker">
                <label class="settings-field wide">
                  伴侣标签
                  <select
                    :value="currentCompanion?.id || ''"
                    :disabled="companionLoading"
                    @change="selectCompanionFromList"
                  >
                    <option v-for="companion in companions" :key="companion.id" :value="companion.id">
                      {{ companion.name }} / {{ companionSkinLabel(companion) }} /
                      {{ companion.model || '沿用全局模型' }}
                    </option>
                  </select>
                </label>
                <div v-if="currentCompanion" class="settings-companion-current">
                  <small>
                    当前使用：{{ currentCompanion.name }} / {{ companionSkinLabel(currentCompanion) }} /
                    {{ currentCompanion.model || '沿用全局模型' }}
                  </small>
                  <div class="settings-companion-current-actions">
                    <button type="button" @click="editCompanion(currentCompanion)">编辑当前</button>
                    <button
                      v-if="currentCompanion.id !== 'default'"
                      type="button"
                      class="danger-button"
                      @click="removeCompanion(currentCompanion)"
                    >
                      删除当前
                    </button>
                  </div>
                </div>
              </div>

              <h3>{{ editingCompanionId ? '编辑伴侣档案' : '添加伴侣档案' }}</h3>
              <div class="settings-form-grid">
                <label class="settings-field">
                  名称
                  <input v-model="companionDraft.name" maxlength="40" placeholder="例如：凯蒂" />
                </label>
                <label class="settings-field">
                  绑定形象
                  <select v-model="companionDraft.skinId">
                    <option v-for="skin in petSkins" :key="skin.id" :value="skin.id">{{ skin.name }}</option>
                  </select>
                </label>
                <label class="settings-field">
                  模型覆盖（可选）
                  <input v-model="companionDraft.model" placeholder="留空时沿用 AI 接口配置" />
                  <small>仅覆盖模型名称；服务商、Base URL 与 API Key 仍使用“AI 接口”中的连接。</small>
                </label>
                <label class="settings-field">
                  语音标识（预留）
                  <input v-model="companionDraft.voiceId" placeholder="voice id" />
                </label>
                <label class="settings-field wide">
                  角色设定
                  <textarea v-model="companionDraft.personaPrompt" placeholder="描述该伴侣的身份、性格和说话风格" />
                </label>
                <label class="settings-field wide">
                  附加规则（可选）
                  <textarea v-model="companionDraft.systemPrompt" placeholder="仅对该伴侣生效的边界或回复规则" />
                </label>
                <label class="settings-field">
                  好感度
                  <input v-model.number="companionDraft.favorability" type="number" />
                </label>
                <label class="settings-field">
                  亲密度
                  <input v-model.number="companionDraft.intimacy" type="number" />
                </label>
                <label class="settings-field wide">
                  当前情绪
                  <input v-model="companionDraft.mood" placeholder="例如：开心、平静" />
                </label>
              </div>
              <div class="settings-companion-actions">
                <button class="primary-button" type="button" :disabled="companionLoading" @click="saveCompanion">
                  {{ companionLoading ? '保存中...' : '保存档案' }}
                </button>
                <button class="settings-companion-reset-button" type="button" @click="startNewCompanion">
                  清空并新建
                </button>
              </div>
              <p v-if="companionStatus" class="form-success">{{ companionStatus }}</p>
              <p v-if="companionError" class="form-error">{{ companionError }}</p>
            </section>

            <section v-show="activeSettingsSection === 'ai'" class="settings-section">
              <h3>AI 接口</h3>
              <div class="settings-ai-profiles">
                <div class="settings-ai-profile-heading">
                  <div>
                    <strong>API 配置标签</strong>
                    <small>最多保存 20 个接口连接，通过列表选择，避免添加多个标签后占用设置空间。</small>
                  </div>
                  <button
                    v-if="selectedAiProfile"
                    type="button"
                    class="settings-ai-profile-update"
                    @click="updateSelectedAiProfile"
                  >
                    更新当前标签
                  </button>
                </div>
                <form class="settings-ai-profile-add" @submit.prevent="addAiProfile">
                  <input
                    v-model="settingsDraft.newAiProfileLabel"
                    placeholder="输入标签，例如：DeepSeek 工作、Ollama 本地"
                    maxlength="40"
                    autocomplete="off"
                  />
                  <button class="primary-button" type="submit">保存当前连接</button>
                </form>
                <div v-if="settingsDraft.aiProfiles.length > 0" class="settings-ai-profile-picker">
                  <label class="settings-field">
                    已保存的连接标签
                    <select :value="settingsDraft.aiActiveProfileId" @change="selectAiProfileFromList">
                      <option value="">不绑定标签 / 使用当前表单</option>
                      <option v-for="profile in settingsDraft.aiProfiles" :key="profile.id" :value="profile.id">
                        {{ profile.label }} / {{ profile.model || '未设置模型' }}
                      </option>
                    </select>
                  </label>
                  <div v-if="selectedAiProfile" class="settings-ai-profile-current">
                    <small>{{ selectedAiProfile.provider }} / {{ selectedAiProfile.model || '未设置模型' }}</small>
                    <button
                      class="settings-ai-profile-delete"
                      type="button"
                      title="删除配置标签"
                      @click="removeSelectedAiProfile"
                    >
                      删除标签
                    </button>
                  </div>
                </div>
                <p v-else class="settings-empty">还没有 API 配置标签，当前表单仍可直接保存并使用。</p>
                <p v-if="aiProfileStatus" class="settings-empty">{{ aiProfileStatus }}</p>
                <p v-if="aiProfileError" class="form-error">{{ aiProfileError }}</p>
              </div>
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
                  默认模型
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
                  全局回复规则（所有伴侣共用）
                  <textarea
                    v-model="settingsDraft.aiSystemPrompt"
                    rows="4"
                    placeholder="定义所有伴侣共用的隐私、安全与回复规则，不在这里重复具体角色人设"
                  />
                </label>
              </div>

              <p v-if="currentCompanion?.model" class="settings-empty">
                当前伴侣「{{ currentCompanion.name }}」设置了模型覆盖
                {{ currentCompanion.model }}；实际聊天使用该模型，但仍通过本页的服务商、Base URL 和 API Key 连接。
              </p>
              <p class="settings-empty">{{ selectedAiProviderPreset().help }}</p>
            </section>

            <section v-show="activeSettingsSection === 'memory'" class="settings-section">
              <h3>宠物记忆</h3>
              <label class="settings-toggle-row">
                <span>
                  <strong>启用宠物记忆</strong>
                  <small>开启后会从当前伴侣对话中提取长期记忆；按伴侣保存的聊天记录可在下方单独清空。</small>
                </span>
                <input v-model="settingsDraft.aiMemoryEnabled" type="checkbox" />
              </label>
              <label class="settings-toggle-row">
                <span>
                  <strong>启用短期记忆压缩摘要</strong>
                  <small>保留最近原文对话，同时把更早短期对话滚动压缩成一条特殊记忆参与回复。</small>
                </span>
                <input
                  v-model="settingsDraft.aiShortMemorySummaryEnabled"
                  type="checkbox"
                  :disabled="!settingsDraft.aiMemoryEnabled"
                />
              </label>
              <div class="settings-form-grid settings-memory-form">
                <label class="settings-field">
                  最近原文轮数
                  <input
                    v-model.number="settingsDraft.aiShortMemoryRecentTurns"
                    type="number"
                    min="2"
                    max="40"
                    :disabled="!settingsDraft.aiMemoryEnabled || !settingsDraft.aiShortMemorySummaryEnabled"
                  />
                </label>
                <label class="settings-field">
                  压缩触发轮数
                  <input
                    v-model.number="settingsDraft.aiShortMemoryCompressionTriggerTurns"
                    type="number"
                    min="4"
                    max="80"
                    :disabled="!settingsDraft.aiMemoryEnabled || !settingsDraft.aiShortMemorySummaryEnabled"
                  />
                </label>
                <p class="settings-empty wide">
                  默认保留最近 10 轮原文；更早聊天累计达到触发轮数后，会更新短期摘要特殊记忆。
                </p>
              </div>
              <div class="settings-memory-panel">
                <div class="settings-memory-toolbar-header">
                  <div class="settings-memory-copy">
                    <strong>记忆管理</strong>
                    <small>查看当前伴侣的长期记忆，备份数据，或清理其本机聊天记录。</small>
                  </div>
                  <button
                    type="button"
                    class="settings-memory-refresh"
                    :disabled="petMemoriesLoading"
                    @click="loadPetMemories"
                  >
                    {{ petMemoriesLoading ? '读取中...' : '刷新列表' }}
                  </button>
                </div>
                <div class="settings-memory-toolbar" aria-label="记忆数据操作">
                  <div class="settings-memory-toolbar-group">
                    <span class="settings-memory-toolbar-label">数据</span>
                    <button type="button" :disabled="petMemoriesLoading" @click="importPetMemory">
                      导入 JSON
                    </button>
                    <button type="button" @click="exportPetMemory">导出 JSON</button>
                    <button type="button" @click="openPetMemoryDirectory">打开目录</button>
                  </div>
                  <div class="settings-memory-toolbar-group">
                    <span class="settings-memory-toolbar-label">清理</span>
                    <button
                      type="button"
                      class="caution-button"
                      :disabled="petMemoriesLoading"
                      @click="clearPetMemoryMessages"
                    >
                      清空短期
                    </button>
                    <button
                      type="button"
                      class="danger-button"
                      :disabled="petMemoriesLoading || petMemories.length === 0"
                      @click="clearPetMemories"
                    >
                      清空长期
                    </button>
                  </div>
                </div>
              </div>
              <div class="settings-memory-editor">
                <div class="settings-memory-block-heading">
                  <strong>{{ editingMemoryId ? '编辑长期记忆' : '手动添加长期记忆' }}</strong>
                  <small>适合补充需要稳定记住的偏好、边界或重要信息。</small>
                </div>
                <div class="settings-form-grid settings-memory-form">
                  <label class="settings-field">
                    记忆类型
                    <select v-model="memoryDraft.memoryType">
                      <option
                        v-for="option in memoryTypeOptions"
                        :key="option.value"
                        :value="option.value"
                      >
                        {{ option.label }}
                      </option>
                    </select>
                  </label>
                  <label class="settings-field">
                    重要度
                    <input v-model.number="memoryDraft.importance" type="number" min="1" max="10" />
                  </label>
                  <label class="settings-field">
                    可信度
                    <input
                      v-model.number="memoryDraft.confidence"
                      type="number"
                      min="0"
                      max="1"
                      step="0.1"
                    />
                  </label>
                  <label class="settings-field wide">
                    记忆内容
                    <textarea
                      v-model="memoryDraft.content"
                      rows="3"
                      placeholder="例如：用户难过时不喜欢被说教，更希望被温柔安慰。"
                    />
                  </label>
                  <label class="settings-field wide">
                    标签
                    <input v-model="memoryDraft.tags" placeholder="用逗号、空格或顿号分隔" />
                  </label>
                  <div class="settings-memory-editor-actions wide">
                    <button
                      type="button"
                      class="primary-button"
                      :disabled="petMemoriesLoading"
                      @click="savePetMemoryDraft"
                    >
                      {{ editingMemoryId ? '保存修改' : '添加记忆' }}
                    </button>
                    <button
                      v-if="editingMemoryId"
                      type="button"
                      :disabled="petMemoriesLoading"
                      @click="resetMemoryDraft"
                    >
                      取消编辑
                    </button>
                  </div>
                </div>
              </div>
              <p v-if="petMemoryStatus" class="settings-empty">{{ petMemoryStatus }}</p>
              <div v-if="petMemories.length > 0" class="settings-memory-results">
                <div class="settings-memory-block-heading">
                  <strong>已保存的长期记忆</strong>
                  <small>当前伴侣已保存 {{ petMemories.length }} 条长期记忆，可直接编辑或删除。</small>
                </div>
                <div class="settings-memory-list">
                  <article v-for="memory in petMemories" :key="memory.id" class="settings-memory-row">
                    <div>
                      <strong>
                        {{ memoryTypeLabel(memory.memoryType) }} · 重要度 {{ memory.importance }} ·
                        可信度 {{ confidencePercent(memory.confidence) }}
                      </strong>
                      <p>{{ memory.content }}</p>
                      <small v-if="memory.tags.length > 0">标签：{{ memory.tags.join('、') }}</small>
                      <small>更新时间：{{ memory.updatedAt }}</small>
                    </div>
                    <div class="settings-memory-row-actions">
                      <button type="button" title="编辑记忆" @click="editPetMemory(memory)">编辑</button>
                      <button
                        type="button"
                        class="danger-button"
                        title="删除记忆"
                        @click="deletePetMemory(memory)"
                      >
                        删除
                      </button>
                    </div>
                  </article>
                </div>
              </div>
              <p v-else class="settings-empty">
                当前伴侣还没有长期记忆。启用记忆后，对话中的称呼、偏好、边界、习惯或共同经历会保存到这里。
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

            <section v-show="activeSettingsSection === 'about'" class="settings-section">
              <h3>开源许可</h3>
              <div class="settings-license-list">
                <article class="settings-license-card">
                  <strong>Twemoji</strong>
                  <p>Emoji graphics provided by Twemoji.</p>
                  <p>Licensed under CC-BY 4.0.</p>
                  <small>默认表情包文件位于项目内 src/assets/emoji/twemoji/svg。</small>
                </article>
              </div>
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

    <div
      v-if="importantConfirmation"
      class="modal-backdrop confirmation-backdrop"
      @click.self="settleImportantConfirmation(false)"
    >
      <section class="confirmation-modal" role="alertdialog" aria-modal="true">
        <header>
          <span class="confirmation-badge">重要操作</span>
          <button type="button" class="window-close" @click="settleImportantConfirmation(false)">
            ×
          </button>
        </header>
        <h2>{{ importantConfirmation.title }}</h2>
        <p class="confirmation-message">{{ importantConfirmation.message }}</p>
        <p class="confirmation-detail">{{ importantConfirmation.detail }}</p>
        <footer>
          <button type="button" @click="settleImportantConfirmation(false)">取消</button>
          <button
            type="button"
            :class="importantConfirmation.variant === 'danger' ? 'danger-button' : 'warning-button'"
            @click="settleImportantConfirmation(true)"
          >
            {{ importantConfirmation.confirmLabel }}
          </button>
        </footer>
      </section>
    </div>
  </main>
</template>
