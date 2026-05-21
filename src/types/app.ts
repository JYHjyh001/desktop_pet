export type AppItemKind = 'app' | 'folder' | 'website'
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

export interface PetAnimationSet {
  idle?: string | null
  hover?: string | null
  dragging?: string | null
  click?: string | null
}

export interface PetSkinSummary {
  id: string
  name: string
  builtin: boolean
  preview?: string | null
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
  dataDir: string
}

export type AiProvider = 'openai' | 'deepseek' | 'anthropic' | 'gemini' | 'ollama' | 'custom'

export interface AiSettings {
  enabled: boolean
  provider: AiProvider | string
  apiKey: string
  baseUrl: string
  model: string
  systemPrompt: string
  temperature: number
  maxTokens: number
}

export interface PetChatMessage {
  role: 'user' | 'assistant'
  content: string
}

export interface PetChatReply {
  message: string
  provider: string
  model: string
}

export interface PetDrawerConfig {
  pet: {
    x: number | null
    y: number | null
    currentSkin: string
    customImage?: string | null
    alwaysOnTop: boolean
  }
  drawer: {
    width: number
    height: number
    theme: string
    alwaysOnTop: boolean
    categories?: string[]
    quickSearchTags?: string[]
    tagDisplayMode?: 'compact' | 'detailed'
  }
  shortcut: {
    toggleDrawer: string
  }
  ai?: AiSettings
}
