export type AppItemKind = 'app' | 'folder' | 'website' | 'file'
export type AppItemKindFilter = 'all' | AppItemKind

export interface PetApp {
  id: string
  name: string
  itemKind: AppItemKind
  path: string
  icon?: string | null
  iconDataUrl?: string | null
  category: string
  runAsAdmin: boolean
  tags: string[]
  favorite: boolean
  autoFavorite?: boolean
  launchCount: number
  launchHistory?: string[]
  lastLaunchAt?: string | null
  createdAt: string
}

export interface AppDraft {
  id?: string
  name: string
  itemKind: AppItemKind
  path: string
  icon?: string | null
  category: string
  runAsAdmin: boolean
  tags: string[]
  favorite: boolean
}

export interface PetPosition {
  x: number
  y: number
}

export type PetAnimationKey =
  | 'idle'
  | 'hover'
  | 'click'
  | 'dragging'
  | 'draggingLeft'
  | 'draggingRight'
  | 'waving'
  | 'jumping'
  | 'waiting'
  | 'running'
  | 'review'
  | 'failed'

export type PetAnimationSet = Partial<Record<PetAnimationKey, string | null>>

export interface PetSkinSummary {
  id: string
  name: string
  builtin: boolean
  preview?: string | null
  animations: PetAnimationSet
}

export interface PetSkinPackageDraft {
  name: string
  animations: PetAnimationSet
}

export interface UpdateCheckResult {
  currentVersion: string
  latestVersion?: string | null
  updateUrl?: string | null
  releaseUrl?: string | null
  assetName?: string | null
  status: 'available' | 'latest' | 'error'
  message: string
}

export interface RuntimeInfo {
  version: string
  executablePath: string
  defaultDataDir: string
  dataDir: string
  memoryDir: string
  petAssetsDir: string
  iconsDir: string
  storageConfigFile: string
}

export interface StorageSettings {
  dataDir: string
  memoryDir: string
  petAssetsDir: string
  iconsDir: string
}

export type AiProvider = 'openai' | 'deepseek' | 'anthropic' | 'gemini' | 'ollama' | 'custom'
export type DrawerTheme = 'light' | 'animal-island'
export type MusicImmersiveTheme =
  | DrawerTheme
  | 'cinema'
  | 'galaxy'
  | 'neon'
  | 'sunset'
  | 'midnight'
export type MusicImmersiveThemePreference = 'follow' | MusicImmersiveTheme
export type MusicSpectrumStyle = 'bars' | 'mirror' | 'orbit' | 'particles' | 'ribbon' | 'none'
export type MusicLineStyle = 'wave' | 'beam' | 'scan' | 'constellation' | 'none'
export type MusicRippleStyle = 'rings' | 'water' | 'heartbeat' | 'halo' | 'none'
export type MusicVisualStagePreset = 'galaxy' | 'dj'
export interface MusicStageTuning {
  height: number
  response: number
  density: number
  wave: number
  trigger: number
  camera: number
  layerHeight: number
  centerPulse: boolean
}

export const DEFAULT_MUSIC_STAGE_TUNING: Readonly<MusicStageTuning> = {
  height: 1,
  response: 1,
  density: 1,
  wave: 1,
  trigger: 1,
  camera: 1,
  layerHeight: 1,
  centerPulse: false,
}
export type ChatEmojiFrequency = 'none' | 'low' | 'normal' | 'high'
export type PetActionBinding =
  | 'smartCodexOrDrawer'
  | 'toggleDrawer'
  | 'showDrawer'
  | 'petMenu'
  | 'petChat'
  | 'story'
  | 'music'
  | 'none'

export interface AiConnectionProfile {
  id: string
  label: string
  provider: AiProvider | string
  apiKey: string
  baseUrl: string
  model: string
}

export interface AiSettings {
  enabled: boolean
  memoryEnabled?: boolean
  shortMemorySummaryEnabled?: boolean
  shortMemoryRecentTurns?: number
  shortMemoryCompressionTriggerTurns?: number
  provider: AiProvider | string
  apiKey: string
  baseUrl: string
  model: string
  systemPrompt: string
  temperature: number
  maxTokens: number
  emojiFrequency?: ChatEmojiFrequency | string
  activeProfileId?: string
  profiles?: AiConnectionProfile[]
}

export interface AiConnectionTestResult {
  ok: boolean
  provider: string
  model: string
  message: string
}

export interface WechatClawbotSettings {
  enabled: boolean
  openclawCommand: string
  channel: string
  account: string
  target: string
  forwardUserMessages: boolean
  forwardAssistantMessages: boolean
  friendModeEnabled: boolean
  bridgeEnabled: boolean
  bridgeHost: string
  bridgePort: number
  bridgePath: string
  bridgeToken: string
}

export interface CodexAppServerSettings {
  enabled: boolean
  autoStart?: boolean
  mode: 'proxy' | 'managed' | 'sessionLog' | string
  command: string
  socketPath?: string
  port: number
  completionNotificationsEnabled: boolean
}

export type CodexAppServerState =
  | 'disconnected'
  | 'starting'
  | 'connected'
  | 'running'
  | 'waiting'
  | 'review'
  | 'completed'
  | 'failed'
  | string

export interface CodexAppServerStatus {
  state: CodexAppServerState
  message: string
  active?: boolean
  threadId?: string | null
  turnId?: string | null
  endpoint?: string | null
  mode?: string | null
  lastEvent?: string | null
  error?: string | null
  notify?: boolean
  updatedAt: number
  summary?: CodexStatusSummary
  tasks?: CodexTaskStatus[]
}

export interface CodexStatusSummary {
  state: CodexAppServerState
  message: string
  attention: 'none' | 'idle' | 'working' | 'waiting' | 'completed' | 'failed' | string
  totalCount: number
  activeCount: number
  runningCount: number
  reviewCount: number
  waitingCount: number
  completedCount: number
  failedCount: number
  unreadCount: number
  unreadCompletedCount: number
  unreadFailedCount: number
  badgeLabel?: string | null
}

export interface CodexTaskStatus {
  id: string
  label: string
  state: CodexAppServerState
  message: string
  mode?: string | null
  lastEvent?: string | null
  updatedAt: number
  unread: boolean
}

export interface WechatClawbotSendResult {
  ok: boolean
  message: string
}

export interface WechatBridgeChatResult {
  ok: boolean
  reply: string
  message: string
  text: string
  provider: string
  model: string
  source: string
  shouldReply: boolean
}

export interface CompanionRelationshipState {
  favorability: number
  intimacy: number
  mood: string
}

export interface CompanionStatus {
  characterId: string
  favorabilityEnabled: boolean
  favorability: number
  relationshipStage: string
  relationshipStageName: string
  mood: number
  trust: number
  intimacy: number
  dailyGain: number
  lastInteractionTime?: string | null
  lastChangeReason?: string | null
  updatedAt: string
}

export interface FavorabilityLog {
  id: number
  characterId: string
  messageId?: number | null
  oldFavorability: number
  changeValue: number
  newFavorability: number
  oldStage: string
  newStage: string
  oldMood: number
  moodChange: number
  newMood: number
  oldTrust: number
  trustChange: number
  newTrust: number
  oldIntimacy: number
  intimacyChange: number
  newIntimacy: number
  reason: string
  source: 'dialogue' | 'manual' | 'reset' | 'system' | string
  createdAt: string
}

export interface FavorabilityChangeResult {
  changed: boolean
  status: CompanionStatus
  log?: FavorabilityLog | null
}

export interface Companion {
  id: string
  name: string
  avatar?: string | null
  personaPrompt: string
  personality: string
  scenario: string
  firstMessage: string
  messageExample: string
  creatorNotes: string
  postHistoryInstructions: string
  systemPrompt: string
  model: string
  voiceId: string
  memoryScope: string
  skinId: string
  relationshipState: CompanionRelationshipState
  createdAt: string
  updatedAt: string
}

export interface CompanionDraft {
  id?: string
  name: string
  avatar?: string | null
  personaPrompt: string
  personality: string
  scenario: string
  firstMessage: string
  messageExample: string
  creatorNotes: string
  postHistoryInstructions: string
  systemPrompt: string
  model: string
  voiceId: string
  skinId: string
  relationshipState: CompanionRelationshipState
}

export type StoryMode = 'random' | 'custom'

export interface StoryCharacterDraft {
  name: string
  gender: string
  ageStage: string
  identity: string
  appearance: string
  personality: string
  relationshipToUser: string
  relationshipToOthers: string
  roleInStory: string
  speakingStyle: string
  hiddenSetting: string
  isInteractable: boolean
}

export interface StoryCreateDraft {
  mode: StoryMode
  storyType: string
  tone: string
  premise: string
  companionIds: string[]
  companionRole: string
  temporaryCharacters: StoryCharacterDraft[]
}

export interface StoryCharacter {
  id: string
  name: string
  source: 'existing_avatar' | 'temporary' | 'generated' | string
  avatarId?: string | null
  roleInStory: string
  personality: string
  appearance: string
  speakingStyle: string
  relationshipToUser: string
  relationshipToOthers: string
  hiddenSetting: string
  isInteractable: boolean
}

export interface StoryChoice {
  id: string
  chapter: number
  scene: string
  userAction: string
  resultSummary: string
  timestamp: number
}

export interface StoryMessage {
  role: 'user' | 'assistant' | string
  content: string
  timestamp: number
}

export interface StorySave {
  id: string
  title: string
  storyType: string
  mode: StoryMode | string
  createdAt: number
  updatedAt: number
  userRole: string
  currentChapter: number
  currentScene: string
  currentLocation: string
  currentTime: string
  characters: StoryCharacter[]
  activeCharacterIds: string[]
  relationshipValues: Record<string, Record<string, string | number>>
  emotionStates: Record<string, string>
  importantChoices: StoryChoice[]
  unlockedEvents: string[]
  hiddenFlags: Record<string, boolean>
  inventory: string[]
  clues: string[]
  storySummary: string
  recentMessages: StoryMessage[]
}

export interface StoryTurnReply {
  story: StorySave
  reply: string
}

export type MemoryType =
  | 'nickname'
  | 'preference'
  | 'dislike'
  | 'relationship'
  | 'emotion'
  | 'habit'
  | 'life_event'
  | 'important_person'
  | 'interest'
  | 'goal'
  | 'boundary'
  | 'instruction'
  | 'short_term_summary'
  | 'other'
  | string

export type MemoryAction = 'remember' | 'forget' | 'update' | 'none'

export interface PetMemory {
  id: number
  companionId?: string
  memoryType: MemoryType
  content: string
  importance: number
  tags: string[]
  sourceMessage?: string
  confidence?: number
  deleted?: boolean
  createdAt: string
  updatedAt: string
}

export interface PetMemoryDraft {
  memoryType: MemoryType
  content: string
  importance: number
  tags: string[]
  sourceMessage?: string
  confidence?: number
}

export interface ExtractedMemory {
  type: MemoryType
  content: string
  importance: number
  tags: string[]
  confidence?: number
}

export interface MemoryExtractionResult {
  action: MemoryAction
  reason: string
  memories: ExtractedMemory[]
}

export interface PetChatMessage {
  role: 'user' | 'assistant'
  content: string
  createdAt?: string
  timeContext?: string
}

export interface PetMemoryMessage extends PetChatMessage {
  id: number
  companionId: string
  createdAt: string
}

export interface PetChatReply {
  message: string
  provider: string
  model: string
  memoryWarning?: string
  favorabilityChange?: FavorabilityChangeResult | null
}

export interface PetDrawerConfig {
  pet: {
    x: number | null
    y: number | null
    size: number
    currentSkin: string
    customImage?: string | null
    alwaysOnTop: boolean
  }
  drawer: {
    width: number
    height: number
    theme: DrawerTheme | string
    musicImmersiveTheme?: MusicImmersiveThemePreference | string
    chatTypewriterEnabled?: boolean
    chatNarrationEnabled?: boolean
    chatMusicLinkEnabled?: boolean
    alwaysOnTop: boolean
    categories?: string[]
    quickSearchTags?: string[]
    tagDisplayMode?: 'compact' | 'detailed'
  }
  shortcut: {
    toggleDrawer: string
    petSingleClick?: PetActionBinding | string
    petDoubleClick?: PetActionBinding | string
    petRightClick?: PetActionBinding | string
  }
  system?: {
    startOnBoot: boolean
    autoFavoriteEnabled: boolean
  }
  ai?: AiSettings
  wechatClawbot?: WechatClawbotSettings
  codexAppServer?: CodexAppServerSettings
  companions?: Companion[]
  currentCompanionId?: string
  companionsInitialized?: boolean
}
