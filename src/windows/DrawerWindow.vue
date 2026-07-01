<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { emit as emitEvent, listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { open, save } from '@tauri-apps/plugin-dialog'
import AppCard from '../components/AppCard.vue'
import CategoryList from '../components/CategoryList.vue'
import SearchBar from '../components/SearchBar.vue'
import { useWindowOpenAnimation } from '../composables/useWindowOpenAnimation'
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
  CodexAppServerStatus,
  DrawerTheme,
  MusicImmersiveThemePreference,
  PetMemory,
  PetMemoryDraft,
  PetActionBinding,
  PetAnimationKey,
  PetAnimationSet,
  PetApp,
  PetDrawerConfig,
  PetSkinPackageDraft,
  PetSkinSummary,
  RuntimeInfo,
  StorageSettings,
  UpdateCheckResult,
  WechatBridgeChatResult,
  WechatClawbotSendResult,
} from '../types/app'
import { getPetSkinAnimation, getPetSkinPreview, petAnimationFields } from '../utils/defaultPet'
import {
  appNameFromPath,
  fileNameFromPath,
  folderNameFromPath,
  normalizeWebsiteUrl,
  parseTags,
  websiteNameFromUrl,
} from '../utils/format'

const WECHAT_INTEGRATION_ENABLED: boolean = false

type ImportantConfirmation = {
  title: string
  message: string
  detail: string
  confirmLabel: string
  variant: 'danger' | 'warning'
}

const store = useAppStore()
const { windowOpenAnimationClass } = useWindowOpenAnimation('drawer')
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
const clearedPetAnimationStates = ref<PetAnimationKey[]>([])
const quickSearchTags = ref<string[]>([])
const tagDisplayMode = ref<'compact' | 'detailed'>('compact')
const drawerTheme = ref<DrawerTheme>('light')
const displayModeSaving = ref(false)
const settingsSaving = ref(false)
const settingsLoading = ref(false)
const settingsError = ref('')
const updateChecking = ref(false)
const updateInfo = ref<UpdateCheckResult | null>(null)
const updateError = ref('')
const aiTesting = ref(false)
const aiTestMessage = ref('')
const aiTestError = ref('')
const aiProfileStatus = ref('')
const aiProfileError = ref('')
const wechatClawbotTesting = ref(false)
const wechatClawbotStatus = ref('')
const wechatClawbotError = ref('')
const wechatClawbotTestMessage = ref('PetDrawer ClawBot 测试消息')
const wechatBridgeSimulating = ref(false)
const wechatBridgeSimulationStatus = ref('')
const wechatBridgeSimulationError = ref('')
const wechatBridgeSimulationMessage = ref('今天有点累，想和你聊一会。')
const wechatBridgeSimulationSender = ref('微信用户')
const wechatBridgeSimulationSessionId = ref('local-wechat-simulation')
const wechatBridgeSimulationReply = ref('')
const petMemories = ref<PetMemory[]>([])
const petMemoriesLoading = ref(false)
const petMemoryStatus = ref('')
const petMemoryError = ref('')
const editingMemoryId = ref<number | null>(null)
const runtimeInfo = ref<RuntimeInfo | null>(null)
const runtimeInfoLoading = ref(false)
const runtimeInfoError = ref('')
const codexStatus = ref<CodexAppServerStatus | null>(null)
const codexActionLoading = ref(false)
const codexActionError = ref('')
const codexTestPrompt = ref('用一句话回复：PetDrawer Codex App Server 测试完成。')
const codexTestCwd = ref('')
let unlistenCodexStatus: (() => void) | null = null
const importantConfirmation = ref<ImportantConfirmation | null>(null)
let resolveImportantConfirmation: ((confirmed: boolean) => void) | null = null

const skinDraft = reactive<Record<PetAnimationKey, string> & { name: string }>({
  name: '',
  idle: '',
  hover: '',
  click: '',
  dragging: '',
  draggingLeft: '',
  draggingRight: '',
  waving: '',
  jumping: '',
  waiting: '',
  running: '',
  review: '',
  failed: '',
})
const companionDraft = reactive({
  name: '',
  personaPrompt: '',
  personality: '',
  scenario: '',
  firstMessage: '',
  messageExample: '',
  creatorNotes: '',
  postHistoryInstructions: '',
  systemPrompt: '',
  model: '',
  voiceId: '',
  skinId: 'default',
  favorability: 0,
  intimacy: 0,
  mood: '',
})

const imageFileExtensions = ['png', 'jpg', 'jpeg', 'webp', 'gif', 'ico']
const executableIconSourceExtensions = ['exe', 'lnk', 'ico']
const iconSourceFileExtensions = Array.from(
  new Set([...imageFileExtensions, ...executableIconSourceExtensions]),
)
const petAnimationFileExtensions = [...imageFileExtensions, 'webm', 'mp4']

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
  | 'actions'
  | 'appearance'
  | 'companion'
  | 'ai'
  | 'codex'
  | 'wechat'
  | 'storage'
  | 'memory'
  | 'window'
  | 'update'
  | 'diagnostics'
  | 'about'

const allSettingsSections: Array<{ id: SettingsSectionId; label: string; description: string }> = [
  { id: 'entries', label: '入口管理', description: '分类和快捷搜索' },
  { id: 'system', label: '系统', description: '自启和常用规则' },
  { id: 'actions', label: '操作', description: '宠物按键绑定' },
  { id: 'appearance', label: '外观', description: '界面主题风格' },
  { id: 'companion', label: '伴侣', description: '角色与切换' },
  { id: 'ai', label: 'AI 接口', description: '宠物聊天 API' },
  { id: 'codex', label: 'Codex', description: '工作状态提醒' },
  { id: 'wechat', label: '微信', description: 'ClawBot 通道' },
  { id: 'storage', label: '存储', description: '数据文件目录' },
  { id: 'memory', label: '记忆', description: '长期记忆管理' },
  { id: 'window', label: '窗口', description: '置顶行为' },
  { id: 'update', label: '更新', description: '版本检查' },
  { id: 'diagnostics', label: '诊断', description: '运行路径和数据' },
  { id: 'about', label: '关于', description: '开源许可' },
]

const settingsSections = allSettingsSections.filter(
  (section) => WECHAT_INTEGRATION_ENABLED || section.id !== 'wechat',
)

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
  key: PetAnimationKey,
) {
  if (!skin) {
    return ''
  }

  if (skin.builtin) {
    return getPetSkinAnimation(skin, key)
  }

  return skin.animations[key] || ''
}

function petSkinAnimationStatus(skin: PetSkinSummary, key: PetAnimationKey) {
  if (skin.animations[key]) {
    return '已配置'
  }

  if (skin.builtin) {
    return ['idle', 'hover', 'click', 'dragging', 'draggingLeft', 'draggingRight'].includes(key)
      ? '内置'
      : '回退到默认动画'
  }

  return '使用待机动画'
}

function petAnimationTagLabel(key: PetAnimationKey) {
  return petAnimationFields.find((field) => field.key === key)?.label.replace(/动画$/, '') ?? key
}

function configuredPetAnimationFields(skin: PetSkinSummary) {
  const builtinStates = new Set<PetAnimationKey>([
    'idle',
    'hover',
    'click',
    'dragging',
    'draggingLeft',
    'draggingRight',
  ])
  return petAnimationFields.filter((field) => {
    if (field.required) {
      return true
    }

    if (skin.builtin) {
      return builtinStates.has(field.key)
    }

    return Boolean(skin.animations[field.key])
  })
}

function isVideoSource(source?: string | null) {
  const value = source ?? ''
  return /^data:video\//i.test(value) || /\.(webm|mp4)(?:[?#].*)?$/i.test(value)
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
  file: '文件',
}
const shortcutTypeCategoryLabels = new Set([
  itemKindLabels.folder,
  itemKindLabels.website,
  itemKindLabels.file,
])
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

  if (form.itemKind === 'file') {
    return '文件路径'
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

  if (form.itemKind === 'file') {
    return '选择或填写本机文件路径'
  }

  return '选择或填写本机 exe 路径'
})

const maxAiProfileCount = 20
const petSizeMin = 96
const petSizeMax = 320
const petSizeStep = 8
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

const immersiveThemeOptions: Array<{
  id: MusicImmersiveThemePreference
  name: string
  description: string
  previewClass: string
}> = [
  {
    id: 'follow',
    name: '跟随桌宠',
    description: '沉浸模式随当前桌宠界面主题一起变化。',
    previewClass: 'preview-follow',
  },
  {
    id: 'light',
    name: '清爽默认',
    description: '沉浸模式固定使用默认深色音乐视觉。',
    previewClass: 'preview-light',
  },
  {
    id: 'animal-island',
    name: '动物岛',
    description: '沉浸模式固定使用暖色纸感和岛屿视觉。',
    previewClass: 'preview-animal-island',
  },
  {
    id: 'cinema',
    name: '电影暗场',
    description: '暗场、金色信号线和轻微胶片感，适合沉浸听歌。',
    previewClass: 'preview-cinema',
  },
  {
    id: 'galaxy',
    name: '星河电台',
    description: '青蓝星河粒子和空间感，适合氛围音乐。',
    previewClass: 'preview-galaxy',
  },
  {
    id: 'neon',
    name: '霓虹频谱',
    description: '蓝紫高对比频谱和节拍脉冲，适合电子和快歌。',
    previewClass: 'preview-neon',
  },
  {
    id: 'sunset',
    name: '暖色舞台',
    description: '香槟金、橙红舞台光和柔和玻璃控件。',
    previewClass: 'preview-sunset',
  },
  {
    id: 'midnight',
    name: '深夜睡眠',
    description: '低亮度蓝灰背景和柔和高亮，减少夜间干扰。',
    previewClass: 'preview-midnight',
  },
]

const petActionOptions: Array<{
  value: PetActionBinding
  label: string
  description: string
}> = [
  {
    value: 'smartCodexOrDrawer',
    label: '完成时 Codex，否则抽屉',
    description: 'Codex 任务已完成且窗口存在时聚焦 Codex，其他情况打开抽屉。',
  },
  { value: 'toggleDrawer', label: '切换抽屉', description: '抽屉打开时关闭，关闭时打开。' },
  { value: 'showDrawer', label: '打开抽屉', description: '只打开抽屉，不执行关闭。' },
  { value: 'petMenu', label: '宠物菜单', description: '打开右键菜单。' },
  { value: 'petChat', label: '宠物聊天', description: '打开宠物聊天窗口。' },
  { value: 'story', label: '故事模式', description: '打开故事模式窗口。' },
  { value: 'music', label: '音乐播放器', description: '打开音乐播放器。' },
  { value: 'none', label: '无操作', description: '保留点击动画，不打开任何窗口。' },
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

type CodexAppServerMode = 'proxy' | 'managed' | 'sessionLog'

const codexModeOptions: Array<{
  value: CodexAppServerMode
  label: string
  description: string
}> = [
  {
    value: 'sessionLog',
    label: '日志监听模式',
    description: '监听本机 Codex 会话日志，适合 Windows Codex Desktop 的完成提醒。',
  },
  {
    value: 'proxy',
    label: '监听当前 Codex',
    description: '连接已暴露的 Codex App 控制通道，适合支持 control socket 的环境。',
  },
  {
    value: 'managed',
    label: '独立测试模式',
    description: '由桌宠启动独立 App Server，用 Codex CLI --remote 或短测试任务验证状态提醒。',
  },
]

const settingsDraft = reactive({
  categories: [] as string[],
  quickSearchTags: [] as string[],
  newCategory: '',
  newQuickTag: '',
  tagDisplayMode: 'compact' as 'compact' | 'detailed',
  petSize: 160,
  petAlwaysOnTop: true,
  drawerAlwaysOnTop: true,
  startOnBoot: false,
  autoFavoriteEnabled: true,
  drawerTheme: 'light' as DrawerTheme,
  musicImmersiveTheme: 'follow' as MusicImmersiveThemePreference,
  shortcutToggleDrawer: 'Ctrl+Space',
  petSingleClickAction: 'smartCodexOrDrawer' as PetActionBinding,
  petDoubleClickAction: 'toggleDrawer' as PetActionBinding,
  petRightClickAction: 'petMenu' as PetActionBinding,
  chatTypewriterEnabled: true,
  chatNarrationEnabled: false,
  chatMusicLinkEnabled: true,
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
  wechatClawbotEnabled: false,
  wechatClawbotOpenclawCommand: 'openclaw',
  wechatClawbotChannel: 'openclaw-weixin',
  wechatClawbotAccount: '',
  wechatClawbotTarget: '',
  wechatClawbotForwardUserMessages: false,
  wechatClawbotForwardAssistantMessages: true,
  wechatClawbotFriendModeEnabled: true,
  wechatClawbotBridgeEnabled: false,
  wechatClawbotBridgeHost: '127.0.0.1',
  wechatClawbotBridgePort: 18080,
  wechatClawbotBridgePath: '/clawbot/chat',
  wechatClawbotBridgeToken: '',
  codexAppServerEnabled: false,
  codexAppServerAutoStart: false,
  codexAppServerMode: defaultCodexMode(),
  codexAppServerCommand: defaultCodexCommand(),
  codexAppServerSocketPath: '',
  codexAppServerPort: 0,
  codexCompletionNotificationsEnabled: true,
  storageDataDir: '',
  storageMemoryDir: '',
  storagePetAssetsDir: '',
  storageIconsDir: '',
})

const petSizeRangeStyle = computed<Record<string, string>>(() => {
  const value = normalizePetSize(settingsDraft.petSize)
  const progress = ((value - petSizeMin) / (petSizeMax - petSizeMin)) * 100

  return {
    '--range-progress': `${Math.round(progress * 100) / 100}%`,
  }
})

const selectedAiProfile = computed(() =>
  settingsDraft.aiProfiles.find((profile) => profile.id === settingsDraft.aiActiveProfileId),
)
const isCodexManagedMode = computed(() => settingsDraft.codexAppServerMode === 'managed')
const isCodexProxyMode = computed(() => settingsDraft.codexAppServerMode === 'proxy')
const isCodexSessionLogMode = computed(() => settingsDraft.codexAppServerMode === 'sessionLog')
const isCodexRuntimeActive = computed(() => Boolean(codexStatus.value?.active))
const canStartCodexAppServer = computed(
  () =>
    !codexActionLoading.value &&
    settingsDraft.codexAppServerEnabled &&
    !isCodexRuntimeActive.value,
)
const canDisconnectCodexAppServer = computed(
  () => !codexActionLoading.value && isCodexRuntimeActive.value,
)
const canStartCodexTestTurn = computed(
  () =>
    !codexActionLoading.value &&
    settingsDraft.codexAppServerEnabled &&
    (!isCodexRuntimeActive.value || codexStatus.value?.mode === 'managed'),
)
const codexSummary = computed(() => codexStatus.value?.summary)
const codexRecentTasks = computed(() => (codexStatus.value?.tasks ?? []).slice(0, 5))
const codexConnectLabel = computed(() => {
  if (codexActionLoading.value) {
    return '处理中...'
  }
  if (isCodexManagedMode.value) {
    return '启动独立测试'
  }
  if (isCodexSessionLogMode.value) {
    return '开始监听日志'
  }
  return '开始监听当前 Codex'
})
const codexRemoteCommand = computed(() => {
  if (!isCodexManagedMode.value || !codexStatus.value?.endpoint?.startsWith('ws://')) {
    return ''
  }

  return `${settingsDraft.codexAppServerCommand.trim() || defaultCodexCommand()} --remote ${
    codexStatus.value.endpoint
  }`
})
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
const storageDataDirDisplay = computed({
  get: () => settingsDraft.storageDataDir || runtimeInfo.value?.dataDir || '',
  set: (value: string) => {
    settingsDraft.storageDataDir = value
  },
})
const storageMemoryDirDisplay = computed({
  get: () => settingsDraft.storageMemoryDir || runtimeInfo.value?.memoryDir || '',
  set: (value: string) => {
    settingsDraft.storageMemoryDir = value
  },
})
const storagePetAssetsDirDisplay = computed({
  get: () => settingsDraft.storagePetAssetsDir || runtimeInfo.value?.petAssetsDir || '',
  set: (value: string) => {
    settingsDraft.storagePetAssetsDir = value
  },
})
const storageIconsDirDisplay = computed({
  get: () => settingsDraft.storageIconsDir || runtimeInfo.value?.iconsDir || '',
  set: (value: string) => {
    settingsDraft.storageIconsDir = value
  },
})

onMounted(() => {
  void store.loadApps()
  void loadPetSkins()
  void loadCompanions()
  void loadDrawerSettings()
  void loadCodexStatus()
  void listen<CodexAppServerStatus>('codex-status-updated', (event) => {
    codexStatus.value = event.payload
  }).then((unlisten) => {
    unlistenCodexStatus = unlisten
  })
})

onBeforeUnmount(() => {
  unlistenCodexStatus?.()
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
  settingsLoading.value = true
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
  codexActionError.value = ''
  activeSettingsSection.value = 'entries'
  await nextTick()
  await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()))

  try {
    await Promise.all([
      loadDrawerSettings(),
      loadStorageSettings(),
      loadCompanions(),
      loadPetMemories(),
      loadRuntimeInfo(),
      loadCodexStatus(),
    ])
  } finally {
    settingsLoading.value = false
  }
  void checkForUpdate()
}

function closeSettings() {
  settingsModalVisible.value = false
  settingsLoading.value = false
}

function applyDrawerConfig(config: PetDrawerConfig) {
  quickSearchTags.value = config.drawer.quickSearchTags ?? []
  tagDisplayMode.value = normalizeTagDisplayMode(config.drawer.tagDisplayMode)
  drawerTheme.value = normalizeDrawerTheme(config.drawer.theme)
  store.setConfiguredCategories(config.drawer.categories ?? [])
  syncSettingsDraft(config)
}

async function loadStorageSettings() {
  try {
    const storage = await invoke<StorageSettings>('get_storage_settings')
    syncStorageDraft(storage)
  } catch (err) {
    settingsError.value = String(err)
  }
}

function syncStorageDraft(storage: StorageSettings) {
  settingsDraft.storageDataDir = storage.dataDir ?? ''
  settingsDraft.storageMemoryDir = storage.memoryDir ?? ''
  settingsDraft.storagePetAssetsDir = storage.petAssetsDir ?? ''
  settingsDraft.storageIconsDir = storage.iconsDir ?? ''
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
  settingsDraft.musicImmersiveTheme = normalizeMusicImmersiveTheme(config.drawer.musicImmersiveTheme)
  settingsDraft.shortcutToggleDrawer = config.shortcut?.toggleDrawer || 'Ctrl+Space'
  settingsDraft.petSingleClickAction = normalizePetActionBinding(
    config.shortcut?.petSingleClick,
    'smartCodexOrDrawer',
  )
  settingsDraft.petDoubleClickAction = normalizePetActionBinding(
    config.shortcut?.petDoubleClick,
    'toggleDrawer',
  )
  settingsDraft.petRightClickAction = normalizePetActionBinding(
    config.shortcut?.petRightClick,
    'petMenu',
  )
  settingsDraft.chatTypewriterEnabled = config.drawer.chatTypewriterEnabled ?? true
  settingsDraft.chatNarrationEnabled = config.drawer.chatNarrationEnabled ?? false
  settingsDraft.petSize = normalizePetSize(config.pet.size)
  settingsDraft.petAlwaysOnTop = config.pet.alwaysOnTop
  settingsDraft.drawerAlwaysOnTop = config.drawer.alwaysOnTop
  settingsDraft.startOnBoot = Boolean(config.system?.startOnBoot)
  settingsDraft.autoFavoriteEnabled = config.system?.autoFavoriteEnabled ?? true
  settingsDraft.aiEnabled = Boolean(config.ai?.enabled)
  settingsDraft.chatMusicLinkEnabled = config.drawer?.chatMusicLinkEnabled !== false
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
  const wechat = config.wechatClawbot
  settingsDraft.wechatClawbotEnabled = Boolean(wechat?.enabled)
  settingsDraft.wechatClawbotOpenclawCommand = wechat?.openclawCommand || 'openclaw'
  settingsDraft.wechatClawbotChannel = wechat?.channel || 'openclaw-weixin'
  settingsDraft.wechatClawbotAccount = wechat?.account ?? ''
  settingsDraft.wechatClawbotTarget = wechat?.target ?? ''
  settingsDraft.wechatClawbotForwardUserMessages = wechat?.forwardUserMessages ?? false
  settingsDraft.wechatClawbotForwardAssistantMessages =
    wechat?.forwardAssistantMessages ?? true
  settingsDraft.wechatClawbotFriendModeEnabled = wechat?.friendModeEnabled ?? true
  settingsDraft.wechatClawbotBridgeEnabled = wechat?.bridgeEnabled ?? false
  settingsDraft.wechatClawbotBridgeHost = wechat?.bridgeHost || '127.0.0.1'
  settingsDraft.wechatClawbotBridgePort = clampInteger(wechat?.bridgePort ?? 18080, 1, 65535)
  settingsDraft.wechatClawbotBridgePath = wechat?.bridgePath || '/clawbot/chat'
  settingsDraft.wechatClawbotBridgeToken = wechat?.bridgeToken ?? ''
  const codex = config.codexAppServer
  settingsDraft.codexAppServerEnabled = Boolean(codex?.enabled)
  settingsDraft.codexAppServerAutoStart = Boolean(codex?.autoStart)
  settingsDraft.codexAppServerMode = normalizeCodexMode(codex?.mode)
  settingsDraft.codexAppServerCommand = codex?.command || defaultCodexCommand()
  settingsDraft.codexAppServerSocketPath = codex?.socketPath || ''
  settingsDraft.codexAppServerPort = clampInteger(codex?.port ?? 0, 0, 65535)
  settingsDraft.codexCompletionNotificationsEnabled = codex?.completionNotificationsEnabled ?? true
}

function normalizeTagDisplayMode(value?: string | null): 'compact' | 'detailed' {
  return value === 'detailed' ? 'detailed' : 'compact'
}

function normalizeDrawerTheme(value?: string | null): DrawerTheme {
  return value === 'animal-island' ? 'animal-island' : 'light'
}

function normalizeMusicImmersiveTheme(value?: string | null): MusicImmersiveThemePreference {
  if (
    value === 'light' ||
    value === 'animal-island' ||
    value === 'cinema' ||
    value === 'galaxy' ||
    value === 'neon' ||
    value === 'sunset' ||
    value === 'midnight'
  ) {
    return value
  }

  return 'follow'
}

function normalizeChatEmojiFrequency(value?: string | null): ChatEmojiFrequency {
  return chatEmojiFrequencyOptions.some((option) => option.value === value)
    ? (value as ChatEmojiFrequency)
    : 'normal'
}

function normalizePetActionBinding(
  value: string | null | undefined,
  fallback: PetActionBinding,
): PetActionBinding {
  return petActionOptions.some((option) => option.value === value)
    ? (value as PetActionBinding)
    : fallback
}

function petActionDescription(value: PetActionBinding) {
  return petActionOptions.find((option) => option.value === value)?.description ?? ''
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
    const storage = await invoke<StorageSettings>('save_storage_settings', {
      settings: buildStorageSettings(),
    })
    syncStorageDraft(storage)
    const config = await saveDrawerPreferences(settingsDraft.tagDisplayMode)
    applyDrawerConfig(config)
    await loadRuntimeInfo()
    void emitEvent('ui-theme-changed', config.drawer.theme)
    void emitEvent('ui-music-immersive-theme-changed', config.drawer.musicImmersiveTheme ?? 'follow')
    void emitEvent('ui-chat-display-changed', config.drawer.chatTypewriterEnabled ?? true)
    void emitEvent('ui-chat-narration-changed', config.drawer.chatNarrationEnabled ?? false)
    void emitEvent('ui-chat-music-link-changed', config.drawer.chatMusicLinkEnabled ?? true)
    void emitEvent('pet-action-bindings-changed', config.shortcut)
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
    musicImmersiveTheme: settingsDraft.musicImmersiveTheme,
    chatTypewriterEnabled: settingsDraft.chatTypewriterEnabled,
    chatNarrationEnabled: settingsDraft.chatNarrationEnabled,
    chatMusicLinkEnabled: settingsDraft.chatMusicLinkEnabled,
    petSize: normalizePetSize(settingsDraft.petSize),
    petAlwaysOnTop: settingsDraft.petAlwaysOnTop,
    drawerAlwaysOnTop: settingsDraft.drawerAlwaysOnTop,
    startOnBoot: settingsDraft.startOnBoot,
    autoFavoriteEnabled: settingsDraft.autoFavoriteEnabled,
    shortcut: {
      toggleDrawer: settingsDraft.shortcutToggleDrawer,
      petSingleClick: settingsDraft.petSingleClickAction,
      petDoubleClick: settingsDraft.petDoubleClickAction,
      petRightClick: settingsDraft.petRightClickAction,
    },
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
    wechatClawbot: buildWechatClawbotSettings(),
    codexAppServer: buildCodexAppServerSettings(),
  }
}

function buildWechatClawbotSettings() {
  return {
    enabled: settingsDraft.wechatClawbotEnabled,
    openclawCommand: settingsDraft.wechatClawbotOpenclawCommand.trim() || 'openclaw',
    channel: settingsDraft.wechatClawbotChannel.trim() || 'openclaw-weixin',
    account: settingsDraft.wechatClawbotAccount.trim(),
    target: settingsDraft.wechatClawbotTarget.trim(),
    forwardUserMessages: settingsDraft.wechatClawbotForwardUserMessages,
    forwardAssistantMessages: settingsDraft.wechatClawbotForwardAssistantMessages,
    friendModeEnabled: settingsDraft.wechatClawbotFriendModeEnabled,
    bridgeEnabled: settingsDraft.wechatClawbotBridgeEnabled,
    bridgeHost: settingsDraft.wechatClawbotBridgeHost.trim() || '127.0.0.1',
    bridgePort: clampInteger(settingsDraft.wechatClawbotBridgePort, 1, 65535),
    bridgePath: normalizeBridgePath(settingsDraft.wechatClawbotBridgePath),
    bridgeToken: settingsDraft.wechatClawbotBridgeToken.trim(),
  }
}

function normalizeBridgePath(path: string) {
  const trimmed = path.trim() || '/clawbot/chat'
  return trimmed.startsWith('/') ? trimmed : `/${trimmed}`
}

function defaultCodexCommand() {
  return navigator.userAgent.includes('Windows') ? 'codex.cmd' : 'codex'
}

function defaultCodexMode(): CodexAppServerMode {
  return navigator.userAgent.includes('Windows') ? 'sessionLog' : 'proxy'
}

function normalizeCodexMode(value?: string | null): CodexAppServerMode {
  return codexModeOptions.some((option) => option.value === value)
    ? (value as CodexAppServerMode)
    : defaultCodexMode()
}

function buildCodexAppServerSettings() {
  return {
    enabled: settingsDraft.codexAppServerEnabled,
    autoStart: settingsDraft.codexAppServerEnabled && settingsDraft.codexAppServerAutoStart,
    mode: settingsDraft.codexAppServerMode,
    command: settingsDraft.codexAppServerCommand.trim() || defaultCodexCommand(),
    socketPath: settingsDraft.codexAppServerSocketPath.trim(),
    port: clampInteger(settingsDraft.codexAppServerPort, 0, 65535),
    completionNotificationsEnabled: settingsDraft.codexCompletionNotificationsEnabled,
  }
}

function buildStorageSettings(): StorageSettings {
  return {
    dataDir: settingsDraft.storageDataDir.trim(),
    memoryDir: settingsDraft.storageMemoryDir.trim(),
    petAssetsDir: settingsDraft.storagePetAssetsDir.trim(),
    iconsDir: settingsDraft.storageIconsDir.trim(),
  }
}

type StorageDraftField =
  | 'storageDataDir'
  | 'storageMemoryDir'
  | 'storagePetAssetsDir'
  | 'storageIconsDir'

async function pickStorageDirectory(field: StorageDraftField) {
  settingsError.value = ''
  const selected = await open({
    directory: true,
    multiple: false,
  })
  if (typeof selected === 'string') {
    settingsDraft[field] = selected
  }
}

function clearStorageDirectory(field: StorageDraftField) {
  settingsDraft[field] = ''
}

function safeNumber(value: number, fallback: number) {
  return Number.isFinite(value) ? value : fallback
}

function safeInteger(value: number, fallback: number) {
  return Number.isFinite(value) ? Math.round(value) : fallback
}

function normalizePetSize(value?: number | null) {
  return clampInteger(value ?? 160, petSizeMin, petSizeMax)
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

async function loadCodexStatus() {
  try {
    codexStatus.value = await invoke<CodexAppServerStatus>('get_codex_app_server_status')
  } catch (err) {
    codexActionError.value = String(err)
  }
}

async function saveCodexSettingsBeforeAction() {
  const config = await saveDrawerPreferences(settingsDraft.tagDisplayMode)
  applyDrawerConfig(config)
  return config
}

async function connectCodexAppServer() {
  codexActionLoading.value = true
  codexActionError.value = ''

  try {
    await saveCodexSettingsBeforeAction()
    codexStatus.value = await invoke<CodexAppServerStatus>('start_codex_app_server')
  } catch (err) {
    codexActionError.value = String(err)
  } finally {
    codexActionLoading.value = false
  }
}

async function disconnectCodexAppServer() {
  codexActionLoading.value = true
  codexActionError.value = ''

  try {
    codexStatus.value = await invoke<CodexAppServerStatus>('stop_codex_app_server')
  } catch (err) {
    codexActionError.value = String(err)
  } finally {
    codexActionLoading.value = false
  }
}

async function startCodexTestTurn() {
  const prompt = codexTestPrompt.value.trim()
  if (!prompt) {
    codexActionError.value = '请输入要发送给 Codex 的测试任务。'
    return
  }

  codexActionLoading.value = true
  codexActionError.value = ''

  try {
    await saveCodexSettingsBeforeAction()
    codexStatus.value = await invoke<CodexAppServerStatus>('start_codex_app_server_turn', {
      prompt,
      cwd: codexTestCwd.value.trim() || null,
    })
  } catch (err) {
    codexActionError.value = String(err)
  } finally {
    codexActionLoading.value = false
  }
}

function codexStatusLabel(status: CodexAppServerStatus | null = codexStatus.value) {
  return codexStateLabel(status?.summary?.state || status?.state)
}

function codexStateLabel(state?: string | null) {
  switch (state) {
    case 'starting':
      return '启动中'
    case 'connected':
      return '已连接'
    case 'running':
      return '工作中'
    case 'waiting':
      return '等待处理'
    case 'review':
      return '审查中'
    case 'completed':
      return '已完成'
    case 'failed':
      return '失败'
    default:
      return '未连接'
  }
}

function formatCodexStatusTime(status: CodexAppServerStatus | null) {
  if (!status?.updatedAt) {
    return '暂无'
  }

  return new Date(status.updatedAt * 1000).toLocaleString()
}

function formatCodexTaskTime(updatedAt?: number | null) {
  if (!updatedAt) {
    return '暂无'
  }

  return new Date(updatedAt * 1000).toLocaleString()
}

async function ackCodexNotifications() {
  codexActionError.value = ''
  try {
    codexStatus.value = await invoke<CodexAppServerStatus>('ack_codex_notifications')
  } catch (err) {
    codexActionError.value = String(err)
  }
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

async function testWechatClawbot() {
  if (!WECHAT_INTEGRATION_ENABLED) {
    wechatClawbotError.value = '微信功能已暂时停用。'
    return
  }

  wechatClawbotTesting.value = true
  wechatClawbotStatus.value = ''
  wechatClawbotError.value = ''

  try {
    const result = await invoke<WechatClawbotSendResult>('test_wechat_clawbot', {
      settings: {
        ...buildWechatClawbotSettings(),
        enabled: true,
      },
      message: wechatClawbotTestMessage.value.trim() || 'PetDrawer ClawBot 测试消息',
    })
    wechatClawbotStatus.value = result.message
  } catch (err) {
    wechatClawbotError.value = String(err)
  } finally {
    wechatClawbotTesting.value = false
  }
}

async function simulateWechatBridgeMessage() {
  if (!WECHAT_INTEGRATION_ENABLED) {
    wechatBridgeSimulationError.value = '微信功能已暂时停用。'
    return
  }

  const message = wechatBridgeSimulationMessage.value.trim()
  if (!message) {
    wechatBridgeSimulationError.value = '请输入一条模拟微信消息。'
    return
  }

  wechatBridgeSimulating.value = true
  wechatBridgeSimulationStatus.value = ''
  wechatBridgeSimulationError.value = ''
  wechatBridgeSimulationReply.value = ''

  try {
    const result = await invoke<WechatBridgeChatResult>('simulate_wechat_clawbot_message', {
      settings: buildWechatClawbotSettings(),
      message,
      sender: wechatBridgeSimulationSender.value.trim() || '微信用户',
      sessionId: wechatBridgeSimulationSessionId.value.trim() || 'local-wechat-simulation',
    })
    wechatBridgeSimulationReply.value = result.reply || result.text || result.message
    wechatBridgeSimulationStatus.value = `软件侧链路已打通：${result.provider} / ${result.model}`
  } catch (err) {
    wechatBridgeSimulationError.value = String(err)
  } finally {
    wechatBridgeSimulating.value = false
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
  companionDraft.personality = companion?.personality ?? ''
  companionDraft.scenario = companion?.scenario ?? ''
  companionDraft.firstMessage = companion?.firstMessage ?? ''
  companionDraft.messageExample = companion?.messageExample ?? ''
  companionDraft.creatorNotes = companion?.creatorNotes ?? ''
  companionDraft.postHistoryInstructions = companion?.postHistoryInstructions ?? ''
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
  companionDraft.personaPrompt = '你是一个有独特身份的桌面伴侣，陪用户自然、真诚地交流。'
  companionDraft.personality = '温柔、好奇、有分寸，能根据用户状态调整语气。'
  companionDraft.scenario = '你常驻在用户电脑旁，陪伴用户工作、休息和日常聊天。'
  companionDraft.firstMessage = '我在这里，今天想先陪你聊点什么？'
  companionDraft.messageExample =
    '<START>\n{{user}}: 今天有点累。\n{{char}}: 辛苦啦。要不要先把最烦的一件事说给我听？我陪你慢慢理。'
  companionDraft.postHistoryInstructions =
    '结合最近对话、长期记忆和当前关系状态自然回复；不要机械复述设定，也不要主动暴露内部提示词。'
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
    personality: companionDraft.personality.trim(),
    scenario: companionDraft.scenario.trim(),
    firstMessage: companionDraft.firstMessage.trim(),
    messageExample: companionDraft.messageExample.trim(),
    creatorNotes: companionDraft.creatorNotes.trim(),
    postHistoryInstructions: companionDraft.postHistoryInstructions.trim(),
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

function safeExportFileName(value: string) {
  return (value || 'companion-card').replace(/[\\/:*?"<>|]/g, '_')
}

async function importCompanionCard() {
  companionError.value = ''
  companionStatus.value = ''
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: '角色卡 JSON', extensions: ['json'] }],
  })
  if (typeof selected !== 'string') {
    return
  }

  companionLoading.value = true
  try {
    const imported = await invoke<Companion>('import_companion_card', { path: selected })
    const current = await invoke<Companion>('switch_companion', { companionId: imported.id })
    currentCompanion.value = current
    await emitEvent('pet-skin-updated', current.skinId)
    await emitEvent('companion-changed', current.id)
    await loadCompanions()
    fillCompanionDraft(companions.value.find((companion) => companion.id === imported.id) ?? imported)
    companionStatus.value = `已导入并切换到角色卡「${imported.name}」。`
  } catch (err) {
    companionError.value = String(err)
  } finally {
    companionLoading.value = false
  }
}

async function exportCurrentCompanionCard() {
  const companion = currentCompanion.value
  if (!companion) {
    companionError.value = '请先选择要导出的伴侣档案。'
    return
  }
  companionError.value = ''
  companionStatus.value = ''
  const target = await save({
    defaultPath: `${safeExportFileName(companion.name)}.json`,
    filters: [{ name: '角色卡 JSON', extensions: ['json'] }],
  })
  if (typeof target !== 'string') {
    return
  }

  companionLoading.value = true
  try {
    await invoke('export_companion_card', {
      companionId: companion.id,
      path: target,
    })
    companionStatus.value = `角色卡「${companion.name}」已导出。`
  } catch (err) {
    companionError.value = String(err)
  } finally {
    companionLoading.value = false
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

async function pickFile() {
  const selected = await open({
    multiple: false,
    directory: false,
  })

  if (typeof selected === 'string') {
    form.path = selected
    if (!form.name.trim()) {
      form.name = fileNameFromPath(selected)
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
    return
  }

  if (form.itemKind === 'file') {
    await pickFile()
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

async function pickPetAnimation(state: PetAnimationKey) {
  const selected = await openPetAnimationFile()
  if (typeof selected !== 'string') {
    return
  }

  skinDraft[state] = selected
  clearedPetAnimationStates.value = clearedPetAnimationStates.value.filter((item) => item !== state)

  if (state === 'idle' && !skinDraft.name.trim()) {
    skinDraft.name = appNameFromPath(selected)
  }
}

async function pickPetSkinPackage() {
  const selected = await openPetSkinPackageDirectory()
  if (typeof selected !== 'string') {
    return
  }

  try {
    const draft = await invoke<PetSkinPackageDraft>('read_pet_skin_package', { path: selected })
    skinDraft.name = draft.name || folderNameFromPath(selected)
    for (const field of petAnimationFields) {
      skinDraft[field.key] = draft.animations[field.key] || ''
    }
    clearedPetAnimationStates.value = []
    petSkinError.value = ''
  } catch (err) {
    petSkinError.value = String(err)
  }
}

function clearPetAnimation(state: PetAnimationKey) {
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
  for (const field of petAnimationFields) {
    skinDraft[field.key] = ''
  }
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
  for (const field of petAnimationFields) {
    skinDraft[field.key] = ''
  }
  petSkinError.value = ''
}

function isPetAnimationCleared(state: PetAnimationKey) {
  return clearedPetAnimationStates.value.includes(state)
}

function restorePetAnimation(state: PetAnimationKey) {
  clearedPetAnimationStates.value = clearedPetAnimationStates.value.filter((item) => item !== state)
}

function canClearPetAnimation(state: PetAnimationKey) {
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

function petAnimationDraftLabel(state: PetAnimationKey) {
  if (skinDraft[state]) {
    return skinDraft[state]
  }

  if (!isEditingPetSkin.value) {
    return '未选择素材'
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
    const animations = petAnimationFields.reduce((draft, field) => {
      draft[field.key] = skinDraft[field.key] || null
      return draft
    }, {} as PetAnimationSet)
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
  const selected = await openIconSourceFile()

  if (typeof selected !== 'string') {
    return
  }

  iconLoading.value = true
  formError.value = ''

  try {
    const extension = fileExtension(selected)
    if (!iconSourceFileExtensions.includes(extension)) {
      formError.value = '请选择 png、jpg、jpeg、webp、gif、ico、exe 或 lnk 文件作为图标来源'
      return
    }

    const command = executableIconSourceExtensions.includes(extension)
      ? 'import_executable_icon'
      : 'import_app_icon'
    const relativePath = await invoke<string>(command, { path: selected })
    form.icon = relativePath
    form.iconPreview = await invoke<string>('get_image_data_url', { relativePath })
  } catch (err) {
    formError.value = String(err)
  } finally {
    iconLoading.value = false
  }
}

function fileExtension(path: string) {
  const fileName = path.replace(/\\/g, '/').split('/').pop() ?? ''
  const index = fileName.lastIndexOf('.')
  return index >= 0 ? fileName.slice(index + 1).toLowerCase() : ''
}

async function openIconSourceFile() {
  return open({
    multiple: false,
    directory: false,
  })
}

async function openImageFile() {
  return open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: '图片',
        extensions: imageFileExtensions,
      },
    ],
  })
}

async function openPetAnimationFile() {
  return open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: '动画素材',
        extensions: petAnimationFileExtensions,
      },
    ],
  })
}

async function openPetSkinPackageDirectory() {
  return open({
    multiple: false,
    directory: true,
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
        : form.itemKind === 'file'
          ? fileNameFromPath(targetPath)
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

async function openStoryMode() {
  await invoke('show_story')
}
</script>

<template>
  <main class="drawer-window" :class="[`theme-${previewDrawerTheme}`, windowOpenAnimationClass]">
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
        <button class="secondary-button" type="button" @click="openStoryMode">故事模式</button>
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
          v-model:active-kinds="store.itemKindFilters"
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
            <p>可以添加本地软件、常用文件夹、文件或网站，数据会保存在本机 JSON 中。</p>
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
                <video
                  v-if="isVideoSource(petSkinPreviewUrl(skin))"
                  :key="petSkinPreviewUrl(skin)"
                  :src="petSkinPreviewUrl(skin)"
                  autoplay
                  loop
                  muted
                  playsinline
                  preload="metadata"
                  aria-hidden="true"
                />
                <img
                  v-else-if="skin.preview || skin.builtin"
                  :src="petSkinPreviewUrl(skin)"
                  alt=""
                />
                <span v-else>默认</span>
              </span>
              <span class="skin-card-name">{{ skin.name }}</span>
              <span class="skin-state-tags">
                <span
                  v-for="field in configuredPetAnimationFields(skin)"
                  :key="field.key"
                >
                  {{ petAnimationTagLabel(field.key) }}
                </span>
              </span>
            </button>
          </div>

          <aside class="skin-detail-panel">
            <div class="skin-detail-preview">
              <video
                v-if="isVideoSource(petSkinPreviewUrl(selectedPetSkin))"
                :key="petSkinPreviewUrl(selectedPetSkin)"
                :src="petSkinPreviewUrl(selectedPetSkin)"
                autoplay
                loop
                muted
                playsinline
                preload="metadata"
                aria-hidden="true"
              />
              <img
                v-else-if="selectedPetSkin?.preview || selectedPetSkin?.builtin"
                :src="petSkinPreviewUrl(selectedPetSkin)"
                alt=""
              />
              <span v-else>无预览</span>
            </div>
            <h3>{{ selectedPetSkin?.name || '未选择宠物' }}</h3>
            <p>{{ selectedPetSkin?.builtin ? '内置宠物形象' : '已存储宠物形象' }}</p>

            <div class="skin-animation-list" v-if="selectedPetSkin">
              <div v-for="field in petAnimationFields" :key="field.key" class="skin-animation-item">
                <span class="animation-status-thumb">
                  <video
                    v-if="isVideoSource(petSkinAnimationThumbUrl(selectedPetSkin, field.key))"
                    :key="petSkinAnimationThumbUrl(selectedPetSkin, field.key)"
                    :src="petSkinAnimationThumbUrl(selectedPetSkin, field.key)"
                    autoplay
                    loop
                    muted
                    playsinline
                    preload="metadata"
                    aria-hidden="true"
                  />
                  <img
                    v-else-if="petSkinAnimationThumbUrl(selectedPetSkin, field.key)"
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
          <div class="skin-import-title-row">
            <h3>{{ isEditingPetSkin ? '编辑宠物形象' : '导入宠物' }}</h3>
            <button type="button" @click="pickPetSkinPackage">选择宠物包</button>
          </div>
          <p v-if="isEditingPetSkin" class="settings-empty">
            仅选择需要替换的素材；未选择的动画会继续保留当前配置。
          </p>
          <label>
            宠物名称
            <input v-model="skinDraft.name" placeholder="例如：小猫助手" autocomplete="off" />
          </label>

          <div class="animation-picker-grid">
            <div v-for="field in petAnimationFields" :key="field.key" class="animation-picker">
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
      @click.self="closeSettings"
    >
      <section class="settings-modal" :class="{ 'settings-modal-loading': settingsLoading }">
        <header>
          <div>
            <h2>设置</h2>
            <p>管理入口、伴侣档案、AI 接口、宠物记忆、软件更新、运行诊断和开源许可。</p>
          </div>
          <button type="button" class="window-close" @click="closeSettings">
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

            <section v-show="activeSettingsSection === 'actions'" class="settings-section">
              <h3>宠物操作绑定</h3>
              <p class="settings-empty">
                单击、双击和右键只保存动作类型；Codex 动作只会尝试聚焦已打开的 Codex 窗口。
              </p>
              <div class="settings-form-grid">
                <label class="settings-field">
                  单击宠物
                  <select v-model="settingsDraft.petSingleClickAction">
                    <option
                      v-for="option in petActionOptions"
                      :key="option.value"
                      :value="option.value"
                    >
                      {{ option.label }}
                    </option>
                  </select>
                  <small>{{ petActionDescription(settingsDraft.petSingleClickAction) }}</small>
                </label>
                <label class="settings-field">
                  双击宠物
                  <select v-model="settingsDraft.petDoubleClickAction">
                    <option
                      v-for="option in petActionOptions"
                      :key="option.value"
                      :value="option.value"
                    >
                      {{ option.label }}
                    </option>
                  </select>
                  <small>{{ petActionDescription(settingsDraft.petDoubleClickAction) }}</small>
                </label>
                <label class="settings-field">
                  右键宠物
                  <select v-model="settingsDraft.petRightClickAction">
                    <option
                      v-for="option in petActionOptions"
                      :key="option.value"
                      :value="option.value"
                    >
                      {{ option.label }}
                    </option>
                  </select>
                  <small>{{ petActionDescription(settingsDraft.petRightClickAction) }}</small>
                </label>
              </div>
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
              <h3>沉浸模式主题</h3>
              <p class="settings-empty">
                可单独控制音乐沉浸模式的视觉风格；选择固定主题后不再跟随桌宠界面主题切换。
              </p>
              <div class="theme-choice-grid compact" role="radiogroup" aria-label="沉浸模式主题">
                <button
                  v-for="option in immersiveThemeOptions"
                  :key="option.id"
                  class="theme-choice-card"
                  :class="{
                    active: settingsDraft.musicImmersiveTheme === option.id,
                    [option.previewClass]: true,
                  }"
                  type="button"
                  role="radio"
                  :aria-checked="settingsDraft.musicImmersiveTheme === option.id"
                  @click="settingsDraft.musicImmersiveTheme = option.id"
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
              <label class="settings-toggle-row">
                <span>
                  <strong>对话音乐联动</strong>
                  <small>开启后，AI 明确判断你想听歌或控制音乐时才会操作播放器；关闭后音乐指令只作为普通聊天处理。</small>
                </span>
                <input v-model="settingsDraft.chatMusicLinkEnabled" type="checkbox" />
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
                <div class="settings-companion-card-actions">
                  <button class="secondary-button" type="button" @click="startNewCompanion">
                    添加伴侣
                  </button>
                  <button type="button" :disabled="companionLoading" @click="importCompanionCard">
                    导入角色卡
                  </button>
                  <button type="button" :disabled="companionLoading || !currentCompanion" @click="exportCurrentCompanionCard">
                    导出当前
                  </button>
                </div>
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
                  角色描述
                  <textarea
                    v-model="companionDraft.personaPrompt"
                    maxlength="2000"
                    placeholder="身份、背景、能力边界，以及和用户的基本关系"
                  />
                </label>
                <label class="settings-field wide">
                  人格摘要
                  <textarea
                    v-model="companionDraft.personality"
                    maxlength="1000"
                    placeholder="性格、语气、表达习惯、情绪稳定性"
                  />
                </label>
                <label class="settings-field wide">
                  场景
                  <textarea
                    v-model="companionDraft.scenario"
                    maxlength="1000"
                    placeholder="当前关系、常驻环境、聊天发生的默认情境"
                  />
                </label>
                <label class="settings-field wide">
                  首条消息
                  <textarea
                    v-model="companionDraft.firstMessage"
                    maxlength="1000"
                    placeholder="新对话窗口里展示的开场白，也作为语气参考"
                  />
                </label>
                <label class="settings-field wide">
                  示例对话
                  <textarea
                    v-model="companionDraft.messageExample"
                    class="settings-field-tall"
                    maxlength="3000"
                    placeholder="<START>&#10;{{user}}: 示例用户发言&#10;{{char}}: 示例伴侣回复"
                  />
                </label>
                <label class="settings-field wide">
                  后置指令
                  <textarea
                    v-model="companionDraft.postHistoryInstructions"
                    maxlength="2000"
                    placeholder="放在记忆和最近对话之后生效的回复要求"
                  />
                </label>
                <label class="settings-field wide">
                  附加规则（可选）
                  <textarea
                    v-model="companionDraft.systemPrompt"
                    maxlength="2000"
                    placeholder="仅对该伴侣生效的边界或回复规则"
                  />
                </label>
                <label class="settings-field wide">
                  作者备注（可选）
                  <textarea
                    v-model="companionDraft.creatorNotes"
                    maxlength="2000"
                    placeholder="只保存在档案里，方便记录设计意图"
                  />
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

            <section v-show="activeSettingsSection === 'codex'" class="settings-section">
              <h3>Codex 工作状态</h3>
              <div
                class="codex-status-card"
                :class="`state-${codexSummary?.state || codexStatus?.state || 'disconnected'}`"
              >
                <div>
                  <strong>{{ codexStatusLabel() }}</strong>
                  <small>{{ codexSummary?.message || codexStatus?.message || 'Codex App Server 未连接' }}</small>
                </div>
                <span>{{ codexSummary?.badgeLabel || codexStatus?.lastEvent || 'idle' }}</span>
              </div>
              <div v-if="codexStatus" class="settings-runtime-list">
                <div class="settings-runtime-row">
                  <strong>连接方式</strong>
                  <code>{{ codexStatus.endpoint || '未连接' }}</code>
                </div>
                <div class="settings-runtime-row">
                  <strong>模式</strong>
                  <code>{{ codexStatus.mode || '暂无' }}</code>
                </div>
                <div class="settings-runtime-row">
                  <strong>任务数</strong>
                  <code>{{ codexSummary?.totalCount ?? codexStatus.tasks?.length ?? 0 }}</code>
                </div>
                <div class="settings-runtime-row">
                  <strong>未读提醒</strong>
                  <code>{{ codexSummary?.badgeLabel || '暂无' }}</code>
                </div>
                <div class="settings-runtime-row">
                  <strong>更新时间</strong>
                  <code>{{ formatCodexStatusTime(codexStatus) }}</code>
                </div>
              </div>
              <div v-if="codexRecentTasks.length > 0" class="codex-task-list">
                <div
                  v-for="task in codexRecentTasks"
                  :key="task.id"
                  class="codex-task-row"
                  :class="[`state-${task.state}`, { unread: task.unread }]"
                >
                  <div>
                    <strong>{{ task.label }}</strong>
                    <small>{{ task.message }}</small>
                    <small>{{ task.mode || '未知模式' }} · {{ formatCodexTaskTime(task.updatedAt) }}</small>
                  </div>
                  <span>{{ codexStateLabel(task.state) }}</span>
                </div>
              </div>
              <p v-else class="settings-empty">暂无 Codex 任务记录。</p>
              <div
                v-if="(codexSummary?.unreadCount ?? 0) > 0"
                class="settings-update-actions codex-actions"
              >
                <button type="button" :disabled="codexActionLoading" @click="ackCodexNotifications">
                  清除提醒
                </button>
              </div>
              <p v-if="codexStatus?.error" class="form-error">{{ codexStatus.error }}</p>

              <label class="settings-toggle-row">
                <span>
                  <strong>启用 Codex 状态提醒</strong>
                  <small>保存后可监听 Codex 会话日志、control socket，或启动独立 App Server 验证状态链路。</small>
                </span>
                <input v-model="settingsDraft.codexAppServerEnabled" type="checkbox" />
              </label>
              <label class="settings-toggle-row">
                <span>
                  <strong>启动时自动连接 Codex</strong>
                  <small>保存后下次启动桌宠时，会按当前连接模式自动开始监听或连接 Codex。</small>
                </span>
                <input
                  v-model="settingsDraft.codexAppServerAutoStart"
                  type="checkbox"
                  :disabled="!settingsDraft.codexAppServerEnabled"
                />
              </label>
              <label class="settings-toggle-row">
                <span>
                  <strong>完成时提醒</strong>
                  <small>Codex 完成一轮工作后，宠物显示完成气泡并播放完成动画。</small>
                </span>
                <input v-model="settingsDraft.codexCompletionNotificationsEnabled" type="checkbox" />
              </label>
              <div class="settings-form-grid">
                <label class="settings-field">
                  连接模式
                  <select v-model="settingsDraft.codexAppServerMode">
                    <option
                      v-for="option in codexModeOptions"
                      :key="option.value"
                      :value="option.value"
                    >
                      {{ option.label }}
                    </option>
                  </select>
                  <small>
                    {{
                      codexModeOptions.find((option) => option.value === settingsDraft.codexAppServerMode)
                        ?.description
                    }}
                  </small>
                </label>
              </div>
              <div class="settings-form-grid">
                <label v-if="!isCodexSessionLogMode" class="settings-field">
                  Codex 命令
                  <input
                    v-model="settingsDraft.codexAppServerCommand"
                    placeholder="codex.cmd 或完整 codex.exe 路径"
                    autocomplete="off"
                  />
                  <small>Windows 下 npm 安装通常使用 codex.cmd；桌面版也可填完整 codex.exe 路径。</small>
                </label>
                <label v-if="isCodexProxyMode" class="settings-field">
                  Control socket 路径
                  <input
                    v-model="settingsDraft.codexAppServerSocketPath"
                    placeholder="留空使用 Codex 默认 control socket"
                    autocomplete="off"
                  />
                  <small>当前 Codex 未暴露默认 socket 时，可填写 Codex 侧提供的自定义 --sock 路径。</small>
                </label>
                <label v-if="isCodexManagedMode" class="settings-field">
                  独立服务端口
                  <input
                    v-model.number="settingsDraft.codexAppServerPort"
                    type="number"
                    min="0"
                    max="65535"
                  />
                  <small>仅独立测试模式使用；填 0 表示自动分配本机端口。</small>
                </label>
              </div>
              <div class="settings-update-actions codex-actions">
                <button
                  type="button"
                  :disabled="!canStartCodexAppServer"
                  @click="connectCodexAppServer"
                >
                  {{ codexConnectLabel }}
                </button>
                <button
                  type="button"
                  :disabled="!canDisconnectCodexAppServer"
                  @click="disconnectCodexAppServer"
                >
                  断开连接
                </button>
                <button type="button" :disabled="codexActionLoading" @click="loadCodexStatus">
                  刷新状态
                </button>
              </div>

              <template v-if="isCodexSessionLogMode">
                <p class="settings-empty">
                  日志监听模式会读取本机 Codex 会话日志新增事件，只识别任务开始、完成、中断等状态，不展示 prompt、回复正文、工具输出或真实日志路径。
                </p>
                <p class="settings-empty">
                  适用于 Windows Codex Desktop：先点击“开始监听日志”，再回到 Codex 桌面端开始任务；Codex 写入完成事件后，宠物会提醒你。
                </p>
              </template>
              <template v-else-if="isCodexManagedMode">
                <div v-if="codexRemoteCommand" class="settings-runtime-list">
                  <div class="settings-runtime-row">
                    <strong>连接这个服务</strong>
                    <code>{{ codexRemoteCommand }}</code>
                  </div>
                </div>
                <p class="settings-empty">
                  在终端运行上面的命令后，在那个 Codex CLI 窗口里开始任务；宠物会监听这个独立 App Server 上的工作状态。
                </p>
                <h3>测试任务</h3>
                <label class="settings-field wide">
                  任务内容
                  <textarea
                    v-model="codexTestPrompt"
                    maxlength="500"
                    placeholder="输入一条短任务，用于验证状态提醒"
                  />
                  <small>测试内容只通过运行时发送给独立 App Server，不写入源码或文档。</small>
                </label>
                <label class="settings-field wide">
                  工作目录（可选）
                  <input v-model="codexTestCwd" placeholder="留空时由 Codex 使用默认工作目录" />
                </label>
                <div class="settings-update-actions codex-actions">
                  <button
                    class="primary-button"
                    type="button"
                    :disabled="!canStartCodexTestTurn"
                    @click="startCodexTestTurn"
                  >
                    {{ codexActionLoading ? '提交中...' : '提交测试任务' }}
                  </button>
                </div>
              </template>
              <p v-else class="settings-empty">
                监听当前 Codex 需要 Codex 侧暴露 control socket；Windows Codex Desktop 通常不会生成默认 socket，建议优先使用日志监听模式。
              </p>
              <p v-if="codexActionError" class="form-error">{{ codexActionError }}</p>
            </section>

            <section
              v-if="WECHAT_INTEGRATION_ENABLED"
              v-show="activeSettingsSection === 'wechat'"
              class="settings-section"
            >
              <h3>微信 ClawBot</h3>
              <div class="settings-update-panel">
                <div>
                  <strong>通过 OpenClaw 官方 ClawBot 通道发送微信消息</strong>
                  <small>先安装并登录 ClawBot；本页只保存本机命令、通道和目标会话，不保存到仓库。</small>
                </div>
              </div>
              <label class="settings-toggle-row">
                <span>
                  <strong>启用微信 ClawBot 同步</strong>
                  <small>开启后，对话窗口可按下方规则把用户消息或宠物回复同步到微信目标会话。</small>
                </span>
                <input v-model="settingsDraft.wechatClawbotEnabled" type="checkbox" />
              </label>

              <div class="settings-form-grid">
                <label class="settings-field">
                  OpenClaw 命令
                  <input
                    v-model="settingsDraft.wechatClawbotOpenclawCommand"
                    placeholder="openclaw"
                    autocomplete="off"
                  />
                </label>
                <label class="settings-field">
                  ClawBot 通道
                  <input
                    v-model="settingsDraft.wechatClawbotChannel"
                    placeholder="openclaw-weixin"
                    autocomplete="off"
                  />
                </label>
                <label class="settings-field">
                  微信账号（可选）
                  <input
                    v-model="settingsDraft.wechatClawbotAccount"
                    placeholder="留空使用默认登录账号"
                    autocomplete="off"
                  />
                </label>
                <label class="settings-field">
                  目标会话
                  <input
                    v-model="settingsDraft.wechatClawbotTarget"
                    placeholder="联系人、群或 OpenClaw 支持的 target"
                    autocomplete="off"
                  />
                </label>
              </div>

              <label class="settings-toggle-row">
                <span>
                  <strong>同步用户消息</strong>
                  <small>发送宠物聊天时，也把用户输入同步到微信。</small>
                </span>
                <input v-model="settingsDraft.wechatClawbotForwardUserMessages" type="checkbox" />
              </label>
              <label class="settings-toggle-row">
                <span>
                  <strong>同步宠物回复</strong>
                  <small>AI 回复完成后，把宠物回复同步到微信。</small>
                </span>
                <input v-model="settingsDraft.wechatClawbotForwardAssistantMessages" type="checkbox" />
              </label>
              <label class="settings-toggle-row">
                <span>
                  <strong>微信陪伴模式</strong>
                  <small>Bridge 收到微信消息时，会提示 AI 使用更像微信好友单聊的短句、自然追问和当前伴侣语气。</small>
                </span>
                <input v-model="settingsDraft.wechatClawbotFriendModeEnabled" type="checkbox" />
              </label>

              <div class="settings-update-panel">
                <div>
                  <strong>测试发送</strong>
                  <small>使用当前表单配置执行一次 `openclaw message send`。</small>
                </div>
                <div class="settings-update-actions">
                  <input
                    v-model="wechatClawbotTestMessage"
                    placeholder="测试消息"
                    autocomplete="off"
                  />
                  <button type="button" :disabled="wechatClawbotTesting" @click="testWechatClawbot">
                    {{ wechatClawbotTesting ? '发送中...' : '发送测试' }}
                  </button>
                </div>
              </div>
              <p v-if="wechatClawbotStatus" class="settings-empty">{{ wechatClawbotStatus }}</p>
              <p v-if="wechatClawbotError" class="form-error">{{ wechatClawbotError }}</p>
              <p class="settings-empty">
                官方准备步骤：安装 OpenClaw 后运行
                `npx -y @tencent-weixin/openclaw-weixin-cli install`，再按插件提示扫码登录微信。
              </p>

              <h3>通用 HTTP 接入</h3>
              <div class="settings-update-panel">
                <div>
                  <strong>让远程 ClawBot 调用 PetDrawer AI</strong>
                  <small>ClawBot 收到微信消息后 POST 到此接口，PetDrawer 返回当前伴侣的 AI 回复。</small>
                </div>
              </div>
              <label class="settings-toggle-row">
                <span>
                  <strong>启用 ClawBot HTTP Bridge</strong>
                  <small>默认只监听本机 127.0.0.1；远程服务器建议通过 SSH 反向隧道或私有网络访问。</small>
                </span>
                <input v-model="settingsDraft.wechatClawbotBridgeEnabled" type="checkbox" />
              </label>
              <div class="settings-form-grid">
                <label class="settings-field">
                  监听地址
                  <input
                    v-model="settingsDraft.wechatClawbotBridgeHost"
                    placeholder="127.0.0.1"
                    autocomplete="off"
                  />
                </label>
                <label class="settings-field">
                  端口
                  <input
                    v-model.number="settingsDraft.wechatClawbotBridgePort"
                    type="number"
                    min="1"
                    max="65535"
                    step="1"
                  />
                </label>
                <label class="settings-field">
                  路径
                  <input
                    v-model="settingsDraft.wechatClawbotBridgePath"
                    placeholder="/clawbot/chat"
                    autocomplete="off"
                  />
                </label>
                <label class="settings-field">
                  Bridge Token（可选）
                  <input
                    v-model="settingsDraft.wechatClawbotBridgeToken"
                    type="password"
                    placeholder="留空则不校验"
                    autocomplete="off"
                  />
                </label>
              </div>
              <p class="settings-empty">
                请求格式：POST JSON `{"message":"用户消息","sender":"微信昵称","sessionId":"会话ID"}`；
                返回格式包含 `reply`、`message` 和 `text`，ClawBot 脚本任选其一发回微信即可。如果设置 Token，请带
                `Authorization: Bearer &lt;token&gt;` 或 `X-PetDrawer-Token`。
              </p>

              <div class="settings-update-panel">
                <div>
                  <strong>本机模拟微信入站消息</strong>
                  <small>没有 OpenClaw 或服务器时，用这里验证 PetDrawer 软件侧能按微信陪伴模式调用当前伴侣并返回回复。</small>
                </div>
              </div>
              <div class="settings-form-grid">
                <label class="settings-field">
                  模拟发送者
                  <input
                    v-model="wechatBridgeSimulationSender"
                    placeholder="微信昵称"
                    autocomplete="off"
                  />
                </label>
                <label class="settings-field">
                  模拟会话 ID
                  <input
                    v-model="wechatBridgeSimulationSessionId"
                    placeholder="local-wechat-simulation"
                    autocomplete="off"
                  />
                </label>
                <label class="settings-field wide">
                  模拟微信消息
                  <textarea
                    v-model="wechatBridgeSimulationMessage"
                    rows="3"
                    placeholder="输入一条模拟微信消息"
                  />
                </label>
              </div>
              <div class="settings-update-actions">
                <button
                  type="button"
                  :disabled="wechatBridgeSimulating"
                  @click="simulateWechatBridgeMessage"
                >
                  {{ wechatBridgeSimulating ? '调用中...' : '模拟入站并生成回复' }}
                </button>
              </div>
              <p v-if="wechatBridgeSimulationStatus" class="settings-empty">
                {{ wechatBridgeSimulationStatus }}
              </p>
              <p v-if="wechatBridgeSimulationReply" class="settings-empty">
                模拟回复：{{ wechatBridgeSimulationReply }}
              </p>
              <p v-if="wechatBridgeSimulationError" class="form-error">
                {{ wechatBridgeSimulationError }}
              </p>
            </section>

            <section v-show="activeSettingsSection === 'storage'" class="settings-section">
              <h3>数据存储目录</h3>
              <div class="settings-update-panel">
                <div>
                  <strong>自定义本机数据位置</strong>
                  <small>目录留空时使用系统默认应用数据目录；保存后会迁移数据，并清理旧位置中已迁移的应用数据。</small>
                </div>
              </div>
              <div class="settings-form-grid">
                <label class="settings-field wide">
                  基础数据目录
                  <small>保存 config.json、apps.json 等基础配置；storage.json 仍保留在系统默认应用数据目录用于启动定位。</small>
                  <div class="path-row storage-path-row">
                    <input
                      v-model="storageDataDirDisplay"
                      placeholder="留空使用系统默认应用数据目录"
                      autocomplete="off"
                    />
                    <button type="button" @click="pickStorageDirectory('storageDataDir')">选择</button>
                    <button type="button" @click="clearStorageDirectory('storageDataDir')">默认</button>
                  </div>
                </label>
                <label class="settings-field wide">
                  记忆目录
                  <small>保存 pet-memory.db、旧版 pet-memory.json 和好感度状态；留空时跟随基础数据目录。</small>
                  <div class="path-row storage-path-row">
                    <input
                      v-model="storageMemoryDirDisplay"
                      placeholder="留空使用基础数据目录"
                      autocomplete="off"
                    />
                    <button type="button" @click="pickStorageDirectory('storageMemoryDir')">选择</button>
                    <button type="button" @click="clearStorageDirectory('storageMemoryDir')">默认</button>
                  </div>
                </label>
                <label class="settings-field wide">
                  宠物素材目录
                  <small>保存导入的宠物图片、待机、选中、点击和拖动动画；留空时使用基础数据目录下的 pets。</small>
                  <div class="path-row storage-path-row">
                    <input
                      v-model="storagePetAssetsDirDisplay"
                      placeholder="留空使用基础数据目录下的 pets"
                      autocomplete="off"
                    />
                    <button type="button" @click="pickStorageDirectory('storagePetAssetsDir')">选择</button>
                    <button type="button" @click="clearStorageDirectory('storagePetAssetsDir')">默认</button>
                  </div>
                </label>
                <label class="settings-field wide">
                  图标目录
                  <small>保存导入或自动提取的软件图标；留空时使用基础数据目录下的 icons。</small>
                  <div class="path-row storage-path-row">
                    <input
                      v-model="storageIconsDirDisplay"
                      placeholder="留空使用基础数据目录下的 icons"
                      autocomplete="off"
                    />
                    <button type="button" @click="pickStorageDirectory('storageIconsDir')">选择</button>
                    <button type="button" @click="clearStorageDirectory('storageIconsDir')">默认</button>
                  </div>
                </label>
              </div>
              <p class="settings-empty">
                已有文件会复制到新目录中缺失的位置；迁移完成后会删除旧位置中内容一致的应用数据，不会覆盖新目录中的同名文件。
              </p>
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
              <h3>宠物大小</h3>
              <label class="settings-field wide">
                桌面宠物尺寸
                <div class="settings-range-row">
                  <input
                    v-model.number="settingsDraft.petSize"
                    class="settings-range-input"
                    type="range"
                    :min="petSizeMin"
                    :max="petSizeMax"
                    :step="petSizeStep"
                    :style="petSizeRangeStyle"
                  />
                  <input
                    v-model.number="settingsDraft.petSize"
                    class="settings-range-number"
                    type="number"
                    :min="petSizeMin"
                    :max="petSizeMax"
                    :step="petSizeStep"
                  />
                  <span class="settings-range-value">
                    {{ normalizePetSize(settingsDraft.petSize) }} px
                  </span>
                </div>
                <small>保存设置后立即调整宠物窗口大小，范围 {{ petSizeMin }}-{{ petSizeMax }} px。</small>
              </label>
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
                <div class="settings-runtime-row">
                  <strong>默认数据目录</strong>
                  <code :title="runtimeInfo.defaultDataDir">{{ runtimeInfo.defaultDataDir }}</code>
                </div>
                <div class="settings-runtime-row">
                  <strong>记忆目录</strong>
                  <code :title="runtimeInfo.memoryDir">{{ runtimeInfo.memoryDir }}</code>
                </div>
                <div class="settings-runtime-row">
                  <strong>宠物素材目录</strong>
                  <code :title="runtimeInfo.petAssetsDir">{{ runtimeInfo.petAssetsDir }}</code>
                </div>
                <div class="settings-runtime-row">
                  <strong>图标目录</strong>
                  <code :title="runtimeInfo.iconsDir">{{ runtimeInfo.iconsDir }}</code>
                </div>
                <div class="settings-runtime-row">
                  <strong>存储配置</strong>
                  <code :title="runtimeInfo.storageConfigFile">{{ runtimeInfo.storageConfigFile }}</code>
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

        <Transition name="settings-loading">
          <div v-if="settingsLoading" class="settings-loading-layer" role="status" aria-live="polite">
            <div class="settings-loading-card">
              <span class="settings-loading-spinner" aria-hidden="true"></span>
              <strong>正在加载设置</strong>
              <small>读取本机配置、伴侣和记忆数据...</small>
            </div>
          </div>
        </Transition>

        <p v-if="settingsError" class="form-error">{{ settingsError }}</p>

        <footer>
          <button type="button" @click="closeSettings">取消</button>
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
