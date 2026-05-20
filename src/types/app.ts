export interface PetApp {
  id: string
  name: string
  path: string
  icon?: string | null
  iconDataUrl?: string | null
  category: string
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
  path: string
  icon?: string | null
  category: string
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
}
