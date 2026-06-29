<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { LogicalSize } from '@tauri-apps/api/dpi'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { open } from '@tauri-apps/plugin-dialog'
import MusicVisualizerCanvas from '../components/MusicVisualizerCanvas.vue'
import {
  useMusicAudioAnalyzer,
  type MusicEnergyFrame,
} from '../composables/useMusicAudioAnalyzer'
import { useMusicBeatMapAnalyzer } from '../composables/useMusicBeatMapAnalyzer'
import { useMusicLyrics } from '../composables/useMusicLyrics'
import { useWindowOpenAnimation } from '../composables/useWindowOpenAnimation'
import type { DrawerTheme, PetDrawerConfig } from '../types/app'

type RepeatMode = 'none' | 'one' | 'all'
type MusicLibraryView = 'all' | 'favorites' | 'recent' | 'queue'
type MusicPanelView = 'library' | 'playlists' | 'ai' | 'netease'
type MusicRecommendationSource = 'smart' | 'tags' | 'favorites' | 'recent'
type MusicVisualMode = 'rhythm' | 'dance' | 'focus' | 'sleep'
type MusicActionType =
  | 'play_music'
  | 'play_by_query'
  | 'play_by_tags'
  | 'pause'
  | 'resume'
  | 'next'
  | 'previous'
  | 'set_volume'
  | 'favorite_current'
  | 'skip_current'
  | 'start_sleep_mode'
  | 'start_focus_mode'
  | 'start_mood_mode'

interface MusicTrack {
  id: string
  title: string
  artist: string
  album: string
  path: string
  sourcePath: string
  source?: 'local' | 'netease'
  neteaseSongId?: number
  category: string
  tags: string[]
  url: string
  duration: number | null
  favorite: boolean
  playCount: number
  lastPlayedAt: string | null
  playHistory: string[]
}

interface MusicImportItem {
  sourcePath: string
  path: string
}

interface MusicMetadataResult {
  title?: string | null
  artist?: string | null
  album?: string | null
  duration?: number | null
  source: string
  confidence: number
  warnings: string[]
}

interface MusicRecognitionCandidate {
  trackId: string
  title: string
  artist: string
  album: string
  duration: number | null
  source: string
  confidence: number
  warnings: string[]
}

interface MusicPlaylist {
  id: string
  name: string
  trackIds: string[]
  createdAt: string
  updatedAt: string
}

interface MusicTagPresetGroup {
  id: string
  title: string
  tags: string[]
}

interface ScenePlaylistOption {
  id: string
  title: string
  description: string
  source: MusicRecommendationSource
  tags: string[]
}

interface AiRecommendationOption {
  id: string
  title: string
  description: string
  source: MusicRecommendationSource
  tags: string[]
}

interface MusicActionRequest {
  type: 'music_action'
  action: MusicActionType
  tags?: string[]
  query?: string
  volume?: number
  volumeDelta?: number
  source?: string
}

interface NeteaseLoginProfile {
  userId: number
  nickname: string
  avatarUrl?: string | null
}

interface NeteaseLoginStatus {
  loggedIn: boolean
  profile: NeteaseLoginProfile | null
  savedAt: string | null
  checkedAt: string
  message: string
}

interface NeteaseQrLogin {
  key: string
  qrUrl: string
  qrImage: string
  expiresAt: string
}

interface NeteaseQrCheckResult {
  code: number
  status: string
  message: string
  loggedIn: boolean
  profile: NeteaseLoginProfile | null
}

interface NeteasePlaylistSummary {
  id: number
  name: string
  trackCount: number
  playCount: number
  subscribedCount: number
  coverImgUrl?: string | null
  creatorNickname?: string | null
  subscribed: boolean
  updateTime?: number | null
}

interface NeteasePlaylistTrack {
  id: number
  name: string
  artists: string[]
  album?: string | null
  durationMs?: number | null
  coverImgUrl?: string | null
  fee?: number | null
}

interface NeteasePlaylistDetail {
  playlist: NeteasePlaylistSummary
  tracks: NeteasePlaylistTrack[]
  totalTrackCount: number
  truncated: boolean
  message: string
}

interface NeteaseLyricsResult {
  songId: number
  content: string
  lrcContent?: string | null
  yrcContent?: string | null
  klyricContent?: string | null
  translatedContent?: string | null
  source: string
  warnings: string[]
}

interface NeteasePlaybackUrl {
  songId: number
  url: string
  level: string
  bitrate?: number | null
  durationMs?: number | null
  fileType?: string | null
  size?: number | null
  message: string
}

type ImmersivePlaylistSource = 'local' | 'netease'

interface TrackIdentity {
  title: string
  artist: string
}

const ALL_CATEGORY = '全部'
const ALL_TAG = '全部标签'
const DEFAULT_CATEGORY = '未分类'
const MAX_PLAY_HISTORY_PER_TRACK = 30
const MAX_TRACK_TAGS = 12
const MAX_TRACK_TAG_LENGTH = 24
const FULL_MUSIC_WINDOW_SIZE = { width: 520, height: 740 }
const MINI_MUSIC_WINDOW_SIZE = { width: 344, height: 154 }
const TRACKS_STORAGE_KEY = 'pet-drawer-music-tracks'
const SETTINGS_STORAGE_KEY = 'pet-drawer-music-settings'
const PLAYLISTS_STORAGE_KEY = 'pet-drawer-music-playlists'
const NETEASE_TRACK_ID_PREFIX = 'netease'
const panelViewOptions: Array<{ value: MusicPanelView; label: string }> = [
  { value: 'library', label: '本地音乐' },
  { value: 'playlists', label: '歌单' },
  { value: 'ai', label: 'AI 推荐' },
  { value: 'netease', label: '网易云' },
]
const libraryViewOptions: Array<{ value: MusicLibraryView; label: string }> = [
  { value: 'all', label: '本地' },
  { value: 'favorites', label: '喜欢' },
  { value: 'recent', label: '最近' },
  { value: 'queue', label: '队列' },
]
const visualModeOptions: Array<{ value: MusicVisualMode; label: string; description: string }> = [
  { value: 'rhythm', label: '韵律', description: '频谱、光带和节拍脉冲跟随当前歌曲。' },
  { value: 'dance', label: '跳舞', description: '增强低频和节拍反馈，适合快歌。' },
  { value: 'focus', label: '专注', description: '降低动态强度，适合学习和工作。' },
  { value: 'sleep', label: '睡眠', description: '低亮度慢速动画，适合睡前播放。' },
]
const musicTagPresetGroups: MusicTagPresetGroup[] = [
  {
    id: 'mood',
    title: '情绪',
    tags: ['开心', '难过', '治愈', '热血', '安静', '孤独'],
  },
  {
    id: 'scene',
    title: '场景',
    tags: ['学习', '工作', '睡觉', '运动', '聊天', '游戏'],
  },
  {
    id: 'tempo',
    title: '节奏',
    tags: ['慢歌', '中速', '快歌'],
  },
  {
    id: 'language',
    title: '语言',
    tags: ['中文', '英文', '日文', '纯音乐'],
  },
  {
    id: 'user',
    title: '用户',
    tags: ['收藏', '常听', '跳过', '不喜欢'],
  },
]
const musicTagPresetOptions = musicTagPresetGroups.flatMap((group) => group.tags)
const audio = ref<HTMLAudioElement | null>(null)
const musicWindow = getCurrentWindow()
const { windowOpenAnimationClass } = useWindowOpenAnimation('panel')
const tracks = ref<MusicTrack[]>([])
const playQueue = ref<string[]>([])
const customPlaylists = ref<MusicPlaylist[]>([])
const currentIndex = ref(-1)
const activePanelView = ref<MusicPanelView>('library')
const activeLibraryView = ref<MusicLibraryView>('all')
const activeCustomPlaylistId = ref('')
const activeCategoryFilter = ref(ALL_CATEGORY)
const activeTagFilter = ref(ALL_TAG)
const searchQuery = ref('')
const newPlaylistName = ref('')
const playlistTrackPickerVisible = ref(false)
const playlistTrackPickerPlaylistId = ref('')
const playlistTrackPickerQuery = ref('')
const playlistTrackPickerSelectedIds = ref<string[]>([])
const importCategory = ref(DEFAULT_CATEGORY)
const musicStorageDir = ref('')
const drawerTheme = ref<DrawerTheme>('light')
const settingsVisible = ref(false)
const miniPlayerMode = ref(false)
const immersiveMode = ref(false)
const immersivePlaylistVisible = ref(true)
const immersiveRhythmPanelVisible = ref(true)
const immersivePlaylistSource = ref<ImmersivePlaylistSource>('local')
const visualMode = ref<MusicVisualMode>('rhythm')
const visualIntensity = ref(0.72)
const visualReducedMotion = ref(false)
const lyricOffsetMs = ref(0)
const neteaseLoginStatus = ref<NeteaseLoginStatus | null>(null)
const neteaseQrLogin = ref<NeteaseQrLogin | null>(null)
const neteaseLoginBusy = ref(false)
const neteaseQrChecking = ref(false)
const neteaseQrStatus = ref<'idle' | 'waiting' | 'scanned' | 'expired' | 'authorized' | 'error'>(
  'idle',
)
const neteaseLoginNotice = ref('')
const neteaseLoginError = ref('')
const neteasePlaylists = ref<NeteasePlaylistSummary[]>([])
const neteaseSelectedPlaylistId = ref<number | null>(null)
const neteasePlaylistDetail = ref<NeteasePlaylistDetail | null>(null)
const neteasePlaylistsLoading = ref(false)
const neteasePlaylistDetailLoading = ref(false)
const neteasePlaylistError = ref('')
const neteaseTrackActionId = ref<number | null>(null)
const neteaseCurrentTrack = ref<MusicTrack | null>(null)
const neteaseLyricsTrack = ref<NeteasePlaylistTrack | null>(null)
const neteaseLyricsResult = ref<NeteaseLyricsResult | null>(null)
const neteaseLyricsLoading = ref(false)
const neteaseLyricsError = ref('')
const playing = ref(false)
const currentTime = ref(0)
const visualPlaybackTime = ref(0)
const duration = ref(0)
const volume = ref(0.74)
const repeatMode = ref<RepeatMode>('all')
const shuffleEnabled = ref(false)
const playerError = ref('')
const playerStatus = ref('')
const draggingQueueTrackId = ref<string | null>(null)
const draggingPlaylistTrackId = ref<string | null>(null)
const editingTrackId = ref<string | null>(null)
const activeTrackActionsId = ref<string | null>(null)
const recognitionBusyTrackId = ref<string | null>(null)
const recognitionCandidate = ref<MusicRecognitionCandidate | null>(null)
const {
  frequencyData,
  energyFrame,
  analyzerReady,
  analyzerError,
  connectAudioElement,
  resumeAnalyzer,
  resetEnergyFrame,
} = useMusicAudioAnalyzer()
const {
  beatMap,
  beatMapStatus,
  beatMapError,
  beatMapProgress,
  analyzeTrack: analyzeTrackBeatMap,
  frameAt: beatMapFrameAt,
  frequencyDataAt: beatMapFrequencyDataAt,
  resetBeatMap,
} = useMusicBeatMapAnalyzer()
const {
  lyricsStatus,
  lyricsError,
  loadLyricsForTrack,
  resetLyrics,
  lyricsAt,
} = useMusicLyrics()

const currentTrack = computed(() => neteaseCurrentTrack.value ?? tracks.value[currentIndex.value] ?? null)
const editingTrack = computed(() =>
  editingTrackId.value ? trackById(editingTrackId.value) : null,
)
const hasTracks = computed(() => tracks.value.length > 0 || Boolean(neteaseCurrentTrack.value))
const hasQueue = computed(() => playQueue.value.length > 0)
const themeClass = computed(() => `theme-${drawerTheme.value}`)
const categoryOptions = computed(() => {
  const categories = tracks.value
    .map((track) => normalizeMusicCategory(track.category))
    .filter((category) => category !== DEFAULT_CATEGORY)
  return [DEFAULT_CATEGORY, ...Array.from(new Set(categories)).sort((a, b) => a.localeCompare(b))]
})
const categoryFilterOptions = computed(() => [ALL_CATEGORY, ...categoryOptions.value])
const tagOptions = computed(() => {
  const tags = [
    ...musicTagPresetOptions,
    ...tracks.value.flatMap((track) => normalizeTrackTags(track.tags)),
  ]
  return Array.from(new Set(tags)).sort((a, b) => a.localeCompare(b))
})
const tagFilterOptions = computed(() => [ALL_TAG, ...tagOptions.value])
const normalizedSearchQuery = computed(() => searchQuery.value.trim().toLowerCase())
const queuedTracks = computed(() =>
  playQueue.value
    .map((trackId) => trackById(trackId))
    .filter((track): track is MusicTrack => Boolean(track)),
)
const filteredTracks = computed(() => {
  if (activeLibraryView.value === 'queue') {
    return filterTracksBySearch(queuedTracks.value)
  }

  const categoryTracks =
    activeCategoryFilter.value === ALL_CATEGORY
      ? tracks.value
      : tracks.value.filter(
          (track) => normalizeMusicCategory(track.category) === activeCategoryFilter.value,
        )
  const taggedTracks =
    activeTagFilter.value === ALL_TAG
      ? categoryTracks
      : categoryTracks.filter((track) => normalizeTrackTags(track.tags).includes(activeTagFilter.value))

  if (activeLibraryView.value === 'favorites') {
    return filterTracksBySearch(taggedTracks.filter((track) => track.favorite))
  }

  if (activeLibraryView.value === 'recent') {
    return filterTracksBySearch(taggedTracks)
      .filter((track) => track.lastPlayedAt)
      .slice()
      .sort((left, right) => playedAtTime(right.lastPlayedAt) - playedAtTime(left.lastPlayedAt))
  }

  return filterTracksBySearch(taggedTracks)
})
const progressValue = computed(() => (Number.isFinite(currentTime.value) ? currentTime.value : 0))
const durationValue = computed(() => (Number.isFinite(duration.value) ? duration.value : 0))
const miniProgressPercent = computed(() =>
  durationValue.value > 0 ? clamp((progressValue.value / durationValue.value) * 100, 0, 100) : 0,
)
const emptyPlaylistTitle = computed(() => {
  if (!hasTracks.value) {
    return '暂无音乐'
  }

  if (normalizedSearchQuery.value) {
    return '没有匹配的歌曲'
  }

  if (activeTagFilter.value !== ALL_TAG) {
    return '当前标签暂无音乐'
  }

  if (activeLibraryView.value === 'queue') {
    return '播放队列为空'
  }

  if (activeLibraryView.value === 'favorites') {
    return '暂无收藏音乐'
  }

  if (activeLibraryView.value === 'recent') {
    return '暂无最近播放'
  }

  return '当前分类暂无音乐'
})
const playlistTitle = computed(() => {
  if (activePanelView.value === 'playlists') {
    return '歌单'
  }

  if (activePanelView.value === 'ai') {
    return 'AI 推荐'
  }

  if (activePanelView.value === 'netease') {
    return '网易云音乐'
  }

  return activeLibraryView.value === 'queue' ? '播放队列' : '播放列表'
})
const playlistCountLabel = computed(() => {
  if (activePanelView.value === 'playlists') {
    return `${customPlaylists.value.length} 个自定义 · 场景歌单规划中`
  }

  if (activePanelView.value === 'ai') {
    return '本地推荐基础'
  }

  if (activePanelView.value === 'netease') {
    if (!neteaseLoginStatus.value?.loggedIn) {
      return '等待登录'
    }

    return neteasePlaylists.value.length > 0
      ? `${neteasePlaylists.value.length} 个网易云歌单`
      : '账号已连接'
  }

  return activeLibraryView.value === 'queue'
    ? `${playQueue.value.length} 首待播`
    : `${tracks.value.length} 首 · 队列 ${playQueue.value.length}`
})
const clearButtonLabel = computed(() =>
  activeLibraryView.value === 'queue' ? '清空队列' : '清空',
)
const clearButtonDisabled = computed(() =>
  activeLibraryView.value === 'queue' ? !hasQueue.value : !hasTracks.value,
)
const favoriteTrackCount = computed(() => tracks.value.filter((track) => track.favorite).length)
const recentTrackCount = computed(() => tracks.value.filter((track) => track.lastPlayedAt).length)
const playlistTrackPickerTarget = computed(
  () =>
    customPlaylists.value.find((playlist) => playlist.id === playlistTrackPickerPlaylistId.value) ??
    null,
)
const playlistTrackPickerAvailableTracks = computed(() => {
  const playlist = playlistTrackPickerTarget.value
  if (!playlist) {
    return []
  }

  const query = playlistTrackPickerQuery.value.trim().toLowerCase()
  const playlistTrackIds = new Set(playlist.trackIds)
  return tracks.value
    .filter((track) => !playlistTrackIds.has(track.id))
    .filter((track) => !query || trackMatchesSearch(track, query))
    .slice()
    .sort((left, right) => left.title.localeCompare(right.title))
})
const playlistTrackPickerSelectedCount = computed(() => playlistTrackPickerSelectedIds.value.length)
const taggedTrackCount = computed(
  () => tracks.value.filter((track) => normalizeTrackTags(track.tags).length > 0).length,
)
const scenePlaylistOptions: ScenePlaylistOption[] = [
  {
    id: 'ai',
    title: 'AI 推荐',
    description: '根据喜欢、最近播放和标签挑一组歌',
    source: 'smart',
    tags: [],
  },
  {
    id: 'quiet',
    title: '安静陪伴',
    description: '适合低打扰陪伴和聊天背景',
    source: 'tags',
    tags: ['安静', '治愈', '慢歌', '聊天'],
  },
  {
    id: 'focus',
    title: '学习专注',
    description: '优先纯音乐、学习、工作和中速歌曲',
    source: 'tags',
    tags: ['学习', '工作', '纯音乐', '中速', '安静'],
  },
  {
    id: 'sleep',
    title: '睡前放松',
    description: '适合睡前放松、安静和慢歌',
    source: 'tags',
    tags: ['睡觉', '安静', '治愈', '慢歌', '纯音乐'],
  },
  {
    id: 'healing',
    title: '心情治愈',
    description: '难过、孤独或疲惫时的温和推荐',
    source: 'tags',
    tags: ['治愈', '难过', '孤独', '安静', '慢歌'],
  },
  {
    id: 'energy',
    title: '元气满满',
    description: '开心、热血、运动和快节奏歌曲',
    source: 'tags',
    tags: ['开心', '热血', '运动', '快歌'],
  },
]
const aiRecommendationOptions: AiRecommendationOption[] = [
  {
    id: 'tags',
    title: '按标签推荐',
    description: '优先使用情绪、场景和节奏标签',
    source: 'tags',
    tags: [],
  },
  {
    id: 'favorites',
    title: '按喜欢推荐',
    description: '从已收藏歌曲里挑选',
    source: 'favorites',
    tags: [],
  },
  {
    id: 'recent',
    title: '按最近播放',
    description: '延续最近听过的歌曲',
    source: 'recent',
    tags: [],
  },
]
const aiRecommendationSummary = computed(() => {
  if (taggedTrackCount.value > 0) {
    return `已准备 ${taggedTrackCount.value} 首带标签歌曲，可直接按标签推荐。`
  }

  if (favoriteTrackCount.value > 0) {
    return `已准备 ${favoriteTrackCount.value} 首喜欢的歌曲，可作为后续 AI 推荐基础。`
  }

  return '添加收藏或标签后，可作为后续 AI 推荐基础。'
})
const neteaseLoggedIn = computed(() => Boolean(neteaseLoginStatus.value?.loggedIn))
const neteaseProfile = computed(() => neteaseLoginStatus.value?.profile ?? null)
const neteaseSelectedPlaylist = computed(
  () =>
    neteasePlaylists.value.find((playlist) => playlist.id === neteaseSelectedPlaylistId.value) ??
    null,
)
const neteasePlaylistStatusLabel = computed(() => {
  if (neteasePlaylistError.value) {
    return neteasePlaylistError.value
  }

  if (!neteaseLoggedIn.value) {
    return '登录后可以读取你的网易云歌单。'
  }

  if (neteasePlaylistsLoading.value) {
    return '正在读取歌单...'
  }

  if (neteasePlaylistDetailLoading.value) {
    return '正在读取歌单歌曲...'
  }

  if (neteasePlaylistDetail.value) {
    return neteasePlaylistDetail.value.message
  }

  if (neteasePlaylists.value.length > 0) {
    return '选择一个歌单查看歌曲摘要。'
  }

  return '点击读取歌单获取网易云歌单列表。'
})
const neteaseStatusLabel = computed(() => {
  if (neteaseLoggedIn.value) {
    return '已登录'
  }

  if (neteaseQrStatus.value === 'scanned') {
    return '待确认'
  }

  if (neteaseQrStatus.value === 'waiting') {
    return '等待扫码'
  }

  if (neteaseQrStatus.value === 'expired') {
    return '已过期'
  }

  if (neteaseQrStatus.value === 'error') {
    return '连接失败'
  }

  return '未登录'
})
const neteaseStatusDetail = computed(() => {
  if (neteaseLoginError.value) {
    return neteaseLoginError.value
  }

  if (neteaseLoginNotice.value) {
    return neteaseLoginNotice.value
  }

  return neteaseLoginStatus.value?.message ?? '尚未登录网易云音乐。'
})
const neteaseQrExpired = computed(() => {
  if (!neteaseQrLogin.value) {
    return false
  }

  const expiresAt = Number(neteaseQrLogin.value.expiresAt)
  if (!Number.isFinite(expiresAt)) {
    return false
  }

  return Date.now() >= expiresAt * 1000
})
const repeatModeLabel = computed(() => {
  if (repeatMode.value === 'one') {
    return '单曲循环'
  }

  if (repeatMode.value === 'all') {
    return '列表循环'
  }

  return '播完停止'
})
const visualModeLabel = computed(
  () => visualModeOptions.find((option) => option.value === visualMode.value)?.label ?? '韵律',
)
const beatMapMatchesCurrentTrack = computed(
  () => Boolean(currentTrack.value && beatMap.value?.trackId === currentTrack.value.id),
)
const visualTimeValue = computed(() =>
  Number.isFinite(visualPlaybackTime.value) ? visualPlaybackTime.value : progressValue.value,
)
const visualLyricsTime = computed(() =>
  clamp(visualTimeValue.value + lyricOffsetMs.value / 1000, 0, durationValue.value || Number.POSITIVE_INFINITY),
)
const fallbackVisualEnergyFrame = computed(() =>
  createFallbackVisualEnergyFrame(
    visualTimeValue.value,
    currentTrack.value?.id ?? '',
    playing.value && Boolean(currentTrack.value),
  ),
)
const visualEnergyFrame = computed(() => {
  if (beatMapMatchesCurrentTrack.value) {
    const frame = beatMapFrameAt(visualTimeValue.value)
    if (frame) {
      return frame
    }
  }

  if (!analyzerReady.value && currentTrack.value) {
    return fallbackVisualEnergyFrame.value
  }

  return energyFrame.value
})
const visualFrequencyData = computed(() => {
  if (beatMapMatchesCurrentTrack.value) {
    const data = beatMapFrequencyDataAt(visualTimeValue.value)
    if (data) {
      return data
    }
  }

  if (!analyzerReady.value && currentTrack.value) {
    return synthesizeFallbackFrequencyData(visualEnergyFrame.value, visualTimeValue.value)
  }

  return frequencyData.value
})
const currentTrackOnline = computed(() => currentTrack.value?.source === 'netease')
const immersiveLocalPlaylistTracks = computed(() => {
  const seenTrackIds = new Set<string>()
  const playlistTracks: MusicTrack[] = []

  if (currentTrack.value) {
    playlistTracks.push(currentTrack.value)
    seenTrackIds.add(currentTrack.value.id)
  }

  const sourceTracks = playQueue.value.length > 0 ? queuedTracks.value : tracks.value
  for (const track of sourceTracks) {
    if (seenTrackIds.has(track.id)) {
      continue
    }

    playlistTracks.push(track)
    seenTrackIds.add(track.id)
  }

  return playlistTracks.slice(0, 18)
})
const immersiveNeteasePlaylistTracks = computed(() =>
  (neteasePlaylistDetail.value?.tracks ?? []).slice(0, 50),
)
const immersivePlaylistCountLabel = computed(() => {
  if (immersivePlaylistSource.value === 'netease') {
    if (!neteaseLoggedIn.value) {
      return '未登录网易云'
    }

    if (neteasePlaylistDetailLoading.value) {
      return '正在读取网易云歌单'
    }

    const playlistName = neteaseSelectedPlaylist.value?.name ?? '网易云歌单'
    const count = immersiveNeteasePlaylistTracks.value.length
    return count > 0 ? `${playlistName} · ${count} 首` : '等待读取网易云歌单'
  }

  if (currentTrackOnline.value) {
    return playQueue.value.length > 0
      ? `网易云临时播放 · 队列 ${playQueue.value.length}`
      : '网易云临时播放'
  }

  return playQueue.value.length > 0
    ? `${playQueue.value.length} 首队列`
    : `${tracks.value.length} 首本机歌曲`
})
const immersivePlaylistEmptyLabel = computed(() => {
  if (immersivePlaylistSource.value === 'netease') {
    if (!neteaseLoggedIn.value) {
      return '先登录网易云音乐'
    }

    if (neteasePlaylistDetailLoading.value) {
      return '正在读取网易云歌单'
    }

    return '先在网易云页读取一个歌单'
  }

  return '暂无歌曲'
})
const immersiveLyrics = computed(() => lyricsAt(visualLyricsTime.value, currentTrack.value))
const immersiveLyricsStatusLabel = computed(() => {
  if (lyricsStatus.value === 'loading') {
    return currentTrackOnline.value ? '在线歌词读取中' : '本机歌词读取中'
  }

  if (lyricsStatus.value === 'ready') {
    if (immersiveLyrics.value.interlude) {
      return '间奏'
    }

    if (immersiveLyrics.value.karaoke) {
      return '逐字同步'
    }

    return immersiveLyrics.value.synced ? '歌词同步' : '歌词浏览'
  }

  if (lyricsStatus.value === 'error') {
    return '歌词不可用'
  }

  if (lyricsStatus.value === 'empty') {
    return '未找到歌词'
  }

  return currentTrackOnline.value ? '在线歌词' : '本机歌词'
})
const lyricOffsetLabel = computed(() => {
  const value = Math.round(lyricOffsetMs.value)
  if (value === 0) {
    return '0 ms'
  }

  return value > 0 ? `提前 ${value} ms` : `延后 ${Math.abs(value)} ms`
})
const visualizerStatusLabel = computed(() => {
  if (!currentTrack.value) {
    return '待机'
  }

  if (beatMapStatus.value === 'analyzing') {
    return `节奏分析中 ${Math.round(beatMapProgress.value * 100)}%`
  }

  if (beatMapMatchesCurrentTrack.value) {
    return playing.value ? `${visualModeLabel.value}节奏同步` : '已暂停'
  }

  if (beatMapStatus.value === 'error' && !analyzerReady.value) {
    return '安全视觉回退'
  }

  if (analyzerError.value && !analyzerReady.value) {
    return '安全视觉回退'
  }

  if (!analyzerReady.value) {
    return '等待音频'
  }

  return playing.value ? `${visualModeLabel.value}响应中` : '已暂停'
})
const visualizerEnergyLabel = computed(() => {
  const energy = visualEnergyFrame.value
  const value = Math.round(clamp((energy.bass * 0.45 + energy.mid * 0.35 + energy.treble * 0.2) * 100, 0, 100))
  return `${value}%`
})
const visualizerHintLabel = computed(() => {
  if (!currentTrack.value) {
    return '选择音乐后开始可视化。'
  }

  if (beatMapStatus.value === 'analyzing') {
    return '正在本机分析节奏，不影响播放。'
  }

  if (beatMapMatchesCurrentTrack.value) {
    return currentTrackOnline.value ? '在线音频节奏图已同步。' : '本机节奏图已同步，播放与分析已分离。'
  }

  if (beatMapError.value) {
    return analyzerReady.value ? '离线节奏图不可用，已使用实时频谱分析。' : beatMapError.value
  }

  if (analyzerError.value) {
    return analyzerError.value
  }

  return analyzerReady.value ? '实时频谱分析可用。' : '播放后开始本机节奏分析。'
})
let unlistenThemeChanged: (() => void) | null = null
let unlistenMusicAction: (() => void) | null = null
let playlistsRestored = false
let beatMapRequestedTrackId = ''
let lyricsRequestedTrackId = ''
let visualClockFrameId: number | null = null
let lastVisualClockUpdate = 0
let neteaseQrPollTimer: number | null = null

onMounted(async () => {
  restoreSettings()
  restoreTracks()
  restorePlaylists()
  syncAudioVolume()
  await loadTheme()
  void refreshNeteaseLoginStatus(false)
  unlistenThemeChanged = await listen<string>('ui-theme-changed', (event) => {
    drawerTheme.value = event.payload === 'animal-island' ? 'animal-island' : 'light'
  })
  unlistenMusicAction = await listen<MusicActionRequest>('music-action-requested', (event) => {
    void handleMusicActionRequest(event.payload)
  })
})

onBeforeUnmount(() => {
  unlistenThemeChanged?.()
  unlistenMusicAction?.()
  stopVisualClock()
  stopNeteaseQrPolling()
})

watch(volume, () => {
  syncAudioVolume()
  saveSettings()
})

watch(
  [
    repeatMode,
    shuffleEnabled,
    musicStorageDir,
    importCategory,
    visualMode,
    visualIntensity,
    visualReducedMotion,
    lyricOffsetMs,
  ],
  saveSettings,
)

watch(currentTrack, () => {
  beatMapRequestedTrackId = ''
  lyricsRequestedTrackId = ''
  visualPlaybackTime.value = 0
  resetEnergyFrame()
  resetBeatMap()
  resetLyrics()
  if (immersiveMode.value && currentTrack.value) {
    void analyzeCurrentTrackBeatMap()
    void loadCurrentTrackLyrics()
  }
})

watch(activePanelView, (view) => {
  if (view === 'netease') {
    void (async () => {
      await refreshNeteaseLoginStatus(false)
      if (
        neteaseLoggedIn.value &&
        neteasePlaylists.value.length === 0 &&
        !neteasePlaylistsLoading.value
      ) {
        await refreshNeteasePlaylists(false)
      }
    })()
    return
  }

  stopNeteaseQrPolling()
})

watch(playing, (isPlaying) => {
  if (isPlaying && immersiveMode.value) {
    void prepareImmersiveVisualization()
    startVisualClock()
  } else {
    stopVisualClock()
    syncVisualPlaybackTime()
  }
})

watch(
  playQueue,
  () => {
    saveSettings()
  },
  { deep: true },
)

watch(categoryOptions, () => {
  if (
    activeCategoryFilter.value !== ALL_CATEGORY &&
    !categoryOptions.value.includes(activeCategoryFilter.value)
  ) {
    activeCategoryFilter.value = ALL_CATEGORY
  }
})

watch(tagOptions, () => {
  if (activeTagFilter.value !== ALL_TAG && !tagOptions.value.includes(activeTagFilter.value)) {
    activeTagFilter.value = ALL_TAG
  }
})

watch(
  tracks,
  () => {
    saveTracks()
    if (playlistsRestored) {
      syncCustomPlaylists()
    }
  },
  { deep: true },
)

watch(
  customPlaylists,
  () => {
    if (playlistsRestored) {
      savePlaylists()
    }
  },
  { deep: true },
)

function restoreSettings() {
  try {
    const raw = localStorage.getItem(SETTINGS_STORAGE_KEY)
    if (!raw) {
      return
    }

    const saved = JSON.parse(raw) as {
      volume?: number
      repeatMode?: RepeatMode
      shuffleEnabled?: boolean
      musicStorageDir?: string
      importCategory?: string
      playQueue?: string[]
      visualMode?: MusicVisualMode
      visualIntensity?: number
      visualReducedMotion?: boolean
      lyricOffsetMs?: number
    }

    if (typeof saved.volume === 'number') {
      volume.value = clamp(saved.volume, 0, 1)
    }
    if (saved.repeatMode === 'none' || saved.repeatMode === 'one' || saved.repeatMode === 'all') {
      repeatMode.value = saved.repeatMode
    }
    shuffleEnabled.value = Boolean(saved.shuffleEnabled)
    musicStorageDir.value = saved.musicStorageDir?.trim() ?? ''
    importCategory.value = normalizeMusicCategory(saved.importCategory)
    playQueue.value = normalizeQueueIds(saved.playQueue)
    if (isMusicVisualMode(saved.visualMode)) {
      visualMode.value = saved.visualMode
    }
    if (typeof saved.visualIntensity === 'number') {
      visualIntensity.value = clamp(saved.visualIntensity, 0.2, 1)
    }
    visualReducedMotion.value = Boolean(saved.visualReducedMotion)
    if (typeof saved.lyricOffsetMs === 'number') {
      lyricOffsetMs.value = clamp(Math.round(saved.lyricOffsetMs), -2000, 2000)
    }
  } catch {
    localStorage.removeItem(SETTINGS_STORAGE_KEY)
  }
}

function restoreTracks() {
  try {
    const raw = localStorage.getItem(TRACKS_STORAGE_KEY)
    if (!raw) {
      return
    }

    const saved = JSON.parse(raw) as Array<{
      id?: string
      path?: string
      sourcePath?: string
      title?: string
      artist?: string
      album?: string
      category?: string
      tags?: string[]
      duration?: number | null
      favorite?: boolean
      playCount?: number
      lastPlayedAt?: string | null
      playHistory?: string[]
    }>
    tracks.value = saved
      .filter(
        (item): item is {
          id?: string
          path: string
          sourcePath?: string
          title?: string
          artist?: string
          album?: string
          category?: string
          tags?: string[]
          duration?: number | null
          favorite?: boolean
          playCount?: number
          lastPlayedAt?: string | null
          playHistory?: string[]
        } => Boolean(item.path),
      )
      .map((item) => {
        const restoredIdentity = normalizeStoredTrackIdentity(
          item.title,
          item.artist,
          item.sourcePath || item.path,
        )

        return createTrackWithIdentity(
          item.path,
          restoredIdentity,
          item.sourcePath || item.path,
          normalizeMusicCategory(item.category),
          {
            id: item.id,
            artist: restoredIdentity.artist,
            album: normalizeTrackAlbum(item.album),
            tags: normalizeTrackTags(item.tags),
            duration: sanitizeTrackDuration(item.duration),
            favorite: Boolean(item.favorite),
            playCount: sanitizePlayCount(item.playCount),
            lastPlayedAt: normalizeTrackDate(item.lastPlayedAt),
            playHistory: normalizeTrackHistory(item.playHistory),
          },
        )
      })

    if (tracks.value.length > 0) {
      currentIndex.value = 0
    }
    syncPlaybackQueue()
  } catch {
    localStorage.removeItem(TRACKS_STORAGE_KEY)
  }
}

function restorePlaylists() {
  try {
    const raw = localStorage.getItem(PLAYLISTS_STORAGE_KEY)
    if (!raw) {
      playlistsRestored = true
      return
    }

    const saved = JSON.parse(raw) as Array<{
      id?: string
      name?: string
      trackIds?: string[]
      createdAt?: string
      updatedAt?: string
    }>
    customPlaylists.value = saved
      .map(normalizeCustomPlaylist)
      .filter((playlist): playlist is MusicPlaylist => Boolean(playlist))

    if (!customPlaylists.value.some((playlist) => playlist.id === activeCustomPlaylistId.value)) {
      activeCustomPlaylistId.value = ''
    }
    playlistsRestored = true
  } catch {
    localStorage.removeItem(PLAYLISTS_STORAGE_KEY)
    playlistsRestored = true
  }
}

function saveSettings() {
  localStorage.setItem(
    SETTINGS_STORAGE_KEY,
    JSON.stringify({
      volume: volume.value,
      repeatMode: repeatMode.value,
      shuffleEnabled: shuffleEnabled.value,
      musicStorageDir: musicStorageDir.value,
      importCategory: normalizeMusicCategory(importCategory.value),
      playQueue: sanitizeQueueIds(playQueue.value),
      visualMode: visualMode.value,
      visualIntensity: clamp(visualIntensity.value, 0.2, 1),
      visualReducedMotion: visualReducedMotion.value,
      lyricOffsetMs: clamp(Math.round(lyricOffsetMs.value), -2000, 2000),
    }),
  )
}

function saveTracks() {
  localStorage.setItem(
    TRACKS_STORAGE_KEY,
    JSON.stringify(
      tracks.value
        .filter((track) => track.source !== 'netease')
        .map((track) => ({
          id: track.id,
          path: track.path,
          sourcePath: track.sourcePath,
          title: track.title,
          artist: normalizeTrackArtist(track.artist),
          album: normalizeTrackAlbum(track.album),
          category: normalizeMusicCategory(track.category),
          tags: normalizeTrackTags(track.tags),
          duration: sanitizeTrackDuration(track.duration),
          favorite: track.favorite,
          playCount: sanitizePlayCount(track.playCount),
          lastPlayedAt: normalizeTrackDate(track.lastPlayedAt),
          playHistory: normalizeTrackHistory(track.playHistory),
        })),
    ),
  )
}

function savePlaylists() {
  localStorage.setItem(
    PLAYLISTS_STORAGE_KEY,
    JSON.stringify(
      customPlaylists.value.map((playlist) => ({
        id: playlist.id,
        name: normalizePlaylistName(playlist.name),
        trackIds: normalizePlaylistTrackIds(playlist.trackIds),
        createdAt: normalizeTrackDate(playlist.createdAt) || new Date().toISOString(),
        updatedAt: normalizeTrackDate(playlist.updatedAt) || new Date().toISOString(),
      })),
    ),
  )
}

async function loadTheme() {
  try {
    const config = await invoke<PetDrawerConfig>('get_config')
    drawerTheme.value = config.drawer.theme === 'animal-island' ? 'animal-island' : 'light'
  } catch {
    drawerTheme.value = 'light'
  }
}

function createTrack(
  path: string,
  title?: string,
  sourcePath = path,
  category = DEFAULT_CATEGORY,
  playback?: MusicTrackPlaybackDraft,
): MusicTrack {
  const inferredIdentity = inferTrackIdentity(title || trackTitleFromPath(sourcePath || path))
  return createTrackWithIdentity(path, inferredIdentity, sourcePath, category, playback)
}

type MusicTrackPlaybackDraft = Partial<
  Pick<
    MusicTrack,
    | 'id'
    | 'artist'
    | 'album'
    | 'tags'
    | 'duration'
    | 'favorite'
    | 'playCount'
    | 'lastPlayedAt'
    | 'playHistory'
  >
>

function createTrackWithIdentity(
  path: string,
  identity: TrackIdentity,
  sourcePath = path,
  category = DEFAULT_CATEGORY,
  playback?: MusicTrackPlaybackDraft,
): MusicTrack {
  const fallbackIdentity = inferTrackIdentity(trackTitleFromPath(sourcePath || path))
  const title = normalizeTrackTitle(identity.title) || fallbackIdentity.title
  const artist =
    normalizeTrackArtist(playback?.artist) ||
    normalizeTrackArtist(identity.artist) ||
    fallbackIdentity.artist

  return {
    id: normalizeTrackId(playback?.id) || createTrackId(),
    title,
    artist,
    album: normalizeTrackAlbum(playback?.album),
    path,
    sourcePath,
    source: 'local',
    category: normalizeMusicCategory(category),
    tags: normalizeTrackTags(playback?.tags),
    url: safeConvertFileSrc(path),
    duration: sanitizeTrackDuration(playback?.duration),
    favorite: Boolean(playback?.favorite),
    playCount: sanitizePlayCount(playback?.playCount),
    lastPlayedAt: normalizeTrackDate(playback?.lastPlayedAt),
    playHistory: normalizeTrackHistory(playback?.playHistory),
  }
}

function createTrackId() {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return crypto.randomUUID()
  }

  return `${Date.now()}-${Math.random().toString(36).slice(2)}`
}

function normalizeTrackId(value?: string | null) {
  return value?.trim() ?? ''
}

function safeConvertFileSrc(path: string) {
  try {
    return convertFileSrc(path)
  } catch {
    return path
  }
}

function trackTitleFromPath(path: string) {
  const fileName = path.split(/[\\/]/).pop() || '未命名音乐'
  return fileName.replace(/\.[^.]+$/, '') || fileName
}

function inferTrackIdentity(rawTitle: string): TrackIdentity {
  return inferTrackIdentityByOrder(rawTitle, 'title-artist')
}

function inferArtistFirstTrackIdentity(rawTitle: string): TrackIdentity {
  return inferTrackIdentityByOrder(rawTitle, 'artist-title')
}

function inferTrackIdentityByOrder(
  rawTitle: string,
  order: 'title-artist' | 'artist-title',
): TrackIdentity {
  const fallbackTitle = stripTrackNumberPrefix(rawTitle.trim()) || '未命名音乐'
  const parts = splitTrackIdentityParts(fallbackTitle)

  if (parts.length >= 2) {
    const artist =
      order === 'artist-title'
        ? parts[0].trim()
        : parts[parts.length - 1].trim()
    const title =
      order === 'artist-title'
        ? parts.slice(1).join(' - ').trim()
        : parts.slice(0, -1).join(' - ').trim()

    if (!isLikelyArtistCandidate(artist, title)) {
      return {
        artist: '',
        title: fallbackTitle,
      }
    }

    return {
      artist,
      title: title || fallbackTitle,
    }
  }

  return {
    artist: '',
    title: fallbackTitle,
  }
}

function splitTrackIdentityParts(value: string) {
  const spacedParts = value
    .split(/\s+[-–—]\s+/)
    .map((part) => part.trim())
    .filter(Boolean)

  if (spacedParts.length >= 2) {
    return spacedParts
  }

  return value
    .split(/\s*[-–—]\s*/)
    .map((part) => part.trim())
    .filter(Boolean)
}

function normalizeStoredTrackIdentity(
  title: string | undefined,
  artist: string | undefined,
  sourcePath: string,
): TrackIdentity {
  const storedTitle = normalizeTrackTitle(title)
  const storedArtist = normalizeTrackArtist(artist)
  const filenameTitle = trackTitleFromPath(sourcePath)
  const titleFirstIdentity = inferTrackIdentity(filenameTitle)
  const oldArtistFirstIdentity = inferArtistFirstTrackIdentity(filenameTitle)

  if (
    shouldMigrateReversedStoredIdentity(
      storedTitle,
      storedArtist,
      titleFirstIdentity,
      oldArtistFirstIdentity,
    )
  ) {
    return titleFirstIdentity
  }

  return {
    title: storedTitle || titleFirstIdentity.title,
    artist: storedArtist,
  }
}

function shouldMigrateReversedStoredIdentity(
  storedTitle: string,
  storedArtist: string,
  titleFirstIdentity: TrackIdentity,
  oldArtistFirstIdentity: TrackIdentity,
) {
  return Boolean(
    storedTitle &&
      storedArtist &&
      titleFirstIdentity.artist &&
      oldArtistFirstIdentity.artist &&
      storedTitle === oldArtistFirstIdentity.title &&
      storedArtist === oldArtistFirstIdentity.artist &&
      storedTitle !== titleFirstIdentity.title &&
      storedArtist !== titleFirstIdentity.artist,
  )
}

function stripTrackNumberPrefix(value: string) {
  return value.replace(/^\s*\d{1,3}[\s._-]+/, '').trim()
}

function isLikelyArtistCandidate(artist: string, title: string) {
  if (!artist || !title || artist.length > 64) {
    return false
  }

  if (/^\d+$/.test(artist)) {
    return false
  }

  if (/^(cd|disc|disk|track|vol|volume|no)\.?\s*\d*$/i.test(artist)) {
    return false
  }

  return true
}

async function chooseMusicFiles() {
  playerError.value = ''
  playerStatus.value = ''

  try {
    const selected = await open({
      multiple: true,
      filters: [
        {
          name: '音频文件',
          extensions: ['mp3', 'wav', 'ogg', 'flac', 'm4a', 'aac', 'webm'],
        },
      ],
    })
    const paths = normalizeSelectedPaths(selected)

    if (paths.length === 0) {
      return
    }

    await importMusicPaths(paths, normalizeMusicCategory(importCategory.value))
  } catch (err) {
    playerStatus.value = ''
    playerError.value = `无法选择音乐文件：${String(err)}`
  }
}

function showMusicSettings() {
  settingsVisible.value = true
  playerError.value = ''
  playerStatus.value = ''
}

async function chooseMusicFolder() {
  playerError.value = ''
  playerStatus.value = ''

  try {
    const selected = await open({
      directory: true,
      recursive: true,
      title: '选择音乐文件夹',
    })

    if (!selected || Array.isArray(selected)) {
      return
    }

    const paths = await invoke<string[]>('list_music_files_in_directory', { directory: selected })
    const category =
      normalizeMusicCategory(importCategory.value) === DEFAULT_CATEGORY
        ? folderNameFromPath(selected)
        : normalizeMusicCategory(importCategory.value)
    await importMusicPaths(paths, category)
  } catch (err) {
    playerStatus.value = ''
    playerError.value = `无法添加文件夹音乐：${String(err)}`
  }
}

async function chooseMusicStorageDirectory() {
  playerError.value = ''
  playerStatus.value = ''

  try {
    const selected = await open({
      directory: true,
      title: '选择歌曲文件存储目录',
    })

    if (!selected || Array.isArray(selected)) {
      return
    }

    musicStorageDir.value = selected
    playerStatus.value = '后续添加的歌曲会复制到该目录。'
    saveSettings()
  } catch (err) {
    playerStatus.value = ''
    playerError.value = `无法选择歌曲文件存储目录：${String(err)}`
  }
}

function clearMusicStorageDirectory() {
  musicStorageDir.value = ''
  playerStatus.value = '已改为使用歌曲原始位置。'
  saveSettings()
}

async function refreshNeteaseLoginStatus(showStatus = true) {
  neteaseLoginError.value = ''
  if (showStatus) {
    neteaseLoginNotice.value = '正在检查网易云登录状态...'
  }

  try {
    const status = await invoke<NeteaseLoginStatus>('get_netease_login_status')
    neteaseLoginStatus.value = status
    if (!status.loggedIn) {
      resetNeteasePlaylistState()
    }
    if (showStatus) {
      neteaseLoginNotice.value = status.message
    }
  } catch (err) {
    neteaseLoginError.value = `网易云状态读取失败：${String(err)}`
  }
}

async function startNeteaseQrLogin() {
  stopNeteaseQrPolling()
  neteaseLoginBusy.value = true
  neteaseLoginError.value = ''
  neteaseLoginNotice.value = '正在生成网易云登录二维码...'
  neteaseQrStatus.value = 'idle'

  try {
    neteaseQrLogin.value = await invoke<NeteaseQrLogin>('create_netease_qr_login')
    neteaseQrStatus.value = 'waiting'
    neteaseLoginNotice.value = '等待网易云 App 扫码。'
    startNeteaseQrPolling()
  } catch (err) {
    neteaseQrLogin.value = null
    neteaseQrStatus.value = 'error'
    neteaseLoginError.value = `网易云二维码生成失败：${String(err)}`
  } finally {
    neteaseLoginBusy.value = false
  }
}

function cancelNeteaseQrLogin() {
  stopNeteaseQrPolling()
  neteaseQrLogin.value = null
  neteaseQrStatus.value = 'idle'
  neteaseLoginNotice.value = neteaseLoginStatus.value?.message ?? '已取消网易云扫码登录。'
}

function startNeteaseQrPolling() {
  stopNeteaseQrPolling()
  void checkNeteaseQrLogin()
  neteaseQrPollTimer = window.setInterval(() => {
    void checkNeteaseQrLogin()
  }, 2000)
}

function stopNeteaseQrPolling() {
  if (neteaseQrPollTimer !== null) {
    window.clearInterval(neteaseQrPollTimer)
    neteaseQrPollTimer = null
  }
}

async function checkNeteaseQrLogin() {
  if (!neteaseQrLogin.value || neteaseQrChecking.value) {
    return
  }

  if (neteaseQrExpired.value) {
    stopNeteaseQrPolling()
    neteaseQrStatus.value = 'expired'
    neteaseLoginNotice.value = '二维码已过期，请重新生成。'
    return
  }

  neteaseQrChecking.value = true
  neteaseLoginError.value = ''

  try {
    const result = await invoke<NeteaseQrCheckResult>('check_netease_qr_login', {
      key: neteaseQrLogin.value.key,
    })
    neteaseQrStatus.value = normalizeNeteaseQrStatus(result)
    neteaseLoginNotice.value = result.message

    if (result.loggedIn) {
      stopNeteaseQrPolling()
      neteaseQrLogin.value = null
      neteaseQrStatus.value = 'authorized'
      neteaseLoginStatus.value = {
        loggedIn: true,
        profile: result.profile,
        savedAt: null,
        checkedAt: String(Math.floor(Date.now() / 1000)),
        message: result.message,
      }
      await refreshNeteaseLoginStatus(false)
      await refreshNeteasePlaylists(false)
      playerStatus.value = '网易云音乐登录成功。'
    } else if (neteaseQrStatus.value === 'expired' || result.code === 800) {
      stopNeteaseQrPolling()
    }
  } catch (err) {
    stopNeteaseQrPolling()
    neteaseQrStatus.value = 'error'
    neteaseLoginError.value = `网易云扫码状态检查失败：${String(err)}`
  } finally {
    neteaseQrChecking.value = false
  }
}

async function clearNeteaseLogin() {
  if (neteaseLoggedIn.value && !window.confirm('清除本机保存的网易云登录状态？')) {
    return
  }

  stopNeteaseQrPolling()
  neteaseLoginBusy.value = true
  neteaseLoginError.value = ''
  neteaseLoginNotice.value = '正在清除网易云登录状态...'

  try {
    const status = await invoke<NeteaseLoginStatus>('clear_netease_login')
    neteaseLoginStatus.value = status
    neteaseQrLogin.value = null
    neteaseQrStatus.value = 'idle'
    neteaseLoginNotice.value = status.message
    resetNeteasePlaylistState()
    resetNeteasePlaybackState()
  } catch (err) {
    neteaseLoginError.value = `清除网易云登录状态失败：${String(err)}`
  } finally {
    neteaseLoginBusy.value = false
  }
}

async function refreshNeteasePlaylists(showStatus = true) {
  if (!neteaseLoggedIn.value) {
    neteasePlaylistError.value = '请先登录网易云音乐。'
    return
  }

  neteasePlaylistsLoading.value = true
  neteasePlaylistError.value = ''
  if (showStatus) {
    neteaseLoginNotice.value = '正在读取网易云歌单...'
  }

  try {
    const playlists = await invoke<NeteasePlaylistSummary[]>('list_netease_playlists')
    neteasePlaylists.value = playlists
    neteasePlaylistDetail.value = null
    neteaseSelectedPlaylistId.value = playlists[0]?.id ?? null
    if (playlists[0]) {
      await loadNeteasePlaylistDetail(playlists[0], false)
    }
    if (showStatus) {
      neteaseLoginNotice.value =
        playlists.length > 0 ? `已读取 ${playlists.length} 个网易云歌单。` : '没有读取到网易云歌单。'
    }
  } catch (err) {
    neteasePlaylistError.value = `网易云歌单读取失败：${String(err)}`
  } finally {
    neteasePlaylistsLoading.value = false
  }
}

async function loadNeteasePlaylistDetail(playlist: NeteasePlaylistSummary, showStatus = true) {
  neteaseSelectedPlaylistId.value = playlist.id
  neteasePlaylistDetailLoading.value = true
  neteasePlaylistError.value = ''
  if (showStatus) {
    neteaseLoginNotice.value = `正在读取《${playlist.name}》...`
  }

  try {
    neteasePlaylistDetail.value = await invoke<NeteasePlaylistDetail>(
      'get_netease_playlist_detail',
      { playlistId: playlist.id },
    )
    if (showStatus) {
      neteaseLoginNotice.value = neteasePlaylistDetail.value.message
    }
  } catch (err) {
    neteasePlaylistDetail.value = null
    neteasePlaylistError.value = `网易云歌单歌曲读取失败：${String(err)}`
  } finally {
    neteasePlaylistDetailLoading.value = false
  }
}

async function playNeteaseTrack(track: NeteasePlaylistTrack) {
  if (!neteaseLoggedIn.value) {
    neteasePlaylistError.value = '请先登录网易云音乐。'
    return
  }

  neteaseTrackActionId.value = track.id
  neteasePlaylistError.value = ''
  playerError.value = ''
  playerStatus.value = `正在获取《${track.name}》的在线播放地址...`

  try {
    const playback = await invoke<NeteasePlaybackUrl>('get_netease_song_playback_url', {
      songId: track.id,
      level: 'standard',
    })
    const onlineTrack = createNeteaseMusicTrack(track, playback)
    pausePlayback()
    currentIndex.value = -1
    neteaseCurrentTrack.value = onlineTrack
    if (immersiveMode.value) {
      immersivePlaylistSource.value = 'netease'
    }
    currentTime.value = 0
    visualPlaybackTime.value = 0
    duration.value = onlineTrack.duration ?? 0
    await nextTick()
    await playCurrent(true)
    if (!playerError.value) {
      playerStatus.value = `正在在线播放网易云《${track.name}》。`
      void showNeteaseLyrics(track, false)
    }
  } catch (err) {
    playerStatus.value = ''
    playerError.value = `网易云在线播放失败：${String(err)}`
  } finally {
    neteaseTrackActionId.value = null
  }
}

async function showNeteaseLyrics(track: NeteasePlaylistTrack, showStatus = true) {
  if (!neteaseLoggedIn.value) {
    neteaseLyricsError.value = '请先登录网易云音乐。'
    return
  }

  neteaseLyricsTrack.value = track
  neteaseLyricsLoading.value = true
  neteaseLyricsError.value = ''
  if (showStatus) {
    neteaseLoginNotice.value = `正在读取《${track.name}》在线歌词...`
  }

  try {
    neteaseLyricsResult.value = await invoke<NeteaseLyricsResult>('read_netease_lyrics', {
      songId: track.id,
    })
    if (showStatus) {
      neteaseLoginNotice.value = `已读取《${track.name}》在线歌词。`
    }
  } catch (err) {
    neteaseLyricsResult.value = null
    neteaseLyricsError.value = `网易云歌词读取失败：${String(err)}`
  } finally {
    neteaseLyricsLoading.value = false
  }
}

function closeNeteaseLyrics() {
  neteaseLyricsTrack.value = null
  neteaseLyricsResult.value = null
  neteaseLyricsLoading.value = false
  neteaseLyricsError.value = ''
}

function resetNeteasePlaylistState() {
  neteasePlaylists.value = []
  neteaseSelectedPlaylistId.value = null
  neteasePlaylistDetail.value = null
  neteasePlaylistsLoading.value = false
  neteasePlaylistDetailLoading.value = false
  neteasePlaylistError.value = ''
  closeNeteaseLyrics()
}

function resetNeteasePlaybackState() {
  if (neteaseCurrentTrack.value) {
    pausePlayback()
    neteaseCurrentTrack.value = null
    currentTime.value = 0
    visualPlaybackTime.value = 0
    duration.value = 0
  }
}

function createNeteaseMusicTrack(
  track: NeteasePlaylistTrack,
  playback: NeteasePlaybackUrl,
): MusicTrack {
  const durationSeconds =
    sanitizeTrackDuration((playback.durationMs ?? track.durationMs ?? 0) / 1000) ?? null

  return {
    id: `${NETEASE_TRACK_ID_PREFIX}:${track.id}`,
    title: normalizeTrackTitle(track.name) || '网易云歌曲',
    artist: formatNeteaseTrackArtists(track),
    album: normalizeTrackAlbum(track.album),
    path: `${NETEASE_TRACK_ID_PREFIX}:${track.id}`,
    sourcePath: `${NETEASE_TRACK_ID_PREFIX}:${track.id}`,
    source: 'netease',
    neteaseSongId: track.id,
    category: '网易云',
    tags: [],
    url: playback.url,
    duration: durationSeconds,
    favorite: false,
    playCount: 0,
    lastPlayedAt: null,
    playHistory: [],
  }
}

async function importMusicPaths(paths: string[], category: string) {
  playerError.value = ''
  playerStatus.value = ''
  const normalizedCategory = normalizeMusicCategory(category)
  const existingPaths = new Set(tracks.value.map((track) => track.path.toLowerCase()))
  const existingSources = new Set(
    tracks.value.map((track) => (track.sourcePath || track.path).toLowerCase()),
  )
  const candidatePaths = paths.filter((path) => {
    const normalizedPath = path.toLowerCase()
    return !existingPaths.has(normalizedPath) && !existingSources.has(normalizedPath)
  })

  if (candidatePaths.length === 0) {
    playerError.value = '所选音乐已经在播放列表中。'
    return
  }

  const imported = await invoke<MusicImportItem[]>('import_music_files', {
    paths: candidatePaths,
    storageDir: musicStorageDir.value,
  })
  const importedPathSet = new Set(tracks.value.map((track) => track.path.toLowerCase()))
  const newTracks: MusicTrack[] = []
  let metadataAppliedCount = 0
  for (const item of imported) {
    if (importedPathSet.has(item.path.toLowerCase())) {
      continue
    }

    const importedTrack = await createImportedTrack(item, normalizedCategory)
    newTracks.push(importedTrack.track)
    importedPathSet.add(item.path.toLowerCase())
    if (importedTrack.metadataApplied) {
      metadataAppliedCount += 1
    }
  }

  if (newTracks.length === 0) {
    playerError.value = '所选音乐已经在播放列表中。'
    return
  }

  const firstNewIndex = tracks.value.length
  tracks.value.push(...newTracks)
  importCategory.value = normalizedCategory
  activeLibraryView.value = 'all'
  activeCategoryFilter.value = normalizedCategory
  playerStatus.value =
    metadataAppliedCount > 0
      ? `已添加 ${newTracks.length} 首音乐，其中 ${metadataAppliedCount} 首优先使用 metadata。`
      : `已添加 ${newTracks.length} 首音乐。`
  settingsVisible.value = false

  if (currentIndex.value < 0) {
    await playTrack(firstNewIndex)
  }
}

async function createImportedTrack(item: MusicImportItem, category: string) {
  const fallbackIdentity = inferTrackIdentity(trackTitleFromPath(item.sourcePath || item.path))

  try {
    const metadata = await invoke<MusicMetadataResult>('read_music_metadata', { path: item.path })
    const metadataTitle = normalizeTrackTitle(metadata.title)
    const metadataArtist = normalizeTrackArtist(metadata.artist)
    const metadataAlbum = normalizeTrackAlbum(metadata.album)
    const metadataDuration = sanitizeTrackDuration(metadata.duration)
    const metadataApplied = Boolean(
      metadataTitle || metadataArtist || metadataAlbum || metadataDuration,
    )
    const track = createTrackWithIdentity(
      item.path,
      {
        title: metadataTitle || fallbackIdentity.title,
        artist: metadataArtist || fallbackIdentity.artist,
      },
      item.sourcePath,
      category,
      {
        album: metadataAlbum,
        duration: metadataDuration,
      },
    )

    return { track, metadataApplied }
  } catch {
    return {
      track: createTrack(item.path, trackTitleFromPath(item.sourcePath), item.sourcePath, category),
      metadataApplied: false,
    }
  }
}

function normalizeSelectedPaths(selected: string | string[] | null) {
  if (!selected) {
    return []
  }

  return Array.isArray(selected) ? selected : [selected]
}

function normalizeMusicCategory(value?: string | null) {
  const category = value?.trim() ?? ''
  return category || DEFAULT_CATEGORY
}

function isMusicVisualMode(value?: string | null): value is MusicVisualMode {
  return visualModeOptions.some((option) => option.value === value)
}

function normalizeTrackTitle(value?: string | null) {
  return value?.trim() ?? ''
}

function normalizeTrackArtist(value?: string | null) {
  return value?.trim() ?? ''
}

function normalizeTrackAlbum(value?: string | null) {
  return value?.trim() ?? ''
}

function normalizeTrackTags(value?: string[] | string | null) {
  const rawTags = Array.isArray(value)
    ? value
    : (value ?? '')
        .split(/[,，、/|]+/)
        .map((tag) => tag.trim())
  const seenTags = new Set<string>()

  return rawTags
    .map((tag) => tag.trim())
    .filter((tag) => {
      if (!tag || tag.length > MAX_TRACK_TAG_LENGTH || seenTags.has(tag)) {
        return false
      }

      seenTags.add(tag)
      return true
    })
    .slice(0, MAX_TRACK_TAGS)
}

function formatTrackTagsInput(track: MusicTrack) {
  return normalizeTrackTags(track.tags).join('，')
}

function formatTrackTagsLabel(track: MusicTrack) {
  const tags = normalizeTrackTags(track.tags)
  return tags.length > 0 ? `标签 ${tags.join(' / ')}` : ''
}

function folderNameFromPath(path: string) {
  return normalizeMusicCategory(path.split(/[\\/]/).filter(Boolean).pop())
}

function sanitizePlayCount(value?: number) {
  if (!Number.isFinite(value)) {
    return 0
  }

  return Math.max(0, Math.floor(value ?? 0))
}

function sanitizeTrackDuration(value?: number | null) {
  if (!Number.isFinite(value) || !value || value <= 0) {
    return null
  }

  return Math.round(value)
}

function normalizeTrackDate(value?: string | null) {
  if (!value || Number.isNaN(Date.parse(value))) {
    return null
  }

  return value
}

function normalizeTrackHistory(history?: string[]) {
  if (!Array.isArray(history)) {
    return []
  }

  return history.filter((item) => normalizeTrackDate(item)).slice(0, MAX_PLAY_HISTORY_PER_TRACK)
}

function normalizePlaylistName(value?: string | null) {
  const name = value?.trim() ?? ''
  return name.slice(0, 32)
}

function normalizeQueueIds(value?: string[]) {
  if (!Array.isArray(value)) {
    return []
  }

  return value.map(normalizeTrackId).filter(Boolean)
}

function normalizePlaylistTrackIds(value?: string[]) {
  const validTrackIds = new Set(tracks.value.map((track) => track.id))
  const seenTrackIds = new Set<string>()

  return normalizeQueueIds(value).filter((trackId) => {
    if (!validTrackIds.has(trackId) || seenTrackIds.has(trackId)) {
      return false
    }

    seenTrackIds.add(trackId)
    return true
  })
}

function normalizeCustomPlaylist(
  playlist: {
    id?: string
    name?: string
    trackIds?: string[]
    createdAt?: string
    updatedAt?: string
  },
): MusicPlaylist | null {
  const id = normalizeTrackId(playlist.id)
  const name = normalizePlaylistName(playlist.name)
  if (!id || !name) {
    return null
  }

  const now = new Date().toISOString()
  return {
    id,
    name,
    trackIds: normalizePlaylistTrackIds(playlist.trackIds),
    createdAt: normalizeTrackDate(playlist.createdAt) || now,
    updatedAt: normalizeTrackDate(playlist.updatedAt) || now,
  }
}

function sanitizeQueueIds(value: string[]) {
  const validTrackIds = new Set(tracks.value.map((track) => track.id))
  const seenTrackIds = new Set<string>()
  return normalizeQueueIds(value).filter((trackId) => {
    if (!validTrackIds.has(trackId) || seenTrackIds.has(trackId)) {
      return false
    }

    seenTrackIds.add(trackId)
    return true
  })
}

function syncPlaybackQueue() {
  const sanitizedQueue = sanitizeQueueIds(playQueue.value)
  if (sanitizedQueue.join('\n') !== playQueue.value.join('\n')) {
    playQueue.value = sanitizedQueue
  }
}

function syncCustomPlaylists() {
  let changed = false
  customPlaylists.value = customPlaylists.value.map((playlist) => {
    const trackIds = normalizePlaylistTrackIds(playlist.trackIds)
    if (trackIds.join('\n') !== playlist.trackIds.join('\n')) {
      changed = true
      return {
        ...playlist,
        trackIds,
        updatedAt: new Date().toISOString(),
      }
    }

    return playlist
  })

  if (
    activeCustomPlaylistId.value &&
    !customPlaylists.value.some((playlist) => playlist.id === activeCustomPlaylistId.value)
  ) {
    activeCustomPlaylistId.value = ''
  }
}

function trackById(trackId: string) {
  return tracks.value.find((track) => track.id === trackId) ?? null
}

function playedAtTime(value?: string | null) {
  const time = value ? Date.parse(value) : Number.NaN
  return Number.isNaN(time) ? 0 : time
}

function filterTracksBySearch(trackList: MusicTrack[]) {
  if (!normalizedSearchQuery.value) {
    return trackList
  }

  return trackList.filter((track) => trackMatchesSearch(track, normalizedSearchQuery.value))
}

function trackMatchesSearch(track: MusicTrack, query: string) {
  const searchableText = [
    track.title,
    track.artist,
    track.album,
    normalizeMusicCategory(track.category),
    normalizeTrackTags(track.tags).join('\n'),
    track.sourcePath,
    track.path,
  ]
    .join('\n')
    .toLowerCase()

  return query
    .split(/\s+/)
    .filter(Boolean)
    .every((keyword) => searchableText.includes(keyword))
}

function scenePlaylistTrackCount(option: ScenePlaylistOption) {
  return recommendationTracks(option.source, option.tags).length
}

function scenePlaylistDescription(option: ScenePlaylistOption) {
  const count = scenePlaylistTrackCount(option)
  if (count > 0) {
    const tagLabel = option.tags.length > 0 ? option.tags.join(' / ') : '喜欢 / 最近 / 标签'
    return `${count} 首可播 · ${tagLabel}`
  }

  return option.tags.length > 0 ? `需要标签：${option.tags.join(' / ')}` : option.description
}

function aiRecommendationTrackCount(option: AiRecommendationOption) {
  return recommendationTracks(option.source, option.tags).length
}

function aiRecommendationDescription(option: AiRecommendationOption) {
  const count = aiRecommendationTrackCount(option)
  return count > 0 ? `${option.description} · ${count} 首可选` : option.description
}

async function playScenePlaylist(option: ScenePlaylistOption) {
  await playRecommendedTracks(recommendationTracks(option.source, option.tags), option.title)
}

async function playAiRecommendation(option: AiRecommendationOption) {
  await playRecommendedTracks(recommendationTracks(option.source, option.tags), option.title)
}

async function playRecommendedTracks(trackList: MusicTrack[], label: string) {
  const recommendedTracks = uniqueRecommendedTracks(trackList)
  const [firstTrack, ...remainingTracks] = recommendedTracks

  if (!firstTrack) {
    playerStatus.value = ''
    playerError.value = '还没有符合条件的歌曲。请先导入音乐，并给歌曲添加对应标签或收藏。'
    return
  }

  playQueue.value = remainingTracks.map((track) => track.id)
  activeTrackActionsId.value = null
  playerError.value = ''
  await playTrackById(firstTrack.id)

  if (!playerError.value) {
    const queuedLabel =
      remainingTracks.length > 0 ? `，并排队 ${remainingTracks.length} 首后续歌曲` : ''
    playerStatus.value = `已为你播放「${label}」：《${firstTrack.title}》${queuedLabel}。`
  }
}

async function playTracksByQuery(query: string) {
  const normalizedQuery = normalizeMusicQuery(query)
  if (!normalizedQuery) {
    playerStatus.value = ''
    playerError.value = '没有收到要点播的歌名。'
    return
  }

  const matchedTracks = tracksByQuery(normalizedQuery)
  const [firstTrack, ...remainingTracks] = matchedTracks
  if (!firstTrack) {
    playerStatus.value = ''
    playerError.value = `没有找到与“${normalizedQuery}”匹配的本地歌曲。`
    return
  }

  playQueue.value = remainingTracks.map((track) => track.id)
  activeTrackActionsId.value = null
  playerError.value = ''
  await playTrackById(firstTrack.id)

  if (!playerError.value) {
    const queuedLabel =
      remainingTracks.length > 0 ? `，并排队 ${remainingTracks.length} 首相近歌曲` : ''
    playerStatus.value = `已为你点播《${firstTrack.title}》${queuedLabel}。`
  }
}

function tracksByQuery(query: string) {
  const normalizedQuery = normalizeMusicQueryText(query)
  const queryTokens = musicQueryTokens(query)
  if (!normalizedQuery && queryTokens.length === 0) {
    return []
  }

  return tracks.value
    .map((track) => ({
      track,
      score: musicQueryMatchScore(track, normalizedQuery, queryTokens),
    }))
    .filter((item) => item.score > 0)
    .sort((left, right) => {
      const scoreDelta = right.score - left.score
      if (scoreDelta !== 0) {
        return scoreDelta
      }

      const playedDelta = playedAtTime(right.track.lastPlayedAt) - playedAtTime(left.track.lastPlayedAt)
      if (playedDelta !== 0) {
        return playedDelta
      }

      return left.track.title.localeCompare(right.track.title)
    })
    .map((item) => item.track)
}

function musicQueryMatchScore(track: MusicTrack, normalizedQuery: string, queryTokens: string[]) {
  const title = normalizeMusicQueryText(track.title)
  const artist = normalizeMusicQueryText(track.artist)
  const album = normalizeMusicQueryText(track.album)
  const combined = `${title} ${artist} ${album}`.trim()
  let score = 0

  if (normalizedQuery && title === normalizedQuery) {
    score += 120
  } else if (normalizedQuery && title.includes(normalizedQuery)) {
    score += 90
  } else if (normalizedQuery && combined.includes(normalizedQuery)) {
    score += 70
  }

  if (queryTokens.length > 0 && queryTokens.every((token) => combined.includes(token))) {
    score += 60
  }

  for (const token of queryTokens) {
    if (title === token) {
      score += 40
    } else if (title.includes(token)) {
      score += 24
    }
    if (artist.includes(token)) {
      score += 14
    }
    if (album.includes(token)) {
      score += 8
    }
  }

  score += recommendationScore(track, preferredRecommendationTags()) * 0.1
  return score
}

function normalizeMusicQuery(value?: string | null) {
  return String(value ?? '')
    .trim()
    .replace(/^[《"'“‘]+|[》"'”’]+$/g, '')
    .replace(/\s+/g, ' ')
    .replace(/^(?:一首|首|这首歌|那首歌|歌曲|音乐|点首|点一下|放一下)/, '')
    .replace(/(?:可以吗|好不好|好吗|行吗|谢谢|拜托|一下|吧|呀|啊)$/g, '')
    .trim()
    .slice(0, 80)
}

function normalizeMusicQueryText(value?: string | null) {
  return normalizeMusicQuery(value)
    .toLowerCase()
    .replace(/[·・.。,:：，、\-_\s"'“”‘’《》()[\]（）【】]/g, '')
}

function musicQueryTokens(query: string) {
  const tokens = normalizeMusicQuery(query)
    .toLowerCase()
    .split(/[\s·・.。,:：，、\-_"'“”‘’《》()[\]（）【】]|的/g)
    .map((token) => normalizeMusicQueryText(token))
    .filter((token) => token.length >= 2)
  return Array.from(new Set(tokens)).slice(0, 8)
}

function recommendationTracks(source: MusicRecommendationSource, tags: string[]) {
  if (source === 'smart') {
    return smartRecommendationTracks()
  }

  if (source === 'favorites') {
    return sortRecommendedTracks(
      tracks.value.filter((track) => track.favorite),
      preferredRecommendationTags(),
    )
  }

  if (source === 'recent') {
    return sortRecommendedTracks(
      tracks.value.filter((track) => track.lastPlayedAt),
      preferredRecommendationTags(),
    )
  }

  const targetTags = normalizeTrackTags(tags)
  if (targetTags.length > 0) {
    return sortRecommendedTracks(
      tracks.value.filter((track) => trackMatchesAnyRecommendationTag(track, targetTags)),
      targetTags,
    )
  }

  const preferredTags = preferredRecommendationTags()
  const taggedTracks = tracks.value.filter((track) => normalizeTrackTags(track.tags).length > 0)
  const matchedTracks =
    preferredTags.length > 0
      ? taggedTracks.filter((track) => trackMatchesAnyRecommendationTag(track, preferredTags))
      : taggedTracks

  return sortRecommendedTracks(matchedTracks.length > 0 ? matchedTracks : taggedTracks, preferredTags)
}

function smartRecommendationTracks() {
  const preferredTags = preferredRecommendationTags()
  const signaledTracks = tracks.value.filter((track) => {
    if (track.favorite || track.lastPlayedAt) {
      return true
    }

    return preferredTags.length > 0 && trackMatchesAnyRecommendationTag(track, preferredTags)
  })

  return sortRecommendedTracks(signaledTracks.length > 0 ? signaledTracks : tracks.value, preferredTags)
}

function preferredRecommendationTags() {
  const sourceTracks = tracks.value.filter((track) => track.favorite || track.lastPlayedAt)
  const candidates = sourceTracks.length > 0 ? sourceTracks : tracks.value
  const tagScores = new Map<string, number>()

  for (const track of candidates) {
    for (const tag of normalizeTrackTags(track.tags)) {
      const score = (track.favorite ? 3 : 1) + (track.lastPlayedAt ? 2 : 0)
      tagScores.set(tag, (tagScores.get(tag) ?? 0) + score)
    }
  }

  return Array.from(tagScores.entries())
    .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
    .slice(0, 4)
    .map(([tag]) => tag)
}

function trackMatchesAnyRecommendationTag(track: MusicTrack, tags: string[]) {
  const trackTags = normalizeTrackTags(track.tags)
  const category = normalizeMusicCategory(track.category)

  return tags.some((tag) => {
    if (tag === '收藏' || tag === '喜欢') {
      return track.favorite || trackTags.includes(tag)
    }
    if (tag === '常听') {
      return sanitizePlayCount(track.playCount) > 0 || trackTags.includes(tag)
    }

    return trackTags.includes(tag) || category === tag
  })
}

function sortRecommendedTracks(trackList: MusicTrack[], tags: string[]) {
  return uniqueRecommendedTracks(trackList).sort((left, right) => {
    const scoreDelta = recommendationScore(right, tags) - recommendationScore(left, tags)
    if (scoreDelta !== 0) {
      return scoreDelta
    }

    const playedDelta = playedAtTime(right.lastPlayedAt) - playedAtTime(left.lastPlayedAt)
    if (playedDelta !== 0) {
      return playedDelta
    }

    return left.title.localeCompare(right.title)
  })
}

function recommendationScore(track: MusicTrack, tags: string[]) {
  const trackTags = normalizeTrackTags(track.tags)
  const category = normalizeMusicCategory(track.category)
  let score = 0

  tags.forEach((tag, index) => {
    const tagWeight = Math.max(4, 12 - index * 2)
    if (trackTags.includes(tag)) {
      score += tagWeight
    }
    if (category === tag) {
      score += Math.max(2, tagWeight / 2)
    }
  })

  if (track.favorite) {
    score += 6
  }
  if (trackTags.length > 0) {
    score += 1
  }

  score += Math.min(sanitizePlayCount(track.playCount), 30) * 0.2

  const lastPlayed = playedAtTime(track.lastPlayedAt)
  if (lastPlayed > 0) {
    const daysSincePlayed = Math.max(0, (Date.now() - lastPlayed) / 86400000)
    score += Math.max(0, 5 - daysSincePlayed / 7)
  }

  return score
}

function uniqueRecommendedTracks(trackList: MusicTrack[]) {
  const seenTrackIds = new Set<string>()
  return trackList.filter((track) => {
    if (seenTrackIds.has(track.id)) {
      return false
    }

    seenTrackIds.add(track.id)
    return true
  })
}

async function handleMusicActionRequest(request: MusicActionRequest) {
  if (!request || request.type !== 'music_action') {
    return
  }

  playerError.value = ''
  activeTrackActionsId.value = null

  if (request.action === 'play_music') {
    await playRecommendedTracks(smartRecommendationTracks(), 'AI 推荐')
    return
  }

  if (request.action === 'play_by_query') {
    await playTracksByQuery(request.query ?? '')
    return
  }

  if (request.action === 'play_by_tags') {
    const tags = normalizeTrackTags(request.tags)
    await playRecommendedTracks(recommendationTracks('tags', tags), tags.join(' / ') || '标签推荐')
    return
  }

  if (request.action === 'start_focus_mode') {
    await playRecommendedTracks(
      recommendationTracks('tags', ['学习', '工作', '纯音乐', '中速', '安静']),
      '专注模式',
    )
    return
  }

  if (request.action === 'start_sleep_mode') {
    setPlayerVolume(Math.min(volume.value, 0.36))
    await playRecommendedTracks(
      recommendationTracks('tags', ['睡觉', '安静', '慢歌', '纯音乐', '治愈']),
      '睡眠模式',
    )
    return
  }

  if (request.action === 'start_mood_mode') {
    const tags = normalizeTrackTags(request.tags)
    await playRecommendedTracks(
      recommendationTracks('tags', tags.length > 0 ? tags : ['治愈', '安静', '慢歌']),
      '情绪陪伴',
    )
    return
  }

  if (request.action === 'pause') {
    pausePlayback()
    playerStatus.value = '已暂停音乐。'
    return
  }

  if (request.action === 'resume') {
    if (currentTrack.value) {
      await playCurrent()
    } else if (tracks.value.length > 0) {
      await playTrack(0)
    } else {
      playerError.value = '还没有可播放的本地音乐。'
      return
    }

    if (!playerError.value) {
      playerStatus.value = currentTrack.value ? `已继续播放《${currentTrack.value.title}》。` : '已继续播放音乐。'
    }
    return
  }

  if (request.action === 'next') {
    await playNext(true)
    if (!playerError.value) {
      playerStatus.value = currentTrack.value ? `已切到《${currentTrack.value.title}》。` : '已切到下一首。'
    }
    return
  }

  if (request.action === 'previous') {
    await playPrevious()
    if (!playerError.value) {
      playerStatus.value = currentTrack.value ? `已回到《${currentTrack.value.title}》。` : '已切到上一首。'
    }
    return
  }

  if (request.action === 'set_volume') {
    const nextVolume =
      typeof request.volume === 'number'
        ? request.volume
        : volume.value + (typeof request.volumeDelta === 'number' ? request.volumeDelta : 0)
    setPlayerVolume(nextVolume)
    playerStatus.value = `音量已调整到 ${Math.round(volume.value * 100)}%。`
    return
  }

  if (request.action === 'favorite_current') {
    if (!currentTrack.value) {
      playerError.value = '当前没有正在播放的歌曲。'
      return
    }

    currentTrack.value.favorite = true
    playerStatus.value = `已收藏《${currentTrack.value.title}》。`
    return
  }

  if (request.action === 'skip_current') {
    markCurrentTrackSkipped()
    await playNext(true)
    if (!playerError.value) {
      playerStatus.value = currentTrack.value ? `已跳过，正在播放《${currentTrack.value.title}》。` : '已跳过当前歌曲。'
    }
  }
}

function markCurrentTrackSkipped() {
  if (!currentTrack.value) {
    return
  }

  const tags = normalizeTrackTags(currentTrack.value.tags)
  if (!tags.includes('跳过') && tags.length < MAX_TRACK_TAGS) {
    currentTrack.value.tags = normalizeTrackTags([...tags, '跳过'])
  }
}

async function togglePlay() {
  if (!currentTrack.value && tracks.value.length > 0) {
    await playTrack(0)
    return
  }

  if (!audio.value || !currentTrack.value) {
    return
  }

  if (playing.value) {
    audio.value.pause()
    playing.value = false
    return
  }

  await playCurrent()
}

async function playTrack(index: number) {
  if (index < 0 || index >= tracks.value.length) {
    return
  }

  neteaseCurrentTrack.value = null
  currentIndex.value = index
  currentTime.value = 0
  visualPlaybackTime.value = 0
  duration.value = 0
  playerError.value = ''
  await nextTick()
  await playCurrent(true)
}

async function playTrackById(trackId: string) {
  const index = tracks.value.findIndex((track) => track.id === trackId)
  if (index >= 0) {
    if (activeLibraryView.value === 'queue') {
      removeTrackFromQueue(trackId)
    }
    await playTrack(index)
  }
}

function immersiveTrackActive(track: MusicTrack) {
  return currentTrack.value?.id === track.id
}

function immersiveNeteaseTrackActive(track: NeteasePlaylistTrack) {
  return currentTrack.value?.source === 'netease' && currentTrack.value.neteaseSongId === track.id
}

function canPlayImmersiveTrack(track: MusicTrack) {
  return track.source !== 'netease' && !immersiveTrackActive(track)
}

function canPlayImmersiveNeteaseTrack(track: NeteasePlaylistTrack) {
  return neteaseLoggedIn.value && !immersiveNeteaseTrackActive(track) && neteaseTrackActionId.value !== track.id
}

async function playImmersiveTrack(track: MusicTrack) {
  if (!canPlayImmersiveTrack(track)) {
    return
  }

  immersivePlaylistSource.value = 'local'
  await playTrackById(track.id)
}

async function playImmersiveNeteaseTrack(track: NeteasePlaylistTrack) {
  if (!canPlayImmersiveNeteaseTrack(track)) {
    return
  }

  immersivePlaylistSource.value = 'netease'
  await playNeteaseTrack(track)
}

function toggleImmersivePlaylistVisible() {
  immersivePlaylistVisible.value = !immersivePlaylistVisible.value
}

function toggleImmersiveRhythmPanelVisible() {
  immersiveRhythmPanelVisible.value = !immersiveRhythmPanelVisible.value
}

async function playCurrent(resetTime = false) {
  if (!audio.value || !currentTrack.value) {
    return
  }

  const track = currentTrack.value
  try {
    if (resetTime) {
      audio.value.currentTime = 0
    }
    await audio.value.play()
    playing.value = true
    if (immersiveMode.value) {
      void prepareImmersiveVisualization()
    }
    if (resetTime && track === currentTrack.value) {
      recordTrackPlayback(track)
    }
  } catch (err) {
    playing.value = false
    playerError.value = `无法播放该音频：${String(err)}`
  }
}

function recordTrackPlayback(track: MusicTrack) {
  const playedAt = new Date().toISOString()
  track.playCount = sanitizePlayCount(track.playCount) + 1
  track.lastPlayedAt = playedAt
  track.playHistory = [playedAt, ...normalizeTrackHistory(track.playHistory)].slice(
    0,
    MAX_PLAY_HISTORY_PER_TRACK,
  )
}

function pausePlayback() {
  audio.value?.pause()
  playing.value = false
}

async function playPrevious() {
  if (tracks.value.length === 0) {
    return
  }

  if (audio.value && audio.value.currentTime > 4) {
    audio.value.currentTime = 0
    currentTime.value = 0
    return
  }

  const previousIndex =
    currentIndex.value <= 0 ? tracks.value.length - 1 : currentIndex.value - 1
  await playTrack(previousIndex)
}

async function playNext(manual = true) {
  const index = nextTrackIndex(manual)
  if (index < 0) {
    pausePlayback()
    if (audio.value) {
      audio.value.currentTime = 0
    }
    currentTime.value = 0
    return
  }

  await playTrack(index)
}

function nextTrackIndex(manual: boolean) {
  const count = tracks.value.length
  if (count === 0) {
    return -1
  }

  const queuedIndex = dequeueNextTrackIndex()
  if (queuedIndex >= 0) {
    return queuedIndex
  }

  if (shuffleEnabled.value && count > 1) {
    let next = currentIndex.value
    while (next === currentIndex.value) {
      next = Math.floor(Math.random() * count)
    }
    return next
  }

  const next = currentIndex.value + 1
  if (next < count) {
    return next
  }

  if (manual || repeatMode.value === 'all') {
    return 0
  }

  return -1
}

function dequeueNextTrackIndex() {
  while (playQueue.value.length > 0) {
    const [nextTrackId, ...remainingQueue] = playQueue.value
    playQueue.value = remainingQueue
    const queuedIndex = tracks.value.findIndex((track) => track.id === nextTrackId)
    if (queuedIndex >= 0) {
      return queuedIndex
    }
  }

  return -1
}

function handleLoadedMetadata() {
  const loadedDuration = audio.value?.duration ?? 0
  duration.value = loadedDuration

  const sanitizedDuration = sanitizeTrackDuration(loadedDuration)
  if (currentTrack.value && sanitizedDuration) {
    currentTrack.value.duration = sanitizedDuration
  }
}

function handleTimeUpdate() {
  const time = audio.value?.currentTime ?? 0
  currentTime.value = time
  if (!immersiveMode.value || !playing.value) {
    visualPlaybackTime.value = time
  }
}

async function handleEnded() {
  if (repeatMode.value === 'one') {
    await playCurrent(true)
    return
  }

  await playNext(false)
}

function handleAudioError() {
  playing.value = false
  playerStatus.value = ''
  playerError.value = currentTrackOnline.value
    ? '当前网易云音频无法读取，播放链接可能已过期或受版权、会员、地区限制。'
    : '当前音频无法读取，请确认文件仍在原位置并且格式受系统支持。'
}

async function prepareImmersiveVisualization() {
  if (currentTrack.value) {
    void analyzeCurrentTrackBeatMap()
    void loadCurrentTrackLyrics()
  }

  if (audio.value && playing.value) {
    startVisualClock()
    connectAudioElement(audio.value)
    void resumeAnalyzer()
  }
}

function startVisualClock() {
  if (visualClockFrameId !== null) {
    return
  }

  lastVisualClockUpdate = 0

  const tick = (timestamp: number) => {
    visualClockFrameId = window.requestAnimationFrame(tick)
    if (!immersiveMode.value || !playing.value) {
      return
    }

    if (timestamp - lastVisualClockUpdate < 1000 / 30) {
      return
    }

    lastVisualClockUpdate = timestamp
    syncVisualPlaybackTime()
  }

  visualClockFrameId = window.requestAnimationFrame(tick)
}

function stopVisualClock() {
  if (visualClockFrameId !== null) {
    window.cancelAnimationFrame(visualClockFrameId)
    visualClockFrameId = null
  }
}

function syncVisualPlaybackTime() {
  const time = audio.value?.currentTime ?? currentTime.value
  if (Number.isFinite(time)) {
    visualPlaybackTime.value = time
  }
}

async function analyzeCurrentTrackBeatMap() {
  const track = currentTrack.value
  if (!track) {
    return
  }

  if (track.source === 'netease') {
    beatMapRequestedTrackId = track.id
    return
  }

  if (beatMapMatchesCurrentTrack.value) {
    return
  }

  if (beatMapStatus.value === 'analyzing' && beatMapRequestedTrackId === track.id) {
    return
  }

  beatMapRequestedTrackId = track.id
  await analyzeTrackBeatMap({
    id: track.id,
    url: track.url,
    duration: track.duration,
  })
}

async function loadCurrentTrackLyrics() {
  const track = currentTrack.value
  if (!track) {
    return
  }

  if (
    lyricsRequestedTrackId === track.id &&
    (lyricsStatus.value === 'loading' || lyricsStatus.value === 'ready')
  ) {
    return
  }

  lyricsRequestedTrackId = track.id
  await loadLyricsForTrack(track)
}

function seek(event: Event) {
  const nextTime = Number((event.target as HTMLInputElement).value)
  if (!audio.value || !Number.isFinite(nextTime)) {
    return
  }

  audio.value.currentTime = nextTime
  currentTime.value = nextTime
  visualPlaybackTime.value = nextTime
}

function setVolume(event: Event) {
  setPlayerVolume(Number((event.target as HTMLInputElement).value))
}

function setPlayerVolume(value: number) {
  volume.value = clamp(Number.isFinite(value) ? value : volume.value, 0, 1)
}

function syncAudioVolume() {
  if (audio.value) {
    audio.value.volume = volume.value
  }
}

function toggleRepeatMode() {
  repeatMode.value =
    repeatMode.value === 'all' ? 'one' : repeatMode.value === 'one' ? 'none' : 'all'
}

async function removeTrack(trackId: string) {
  const index = tracks.value.findIndex((track) => track.id === trackId)
  if (index < 0) {
    return
  }

  const removingCurrent = index === currentIndex.value
  activeTrackActionsId.value = null
  removeTrackFromQueue(trackId)
  tracks.value.splice(index, 1)

  if (tracks.value.length === 0) {
    pausePlayback()
    currentIndex.value = -1
    currentTime.value = 0
    duration.value = 0
    return
  }

  if (index < currentIndex.value) {
    currentIndex.value -= 1
    return
  }

  if (removingCurrent) {
    currentIndex.value = Math.min(index, tracks.value.length - 1)
    await nextTick()
    if (playing.value) {
      await playCurrent(true)
    }
  }
}

function trackNumber(trackId: string) {
  const index = tracks.value.findIndex((track) => track.id === trackId)
  return index >= 0 ? index + 1 : 0
}

function trackDisplayNumber(trackId: string) {
  return activeLibraryView.value === 'queue' ? queuePosition(trackId) : trackNumber(trackId)
}

function updateTrackCategory(track: MusicTrack) {
  track.category = normalizeMusicCategory(track.category)
}

function updateTrackTags(track: MusicTrack, value: string) {
  track.tags = normalizeTrackTags(value)
}

function trackHasPresetTag(track: MusicTrack, tag: string) {
  return normalizeTrackTags(track.tags).includes(tag)
}

function toggleTrackPresetTag(track: MusicTrack, tag: string) {
  const normalizedTag = normalizeTrackTags([tag])[0]
  if (!normalizedTag) {
    return
  }

  const currentTags = normalizeTrackTags(track.tags)
  if (currentTags.includes(normalizedTag)) {
    track.tags = currentTags.filter((item) => item !== normalizedTag)
    playerError.value = ''
    playerStatus.value = `已从《${track.title}》移除标签「${normalizedTag}」。`
    return
  }

  if (currentTags.length >= MAX_TRACK_TAGS) {
    playerError.value = `每首歌最多 ${MAX_TRACK_TAGS} 个标签。`
    return
  }

  track.tags = normalizeTrackTags([...currentTags, normalizedTag])
  playerError.value = ''
  playerStatus.value = `已为《${track.title}》添加标签「${normalizedTag}」。`
}

function inputEventValue(event: Event) {
  return event.target instanceof HTMLInputElement ? event.target.value : ''
}

function updateTrackIdentity(track: MusicTrack) {
  const inferredIdentity = inferTrackIdentity(trackTitleFromPath(track.sourcePath || track.path))
  track.title = track.title.trim() || inferredIdentity.title
  track.artist = normalizeTrackArtist(track.artist)
  track.album = normalizeTrackAlbum(track.album)
  track.tags = normalizeTrackTags(track.tags)
}

function toggleTrackEditor(trackId: string) {
  editingTrackId.value = editingTrackId.value === trackId ? null : trackId
  activeTrackActionsId.value = null
  recognitionCandidate.value = null
  playerError.value = ''
  playerStatus.value = ''
}

function closeTrackDialog() {
  editingTrackId.value = null
  recognitionCandidate.value = null
}

function finishTrackIdentityEdit(track: MusicTrack) {
  updateTrackIdentity(track)
  editingTrackId.value = null
  activeTrackActionsId.value = null
  playerError.value = ''
  playerStatus.value = `已更新《${track.title}》的歌曲信息。`
}

async function recognizeTrackMetadata(track: MusicTrack) {
  activeTrackActionsId.value = null
  editingTrackId.value = null
  recognitionCandidate.value = null
  recognitionBusyTrackId.value = track.id
  playerError.value = ''
  playerStatus.value = '正在读取本地音频 metadata 标签...'

  try {
    const metadata = await invoke<MusicMetadataResult>('read_music_metadata', { path: track.path })
    const candidate = createRecognitionCandidate(track, metadata)
    if (!candidate.title && !candidate.artist && !candidate.album && !candidate.duration) {
      playerStatus.value = ''
      playerError.value = 'metadata 中没有可用的歌名或歌手信息，请手动编辑歌曲信息。'
      return
    }

    recognitionCandidate.value = candidate
    playerStatus.value = '已读取 metadata 标签，请确认是否应用到歌曲信息。'
  } catch (err) {
    playerStatus.value = ''
    playerError.value = `metadata 读取失败：${String(err)}`
  } finally {
    recognitionBusyTrackId.value = null
  }
}

function createRecognitionCandidate(
  track: MusicTrack,
  metadata: MusicMetadataResult,
): MusicRecognitionCandidate {
  return {
    trackId: track.id,
    title: metadata.title?.trim() || '',
    artist: metadata.artist?.trim() || '',
    album: metadata.album?.trim() || '',
    duration: sanitizeTrackDuration(metadata.duration),
    source: metadata.source || 'metadata',
    confidence: clamp(metadata.confidence ?? 0, 0, 1),
    warnings: Array.isArray(metadata.warnings) ? metadata.warnings : [],
  }
}

function applyRecognitionCandidate() {
  if (!recognitionCandidate.value) {
    return
  }

  const candidate = recognitionCandidate.value
  const track = trackById(candidate.trackId)
  if (!track) {
    recognitionCandidate.value = null
    return
  }

  if (candidate.title) {
    track.title = candidate.title
  }
  if (candidate.artist) {
    track.artist = candidate.artist
  }
  if (candidate.album) {
    track.album = candidate.album
  }
  if (candidate.duration) {
    track.duration = candidate.duration
  }

  recognitionCandidate.value = null
  playerError.value = ''
  playerStatus.value = `已应用《${track.title}》的识别结果。`
}

function dismissRecognitionCandidate() {
  recognitionCandidate.value = null
  playerStatus.value = ''
}

function toggleTrackFavorite(track: MusicTrack) {
  track.favorite = !track.favorite
  activeTrackActionsId.value = null
  playerError.value = ''
  playerStatus.value = track.favorite
    ? `已收藏《${track.title}》。`
    : `已取消收藏《${track.title}》。`
}

function queueTrackNext(track: MusicTrack) {
  insertTrackIntoQueue(track.id, 0)
  activeTrackActionsId.value = null
  playerError.value = ''
  playerStatus.value = `已将《${track.title}》添加到下一首。`
}

function queueTrackEnd(track: MusicTrack) {
  insertTrackIntoQueue(track.id, playQueue.value.length)
  activeTrackActionsId.value = null
  playerError.value = ''
  playerStatus.value = `已将《${track.title}》添加到队尾。`
}

function insertTrackIntoQueue(trackId: string, index: number) {
  if (!trackById(trackId)) {
    return
  }

  const nextQueue = playQueue.value.filter((queuedTrackId) => queuedTrackId !== trackId)
  const targetIndex = clamp(index, 0, nextQueue.length)
  nextQueue.splice(targetIndex, 0, trackId)
  playQueue.value = nextQueue
}

function removeTrackFromQueue(trackId: string) {
  if (!playQueue.value.includes(trackId)) {
    return
  }

  playQueue.value = playQueue.value.filter((queuedTrackId) => queuedTrackId !== trackId)
}

function removeQueuedTrack(track: MusicTrack) {
  removeTrackFromQueue(track.id)
  activeTrackActionsId.value = null
  playerError.value = ''
  playerStatus.value = `已从播放队列移除《${track.title}》。`
}

function clearPlaybackQueue() {
  playQueue.value = []
  playerError.value = ''
  playerStatus.value = '已清空播放队列。'
}

function clearCurrentList() {
  if (activeLibraryView.value === 'queue') {
    clearPlaybackQueue()
    return
  }

  clearPlaylist()
}

function isTrackQueued(trackId: string) {
  return playQueue.value.includes(trackId)
}

function queuePosition(trackId: string) {
  const index = playQueue.value.findIndex((queuedTrackId) => queuedTrackId === trackId)
  return index >= 0 ? index + 1 : 0
}

function startQueueDrag(trackId: string, event: DragEvent) {
  if (activeLibraryView.value !== 'queue') {
    return
  }

  draggingQueueTrackId.value = trackId
  event.dataTransfer?.setData('text/plain', trackId)
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move'
  }
}

function dropQueueTrack(targetTrackId: string, event: DragEvent) {
  if (activeLibraryView.value !== 'queue') {
    return
  }

  event.preventDefault()
  const draggedTrackId =
    draggingQueueTrackId.value || event.dataTransfer?.getData('text/plain') || ''
  moveQueuedTrack(draggedTrackId, targetTrackId)
  draggingQueueTrackId.value = null
}

function moveQueuedTrack(draggedTrackId: string, targetTrackId: string) {
  if (!draggedTrackId || draggedTrackId === targetTrackId) {
    return
  }

  const nextQueue = playQueue.value.slice()
  const fromIndex = nextQueue.findIndex((trackId) => trackId === draggedTrackId)
  const toIndex = nextQueue.findIndex((trackId) => trackId === targetTrackId)
  if (fromIndex < 0 || toIndex < 0) {
    return
  }

  const [movedTrackId] = nextQueue.splice(fromIndex, 1)
  nextQueue.splice(toIndex, 0, movedTrackId)
  playQueue.value = nextQueue
}

function finishQueueDrag() {
  draggingQueueTrackId.value = null
}

function toggleTrackActions(trackId: string) {
  activeTrackActionsId.value = activeTrackActionsId.value === trackId ? null : trackId
}

function openTrackActions(trackId: string) {
  activeTrackActionsId.value = trackId
}

function clearMusicSearch() {
  searchQuery.value = ''
}

function createCustomPlaylist() {
  const name = normalizePlaylistName(newPlaylistName.value) || '新建歌单'
  const playlist: MusicPlaylist = {
    id: createTrackId(),
    name: uniquePlaylistName(name),
    trackIds: [],
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  }

  customPlaylists.value.push(playlist)
  activeCustomPlaylistId.value = playlist.id
  newPlaylistName.value = ''
  playerError.value = ''
  playerStatus.value = `已新建歌单「${playlist.name}」。`
}

function uniquePlaylistName(name: string, currentPlaylistId = '') {
  const baseName = normalizePlaylistName(name) || '新建歌单'
  const existingNames = new Set(
    customPlaylists.value
      .filter((playlist) => playlist.id !== currentPlaylistId)
      .map((playlist) => playlist.name),
  )

  if (!existingNames.has(baseName)) {
    return baseName
  }

  for (let index = 2; index < 1000; index += 1) {
    const candidate = normalizePlaylistName(`${baseName} ${index}`)
    if (!existingNames.has(candidate)) {
      return candidate
    }
  }

  return `${baseName}-${Date.now()}`
}

function selectCustomPlaylist(playlistId: string) {
  activeCustomPlaylistId.value = activeCustomPlaylistId.value === playlistId ? '' : playlistId
}

function renameCustomPlaylist(playlist: MusicPlaylist, value: string) {
  const name = uniquePlaylistName(value, playlist.id)
  playlist.name = name
  playlist.updatedAt = new Date().toISOString()
  playerError.value = ''
  playerStatus.value = `已重命名歌单为「${name}」。`
}

function renameCustomPlaylistWithPrompt(playlist: MusicPlaylist) {
  const nextName = window.prompt('新的歌单名称', playlist.name)
  if (nextName === null) {
    return
  }

  renameCustomPlaylist(playlist, nextName)
}

function deleteCustomPlaylist(playlistId: string) {
  const playlist = customPlaylists.value.find((item) => item.id === playlistId)
  if (playlist && !window.confirm(`删除歌单「${playlist.name}」？歌曲不会从本地音乐库移除。`)) {
    return
  }

  customPlaylists.value = customPlaylists.value.filter((item) => item.id !== playlistId)
  if (activeCustomPlaylistId.value === playlistId) {
    activeCustomPlaylistId.value = ''
  }
  if (playlistTrackPickerPlaylistId.value === playlistId) {
    closePlaylistTrackPicker()
  }
  playerError.value = ''
  playerStatus.value = playlist ? `已删除歌单「${playlist.name}」。` : '已删除歌单。'
}

function customPlaylistTracks(playlist: MusicPlaylist) {
  return normalizePlaylistTrackIds(playlist.trackIds)
    .map((trackId) => trackById(trackId))
    .filter((track): track is MusicTrack => Boolean(track))
}

function playlistTrackCountLabel(playlist: MusicPlaylist) {
  const count = customPlaylistTracks(playlist).length
  return count > 0 ? `${count} 首歌曲` : '空歌单'
}

function playlistPreviewLabel(playlist: MusicPlaylist) {
  const playlistTracks = customPlaylistTracks(playlist)
  if (playlistTracks.length === 0) {
    return '添加歌曲后会显示在这里'
  }

  return playlistTracks
    .slice(0, 3)
    .map((track) => track.title)
    .join(' / ')
}

function openPlaylistTrackPicker(playlist: MusicPlaylist) {
  activeCustomPlaylistId.value = playlist.id
  playlistTrackPickerPlaylistId.value = playlist.id
  playlistTrackPickerQuery.value = ''
  playlistTrackPickerSelectedIds.value = []
  playlistTrackPickerVisible.value = true
  playerError.value = ''
  playerStatus.value = ''
}

function closePlaylistTrackPicker() {
  playlistTrackPickerVisible.value = false
  playlistTrackPickerPlaylistId.value = ''
  playlistTrackPickerQuery.value = ''
  playlistTrackPickerSelectedIds.value = []
}

function togglePlaylistTrackPickerSelection(trackId: string) {
  if (playlistTrackPickerSelectedIds.value.includes(trackId)) {
    playlistTrackPickerSelectedIds.value = playlistTrackPickerSelectedIds.value.filter(
      (item) => item !== trackId,
    )
    return
  }

  playlistTrackPickerSelectedIds.value = [...playlistTrackPickerSelectedIds.value, trackId]
}

function addPickedTracksToPlaylist() {
  const playlist = playlistTrackPickerTarget.value
  if (!playlist) {
    closePlaylistTrackPicker()
    return
  }

  const trackIds = playlistTrackPickerSelectedIds.value.filter(
    (trackId) => trackById(trackId) && !playlist.trackIds.includes(trackId),
  )
  if (trackIds.length === 0) {
    playerError.value = '请先选择要加入歌单的歌曲。'
    return
  }

  playlist.trackIds = normalizePlaylistTrackIds([...playlist.trackIds, ...trackIds])
  playlist.updatedAt = new Date().toISOString()
  closePlaylistTrackPicker()
  playerError.value = ''
  playerStatus.value = `已将 ${trackIds.length} 首歌曲加入「${playlist.name}」。`
}

function removeTrackFromPlaylist(playlist: MusicPlaylist, trackId: string) {
  const track = trackById(trackId)
  playlist.trackIds = playlist.trackIds.filter((item) => item !== trackId)
  playlist.updatedAt = new Date().toISOString()
  playerError.value = ''
  playerStatus.value = track
    ? `已从「${playlist.name}」移除《${track.title}》。`
    : `已从「${playlist.name}」移除歌曲。`
}

async function playCustomPlaylist(playlist: MusicPlaylist) {
  const firstTrackId = normalizePlaylistTrackIds(playlist.trackIds)[0]
  if (!firstTrackId) {
    playerError.value = '这个歌单还没有歌曲。'
    return
  }

  await playTrackById(firstTrackId)
}

function queueCustomPlaylist(playlist: MusicPlaylist) {
  const trackIds = normalizePlaylistTrackIds(playlist.trackIds)
  if (trackIds.length === 0) {
    playerError.value = '这个歌单还没有歌曲。'
    return
  }

  const nextQueue = playQueue.value.filter((trackId) => !trackIds.includes(trackId))
  playQueue.value = [...nextQueue, ...trackIds]
  playerError.value = ''
  playerStatus.value = `已将「${playlist.name}」加入播放队列。`
}

function startPlaylistTrackDrag(trackId: string, event: DragEvent) {
  draggingPlaylistTrackId.value = trackId
  event.dataTransfer?.setData('text/plain', trackId)
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move'
  }
}

function dropPlaylistTrack(playlist: MusicPlaylist, targetTrackId: string, event: DragEvent) {
  event.preventDefault()
  const draggedTrackId =
    draggingPlaylistTrackId.value || event.dataTransfer?.getData('text/plain') || ''
  movePlaylistTrack(playlist, draggedTrackId, targetTrackId)
  draggingPlaylistTrackId.value = null
}

function movePlaylistTrack(
  playlist: MusicPlaylist,
  draggedTrackId: string,
  targetTrackId: string,
) {
  if (!draggedTrackId || draggedTrackId === targetTrackId) {
    return
  }

  const nextTrackIds = playlist.trackIds.slice()
  const fromIndex = nextTrackIds.findIndex((trackId) => trackId === draggedTrackId)
  const toIndex = nextTrackIds.findIndex((trackId) => trackId === targetTrackId)
  if (fromIndex < 0 || toIndex < 0) {
    return
  }

  const [movedTrackId] = nextTrackIds.splice(fromIndex, 1)
  nextTrackIds.splice(toIndex, 0, movedTrackId)
  playlist.trackIds = nextTrackIds
  playlist.updatedAt = new Date().toISOString()
}

function finishPlaylistTrackDrag() {
  draggingPlaylistTrackId.value = null
}

function clearPlaylist() {
  pausePlayback()
  resetNeteasePlaybackState()
  tracks.value = []
  playQueue.value = []
  currentIndex.value = -1
  currentTime.value = 0
  duration.value = 0
  playerError.value = ''
  playerStatus.value = ''
}

async function startDrag() {
  if (immersiveMode.value) {
    return
  }

  try {
    await musicWindow.startDragging()
  } catch {
    // Dragging can fail when the pointer is already captured by an inner control.
  }
}

function stopHeaderDrag(event: PointerEvent) {
  event.stopPropagation()
}

async function setImmersiveMode(nextImmersiveMode: boolean) {
  if (immersiveMode.value === nextImmersiveMode) {
    return
  }

  immersiveMode.value = nextImmersiveMode
  if (nextImmersiveMode) {
    miniPlayerMode.value = false
    settingsVisible.value = false
    activeTrackActionsId.value = null
    editingTrackId.value = null
    recognitionCandidate.value = null
    await nextTick()
    syncVisualPlaybackTime()
    void prepareImmersiveVisualization()
    await applyMusicWindowDisplayMode('immersive')
  } else {
    stopVisualClock()
    syncVisualPlaybackTime()
    lyricsRequestedTrackId = ''
    resetLyrics()
    await applyMusicWindowDisplayMode('full')
  }
}

async function toggleImmersiveMode() {
  await setImmersiveMode(!immersiveMode.value)
}

async function setMiniPlayerMode(nextMiniMode: boolean) {
  if (miniPlayerMode.value === nextMiniMode) {
    return
  }

  miniPlayerMode.value = nextMiniMode
  if (nextMiniMode) {
    immersiveMode.value = false
    stopVisualClock()
    syncVisualPlaybackTime()
    lyricsRequestedTrackId = ''
    resetLyrics()
  }
  settingsVisible.value = false
  activeTrackActionsId.value = null
  editingTrackId.value = null
  recognitionCandidate.value = null
  await applyMusicWindowDisplayMode(nextMiniMode ? 'mini' : 'full')
}

async function applyMusicWindowDisplayMode(mode: 'full' | 'mini' | 'immersive') {
  try {
    if (mode === 'immersive') {
      await musicWindow.setFullscreen(true)
      return
    }

    await musicWindow.setFullscreen(false)
    const size = mode === 'mini' ? MINI_MUSIC_WINDOW_SIZE : FULL_MUSIC_WINDOW_SIZE
    await musicWindow.setSize(new LogicalSize(size.width, size.height))
  } catch (err) {
    playerError.value =
      mode === 'immersive'
        ? `无法进入全屏沉浸模式：${String(err)}`
        : `无法恢复音乐窗口尺寸：${String(err)}`
  }
}

async function toggleMiniPlayerMode() {
  await setMiniPlayerMode(!miniPlayerMode.value)
}

async function hideMusicPlayer() {
  if (immersiveMode.value) {
    immersiveMode.value = false
    stopVisualClock()
    syncVisualPlaybackTime()
    lyricsRequestedTrackId = ''
    resetLyrics()
    await applyMusicWindowDisplayMode('full')
  }

  await invoke('hide_music_player')
}

function formatTime(value: number) {
  if (!Number.isFinite(value) || value <= 0) {
    return '0:00'
  }

  const minutes = Math.floor(value / 60)
  const seconds = Math.floor(value % 60)
  return `${minutes}:${seconds.toString().padStart(2, '0')}`
}

function normalizeNeteaseQrStatus(result: NeteaseQrCheckResult) {
  if (result.loggedIn || result.status === 'authorized') {
    return 'authorized'
  }

  if (result.status === 'scanned' || result.code === 802) {
    return 'scanned'
  }

  if (result.status === 'expired' || result.code === 800) {
    return 'expired'
  }

  if (result.status === 'waiting' || result.code === 801) {
    return 'waiting'
  }

  return 'error'
}

function formatNeteaseTimestamp(value?: string | null) {
  if (!value) {
    return '未知时间'
  }

  const seconds = Number(value)
  const date = Number.isFinite(seconds) ? new Date(seconds * 1000) : new Date(value)
  if (Number.isNaN(date.getTime())) {
    return '未知时间'
  }

  return date.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function formatNeteaseCount(value?: number | null) {
  const count = Number(value ?? 0)
  if (!Number.isFinite(count) || count <= 0) {
    return '0'
  }

  if (count >= 100_000_000) {
    return `${(count / 100_000_000).toFixed(count >= 1_000_000_000 ? 0 : 1)}亿`
  }

  if (count >= 10_000) {
    return `${(count / 10_000).toFixed(count >= 100_000 ? 0 : 1)}万`
  }

  return String(Math.round(count))
}

function formatNeteasePlaylistMeta(playlist: NeteasePlaylistSummary) {
  return [
    `${formatNeteaseCount(playlist.trackCount)} 首`,
    `播放 ${formatNeteaseCount(playlist.playCount)}`,
    playlist.creatorNickname ? `创建者 ${playlist.creatorNickname}` : '',
    playlist.subscribed ? '收藏歌单' : '',
  ]
    .filter(Boolean)
    .join(' · ')
}

function formatNeteasePlaylistUpdate(playlist: NeteasePlaylistSummary) {
  const updateTime = Number(playlist.updateTime ?? 0)
  if (!Number.isFinite(updateTime) || updateTime <= 0) {
    return ''
  }

  const date = new Date(updateTime)
  if (Number.isNaN(date.getTime())) {
    return ''
  }

  return `更新 ${date.toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit' })}`
}

function formatNeteaseTrackArtists(track: NeteasePlaylistTrack) {
  return track.artists.filter(Boolean).join(' / ') || '未知歌手'
}

function formatNeteaseTrackDuration(track: NeteasePlaylistTrack) {
  const durationMs = Number(track.durationMs ?? 0)
  if (!Number.isFinite(durationMs) || durationMs <= 0) {
    return '未知时长'
  }

  return formatTime(durationMs / 1000)
}

function formatNeteaseTrackMeta(track: NeteasePlaylistTrack) {
  return [track.album ? `专辑 ${track.album}` : '', formatNeteaseTrackDuration(track)]
    .filter(Boolean)
    .join(' · ')
}

function formatTrackMeta(track: MusicTrack) {
  const details = [`播放 ${sanitizePlayCount(track.playCount)} 次`]

  if (track.lastPlayedAt) {
    details.push(`最近 ${formatPlayedAt(track.lastPlayedAt)}`)
  }

  if (track.favorite) {
    details.unshift('已收藏')
  }

  return details.join(' · ')
}

function formatTrackListMeta(track: MusicTrack) {
  return [
    track.album ? `专辑 ${track.album}` : '',
    formatTrackTagsLabel(track),
    formatTrackDuration(track),
    trackSourceLabel(track),
    formatTrackMeta(track),
  ]
    .filter(Boolean)
    .join(' · ')
}

function formatCurrentTrackDetail(track: MusicTrack) {
  if (track.source === 'netease') {
    return [
      track.album ? `专辑：${track.album}` : '',
      `时长：${formatTrackDuration(track)}`,
      '来源：网易云在线',
    ]
      .filter(Boolean)
      .join(' · ')
  }

  return [
    track.album ? `专辑：${track.album}` : '',
    formatTrackTagsLabel(track),
    `时长：${formatTrackDuration(track)}`,
    `来源：${trackSourceLabel(track)}`,
    `分类：${normalizeMusicCategory(track.category)}`,
    formatTrackMeta(track),
  ]
    .filter(Boolean)
    .join(' · ')
}

function trackArtistLabel(track: MusicTrack) {
  return normalizeTrackArtist(track.artist) || '未知歌手'
}

function formatTrackDuration(track: MusicTrack) {
  const trackDuration =
    currentTrack.value?.id === track.id && durationValue.value > 0
      ? durationValue.value
      : track.duration

  return trackDuration ? formatTime(trackDuration) : '未知时长'
}

function immersiveTrackDurationLabel(track: MusicTrack) {
  const trackDuration =
    immersiveTrackActive(track) && durationValue.value > 0
      ? durationValue.value
      : sanitizeTrackDuration(track.duration) ?? 0

  if (trackDuration > 0) {
    return formatTime(trackDuration)
  }

  return track.source === 'netease' ? '在线' : '--:--'
}

function immersiveNeteaseTrackDurationLabel(track: NeteasePlaylistTrack) {
  if (neteaseTrackActionId.value === track.id) {
    return '获取中'
  }

  const durationMs = Number(track.durationMs ?? 0)
  return Number.isFinite(durationMs) && durationMs > 0 ? formatTime(durationMs / 1000) : '--:--'
}

function trackSourceLabel(track: MusicTrack) {
  if (track.source === 'netease') {
    return '网易云在线'
  }

  return track.sourcePath && track.sourcePath !== track.path ? '存储目录' : '原始位置'
}

function trackSourceTitle(track: MusicTrack) {
  if (track.source === 'netease') {
    return `网易云歌曲 ID：${track.neteaseSongId ?? '未知'}`
  }

  if (track.sourcePath && track.sourcePath !== track.path) {
    return `来源：${track.sourcePath}\n播放文件：${track.path}`
  }

  return `来源：${track.path}`
}

function recognitionSourceLabel(source: string) {
  return source === 'metadata' ? 'metadata 标签' : source
}

function recognitionConfidenceLabel(candidate: MusicRecognitionCandidate) {
  return `${Math.round(clamp(candidate.confidence, 0, 1) * 100)}%`
}

function formatPlayedAt(value: string) {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return '未知时间'
  }

  return date.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function createFallbackVisualEnergyFrame(
  time: number,
  seedText: string,
  active: boolean,
): MusicEnergyFrame {
  if (!active) {
    return {
      bass: 0.08,
      mid: 0.06,
      treble: 0.04,
      volume: 0.06,
      beat: 0,
    }
  }

  const seed = stringHashRatio(seedText)
  const tempo = 1.15 + seed * 0.7
  const pulse = Math.max(0, Math.sin(time * Math.PI * 2 * tempo))
  const bass = 0.25 + pulse * 0.46 + Math.sin(time * 1.7 + seed * 5.1) * 0.08
  const mid = 0.2 + Math.sin(time * 2.9 + seed * 8.3) * 0.16 + pulse * 0.18
  const treble = 0.16 + Math.sin(time * 5.7 + seed * 11.6) * 0.13 + pulse * 0.1
  const volume = bass * 0.45 + mid * 0.35 + treble * 0.2

  return {
    bass: clamp(bass, 0.04, 0.95),
    mid: clamp(mid, 0.04, 0.9),
    treble: clamp(treble, 0.03, 0.85),
    volume: clamp(volume, 0.04, 0.92),
    beat: clamp((pulse - 0.62) * 2.4, 0, 1),
  }
}

function synthesizeFallbackFrequencyData(frame: MusicEnergyFrame, time: number) {
  const data = new Uint8Array(128)

  for (let index = 0; index < data.length; index += 1) {
    const progress = index / Math.max(1, data.length - 1)
    const bandEnergy =
      progress < 0.18 ? frame.bass : progress < 0.58 ? frame.mid : frame.treble
    const ripple =
      Math.sin(index * 0.39 + time * 4.8) * 0.07 +
      Math.sin(index * 0.15 - time * 2.2) * 0.05
    const value = clamp(
      (bandEnergy * (1 - progress * 0.38) + frame.volume * 0.14 + frame.beat * 0.18 + ripple) *
        255,
      0,
      255,
    )
    data[index] = Math.round(value)
  }

  return data
}

function stringHashRatio(value: string) {
  let hash = 2166136261

  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index)
    hash = Math.imul(hash, 16777619)
  }

  return (hash >>> 0) / 4294967295
}

function clamp(value: number, min: number, max: number) {
  if (!Number.isFinite(value)) {
    return min
  }

  return Math.min(max, Math.max(min, value))
}
</script>

<template>
  <main
    class="music-window"
    :class="[
      themeClass,
      windowOpenAnimationClass,
      { 'music-window-mini': miniPlayerMode, 'music-window-immersive': immersiveMode },
    ]"
  >
    <audio
      ref="audio"
      :src="currentTrack?.url"
      preload="metadata"
      @loadedmetadata="handleLoadedMetadata"
      @timeupdate="handleTimeUpdate"
      @ended="handleEnded"
      @error="handleAudioError"
    />

    <section v-if="miniPlayerMode" class="music-mini-player" @pointerdown="startDrag">
      <div class="music-mini-disc" :class="{ 'music-disc-playing': playing }" aria-hidden="true">
        <span />
      </div>
      <div class="music-mini-body">
        <div class="music-mini-header">
          <div class="music-mini-title">
            <strong>{{ currentTrack?.title || '未选择音乐' }}</strong>
            <span>{{ currentTrack ? trackArtistLabel(currentTrack) : '宠物电台待机中' }}</span>
          </div>
          <div class="music-mini-actions" @pointerdown.stop>
            <button type="button" title="恢复完整播放器" aria-label="恢复完整播放器" @click="setMiniPlayerMode(false)">
              ▣
            </button>
            <button type="button" title="隐藏播放器" aria-label="隐藏播放器" @click="hideMusicPlayer">
              ×
            </button>
          </div>
        </div>
        <div class="music-mini-progress" @pointerdown.stop>
          <input
            type="range"
            min="0"
            :max="durationValue"
            step="1"
            :value="progressValue"
            :disabled="!currentTrack || durationValue <= 0"
            aria-label="迷你播放器进度"
            @input="seek"
          />
        </div>
        <div class="music-mini-controls" @pointerdown.stop>
          <button type="button" :disabled="!hasTracks" title="上一首" aria-label="上一首" @click="playPrevious">
            ‹
          </button>
          <button
            type="button"
            class="music-mini-play"
            :disabled="!hasTracks"
            :title="playing ? '暂停' : '播放'"
            :aria-label="playing ? '暂停' : '播放'"
            @click="togglePlay"
          >
            {{ playing ? 'Ⅱ' : '▶' }}
          </button>
          <button type="button" :disabled="!hasTracks" title="下一首" aria-label="下一首" @click="playNext(true)">
            ›
          </button>
          <span>{{ formatTime(progressValue) }} / {{ formatTime(durationValue) }}</span>
          <span class="music-mini-percent">{{ Math.round(miniProgressPercent) }}%</span>
        </div>
      </div>
    </section>

    <section
      v-else-if="immersiveMode"
      class="music-immersive"
      :class="{
        'is-playlist-hidden': !immersivePlaylistVisible,
        'is-panel-hidden': !immersiveRhythmPanelVisible,
      }"
      @pointerdown="startDrag"
    >
      <MusicVisualizerCanvas
        :frequency-data="visualFrequencyData"
        :energy="visualEnergyFrame"
        :playing="playing"
        :mode="visualMode"
        :intensity="visualIntensity"
        :reduced-motion="visualReducedMotion"
        :theme="drawerTheme"
      />
      <div class="music-immersive-vignette" aria-hidden="true" />

      <section
        class="music-immersive-lyrics"
        :class="{
          'is-synced': immersiveLyrics.synced,
          'is-loading': lyricsStatus === 'loading',
          'is-empty': lyricsStatus === 'empty',
          'is-error': lyricsStatus === 'error',
          'is-interlude': immersiveLyrics.interlude,
          'is-reduced-motion': visualReducedMotion,
        }"
        aria-label="沉浸歌词"
      >
        <Transition name="immersive-lyric-status" mode="out-in">
          <span :key="immersiveLyricsStatusLabel" class="music-immersive-lyrics-status">
            {{ immersiveLyricsStatusLabel }}
          </span>
        </Transition>
        <div class="music-immersive-lyrics-stack">
          <div class="music-immersive-lyrics-slot side">
            <Transition name="immersive-lyric-side" mode="out-in">
              <p :key="immersiveLyrics.previousKey" class="music-immersive-lyrics-line previous">
                {{ immersiveLyrics.previous }}
              </p>
            </Transition>
          </div>
          <div class="music-immersive-lyrics-slot main">
            <Transition name="immersive-lyric-main">
              <p
                :key="immersiveLyrics.currentKey"
                class="music-immersive-lyrics-line current"
                :style="{ '--lyric-progress': `${Math.round(immersiveLyrics.progress * 1000) / 10}%` }"
              >
                <span class="music-immersive-lyrics-line-text">
                  {{ immersiveLyrics.current }}
                </span>
              </p>
            </Transition>
          </div>
          <div class="music-immersive-lyrics-slot side">
            <Transition name="immersive-lyric-side" mode="out-in">
              <p :key="immersiveLyrics.nextKey" class="music-immersive-lyrics-line next">
                {{ lyricsStatus === 'error' ? lyricsError : immersiveLyrics.next }}
              </p>
            </Transition>
          </div>
        </div>
      </section>

      <header class="music-immersive-header">
        <div>
          <span>{{ visualizerStatusLabel }}</span>
          <h1>{{ currentTrack?.title || '沉浸音乐模式' }}</h1>
          <p>{{ currentTrack ? trackArtistLabel(currentTrack) : '选择音乐后开始可视化' }}</p>
        </div>
        <div class="music-immersive-actions" @pointerdown.stop>
          <button
            type="button"
            :class="{ active: immersivePlaylistVisible }"
            :title="immersivePlaylistVisible ? '隐藏播放列表' : '显示播放列表'"
            @click="toggleImmersivePlaylistVisible"
          >
            {{ immersivePlaylistVisible ? '隐藏列表' : '显示列表' }}
          </button>
          <button
            type="button"
            :class="{ active: immersiveRhythmPanelVisible }"
            :title="immersiveRhythmPanelVisible ? '隐藏韵律面板' : '显示韵律面板'"
            @click="toggleImmersiveRhythmPanelVisible"
          >
            {{ immersiveRhythmPanelVisible ? '隐藏韵律' : '显示韵律' }}
          </button>
          <button type="button" title="返回普通播放器" @click="setImmersiveMode(false)">返回</button>
          <button type="button" title="切换小悬浮播放器" @click="setMiniPlayerMode(true)">迷你</button>
          <button type="button" class="window-close" title="隐藏播放器" @click="hideMusicPlayer">
            ×
          </button>
        </div>
      </header>

      <aside v-if="immersivePlaylistVisible" class="music-immersive-playlist" @pointerdown.stop>
        <div class="music-immersive-playlist-heading">
          <div class="music-immersive-playlist-title">
            <strong>播放列表</strong>
            <span>{{ immersivePlaylistCountLabel }}</span>
          </div>
          <div class="music-immersive-playlist-source" role="tablist" aria-label="播放列表来源">
            <button
              type="button"
              :class="{ active: immersivePlaylistSource === 'local' }"
              role="tab"
              :aria-selected="immersivePlaylistSource === 'local'"
              @click="immersivePlaylistSource = 'local'"
            >
              本地
            </button>
            <button
              type="button"
              :class="{ active: immersivePlaylistSource === 'netease' }"
              role="tab"
              :aria-selected="immersivePlaylistSource === 'netease'"
              @click="immersivePlaylistSource = 'netease'"
            >
              网易云
            </button>
          </div>
        </div>

        <div
          v-if="immersivePlaylistSource === 'local' && immersiveLocalPlaylistTracks.length > 0"
          class="music-immersive-playlist-list"
          role="list"
          aria-label="沉浸模式本地播放列表"
        >
          <button
            v-for="track in immersiveLocalPlaylistTracks"
            :key="`local-${track.id}`"
            type="button"
            class="music-immersive-playlist-row"
            :class="{
              active: immersiveTrackActive(track),
              online: track.source === 'netease',
            }"
            :disabled="!canPlayImmersiveTrack(track)"
            :aria-current="immersiveTrackActive(track) ? 'true' : undefined"
            :title="track.title"
            @click="playImmersiveTrack(track)"
          >
            <span class="music-immersive-playlist-index">
              {{ immersiveTrackActive(track) ? '▶' : '♪' }}
            </span>
            <span class="music-immersive-playlist-main">
              <strong>{{ track.title }}</strong>
              <small>{{ trackArtistLabel(track) }}</small>
            </span>
            <span class="music-immersive-playlist-duration">
              {{ immersiveTrackDurationLabel(track) }}
            </span>
          </button>
        </div>

        <div
          v-else-if="immersivePlaylistSource === 'netease' && immersiveNeteasePlaylistTracks.length > 0"
          class="music-immersive-playlist-list"
          role="list"
          aria-label="沉浸模式网易云播放列表"
        >
          <button
            v-for="(track, index) in immersiveNeteasePlaylistTracks"
            :key="`netease-${track.id}`"
            type="button"
            class="music-immersive-playlist-row online"
            :class="{ active: immersiveNeteaseTrackActive(track) }"
            :disabled="!canPlayImmersiveNeteaseTrack(track)"
            :aria-current="immersiveNeteaseTrackActive(track) ? 'true' : undefined"
            :title="`${track.name} - ${formatNeteaseTrackArtists(track)}`"
            @click="playImmersiveNeteaseTrack(track)"
          >
            <span class="music-immersive-playlist-index">
              {{ immersiveNeteaseTrackActive(track) ? '▶' : index + 1 }}
            </span>
            <span class="music-immersive-playlist-main">
              <strong>{{ track.name }}</strong>
              <small>{{ formatNeteaseTrackArtists(track) }}</small>
            </span>
            <span class="music-immersive-playlist-duration">
              {{ immersiveNeteaseTrackDurationLabel(track) }}
            </span>
          </button>
        </div>

        <div v-else class="music-immersive-playlist-empty">
          {{ immersivePlaylistEmptyLabel }}
        </div>
      </aside>

      <aside v-if="immersiveRhythmPanelVisible" class="music-immersive-panel" @pointerdown.stop>
        <div class="music-immersive-panel-heading">
          <strong>{{ visualModeLabel }}</strong>
          <span>能量 {{ visualizerEnergyLabel }}</span>
        </div>

        <div class="music-visual-mode-grid" role="tablist" aria-label="沉浸视觉模式">
          <button
            v-for="option in visualModeOptions"
            :key="option.value"
            type="button"
            :class="{ active: visualMode === option.value }"
            :title="option.description"
            @click="visualMode = option.value"
          >
            {{ option.label }}
          </button>
        </div>

        <label class="music-immersive-range">
          强度
          <input
            v-model.number="visualIntensity"
            type="range"
            min="0.2"
            max="1"
            step="0.01"
            aria-label="可视化强度"
          />
        </label>

        <label class="music-immersive-range music-immersive-offset">
          歌词
          <input
            v-model.number="lyricOffsetMs"
            type="range"
            min="-2000"
            max="2000"
            step="50"
            aria-label="歌词同步偏移"
          />
          <output>{{ lyricOffsetLabel }}</output>
        </label>

        <label class="music-immersive-check">
          <input v-model="visualReducedMotion" type="checkbox" />
          <span>降低动态</span>
        </label>

        <p
          v-if="beatMapError || (analyzerError && !beatMapMatchesCurrentTrack)"
          class="music-immersive-warning"
        >
          {{ visualizerHintLabel }}
        </p>
        <p v-else class="music-immersive-hint">
          {{ visualizerHintLabel }}
        </p>
      </aside>

      <div v-if="!currentTrack" class="music-immersive-empty" @pointerdown.stop>
        <strong>还没有正在播放的音乐</strong>
        <span>返回播放器添加或选择本地歌曲。</span>
        <button type="button" @click="setImmersiveMode(false)">返回播放器</button>
      </div>

      <footer class="music-immersive-controls" @pointerdown.stop>
        <div class="music-immersive-main-controls">
          <button type="button" :disabled="!hasTracks" title="上一首" @click="playPrevious">
            上一首
          </button>
          <button
            type="button"
            class="music-play-button"
            :disabled="!hasTracks"
            :title="playing ? '暂停' : '播放'"
            @click="togglePlay"
          >
            {{ playing ? '暂停' : '播放' }}
          </button>
          <button type="button" :disabled="!hasTracks" title="下一首" @click="playNext(true)">
            下一首
          </button>
        </div>

        <div class="music-immersive-progress">
          <span>{{ formatTime(progressValue) }}</span>
          <input
            type="range"
            min="0"
            :max="durationValue"
            step="1"
            :value="progressValue"
            :disabled="!currentTrack || durationValue <= 0"
            aria-label="沉浸模式播放进度"
            @input="seek"
          />
          <span>{{ formatTime(durationValue) }}</span>
        </div>

        <div class="music-immersive-options">
          <button
            type="button"
            :class="{ active: shuffleEnabled }"
            :disabled="tracks.length < 2"
            @click="shuffleEnabled = !shuffleEnabled"
          >
            随机
          </button>
          <button type="button" :disabled="!hasTracks" @click="toggleRepeatMode">
            {{ repeatModeLabel }}
          </button>
          <label>
            音量
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              :value="volume"
              aria-label="沉浸模式音量"
              @input="setVolume"
            />
          </label>
        </div>
      </footer>
    </section>

    <template v-else>
    <header class="music-header" @pointerdown="startDrag">
      <div class="music-header-title">
        <h1>音乐播放</h1>
        <p>{{ currentTrack ? (currentTrackOnline ? '正在播放网易云在线音乐' : '正在播放本机音乐') : '选择音频开始播放' }}</p>
      </div>
      <div class="music-header-actions" @pointerdown="stopHeaderDrag">
        <button type="button" title="进入沉浸式音乐模式" @click="toggleImmersiveMode">沉浸</button>
        <button type="button" title="切换小悬浮播放器" @click="toggleMiniPlayerMode">迷你</button>
        <button type="button" title="打开音乐设置" @click="showMusicSettings">设置</button>
        <button type="button" class="window-close" title="隐藏播放器" @click="hideMusicPlayer">
          ×
        </button>
      </div>
    </header>
    <datalist id="music-category-options">
      <option v-for="category in categoryOptions" :key="category" :value="category" />
    </datalist>
    <datalist id="music-tag-options">
      <option v-for="tag in tagOptions" :key="tag" :value="tag" />
    </datalist>

    <section class="music-now">
      <div class="music-disc" :class="{ 'music-disc-playing': playing }" aria-hidden="true">
        <span />
      </div>
      <div class="music-current-copy">
        <strong>{{ currentTrack?.title || '未选择音乐' }}</strong>
        <small v-if="currentTrack" class="music-current-artist">
          {{ trackArtistLabel(currentTrack) }}
        </small>
        <small v-if="currentTrack" class="music-current-meta" :title="trackSourceTitle(currentTrack)">
          {{ formatCurrentTrackDetail(currentTrack) }}
        </small>
        <small v-else class="music-current-meta">支持 mp3、wav、flac、m4a、aac、ogg、webm。</small>
      </div>
    </section>

    <section class="music-controls" aria-label="播放控制">
      <div class="music-main-controls">
        <button type="button" :disabled="!hasTracks" title="上一首" @click="playPrevious">
          上一首
        </button>
        <button
          type="button"
          class="music-play-button"
          :disabled="!hasTracks"
          :title="playing ? '暂停' : '播放'"
          @click="togglePlay"
        >
          {{ playing ? '暂停' : '播放' }}
        </button>
        <button type="button" :disabled="!hasTracks" title="下一首" @click="playNext(true)">
          下一首
        </button>
      </div>

      <div class="music-progress-row">
        <span>{{ formatTime(progressValue) }}</span>
        <input
          type="range"
          min="0"
          :max="durationValue"
          step="1"
          :value="progressValue"
          :disabled="!currentTrack || durationValue <= 0"
          aria-label="播放进度"
          @input="seek"
        />
        <span>{{ formatTime(durationValue) }}</span>
      </div>

      <div class="music-option-row">
        <button
          type="button"
          :class="{ active: shuffleEnabled }"
          :disabled="tracks.length < 2"
          @click="shuffleEnabled = !shuffleEnabled"
        >
          随机
        </button>
        <button type="button" :disabled="!hasTracks" @click="toggleRepeatMode">
          {{ repeatModeLabel }}
        </button>
        <label>
          音量
          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            :value="volume"
            aria-label="音量"
            @input="setVolume"
          />
        </label>
      </div>
    </section>

    <p v-if="playerStatus" class="music-status">{{ playerStatus }}</p>
    <p v-if="playerError" class="music-error">{{ playerError }}</p>

    <section class="music-playlist">
      <div class="music-playlist-header">
        <div class="music-panel-tabs" role="tablist" aria-label="音乐功能分区">
          <button
            v-for="option in panelViewOptions"
            :key="option.value"
            type="button"
            :class="{ active: activePanelView === option.value }"
            @click="activePanelView = option.value"
          >
            {{ option.label }}
          </button>
        </div>
        <div class="music-playlist-title">
          <strong>{{ playlistTitle }}</strong>
          <span>{{ playlistCountLabel }}</span>
        </div>
        <div v-if="activePanelView === 'library'" class="music-library-tabs" role="tablist" aria-label="音乐库视图">
          <button
            v-for="option in libraryViewOptions"
            :key="option.value"
            type="button"
            :class="{ active: activeLibraryView === option.value }"
            @click="activeLibraryView = option.value"
          >
            {{ option.label }}
          </button>
        </div>
        <div v-if="activePanelView === 'library'" class="music-playlist-filters">
          <div class="music-search-field">
            <input
              v-model="searchQuery"
              type="search"
              placeholder="搜索歌名、歌手、分类、标签或来源"
              aria-label="搜索歌曲"
            />
            <button
              type="button"
              :disabled="!searchQuery"
              title="清空搜索"
              aria-label="清空搜索"
              @click="clearMusicSearch"
            >
              ×
            </button>
          </div>
          <select v-model="activeCategoryFilter" aria-label="分类筛选">
            <option v-for="category in categoryFilterOptions" :key="category" :value="category">
              {{ category }}
            </option>
          </select>
          <select v-model="activeTagFilter" aria-label="标签筛选">
            <option v-for="tag in tagFilterOptions" :key="tag" :value="tag">
              {{ tag }}
            </option>
          </select>
          <button
            type="button"
            :disabled="clearButtonDisabled"
            @click="clearCurrentList"
          >
            {{ clearButtonLabel }}
          </button>
        </div>
      </div>

      <div v-if="activePanelView === 'playlists'" class="music-playlist-manager">
        <section class="music-playlist-section">
          <header class="music-playlist-section-header">
            <div>
              <strong>场景电台</strong>
              <span>按标签自动选歌，点击后直接播放一组歌曲。</span>
            </div>
          </header>

          <div class="music-shortcut-grid music-scene-grid">
            <button
              v-for="scenePlaylist in scenePlaylistOptions"
              :key="scenePlaylist.id"
              type="button"
              class="music-shortcut-card"
              :disabled="scenePlaylistTrackCount(scenePlaylist) === 0"
              :title="scenePlaylist.description"
              @click="playScenePlaylist(scenePlaylist)"
            >
              <strong>{{ scenePlaylist.title }}</strong>
              <span>{{ scenePlaylistDescription(scenePlaylist) }}</span>
            </button>
          </div>
        </section>

        <section class="music-playlist-section">
          <header class="music-playlist-section-header">
            <div>
              <strong>我的歌单</strong>
              <span>{{ customPlaylists.length }} 个歌单 · 点击歌单展开歌曲</span>
            </div>
            <div class="music-playlist-create">
              <input
                v-model="newPlaylistName"
                autocomplete="off"
                placeholder="新建歌单"
                @keydown.enter="createCustomPlaylist"
              />
              <button type="button" @click="createCustomPlaylist">新建</button>
            </div>
          </header>

          <div v-if="customPlaylists.length > 0" class="music-custom-playlists">
            <article
              v-for="playlist in customPlaylists"
              :key="playlist.id"
              class="music-custom-playlist-card"
              :class="{ active: activeCustomPlaylistId === playlist.id }"
            >
              <button
                type="button"
                class="music-custom-playlist-summary"
                @click="selectCustomPlaylist(playlist.id)"
              >
                <span aria-hidden="true">{{ activeCustomPlaylistId === playlist.id ? '▾' : '▸' }}</span>
                <div>
                  <strong>{{ playlist.name }}</strong>
                  <small>{{ playlistTrackCountLabel(playlist) }}</small>
                </div>
                <small>{{ playlistPreviewLabel(playlist) }}</small>
              </button>

              <div class="music-custom-playlist-actions">
                <button type="button" @click="playCustomPlaylist(playlist)">播放</button>
                <button type="button" @click="queueCustomPlaylist(playlist)">入队</button>
                <button type="button" @click="openPlaylistTrackPicker(playlist)">加歌</button>
                <button type="button" @click="renameCustomPlaylistWithPrompt(playlist)">改名</button>
                <button type="button" @click="deleteCustomPlaylist(playlist.id)">删除</button>
              </div>

              <div
                v-if="activeCustomPlaylistId === playlist.id"
                class="music-custom-playlist-detail"
              >
                <div v-if="customPlaylistTracks(playlist).length > 0" class="music-track-list">
                  <article
                    v-for="track in customPlaylistTracks(playlist)"
                    :key="track.id"
                    class="music-track-row"
                    :class="{
                      active: currentTrack?.id === track.id,
                      dragging: draggingPlaylistTrackId === track.id,
                    }"
                    draggable="true"
                    @dragstart="startPlaylistTrackDrag(track.id, $event)"
                    @dragover.prevent
                    @drop="dropPlaylistTrack(playlist, track.id, $event)"
                    @dragend="finishPlaylistTrackDrag"
                  >
                    <button type="button" class="music-track-main" @click="playTrackById(track.id)">
                      <span>{{ playlist.trackIds.indexOf(track.id) + 1 }}</span>
                      <div>
                        <strong>{{ track.title }}</strong>
                        <small class="music-track-artist">{{ trackArtistLabel(track) }}</small>
                        <small class="music-track-meta" :title="trackSourceTitle(track)">
                          {{ formatTrackListMeta(track) }}
                        </small>
                      </div>
                    </button>
                    <div class="music-track-actions">
                      <button
                        type="button"
                        class="music-track-more"
                        title="从歌单移除"
                        aria-label="从歌单移除"
                        @click="removeTrackFromPlaylist(playlist, track.id)"
                      >
                        ×
                      </button>
                    </div>
                  </article>
                </div>
                <div v-else class="music-empty music-playlist-empty">
                  <strong>歌单暂无歌曲</strong>
                  <span>点击“加歌”从本地音乐库中选择。</span>
                </div>
              </div>
            </article>
          </div>

          <div v-else class="music-empty">
            <strong>暂无自定义歌单</strong>
            <span>输入名称后创建第一个歌单。</span>
          </div>
        </section>
      </div>

      <div v-else-if="activePanelView === 'ai'" class="music-ai-panel">
        <strong>AI 推荐</strong>
        <p>{{ aiRecommendationSummary }}</p>
        <div class="music-shortcut-grid">
          <button
            v-for="option in aiRecommendationOptions"
            :key="option.id"
            type="button"
            class="music-shortcut-card"
            :disabled="aiRecommendationTrackCount(option) === 0"
            :title="option.description"
            @click="playAiRecommendation(option)"
          >
            <strong>{{ option.title }}</strong>
            <span>{{ aiRecommendationDescription(option) }}</span>
          </button>
        </div>
      </div>

      <div v-else-if="activePanelView === 'netease'" class="music-platform-panel">
        <section class="music-platform-card">
          <header class="music-platform-header">
            <div>
              <strong>网易云音乐</strong>
              <span>{{ neteaseStatusLabel }}</span>
            </div>
            <div class="music-platform-actions">
              <button type="button" :disabled="neteaseLoginBusy" @click="refreshNeteaseLoginStatus(true)">
                刷新
              </button>
              <button
                v-if="neteaseLoggedIn"
                type="button"
                :disabled="neteasePlaylistsLoading || neteasePlaylistDetailLoading"
                @click="refreshNeteasePlaylists(true)"
              >
                {{ neteasePlaylistsLoading ? '读取中' : '读取歌单' }}
              </button>
              <button
                v-if="!neteaseLoggedIn"
                type="button"
                :disabled="neteaseLoginBusy"
                @click="startNeteaseQrLogin"
              >
                扫码登录
              </button>
              <button v-else type="button" :disabled="neteaseLoginBusy" @click="clearNeteaseLogin">
                退出
              </button>
            </div>
          </header>

          <div v-if="neteaseProfile" class="music-platform-profile">
            <img
              v-if="neteaseProfile.avatarUrl"
              :src="neteaseProfile.avatarUrl"
              alt=""
              referrerpolicy="no-referrer"
            />
            <div v-else class="music-platform-avatar" aria-hidden="true">
              云
            </div>
            <div>
              <strong>{{ neteaseProfile.nickname }}</strong>
              <span>用户 ID {{ neteaseProfile.userId }}</span>
              <small v-if="neteaseLoginStatus?.savedAt">
                本机保存于 {{ formatNeteaseTimestamp(neteaseLoginStatus.savedAt) }}
              </small>
            </div>
          </div>

          <div v-else class="music-platform-empty">
            <strong>{{ neteaseLoggedIn ? '网易云账号已连接' : '未连接网易云账号' }}</strong>
            <span>{{ neteaseStatusDetail }}</span>
          </div>

          <div v-if="neteaseQrLogin" class="music-platform-qr">
            <img :src="neteaseQrLogin.qrImage" alt="网易云音乐登录二维码" />
            <div>
              <strong>{{ neteaseStatusLabel }}</strong>
              <span>{{ neteaseStatusDetail }}</span>
              <small>过期时间 {{ formatNeteaseTimestamp(neteaseQrLogin.expiresAt) }}</small>
              <div class="music-platform-actions">
                <button type="button" :disabled="neteaseLoginBusy" @click="startNeteaseQrLogin">
                  重新生成
                </button>
                <button type="button" @click="cancelNeteaseQrLogin">取消</button>
              </div>
            </div>
          </div>

          <p
            v-if="neteaseLoginError || neteaseLoginNotice"
            class="music-platform-message"
            :class="{ error: Boolean(neteaseLoginError) }"
          >
            {{ neteaseStatusDetail }}
          </p>
        </section>

        <section v-if="neteaseLoggedIn" class="music-platform-card">
          <header class="music-platform-section-header">
            <div>
              <strong>我的歌单</strong>
              <span>{{ neteasePlaylistStatusLabel }}</span>
            </div>
            <span>{{ neteasePlaylists.length }} 个</span>
          </header>

          <p
            v-if="neteasePlaylistError || neteasePlaylistsLoading || neteasePlaylistDetail"
            class="music-platform-message"
            :class="{ error: Boolean(neteasePlaylistError) }"
          >
            {{ neteasePlaylistStatusLabel }}
          </p>

          <div v-if="neteasePlaylistsLoading && neteasePlaylists.length === 0" class="music-platform-empty">
            <strong>正在读取网易云歌单</strong>
            <span>只读取歌单和歌曲摘要，不读取播放地址。</span>
          </div>

          <div v-else-if="neteasePlaylists.length === 0" class="music-platform-empty">
            <strong>暂无歌单数据</strong>
            <span>点击“读取歌单”重新获取网易云歌单列表。</span>
          </div>

          <div v-else class="music-platform-playlist-list">
            <button
              v-for="playlist in neteasePlaylists"
              :key="playlist.id"
              type="button"
              class="music-platform-playlist-row"
              :class="{ active: playlist.id === neteaseSelectedPlaylistId }"
              :disabled="neteasePlaylistDetailLoading"
              @click="loadNeteasePlaylistDetail(playlist)"
            >
              <img
                v-if="playlist.coverImgUrl"
                :src="playlist.coverImgUrl"
                alt=""
                referrerpolicy="no-referrer"
              />
              <span v-else class="music-platform-cover" aria-hidden="true">歌</span>
              <span class="music-platform-playlist-copy">
                <strong>{{ playlist.name }}</strong>
                <small>{{ formatNeteasePlaylistMeta(playlist) }}</small>
                <small v-if="formatNeteasePlaylistUpdate(playlist)">
                  {{ formatNeteasePlaylistUpdate(playlist) }}
                </small>
              </span>
            </button>
          </div>

          <section v-if="neteaseSelectedPlaylist" class="music-platform-detail">
            <header class="music-platform-section-header">
              <div>
                <strong>{{ neteaseSelectedPlaylist.name }}</strong>
                <span>
                  {{ neteasePlaylistDetail?.tracks.length ?? 0 }} /
                  {{ neteasePlaylistDetail?.totalTrackCount ?? neteaseSelectedPlaylist.trackCount }} 首
                </span>
              </div>
              <span v-if="neteasePlaylistDetail?.truncated">已截断</span>
            </header>

            <div v-if="neteasePlaylistDetailLoading" class="music-platform-empty">
              <strong>正在读取歌曲摘要</strong>
              <span>读取完成后会显示歌曲、歌手、专辑和时长。</span>
            </div>

            <div
              v-else-if="neteasePlaylistDetail?.tracks.length"
              class="music-platform-track-list"
            >
              <article
                v-for="(track, index) in neteasePlaylistDetail.tracks"
                :key="`${track.id}-${index}`"
                class="music-platform-track-row"
                :class="{ active: currentTrack?.source === 'netease' && currentTrack.neteaseSongId === track.id }"
              >
                <span class="music-platform-track-index">{{ index + 1 }}</span>
                <img
                  v-if="track.coverImgUrl"
                  :src="track.coverImgUrl"
                  alt=""
                  referrerpolicy="no-referrer"
                />
                <span v-else class="music-platform-track-cover" aria-hidden="true">♪</span>
                <span class="music-platform-track-copy">
                  <strong>{{ track.name }}</strong>
                  <small>{{ formatNeteaseTrackArtists(track) }}</small>
                  <small>{{ formatNeteaseTrackMeta(track) }}</small>
                </span>
                <span class="music-platform-track-actions">
                  <button
                    type="button"
                    :disabled="neteaseLyricsLoading && neteaseLyricsTrack?.id === track.id"
                    @click="showNeteaseLyrics(track)"
                  >
                    {{ neteaseLyricsLoading && neteaseLyricsTrack?.id === track.id ? '读取中' : '歌词' }}
                  </button>
                  <button
                    type="button"
                    :disabled="neteaseTrackActionId === track.id"
                    @click="playNeteaseTrack(track)"
                  >
                    {{ neteaseTrackActionId === track.id ? '获取中' : '播放' }}
                  </button>
                </span>
              </article>
            </div>

            <div v-else class="music-platform-empty">
              <strong>暂无歌曲摘要</strong>
              <span>当前歌单没有返回可展示的歌曲信息。</span>
            </div>
          </section>
        </section>

        <section
          v-if="neteaseLyricsTrack || neteaseLyricsError"
          class="music-platform-card music-platform-lyrics-card"
        >
          <header class="music-platform-section-header">
            <div>
              <strong>{{ neteaseLyricsTrack?.name || '在线歌词' }}</strong>
              <span>
                {{
                  neteaseLyricsLoading
                    ? '正在读取在线歌词'
                    : neteaseLyricsError || '网易云在线歌词'
                }}
              </span>
            </div>
            <button type="button" @click="closeNeteaseLyrics">关闭</button>
          </header>

          <p
            v-if="neteaseLyricsError || neteaseLyricsResult?.warnings.length"
            class="music-platform-message"
            :class="{ error: Boolean(neteaseLyricsError) }"
          >
            {{ neteaseLyricsError || neteaseLyricsResult?.warnings.join('；') }}
          </p>

          <div v-if="neteaseLyricsLoading" class="music-platform-empty">
            <strong>正在读取在线歌词</strong>
            <span>读取完成后会显示歌词文本，沉浸模式会自动同步使用。</span>
          </div>

          <div v-else-if="neteaseLyricsResult" class="music-platform-lyrics-view">
            <p class="music-platform-message">
              {{
                neteaseLyricsResult.yrcContent
                  ? '已读取网易云逐字歌词，沉浸模式优先使用 YRC 同步。'
                  : '当前歌曲未返回逐字歌词，沉浸模式使用普通歌词同步。'
              }}
            </p>
            <pre>{{ neteaseLyricsResult.lrcContent || neteaseLyricsResult.content || neteaseLyricsResult.translatedContent }}</pre>
            <details v-if="neteaseLyricsResult.translatedContent">
              <summary>翻译歌词</summary>
              <pre>{{ neteaseLyricsResult.translatedContent }}</pre>
            </details>
          </div>
        </section>

        <section class="music-platform-card is-muted">
          <strong>后续</strong>
          <span>已支持读取歌单、在线歌词和临时在线播放；搜索跳转与客户端兜底后续单独接入。</span>
        </section>
      </div>

      <div v-else-if="filteredTracks.length > 0" class="music-track-list">
        <article
          v-for="track in filteredTracks"
          :key="track.id"
          class="music-track-row"
          :class="{
            active: track.id === currentTrack?.id,
            'queue-mode': activeLibraryView === 'queue',
            dragging: draggingQueueTrackId === track.id,
          }"
          :draggable="activeLibraryView === 'queue'"
          @dragstart="startQueueDrag(track.id, $event)"
          @dragover.prevent
          @drop="dropQueueTrack(track.id, $event)"
          @dragend="finishQueueDrag"
          @contextmenu.prevent="openTrackActions(track.id)"
        >
          <button type="button" class="music-track-main" @click="playTrackById(track.id)">
            <span>{{ trackDisplayNumber(track.id) }}</span>
            <div>
              <strong>{{ track.title }}</strong>
              <small class="music-track-artist">{{ trackArtistLabel(track) }}</small>
              <small class="music-track-meta" :title="trackSourceTitle(track)">
                {{ formatTrackListMeta(track) }}
              </small>
            </div>
          </button>
          <div class="music-track-actions">
            <button
              type="button"
              class="music-track-more"
              :class="{ active: activeTrackActionsId === track.id }"
              title="更多操作"
              aria-label="更多操作"
              @click="toggleTrackActions(track.id)"
            >
              <span aria-hidden="true">⋯</span>
            </button>
          </div>
          <div
            v-if="activeTrackActionsId === track.id"
            class="music-track-action-menu"
            @click.stop
            @dragstart.stop
          >
            <template v-if="activeLibraryView !== 'queue'">
              <button
                type="button"
                title="添加到下一首"
                aria-label="添加到下一首"
                @click="queueTrackNext(track)"
              >
                <span aria-hidden="true">⤴</span>
                下一首
              </button>
              <button
                type="button"
                :class="{ active: isTrackQueued(track.id) }"
                title="添加到队尾"
                aria-label="添加到队尾"
                @click="queueTrackEnd(track)"
              >
                <span aria-hidden="true">＋</span>
                队尾
              </button>
            </template>
            <button
              v-else
              type="button"
              title="从队列移除"
              aria-label="从队列移除"
              @click="removeQueuedTrack(track)"
            >
              <span aria-hidden="true">−</span>
              出队
            </button>
            <button
              type="button"
              :disabled="recognitionBusyTrackId === track.id"
              title="读取 metadata 标签"
              aria-label="读取 metadata 标签"
              @click="recognizeTrackMetadata(track)"
            >
              <span aria-hidden="true">⌁</span>
              {{ recognitionBusyTrackId === track.id ? '读取中' : '标签' }}
            </button>
            <button
              type="button"
              :class="{ active: editingTrackId === track.id }"
              title="编辑歌名和歌手"
              aria-label="编辑歌名和歌手"
              @click="toggleTrackEditor(track.id)"
            >
              <span aria-hidden="true">✎</span>
              编辑
            </button>
            <button
              type="button"
              :class="{ active: track.favorite }"
              :title="track.favorite ? '取消收藏' : '收藏歌曲'"
              :aria-label="track.favorite ? '取消收藏歌曲' : '收藏歌曲'"
              @click="toggleTrackFavorite(track)"
            >
              <span aria-hidden="true">{{ track.favorite ? '♥' : '♡' }}</span>
              {{ track.favorite ? '取消收藏' : '收藏' }}
            </button>
            <button
              v-if="activeLibraryView !== 'queue'"
              type="button"
              title="移除歌曲"
              aria-label="移除歌曲"
              @click="removeTrack(track.id)"
            >
              <span aria-hidden="true">×</span>
              移除
            </button>
          </div>
        </article>
      </div>

      <div v-else class="music-empty">
        <strong>{{ emptyPlaylistTitle }}</strong>
        <button type="button" @click="showMusicSettings">打开设置添加音乐</button>
      </div>
    </section>

    <div
      v-if="editingTrack || recognitionCandidate"
      class="music-editor-backdrop"
      @click.self="closeTrackDialog"
    >
      <section
        v-if="editingTrack"
        class="music-editor-panel music-track-editor"
        role="dialog"
        aria-modal="true"
        aria-label="编辑歌曲信息和标签"
      >
        <header>
          <div>
            <h2>编辑歌曲</h2>
            <p>{{ editingTrack.title }} · {{ trackArtistLabel(editingTrack) }}</p>
          </div>
          <button type="button" class="window-close" title="关闭编辑" @click="closeTrackDialog">
            ×
          </button>
        </header>

        <div class="music-track-editor-fields">
          <label>
            歌名
            <input
              v-model="editingTrack.title"
              autocomplete="off"
              placeholder="歌曲名"
              @blur="updateTrackIdentity(editingTrack)"
            />
          </label>
          <label>
            歌手
            <input
              v-model="editingTrack.artist"
              autocomplete="off"
              placeholder="未知歌手"
              @blur="updateTrackIdentity(editingTrack)"
            />
          </label>
          <label>
            专辑
            <input
              v-model="editingTrack.album"
              autocomplete="off"
              placeholder="未设置"
              @blur="updateTrackIdentity(editingTrack)"
            />
          </label>
          <label>
            分类
            <input
              v-model="editingTrack.category"
              list="music-category-options"
              autocomplete="off"
              placeholder="未分类"
              @blur="updateTrackCategory(editingTrack)"
              @change="updateTrackCategory(editingTrack)"
            />
          </label>
          <label class="music-track-editor-tags">
            标签
            <input
              :value="formatTrackTagsInput(editingTrack)"
              list="music-tag-options"
              autocomplete="off"
              placeholder="治愈，学习，中文"
              @blur="updateTrackTags(editingTrack, inputEventValue($event))"
              @change="updateTrackTags(editingTrack, inputEventValue($event))"
            />
          </label>
        </div>

        <div class="music-tag-presets" aria-label="预设标签">
          <section v-for="group in musicTagPresetGroups" :key="group.id">
            <span>{{ group.title }}</span>
            <button
              v-for="tag in group.tags"
              :key="tag"
              type="button"
              :class="{ active: trackHasPresetTag(editingTrack, tag) }"
              @click="toggleTrackPresetTag(editingTrack, tag)"
            >
              {{ tag }}
            </button>
          </section>
        </div>

        <footer class="music-dialog-actions">
          <button type="button" @click="finishTrackIdentityEdit(editingTrack)">完成</button>
          <button type="button" @click="closeTrackDialog">取消</button>
        </footer>
      </section>

      <section
        v-else-if="recognitionCandidate"
        class="music-editor-panel music-recognition-panel"
        role="dialog"
        aria-modal="true"
        aria-label="metadata 标签结果"
      >
        <header>
          <div>
            <h2>metadata 标签结果</h2>
            <p>
              来源：{{ recognitionSourceLabel(recognitionCandidate.source) }} · 置信度：{{
                recognitionConfidenceLabel(recognitionCandidate)
              }}
            </p>
          </div>
          <button type="button" class="window-close" title="关闭标签结果" @click="dismissRecognitionCandidate">
            ×
          </button>
        </header>
        <dl>
          <div>
            <dt>歌名</dt>
            <dd>{{ recognitionCandidate.title || '未读取到' }}</dd>
          </div>
          <div>
            <dt>歌手（参与创作的艺术家）</dt>
            <dd>{{ recognitionCandidate.artist || '未读取到' }}</dd>
          </div>
          <div>
            <dt>专辑</dt>
            <dd>{{ recognitionCandidate.album || '未读取到' }}</dd>
          </div>
          <div>
            <dt>时长</dt>
            <dd>
              {{ recognitionCandidate.duration ? formatTime(recognitionCandidate.duration) : '未读取到' }}
            </dd>
          </div>
        </dl>
        <p v-if="recognitionCandidate.warnings.length > 0">
          {{ recognitionCandidate.warnings.join('；') }}
        </p>
        <footer class="music-dialog-actions">
          <button type="button" @click="applyRecognitionCandidate">应用结果</button>
          <button type="button" @click="dismissRecognitionCandidate">忽略</button>
        </footer>
      </section>
    </div>

    <div
      v-if="playlistTrackPickerVisible"
      class="music-editor-backdrop"
      @click.self="closePlaylistTrackPicker"
    >
      <section
        class="music-editor-panel music-playlist-picker"
        role="dialog"
        aria-modal="true"
        aria-label="添加歌曲到歌单"
      >
        <header>
          <div>
            <h2>添加歌曲</h2>
            <p>
              {{ playlistTrackPickerTarget?.name || '歌单' }} · 已选
              {{ playlistTrackPickerSelectedCount }} 首
            </p>
          </div>
          <button type="button" class="window-close" title="关闭添加歌曲" @click="closePlaylistTrackPicker">
            ×
          </button>
        </header>

        <div class="music-search-field music-playlist-picker-search">
          <input
            v-model="playlistTrackPickerQuery"
            type="search"
            placeholder="搜索歌名、歌手、分类或标签"
            aria-label="搜索可加入歌单的歌曲"
          />
          <button
            type="button"
            :disabled="!playlistTrackPickerQuery"
            title="清空搜索"
            aria-label="清空搜索"
            @click="playlistTrackPickerQuery = ''"
          >
            ×
          </button>
        </div>

        <div v-if="playlistTrackPickerAvailableTracks.length > 0" class="music-playlist-picker-list">
          <label
            v-for="track in playlistTrackPickerAvailableTracks"
            :key="track.id"
            class="music-playlist-picker-row"
            :class="{ active: playlistTrackPickerSelectedIds.includes(track.id) }"
          >
            <input
              type="checkbox"
              :checked="playlistTrackPickerSelectedIds.includes(track.id)"
              @change="togglePlaylistTrackPickerSelection(track.id)"
            />
            <span>{{ track.title }}</span>
            <small>{{ trackArtistLabel(track) }} · {{ formatTrackListMeta(track) }}</small>
          </label>
        </div>
        <div v-else class="music-empty">
          <strong>没有可添加的歌曲</strong>
          <span>当前歌单可能已包含全部匹配歌曲。</span>
        </div>

        <footer class="music-dialog-actions">
          <button
            type="button"
            :disabled="playlistTrackPickerSelectedCount === 0"
            @click="addPickedTracksToPlaylist"
          >
            添加 {{ playlistTrackPickerSelectedCount }} 首
          </button>
          <button type="button" @click="closePlaylistTrackPicker">取消</button>
        </footer>
      </section>
    </div>

    <div v-if="settingsVisible" class="music-settings-backdrop" @click.self="settingsVisible = false">
      <section class="music-settings-panel" role="dialog" aria-modal="true">
        <header>
          <div>
            <h2>音乐设置</h2>
            <p>添加歌曲、批量导入文件夹，并设置后续歌曲的本机存储目录。</p>
          </div>
          <button type="button" class="window-close" title="关闭设置" @click="settingsVisible = false">
            ×
          </button>
        </header>

        <label class="music-field">
          添加分类
          <input
            v-model="importCategory"
            list="music-category-options"
            placeholder="未分类"
            autocomplete="off"
          />
        </label>

        <div class="music-library-actions">
          <button type="button" @click="chooseMusicFiles">添加文件</button>
          <button type="button" @click="chooseMusicFolder">添加文件夹</button>
        </div>

        <label class="music-storage-field">
          <span>存储目录</span>
          <input
            readonly
            :value="musicStorageDir || '未设置，直接播放歌曲原始位置'"
            :title="musicStorageDir || '未设置，直接播放歌曲原始位置'"
          />
          <button type="button" @click="chooseMusicStorageDirectory">选择</button>
          <button type="button" :disabled="!musicStorageDir" @click="clearMusicStorageDirectory">
            原位置
          </button>
        </label>
      </section>
    </div>
    </template>
  </main>
</template>
